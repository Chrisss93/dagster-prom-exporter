mod exporter;

use exporter::Exporter;

use anyhow::Result;
use hyper::http::{Method, StatusCode};
use hyper::rt::Executor;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Response};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::spawn_local;
use tokio::time::{interval, Duration};

use std::cell::RefCell;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;

pub async fn serve(url: String, host: IpAddr, port: u16, refresh_secs: u64, concurrency_metrics: bool) -> Result<()> {
    let addr: SocketAddr = (host, port).into();
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("Listening on {addr}");

    let mut timer = interval(Duration::from_secs(refresh_secs));
    let refresh = Rc::new(RefCell::new(true));
    let refresh_timer = Rc::clone(&refresh);

    spawn_local(async move {
        loop {
            timer.tick().await;
            *refresh.borrow_mut() = true;
        }
    });

    let exporter = Rc::new(Exporter::new(url, refresh_timer, concurrency_metrics));

    loop {
        let (stream, _) = listener.accept().await?;
        let exporter = Rc::clone(&exporter);

        spawn_local(async move {
            if let Err(e) = metrics_handler(stream, exporter).await {
                eprintln!("Error serving connection: {e}");
            }
        });
    }
}

const OPENMETRICS_CONTENT: &str = "application/openmetrics-text; version=1.0.0; charset=utf-8";

async fn metrics_handler(stream: TcpStream, exporter: Rc<exporter::Exporter>) -> Result<(), hyper::Error> {
    Http::new()
        .with_executor(LocalExec)
        .http1_only(true)
        .serve_connection(
            stream,
            service_fn(|req| {
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
                        Ok(b) => resp.header("Content-Type", OPENMETRICS_CONTENT).body(b.into()),
                        Err(e) => resp.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into())
                    }
                }
            })
        )
        .await
}


#[derive(Clone)]
struct LocalExec;

impl<F: Future + 'static> Executor<F> for LocalExec {
    fn execute(&self, fut: F) {
        spawn_local(fut);
    }
}
