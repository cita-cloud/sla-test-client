mod client;
mod config;
mod record;

use clap::Parser;
use common::{log::set_log, toml::read_toml};
use config::Config;
use tracing::info;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// config file path
    #[arg(short, long, default_value = "config/client.toml")]
    config: String,
}

fn main() {
    let args = Args::parse();
    let config: Config = read_toml(&args.config);
    set_log(&config.log_filter);

    info!("{:?}", &args);
    info!("{:?}", &config);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(client::start(&config));
}
