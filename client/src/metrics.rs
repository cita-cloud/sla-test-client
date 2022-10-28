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
use common::time::{get_latest_finalized_minute, unix_now};
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use log::info;
use prometheus::{
    core::{AtomicU64, GenericCounter},
    gather, register_int_counter, Encoder, TextEncoder,
};
use std::convert::Infallible;
use std::sync::mpsc::Receiver;
use storage::{sledb::SledStorage, Storage};

pub async fn start(
    vr_receiver: Receiver<VerifiedResult>,
    storage: SledStorage,
    check_timeout: u32,
    chain_block_interval: u32,
) {
    info!("metrics start observing");
    let unavailable_counter =
        register_int_counter!("Unavailable_Counter", "SLA test unavailable counter(min)").unwrap();
    let observed_counter =
        register_int_counter!("Observed_Counter", "SLA test total observed counter(min)").unwrap();
    recover_data(
        &unavailable_counter,
        &observed_counter,
        storage,
        check_timeout,
        chain_block_interval,
    );
    loop {
        if let Ok(vr) = vr_receiver.recv() {
            observed_counter.inc();
            if vr.failed_num != 0 {
                info!("{} unavailable", &vr.timestamp);
                unavailable_counter.inc()
            } else {
                info!("{} available", &vr.timestamp);
            }
        }
    }
}

fn recover_data(
    unavailable_counter: &GenericCounter<AtomicU64>,
    observed_counter: &GenericCounter<AtomicU64>,
    storage: SledStorage,
    check_timeout: u32,
    chain_block_interval: u32,
) {
    let finalized_minute =
        get_latest_finalized_minute(unix_now(), check_timeout, chain_block_interval);
    let (unavailable, observed) = storage.all::<VerifiedResult>().iter().fold(
        (0, 0),
        |(mut unavailable, mut observed), vr| {
            if vr.timestamp <= finalized_minute {
                observed += 1;
                if vr.failed_num != 0 {
                    unavailable += 1;
                }
            }
            (unavailable, observed)
        },
    );
    unavailable_counter.inc_by(unavailable);
    observed_counter.inc_by(observed);
    info!(
        "recover metrics data before minute: {}, unavailable: {}, observed: {}",
        finalized_minute, unavailable, observed
    );
}

pub async fn run_metrics_exporter(
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let make_svc =
        make_service_fn(move |_conn| async move { Ok::<_, Infallible>(service_fn(serve_req)) });
    let addr = ([0, 0, 0, 0], port).into();
    let server = Server::bind(&addr).serve(make_svc);
    info!("export metrics to {}", addr.to_string());
    server.await?;
    Ok(())
}

async fn serve_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = gather();
            encoder.encode(&metric_families, &mut buffer).unwrap();

            Response::builder()
                .status(200)
                .header(CONTENT_TYPE, encoder.format_type())
                .body(Body::from(buffer))
                .unwrap()
        }
        _ => Response::builder()
            .status(404)
            .body(Body::from(
                "
                default:\n
                /61616/metrics for sla-test-client\n
                ",
            ))
            .unwrap(),
    };

    Ok(response)
}
