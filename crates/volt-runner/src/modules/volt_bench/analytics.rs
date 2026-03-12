use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_PROFILE_DATASET_SIZE: u64 = 24_000;
const DEFAULT_DATASET_SIZE: u64 = 36_000;
const DEFAULT_ITERATIONS: u64 = 6;
const DEFAULT_MIN_SCORE: u64 = 58;
const DEFAULT_TOP_N: u64 = 18;

const CATEGORY_NAMES: [&str; 6] = [
    "Finance",
    "Security",
    "Ops",
    "Growth",
    "Compliance",
    "Platform",
];
const REGION_NAMES: [&str; 5] = ["us-east", "us-west", "emea", "latam", "apac"];
const OWNER_NAMES: [&str; 7] = [
    "Avery", "Jordan", "Morgan", "Kai", "Sage", "Parker", "Riley",
];
const STATUS_NAMES: [&str; 4] = ["active", "pending", "review", "archived"];
const TAG_SETS: [[&str; 3]; 5] = [
    ["risk", "delta", "forecast"],
    ["uptime", "queue", "latency"],
    ["margin", "renewal", "pipeline"],
    ["trust", "audit", "policy"],
    ["urgent", "backfill", "priority"],
];

static DATASET_CACHE: OnceLock<Mutex<HashMap<u64, Arc<Vec<AnalyticsRecord>>>>> = OnceLock::new();

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

#[derive(Debug, Clone)]
struct AnalyticsRecord {
    id: u64,
    title: String,
    category: &'static str,
    region: &'static str,
    status: &'static str,
    priority: u64,
    score: u64,
    revenue: u64,
    margin: u64,
    updated_at: u64,
    search_blob: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsProfile {
    dataset_size: u64,
    cached_sizes: Vec<u64>,
    category_spread: BTreeMap<String, u64>,
    region_spread: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoryWinner {
    category: String,
    total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsSample {
    id: u64,
    title: String,
    category: String,
    region: String,
    score: u64,
    revenue: u64,
    margin: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsBenchmarkResult {
    dataset_size: u64,
    iterations: u64,
    query: String,
    min_score: u64,
    top_n: u64,
    backend_duration_ms: u64,
    filter_duration_ms: u64,
    sort_duration_ms: u64,
    aggregate_duration_ms: u64,
    peak_matches: u64,
    total_matches_across_iterations: u64,
    category_winners: Vec<CategoryWinner>,
    sample: Vec<AnalyticsSample>,
    payload_bytes: u64,
}

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

fn dataset_cache() -> &'static Mutex<HashMap<u64, Arc<Vec<AnalyticsRecord>>>> {
    DATASET_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_dataset(dataset_size: u64) -> Result<Arc<Vec<AnalyticsRecord>>, String> {
    let cache = dataset_cache();
    let mut cache = cache
        .lock()
        .map_err(|_| "analytics dataset cache mutex poisoned".to_string())?;

    if let Some(existing) = cache.get(&dataset_size) {
        return Ok(existing.clone());
    }

    let mut rows = Vec::with_capacity(dataset_size as usize);
    for index in 0..dataset_size {
        let category = CATEGORY_NAMES[index as usize % CATEGORY_NAMES.len()];
        let region = REGION_NAMES[((index * 3) as usize) % REGION_NAMES.len()];
        let owner = OWNER_NAMES[((index * 5) as usize) % OWNER_NAMES.len()];
        let status = STATUS_NAMES[((index * 7) as usize) % STATUS_NAMES.len()];
        let tags = TAG_SETS[index as usize % TAG_SETS.len()];
        let priority = (index * 17) % 100;
        let score = (index * 19 + priority * 3) % 100;
        let revenue = 900 + ((index * 97) % 12_500);
        let margin = ((index * 31) % 41) + 8;
        let title = format!("{category} {region} {owner} risk pipeline {}", index + 1);
        let search_blob = format!(
            "{} {} {} {} {}",
            title,
            owner,
            region,
            tags[0],
            [tags[1], tags[2]].join(" ")
        )
        .to_ascii_lowercase();

        rows.push(AnalyticsRecord {
            id: index + 1,
            title,
            category,
            region,
            status,
            priority,
            score,
            revenue,
            margin,
            updated_at: 1_710_000_000_000 + index * 91_000,
            search_blob,
        });
    }

    let rows = Arc::new(rows);
    cache.insert(dataset_size, rows.clone());
    Ok(rows)
}

fn row_weight(row: &AnalyticsRecord) -> u64 {
    row.score * 4 + row.margin * 3 + row.priority + (row.revenue / 200)
}

fn clamp_positive_integer(value: Option<f64>, fallback: u64, min: u64, max: u64) -> u64 {
    let normalized = value
        .filter(|number| number.is_finite())
        .map(|number| number.round() as i128)
        .unwrap_or(i128::from(fallback));

    normalized.clamp(i128::from(min), i128::from(max)) as u64
}

fn duration_ms(duration: std::time::Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn serialized_len<T: Serialize>(value: &T) -> Result<u64, String> {
    serde_json::to_vec(value)
        .map(|payload| payload.len() as u64)
        .map_err(|error| format!("failed to serialize analytics benchmark payload: {error}"))
}
