use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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

#[derive(Debug, Clone)]
pub(super) struct AnalyticsRecord {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) category: &'static str,
    pub(super) region: &'static str,
    pub(super) status: &'static str,
    pub(super) priority: u64,
    pub(super) score: u64,
    pub(super) revenue: u64,
    pub(super) margin: u64,
    pub(super) updated_at: u64,
    pub(super) search_blob: String,
}

pub(super) fn dataset_cache() -> &'static Mutex<HashMap<u64, Arc<Vec<AnalyticsRecord>>>> {
    DATASET_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn get_dataset(dataset_size: u64) -> Result<Arc<Vec<AnalyticsRecord>>, String> {
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

pub(super) fn row_weight(row: &AnalyticsRecord) -> u64 {
    row.score * 4 + row.margin * 3 + row.priority + (row.revenue / 200)
}
