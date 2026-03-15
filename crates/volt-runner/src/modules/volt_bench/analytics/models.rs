use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsProfileOptions {
    pub dataset_size: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsBenchmarkOptions {
    pub dataset_size: Option<f64>,
    pub iterations: Option<f64>,
    pub search_term: Option<String>,
    pub min_score: Option<f64>,
    pub top_n: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnalyticsProfile {
    pub(super) dataset_size: u64,
    pub(super) cached_sizes: Vec<u64>,
    pub(super) category_spread: BTreeMap<String, u64>,
    pub(super) region_spread: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CategoryWinner {
    pub(super) category: String,
    pub(super) total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnalyticsSample {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) category: String,
    pub(super) region: String,
    pub(super) score: u64,
    pub(super) revenue: u64,
    pub(super) margin: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnalyticsBenchmarkResult {
    pub(super) dataset_size: u64,
    pub(super) iterations: u64,
    pub(super) query: String,
    pub(super) min_score: u64,
    pub(super) top_n: u64,
    pub(super) backend_duration_ms: u64,
    pub(super) filter_duration_ms: u64,
    pub(super) sort_duration_ms: u64,
    pub(super) aggregate_duration_ms: u64,
    pub(super) peak_matches: u64,
    pub(super) total_matches_across_iterations: u64,
    pub(super) category_winners: Vec<CategoryWinner>,
    pub(super) sample: Vec<AnalyticsSample>,
    pub(super) payload_bytes: u64,
}
