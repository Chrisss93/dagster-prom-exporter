use prometheus_client::registry::{Registry, Unit};
use prometheus_client::metrics::{family::Family, gauge::Gauge, counter::Counter};

use std::sync::atomic::AtomicU64;
use std::time::{SystemTime, UNIX_EPOCH};

use super::float_gauge::GaugeF;
use super::labels::*;

pub(super) struct Metrics {
    pub(super) cursor: f64,

    pub(super) run_total: Family<RunLabel, Counter>,
    pub(super) run_duration_seconds: GaugeF<RunLabel>,
    pub(super) run_queue_seconds: GaugeF<RunLabel>,
    pub(super) runs_by_instigation_total: Family<InstigationLabel, Gauge>,

    pub(super) step_total: Family<StepLabel, Counter>,
    pub(super) step_duration_seconds: GaugeF<StepLabel>,
    pub(super) step_attempts: Family<StepLabel, Gauge>,
    pub(super) expectation_failure: Family<ExpectationLabel, Gauge>,
    pub(super) asset_materialization_timestamp: GaugeF<MaterializationLabel>,
    
    pub(super) daemon_last_heartbeat_seconds: GaugeF<DaemonStatusLabel>,
    pub(super) workspace_location_last_update_seconds: GaugeF<WorkspaceLocationLabel>,

    pub(super) concurrency_slots: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_active_slots: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_pending_steps: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_assigned_steps: Family<Vec<(String, String)>, Gauge>,

    pub(super) exporter_last_scrape_runs: Gauge,
    pub(super) exporter_last_scrape_timestamp: Gauge<f64, AtomicU64>,
}

impl Metrics {
    pub(super) fn new() -> Metrics {
         Metrics {
            cursor: SystemTime::now().duration_since(UNIX_EPOCH).expect("clock time").as_secs_f64(),

            run_total: Family::<RunLabel, Counter>::default(),
            run_duration_seconds: GaugeF::<RunLabel>::default(),
            run_queue_seconds: GaugeF::<RunLabel>::default(),
            runs_by_instigation_total: Family::<InstigationLabel, Gauge>::default(),

            step_total: Family::<StepLabel, Counter>::default(),
            step_duration_seconds: GaugeF::<StepLabel>::default(),
            step_attempts: Family::<StepLabel, Gauge>::default(),
            expectation_failure: Family::<ExpectationLabel, Gauge>::default(),
            asset_materialization_timestamp: GaugeF::<MaterializationLabel>::default(),

            daemon_last_heartbeat_seconds: GaugeF::<DaemonStatusLabel>::default(),
            workspace_location_last_update_seconds: GaugeF::<WorkspaceLocationLabel>::default(),

            concurrency_slots: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_active_slots: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_pending_steps: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_assigned_steps: Family::<Vec<(String, String)>, Gauge>::default(),

            exporter_last_scrape_runs: Gauge::default(),
            exporter_last_scrape_timestamp: Gauge::<f64, AtomicU64>::default(),
        }
    }

    pub(super) fn registry(&self, concurrency_metrics: bool) -> Registry {
        let mut registry =  <Registry>::default();
        registry.register(
            "run",
            "The cumulative total number of runs since the exporter was started",
            self.run_total.clone()
        );
        registry.register_with_unit(
            "run_duration",
            "The total execution time of the latest Dagster runs",
            Unit::Seconds,
            self.run_duration_seconds.clone()
        );
        registry.register_with_unit(
            "run_queue_seconds",
            "The total queueing time of the latest Dagster runs",
            Unit::Seconds,
            self.run_queue_seconds.clone()
        );
        registry.register(
            "runs_by_instigation_total",
            "The total number of runs triggered per instigator (schedule/sensor)",
            self.runs_by_instigation_total.clone()
        );
        registry.register(
            "step",
            "The cumulative total number of steps since the exporter was started",
            self.step_total.clone()
        );
        registry.register(
            "step_duration_seconds",
            "The individual execution time of the latest Dagster runs' steps",
            self.step_duration_seconds.clone()
        );
        registry.register(
            "step_attempts",
            "The number of attempts for the latest Dagster runs' steps",
            self.step_attempts.clone()
        );
        registry.register(
            "expectation_failure",
            "The value of this metric is 1 if the expectation is currently failing",
            self.expectation_failure.clone()
        );
        registry.register_with_unit(
            "asset_materialization_latest",
            "The unix seconds of an asset's latest materialization",
            Unit::Seconds,
            self.asset_materialization_timestamp.clone()
        );
        registry.register_with_unit(
            "daemon_last_heartbeat",
            "The last daemon heartbeat time reported to Dagit",
            Unit::Seconds,
            self.daemon_last_heartbeat_seconds.clone()
        );
        registry.register_with_unit(
            "workspace_location_last_update",
            "The last update time for the Dagster instance's registered workspaces",
            Unit::Seconds,
            self.workspace_location_last_update_seconds.clone()
        );
        if concurrency_metrics {
            registry.register(
                "concurrency_slots",
                "The total number of tagged concurrency slots available for the Dagster instance",
                self.concurrency_slots.clone()
            );
            registry.register(
                "concurrency_active_slots",
                "The number of active tagged concurrency slots used by the Dagster instance",
                self.concurrency_active_slots.clone()
            );
            registry.register(
                "concurrency_pending_steps",
                "The number of pending steps per concurrency tag for the Dagster instance",
                self.concurrency_pending_steps.clone()
            );
            registry.register(
                "concurrency_assigned_steps",
                "The number of assigned steps per concurrency tag for the Dagster instance",
                self.concurrency_assigned_steps.clone()
            );
        }
        registry.register(
            "exporter_last_scrape_runs",
            "The number of runs collected by the exporter's last query to Dagit",
            self.exporter_last_scrape_runs.clone()
        );
        registry.register(
            "exporter_last_scrape_timestamp",
            "The timestamp of the exporter's last query to Dagit",
            self.exporter_last_scrape_timestamp.clone()
        );
        return registry;
    }
}
