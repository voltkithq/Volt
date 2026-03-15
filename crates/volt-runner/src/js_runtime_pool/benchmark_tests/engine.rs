use serde_json::{Value as JsonValue, json};
use std::env;

const BENCHMARK_ENGINE_ENV: &str = "VOLT_BENCH_ENGINE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BenchmarkEngine {
    Js,
    Native,
    Direct,
}

impl BenchmarkEngine {
    pub(super) fn from_env() -> Result<Self, String> {
        match env::var(BENCHMARK_ENGINE_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("direct") => Ok(Self::Direct),
            Ok(value) if value.eq_ignore_ascii_case("native") => Ok(Self::Native),
            Ok(value) if value.eq_ignore_ascii_case("js") || value.trim().is_empty() => {
                Ok(Self::Js)
            }
            Ok(value) => Err(format!("unsupported {BENCHMARK_ENGINE_ENV} value: {value}")),
            Err(env::VarError::NotPresent) => Ok(Self::Js),
            Err(error) => Err(format!("failed to read {BENCHMARK_ENGINE_ENV}: {error}")),
        }
    }

    pub(super) fn payload(self, payload: JsonValue) -> JsonValue {
        if !matches!(self, Self::Native) {
            return payload;
        }

        let mut payload = payload;
        if let Some(object) = payload.as_object_mut() {
            object.insert("engine".to_string(), json!("native"));
        }
        payload
    }

    pub(super) fn analytics_profile_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::DATA_PROFILE_CHANNEL,
            Self::Js | Self::Native => "analytics:profile",
        }
    }

    pub(super) fn analytics_run_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::DATA_QUERY_CHANNEL,
            Self::Js | Self::Native => "analytics:run",
        }
    }

    pub(super) fn workflow_run_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::WORKFLOW_RUN_CHANNEL,
            Self::Js | Self::Native => "workflow:run",
        }
    }
}
