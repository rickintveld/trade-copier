use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,
    pub symbol: String,
    #[serde(rename = "type")]
    pub trade_type: String, // "buy" or "sell"
    pub lots: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sl: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp: Option<f64>,
    #[serde(default = "default_cmd")]
    pub cmd: String, // "open", "close", "modify"
}

fn default_cmd() -> String {
    "open".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlaveConfig {
    pub name: String,
    pub address: String,
    pub multiplier: f64,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub slaves: Vec<SlaveConfig>,
}
