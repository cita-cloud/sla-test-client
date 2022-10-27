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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub sender_interval: u64,
    pub checker_interval: u64,
    pub log_file: String,
    pub storage_path: String,
    pub cache_url: String,
    pub metrics_port: u16,
    pub data_for_send: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sender_interval: 30,
            checker_interval: 5,
            log_file: "config/client-log4rs.yaml".to_string(),
            storage_path: "test_db".to_string(),
            cache_url: "http://127.0.0.1:32056".to_string(),
            metrics_port: 61616,
            data_for_send: vec![],
        }
    }
}
