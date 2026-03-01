use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{config::AppConfig, mode::report_once::report_positions, report};
use chrono::Utc;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
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

pub async fn serve(
    config: AppConfig,
    token: CancellationToken,
    location: Arc<Mutex<Vec<(u32, report::Report)>>>,
) -> () {
    let config_clone = config.clone();
    let location_clone = Arc::clone(&location);
    tokio::spawn(async move {
        let location_service = LocationService { location };
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
            let reports = report_positions(&config_clone).await;

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

    token.cancelled().await
}
