use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::AppConfig,
    mode::report_once::{report_positions, report_single_device},
    report,
};
use tokio_util::sync::CancellationToken;
use zbus::{connection, interface};

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
    let config_clone2 = config.clone();
    let location_clone = Arc::clone(&location);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let reports = report_positions(&config_clone2).await;

            let mut l = location_clone.lock().unwrap();
            *l = reports
        }
    });
    let greeter = LocationService { location };
    let conn = connection::Builder::session()
        .unwrap()
        .name("life.vern.traccar")
        .unwrap()
        .serve_at("/GetLocation", greeter)
        .unwrap()
        .build()
        .await
        .unwrap();

    token.cancelled().await
}
