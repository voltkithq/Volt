use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use boa_engine::{Context, IntoJsFunctionCopied, JsResult, Module};
use sha2::{Digest, Sha256};

use super::{js_error, native_function_module};

fn sha256(data: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let digest = hasher.finalize();
    hex_encode(&digest)
}

fn base64_encode(data: String) -> String {
    STANDARD.encode(data.as_bytes())
}

fn base64_decode(data: String) -> JsResult<String> {
    let decoded = STANDARD.decode(data.as_bytes()).map_err(|error| {
        js_error(
            "volt:crypto",
            "base64Decode",
            format!("base64 decode failed: {error}"),
        )
    })?;

    String::from_utf8(decoded).map_err(|error| {
        js_error(
            "volt:crypto",
            "base64Decode",
            format!("decoded data is not valid UTF-8: {error}"),
        )
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub fn build_module(context: &mut Context) -> Module {
    let sha256 = sha256.into_js_function_copied(context);
    let base64_encode = base64_encode.into_js_function_copied(context);
    let base64_decode = base64_decode.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("sha256", sha256),
            ("base64Encode", base64_encode),
            ("base64Decode", base64_decode),
        ],
    )
}
