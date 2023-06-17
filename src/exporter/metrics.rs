use prometheus_client::{registry::Registry, metrics::{family::Family, gauge::Gauge}};

use crate::exporter::float_gauge::GaugeF;

use super::labels::*;

pub(super) struct Metrics {
    pub(super) run_duration_seconds: GaugeF<RunLabel>,
    pub(super) run_queue_seconds: GaugeF<RunLabel>,
    pub(crate) runs_by_instigation_total: Family<InstigationLabel, Gauge>,

    pub(super) step_duration_seconds: GaugeF<StepLabel>,
    pub(super) step_attempts: Family<StepLabel, Gauge>,
    pub(super) expectation_success: Family<ExpectationLabel, Gauge>,
    pub(super) asset_materialization_timestamp: GaugeF<MaterializationLabel>,
    
    pub(super) daemon_last_heartbeat_seconds: GaugeF<DaemonStatusLabel>,
    pub(super) workspace_location_last_update_seconds: GaugeF<WorkspaceLocationLabel>,

    pub(super) concurrency_slots: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_active_slots: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_pending_steps: Family<Vec<(String, String)>, Gauge>,
    pub(super) concurrency_assigned_steps: Family<Vec<(String, String)>, Gauge>
}

impl Metrics {
    pub(super) fn new() -> Metrics {
         Metrics {
            run_duration_seconds: GaugeF::<RunLabel>::default(),
            run_queue_seconds: GaugeF::<RunLabel>::default(),
            runs_by_instigation_total: Family::<InstigationLabel, Gauge>::default(),
            step_duration_seconds: GaugeF::<StepLabel>::default(),
            step_attempts: Family::<StepLabel, Gauge>::default(),
            expectation_success: Family::<ExpectationLabel, Gauge>::default(),
            asset_materialization_timestamp: GaugeF::<MaterializationLabel>::default(),
            daemon_last_heartbeat_seconds: GaugeF::<DaemonStatusLabel>::default(),
            workspace_location_last_update_seconds: GaugeF::<WorkspaceLocationLabel>::default(),
            concurrency_slots: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_active_slots: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_pending_steps: Family::<Vec<(String, String)>, Gauge>::default(),
            concurrency_assigned_steps: Family::<Vec<(String, String)>, Gauge>::default()
        }
    }

    pub(super) fn registry(&self, concurrency_metrics: bool) -> Registry {
        let mut registry =  <Registry>::default();
        registry.register(
            "run_duration_seconds",
            "The total execution time of the latest Dagster runs",
            self.run_duration_seconds.clone()
        );
        registry.register(
            "run_queue_seconds",
            "The total queueing time of the latest Dagster runs",
            self.run_queue_seconds.clone()
        );
        registry.register(
            "runs_by_instigation_total",
            "The total number of runs triggered per instigator (schedule/sensor)",
            self.runs_by_instigation_total.clone()
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
            "expectation_success",
            "The latest Dagster runs' expectation results",
            self.expectation_success.clone()
        );
        registry.register(
            "asset_materialization_timestamp",
            "The time at which the latest Dagster runs' assets were materialized",
            self.asset_materialization_timestamp.clone()
        );
        registry.register(
            "daemon_last_heartbeat",
            "The last daemon heartbeat time reported to Dagit",
            self.daemon_last_heartbeat_seconds.clone()
        );
        registry.register(
            "workspace_location_last_update_seconds",
            "The last update time for the Dagster instance's registered workspaces",
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
        return registry;
    }
}
