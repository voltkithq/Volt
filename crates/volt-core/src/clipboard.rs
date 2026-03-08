use arboard::Clipboard;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("clipboard operation failed: {0}")]
    Operation(String),

    #[error("clipboard data too large: {0} bytes (max: {1})")]
    TooLarge(usize, usize),

    #[error(
        "RGBA buffer size mismatch: expected {expected} bytes ({width}x{height}x4), got {actual}"
    )]
    DimensionMismatch {
        expected: usize,
        actual: usize,
        width: u32,
        height: u32,
    },

    #[error("image dimensions overflow while calculating RGBA buffer size")]
    DimensionOverflow,
}

/// Maximum image data size (10 MB) to prevent OOM.
const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Read text from the system clipboard.
pub fn read_text() -> Result<String, ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::Operation(e.to_string()))?;
    clipboard
        .get_text()
        .map_err(|e| ClipboardError::Operation(e.to_string()))
}

/// Write text to the system clipboard.
pub fn write_text(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::Operation(e.to_string()))?;
    clipboard
        .set_text(text)
        .map_err(|e| ClipboardError::Operation(e.to_string()))
}

/// Image data with RGBA pixel bytes and dimensions.
pub struct ImageData {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Read an image from the system clipboard.
pub fn read_image() -> Result<ImageData, ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::Operation(e.to_string()))?;
    let img = clipboard
        .get_image()
        .map_err(|e| ClipboardError::Operation(e.to_string()))?;

    let size = img.bytes.len();
    if size > MAX_IMAGE_SIZE {
        return Err(ClipboardError::TooLarge(size, MAX_IMAGE_SIZE));
    }

    Ok(ImageData {
        rgba: img.bytes.into_owned(),
        width: img.width as u32,
        height: img.height as u32,
    })
}

/// Write an image to the system clipboard.
pub fn write_image(data: &ImageData) -> Result<(), ClipboardError> {
    if data.rgba.len() > MAX_IMAGE_SIZE {
        return Err(ClipboardError::TooLarge(data.rgba.len(), MAX_IMAGE_SIZE));
    }

    let expected = (data.width as usize)
        .checked_mul(data.height as usize)
        .and_then(|v| v.checked_mul(4))
        .ok_or(ClipboardError::DimensionOverflow)?;
    if data.rgba.len() != expected {
        return Err(ClipboardError::DimensionMismatch {
            expected,
            actual: data.rgba.len(),
            width: data.width,
            height: data.height,
        });
    }

    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::Operation(e.to_string()))?;
    let img = arboard::ImageData {
        width: data.width as usize,
        height: data.height as usize,
        bytes: std::borrow::Cow::Borrowed(&data.rgba),
    };
    clipboard
        .set_image(img)
        .map_err(|e| ClipboardError::Operation(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_size_limit() {
        let oversized = ImageData {
            rgba: vec![0u8; MAX_IMAGE_SIZE + 1],
            width: 100,
            height: 100,
        };
        assert!(write_image(&oversized).is_err());
    }

    // ── Expanded tests ─────────────────────────────────────────────

    #[test]
    fn test_image_size_exactly_at_limit() {
        // MAX_IMAGE_SIZE bytes with matching dimensions
        // width * height * 4 (RGBA) = MAX_IMAGE_SIZE
        let pixels = MAX_IMAGE_SIZE / 4;
        let at_limit = ImageData {
            rgba: vec![0u8; MAX_IMAGE_SIZE],
            width: pixels as u32,
            height: 1,
        };
        // Should NOT fail the size check (may fail on clipboard access, but not TooLarge)
        let result = write_image(&at_limit);
        if let Err(e) = result {
            assert!(!matches!(e, ClipboardError::TooLarge(_, _)));
        }
    }

    #[test]
    fn test_image_size_zero() {
        let empty = ImageData {
            rgba: vec![],
            width: 0,
            height: 0,
        };
        // Zero-size should pass the size check
        let result = write_image(&empty);
        if let Err(e) = result {
            assert!(!matches!(e, ClipboardError::TooLarge(_, _)));
        }
    }

    #[test]
    fn test_dimension_mismatch_rejected() {
        // 10x10 image should need 400 bytes (10*10*4), but we provide 100
        let mismatched = ImageData {
            rgba: vec![0u8; 100],
            width: 10,
            height: 10,
        };
        let result = write_image(&mismatched);
        assert!(result.is_err());
        match result.unwrap_err() {
            ClipboardError::DimensionMismatch {
                expected,
                actual,
                width,
                height,
            } => {
                assert_eq!(expected, 400);
                assert_eq!(actual, 100);
                assert_eq!(width, 10);
                assert_eq!(height, 10);
            }
            other => panic!("expected DimensionMismatch, got: {other}"),
        }
    }

    #[test]
    fn test_dimension_mismatch_display() {
        let e = ClipboardError::DimensionMismatch {
            expected: 400,
            actual: 100,
            width: 10,
            height: 10,
        };
        let msg = e.to_string();
        assert!(msg.contains("400"));
        assert!(msg.contains("100"));
        assert!(msg.contains("10x10x4"));
    }

    #[test]
    fn test_clipboard_error_operation_display() {
        let e = ClipboardError::Operation("no clipboard backend".into());
        let msg = e.to_string();
        assert!(msg.contains("clipboard operation"));
        assert!(msg.contains("no clipboard backend"));
    }

    #[test]
    fn test_clipboard_error_too_large_display() {
        let e = ClipboardError::TooLarge(20_000_000, MAX_IMAGE_SIZE);
        let msg = e.to_string();
        assert!(msg.contains("20000000"));
        assert!(msg.contains(&MAX_IMAGE_SIZE.to_string()));
    }

    #[test]
    fn test_dimension_overflow_rejected() {
        let huge = ImageData {
            rgba: vec![],
            width: u32::MAX,
            height: u32::MAX,
        };
        let result = write_image(&huge);
        assert!(matches!(result, Err(ClipboardError::DimensionOverflow)));
    }
}
