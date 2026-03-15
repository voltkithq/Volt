use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};

use super::{native_function_module, plugin_manager, promise_from_result};

fn delegate_grant(plugin_id: String, grant_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, delegate_grant_result(plugin_id, grant_id)).into()
}

fn revoke_grant(plugin_id: String, grant_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, revoke_grant_result(plugin_id, grant_id)).into()
}

fn prefetch_for(surface: String, context: &mut Context) -> JsValue {
    promise_from_result(context, prefetch_for_result(surface)).into()
}

fn delegate_grant_result(plugin_id: String, grant_id: String) -> Result<(), String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    let grant_id = required_name(grant_id, "grant id")?;
    plugin_manager()?
        .delegate_grant(&plugin_id, &grant_id)
        .map_err(|error| error.to_string())
}

fn revoke_grant_result(plugin_id: String, grant_id: String) -> Result<(), String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    let grant_id = required_name(grant_id, "grant id")?;
    plugin_manager()?
        .revoke_grant(&plugin_id, &grant_id)
        .map_err(|error| error.to_string())
}

fn prefetch_for_result(surface: String) -> Result<(), String> {
    plugin_manager()?.prefetch_for(&required_name(surface, "surface")?);
    Ok(())
}

fn required_name(value: String, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(trimmed.to_string())
}

pub fn build_module(context: &mut Context) -> Module {
    let delegate_grant = delegate_grant.into_js_function_copied(context);
    let revoke_grant = revoke_grant.into_js_function_copied(context);
    let prefetch_for = prefetch_for.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("delegateGrant", delegate_grant),
            ("revokeGrant", revoke_grant),
            ("prefetchFor", prefetch_for),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{ModuleConfig, configure};

    #[test]
    fn missing_plugin_manager_is_reported() {
        configure(ModuleConfig::default()).expect("configure");

        let error = delegate_grant_result("acme.search".to_string(), "grant-1".to_string())
            .expect_err("missing plugin manager");

        assert!(error.contains("plugin manager is unavailable"));
    }

    #[test]
    fn prefetch_for_without_plugin_manager_returns_error() {
        configure(ModuleConfig::default()).expect("configure");

        let error = prefetch_for_result("search-panel".to_string()).expect_err("missing manager");

        assert!(error.contains("plugin manager is unavailable"));
    }
}
