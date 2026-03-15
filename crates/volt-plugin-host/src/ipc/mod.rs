//! Plugin-side IPC protocol implementation.
//!
//! Framed JSON messages over stdin/stdout with 4-byte LE length prefix.
//! Handles message reading, writing, heartbeat responses, and lifecycle signals.

mod framing;
mod message;
#[cfg(test)]
mod tests;

pub use framing::{read_message, write_message};
pub use message::{IpcMessage, MessageType};
