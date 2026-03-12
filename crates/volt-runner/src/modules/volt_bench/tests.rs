use serde_json::json;

use super::analytics::{
    AnalyticsBenchmarkOptions, AnalyticsProfileOptions, analytics_profile_json,
    run_analytics_benchmark_json,
};
use super::workflow::{WorkflowBenchmarkOptions, run_workflow_benchmark_json};

#[test]
fn analytics_profile_is_deterministic() {
    let profile = analytics_profile_json(AnalyticsProfileOptions {
        dataset_size: Some(2_000.0),
    })
    .expect("analytics profile result");

    assert_eq!(profile["datasetSize"], json!(2_000));
    assert!(
        profile["cachedSizes"]
            .as_array()
            .expect("cached sizes array")
            .contains(&json!(2_000))
    );
    assert_eq!(profile["categorySpread"]["Finance"], json!(334));
}

#[test]
fn analytics_benchmark_returns_sample_payload() {
    let result = run_analytics_benchmark_json(AnalyticsBenchmarkOptions {
        dataset_size: Some(4_000.0),
        iterations: Some(2.0),
        search_term: Some("risk".to_string()),
        min_score: Some(50.0),
        top_n: Some(8.0),
    })
    .expect("analytics benchmark result");

    assert_eq!(result["datasetSize"], json!(4_000));
    assert_eq!(result["iterations"], json!(2));
    assert_eq!(result["sample"].as_array().expect("sample array").len(), 8);
    assert!(result["backendDurationMs"].as_u64().is_some());
}

#[test]
fn workflow_benchmark_respects_pipeline_filter() {
    let result = run_workflow_benchmark_json(WorkflowBenchmarkOptions {
        batch_size: Some(400.0),
        passes: Some(2.0),
        pipeline: Some(vec![
            "normalizeText".to_string(),
            "buildDigests".to_string(),
        ]),
    })
    .expect("workflow benchmark result");

    assert_eq!(result["batchSize"], json!(500));
    assert_eq!(result["passes"], json!(2));
    assert_eq!(
        result["pipeline"].as_array().expect("pipeline array"),
        &vec![json!("normalizeText"), json!("buildDigests")]
    );
    assert_eq!(
        result["stepTimings"]
            .as_array()
            .expect("step timings")
            .len(),
        2
    );
}
