use chrono::Days;
use futures::future::join_all;

use chrono::Utc;

use traccar_lib::Device;
use traccar_lib::Position;
use traccar_lib::Report;
use traccar_lib::Reporter;
use traccar_lib::TracarrError;

use crate::config::AppConfig;

pub async fn print_positions(config: &AppConfig, mut reporter: Reporter) -> Option<TracarrError> {
    let client = traccar_lib::Traccar::new(config.host(), config.token()).unwrap();
    let geofences = client.geofences_all().await.unwrap();
    reporter.geofences_set(&geofences);

    let devices = match fetch_positions(&client).await {
        Ok(reports) => reports,
        Err(e) => return Some(e),
    };

    let now = Utc::now();

    for (device, position) in &devices {
        if position.is_none() {
            println!("Device #{} unavailable", device.id);
        } else {
            println!(
                "{}",
                reporter.report_device(device, position.as_ref().unwrap(), now)
            );
        }
    }

    None
}

pub async fn fetch_positions(
    client: &traccar_lib::Traccar, // config: &AppConfig,
                                   // reporter: Reporter,
) -> Result<Vec<(Device, Option<Position>)>, TracarrError> {
    let devices = client.list_devices().await?;

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
    Ok(devices_with_position)
}

pub async fn fetch_position_history(
    config: &AppConfig,
    mut reporter: Reporter,
) -> Result<Vec<(u32, Option<Vec<Report>>)>, TracarrError> {
    let client = traccar_lib::Traccar::new(config.host(), config.token())?;
    let devices = client.list_devices().await?;
    let geofences = client.geofences_all().await?;

    reporter.geofences_set(&geofences);

    let now = Utc::now();
    let a_day_ago = now
        .checked_sub_days(Days::new(1))
        .expect("daylight saving times oopsie");

    // For every device, get the location history
    let devices_with_position: Vec<(Device, Option<Vec<Position>>)> =
        join_all(devices.into_iter().map(async |device| {
            let position = match device.position_id {
                Some(_) => Some(client.position_history(device.id, a_day_ago, now).await),
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
        .filter_map(|(device, positions)| {
            let device_config = config.device_config(device.id);
            if device_config.is_some_and(|config| config.hidden.is_some_and(|a| a)) {
                return None;
            }
            let report = positions.as_ref().map(|position| {
                position
                    .iter()
                    .map(|pos| reporter.report_device(device, pos, now))
                    .collect()
            });

            Some((device.id, report))
        })
        .collect())
}

pub(crate) async fn print_history(config: &AppConfig, reporter: Reporter) -> Option<TracarrError> {
    let devices = match fetch_position_history(config, reporter).await {
        Ok(reports) => reports,
        Err(e) => return Some(e),
    };

    for (_, reports) in &devices {
        reports
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|report| println!("{report}"));
    }

    None
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use geo::{LineString, Point, Polygon};
    use traccar_lib::{GeoFenceResponse, Landmark};

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

        let mut r = Reporter::new();
        r.geofences_set(&geo);

        let now = DateTime::from_timestamp_nanos(1_000_000_000);

        let report = r.report_device(&device, &position, now);

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

        let mut r = Reporter::new();
        r.landmarks_set(&[landmark]);

        let report = r.report_device(&device, &position, now);

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
        let r = Reporter::new();

        let report = r.report_device(&device, &position, now);

        assert_eq!(report.to_string(), "Device was at 0,0 1 seconds ago")
    }
}
