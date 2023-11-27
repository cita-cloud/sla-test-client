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

mod client;
mod config;
mod metrics;
mod record;
mod time;

#[macro_use]
extern crate tracing as logger;

use anyhow::{Ok, Result};
use clap::Parser;
use cloud_util::graceful_shutdown::graceful_shutdown;
use common_rs::configure::{file_config, hot_reload};
use parking_lot::RwLock;
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};
use storage_dal::Storage;

use client::Client;
use config::Config;
use metrics::run_metrics_exporter;
use record::VerifiedResult;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// config file path
    #[arg(short, long, default_value = "config/client.toml")]
    config: String,
}

fn main() {
    ::std::env::set_var("RUST_BACKTRACE", "full");

    let rt = tokio::runtime::Runtime::new().unwrap();

    let args = Args::parse();
    let config: Config = file_config(&args.config).unwrap_or_default();

    // init tracer
    cloud_util::tracer::init_tracer("sla-client".to_owned(), &config.log_config)
        .map_err(|e| println!("tracer init err: {e}"))
        .unwrap();

    info!("{:?}", &args);
    info!("{:?}", &config);
    if let Err(err) = rt.block_on(start(config, args.config)) {
        error!("sla-client start err: {:?}", err);
    }
}

async fn start(config: Config, config_path: String) -> Result<()> {
    let graceful_shutdown_rx = graceful_shutdown();

    let storage = Storage::init_sled(&config.storage_path);
    let http_client = reqwest::ClientBuilder::default()
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(2))
        .build()?;

    let (vr_sender, vr_receiver) = mpsc::channel::<VerifiedResult>();

    let metrics_port = config.metrics_port;
    tokio::spawn(crate::metrics::start(
        vr_receiver,
        storage.clone(),
        config.validator_timeout,
        config.chain_for_send.clone(),
    ));
    tokio::spawn(run_metrics_exporter(metrics_port));

    let mut sender_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.sender_interval));
    let mut validator_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(config.validator_interval));

    let config = Arc::new(RwLock::new(config));

    hot_reload(config.clone(), config_path).await?;

    let client = Client {
        config,
        storage: storage.clone(),
        http_client,
        vr_sender,
    };

    loop {
        tokio::select! {
            _ = sender_interval.tick() => {
                client.sender().await;
            },
            _ = validator_interval.tick() => {
                client.validator().await;
            }
            _ = graceful_shutdown_rx.recv_async() => {
                info!("graceful_shutdown");
                break;
            }
        }
    }
    Ok(())
}
