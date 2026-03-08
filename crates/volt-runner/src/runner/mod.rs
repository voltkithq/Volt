use std::fmt::{Display, Formatter};

pub mod assets;
pub mod config;
pub mod fs;
mod overrides;

#[derive(Debug)]
pub enum RunnerError {
    Io {
        context: String,
        source: std::io::Error,
    },
    Json(serde_json::Error),
    Config(String),
    AssetBundle(String),
    App(String),
}

impl Display for RunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Json(source) => write!(f, "invalid embedded config JSON: {source}"),
            Self::Config(message) => write!(f, "invalid runner config: {message}"),
            Self::AssetBundle(message) => write!(f, "invalid asset bundle: {message}"),
            Self::App(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RunnerError {}
