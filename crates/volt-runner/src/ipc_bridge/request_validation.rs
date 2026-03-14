use serde_json::Value as JsonValue;

const IPC_PROTOTYPE_CHECK_MAX_DEPTH: usize = 64;

pub(super) fn extract_request_id(raw: &str) -> String {
    match serde_json::from_str::<JsonValue>(raw) {
        Ok(value) => extract_request_id_from_value(&value),
        Err(_) => "unknown".to_string(),
    }
}

pub(super) fn extract_request_id_from_value(value: &JsonValue) -> String {
    value
        .get("id")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn validate_prototype_pollution(value: &JsonValue, depth: usize) -> Result<(), String> {
    if depth > IPC_PROTOTYPE_CHECK_MAX_DEPTH {
        return Err(format!(
            "payload nesting exceeds max depth ({depth} > {IPC_PROTOTYPE_CHECK_MAX_DEPTH})"
        ));
    }

    match value {
        JsonValue::Object(map) => {
            for key in map.keys() {
                if key == "__proto__" || key == "constructor" || key == "prototype" {
                    return Err("prototype pollution attempt detected".to_string());
                }
            }
            for nested in map.values() {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        JsonValue::Array(array) => {
            for nested in array {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        _ => {}
    }

    Ok(())
}
