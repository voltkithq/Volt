mod helpers;
mod resolve;
mod scoped;

use thiserror::Error;

pub use resolve::{safe_resolve, safe_resolve_for_create};
pub use scoped::{
    copy, exists, mkdir, read_dir, read_file, read_file_text, remove, rename, replace_file, stat,
    write_file,
};

#[derive(Error, Debug)]
pub enum FsError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path security violation: {0}")]
    Security(String),

    #[error("path is outside the allowed scope")]
    OutOfScope,
}

/// File metadata info returned by stat().
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub readonly: bool,
    /// Last modification time as milliseconds since Unix epoch.
    pub modified_ms: f64,
    /// Creation time as milliseconds since Unix epoch.
    /// `None` on platforms/filesystems that do not support birth time.
    pub created_ms: Option<f64>,
}

#[cfg(test)]
mod tests;
