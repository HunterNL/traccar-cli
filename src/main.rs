use clap::Parser;
use geo::Point;

use crate::config::ConfigBase;

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
    let config_dir = match args.config_dir {
        Some(path) => ConfigBase::new(path),
        None => ConfigBase::default(),
    };

    let err = match args.command {
        // Default, list the current position of all devices once
        Some(arguments::Commands::List) | None => {
            mode::report_once::print_positions(&config_dir).await
        }
        // Live updates for a single device
        Some(arguments::Commands::Tail) => {
            unimplemented!()
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
