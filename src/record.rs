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
use serde_json::Value;
use storage_dal::StorageData;

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct Record {
    /// Units in ms
    pub timestamp: u64,
    pub api: String,
    pub data: String,
    pub resp: Value,
    pub status: u8,
}

impl Record {
    pub fn add_resp(&mut self, resp: Value) {
        self.status = resp["status"].as_u64().unwrap() as u8;
        self.resp = resp;
    }
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct UnverifiedTX {
    pub tx_hash: String,
    /// Units in ms
    pub sent_timestamp: u64,
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct VerifiedResult {
    /// Units in minutes
    pub timestamp: u64,
    pub sent_num: u8,
    pub sent_failed_num: u8,
    pub failed_num: u8,
    pub succeed_num: u8,
}

impl VerifiedResult {
    pub const fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            sent_num: 0,
            sent_failed_num: 0,
            failed_num: 0,
            succeed_num: 0,
        }
    }
}
