use serde::{Deserialize, Serialize};
use storage::StorageData;
use storage_derive::StorageData;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Resp {
    pub status: String,
    pub data: Option<String>,
    pub message: String,
}

#[derive(StorageData, Debug, Clone, Default, Deserialize, Serialize)]
pub struct Record {
    pub timestamp: u64,
    pub api: String,
    pub data: String,
    pub resp: Option<Resp>,
    pub status: String,
}

impl Record {
    pub fn add_resp(&mut self, resp: Resp) {
        self.status = resp.status.clone();
        self.resp = Some(resp);
    }
}
