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

use clap::Parser;
use log::info;
use std::{path::Path, sync::mpsc};

use client::Client;
use common::toml::calculate_md5;
use common::{signal::handle_signals, toml::read_toml};
use config::Config;
use metrics::run_metrics_exporter;
use record::VerifiedResult;
use storage::sledb::SledStorage;

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
    rt.spawn(handle_signals());

    let args = Args::parse();
    let config: Config = read_toml(&args.config).unwrap_or_default();

    // init log4rs
    log4rs::init_file(&config.log_file, Default::default())
        .map_err(|e| println!("log init err: {e}"))
        .ok();

    info!("{:?}", &args);
    info!("{:?}", &config);
    rt.block_on(start(config, &args.config));
}

async fn start(config: Config, config_path: impl AsRef<Path> + Clone) {
    let storage = SledStorage::init(&config.storage_path);
    let http_client = reqwest::ClientBuilder::default().build().unwrap();

    let (vr_sender, vr_receiver) = mpsc::channel::<VerifiedResult>();

    let mut client = Client {
        config,
        storage: storage.clone(),
        http_client,
        vr_sender,
    };

    let metrics_port = client.config.metrics_port;
    tokio::spawn(crate::metrics::start(
        vr_receiver,
        storage.clone(),
        client.config.validator_timeout,
        client.config.chain_block_interval,
    ));
    tokio::spawn(run_metrics_exporter(metrics_port));

    let mut config_md5 = calculate_md5(&config_path).unwrap();

    let mut sender_interval = tokio::time::interval(tokio::time::Duration::from_secs(
        client.config.sender_interval,
    ));
    let mut validator_interval = tokio::time::interval(tokio::time::Duration::from_secs(
        client.config.validator_interval,
    ));
    let mut hot_update_interval = tokio::time::interval(tokio::time::Duration::from_secs(
        client.config.hot_update_interval,
    ));

    loop {
        tokio::select! {
            _ = sender_interval.tick() => {
                client.sender().await;
            },
            _ = validator_interval.tick() => {
                client.validator().await;
            }
            _ = hot_update_interval.tick() => {
                client.hot_update(&mut config_md5, config_path.clone());
            }
        }
    }
}
