use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mt5-manager")]
#[command(about = "Manage multiple MetaTrader 5 instances on macOS", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new MT5 slave instance
    Create {
        /// Name for the instance
        #[arg(short, long)]
        name: String,

        /// Path to MT5 installer executable
        #[arg(short, long)]
        installer: PathBuf,
    },

    /// Delete an existing MT5 slave instance
    Delete {
        /// Name of the instance to delete
        name: String,
    },

    /// Start MT5 instance(s)
    Start {
        /// Name of the instance to start (omit to start all)
        name: Option<String>,

        /// Start all instances
        #[arg(short, long)]
        all: bool,
    },

    /// List all MT5 instances
    List,
}
