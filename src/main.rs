use clap::Parser;
use geo::Point;
use tokio_util::sync::CancellationToken;

use crate::config::{AppConfig, ConfigBase};

mod arguments;
mod config;
mod mode;
mod notify;
mod report;

#[derive(Debug, Clone)]
struct Landmark {
    name: String,
    position: Point,
}

fn main() {
    run();
}

#[tokio::main]
async fn run() {
    let args = arguments::Cli::parse();
    let config_dir = match args.config_dir.as_ref() {
        Some(path) => ConfigBase::new(path),
        None => ConfigBase::default(),
    };
    let token = CancellationToken::new();

    let err = match args.command {
        None => mode::report_once::print_positions(&config_dir).await,
        // Default, list the current position of all devices once
        Some(arguments::Commands::List { recent }) => {
            if recent {
                mode::report_once::print_history(&config_dir).await
            } else {
                mode::report_once::print_positions(&config_dir).await
            }
        }
        // Live updates for a single device
        Some(arguments::Commands::Tail { device_id }) => {
            let config = config_dir.read_config_file().unwrap();
            let landmarks = config_dir.read_landmark_file().unwrap_or_default();
            let config2 = AppConfig::from_config_file(&config, landmarks.clone()).unwrap();
            mode::live_tail::tail_devices(config2, token, device_id, &landmarks).await;
            None
        }
        // Serve a dbus interface
        Some(arguments::Commands::Serve) => mode::serve::serve(&config_dir).await,

        // Provide credentials
        Some(arguments::Commands::Login) => mode::login_wizard::run(config_dir),
    };

    if let Some(err) = err {
        println!("Error: {err}")
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
