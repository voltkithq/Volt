use std::io::Read;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::{JsObject, ObjectInitializer};
use boa_engine::property::Attribute;
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use serde_json::Value;

use crate::modules::{format_js_error, json_to_js_value, resolve_promise};

use super::constants::{HTTP_MAX_RESPONSE_BODY_BYTES, RESPONSE_BODY_PROPERTY, ResponseHeaders};

pub(crate) fn read_response_body_with_limit(
    response: reqwest::blocking::Response,
) -> Result<String, String> {
    if let Some(content_length) = response.content_length()
        && content_length > HTTP_MAX_RESPONSE_BODY_BYTES as u64
    {
        return Err(format!(
            "HTTP response body exceeds {} bytes",
            HTTP_MAX_RESPONSE_BODY_BYTES
        ));
    }

    let mut body_buffer = Vec::new();
    response
        .take((HTTP_MAX_RESPONSE_BODY_BYTES as u64) + 1)
        .read_to_end(&mut body_buffer)
        .map_err(|err| format!("failed to read HTTP response body: {err}"))?;

    if body_buffer.len() > HTTP_MAX_RESPONSE_BODY_BYTES {
        return Err(format!(
            "HTTP response body exceeds {} bytes",
            HTTP_MAX_RESPONSE_BODY_BYTES
        ));
    }

    Ok(String::from_utf8_lossy(&body_buffer).into_owned())
}

pub(super) fn build_response_object(
    context: &mut Context,
    status: i32,
    headers: ResponseHeaders,
    body: String,
) -> Result<JsObject, String> {
    let headers = serde_json::to_value(headers)
        .map_err(|error| format!("failed to serialize response headers: {error}"))?;
    let headers = json_to_js_value(&headers, context)?;

    let mut response = ObjectInitializer::new(context);
    response.property(js_string!("status"), status, Attribute::all());
    response.property(js_string!("headers"), headers, Attribute::all());
    response.property(
        js_string!(RESPONSE_BODY_PROPERTY),
        JsValue::from(js_string!(body.as_str())),
        Attribute::default(),
    );
    response.function(
        NativeFunction::from_fn_ptr(response_text),
        js_string!("text"),
        0,
    );
    response.function(
        NativeFunction::from_fn_ptr(response_json),
        js_string!("json"),
        0,
    );

    Ok(response.build())
}

fn response_text(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_body(this, context).map_err(http_response_error("text"))?;
    Ok(resolve_promise(context, JsValue::from(js_string!(body.as_str()))).into())
}

fn response_json(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_body(this, context).map_err(http_response_error("json"))?;
    let parsed = serde_json::from_str::<Value>(&body).map_err(|error| {
        super::js_error("json", format!("response body is not valid JSON: {error}"))
    })?;
    let value = json_to_js_value(&parsed, context).map_err(|error| {
        super::js_error("json", format!("failed to convert JSON response: {error}"))
    })?;
    Ok(resolve_promise(context, value).into())
}

fn response_body(this: &JsValue, context: &mut Context) -> Result<String, String> {
    let object = this
        .as_object()
        .ok_or_else(|| "response object is not available".to_string())?;
    let body = object
        .get(js_string!(RESPONSE_BODY_PROPERTY), context)
        .map_err(format_js_error)?;
    body.to_string(context)
        .map(|value| value.to_std_string_escaped())
        .map_err(format_js_error)
}

fn http_response_error(function: &'static str) -> impl Fn(String) -> JsError + Copy {
    move |message| super::js_error(function, message)
}

pub(crate) fn response_header_value(value: &reqwest::header::HeaderValue) -> String {
    match value.to_str() {
        Ok(text) => text.to_string(),
        Err(_) => String::from_utf8_lossy(value.as_bytes()).into_owned(),
    }
}
