use std::sync::{Arc, Mutex};

use clap::Parser;
use geo::Point;

use tokio_util::sync::CancellationToken;

mod arguments;
mod config;
mod mode;
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
    let config_file = config::config_read();
    let landmarks = config::landmarks_read();

    match args.command {
        Some(arguments::Commands::Tail) => {
            unimplemented!()
        }
        Some(arguments::Commands::Serve) => {
            // let config = AppConfig::from_config_file(&config_file, landmarks);
            let cancel_token = CancellationToken::new();

            let token_clone = cancel_token.clone();

            ctrlc::set_handler(move || token_clone.cancel()).expect("Error setting Ctrl-C handler");

            mode::serve::serve(
                config_file,
                landmarks,
                cancel_token,
                Arc::new(Mutex::new(vec![])),
            )
            .await;
        }
        Some(arguments::Commands::Login) => mode::login_wizard::run(&config_file),
        Some(arguments::Commands::List) | None => {
            let reports = mode::report_once::report_positions(&config_file, landmarks).await;
            reports.iter().for_each(|a| {
                println!("{}", a.1);
            });
        }
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
