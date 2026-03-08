//! Asset embedding and serving for the `volt://` custom protocol.
//!
//! In development, the WebView loads from the Vite dev server (http://localhost).
//! In production, assets are embedded in the binary and served via `volt://localhost/`.

mod bundle;
mod fs;
mod serve;
#[cfg(test)]
mod tests;

pub use bundle::AssetBundle;
pub use serve::{mime_type_for_path, serve_asset};
