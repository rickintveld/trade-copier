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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>, // "open", "close", "modify"
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlaveConfig {
    pub name: String,
    pub address: String,
    pub local_bind: String,
    pub multiplier: f64,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub slaves: Vec<SlaveConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ack {
    pub ack: u64,
}
