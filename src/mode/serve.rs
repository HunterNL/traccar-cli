use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::{AppConfig, ConfigBase},
    mode::report_once::fetch_positions,
    notify::send_notification,
    report,
};
mod zbus;
use chrono::Utc;
use futures::future::join_all;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use traccar_lib::TracarrError;

struct ReportStore(Arc<Mutex<Vec<(u32, report::Report)>>>);

impl ReportStore {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(vec![])))
    }

    pub fn add_report(&mut self, device_id: u32, report: report::Report) -> Vec<GeoFenceMovement> {
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

    pub fn get_by_id(&self, id: u32) -> Option<(u32, report::Report)> {
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
    old_report: &report::Report,
    new_report: &report::Report,
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

pub async fn serve(config_dir: &ConfigBase) -> Option<TracarrError> {
    let device_locations = ReportStore::new();
    let landmarks = config_dir.read_landmark_file().unwrap_or_default();
    let config_file = config_dir.read_config_file().unwrap();
    let cancel_token = CancellationToken::new();

    let token_clone = cancel_token.clone();

    ctrlc::set_handler(move || token_clone.cancel()).expect("Error setting Ctrl-C handler");
    let config = AppConfig::from_config_file(&config_file, landmarks).expect("Config error");

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
            let reports = fetch_positions(&config).await; //.expect("error fetching positions");

            if let Err(e) = reports {
                eprintln!("Error fetching data: {e}");
                eprintln!("{e:#?}");
                // print_source(&e);
                tokio::time::sleep(Duration::from_secs(20)).await;
                continue;
            }

            let reports = reports.unwrap();

            for (id, report) in &reports {
                let body = (
                    id,
                    report
                        .as_ref()
                        .map(|r| &r.position)
                        // .as_ref()
                        .map_or("Unavailable".to_string(), |e| e.to_string()),
                );

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
            let next_report_time = reports
                .iter()
                .filter(|a| a.1.is_some())
                .filter_map(|a| a.1.as_ref().unwrap().next_update_expected)
                .map(|a| a + Duration::from_secs(5)) //Add 5 seconds leeway for Traccar to handle the update
                .min();

            let sleep_duration: Duration = next_report_time
                .map(|date| date - Utc::now())
                .and_then(|delta| delta.to_std().ok())
                // .and_then(|a| a.try_into().ok())
                .unwrap_or(Duration::from_secs(30));

            let movements: Vec<GeoFenceMovement> = reports
                .into_iter()
                .filter(|e| e.1.is_some())
                .flat_map(|report| location_clone.add_report(report.0, report.1.unwrap()))
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
