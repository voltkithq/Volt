use boa_engine::object::builtins::JsFunction;
use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use serde_json::Value;

use super::{
    bind_native_event_handler, js_error, native_function_module, plugin_manager,
    promise_from_json_result, promise_from_result,
};

const NATIVE_EVENT_ON_GLOBAL: &str = "__volt_native_event_on__";
const NATIVE_EVENT_OFF_GLOBAL: &str = "__volt_native_event_off__";

fn delegate_grant(plugin_id: String, grant_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, delegate_grant_result(plugin_id, grant_id)).into()
}

fn revoke_grant(plugin_id: String, grant_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, revoke_grant_result(plugin_id, grant_id)).into()
}

fn prefetch_for(surface: String, context: &mut Context) -> JsValue {
    promise_from_result(context, prefetch_for_result(surface)).into()
}

fn get_states(context: &mut Context) -> JsValue {
    promise_from_json_result(context, get_states_result()).into()
}

fn get_plugin_state(plugin_id: String, context: &mut Context) -> JsValue {
    promise_from_json_result(context, get_plugin_state_result(plugin_id)).into()
}

fn get_errors(context: &mut Context) -> JsValue {
    promise_from_json_result(context, get_errors_result()).into()
}

fn get_plugin_errors(plugin_id: String, context: &mut Context) -> JsValue {
    promise_from_json_result(context, get_plugin_errors_result(plugin_id)).into()
}

fn get_discovery_issues(context: &mut Context) -> JsValue {
    promise_from_json_result(context, get_discovery_issues_result()).into()
}

fn retry_plugin(plugin_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, retry_plugin_result(plugin_id)).into()
}

fn enable_plugin(plugin_id: String, context: &mut Context) -> JsValue {
    promise_from_result(context, enable_plugin_result(plugin_id)).into()
}

fn on(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_plugin_event(context, "on", NATIVE_EVENT_ON_GLOBAL, event_name, handler)
}

fn off(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_plugin_event(context, "off", NATIVE_EVENT_OFF_GLOBAL, event_name, handler)
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

fn get_states_result() -> Result<Value, String> {
    serde_json::to_value(plugin_manager()?.get_states()).map_err(|error| error.to_string())
}

fn get_plugin_state_result(plugin_id: String) -> Result<Value, String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    serde_json::to_value(plugin_manager()?.get_plugin_state(&plugin_id))
        .map_err(|error| error.to_string())
}

fn get_errors_result() -> Result<Value, String> {
    serde_json::to_value(plugin_manager()?.get_errors()).map_err(|error| error.to_string())
}

fn get_plugin_errors_result(plugin_id: String) -> Result<Value, String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    serde_json::to_value(plugin_manager()?.get_plugin_errors(&plugin_id))
        .map_err(|error| error.to_string())
}

fn get_discovery_issues_result() -> Result<Value, String> {
    serde_json::to_value(plugin_manager()?.discovery_issues()).map_err(|error| error.to_string())
}

fn retry_plugin_result(plugin_id: String) -> Result<(), String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    plugin_manager()?
        .retry_plugin(&plugin_id)
        .map_err(|error| error.to_string())
}

fn enable_plugin_result(plugin_id: String) -> Result<(), String> {
    let plugin_id = required_name(plugin_id, "plugin id")?;
    plugin_manager()?
        .enable_plugin(&plugin_id)
        .map_err(|error| error.to_string())
}

fn bind_native_plugin_event(
    context: &mut Context,
    api_function: &'static str,
    global_name: &'static str,
    event_name: String,
    handler: JsFunction,
) -> JsResult<()> {
    bind_native_event_handler(
        context,
        "volt:plugins",
        api_function,
        global_name,
        normalize_event_name(event_name)
            .map_err(|error| js_error("volt:plugins", api_function, error))?,
        handler,
    )
}

fn normalize_event_name(event_name: String) -> Result<&'static str, String> {
    match event_name.trim() {
        "plugin:lifecycle" => Ok("plugin:lifecycle"),
        "plugin:failed" => Ok("plugin:failed"),
        "plugin:activated" => Ok("plugin:activated"),
        "" => Err("plugin event name must not be empty".to_string()),
        other => Err(format!("unsupported plugin event '{other}'")),
    }
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
    let get_states = get_states.into_js_function_copied(context);
    let get_plugin_state = get_plugin_state.into_js_function_copied(context);
    let get_errors = get_errors.into_js_function_copied(context);
    let get_plugin_errors = get_plugin_errors.into_js_function_copied(context);
    let get_discovery_issues = get_discovery_issues.into_js_function_copied(context);
    let retry_plugin = retry_plugin.into_js_function_copied(context);
    let enable_plugin = enable_plugin.into_js_function_copied(context);
    let on = on.into_js_function_copied(context);
    let off = off.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("delegateGrant", delegate_grant),
            ("revokeGrant", revoke_grant),
            ("prefetchFor", prefetch_for),
            ("getStates", get_states),
            ("getPluginState", get_plugin_state),
            ("getErrors", get_errors),
            ("getPluginErrors", get_plugin_errors),
            ("getDiscoveryIssues", get_discovery_issues),
            ("retryPlugin", retry_plugin),
            ("enablePlugin", enable_plugin),
            ("on", on),
            ("off", off),
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

    #[test]
    fn normalize_event_name_accepts_supported_events() {
        assert_eq!(
            normalize_event_name("plugin:lifecycle".to_string()),
            Ok("plugin:lifecycle")
        );
        assert_eq!(
            normalize_event_name("plugin:failed".to_string()),
            Ok("plugin:failed")
        );
        assert_eq!(
            normalize_event_name("plugin:activated".to_string()),
            Ok("plugin:activated")
        );
    }
}
