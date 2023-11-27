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
    /// Units in second
    pub validator_timeout: u64,
    pub log_config: LogConfig,
    pub storage_path: String,
    pub auto_api_url: String,
    pub metrics_port: u16,
    pub chain_for_send: Vec<String>,
    pub data_for_send: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sender_interval: 30,
            validator_interval: 10,
            log_config: Default::default(),
            storage_path: "default_db".to_string(),
            auto_api_url: "http://127.0.0.1:32056".to_string(),
            metrics_port: 61616,
            chain_for_send: vec![],
            validator_timeout: 300,
            data_for_send: Default::default(),
        }
    }
}
