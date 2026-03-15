use serde_json::Value;

use super::super::RunnerError;

pub(super) fn parse_string_array(
    value: Option<&Value>,
    field: &str,
) -> Result<Vec<String>, RunnerError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(array) = value.as_array() else {
        return Err(RunnerError::Config(format!("{field} must be an array")));
    };

    let mut parsed = Vec::with_capacity(array.len());
    for (index, entry) in array.iter().enumerate() {
        let Some(text) = entry.as_str() else {
            return Err(RunnerError::Config(format!(
                "{field}[{index}] must be a string"
            )));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(RunnerError::Config(format!(
                "{field}[{index}] must not be empty"
            )));
        }
        parsed.push(trimmed.to_string());
    }

    Ok(parsed)
}

pub(super) fn parse_positive_u64(
    value: Option<&Value>,
    field: &str,
    default: u64,
) -> Result<u64, RunnerError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let Some(number) = value.as_u64() else {
        return Err(RunnerError::Config(format!(
            "{field} must be a positive integer"
        )));
    };
    if number == 0 {
        return Err(RunnerError::Config(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(number)
}
