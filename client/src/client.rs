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
    metrics::run_metrics_exporter,
    record::{Record, UnverifiedTX, VerifiedResult},
};
use common::{
    time::{get_latest_finalized_minute, ms_to_minute_scale, unix_now},
    toml::{calculate_md5, read_toml},
};
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::mpsc::{self, Sender},
};
use storage::{sledb::SledStorage, Storage};

pub async fn start(mut config: Config, config_path: impl AsRef<Path> + Clone) {
    let storage = SledStorage::init(&config.storage_path);
    let http_client = reqwest::ClientBuilder::default().build().unwrap();

    let (vr_sender, vr_receiver) = mpsc::channel::<VerifiedResult>();
    let metrics_port = config.metrics_port;
    tokio::spawn(crate::metrics::start(
        vr_receiver,
        storage.clone(),
        config.check_timeout,
        config.chain_block_interval,
    ));
    tokio::spawn(run_metrics_exporter(metrics_port));

    let mut config_md5 = calculate_md5(&config_path).unwrap();

    let mut sender_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.sender_interval));
    let mut checker_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.checker_interval));
    let mut hot_update_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.hot_update_interval));

    loop {
        tokio::select! {
            _ = sender_interval.tick() => {
                sender(&http_client, &config, &storage, &vr_sender).await;
            },
            _ = checker_interval.tick() => {
                checker(&http_client, &config, &storage).await;
            }
            _ = hot_update_interval.tick() => {
                hot_update(&mut config, &mut config_md5, config_path.clone());
            }
        }
    }
}

fn hot_update(config: &mut Config, config_md5: &mut [u8; 16], config_path: impl AsRef<Path>) {
    if let Ok(new_md5) = calculate_md5(&config_path) {
        if new_md5 != *config_md5 {
            *config_md5 = new_md5;
            *config = read_toml(config_path).unwrap();
            info!("config file changed: {:?}", config);
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
            resp: json!(null),
            status: 0,
        };
        match http_client
            .post(format!("{}/{}", &config.cache_url, &record.api))
            .header("Content-Type", "application/json")
            .body(record.data.clone())
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(resp) => {
                    debug!("Post '{}': {:?}", &record.api, resp);
                    record.add_resp(resp.clone());
                    // save UnverifiedTX
                    if resp["status"].as_u64().unwrap() == 1 {
                        let utx = UnverifiedTX {
                            tx_hash: resp["data"].to_string().replace('\"', ""),
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
                // Record the result of the first two timeout intervals at the current moment
                let res = storage.get::<VerifiedResult>(
                    get_latest_finalized_minute(
                        record.timestamp,
                        config.check_timeout,
                        config.chain_block_interval,
                    )
                    .to_be_bytes(),
                );
                if let Some(res) = res {
                    let _ = vr_sender.send(res);
                }
                VerifiedResult::new(current_minute)
            });
        if record.status == 1 {
            vr.sent_num += 1;
        } else {
            vr.sent_failed_num += 1;
        }
        info!("sender insert: {:?}", &vr);
        storage.insert(current_minute.to_be_bytes(), vr);

        debug!("insert: {:?}", &record);
        storage.insert(record.timestamp.to_be_bytes(), record);
    }
}

async fn checker(http_client: &reqwest::Client, config: &Config, storage: &SledStorage) {
    let unverified_txs = storage.all::<UnverifiedTX>();
    for unverified_tx in unverified_txs {
        let UnverifiedTX {
            tx_hash,
            sent_timestamp,
        } = unverified_tx;
        let current_minute = ms_to_minute_scale(sent_timestamp);
        let mut vr = storage
            .get::<VerifiedResult>(current_minute.to_be_bytes())
            .unwrap_or_else(|| VerifiedResult::new(current_minute));
        if unix_now() - sent_timestamp
            > (config.check_timeout as u64 * config.chain_block_interval as u64 * 1000)
        {
            // timeout and failed
            warn!("Failed: {:?}", &tx_hash);
            storage.remove::<UnverifiedTX>(&tx_hash);

            vr.failed_num += 1;
            info!("checker insert: {:?}", &vr);
            storage.insert(current_minute.to_be_bytes(), vr);
            continue;
        }

        let mut record = Record {
            timestamp: unix_now(),
            api: "api/get-tx".to_string(),
            data: tx_hash.clone(),
            resp: json!(null),
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
            Ok(resp) => match resp.json::<Value>().await {
                Ok(resp) => {
                    debug!("Get  '{}': {:?}", &record.api, resp);
                    record.add_resp(resp);
                }
                Err(e) => error!("decoding resp from '{}' failed: {}", &record.api, e),
            },
            Err(e) => error!("Call '{}' failed: {}", &record.api, e),
        }

        if record.status == 1 {
            info!("Success: {:?}", &tx_hash);
            storage.remove::<UnverifiedTX>(&tx_hash);

            vr.succeed_num += 1;
            info!("checker insert: {:?}", &vr);
            storage.insert(current_minute.to_be_bytes(), vr);
        }

        debug!("insert: {:?}", &record);
        storage.insert(format!("{}", &record.timestamp), record);
    }
}
