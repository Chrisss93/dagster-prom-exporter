//!
//! Comprehensive integration testing for the dagster-prometheus-exporter against an instance
//! of a dagit webserver and live dagster pipelines by way of testcontainers and docker.
//! Access to a local or remote docker daemon is required but not the docker binary client itself.
//!

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod end_to_end;

use end_to_end::end_to_end;

use tokio::task::LocalSet;

use std::env;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "expensive"]
async fn end_to_end_test() {
    let image = env::var("TESTCONTAINER_IMAGE").unwrap_or_else(|_| "dagster:testcontainers".to_owned());
    let (name, tag) = match image.rsplit_once(':') {
        Some((x, y)) => (x, y),
        None => (image.as_str(), "latest")
    };
    LocalSet::new().run_until(async { end_to_end(name, tag).await }).await
}
