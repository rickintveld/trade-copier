mod cli;
mod instance;
mod manager;
mod wine;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use instance::Instance;
use manager::InstanceManager;
use std::fs;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { name, installer } => {
            handle_create(&name, &installer)?;
        }
        Commands::Delete { name } => {
            handle_delete(&name)?;
        }
        Commands::Start { name, all } => {
            handle_start(name.as_deref(), all)?;
        }
        Commands::List => {
            handle_list()?;
        }
    }

    Ok(())
}

fn handle_create(name: &str, installer: &std::path::Path) -> Result<()> {
    // Check Wine is installed
    wine::check_wine_installed()?;

    let manager = InstanceManager::new()?;

    // Check if instance name already exists
    if let Ok(_) = manager.get_instance(name) {
        anyhow::bail!("Instance '{}' already exists", name);
    }

    // Generate instance ID and prefix path
    let id = manager.next_id()?;
    let prefix_path = manager.generate_prefix_path(id)?;

    // Create Wine prefix
    wine::create_wine_prefix(&prefix_path)?;

    // Create and save instance metadata immediately after Wine prefix creation
    let instance = Instance::new(id, name.to_string(), prefix_path.clone());
    manager.add_instance(instance.clone())?;

    // Install MT5 (if this fails, instance is already saved and can be managed)
    if let Err(e) = wine::install_mt5(&prefix_path, installer) {
        eprintln!("\n⚠ Warning: MT5 installation failed: {}", e);
        eprintln!("The instance was created but MT5 installation incomplete.");
        eprintln!("You can delete it with: mt5-manager delete {}", name);
        return Err(e);
    }

    println!("\n✓ Instance '{}' created successfully!", name);
    println!("  ID: {}", id);
    println!("  Prefix: {}", prefix_path.display());
    println!("\nYou can now start it with: mt5-manager start {}", name);

    Ok(())
}

fn handle_delete(name: &str) -> Result<()> {
    let manager = InstanceManager::new()?;

    // Get instance
    let instance = manager.get_instance(name)?;

    // Confirm deletion
    println!("Are you sure you want to delete instance '{}'?", name);
    println!("This will remove: {}", instance.prefix_path.display());
    println!("Type 'yes' to confirm:");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != "yes" {
        println!("Deletion cancelled");
        return Ok(());
    }

    // Remove from manager first
    manager.remove_instance(name)?;

    // Delete Wine prefix directory
    if instance.prefix_path.exists() {
        fs::remove_dir_all(&instance.prefix_path)?;
        println!("✓ Removed Wine prefix: {}", instance.prefix_path.display());
    }

    println!("✓ Instance '{}' deleted successfully", name);

    Ok(())
}

fn handle_start(name: Option<&str>, all: bool) -> Result<()> {
    wine::check_wine_installed()?;

    let manager = InstanceManager::new()?;

    if all {
        // Start all instances
        let instances = manager.load_instances()?;
        if instances.is_empty() {
            println!("No instances found. Create one with: mt5-manager create");
            return Ok(());
        }

        println!("Starting {} instance(s)...", instances.len());
        for instance in instances {
            let mt5_exe = instance.mt5_executable();
            wine::launch_mt5(&instance.prefix_path, &mt5_exe)?;
            // Small delay between launches
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        println!("\n✓ All instances started");
    } else if let Some(name) = name {
        // Start specific instance
        let instance = manager.get_instance(name)?;
        let mt5_exe = instance.mt5_executable();
        wine::launch_mt5(&instance.prefix_path, &mt5_exe)?;
        println!("\n✓ Instance '{}' started", name);
    } else {
        anyhow::bail!("Please specify an instance name or use --all to start all instances");
    }

    Ok(())
}

fn handle_list() -> Result<()> {
    let manager = InstanceManager::new()?;
    let instances = manager.load_instances()?;

    if instances.is_empty() {
        println!("No instances found.");
        println!("Create one with: mt5-manager create --name <name> --installer <path>");
        return Ok(());
    }

    println!("MT5 Instances:\n");
    for instance in instances {
        println!("  {} (ID: {})", instance.name, instance.id);
        println!("    Prefix: {}", instance.prefix_path.display());
        println!("    Created: {}", instance.created_at);
        println!();
    }

    Ok(())
}
