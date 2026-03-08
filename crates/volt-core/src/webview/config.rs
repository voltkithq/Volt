use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during webview operations.
#[derive(Error, Debug)]
pub enum WebViewError {
    #[error("failed to build webview: {0}")]
    Build(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("navigation blocked to: {0}")]
    NavigationBlocked(String),
}

/// Source content for the webview - either a URL or inline HTML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebViewSource {
    Url(String),
    Html(String),
}

/// Configuration for creating a webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebViewConfig {
    /// Content source: URL or HTML string.
    pub source: WebViewSource,

    /// Whether developer tools are available.
    #[serde(default)]
    pub devtools: bool,

    /// Whether the webview background should be transparent.
    #[serde(default)]
    pub transparent: bool,

    /// Custom user agent string.
    pub user_agent: Option<String>,

    /// Extra origins allowed for top-level navigation in addition to the app origin.
    #[serde(default)]
    pub allowed_origins: Vec<String>,

    /// IPC initialization script injected before page loads.
    pub init_script: Option<String>,
}

impl Default for WebViewConfig {
    fn default() -> Self {
        Self {
            source: WebViewSource::Html(String::from("<html><body><h1>Volt</h1></body></html>")),
            devtools: cfg!(debug_assertions),
            transparent: false,
            user_agent: None,
            allowed_origins: Vec::new(),
            init_script: None,
        }
    }
}
