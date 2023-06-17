# dagster-prom-exporter

A project to learn rust. This is a prometheus exporter for the workflow-orchestrator [Dagster](https://dagster.io)'s internal system metrics, by querying the Dagit GraphQL API. The exporter reports instance-level metrics as well as metrics for the last seen runs of each pipeline (job/graph). Only runs which have reached a terminal state are reported (success, failure, canceled).

