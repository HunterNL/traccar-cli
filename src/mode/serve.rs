use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::{AppConfig, ConfigBase},
    mode::report_once::fetch_positions,
    notify::send_notification,
};
mod zbus;
use chrono::Utc;
use futures::future::join_all;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use traccar_lib::{Device, Position, Report, Reporter, TracarrError};

struct ReportStore(Arc<Mutex<Vec<(u32, Report)>>>);

impl ReportStore {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(vec![])))
    }

    pub fn add_report(&mut self, device_id: u32, report: Report) -> Vec<GeoFenceMovement> {
        let mut inner = self.0.lock().unwrap();

        let existing_entry = inner.iter_mut().find(|a| a.0 == device_id);

        let movements = match &existing_entry {
            Some(old_report) => geofence_movements_from_report(&old_report.1, &report),
            None => vec![],
        };

        match existing_entry {
            Some(entry) => *entry = (device_id, report),
            None => inner.push((device_id, report)),
        }

        movements
    }

    pub fn get_by_id(&self, id: u32) -> Option<(u32, Report)> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .find(|report| report.0 == id)
            .cloned()
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

fn geofence_movements_from_report(
    old_report: &Report,
    new_report: &Report,
) -> Vec<GeoFenceMovement> {
    assert_eq!(old_report.name, new_report.name);
    let name = &old_report.name;

    let old_fences = old_report.position.geofences();

    let new_fences = new_report.position.geofences();

    // Get additions by subtracting old_fences from new_fences
    let additions: Vec<_> = new_fences
        .iter()
        .filter(|new_fence| !old_fences.contains(new_fence))
        .collect();

    // Get subtractions by taking old_fences and subtracting new_fences
    let subtractions: Vec<_> = old_fences
        .iter()
        .filter(|old_fence| !new_fences.contains(old_fence))
        .collect();

    additions
        .into_iter()
        .map(|addition| GeoFenceMovement {
            device_name: name.clone(),
            geofence_name: addition.clone(),
            action: GeoFenceAction::Entered,
        })
        .chain(
            subtractions
                .into_iter()
                .map(|subtraction| GeoFenceMovement {
                    device_name: name.clone(),
                    geofence_name: subtraction.clone(),
                    action: GeoFenceAction::Exited,
                }),
        )
        .collect()
}

struct LocationService {
    location: ReportStore,
}

pub struct GeoFenceMovement {
    device_name: String,
    geofence_name: String,
    action: GeoFenceAction,
}

pub enum GeoFenceAction {
    Exited,
    Entered,
}

// fn print_source(mut e: &dyn std::error::Error) {
//     loop {
//         println!("{e} {e:#?}");
//         if e.source().is_some() {
//             e = e.source().unwrap()
//         } else {
//             break;
//         }
//     }
// }
//
//

// pub fn earlier<T: TimeZone>(t1: DateTime<T>, t2: DateTime<T>) -> DateTime<T> {
//     if t1 < t2 { t1 } else { t2 }
// }

pub async fn serve(config_dir: &ConfigBase, reporter: Reporter) -> Option<TracarrError> {
    let device_locations = ReportStore::new();
    let landmarks = config_dir.read_landmark_file().unwrap_or_default();
    let config_file = config_dir.read_config_file().unwrap();
    let cancel_token = CancellationToken::new();

    let token_clone = cancel_token.clone();

    ctrlc::set_handler(move || token_clone.cancel()).expect("Error setting Ctrl-C handler");
    let config = AppConfig::from_config_file(&config_file).expect("Config error");
    let client = traccar_lib::Traccar::new(config.host(), config.token()).unwrap();
    let geofences = client.geofences_all().await.unwrap();

    let mut reporter = reporter;
    reporter.landmarks_set(&landmarks);
    reporter.geofences_set(&geofences);

    let mut location_clone = device_locations.clone();
    tokio::spawn(async move {
        let location_service = LocationService {
            location: device_locations,
        };
        let dbus_connection = zbus::Builder::session()
            .unwrap()
            .name("life.vern.traccar")
            .unwrap()
            .serve_at("/GetLocation", location_service)
            .unwrap()
            .build()
            .await
            .unwrap();

        loop {
            let devices = fetch_positions(&client).await; //.expect("error fetching positions");

            if let Err(e) = devices {
                eprintln!("Error fetching data: {e}");
                eprintln!("{e:#?}");
                // print_source(&e);
                tokio::time::sleep(Duration::from_secs(20)).await;
                continue;
            }

            let now = Utc::now();
            let devices: Vec<(Device, Position, Report)> = devices
                .unwrap()
                .into_iter()
                .filter_map(|(device, position)| {
                    let report = position
                        .as_ref()
                        .map(|r| reporter.report_device(&device, r, now))?;
                    Some((device, position?, report))
                })
                .collect();

            // let mut next_update_time = None;

            for (_, _, report) in &devices {
                let body = report.position.to_string();

                dbus_connection
                    .emit_signal(
                        None::<zbus::BusName>,
                        "/device_positions",
                        "life.vern.traccar",
                        "position_update",
                        &body,
                    )
                    .await
                    .unwrap();
            }

            // Get the earliest possible refresh

            let next_update_time = devices
                .iter()
                .filter_map(|a| a.2.next_update_expected)
                .min()
                .map(|a| a + Duration::from_secs(5));

            let sleep_duration: Duration = next_update_time
                .map(|date| date - Utc::now())
                .and_then(|delta| delta.to_std().ok())
                // .and_then(|a| a.try_into().ok())
                .unwrap_or(Duration::from_secs(30));

            let movements: Vec<GeoFenceMovement> = devices
                .into_iter()
                .flat_map(|report| location_clone.add_report(report.1.device_id, report.2))
                .collect();

            join_all(movements.iter().map(notify_for_movement)).await;
            sleep(sleep_duration).await;
        }
    });

    cancel_token.cancelled().await;
    None
}

pub async fn notify_for_movement(movement: &GeoFenceMovement) {
    let title = match movement.action {
        GeoFenceAction::Exited => "Geofence exited",
        GeoFenceAction::Entered => "Geofence entered",
    };

    let body = match movement.action {
        GeoFenceAction::Exited => format!(
            "{} has left {}",
            movement.device_name, movement.geofence_name
        ),
        GeoFenceAction::Entered => format!(
            "{} has entered {}",
            movement.device_name, movement.geofence_name
        ),
    };

    send_notification(title, &body).await;
}
