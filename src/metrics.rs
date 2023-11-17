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

use anyhow::Result;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use common_rs::restful::RESTfulError;
use prometheus::{
    core::{AtomicU64, GenericCounter},
    gather, register_int_counter, Encoder, TextEncoder,
};
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::mpsc::Receiver;
use storage_dal::Storage;

pub async fn start(vr_receiver: Receiver<VerifiedResult>, storage: Storage, check_timeout: u64) {
    // sent_failed < unavailable < observed
    info!("metrics start observing");
    let sent_failed_counter =
        register_int_counter!("Sent_failed_Counter", "SLA test sent failed counter(time)").unwrap();
    let unavailable_counter =
        register_int_counter!("Unavailable_Counter", "SLA test unavailable counter(min)").unwrap();
    let observed_counter =
        register_int_counter!("Observed_Counter", "SLA test total observed counter(min)").unwrap();
    recover_data(
        &sent_failed_counter,
        &unavailable_counter,
        &observed_counter,
        storage,
        check_timeout,
    );
    loop {
        if let Ok(vr) = vr_receiver.recv() {
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
) {
    let finalized_minute = get_latest_finalized_minute(unix_now(), check_timeout);
    let (sent_failed, unavailable, observed) = storage.scan::<VerifiedResult>().fold(
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
        "recover metrics data before: {}, sent_failed: {}, unavailable: {}, observed: {}",
        get_readable_time_from_minute(finalized_minute),
        sent_failed,
        unavailable,
        observed
    );
}

pub async fn run_metrics_exporter(port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let app = Router::new()
        .route("/metrics", get(metrics))
        .fallback(|| async {
            debug!("Not Found");
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "code": 404,
                    "message": "Not Found",
                })),
            )
        });

    info!("metrics listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        // .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("axum serve failed: {e}"))
}

async fn metrics() -> Result<impl IntoResponse, RESTfulError> {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(Response::builder()
        .status(200)
        .header(CONTENT_TYPE, encoder.format_type())
        .body(Body::from(buffer))
        .unwrap())
}
