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

impl SlaveConfig {
    /// Validates the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Worker name cannot be empty".to_string());
        }
        if self.multiplier <= 0.0 {
            return Err("Multiplier must be greater than 0".to_string());
        }
        if !self.address.contains(':') {
            return Err("Address must be in format host:port".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitInfo {
    pub profit: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slave_config_validation_valid() {
        let config = SlaveConfig {
            name: "TestWorker".to_string(),
            address: "127.0.0.1:5001".to_string(),
            multiplier: 1.5,
            symbol_prefix: String::new(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_slave_config_validation_empty_name() {
        let config = SlaveConfig {
            name: String::new(),
            address: "127.0.0.1:5001".to_string(),
            multiplier: 1.0,
            symbol_prefix: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_slave_config_validation_invalid_multiplier() {
        let config = SlaveConfig {
            name: "TestWorker".to_string(),
            address: "127.0.0.1:5001".to_string(),
            multiplier: 0.0,
            symbol_prefix: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_slave_config_validation_invalid_address() {
        let config = SlaveConfig {
            name: "TestWorker".to_string(),
            address: "invalid_address".to_string(),
            multiplier: 1.0,
            symbol_prefix: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_trade_serialization() {
        let trade = Trade {
            id: 12345,
            symbol: "EURUSD".to_string(),
            cmd: "open".to_string(),
            trade_type: Some("buy".to_string()),
            lots: 0.1,
            price: Some(1.0850),
            sl: Some(1.0800),
            tp: Some(1.0900),
            order_type: Some("market".to_string()),
        };
        
        let json = serde_json::to_string(&trade).unwrap();
        assert!(json.contains("EURUSD"));
        assert!(json.contains("12345"));
    }

    #[test]
    fn test_default_cmd() {
        assert_eq!(default_cmd(), "open");
    }
}
