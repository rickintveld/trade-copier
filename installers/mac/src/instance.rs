use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: usize,
    pub name: String,
    pub prefix_path: PathBuf,
    pub created_at: String,
}

impl Instance {
    pub fn new(id: usize, name: String, prefix_path: PathBuf) -> Self {
        let created_at = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            prefix_path,
            created_at,
        }
    }

    pub fn mt5_executable(&self) -> PathBuf {
        self.prefix_path
            .join("drive_c/Program Files/MetaTrader 5/terminal64.exe")
    }
}
