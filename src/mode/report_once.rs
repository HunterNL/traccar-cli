use std::time::Duration;

use chrono::DateTime;

use futures::future::join_all;

use chrono::Utc;
use geo::Bearing;
use geo::Distance;
use geo::Point;

use traccar_lib::Device;
use traccar_lib::GeoFenceResponse;
use traccar_lib::Position;
use traccar_lib::TracarrError;

use crate::Landmark;
use crate::config::AppConfig;
use crate::config::ConfigBase;
use crate::config::DeviceConfig;
use crate::report::Report;
use crate::report::ReportPosition;

pub async fn print_positions(config: &ConfigBase) -> Option<TracarrError> {
    let config_file = &config.read_config_file().unwrap();
    let landmarks = config.read_landmark_file().unwrap_or_default();
    let config = AppConfig::from_config_file(config_file, landmarks).expect("Config error");
    let reports = match fetch_positions(&config).await {
        Ok(reports) => reports,
        Err(e) => return Some(e),
    };
    reports.iter().for_each(|a| {
        println!(
            "{}",
            a.1.as_ref()
                .map_or_else(|| format!("Device #{} unavailable", a.0), |e| e.to_string())
        );
    });

    None
}

pub async fn fetch_positions(
    config: &AppConfig,
) -> Result<Vec<(u32, Option<Report>)>, TracarrError> {
    let client = traccar_lib::Traccar::new(config.host(), config.token())?;
    let devices = client.list_devices().await?;
    let geofences = client.geofences_all().await?;
    let landmarks = config.landmarks();
    // let session = client.session_get().await;

    // Join the actual position to a device
    let devices_with_position: Vec<(Device, Option<Position>)> =
        join_all(devices.into_iter().map(async |device| {
            let position = match device.position_id {
                Some(n) => Some(client.position_get(n).await),
                None => None,
            };

            let position = match position {
                Some(Ok(p)) => Some(p),
                Some(Err(e)) => {
                    eprintln!("Error getting position: {e:?}");
                    // TODO Consider bubbling this up
                    None
                }

                None => None,
            };

            // let position = client.position_get(device.position_id).await;
            (device, position)
        }))
        .await;

    let now = Utc::now();

    // devices_with_position.iter().for_each(|(device, position)| {
    Ok(devices_with_position
        .iter()
        .filter_map(|(device, position)| {
            let device_config = config.device_config(device.id);
            if device_config.is_some_and(|config| config.hidden.is_some_and(|a| a)) {
                return None;
            }
            let report = position.as_ref().map(|position| {
                report_device(
                    device,
                    position,
                    geofences.as_slice(),
                    landmarks,
                    device_config,
                    now,
                )
            });

            Some((device.id, report))
        })
        .collect())
}

fn report_device(
    device: &Device,
    position: &Position,
    geofences: &[GeoFenceResponse],
    landmarks: &[Landmark],
    device_config: Option<&DeviceConfig>,
    now: DateTime<Utc>,
) -> Report {
    let dtime = now.signed_duration_since(position.fix_time);
    let seconds_ago = dtime.as_seconds_f32().round() as u32;
    let name = device_config
        .and_then(|a| a.display_name.as_deref())
        .unwrap_or(device.name.as_str());

    let expected_next_fix_time = device_config
        .and_then(|config| config.predict_update_interval_seconds)
        .map(|seconds| {
            let duration = Duration::from_secs(seconds.into());
            position.fix_time + duration
        });

    let in_timeout = device_config
        .and_then(|c| c.report_timeout_seconds)
        .map(|timeout_seconds| seconds_ago < timeout_seconds);

    let position: ReportPosition = {
        // Turn the ids inside a position into a vec of &geofence
        let fences: Vec<&GeoFenceResponse> = position
            .geofence_ids
            .iter()
            .filter_map(|id| geofences.iter().find(|geofence| *id == geofence.id))
            .collect();

        // If we are in a known geofence
        if !fences.is_empty() {
            return Report::new(
                name.to_owned(),
                ReportPosition::InGeofences(fences.iter().map(|a| a.name.to_owned()).collect()),
                in_timeout,
                seconds_ago,
                expected_next_fix_time,
            );
        }

        // We're not in a geofence, go find the nearest landmark
        let device_location = Point::new(position.longitude, position.latitude);

        let closest_landmark = landmarks.iter().min_by(|l1, l2| {
            let a = geo::GeodesicMeasure::wgs84().distance(l1.position, device_location);
            let b = geo::GeodesicMeasure::wgs84().distance(l2.position, device_location);

            f64::total_cmp(&a, &b)
        });

        // Report location relative to closest landmark, or a bare position as fallback
        closest_landmark.map_or(ReportPosition::BarePosition(device_location), |landmark| {
            let bearing = geo::GeodesicMeasure::wgs84().bearing(landmark.position, device_location);
            let distance = geo::GeodesicMeasure::wgs84()
                .distance(landmark.position, device_location)
                .round();

            ReportPosition::RelativeTo {
                distance,
                bearing,
                name: landmark.name.to_owned(),
            }
        })
    };
    Report::new(
        name.to_owned(),
        position,
        in_timeout,
        seconds_ago,
        expected_next_fix_time,
    )
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use geo::{LineString, Polygon};

    use super::*;

    fn default_position() -> Position {
        Default::default()
    }

    #[test]
    fn test_report_geofence() {
        let device = Device {
            id: 0,
            name: "Device".to_owned(),
            position_id: Some(1),
        };

        let mut position = default_position(); // Void island
        position.geofence_ids = vec![2];

        let p = Polygon::new(LineString::from(vec![(0., 0.), (1., 1.), (1., 0.)]), vec![]);
        // let geofence_polygon = Polygon::new(exterior, interiors)
        let geo: Vec<GeoFenceResponse> = vec![GeoFenceResponse {
            id: 2,
            name: "Geofence 1".to_owned(),
            description: None,
            area: p,
        }];

        let now = DateTime::from_timestamp_nanos(1_000_000_000);

        let report = report_device(&device, &position, geo.as_slice(), &[], None, now);

        assert_eq!(report.to_string(), "Device was in Geofence 1 1 seconds ago")
    }

    #[test]
    fn test_report_relative() {
        let device = Device {
            id: 0,
            name: "Device".to_owned(),
            position_id: Some(1),
        };

        let mut position = default_position();
        position.geofence_ids = vec![2];

        let landmark = Landmark {
            name: "Landmark".to_owned(),
            position: Point::new(1.0, 1.0),
        };

        let now = DateTime::from_timestamp_nanos(1_000_000_000);

        let report = report_device(&device, &position, &[], &[landmark], None, now);

        assert_eq!(
            report.to_string(),
            "Device was 157km SW of Landmark 1 seconds ago"
        )
    }

    #[test]
    fn test_report_bare() {
        let device = Device {
            id: 0,
            name: "Device".to_owned(),
            position_id: Some(1),
        };
        let mut position = default_position();
        position.geofence_ids = vec![2];

        let now = DateTime::from_timestamp_nanos(1_000_000_000);

        let report = report_device(&device, &position, &[], &[], None, now);

        assert_eq!(report.to_string(), "Device was at 0,0 1 seconds ago")
    }
}
