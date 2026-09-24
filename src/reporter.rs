use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use geo::{Bearing, Distance, GeodesicMeasure, Point};

use crate::{
    Device, GeoFenceResponse, Position, Report, devices::DeviceConfig, report::ReportPosition,
};

/// Reporter holds reference points, geofences, device preferences, display preferences ect
/// It can print extra information on positions with this context
#[derive(Debug)]
pub struct Reporter {
    landmarks: Vec<Landmark>,
    geofences: Vec<GeoFenceResponse>,
    device_config: HashMap<u32, DeviceConfig>,
}

impl Reporter {
    pub fn new() -> Self {
        Self {
            landmarks: Default::default(),
            geofences: Default::default(),
            device_config: Default::default(),
        }
    }

    pub fn landmarks_set(&mut self, slice: &[Landmark]) {
        self.landmarks.clear();
        self.landmarks.extend_from_slice(slice);
    }

    pub fn geofences_set(&mut self, slice: &[GeoFenceResponse]) {
        self.geofences.clear();
        self.geofences.extend_from_slice(slice);
    }

    pub fn config_set(&mut self, device_id: u32, config: DeviceConfig) {
        self.device_config.insert(device_id, config);
    }

    pub fn report_device(
        &self,
        device: &Device,
        position: &Position,
        now: DateTime<Utc>,
    ) -> Report {
        let device_config = self.device_config.get(&device.id);
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
                .filter_map(|id| self.geofences.iter().find(|geofence| *id == geofence.id))
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

            // This is lazylocked, no need to save somewhere
            let measurement_system = GeodesicMeasure::wgs84();

            let closest_landmark = self.landmarks.iter().min_by(|l1, l2| {
                let a = measurement_system.distance(l1.position, device_location);
                let b = measurement_system.distance(l2.position, device_location);

                f64::total_cmp(&a, &b)
            });

            // Report location relative to closest landmark, or a bare position as fallback
            closest_landmark.map_or(ReportPosition::BarePosition(device_location), |landmark| {
                let bearing = measurement_system.bearing(landmark.position, device_location);
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
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Landmark {
    pub name: String,
    pub position: Point,
}
