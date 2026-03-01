use std::{
    env,
    sync::{Arc, Mutex},
};

use geo::Point;

use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;

mod config;
mod mode;
mod report;

#[derive(Debug, Clone)]
struct Landmark {
    name: String,
    position: Point,
}

fn main() {
    let config = config::config_get();
    run(config);
}

#[tokio::main]
async fn run(config: AppConfig) {
    let tail = env::args().any(|arg| &arg == "tail");
    let serve = env::args().any(|arg| &arg == "serve");

    if !tail && !serve {
        mode::report_once::report_positions(&config.clone()).await;
    }
    let cancel_token = CancellationToken::new();

    let token_clone = cancel_token.clone();

    ctrlc::set_handler(move || token_clone.cancel()).expect("Error setting Ctrl-C handler");

    if serve {
        return mode::serve::serve(config, cancel_token, Arc::new(Mutex::new(vec![]))).await;
    }
    if tail {
        // mode::live_tail::tail_devices(config, cancel_token).await;
    }
}

fn format_distance(distance: &f64) -> Option<String> {
    match distance {
        ..0.0 => None,
        0.0..1000.0 => Some(format!("{distance:.0}m")), // 0-999 meters
        1000f64..10_000f64 => Some(format!("{:.2}km", distance / 1000.0)), //1km-9.99km,
        10_000f64..100_000f64 => Some(format!("{:.1}km", distance / 1000.0)), //10.0km-99.9km
        100_000f64.. => Some(format!("{:.0}km", distance / 1000.0)), //100 km
        _ => None,                                      // _ => Some("Very far away".to_string()),
    }
}
