mod dagster;
mod docker;

use dagster::*;

use testcontainers::GenericImage;
use testcontainers::core::WaitFor;
use tokio::task::spawn_local;
use tokio::time::Duration;

use std::fs::File;
use std::net::{IpAddr, Ipv6Addr};
use std::path::Path;

pub async fn end_to_end(img_name: &str, img_tag: &str) {
    // Prepare docker container running dagster
    let docker_client = docker::Client::new();
    docker_client.build_local_image(
        img_name, 
        img_tag, 
        File::open("tests/Dockerfile").expect("Can't read Dockerfile")
    ).await.expect("Failed to build docker image");

    let image = GenericImage::new(img_name, img_tag)
        .with_exposed_port(3000)
        .with_wait_for( WaitFor::StdOutMessage {
            message: "Serving dagster-webserver on http://0.0.0.0:3000".to_string()
        })
        .with_wait_for( WaitFor::Duration { length: Duration::from_millis(500) } );

    let args = vec!["-f".to_string(), "usercode.py".to_string()];
    
    let ts_client = testcontainers::clients::Http::default();
    let dagster = ts_client.run((image, args)).await;
    docker_client.copy_file(
        dagster.id(), 
        File::open("tests/usercode.py").expect("Can't read python usercode"), 
        Path::new("/dagster/usercode.py")
    ).await.expect("Failed to copy usercode file to container");

    let dagit_url = format!("http://{}:{}/graphql", docker_client.host(), dagster.get_host_port_ipv4(3000).await);

    // Start the dagster prometheus exporter
    println!("Dagster docker container: {}", dagster.id());
    let _exporter = spawn_local(
        dagster_prom_exporter::serve(dagit_url.clone(), IpAddr::V6(Ipv6Addr::LOCALHOST), 3001,  5)
    );
    let exporter_url = "http://localhost:3001/metrics";
    let http = reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap();

    // Wait for dagster to load the usercode that has been copied to the container
    assert!(load_usercode(http.post(&dagit_url)).await, "Dagster usercode cannot been loaded");

    // Launch dagster pipelines. The hello_world job should succeed, the
    // alphabet asset materialization should fail
    let run_ids = run_dagster_pipelines(
        http.post(&dagit_url),
        vec![
            DagsterPipeline::new("hello_world", false, None),
            DagsterPipeline::new("alphabet", true, None)]
    ).await;

    // Wait for dagster pipelines to complete
    let ready = wait_for_pipelines(http.post(&dagit_url), Duration::from_secs(10), run_ids).await;
    assert!(ready, "Dagster pipelines have not completed in time");

    // Query exporter metrics for completed dagster pipelines
    let samples = parse_metrics(http.get(exporter_url)).await.expect("Can't parse prometheus metrics");
    let (asset_ts, scrape_ts) = test_completed_run_metrics(samples);

    // Launch the same pipelines plus an additional one (ascii_job) which should
    // allow the alphabet asset materialization to succeed.
    let more_pipelines = vec![
        DagsterPipeline::new("ascii_job", false, None),
        DagsterPipeline::new("hello_world", false, Some(r#"{"ops": {"world": {"config": {"name": "Chris"}}}}"#)),
        DagsterPipeline::new("alphabet", true, None),
    ];
    let run_ids = run_dagster_pipelines(http.post(&dagit_url), more_pipelines).await;
    let ready = wait_for_pipelines(http.post(&dagit_url), Duration::from_secs(10), run_ids).await;
    assert!(ready, "Dagster pipelines have not completed in time");

    // Query exporter metrics for the new completed dagster pipelines
    let samples = parse_metrics(http.get(exporter_url)).await.expect("Can't parse prometheus metrics");
    test_more_run_metrics(samples, asset_ts, scrape_ts);
}


// For comparing the exporter's Counter metrics, the exporter uses
// prometheus_client which confusingly follows the openmetrics standard, which
// our prometheus_parse client does NOT support, hence in these tests it will
// be parsed as an Untyped metric, not a Counter. It will parsed correctly by
// the actual prometheus server.
// 
// See:
// * https://github.com/ccakes/prometheus-parse-rs/issues/5
// * https://github.com/prometheus/client_rust/issues/51

fn test_completed_run_metrics(samples: Vec<prometheus_parse::Sample>) -> (f64, f64) {
    use prometheus_parse::Labels;
    use prometheus_parse::Value::{Gauge, Untyped};

    let run_total: Vec<&Labels> = samples.iter().filter_map(|x|
        if x.metric == "run_total" && x.value == Untyped(1.0) { Some(&x.labels) } else { None }
    ).collect();

    assert_eq!(run_total.len(), 2, "Expected 2 samples for the run_total metric, got: {:?}", run_total);
    assert!(run_total.iter().all(|x|
        (x.get("pipeline_name") == Some("__ASSET_JOB") && x.get("status") == Some("FAILURE")) ||
        (x.get("pipeline_name") == Some("hello_world") && x.get("status") == Some("SUCCESS"))
        ),
        "Asset materialization should fail and hello_world job should succeed but got: {:?}", run_total
    );

    let duration: Vec<&Labels> = samples.iter().filter_map(|x|
        if x.metric == "run_duration_seconds" { Some(&x.labels) } else { None }
    ).collect();
    assert_eq!(duration.len(), 2, "Expected 2 samples for the run_duration metric, got: {:?}", duration);
    assert!(duration.iter().all(|x|
        (x.get("pipeline_name") == Some("__ASSET_JOB") && x.get("status") == Some("FAILURE")) ||
        (x.get("pipeline_name") == Some("hello_world") && x.get("status") == Some("SUCCESS"))
        ),
        "Asset materialization should fail and hello_world job should succeed but got: {:?}", duration
    );

    let step_total: Vec<&Labels> = samples.iter().filter_map(|x|
        if x.metric == "step_total" && x.value == Untyped(1.0) { Some(&x.labels) } else { None }
    ).collect();

    assert_eq!(step_total.len(), 4, "Expected 4 samples for the step_total metric, got: {:?}", step_total);
    assert!(step_total.iter().all(|x| {
        let (a, b, c) = (x.get("pipeline_name"), x.get("step_key"), x.get("status"));
        ((a, b, c) == (Some("hello_world"), Some("hello"), Some("SUCCESS"))) ||
        ((a, b, c) == (Some("hello_world"), Some("world"), Some("SUCCESS"))) ||
        ((a, b, c) == (Some("hello_world"), Some("save_s3"), Some("SUCCESS"))) ||
        ((a, b, c) == (Some("__ASSET_JOB"), Some("alphabet"), Some("FAILURE")))
        }),
        "Asset materialization should fail and hello_world job ops should succeed but got: {:?}", step_total
    );
    
    for metric in ["step_duration_seconds", "step_attempts"] {
        let m: Vec<&Labels> = samples.iter().filter_map(|x| {
            if let Gauge(i) = x.value {
                if i > 0.0 && x.metric == metric {
                    return Some(&x.labels)
                }
            };
            None
        }).collect();
        assert_eq!(m.len(), 4, "Expected 4 positive samples for the {} metric, got: {:?}", metric, m);
        assert!(m.iter().all(|x| {
            let (a, b, c) = (x.get("pipeline_name"), x.get("step_key"), x.get("status"));
            ((a, b, c) == (Some("hello_world"), Some("hello"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("hello_world"), Some("world"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("hello_world"), Some("save_s3"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("__ASSET_JOB"), Some("alphabet"), Some("FAILURE")))
            }),
            "Asset materialization should fail and hello_world job ops should succeed but got: {:?}", m
        )
    }

    let expectations: Vec<_> = samples.iter().filter(|x| x.metric == "expectation_failure").collect();
    assert_eq!(expectations.len(), 1,
        "Expected 1 sample for the expectation_failure metric but got: {:?}", expectations
    );
    assert!({
        let expect = expectations.first().unwrap();
        expect.labels.get("step_key") == Some("world") && expect.value == Gauge(1.0)
        },
        "Expected an expectation failure from the world op but got: {:?}", expectations
    );
    
    let assets: Vec<_> = samples.iter().filter(|x| x.metric == "asset_materialization_latest_seconds").collect();
    assert_eq!(assets.len(), 1,
        "Expected 1 sample for the asset_materialization_latest_seconds metric but got: {:?}", assets
    );
    assert!({
        let asset = assets.first().unwrap();
        asset.labels.get("step_key") == Some("save_s3") && match asset.value {
            Gauge(x) => x > 0.0,
            _ => false
        }},
        "Expected an asset materialization from the save_s3 op but got: {:?}", assets
    );

    let last_scraped: Vec<_> = samples.iter()
        .filter(|x| x.metric == "exporter_last_scrape_runs" && match x.value {
            Gauge(i) => i == 2.0,
            _ => false
        })
        .collect();
    assert_eq!(last_scraped.len(), 1, "Expected 2 last scraped runs but got: {:?}", last_scraped);

    let last_scraped_ts: Vec<_> = samples.iter()
        .filter(|x| x.metric == "exporter_last_scrape_timestamp" && match x.value {
            Gauge(i) => i > 0.0,
            _ => false
        })
        .collect();
    assert_eq!(last_scraped_ts.len(), 1, "Expected positive last scraped timestamp but got: {:?}", last_scraped_ts);

    match (assets.first().map(|x| x.value.clone()), last_scraped_ts.first().map(|x| x.value.clone())) {
        (Some(Gauge(a)), Some(Gauge(b))) => (a, b),
        _ => panic!("Can't return asset materialization timestamp and last scrape timestamp")
    }
}


fn test_more_run_metrics(samples: Vec<prometheus_parse::Sample>, asset_ts: f64, scrape_ts: f64) {
    use prometheus_parse::Labels;
    use prometheus_parse::Value::{Gauge, Untyped};

    let run_total: Vec<_> = samples.iter().filter(|x| x.metric == "run_total").collect();
    assert_eq!(run_total.len(), 4, "Expected 4 samples for the run_total metric, got: {:?}", run_total);

    assert!(run_total.iter().all(|x| {
        let (a, b, c) = (&x.value, x.labels.get("pipeline_name"), x.labels.get("status"));
        ((a, b, c) == (&Untyped(1.0), Some("__ASSET_JOB"), Some("FAILURE"))) ||
        ((a, b, c) == (&Untyped(2.0), Some("hello_world"), Some("SUCCESS"))) ||
        ((a, b, c) == (&Untyped(1.0), Some("ascii_job"), Some("SUCCESS"))) ||
        ((a, b, c) == (&Untyped(1.0), Some("__ASSET_JOB"), Some("SUCCESS")))
        }),
        "hello_world should succeed 2x, ascii_job 1x, 1 good and failed asset but got: {:?}", run_total
    );

    let duration: Vec<&Labels> = samples.iter().filter_map(|x|
        if x.metric == "run_duration_seconds" { Some(&x.labels) } else { None }
    ).collect();
    assert_eq!(duration.len(), 3, "Expected 3 samples for the run_duration metric, got: {:?}", duration);
    assert!(duration.iter().all(|x|
        (x.get("pipeline_name") == Some("__ASSET_JOB") && x.get("status") == Some("SUCCESS")) ||
        (x.get("pipeline_name") == Some("hello_world") && x.get("status") == Some("SUCCESS")) || 
        (x.get("pipeline_name") == Some("ascii_job") && x.get("status") == Some("SUCCESS"))
        ),
        "Successful alphabet asset materialization should overwrite status=FAILURE metric but got: {:?}", duration
    );

    let step_total: Vec<_> = samples.iter().filter(|x| x.metric == "step_total").collect();
    assert_eq!(step_total.len(), 6, "Expected 6 samples for the step_total metric, got: {:?}", step_total);

    assert!(step_total.iter().all(|x| {
        let (a, b, c, d) =
            (&x.value, x.labels.get("pipeline_name"), x.labels.get("step_key"), x.labels.get("status"));
        ((a, b, c, d) == (&Untyped(2.0), Some("hello_world"), Some("hello"), Some("SUCCESS"))) ||
        ((a, b, c, d) == (&Untyped(2.0), Some("hello_world"), Some("world"), Some("SUCCESS"))) ||
        ((a, b, c, d) == (&Untyped(2.0), Some("hello_world"), Some("save_s3"), Some("SUCCESS"))) ||
        ((a, b, c, d) == (&Untyped(1.0), Some("__ASSET_JOB"), Some("alphabet"), Some("FAILURE"))) ||
        ((a, b, c, d) == (&Untyped(1.0), Some("__ASSET_JOB"), Some("alphabet"), Some("SUCCESS"))) ||
        ((a, b, c, d) == (&Untyped(1.0), Some("ascii_job"), Some("ascii"), Some("SUCCESS")))
        }),
        "hello_world job ops should suceed 2x, ascii_job 1x, 1 good and failed asset but got: {:?}", step_total
    );
    
    for metric in ["step_duration_seconds", "step_attempts"] {
        let m: Vec<&Labels> = samples.iter().filter_map(|x| {
            if let Gauge(i) = x.value {
                if i > 0.0 && x.metric == metric {
                    return Some(&x.labels)
                }
            };
            None
        }).collect();
        assert_eq!(m.len(), 5, "Expected 5 positive samples for the {} metric, got: {:?}", metric, m);
        assert!(m.iter().all(|x| {
            let (a, b, c) = (x.get("pipeline_name"), x.get("step_key"), x.get("status"));
            ((a, b, c) == (Some("hello_world"), Some("hello"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("hello_world"), Some("world"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("hello_world"), Some("save_s3"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("__ASSET_JOB"), Some("alphabet"), Some("SUCCESS"))) ||
            ((a, b, c) == (Some("ascii_job"), Some("ascii"), Some("SUCCESS")))
            }),
            "Successful alphabet asset materialization should overwrite status=FAILURE metric but got: {:?}", m
        )
    }

    let expectations: Vec<_> = samples.iter().filter(|x| x.metric == "expectation_failure").collect();
    assert_eq!(expectations.len(), 2,
        "Expected 2 samples for the expectation_failure metric but got: {:?}", expectations
    );
    assert!(expectations.iter().all(|x|
        x.value == Gauge(0.0) &&
        x.labels.get("step_key").filter(|x| x == &"world" || x == &"alphabet").is_some()
        ),
        "alphabet asset and latest world op should have 0 expectation failures, but got: {:?}", expectations
    );

    let assets: Vec<_> = samples.iter().filter(|x| x.metric == "asset_materialization_latest_seconds").collect();
    assert_eq!(assets.len(), 3,
        "Expected 3 samples for the asset_materialization_latest_seconds metric but got: {:?}", assets
    );
    assert!(assets.iter().all(|x|
        ["s3/hello_world", "alphabet", "ascii"].map(|x| Some(x)).contains(&x.labels.get("asset_key")) &&
        match x.value {
            Gauge(i) => i > asset_ts,
            _ => false
        }),
        "alphabet and ascii and s3/hello_world assets should materialize but got: {:?}", assets
    );

    let last_scraped: Vec<_> = samples.iter()
        .filter(|x| x.metric == "exporter_last_scrape_runs" && match x.value {
            Gauge(i) => i == 3.0,
            _ => false
        })
        .collect();
    assert_eq!(last_scraped.len(), 1, "Expected 3 last scraped runs but got: {:?}", last_scraped);

    let last_scraped_ts: Vec<_> = samples.iter()
        .filter(|x| x.metric == "exporter_last_scrape_timestamp" && match x.value {
            Gauge(i) => i > scrape_ts,
            _ => false
        })
        .collect();
    assert_eq!(last_scraped_ts.len(), 1, "Expected positive last scraped timestamp but got: {:?}", last_scraped_ts);
}

async fn parse_metrics(req: reqwest::RequestBuilder) -> anyhow::Result<Vec<prometheus_parse::Sample>> {
    let response = req.send().await?;
    let lines: Vec<_> = response.text().await?.lines().map(|s| Ok(s.to_owned())).collect();
    let metrics = prometheus_parse::Scrape::parse(lines.into_iter())?;
    Ok(metrics.samples)
}
