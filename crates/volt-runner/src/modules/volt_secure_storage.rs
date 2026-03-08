use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use volt_core::permissions::Permission;

use super::{
    native_function_module, promise_from_result, require_permission_message, secure_storage_adapter,
};

const MAX_KEY_LENGTH: usize = 256;
const MAX_VALUE_LENGTH: usize = 8192;

fn normalize_key(key: String) -> Result<String, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("secureStorage key must not be empty".to_string());
    }
    if trimmed.len() > MAX_KEY_LENGTH {
        return Err(format!(
            "secureStorage key length must be <= {MAX_KEY_LENGTH} characters"
        ));
    }
    Ok(trimmed.to_string())
}

fn set_secret(key: String, value: String) -> Result<(), String> {
    require_permission_message(Permission::SecureStorage)?;
    let normalized_key = normalize_key(key)?;
    if value.len() > MAX_VALUE_LENGTH {
        return Err(format!(
            "secureStorage value length must be <= {MAX_VALUE_LENGTH} bytes"
        ));
    }
    let adapter = secure_storage_adapter()?;
    adapter
        .set(&normalized_key, &value)
        .map_err(|error| format!("secureStorage.set failed: {error}"))
}

fn get_secret(key: String) -> Result<Option<String>, String> {
    require_permission_message(Permission::SecureStorage)?;
    let normalized_key = normalize_key(key)?;
    let adapter = secure_storage_adapter()?;
    adapter
        .get(&normalized_key)
        .map_err(|error| format!("secureStorage.get failed: {error}"))
}

fn delete_secret(key: String) -> Result<(), String> {
    require_permission_message(Permission::SecureStorage)?;
    let normalized_key = normalize_key(key)?;
    let adapter = secure_storage_adapter()?;
    adapter
        .delete(&normalized_key)
        .map_err(|error| format!("secureStorage.delete failed: {error}"))
}

fn has_secret(key: String) -> Result<bool, String> {
    require_permission_message(Permission::SecureStorage)?;
    let normalized_key = normalize_key(key)?;
    let adapter = secure_storage_adapter()?;
    adapter
        .has(&normalized_key)
        .map_err(|error| format!("secureStorage.has failed: {error}"))
}

fn set(key: String, value: String, context: &mut Context) -> JsValue {
    promise_from_result(context, set_secret(key, value)).into()
}

fn get(key: String, context: &mut Context) -> JsValue {
    super::promise_from_json_result(
        context,
        get_secret(key).map(|value| serde_json::json!(value)),
    )
    .into()
}

fn delete(key: String, context: &mut Context) -> JsValue {
    promise_from_result(context, delete_secret(key)).into()
}

fn has(key: String, context: &mut Context) -> JsValue {
    promise_from_result(context, has_secret(key)).into()
}

pub fn build_module(context: &mut Context) -> Module {
    let set = set.into_js_function_copied(context);
    let get = get.into_js_function_copied(context);
    let delete = delete.into_js_function_copied(context);
    let has = has.into_js_function_copied(context);

    native_function_module(
        context,
        vec![("set", set), ("get", get), ("delete", delete), ("has", has)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_key_rejects_empty_and_long_values() {
        assert!(normalize_key("".to_string()).is_err());
        assert!(normalize_key("   ".to_string()).is_err());
        assert!(normalize_key("x".repeat(MAX_KEY_LENGTH + 1)).is_err());
        assert_eq!(
            normalize_key(" token ".to_string()).expect("trimmed key"),
            "token".to_string()
        );
    }

    #[test]
    fn secure_storage_crud_works_with_memory_backend() {
        crate::modules::configure(crate::modules::ModuleConfig {
            permissions: vec!["secureStorage".to_string()],
            secure_storage_backend: Some("memory".to_string()),
            ..Default::default()
        })
        .expect("configure module state");

        set_secret("token".to_string(), "secret".to_string()).expect("set secret");
        assert_eq!(
            get_secret("token".to_string()).expect("get secret"),
            Some("secret".to_string())
        );
        assert!(has_secret("token".to_string()).expect("has secret"));

        delete_secret("token".to_string()).expect("delete secret");
        assert_eq!(get_secret("token".to_string()).expect("after delete"), None);
        assert!(!has_secret("token".to_string()).expect("has after delete"));
    }

    #[test]
    fn set_secret_rejects_oversized_values() {
        crate::modules::configure(crate::modules::ModuleConfig {
            permissions: vec!["secureStorage".to_string()],
            secure_storage_backend: Some("memory".to_string()),
            ..Default::default()
        })
        .expect("configure module state");

        let oversized = "x".repeat(MAX_VALUE_LENGTH + 1);
        let err = set_secret("token".to_string(), oversized).expect_err("oversized value");
        assert!(err.contains("value length must be"));
    }

    #[test]
    fn invalid_key_returns_validation_errors() {
        crate::modules::configure(crate::modules::ModuleConfig {
            permissions: vec!["secureStorage".to_string()],
            secure_storage_backend: Some("memory".to_string()),
            ..Default::default()
        })
        .expect("configure module state");

        let set_error = set_secret("   ".to_string(), "secret".to_string()).expect_err("set error");
        assert!(set_error.contains("must not be empty"));

        let get_error = get_secret("".to_string()).expect_err("get error");
        assert!(get_error.contains("must not be empty"));

        let delete_error = delete_secret("".to_string()).expect_err("delete error");
        assert!(delete_error.contains("must not be empty"));

        let has_error = has_secret("".to_string()).expect_err("has error");
        assert!(has_error.contains("must not be empty"));
    }
}
