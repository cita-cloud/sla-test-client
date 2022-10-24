use common::unix_now;
use storage::{sledb::SledStorage, Storage};
use tracing::{info, warn};

use crate::{
    config::Config,
    record::{Record, Resp},
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
                    api: "/api/sendTx".to_string(),
                    data: "{
                                'data': '0x',
                                'quota': 1073741824,
                                'to': '524268b46968103ce8323353dab16ae857f09a6f',
                                'value': '0x0'
                            }"
                    .to_string(),
                    resp: None,
                    status: "0".to_string(),
                };

                match http_client
                    .post(format!("{}/{}", &config.cache_url, &record.api))
                    .header("Content-Type", "application/json")
                    .body(record.data.clone())
                    .send()
                    .await
                    .unwrap()
                    .json::<Resp>()
                    .await
                {
                    Ok(resp) => {
                        info!("{:#?}", resp);
                        record.add_resp(resp);
                    }
                    Err(e) => {
                        warn!("Call '{}' failed: {}", &record.api, e);
                    }
                }

                // TODO
                storage.insert(format!("{}", &record.timestamp), record);
            },
            _ = checker_interval.tick() => {
                // TODO
                let mut record = Record {
                    timestamp: unix_now(),
                    api: "/api/get-tx".to_string(),
                    data: "0xe8ea9dbfb8f6edccfc5bc3fe6f563d0410922da6439c4ada95c8b6767f10de81".to_string(),
                    resp: None,
                    status: "0".to_string(),
                };

                match http_client
                    .get(format!("{}/{}/{}", &config.cache_url, &record.api, &record.data))
                    .send()
                    .await
                    .unwrap()
                    .json::<Resp>()
                    .await
                {
                    Ok(resp) => {
                        info!("{:#?}", resp);
                        record.add_resp(resp);
                    }
                    Err(e) => {
                        warn!("Call '{}' failed: {}", &record.api, e);
                    }
                }

                // TODO
                storage.insert(format!("{}", &record.timestamp), record);
            }
        }
    }
}
