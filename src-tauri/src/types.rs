use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,
    pub symbol: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<String>, // "buy" or "sell" (not needed for close/modify)
    pub lots: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sl: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp: Option<f64>,
    #[serde(default = "default_cmd")]
    pub cmd: String, // "open", "close", "modify", "cancel"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_type: Option<String>, // "market", "buy_limit", "sell_limit", "buy_stop", "sell_stop"
}

fn default_cmd() -> String {
    "open".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfig {
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub symbol_prefix: String,
}
