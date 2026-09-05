use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    // Primary command to run
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// custom config file location
    #[arg(long, value_name = "DIRECTORY", global = true)]
    pub config_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Unimplemented
    Tail,
    /// Serve dbus interface
    Serve,
    /// List all devices and their position
    List {
        #[arg(short, long)]
        recent: bool,
    },
    /// Set host and credentials
    Login,
}
