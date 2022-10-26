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
    record::{Record, Resp, UnverifiedTX, VerifiedResult},
};
use common::time::{ms_to_minute_scale, unix_now};
use std::sync::mpsc::{self, Sender};
use storage::{sledb::SledStorage, Storage};
use tracing::{debug, error, info};

pub async fn start(config: &Config) {
    let storage = SledStorage::init(&config.storage_path);
    let http_client = reqwest::ClientBuilder::default().build().unwrap();
    let mut sender_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.sender_interval));
    let mut checker_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.checker_interval));

    let (vr_sender, vr_receiver) = mpsc::channel::<VerifiedResult>();
    let metrics_port = config.metrics_port;
    tokio::spawn(crate::metrics::start(metrics_port, vr_receiver));

    loop {
        tokio::select! {
            _ = sender_interval.tick() => {
                sender(&http_client, config, &storage, &vr_sender).await;
            },

            _ = checker_interval.tick() => {
                checker(&http_client, config, &storage).await;
            }
        }
    }
}

async fn sender(
    http_client: &reqwest::Client,
    config: &Config,
    storage: &SledStorage,
    vr_sender: &Sender<VerifiedResult>,
) {
    for data in &config.data_for_send {
        let mut record = Record {
            timestamp: unix_now(),
            api: "api/sendTx".to_string(),
            data: data.to_string(),
            resp: None,
            status: 0,
        };
        match http_client
            .post(format!("{}/{}", &config.cache_url, &record.api))
            .header("Content-Type", "application/json")
            .body(record.data.clone())
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Resp>().await {
                Ok(resp) => {
                    info!("Post '{}': {:?}", &record.api, resp);
                    record.add_resp(resp.clone());
                    // save UnverifiedTX
                    if resp.status == 1 {
                        let utx = UnverifiedTX {
                            tx_hash: resp.data.unwrap(),
                            sent_timestamp: record.timestamp,
                        };
                        debug!("insert: {:?}", &utx);
                        storage.insert(&utx.tx_hash.clone(), utx);
                    }
                }
                Err(e) => error!("decoding resp from '{}' failed: {}", &record.api, e),
            },
            Err(e) => error!("Call '{}' failed: {}", &record.api, e),
        }

        // When the call or decode fails, the sent_failed_num at current_minute will increase
        let current_minute = ms_to_minute_scale(record.timestamp);
        let mut vr = storage
            .get::<VerifiedResult>(current_minute.to_be_bytes())
            .unwrap_or_else(|| {
                let res = storage.get::<VerifiedResult>((current_minute - 2).to_be_bytes());
                if res.is_some() {
                    vr_sender.send(res.unwrap()).unwrap();
                }
                VerifiedResult::new(current_minute)
            });
        if record.status == 1 {
            vr.sent_num += 1;
        } else {
            vr.sent_failed_num += 1;
        }
        debug!("insert: {:?}", &vr);
        storage.insert(current_minute.to_be_bytes(), vr);

        debug!("insert: {:?}", &record);
        storage.insert(record.timestamp.to_be_bytes(), record);
    }
}

async fn checker(http_client: &reqwest::Client, config: &Config, storage: &SledStorage) {
    let unverified_txs = storage.all::<UnverifiedTX>();
    for unverified_tx in unverified_txs {
        let tx_hash = unverified_tx.tx_hash;
        let sent_timestamp = unverified_tx.sent_timestamp;
        let mut vr = storage
            .get::<VerifiedResult>(ms_to_minute_scale(sent_timestamp).to_be_bytes())
            .unwrap();
        if unix_now() - sent_timestamp > 60000 {
            // timeout and failed
            storage.remove::<UnverifiedTX>(&tx_hash);
            vr.failed_num += 1;
            continue;
        }

        // TODO
        let mut record = Record {
            timestamp: unix_now(),
            api: "api/get-tx".to_string(),
            data: tx_hash.clone(),
            resp: None,
            status: 0,
        };

        match http_client
            .get(format!(
                "{}/{}/{}",
                &config.cache_url, &record.api, &record.data
            ))
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Resp>().await {
                Ok(resp) => {
                    info!(" Get '{}': {:?}", &record.api, resp);
                    record.add_resp(resp);
                }
                Err(e) => error!("decoding resp from '{}' failed: {}", &record.api, e),
            },
            Err(e) => error!("Call '{}' failed: {}", &record.api, e),
        }

        if record.status == 1 {
            storage.remove::<UnverifiedTX>(&tx_hash);
            vr.success_num += 1;
        }

        // TODO
        storage.insert(format!("{}", &record.timestamp), record);
    }
}
