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
use storage::StorageData;
use storage_derive::StorageData;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Resp {
    pub status: u8,
    pub data: Option<String>,
    pub message: String,
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct Record {
    pub timestamp: u64,
    pub api: String,
    pub data: String,
    pub resp: Option<Resp>,
    pub status: u8,
}

impl Record {
    pub fn add_resp(&mut self, resp: Resp) {
        self.status = resp.status;
        self.resp = Some(resp);
    }
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct UnverifiedTX {
    pub tx_hash: String,
    pub sent_timestamp: u64,
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct VerifiedResult {
    pub timestamp: u64,
    pub sent_num: u8,
    pub sent_failed_num: u8,
    pub failed_num: u8,
    pub success_num: u8,
}

impl VerifiedResult {
    pub fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            sent_num: 0,
            sent_failed_num: 0,
            failed_num: 0,
            success_num: 0,
        }
    }
}
