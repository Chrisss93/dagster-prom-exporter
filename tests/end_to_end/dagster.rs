use indoc::indoc;
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use tokio::time::{sleep, timeout, Duration};

use std::fmt::{Display, Formatter, Result};

pub struct Pipeline<'a> {
    name: &'a str,
    is_asset: bool,
    run_config: Option<&'a str>
}

impl<'a> Pipeline<'a> {
    pub const fn new(name: &'a str, is_asset: bool, run_config: Option<&'a str>) -> Pipeline<'a> {
        Pipeline { name, is_asset, run_config }
    }
}

impl Display for Pipeline<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let pipeline = if self.is_asset {
            format!("\"__ASSET_JOB\"\n            assetSelection: {{ path: \"{}\" }}", self.name)
        } else {
            format!(r#""{}""#, self.name)
        };

        write!(
            f,
            indoc! { r#"
            {}: launchRun(executionParams: {{
                    selector: {{
                        repositoryLocationName: "usercode.py"
                        repositoryName: "__repository__"
                        pipelineName: {}
                    }}{}
            }}) {{ 
                __typename
                ... on LaunchRunSuccess {{
                    run {{
                        runId
                    }}
                }}
            }}
            "#},
            self.name,
            pipeline,
            self.run_config.map(|s| format!("\n\trunConfigData: {s:?}")).unwrap_or_default()
        )
    }
}

pub async fn run_dagster_pipelines(req: RequestBuilder, pipelines: Vec<Pipeline<'_>>) -> Vec<String> {
    let graphql = pipelines.iter().fold(String::new(), |acc, x| format!("{acc}\n{x}"));
    let response = req
        .header("Content-Type", "application/graphql")
        .body(format!("mutation{{{graphql}}}"))
        .send()
        .await
        .expect("Can't get Dagit response");

    let json = response.json::<Value>().await.expect("Dagit response not returning json");
    let data = json.get("data").and_then(|x| x.as_object()).expect("Dagit response missing 'data' field");

    data.values()
        .map(|x| {
            assert_eq!(
                x.get("__typename"),
                Some(&json!("LaunchRunSuccess")),
                "Unsuccessful runs: {x:?}"
            );
            x.get("run")
                .and_then(|y| y.get("runId"))
                .and_then(|y| y.as_str())
                .unwrap_or_else(|| panic!("Can't get dagster runId: {x:?}"))
                .to_owned()
        })
        .collect()
}

pub async fn wait_for_pipelines(req: RequestBuilder, max_wait: Duration, run_ids: Vec<String>) -> bool {
    timeout(max_wait, async {
        loop {
            if pipelines_ready(req.try_clone().unwrap(), &run_ids).await {
                break;
            }
            sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .is_ok()
}

pub async fn pipelines_ready(req: RequestBuilder, run_ids: &Vec<String>) -> bool {
    let response = req
        .header("Content-Type", "application/graphql")
        .body(
            indoc! {"
            query {
                runsOrError(filter: {runIds: ?}) {
                    __typename
                    ... on Runs {
                        results {
                            status
                        }
                    }
                }
            }
            "}
            .replace('?', format!("{run_ids:?}").as_str())
        )
        .send()
        .await
        .expect("Can't get Dagit response");

    let json = response.json::<Value>().await.expect("Dagit response not returning json");
    let runs = json
        .get("data")
        .and_then(|x| x.get("runsOrError"))
        .expect("Dagit response missing 'data.runsOrError' field");

    assert_eq!(
        runs.get("__typename"),
        Some(&json!("Runs")),
        "Malformed/unsuccessful dagit response: {runs:?}"
    );

    let results = runs.get("results").and_then(|x| x.as_array());
    assert!(results.is_some(), "Malformed/unsuccessful dagit response: {runs:?}");

    results.unwrap().iter().all(|x| {
        let status = x.get("status").and_then(|y| y.as_str());
        ["SUCCESS", "FAILURE", "CANCELED"].map(Some).contains(&status)
    })
}

pub async fn load_usercode(req: RequestBuilder) -> bool {
    let response = req
        .header("Content-Type", "application/graphql")
        .body(indoc! {"
            mutation {
                reloadWorkspace {
                    __typename
                    ... on Workspace {
                        locationEntries {
                            loadStatus
                        }
                    }
                }
                
            }
        "})
        .send()
        .await
        .expect("Can't get dagit response");

    let json = response.json::<Value>().await.expect("Dagit response not returning json");
    let reload = json
        .get("data")
        .and_then(|x| x.get("reloadWorkspace"))
        .expect("Dagit response missing 'data.reloadWorkspace' field");

    assert_eq!(
        reload.get("__typename"),
        Some(&json!("Workspace")),
        "Malformed/unsuccessful dagit response: {reload:?}"
    );

    reload
        .get("locationEntries")
        .and_then(|x| x.as_array())
        .map(|x| x.iter().all(|y| y.get("loadStatus") == Some(&json!("LOADED"))))
        .is_some_and(|x| x)
}
