use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub sender_interval: u64,
    pub checker_interval: u64,
    pub log_filter: String,
    pub storage_path: String,
    pub cache_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sender_interval: 30,
            checker_interval: 5,
            log_filter: "info".to_string(),
            storage_path: "test_db".to_string(),
            cache_url: "http://127.0.0.1:32056".to_string(),
        }
    }
}
