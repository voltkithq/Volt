use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use serde_json::Value;

mod data;
mod models;
mod util;

use self::data::{dataset_cache, get_dataset, row_weight};
pub use self::models::{AnalyticsBenchmarkOptions, AnalyticsProfileOptions};
use self::models::{AnalyticsBenchmarkResult, AnalyticsProfile, AnalyticsSample, CategoryWinner};
use self::util::{
    DEFAULT_DATASET_SIZE, DEFAULT_ITERATIONS, DEFAULT_MIN_SCORE, DEFAULT_PROFILE_DATASET_SIZE,
    DEFAULT_TOP_N, clamp_positive_integer, duration_ms, serialized_len,
};

pub fn analytics_profile_json(options: AnalyticsProfileOptions) -> Result<Value, String> {
    let dataset_size = clamp_positive_integer(
        options.dataset_size,
        DEFAULT_PROFILE_DATASET_SIZE,
        1_000,
        120_000,
    );
    let rows = get_dataset(dataset_size)?;
    let mut category_spread = BTreeMap::new();
    let mut region_spread = BTreeMap::new();

    for row in rows.iter() {
        *category_spread.entry(row.category.to_string()).or_insert(0) += 1;
        *region_spread.entry(row.region.to_string()).or_insert(0) += 1;
    }

    let cached_sizes = {
        let cache = dataset_cache()
            .lock()
            .map_err(|_| "analytics dataset cache mutex poisoned".to_string())?;
        let mut sizes = cache.keys().copied().collect::<Vec<_>>();
        sizes.sort_unstable();
        sizes
    };

    serde_json::to_value(AnalyticsProfile {
        dataset_size,
        cached_sizes,
        category_spread,
        region_spread,
    })
    .map_err(|error| format!("failed to serialize analytics profile: {error}"))
}

pub fn run_analytics_benchmark_json(options: AnalyticsBenchmarkOptions) -> Result<Value, String> {
    let dataset_size =
        clamp_positive_integer(options.dataset_size, DEFAULT_DATASET_SIZE, 1_000, 120_000);
    let iterations = clamp_positive_integer(options.iterations, DEFAULT_ITERATIONS, 1, 20);
    let min_score = clamp_positive_integer(options.min_score, DEFAULT_MIN_SCORE, 0, 100);
    let top_n = clamp_positive_integer(options.top_n, DEFAULT_TOP_N, 5, 100);
    let query = options
        .search_term
        .and_then(|value| {
            let trimmed = value.trim().to_ascii_lowercase();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .unwrap_or_else(|| "risk".to_string());
    let rows = get_dataset(dataset_size)?;

    let benchmark_started_at = Instant::now();
    let mut filter_duration_ms = 0;
    let mut sort_duration_ms = 0;
    let mut aggregate_duration_ms = 0;
    let mut peak_matches = 0;
    let mut total_matches_across_iterations = 0;
    let mut latest_top = Vec::new();
    let mut latest_winners = Vec::new();

    for _ in 0..iterations {
        let filter_started_at = Instant::now();
        let mut matches = rows
            .iter()
            .filter(|row| {
                row.status != "archived"
                    && row.score >= min_score
                    && row.search_blob.contains(query.as_str())
            })
            .collect::<Vec<_>>();
        filter_duration_ms += duration_ms(filter_started_at.elapsed());
        peak_matches = peak_matches.max(matches.len() as u64);
        total_matches_across_iterations += matches.len() as u64;

        let sort_started_at = Instant::now();
        matches.sort_unstable_by(|left, right| {
            let left_weight = row_weight(left);
            let right_weight = row_weight(right);
            right_weight
                .cmp(&left_weight)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        latest_top = matches
            .iter()
            .take(top_n as usize)
            .map(|row| AnalyticsSample {
                id: row.id,
                title: row.title.clone(),
                category: row.category.to_string(),
                region: row.region.to_string(),
                score: row.score,
                revenue: row.revenue,
                margin: row.margin,
            })
            .collect();
        sort_duration_ms += duration_ms(sort_started_at.elapsed());

        let aggregate_started_at = Instant::now();
        let mut buckets = HashMap::<&'static str, u64>::new();
        for row in &matches {
            let bucket_score = row.score + row.margin + (row.revenue / 500);
            *buckets.entry(row.category).or_insert(0) += bucket_score;
        }
        let mut winners = buckets
            .into_iter()
            .map(|(category, total)| CategoryWinner {
                category: category.to_string(),
                total,
            })
            .collect::<Vec<_>>();
        winners.sort_unstable_by(|left, right| {
            right
                .total
                .cmp(&left.total)
                .then_with(|| left.category.cmp(&right.category))
        });
        latest_winners = winners.into_iter().take(4).collect();
        aggregate_duration_ms += duration_ms(aggregate_started_at.elapsed());
    }

    let mut result = AnalyticsBenchmarkResult {
        dataset_size,
        iterations,
        query,
        min_score,
        top_n,
        backend_duration_ms: duration_ms(benchmark_started_at.elapsed()),
        filter_duration_ms,
        sort_duration_ms,
        aggregate_duration_ms,
        peak_matches,
        total_matches_across_iterations,
        category_winners: latest_winners,
        sample: latest_top,
        payload_bytes: 0,
    };
    result.payload_bytes = serialized_len(&result)?;

    serde_json::to_value(result)
        .map_err(|error| format!("failed to serialize analytics benchmark result: {error}"))
}
