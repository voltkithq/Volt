//! Plugin-side IPC protocol implementation.
//!
//! Framed JSON messages over stdin/stdout with 4-byte LE length prefix.
//! Handles message reading, writing, heartbeat responses, and lifecycle signals.

mod event_loop;
mod framing;
mod message;
#[cfg(test)]
mod tests;

pub use event_loop::run_ipc_loop;
pub use framing::write_message;
pub use message::IpcMessage;
