use crate::instance::Instance;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct InstanceManager {
    instances_file: PathBuf,
}

impl InstanceManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home.join(".mt5-manager");
        let instances_file = config_dir.join("instances.json");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }

        Ok(Self {
            instances_file,
        })
    }

    pub fn load_instances(&self) -> Result<Vec<Instance>> {
        if !self.instances_file.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read_to_string(&self.instances_file)
            .context("Failed to read instances file")?;
        let instances: Vec<Instance> = serde_json::from_str(&data)
            .context("Failed to parse instances file")?;

        Ok(instances)
    }

    pub fn save_instances(&self, instances: &[Instance]) -> Result<()> {
        let data = serde_json::to_string_pretty(instances)
            .context("Failed to serialize instances")?;
        fs::write(&self.instances_file, data)
            .context("Failed to write instances file")?;
        Ok(())
    }

    pub fn add_instance(&self, instance: Instance) -> Result<()> {
        let mut instances = self.load_instances()?;
        instances.push(instance);
        self.save_instances(&instances)?;
        Ok(())
    }

    pub fn remove_instance(&self, name: &str) -> Result<Instance> {
        let mut instances = self.load_instances()?;
        let index = instances
            .iter()
            .position(|i| i.name == name)
            .context(format!("Instance '{}' not found", name))?;
        let instance = instances.remove(index);
        self.save_instances(&instances)?;
        Ok(instance)
    }

    pub fn get_instance(&self, name: &str) -> Result<Instance> {
        let instances = self.load_instances()?;
        instances
            .into_iter()
            .find(|i| i.name == name)
            .context(format!("Instance '{}' not found", name))
    }

    pub fn next_id(&self) -> Result<usize> {
        let instances = self.load_instances()?;
        Ok(instances.iter().map(|i| i.id).max().unwrap_or(0) + 1)
    }

    pub fn generate_prefix_path(&self, id: usize) -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(format!(".wine-mt5-instance{}", id)))
    }
}
