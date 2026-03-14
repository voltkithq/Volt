use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("protocol not allowed: {0}")]
    ProtocolNotAllowed(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("failed to open URL: {0}")]
    OpenFailed(String),
}

/// Allowed URL schemes for shell.openExternal().
const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// Open a URL in the default system application.
/// SECURITY: Only allows http, https, and mailto schemes.
/// Rejects file://, smb://, javascript:, data:, vbscript:, cmd:, etc.
pub fn open_external(url_str: &str) -> Result<(), ShellError> {
    let parsed =
        Url::parse(url_str).map_err(|e| ShellError::InvalidUrl(format!("{url_str}: {e}")))?;

    let scheme = parsed.scheme();
    if !ALLOWED_SCHEMES.contains(&scheme) {
        return Err(ShellError::ProtocolNotAllowed(format!(
            "'{scheme}' is not allowed. Only http, https, and mailto are permitted."
        )));
    }

    // Use the OS-specific open command
    #[cfg(target_os = "windows")]
    {
        // Avoid `cmd /C start` to prevent shell metacharacter injection.
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url_str])
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url_str)
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url_str)
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    Ok(())
}

/// Reveal a file or directory in the platform file manager.
/// The path must be an absolute, canonicalized path that has already been validated
/// against the app's filesystem scope.
pub fn show_item_in_folder(path: &std::path::Path) -> Result<(), ShellError> {
    if !path.exists() {
        return Err(ShellError::OpenFailed(format!(
            "path does not exist: '{}'",
            path.display()
        )));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    #[cfg(target_os = "linux")]
    {
        let folder = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| ShellError::OpenFailed(e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_allowed() {
        // We don't actually open the URL in tests, just validate the scheme
        let parsed = Url::parse("https://example.com").unwrap();
        assert!(ALLOWED_SCHEMES.contains(&parsed.scheme()));
    }

    #[test]
    fn test_file_blocked() {
        let result = open_external("file:///etc/passwd");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShellError::ProtocolNotAllowed(_)
        ));
    }

    #[test]
    fn test_javascript_blocked() {
        let result = open_external("javascript:alert(1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_data_blocked() {
        let result = open_external("data:text/html,<script>alert(1)</script>");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url() {
        let result = open_external("not a url");
        assert!(result.is_err());
    }

    // ── Expanded tests ─────────────────────────────────────────────

    #[test]
    fn test_http_scheme_check() {
        let parsed = Url::parse("http://example.com").unwrap();
        assert!(ALLOWED_SCHEMES.contains(&parsed.scheme()));
    }

    #[test]
    fn test_mailto_scheme_allowed() {
        let parsed = Url::parse("mailto:user@example.com").unwrap();
        assert!(ALLOWED_SCHEMES.contains(&parsed.scheme()));
    }

    #[test]
    fn test_smb_blocked() {
        let result = open_external("smb://server/share");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShellError::ProtocolNotAllowed(_)
        ));
    }

    #[test]
    fn test_ftp_blocked() {
        let result = open_external("ftp://files.example.com/data.zip");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShellError::ProtocolNotAllowed(_)
        ));
    }

    #[test]
    fn test_vbscript_blocked() {
        let result = open_external("vbscript:msgbox");
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_error_protocol_not_allowed_display() {
        let e = ShellError::ProtocolNotAllowed("file is blocked".into());
        let msg = e.to_string();
        assert!(msg.contains("protocol not allowed"));
        assert!(msg.contains("file is blocked"));
    }

    #[test]
    fn test_shell_error_invalid_url_display() {
        let e = ShellError::InvalidUrl("garbage".into());
        assert!(e.to_string().contains("garbage"));
    }

    #[test]
    fn test_shell_error_open_failed_display() {
        let e = ShellError::OpenFailed("no browser found".into());
        assert!(e.to_string().contains("no browser found"));
    }
}
