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

use crate::record::VerifiedResult;
use crate::time::{get_latest_finalized_minute, get_readable_time_from_minute, unix_now};

use color_eyre::eyre::Result;
use flume::Receiver;
use heck::ToSnakeCase;
use prometheus::{
    core::{AtomicU64, GenericCounter},
    gather, register_int_counter, Encoder, TextEncoder,
};
use reqwest::header::CONTENT_TYPE;
use salvo::prelude::*;

use std::collections::HashMap;

use storage_dal::{Storage, StorageData};

struct ChainCounter {
    sent_failed_counter: GenericCounter<AtomicU64>,
    unavailable_counter: GenericCounter<AtomicU64>,
    observed_counter: GenericCounter<AtomicU64>,
}

pub async fn start(
    vr_receiver: Receiver<VerifiedResult>,
    storage: Storage,
    check_timeout: u64,
    chain_for_send: Vec<String>,
) {
    // sent_failed < unavailable < observed
    info!("metrics start observing");
    let mut chain_counter_map: HashMap<String, ChainCounter> = HashMap::new();
    for chain_name in chain_for_send {
        let sent_failed_counter = register_int_counter!(
            format!("{}_Sent_failed_Counter", chain_name.to_snake_case()),
            format!("SLA test sent failed counter(time) for {}", chain_name)
        )
        .unwrap();
        let unavailable_counter = register_int_counter!(
            format!("{}_Unavailable_Counter", chain_name.to_snake_case()),
            format!("SLA test unavailable counter(min) for {}", chain_name)
        )
        .unwrap();
        let observed_counter = register_int_counter!(
            format!("{}_Observed_Counter", chain_name.to_snake_case()),
            format!("SLA test total observed counter(min) for {}", chain_name)
        )
        .unwrap();
        recover_data(
            &sent_failed_counter,
            &unavailable_counter,
            &observed_counter,
            storage.clone(),
            check_timeout,
            chain_name.clone(),
        );
        chain_counter_map.insert(
            chain_name,
            ChainCounter {
                sent_failed_counter,
                unavailable_counter,
                observed_counter,
            },
        );
    }
    loop {
        if let Ok(vr) = vr_receiver.recv() {
            let ChainCounter {
                sent_failed_counter,
                unavailable_counter,
                observed_counter,
            } = chain_counter_map
                .get_mut(&vr.chain_name)
                .unwrap_or_else(|| {
                    panic!(
                        "chain_counter_map get failed, chain_name: {}",
                        vr.chain_name
                    )
                });
            observed_counter.inc();
            if vr.sent_failed_num != 0 {
                warn!(
                    "{} sent_failed, VerifiedResult key: {}",
                    get_readable_time_from_minute(vr.timestamp),
                    vr.timestamp
                );
                sent_failed_counter.inc();
                unavailable_counter.inc()
            } else if vr.failed_num != 0 {
                warn!(
                    "{} unavailable, VerifiedResult key: {}",
                    get_readable_time_from_minute(vr.timestamp),
                    vr.timestamp
                );
                unavailable_counter.inc()
            } else {
                info!(
                    "{} available, VerifiedResult key: {}",
                    get_readable_time_from_minute(vr.timestamp),
                    vr.timestamp
                );
            }
        }
    }
}

fn recover_data(
    sent_failed_counter: &GenericCounter<AtomicU64>,
    unavailable_counter: &GenericCounter<AtomicU64>,
    observed_counter: &GenericCounter<AtomicU64>,
    storage: Storage,
    check_timeout: u64,
    chain_name: String,
) {
    let finalized_minute = get_latest_finalized_minute(unix_now(), check_timeout);
    let (sent_failed, unavailable, observed) = storage
        .op
        .blocking()
        .lister(&format!(
            "STRUCTURED/{}/{}/",
            VerifiedResult::name(),
            chain_name
        ))
        .unwrap()
        .fold(
            (0, 0, 0),
            |(mut sent_failed, mut unavailable, mut observed), vr| {
                if let Ok(vr_entry) = vr {
                    if let Some(vr) = storage.get_by_path::<VerifiedResult>(vr_entry.path()) {
                        if vr.timestamp <= finalized_minute {
                            observed += 1;
                            if vr.sent_failed_num != 0 || vr.failed_num != 0 {
                                unavailable += 1;
                                if vr.sent_failed_num != 0 {
                                    sent_failed += 1;
                                }
                            };
                        }
                        return (sent_failed, unavailable, observed);
                    }
                }
                (0, 0, 0)
            },
        );
    sent_failed_counter.inc_by(sent_failed);
    unavailable_counter.inc_by(unavailable);
    observed_counter.inc_by(observed);
    info!(
        "recover metrics data before({}): {}, sent_failed: {}, unavailable: {}, observed: {}",
        chain_name,
        get_readable_time_from_minute(finalized_minute),
        sent_failed,
        unavailable,
        observed
    );
}

pub async fn run_metrics_exporter(port: u16, rx: Receiver<()>) -> Result<()> {
    let router = Router::new().push(Router::with_path("metrics").get(metrics));

    info!("metrics listening on 0.0.0.0:{port}");

    let acceptor = TcpListener::new(format!("0.0.0.0:{port}")).bind().await;

    let server = Server::new(acceptor);
    let handle = server.handle();
    tokio::spawn(async move {
        if let Ok(()) = rx.recv_async().await {
            handle.stop_graceful(None);
        }
    });
    server.serve(router).await;

    Ok(())
}

#[handler]
async fn metrics(res: &mut Response) {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    res.status_code(StatusCode::OK)
        .add_header(CONTENT_TYPE.as_str(), encoder.format_type(), true)
        .unwrap();
    res.write_body(buffer).unwrap();
}
