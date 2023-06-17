# dagster-prom-exporter

A project to learn rust. This is a prometheus exporter for the workflow-orchestrator [Dagster](https://dagster.io)'s internal system metrics, by querying the Dagit GraphQL API. The exporter reports instance-level metrics as well as metrics for the last seen runs of each pipeline (job/graph). Only runs which have reached a terminal state are reported (success, failure, canceled).

## Issues

### Stale metrics

The exporter isn't great at responding to changes in job definitions after it initially begins exporting that job's metrics. This can be amended by restarting the exporter, thus clearing its local state, but that's not a great solution. For example, if a job is deleted/renamed or steps/ops are deleted/renamed or expectation-result/asset-materialization labels are deleted/renamed, all existing metrics with their labels are still shown by the exporter. `prometheus_client` exposes methods to delete metrics for a specific fully-specified labels or for every label, but I need the middle-ground to delete metrics for a partially-specified label.

