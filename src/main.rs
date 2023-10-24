use anyhow::{anyhow, Result};
use clap::Parser;
use exporter::Exporter;
use hyper::http::{Method, StatusCode};
use hyper::service::service_fn;
use hyper::server::conn::Http;
use hyper::{Body, Response};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::rc::Rc;
use std::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, interval};

mod exporter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local, single-threaded execution
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let executor = tokio::task::LocalSet::new();
    executor.block_on(&rt, serve())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr: SocketAddr = (args.host, args.port).into();
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);

    let mut timer = interval(Duration::from_secs(args.refresh));
    let refresh = Rc::new(RwLock::new(true));
    let refresh_timer = Rc::clone(&refresh);

    tokio::task::spawn_local(async move {
        loop {
            timer.tick().await;
            let mut b = refresh.write().unwrap();
            *b = true;
        }
    });

    let exporter = Rc::new(Exporter::new(args.dagit_url, refresh_timer));

    loop {
        let (stream, _) = listener.accept().await?;
        let exporter = Rc::clone(&exporter);

        tokio::task::spawn_local(async move {
            if let Err(e) = metrics_handler(stream, exporter).await {
                eprintln!("Error serving connection: {:?}", e);
            }
        });
    }
}

async fn metrics_handler(stream: TcpStream, exporter: Rc<exporter::Exporter>) -> Result<(), hyper::Error> {
    Http::new()
        .with_executor(LocalExec)
        .http1_only(true)
        .serve_connection(stream, service_fn(|req| {

            let resp = Response::builder();
            let exporter = Rc::clone(&exporter);
            async move {
                if req.method() != Method::GET || req.uri() != "/metrics" {
                    return resp
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("only GET requests on the /metrics route are supported"));
                }

                if let Err(e) = exporter.query().await {
                    return resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into());
                }

                match exporter.encode() {
                    Ok(b) => resp.body(b.into()),
                    Err(e) => resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into())
                }
            }
        }))
        .await
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


#[derive(Clone)]
struct LocalExec;

impl<F: std::future::Future + 'static> hyper::rt::Executor<F> for LocalExec {
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}
