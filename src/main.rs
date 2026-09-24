use clap::Parser;
use tokio_util::sync::CancellationToken;
use traccar_lib::Reporter;

use crate::config::{AppConfig, ConfigBase};

mod arguments;
mod config;
mod mode;
mod notify;
// mod report;

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
    let config_file = config_dir.read_config_file().unwrap();
    let landmarks = config_dir.read_landmark_file().unwrap_or_default();
    let config = AppConfig::from_config_file(&config_file).unwrap();

    let mut reporter = Reporter::new();
    reporter.landmarks_set(&landmarks);

    config
        .device_config_list()
        .iter()
        .for_each(|(device_id, config)| {
            reporter.config_set(*device_id, config.clone());
        });

    let err = match args.command {
        None => mode::report_once::print_positions(&config, reporter).await,
        // Default, list the current position of all devices once
        Some(arguments::Commands::List { recent }) => {
            if recent {
                mode::report_once::print_history(&config, reporter).await
            } else {
                mode::report_once::print_positions(&config, reporter).await
            }
        }
        // Live updates for a single device
        Some(arguments::Commands::Tail { device_id }) => {
            mode::live_tail::tail_devices(config, reporter, device_id, token).await;
            None
        }
        // Serve a dbus interface
        Some(arguments::Commands::Serve) => mode::serve::serve(&config_dir, reporter).await,

        // Provide credentials
        Some(arguments::Commands::Login) => mode::login_wizard::run(config_dir),
    };

    if let Some(err) = err {
        println!("Error: {err}");
    }
}
