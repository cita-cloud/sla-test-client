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
use common::toml::read_toml;
use config::Config;
use log::info;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// config file path
    #[arg(short, long, default_value = "config/client.toml")]
    config: String,
}

fn main() {
    ::std::env::set_var("RUST_BACKTRACE", "full");
    let args = Args::parse();
    let config: Config = read_toml(&args.config).unwrap_or_default();

    // init log4rs
    log4rs::init_file(&config.log_file, Default::default())
        .map_err(|e| println!("log init err: {}", e))
        .ok();

    info!("{:?}", &args);
    info!("{:?}", &config);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(client::start(&config));
}
