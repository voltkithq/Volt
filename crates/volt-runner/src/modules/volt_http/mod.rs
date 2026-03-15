mod constants;
mod request;
mod response;
mod ssrf;
#[cfg(test)]
mod tests;
mod transport;

use boa_engine::{Context, IntoJsFunctionCopied, JsError, JsValue, Module};
use volt_core::permissions::Permission;

use crate::modules::{
    format_js_error, native_function_module, promise_from_result, require_permission,
};

use self::request::parse_fetch_request;
use self::transport::fetch_impl;

fn fetch(input: JsValue, options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Http).map_err(format_js_error)?;
        let request = parse_fetch_request(input, options, context)?;
        fetch_impl(request, context)
    })();

    promise_from_result(context, result).into()
}

fn allow_private_networks_for_tests() -> bool {
    cfg!(test)
}

fn js_error(function: &'static str, message: impl Into<String>) -> JsError {
    crate::modules::js_error("volt:http", function, message)
}

pub fn build_module(context: &mut Context) -> Module {
    let fetch = fetch.into_js_function_copied(context);
    native_function_module(context, vec![("fetch", fetch)])
}

#[cfg(test)]
pub(crate) use self::constants::{HTTP_MAX_REQUEST_BODY_BYTES, HTTP_MAX_RESPONSE_BODY_BYTES};
#[cfg(test)]
pub(crate) use self::request::{parse_body, parse_fetch_request_json, parse_headers};
#[cfg(test)]
pub(crate) use self::response::{read_response_body_with_limit, response_header_value};
#[cfg(test)]
pub(crate) use self::ssrf::normalize_request_url;
#[cfg(test)]
pub(crate) use self::transport::try_acquire_inflight_slot;
