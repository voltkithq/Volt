use boa_engine::{Context, JsValue};
use serde_json::{Value, json};
use volt_core::updater::UpdateInfo;

pub(crate) fn update_info_to_json(info: UpdateInfo) -> Value {
    json!({
        "version": info.version,
        "url": info.url,
        "signature": info.signature,
        "sha256": info.sha256,
    })
}

pub(crate) fn json_to_js_value(context: &mut Context, value: &Value) -> Result<JsValue, String> {
    crate::modules::json_to_js_value(value, context)
}
