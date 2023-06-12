// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cloud_util::tracer::LogConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Units in second
    pub sender_interval: u64,
    /// Units in second
    pub validator_interval: u64,
    /// Units in block
    pub validator_timeout: u32,
    /// Units in second
    pub chain_block_interval: u32,
    /// Units in second
    pub hot_update_interval: u64,
    pub log_config: LogConfig,
    pub storage_path: String,
    pub cache_url: String,
    pub metrics_port: u16,
    pub data_for_send: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sender_interval: 30,
            validator_interval: 10,
            hot_update_interval: 5,
            log_config: Default::default(),
            storage_path: "default_db".to_string(),
            cache_url: "http://127.0.0.1:32056".to_string(),
            metrics_port: 61616,
            data_for_send: vec![],
            validator_timeout: 20,
            chain_block_interval: 3,
        }
    }
}
