use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use graphql_client::GraphQLQuery;
use graphql_client::reqwest::post_graphql;
use prometheus_client::encoding::text::encode as prom_encode;
use prometheus_client::registry::Registry;
use reqwest::Client;

use core::cell::RefCell;
use core::fmt;
use std::rc::Rc;

mod metrics;
mod update;
mod labels;
mod float_gauge;

use metrics::Metrics;

pub struct Exporter {
    url: String,
    metrics: RefCell<Metrics>,
    registry: Registry,
    client: Client,
    refresh: Rc<RefCell<bool>>,
}

impl Exporter {
    pub fn new(url: String, refresh: Rc<RefCell<bool>>, concurrency_metrics: bool) -> Self {
        let metrics = Metrics::new(concurrency_metrics);
        let registry = metrics.registry();
        Self {
            url,
            metrics: RefCell::new(metrics),
            registry,
            client: Client::builder().user_agent("prometheus-exporter/0.1.0").build().expect("http client"),
            refresh,
        }
    }

    pub async fn query(&self) -> Result<()> {
        if !*self.refresh.borrow() {
            return Ok(())
        }

        let vars = dagit_query::Variables{
            concurrency_metrics: true,
            runs_since: self.metrics.borrow().cursor,
        };

        let resp = post_graphql::<DagitQuery, _>(&self.client, &self.url, vars)
            .await
            .map_err(anyhow::Error::msg)?
            .data.ok_or_else(|| anyhow!("empty data"))?;

        let mut m = self.metrics.borrow_mut();
        m.set_run_metrics(resp.runs_or_error);
        m.set_workspace_metrics(resp.workspace_or_error);
        m.set_daemon_metrics(resp.instance.daemon_health);

        if true {
            m.set_concurrency_metrics(resp.instance.concurrency_limits)
        }

        *self.refresh.borrow_mut() = false;
        Ok(())
    }

    pub fn encode(&self) -> Result<Bytes, fmt::Error> {
        let mut buffer = BytesMut::new();
        prom_encode(&mut buffer, &self.registry).map(|_| buffer.freeze())
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "graphql/dagit_query.graphql",
    schema_path = "graphql/dagit_schema.graphql",
    response_derives = "Debug,PartialEq",
)]
struct DagitQuery;
