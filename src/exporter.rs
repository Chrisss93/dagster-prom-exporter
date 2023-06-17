use anyhow::Result;
use bytes::{Bytes, BytesMut};
use graphql_client::GraphQLQuery;
use graphql_client::reqwest::post_graphql;
use prometheus_client::encoding::text::encode as prom_encode;
use prometheus_client::registry::Registry;
use reqwest::Client;

use std::time::{UNIX_EPOCH, SystemTime};

mod metrics;
mod update;
mod labels;
mod float_gauge;

pub(crate) struct Exporter {
    url: String,
    metrics: metrics::Metrics,
    registry: Registry,
    client: Client,
    cursor: f64
}

impl Exporter {
    pub(crate) fn new(url: String) -> Exporter {
        let metrics = metrics::Metrics::new();
        let registry = metrics.registry(true);
        Exporter{
            url: url,
            metrics: metrics,
            registry: registry,
            client: Client::builder().user_agent("prometheus-exporter/0.1.0").build().expect("http client"),
            cursor: SystemTime::now().duration_since(UNIX_EPOCH).expect("clock time").as_secs_f64()
        }
    }

    pub(crate) async fn query(&mut self) -> Result<()> {
        let vars = dagit_query::Variables{
            concurrency_metrics: true,
            runs_since: self.cursor,
        };

        let resp = post_graphql::<DagitQuery, _>(&self.client, &self.url, vars)
            .await
            .map_err(anyhow::Error::msg)?
            .data.ok_or(anyhow::Error::msg("empty data"))?;

        self.set_run_metrics(resp.runs_or_error);
        self.set_workspace_metrics(resp.workspace_or_error);
        self.set_daemon_metrics(resp.instance.daemon_health);

        if true {
            self.set_concurrency_metrics(resp.instance.concurrency_limits)
        }
        return Ok(());
    }

    pub(crate) fn encode(&self) -> Result<Bytes, std::fmt::Error> {
        let mut buffer = BytesMut::new();
        prom_encode(&mut buffer, &self.registry).map(|_| buffer.freeze())
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "graphql/dagit_query.graphql",
    // schema_path = "graphql/dagit_introspection_schema.json",
    schema_path = "graphql/dagit_schema.graphql",
    response_derives = "Debug,PartialEq",
)]
struct DagitQuery;