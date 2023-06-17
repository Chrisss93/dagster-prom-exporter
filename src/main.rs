use anyhow::{anyhow, Result};
use clap::Parser;
use hyper::http::{Method, StatusCode};
use hyper::service::service_fn;
use hyper::server::conn::Http;
use hyper::{Body, Response};
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr};
use std::rc::Rc;
use std::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};

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

    let metrics = Rc::new(RwLock::new(exporter::Exporter::new(args.dagit_url)));

    loop {
        let (stream, _) = listener.accept().await?;
        let metrics = Rc::clone(&metrics);

        tokio::task::spawn_local(async move {
            if let Err(e) = handler(stream, metrics).await {
                eprintln!("Error serving connection: {:?}", e);
            }
        });
    }
}

async fn handler(stream: TcpStream, metrics: Rc<RwLock<exporter::Exporter>>) -> Result<(), hyper::Error> {
    Http::new()
        .with_executor(LocalExec)
        .http1_only(true)
        .serve_connection(stream, service_fn(move |req| {

            let metrics = Rc::clone(&metrics);
            let resp = Response::builder();
            async move {
                if req.method() != Method::GET && req.uri() != "/metrics" {
                    return resp
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("only GET requests on the /metrics route are supported"));
                }

                match metrics.write() {
                    Err(e) => return resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into()),
                    Ok(mut x) => if let Err(e) = x.query().await {
                        return resp.status(StatusCode::NOT_FOUND).body(e.to_string().into());
                    }
                };

                 match metrics.read().map(|x| x.encode()) {
                    Ok(Ok(b)) => resp.body(b.into()),
                    Ok(Err(e)) => resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into()),
                    Err(e) => resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into())
                }
            }
        }))
        .await
}


const LOCAL_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

#[derive(Parser)]
struct Args {
    /// The url for the Dagster deployment's Dagit GraphQL API
    #[arg(value_parser = valid_url)]
    dagit_url: String,

    /// The network host on which to expose prometheus metrics
    #[arg(short = 'a', long = "listener-host", value_parser = valid_ip, default_value_t = LOCAL_ADDR)]
    host: IpAddr,
    /// The port on which to expose prometheus metrics
    #[arg(short = 'p', long = "listener-port", default_value_t = 3001)]
    port: u16,
}

fn valid_url(s: &str) -> Result<String> {
    match url::Url::parse(s) {
        Err(e) => Err(e.into()),
        Ok(u) if u.has_host() => Ok(u.into()),
        _ => Err(anyhow!("missing a url scheme")),
    }
}

fn valid_ip(s: &str) -> Result<IpAddr, AddrParseError> {
    s.parse::<IpAddr>()
}

#[derive(Clone)]
struct LocalExec;

impl<F: std::future::Future + 'static> hyper::rt::Executor<F> for LocalExec {
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}