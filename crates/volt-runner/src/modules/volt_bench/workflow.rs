use std::collections::BTreeMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_BATCH_SIZE: u64 = 4_500;
const DEFAULT_PASSES: u64 = 3;
const TITLE_PATTERNS: [&str; 6] = [
    "Risk queue",
    "Latency sweep",
    "Renewal drift",
    "Policy delta",
    "Security burst",
    "Audit spill",
];
const BODY_PATTERNS: [&str; 4] = [
    "urgent security queue pressure from renewal cohort",
    "latency tracking indicates policy drift in edge cluster",
    "audit pipeline shows backlog and regional imbalance",
    "renewal ops requested deeper risk review on security path",
];
const OWNER_NAMES: [&str; 6] = ["Avery", "Jordan", "Morgan", "Riley", "Parker", "Sage"];
const REGION_NAMES: [&str; 4] = ["us-east", "us-west", "emea", "apac"];
const HEAT_TERMS: [&str; 7] = [
    "urgent", "queue", "risk", "renewal", "latency", "security", "audit",
];
const PLUGIN_NAMES: [&str; 5] = [
    "normalizeText",
    "extractSignals",
    "scorePriority",
    "routeQueues",
    "buildDigests",
];

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowBenchmarkOptions {
    pub batch_size: Option<f64>,
    pub passes: Option<f64>,
    pub pipeline: Option<Vec<String>>,
}

#[derive(Debug)]
struct WorkflowDocument {
    id: u64,
    title: String,
    body: String,
    owner: &'static str,
    region: &'static str,
    normalized: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    priority: u64,
    risk_score: u64,
    route: String,
    digest: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowStepTiming {
    plugin: String,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowBenchmarkResult {
    batch_size: u64,
    passes: u64,
    pipeline: Vec<String>,
    backend_duration_ms: u64,
    step_timings: Vec<WorkflowStepTiming>,
    route_distribution: BTreeMap<String, u64>,
    average_priority: f64,
    digest_sample: Vec<String>,
    payload_bytes: u64,
}

pub fn run_workflow_benchmark_json(options: WorkflowBenchmarkOptions) -> Result<Value, String> {
    let batch_size = clamp_positive_integer(options.batch_size, DEFAULT_BATCH_SIZE, 500, 25_000);
    let passes = clamp_positive_integer(options.passes, DEFAULT_PASSES, 1, 8);
    let pipeline = normalize_pipeline(options.pipeline);
    let mut documents = build_documents(batch_size);
    let mut step_timings = BTreeMap::<String, u64>::new();
    let started_at = Instant::now();

    for pass in 0..passes {
        for plugin in &pipeline {
            let plugin_started_at = Instant::now();
            match plugin.as_str() {
                "normalizeText" => normalize_text(&mut documents, pass),
                "extractSignals" => extract_signals(&mut documents, pass),
                "scorePriority" => score_priority(&mut documents, pass),
                "routeQueues" => route_queues(&mut documents, pass),
                "buildDigests" => build_digests(&mut documents, pass),
                _ => continue,
            }
            *step_timings.entry(plugin.clone()).or_insert(0) +=
                duration_ms(plugin_started_at.elapsed());
        }
    }

    let mut route_distribution = BTreeMap::new();
    let mut total_priority = 0u64;
    for document in &documents {
        total_priority += document.priority;
        *route_distribution
            .entry(document.route.clone())
            .or_insert(0) += 1;
    }

    let mut result = WorkflowBenchmarkResult {
        batch_size,
        passes,
        pipeline: pipeline.clone(),
        backend_duration_ms: duration_ms(started_at.elapsed()),
        step_timings: step_timings
            .into_iter()
            .map(|(plugin, duration_ms)| WorkflowStepTiming {
                plugin,
                duration_ms,
            })
            .collect(),
        route_distribution,
        average_priority: if documents.is_empty() {
            0.0
        } else {
            ((total_priority as f64 / documents.len() as f64) * 100.0).round() / 100.0
        },
        digest_sample: documents
            .iter()
            .take(6)
            .map(|document| document.digest.clone())
            .collect(),
        payload_bytes: 0,
    };
    result.payload_bytes = serialized_len(&result)?;

    serde_json::to_value(result)
        .map_err(|error| format!("failed to serialize workflow benchmark result: {error}"))
}

fn normalize_pipeline(requested_pipeline: Option<Vec<String>>) -> Vec<String> {
    let Some(requested_pipeline) = requested_pipeline else {
        return PLUGIN_NAMES
            .iter()
            .map(|plugin| plugin.to_string())
            .collect();
    };

    let mut pipeline = requested_pipeline
        .into_iter()
        .filter(|plugin| PLUGIN_NAMES.contains(&plugin.as_str()))
        .fold(Vec::<String>::new(), |mut acc, plugin| {
            if !acc.contains(&plugin) {
                acc.push(plugin);
            }
            acc
        });

    if pipeline.is_empty() {
        pipeline = PLUGIN_NAMES
            .iter()
            .map(|plugin| plugin.to_string())
            .collect();
    }
    pipeline
}

fn build_documents(batch_size: u64) -> Vec<WorkflowDocument> {
    let mut documents = Vec::with_capacity(batch_size as usize);
    for index in 0..batch_size {
        documents.push(WorkflowDocument {
            id: index + 1,
            title: format!(
                "{} {}",
                TITLE_PATTERNS[index as usize % TITLE_PATTERNS.len()],
                index + 1
            ),
            body: format!(
                "{} owner {} region {}",
                BODY_PATTERNS[index as usize % BODY_PATTERNS.len()],
                OWNER_NAMES[index as usize % OWNER_NAMES.len()],
                REGION_NAMES[index as usize % REGION_NAMES.len()],
            ),
            owner: OWNER_NAMES[((index * 3) as usize) % OWNER_NAMES.len()],
            region: REGION_NAMES[((index * 5) as usize) % REGION_NAMES.len()],
            normalized: String::new(),
            tokens: Vec::new(),
            tags: Vec::new(),
            priority: 0,
            risk_score: 0,
            route: "pending".to_string(),
            digest: String::new(),
        });
    }
    documents
}

fn normalize_text(documents: &mut [WorkflowDocument], pass: u64) {
    for document in documents {
        let merged = format!(
            "{} {} {} {}",
            document.title, document.body, document.owner, document.region
        )
        .to_ascii_lowercase();
        document.normalized = normalize_ascii_text(&merged);
        if pass % 2 == 1 {
            document.normalized.push_str(format!(" p{pass}").as_str());
        }
    }
}

fn extract_signals(documents: &mut [WorkflowDocument], pass: u64) {
    for document in documents {
        document.tokens = document
            .normalized
            .split_whitespace()
            .map(ToString::to_string)
            .collect();

        let mut tags = document.tags.clone();
        for term in HEAT_TERMS {
            if document.tokens.iter().any(|token| token == term) {
                push_tag(&mut tags, term);
            }
        }
        if (document.id + pass).is_multiple_of(4) {
            push_tag(&mut tags, "burst");
        }
        document.tags = tags;
    }
}

fn score_priority(documents: &mut [WorkflowDocument], pass: u64) {
    for document in documents {
        let mut score = 0u64;
        score += document.tokens.len() as u64 * 2;
        score += document.tags.len() as u64 * 5;
        score += if document.region == "emea" { 7 } else { 3 };
        score += document.owner.len() as u64;
        if document.normalized.contains("security") {
            score += 18;
        }
        if document.normalized.contains("latency") {
            score += 12;
        }
        document.risk_score = score + pass * 3;
        document.priority = (score + (document.id % 19)).min(100);
    }
}

fn route_queues(documents: &mut [WorkflowDocument], pass: u64) {
    for document in documents {
        if document.risk_score >= 70 {
            document.route = if pass.is_multiple_of(2) {
                "rapid-response".to_string()
            } else {
                "incident-review".to_string()
            };
            continue;
        }
        if document.priority >= 55 {
            document.route = "priority-backlog".to_string();
            continue;
        }
        document.route = if document.region == "apac" {
            "regional-apac".to_string()
        } else {
            "steady-state".to_string()
        };
    }
}

fn build_digests(documents: &mut [WorkflowDocument], pass: u64) {
    for document in documents {
        let headline = document
            .tokens
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let tags = document.tags.join(", ");
        document.digest = format!("{} | {} | {} | p{pass}", document.route, headline, tags);
    }
}

fn normalize_ascii_text(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut last_was_space = true;

    for ch in input.chars() {
        let next = if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            Some(ch)
        } else if ch.is_ascii_whitespace() || !ch.is_ascii_alphanumeric() {
            Some(' ')
        } else {
            None
        };

        match next {
            Some(' ') if !last_was_space => {
                normalized.push(' ');
                last_was_space = true;
            }
            Some(' ') => {}
            Some(value) => {
                normalized.push(value);
                last_was_space = false;
            }
            None => {}
        }
    }

    if normalized.ends_with(' ') {
        normalized.pop();
    }
    normalized
}

fn push_tag(tags: &mut Vec<String>, value: &str) {
    if tags.iter().any(|entry| entry == value) {
        return;
    }
    if tags.len() >= 8 {
        return;
    }
    tags.push(value.to_string());
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
        .map_err(|error| format!("failed to serialize workflow benchmark payload: {error}"))
}
