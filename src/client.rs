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

use crate::{
    config::Config,
    record::{Record, UnverifiedTX, VerifiedResult},
    time::{get_latest_finalized_minute, ms_to_minute_scale, unix_now},
};
use flume::Sender;
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::sync::Arc;
use storage_dal::Storage;

pub(crate) struct Client {
    pub config: Arc<RwLock<Config>>,
    pub storage: Storage,
    pub http_client: reqwest::Client,
    pub vr_sender: Sender<VerifiedResult>,
}

impl Client {
    pub async fn sender(&self) {
        let config = self.config.read().clone();
        for chain_sender in config.chain_sender_vec {
            let mut record = Record {
                timestamp: unix_now(),
                api: chain_sender.sender_url,
                data: chain_sender.data_for_send.clone(),
                resp: json!(null),
                status: 0,
                user_code: chain_sender.user_code,
            };
            match self
                .http_client
                .post(&record.api)
                .header("Content-Type", "application/json")
                .header("request_key", record.timestamp.to_string())
                .header("user_code", &record.user_code)
                .body(record.data.clone())
                .send()
                .await
            {
                Ok(resp) => {
                    debug!("resp: {:?}", resp);
                    match resp.json::<Value>().await {
                        Ok(resp) => {
                            info!("Post '{}': {:?}", &record.api, resp);
                            record.add_resp(resp.clone());
                            // save UnverifiedTX
                            if resp["code"].as_u64().unwrap() == 200 {
                                let utx = UnverifiedTX {
                                    tx_hash: resp["data"]["hash"].to_string().replace('\"', ""),
                                    sent_timestamp: record.timestamp,
                                    chain_name: chain_sender.chain_name.clone(),
                                    user_code: record.user_code.clone(),
                                };
                                debug!("insert: {:?}", &utx);
                                self.storage.insert(&utx.tx_hash.clone(), utx);
                            }
                        }
                        Err(e) => error!("decoding resp from '{}' failed: {}", &record.api, e),
                    }
                }
                Err(e) => error!("Call '{}' failed: {}", &record.api, e),
            }

            // When the call or decode fails, the sent_failed_num at current_minute will increase
            let current_minute = ms_to_minute_scale(record.timestamp);
            let mut vr = self
                .storage
                .get::<VerifiedResult>(&format!("{}/{}", chain_sender.chain_name, current_minute))
                .unwrap_or_else(|| {
                    // Record the result of the first two timeout intervals at the current moment
                    let res = self.storage.get::<VerifiedResult>(&format!(
                        "{}/{}",
                        chain_sender.chain_name,
                        get_latest_finalized_minute(record.timestamp, config.validator_timeout,)
                    ));
                    if let Some(res) = res {
                        let _ = self.vr_sender.send(res);
                    }
                    VerifiedResult::new(current_minute, chain_sender.chain_name.clone())
                });
            if record.status == 200 {
                vr.sent_num += 1;
                info!("sender insert: {:?}", &vr);
            } else {
                vr.sent_failed_num += 1;
                warn!("sender insert: {:?}", &vr);
            }
            self.storage.insert(
                &format!("{}/{}", chain_sender.chain_name, current_minute),
                vr,
            );

            debug!("sender: {:?}", &record);
            // self.storage.insert(&current_minute.to_string(), record);
        }
    }

    pub async fn validator(&self) {
        let unverified_txs = self.storage.scan::<UnverifiedTX>();
        let config = self.config.read().clone();
        for unverified_tx in unverified_txs {
            let utx = self
                .storage
                .get_by_path::<UnverifiedTX>(unverified_tx.unwrap().path())
                .unwrap();
            let current_minute = ms_to_minute_scale(utx.sent_timestamp);
            let mut vr = self
                .storage
                .get::<VerifiedResult>(&format!("{}/{}", utx.chain_name, current_minute))
                .unwrap_or_else(|| VerifiedResult::new(current_minute, utx.chain_name.clone()));
            if unix_now() - utx.sent_timestamp > (config.validator_timeout * 1000) {
                // timeout and failed
                warn!("Failed: {:?}", &utx.tx_hash);
                self.storage.remove::<UnverifiedTX>(&utx.tx_hash);

                vr.failed_num += 1;
                warn!("validator insert: {:?}", &vr);
                self.storage
                    .insert(&format!("{}/{}", utx.chain_name, current_minute), vr);
                continue;
            }

            self.verify_from_api(utx, vr, current_minute, &config.verify_api_url)
                .await;
        }
    }

    async fn verify_from_api(
        &self,
        utx: UnverifiedTX,
        mut vr: VerifiedResult,
        current_minute: u64,
        verify_api_url: &str,
    ) {
        let mut record = Record {
            timestamp: unix_now(),
            api: verify_api_url.to_string(),
            resp: json!(null),
            status: 0,
            user_code: utx.user_code.clone(),
            ..Default::default()
        };

        match self
            .http_client
            .get(&record.api)
            .header("request_key", utx.sent_timestamp.to_string())
            .header("user_code", &record.user_code)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(resp) => {
                    info!("Get  '{}/{}': {:?}", &record.api, &record.data, resp);
                    record.add_resp(resp);
                }
                Err(e) => error!("decoding resp from '{}' failed: {}", &record.api, e),
            },
            Err(e) => error!("Call '{}' failed: {}", &record.api, e),
        }

        if record.status == 200 {
            info!("Success: {:?}", &utx.tx_hash);
            self.storage.remove::<UnverifiedTX>(&utx.tx_hash);

            vr.succeed_num += 1;
            info!("validator insert: {:?}", &vr);
            self.storage
                .insert(&format!("{}/{}", utx.chain_name, current_minute), vr);
        }

        debug!("verify: {:?}", &record);
        // self.storage
        //     .insert(&format!("{}", &record.timestamp), record);
    }
}
