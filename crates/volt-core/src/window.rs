mod build;
mod config;
mod handle;

pub use build::create_window;
pub use config::{WindowConfig, WindowError};
pub use handle::WindowHandle;

#[cfg(test)]
mod tests;
