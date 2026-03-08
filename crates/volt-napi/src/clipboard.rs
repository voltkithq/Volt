use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use volt_core::clipboard;
use volt_core::permissions::Permission;

use crate::permissions::require_permission;

/// Read text from the system clipboard.
#[napi]
pub fn clipboard_read_text() -> napi::Result<String> {
    require_permission(Permission::Clipboard)?;
    clipboard::read_text()
        .map_err(|e| napi::Error::from_reason(format!("Clipboard read failed: {e}")))
}

/// Write text to the system clipboard.
#[napi]
pub fn clipboard_write_text(text: String) -> napi::Result<()> {
    require_permission(Permission::Clipboard)?;
    clipboard::write_text(&text)
        .map_err(|e| napi::Error::from_reason(format!("Clipboard write failed: {e}")))
}

/// Image data returned from clipboard operations.
#[napi(object)]
pub struct VoltImageData {
    /// Raw RGBA pixel data.
    pub rgba: Buffer,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Read an image from the system clipboard.
#[napi]
pub fn clipboard_read_image() -> napi::Result<VoltImageData> {
    require_permission(Permission::Clipboard)?;
    let img = clipboard::read_image()
        .map_err(|e| napi::Error::from_reason(format!("Clipboard read image failed: {e}")))?;

    Ok(VoltImageData {
        rgba: img.rgba.into(),
        width: img.width,
        height: img.height,
    })
}

/// Write an image to the system clipboard.
#[napi]
pub fn clipboard_write_image(data: VoltImageData) -> napi::Result<()> {
    require_permission(Permission::Clipboard)?;
    let img = clipboard::ImageData {
        rgba: data.rgba.to_vec(),
        width: data.width,
        height: data.height,
    };
    clipboard::write_image(&img)
        .map_err(|e| napi::Error::from_reason(format!("Clipboard write image failed: {e}")))
}
