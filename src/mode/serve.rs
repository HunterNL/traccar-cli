use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::{AppConfig, ConfigBase},
    mode::report_once::inner,
    report,
};
use chrono::Utc;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use traccar_lib::TracarrError;
use zbus::{connection, interface, names::BusName};

struct LocationService {
    location: Arc<Mutex<Vec<(u32, report::Report)>>>,
}

#[interface(name = "life.vern.traccar")]
impl LocationService {
    // Can be `async` as well.
    fn Get(&mut self, id: u32) -> String {
        let a = self.location.lock().unwrap();
        let binding = a.iter().find(|a| a.0 == id);
        match binding {
            Some((_, report)) => report.position.to_string(),
            None => "Id not found".to_string(),
        }

        // a.try_reserve_exact()
    }
}

pub async fn serve(config_dir: &ConfigBase) -> Option<TracarrError> {
    let device_locations = Arc::new(Mutex::new(vec![]));
    let landmarks = config_dir.read_landmark_file().unwrap_or_default();
    let config_file = config_dir.read_config_file().unwrap();
    let cancel_token = CancellationToken::new();

    let token_clone = cancel_token.clone();

    ctrlc::set_handler(move || token_clone.cancel()).expect("Error setting Ctrl-C handler");
    let config = AppConfig::from_config_file(&config_file, landmarks).expect("Config error");

    let location_clone = Arc::clone(&device_locations);
    tokio::spawn(async move {
        let location_service = LocationService {
            location: device_locations,
        };
        let dbus_connection = connection::Builder::session()
            .unwrap()
            .name("life.vern.traccar")
            .unwrap()
            .serve_at("/GetLocation", location_service)
            .unwrap()
            .build()
            .await
            .unwrap();

        loop {
            let reports = inner(&config).await.expect("error fetching positions");

            for (id, report) in &reports {
                dbus_connection
                    .emit_signal(
                        None::<BusName>,
                        "/device_positions",
                        "life.vern.traccar",
                        "position_update",
                        &(id, report.position.to_string()),
                    )
                    .await
                    .unwrap();
            }
            let next_report_time = reports
                .iter()
                .filter_map(|a| a.1.next_update_expected)
                .map(|a| a + Duration::from_secs(5)) //Add 5 seconds leeway for Traccar to handle the update
                .min();

            let sleep_duration: Duration = next_report_time
                .map(|date| date - Utc::now())
                .and_then(|delta| delta.to_std().ok())
                // .and_then(|a| a.try_into().ok())
                .unwrap_or(Duration::from_secs(30));

            {
                let mut l = location_clone.lock().unwrap();
                *l = reports;
                // Needed to drop the MutexGuard before awaiting
            }
            sleep(sleep_duration).await;
        }
    });

    cancel_token.cancelled().await;
    None
}
