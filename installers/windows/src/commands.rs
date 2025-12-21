use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new MT5 instance
    Create {
        /// Name of the instance
        name: String,
        
        /// Custom installation path (optional)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    
    /// Delete an existing MT5 instance
    Delete {
        /// Name of the instance to delete
        name: String,
        
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
    
    /// Start MT5 instance(s)
    Start {
        /// Name of the instance to start (omit to start all)
        name: Option<String>,
    },
    
    /// List all MT5 instances
    List,
}
