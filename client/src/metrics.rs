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
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use log::info;
use prometheus::{gather, register_histogram, Encoder, TextEncoder};
use std::convert::Infallible;
use std::sync::mpsc::Receiver;

pub async fn start(vr_receiver: Receiver<VerifiedResult>) {
    info!("metrics start observing");
    let histogram = register_histogram!("SLA_test", "SLA test", vec![0.5],).unwrap();
    loop {
        let vr = vr_receiver.recv().unwrap();
        if vr.success_num != vr.sent_num {
            info!("{} failed", &vr.timestamp);
            histogram.observe(0.0)
        } else {
            info!("{} succeed", &vr.timestamp);
            histogram.observe(1.0)
        }
    }
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
