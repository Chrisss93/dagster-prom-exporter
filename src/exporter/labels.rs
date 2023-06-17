use prometheus_client::encoding::EncodeLabelSet;
use super::dagit_query::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct CommonLabel {
    workspace_location: Option<String>,
    repository_name: Option<String>,
    pipeline_name: String
}

impl CommonLabel {
    pub(super) fn new(repo: Option<DagitQueryRunsOrErrorOnRunsResultsRepositoryOrigin>, job: String) -> CommonLabel {
        match repo {
            Some(r) => CommonLabel {
                workspace_location: Some(r.repository_location_name),
                repository_name: Some(r.repository_name),
                pipeline_name: job
            },
            None => CommonLabel {
                workspace_location: None,
                repository_name: None,
                pipeline_name: job
            },
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct RunLabel {
  pub(super) status: String,
  mode: String,
  #[prometheus(flatten)]
  common: CommonLabel,
}

impl RunLabel {
    pub(super) fn new(status: String, mode: String, common: CommonLabel) -> RunLabel {
        RunLabel {status, mode, common}
    }
    pub (super) fn step_label(&self, step_key: String, status: Option<StepEventStatus>) -> StepLabel {
        StepLabel {
            common: self.common.clone(),
            step_key: step_key,
            status: status.map(|x| format!("{:?}", x))
        }
    }
    pub(super) fn asset_label(&self,
        step_key: Option<String>,
        asset_key: DagitQueryRunsOrErrorOnRunsResultsAssetMaterializationsAssetKey,
        partition: Option<String>) -> MaterializationLabel {
        MaterializationLabel {
            common: self.common.clone(),
            step_key: step_key,
            asset_key: asset_key.path.join("/"),
            partition: partition
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct StepLabel {
  step_key: String,
  pub(super) status: Option<String>,
  #[prometheus(flatten)]
  common: CommonLabel
}

impl StepLabel {
    pub(super) fn expectation_label(&self, label: Option<String>) -> ExpectationLabel {
        ExpectationLabel {
            common: self.common.clone(),
            step_key: self.step_key.clone(),
            label: label,
        }
    }
}


#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct ExpectationLabel {
    step_key: String,
    label: Option<String>,
    #[prometheus(flatten)]
    common: CommonLabel
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct MaterializationLabel {
    step_key: Option<String>,
    asset_key: String,
    partition: Option<String>,
    #[prometheus(flatten)]
    common: CommonLabel
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct SensorLabel {
  workspace_location: String,
  repository_name: String,
  name: String,
  sensor_type: String,
  eval_interval: i32,
  
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct DaemonStatusLabel {
  id: String,
  daemon_type: String,
  required: String,
  healthy: Option<String>,
}

impl DaemonStatusLabel {
    pub(super) fn new(daemon: DagitQueryInstanceDaemonHealthAllDaemonStatuses) -> DaemonStatusLabel {
        DaemonStatusLabel {
            id: daemon.id,
            daemon_type: daemon.daemon_type,
            required: daemon.required.to_string(),
            healthy: daemon.healthy.map(|x| x.to_string()),
        }
    }
}


#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct WorkspaceLocationLabel {
  workspace_location: String,
  load_status: String
}

impl WorkspaceLocationLabel {
    pub(super) fn new(workspace: &DagitQueryWorkspaceOrErrorOnWorkspaceLocationEntries) -> WorkspaceLocationLabel {
        WorkspaceLocationLabel {
            workspace_location: workspace.name.clone(),
            load_status: format!("{:?}", workspace.load_status)
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub(super) struct InstigationLabel {
  workspace_location: String,
  repository_name: String,
  instigation_name: String,
  instigation_type: String,
}

impl InstigationLabel {
    pub(super) fn new(workspace: String, repo: String, name: String, i_type: String) -> InstigationLabel {
        InstigationLabel {
            workspace_location: workspace,
            repository_name: repo,
            instigation_name: name,
            instigation_type: i_type
        }
    }
}