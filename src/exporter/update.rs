use super::dagit_query::*;
use super::labels::*;
use super::metrics::Metrics;

impl Metrics {
    pub(super) fn set_run_metrics(&mut self, runs: DagitQueryRunsOrError) {
        use DagitQueryRunsOrError::Runs;
        use DagitQueryRunsOrErrorOnRunsResultsStats::RunStatsSnapshot;

        let Runs(r) = runs else { return };
        
        for run in r.results.into_iter() {

            if let Some(u) = run.update_time {
                if self.cursor < u {
                    self.cursor = u;
                }
            }

            let label = RunLabel::new(
                format!("{:?}", run.status),
                run.mode,
                CommonLabel::new(run.repository_origin, run.pipeline_name),
            );
            self.clear_old_run_states(&label);

            if let (Some(start), Some(end)) = (run.start_time, run.end_time) {
                self.run_duration_seconds.get_or_create(&label).set(end - start);
            }
            if let RunStatsSnapshot(stats) = run.stats {
                if let (Some(start), Some(end)) = (stats.enqueued_time, stats.launch_time) {
                    self.run_queue_seconds.get_or_create(&label).set(end - start);
                }
            }
            
            for step in run.step_stats.into_iter() {
                let label = label.step_label(step.step_key, step.status);
                self.clear_old_step_states(&label);

                self.step_attempts.get_or_create(&label).set(step.attempts.len() as i64);
                if let (Some(start), Some(end)) = (step.start_time, step.end_time) {
                    self.step_duration_seconds.get_or_create(&label).set(end - start);
                }
                for expectation in step.expectation_results.into_iter() {
                    let label = label.expectation_label(expectation.label);
                    self.expectation_success.get_or_create(&label).set(expectation.success as i64);
                }
            }

            for asset in run.asset_materializations.into_iter() {
                if let (Some(k), Ok(i)) = (asset.asset_key, asset.timestamp.parse::<f64>()) {
                    let label = label.asset_label(asset.step_key, k, asset.partition);
                    self.asset_materialization_timestamp.get_or_create(&label).set(i);
                }
            }
        }
    }

    pub(super) fn set_workspace_metrics(&self, workspaces: DagitQueryWorkspaceOrError) {
        use DagitQueryWorkspaceOrError::Workspace;
        use DagitQueryWorkspaceOrErrorOnWorkspaceLocationEntriesLocationOrLoadError::RepositoryLocation;
        
        let Workspace(w) = workspaces else { return };
        self.workspace_location_last_update_seconds.clear();
        
        for workspace in w.location_entries.into_iter() {
            self.workspace_location_last_update_seconds
                .get_or_create(&WorkspaceLocationLabel::new(&workspace))
                .set(workspace.updated_timestamp);

            let Some(RepositoryLocation(location)) = workspace.location_or_load_error else { return };
            self.runs_by_instigation_total.clear();
            
            for repo in location.repositories.into_iter() {

                for sensor in repo.sensors.into_iter() {
                    let label = InstigationLabel::new(
                        workspace.name.clone(),
                        location.name.clone(),
                        sensor.name,
                        format!("sensor_{:?}", sensor.sensor_type)
                    );
                    self.runs_by_instigation_total
                        .get_or_create(&label)
                        .set(sensor.sensor_state.runs_count);
                }

                for schedule in repo.schedules.into_iter() {
                    let label = InstigationLabel::new(
                        workspace.name.clone(),
                        location.name.clone(),
                        schedule.name,
                        format!("schedule_{}", schedule.mode)
                    );
                    self.runs_by_instigation_total
                        .get_or_create(&label)
                        .set(schedule.schedule_state.runs_count);
                }
            }
        }
    }

    pub(super) fn set_daemon_metrics(&self, daemons: DagitQueryInstanceDaemonHealth) {
        self.daemon_last_heartbeat_seconds.clear();
        for daemon in daemons.all_daemon_statuses.into_iter() {
            if let Some(heartbeat) = daemon.last_heartbeat_time {
                self.daemon_last_heartbeat_seconds
                    .get_or_create(&DaemonStatusLabel::new(daemon))
                    .set(heartbeat);
            }
        }
    }

    pub(super) fn set_concurrency_metrics(&self, concurrency: Vec<DagitQueryInstanceConcurrencyLimits>) {
        self.concurrency_slots.clear();
        self.concurrency_active_slots.clear();
        self.concurrency_pending_steps.clear();
        self.concurrency_assigned_steps.clear();

        for key in concurrency.into_iter() {
            let label = vec![("key".to_owned(), key.concurrency_key)];
            self.concurrency_slots.get_or_create(&label).set(key.slot_count);
            self.concurrency_active_slots.get_or_create(&label).set(key.active_slot_count);
            self.concurrency_pending_steps.get_or_create(&label).set(key.pending_step_count);
            self.concurrency_assigned_steps.get_or_create(&label).set(key.assigned_step_count);
        }
    }

    /// Clear out label sets for outdated run states
    fn clear_old_run_states(&self, label: &RunLabel) {
        use RunStatus::*;
        let variants = [
            QUEUED,
            NOT_STARTED,
            MANAGED,
            STARTING,
            STARTED,
            SUCCESS,
            FAILURE,
            CANCELING,
            CANCELED
        ];
        for status_variant in variants.map(|s| format!("{:?}", s)).into_iter() {
            if label.status != status_variant {
                let mut old_label = label.clone();
                old_label.status = status_variant;
                self.run_duration_seconds.remove(&old_label);
                self.run_queue_seconds.remove(&old_label);
            }
        }
    }

    fn clear_old_step_states(&self, label: &StepLabel) {
        use StepEventStatus::*;
        let variants = [Some(SKIPPED), Some(SUCCESS), Some(FAILURE), Some(IN_PROGRESS), None];

        for status_variant in variants.map(|v| v.map(|inner| format!("{:?}", inner))).into_iter() {
            if label.status != status_variant {
                let mut old_label = label.clone();
                old_label.status = status_variant;
                self.step_attempts.remove(&label);
                self.step_duration_seconds.remove(&label);
            }
        }
    }
}