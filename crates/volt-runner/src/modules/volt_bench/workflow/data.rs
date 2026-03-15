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

#[derive(Debug)]
pub(super) struct WorkflowDocument {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) owner: &'static str,
    pub(super) region: &'static str,
    pub(super) normalized: String,
    pub(super) tokens: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) priority: u64,
    pub(super) risk_score: u64,
    pub(super) route: String,
    pub(super) digest: String,
}

pub(super) fn build_documents(batch_size: u64) -> Vec<WorkflowDocument> {
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

pub(super) fn normalize_ascii_text(input: &str) -> String {
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

pub(super) fn push_tag(tags: &mut Vec<String>, value: &str) {
    if tags.iter().any(|entry| entry == value) || tags.len() >= 8 {
        return;
    }
    tags.push(value.to_string());
}
