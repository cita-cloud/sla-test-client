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

use common::time::{ms_to_minute_scale, unix_now};
use storage::{sledb::SledStorage, Storage};
use tracing::{debug, error, info};

use crate::{
    config::Config,
    record::{Record, Resp, UnverifiedTX, VerifiedResult},
};

pub async fn start(config: &Config) {
    let storage = SledStorage::init(&config.storage_path);
    let mut sender_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.sender_interval));
    let mut checker_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.checker_interval));
    let http_client = reqwest::ClientBuilder::default().build().unwrap();

    loop {
        tokio::select! {
            _ = sender_interval.tick() => {
                let mut record = Record {
                    timestamp: unix_now(),
                    api: "api/sendTx".to_string(),
                    data: "{
                                \"data\": \"0x\",
                                \"quota\": 1073741824,
                                \"to\": \"524268b46968103ce8323353dab16ae857f09a6f\",
                                \"value\": \"0x0\"
                            }"
                    .to_string(),
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

                // When the call or decode fails, the sent_failed_num at the current moment is incremented
                let current_minute = ms_to_minute_scale(record.timestamp);
                let mut vr = storage
                    .get::<VerifiedResult>(current_minute.to_be_bytes())
                    .unwrap_or_else(|| VerifiedResult::new(current_minute));
                if record.status == 1 {
                    vr.sent_num += 1;
                } else {
                    vr.sent_failed_num += 1;
                }
                debug!("insert: {:?}", &vr);
                storage.insert(current_minute.to_be_bytes(), vr);

                debug!("insert: {:?}", &record);
                storage.insert(record.timestamp.to_be_bytes(), record);
            },
            _ = checker_interval.tick() => {
                // TODO
                let mut record = Record {
                    timestamp: unix_now(),
                    api: "api/get-tx".to_string(),
                    data: "0xe8ea9dbfb8f6edccfc5bc3fe6f563d0410922da6439c4ada95c8b6767f10de81".to_string(),
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

                // TODO
                storage.insert(format!("{}", &record.timestamp), record);
            }
        }
    }
}
