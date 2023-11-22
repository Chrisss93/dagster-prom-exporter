use dagster_prom_exporter::serve;

use anyhow::{anyhow, Result};
use clap::Parser;
use tokio::task::LocalSet;

use std::net::{IpAddr, Ipv6Addr};

// TODO: Replace RwLock with the cheaper RefCell since we are in a single-threaded environment.

fn main() -> anyhow::Result<()> {
    // Local, single-threaded execution
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let args = Args::parse();
    LocalSet::new().block_on(&rt, serve(args.dagit_url, args.host, args.port, args.refresh))
}

#[derive(Parser)]
struct Args {
    /// The url for the Dagster deployment's Dagit GraphQL API
    #[arg(value_parser = valid_url)]
    dagit_url: String,

    /// The network host on which to expose prometheus metrics
    #[arg(
        short = 'a', long = "listener-host",
        value_parser = |s: &str| s.parse::<IpAddr>(),
        default_value_t = IpAddr::V6(Ipv6Addr::UNSPECIFIED)
    )]
    host: IpAddr,

    /// The port on which to expose prometheus metrics
    #[arg(short = 'p', long = "listener-port", default_value_t = 3001)]
    port: u16,

    /// How many seconds the exporter should serve old metrics before re-querying the Dagit GraphQL API
    #[arg(short, long, default_value_t = 5)]
    refresh: u64
}

fn valid_url(s: &str) -> Result<String> {
    match url::Url::parse(s) {
        Err(e) => Err(e.into()),
        Ok(u) if u.has_host() => Ok(u.into()),
        _ => Err(anyhow!("missing a url scheme")),
    }
}
