use boa_engine::{Context, IntoJsFunctionCopied, Module};

use super::native_function_module;

fn platform() -> String {
    std::env::consts::OS.to_string()
}

fn arch() -> String {
    std::env::consts::ARCH.to_string()
}

fn home_dir() -> String {
    dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(std::path::PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(std::path::PathBuf::from))
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

pub fn build_module(context: &mut Context) -> Module {
    let platform = platform.into_js_function_copied(context);
    let arch = arch.into_js_function_copied(context);
    let home_dir = home_dir.into_js_function_copied(context);
    let temp_dir = temp_dir.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("platform", platform),
            ("arch", arch),
            ("homeDir", home_dir),
            ("tempDir", temp_dir),
        ],
    )
}
