use serde::Serialize;

pub(super) const DEFAULT_PROFILE_DATASET_SIZE: u64 = 24_000;
pub(super) const DEFAULT_DATASET_SIZE: u64 = 36_000;
pub(super) const DEFAULT_ITERATIONS: u64 = 6;
pub(super) const DEFAULT_MIN_SCORE: u64 = 58;
pub(super) const DEFAULT_TOP_N: u64 = 18;

pub(super) fn clamp_positive_integer(value: Option<f64>, fallback: u64, min: u64, max: u64) -> u64 {
    let normalized = value
        .filter(|number| number.is_finite())
        .map(|number| number.round() as i128)
        .unwrap_or(i128::from(fallback));

    normalized.clamp(i128::from(min), i128::from(max)) as u64
}

pub(super) fn duration_ms(duration: std::time::Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

pub(super) fn serialized_len<T: Serialize>(value: &T) -> Result<u64, String> {
    serde_json::to_vec(value)
        .map(|payload| payload.len() as u64)
        .map_err(|error| format!("failed to serialize analytics benchmark payload: {error}"))
}
