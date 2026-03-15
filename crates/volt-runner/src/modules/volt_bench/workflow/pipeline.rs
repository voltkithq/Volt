use super::data::{WorkflowDocument, normalize_ascii_text, push_tag};

const HEAT_TERMS: [&str; 7] = [
    "urgent", "queue", "risk", "renewal", "latency", "security", "audit",
];
pub(super) const PLUGIN_NAMES: [&str; 5] = [
    "normalizeText",
    "extractSignals",
    "scorePriority",
    "routeQueues",
    "buildDigests",
];

pub(super) fn normalize_pipeline(requested_pipeline: Option<Vec<String>>) -> Vec<String> {
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

pub(super) fn run_pipeline_step(plugin: &str, documents: &mut [WorkflowDocument], pass: u64) {
    match plugin {
        "normalizeText" => normalize_text(documents, pass),
        "extractSignals" => extract_signals(documents, pass),
        "scorePriority" => score_priority(documents, pass),
        "routeQueues" => route_queues(documents, pass),
        "buildDigests" => build_digests(documents, pass),
        _ => {}
    }
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
