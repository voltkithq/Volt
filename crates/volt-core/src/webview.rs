use std::borrow::Cow;
use std::sync::Arc;

use url::Url;
use wry::WebViewBuilder;
use wry::http::{Request, Response};

use crate::embed::{self, AssetBundle};
use crate::ipc::IPC_MAX_REQUEST_BYTES;

mod config;
mod policy;
#[cfg(test)]
mod tests;

pub use config::{WebViewConfig, WebViewError, WebViewSource};

/// Create a wry WebView attached to the given tao Window.
///
/// `asset_bundle` is used by the `volt://` custom protocol to serve embedded assets.
/// Pass `None` during development (the webview loads from the Vite dev server URL instead).
pub fn create_webview(
    window: &tao::window::Window,
    config: &WebViewConfig,
    enable_devtools: bool,
    asset_bundle: Option<Arc<AssetBundle>>,
    js_window_id: String,
) -> Result<wry::WebView, WebViewError> {
    let navigation_origins = policy::navigation_origins_for(config);
    let mut builder =
        apply_devtools_config(WebViewBuilder::new(), enable_devtools || config.devtools)
            .with_transparent(config.transparent)
            .with_navigation_handler(move |url| {
                policy::is_origin_allowed(&url, &navigation_origins)
            });

    // Always inject the Volt IPC bridge.
    let ipc_init = crate::ipc::ipc_init_script();
    builder = builder.with_initialization_script(&ipc_init);

    if let Some(ref script) = config.init_script {
        builder = builder.with_initialization_script(script);
    }

    if let Some(ref ua) = config.user_agent {
        builder = builder.with_user_agent(ua);
    }

    builder = match &config.source {
        WebViewSource::Url(url) => {
            Url::parse(url).map_err(|e| WebViewError::InvalidUrl(format!("{url}: {e}")))?;
            builder.with_url(url)
        }
        WebViewSource::Html(html) => builder.with_html(html),
    };

    let bundle = asset_bundle;
    let ipc_window_id = js_window_id;
    builder = builder.with_ipc_handler(move |request: Request<String>| {
        let raw = request.into_body();
        if raw.len() > IPC_MAX_REQUEST_BYTES {
            let script = crate::ipc::payload_too_large_response_script(&raw);
            let _ = crate::command::send_command(crate::command::AppCommand::EvaluateScript {
                js_id: ipc_window_id.clone(),
                script,
            });
            return;
        }

        let _ = crate::command::send_command(crate::command::AppCommand::IpcMessage {
            js_window_id: ipc_window_id.clone(),
            raw,
        });
    });

    builder = builder.with_custom_protocol(
        "volt".to_string(),
        move |_webview_id: wry::WebViewId<'_>,
              request: Request<Vec<u8>>|
              -> Response<Cow<'static, [u8]>> {
            if let Some(ref bundle) = bundle {
                let path = request.uri().path();
                embed::serve_asset(bundle, path)
            } else {
                Response::builder()
                    .status(404)
                    .header("Content-Type", "text/plain")
                    .body(Cow::Borrowed(b"Not Found" as &[u8]))
                    .expect("failed to build response")
            }
        },
    );

    builder
        .build(window)
        .map_err(|e| WebViewError::Build(e.to_string()))
}

#[cfg(feature = "devtools")]
fn apply_devtools_config(builder: WebViewBuilder, enabled: bool) -> WebViewBuilder {
    builder.with_devtools(enabled)
}

#[cfg(not(feature = "devtools"))]
fn apply_devtools_config(builder: WebViewBuilder, _enabled: bool) -> WebViewBuilder {
    builder
}
