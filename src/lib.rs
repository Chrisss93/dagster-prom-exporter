mod exporter;

use exporter::Exporter;

use anyhow::Result;
use hyper::http::{Method, StatusCode};
use hyper::service::service_fn;
use hyper::server::conn::Http;
use hyper::{Body, Response};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::spawn_local;
use tokio::time::{Duration, interval};

use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::RwLock;


pub async fn serve(url: String, host: IpAddr, port: u16, refresh: u64) -> anyhow::Result<()> {
    let addr: SocketAddr = (host, port).into();
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);

    let mut timer = interval(Duration::from_secs(refresh));
    let refresh = Rc::new(RwLock::new(true));
    let refresh_timer = Rc::clone(&refresh);

    spawn_local(async move {
        loop {
            timer.tick().await;
            let mut b = refresh.write().unwrap();
            *b = true;
        }
    });

    let exporter = Rc::new(Exporter::new(url, refresh_timer));

    loop {
        let (stream, _) = listener.accept().await?;
        let exporter = Rc::clone(&exporter);

        spawn_local(async move {
            if let Err(e) = metrics_handler(stream, exporter).await {
                eprintln!("Error serving connection: {:?}", e);
            }
        });
    }
}

const OPENMETRICS_CONTENT: &str = "application/openmetrics-text; version=1.0.0; charset=utf-8";

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
                    Ok(b) => resp.header("Content-Type", OPENMETRICS_CONTENT) .body(b.into()),
                    Err(e) => resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into())
                }
            }
        }))
        .await
}


#[derive(Clone)]
struct LocalExec;

impl<F: std::future::Future + 'static> hyper::rt::Executor<F> for LocalExec {
    fn execute(&self, fut: F) {
        spawn_local(fut);
    }
}
