use anyhow::Result;
use bytes::{Bytes, BytesMut};
use graphql_client::GraphQLQuery;
use graphql_client::reqwest::post_graphql;
use prometheus_client::encoding::text::encode as prom_encode;
use prometheus_client::registry::Registry;
use reqwest::Client;

use std::rc::Rc;
use std::sync::{RwLock, PoisonError};

mod metrics;
mod update;
mod labels;
mod float_gauge;

pub struct Exporter {
    url: String,
    metrics: RwLock<metrics::Metrics>,
    registry: Registry,
    client: Client,
    refresh: Rc<RwLock<bool>>,
}

impl Exporter {
    pub fn new(url: String, refresh: Rc<RwLock<bool>>) -> Exporter {
        let metrics = metrics::Metrics::new();
        let registry = metrics.registry(true);
        Exporter{
            url: url,
            metrics: RwLock::new(metrics),
            registry: registry,
            client: Client::builder().user_agent("prometheus-exporter/0.1.0").build().expect("http client"),
            refresh: refresh,
        }
    }

    pub async fn query(&self) -> Result<()> {
        if !*self.refresh.read().map_err(Exporter::poisoned)? {
            return Ok(())
        }

        let mut m = self.metrics.write().map_err(Exporter::poisoned)?;

        let vars = dagit_query::Variables{
            concurrency_metrics: true,
            runs_since: m.cursor,
        };

        let resp = post_graphql::<DagitQuery, _>(&self.client, &self.url, vars)
            .await
            .map_err(anyhow::Error::msg)?
            .data.ok_or(anyhow::Error::msg("empty data"))?;

        m.set_run_metrics(resp.runs_or_error);
        m.set_workspace_metrics(resp.workspace_or_error);
        m.set_daemon_metrics(resp.instance.daemon_health);

        if true {
            m.set_concurrency_metrics(resp.instance.concurrency_limits)
        }

        let mut b = self.refresh.write().map_err(Exporter::poisoned)?;
        *b = false;

        return Ok(());
    }

    pub fn encode(&self) -> Result<Bytes, std::fmt::Error> {
        let mut buffer = BytesMut::new();
        prom_encode(&mut buffer, &self.registry).map(|_| buffer.freeze())
    }

    fn poisoned<T>(e: PoisonError<T>) -> anyhow::Error {
        anyhow::Error::msg(e.to_string())
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