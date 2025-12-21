mod commands;
mod config;
mod manager;

use clap::Parser;
use commands::Commands;
use manager::InstanceManager;

#[derive(Parser)]
#[command(name = "mt5-manager")]
#[command(about = "Manage multiple MetaTrader 5 slave instances", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Create { name, path } => {
            let mut manager = InstanceManager::new()?;
            manager.create_instance(name, path)?;
        }
        Commands::Delete { name, force } => {
            let mut manager = InstanceManager::new()?;
            manager.delete_instance(&name, force)?;
        }
        Commands::Start { name } => {
            let manager = InstanceManager::new()?;
            manager.start_instance(name.as_deref())?;
        }
        Commands::List => {
            let manager = InstanceManager::new()?;
            manager.list_instances()?;
        }
    }
    
    Ok(())
}
