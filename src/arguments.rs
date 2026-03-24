use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)] // requires `derive` feature
// #[command(name = "git")]
// #[command(about = "A fictional versioning CLI", long_about = None)]
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
    List,
    /// Set host and credentials
    Login,
}
