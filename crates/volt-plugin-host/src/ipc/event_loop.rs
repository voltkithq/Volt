use std::io::{self, BufReader, Read, Write};

use super::framing::{read_message, write_message};
use super::message::{IpcMessage, MessageType};

/// The main IPC event loop for the plugin process.
///
/// Reads messages from stdin, dispatches them, and writes responses to stdout.
/// Exits when stdin is closed (EOF) or a deactivate signal is received.
pub fn run_ipc_loop<R: Read, W: Write>(
    reader: &mut BufReader<R>,
    writer: &mut W,
) -> io::Result<()> {
    loop {
        let msg = match read_message(reader)? {
            Some(message) => message,
            None => {
                tracing::info!("stdin EOF - exiting IPC loop");
                return Ok(());
            }
        };

        match (&msg.msg_type, msg.method.as_str()) {
            (MessageType::Signal, "heartbeat") => {
                let ack = IpcMessage::signal(&msg.id, "heartbeat-ack");
                write_message(writer, &ack)?;
            }
            (MessageType::Signal, "deactivate") => {
                tracing::info!("received deactivate signal - exiting");
                return Ok(());
            }
            (MessageType::Signal, "cancel") => {
                tracing::debug!(id = %msg.id, "received cancel signal (no-op in skeleton)");
            }
            (MessageType::Signal, "activate") => {
                tracing::info!("received activate signal");
                let response = IpcMessage::response(&msg.id, "activate", None);
                write_message(writer, &response)?;
            }
            (MessageType::Request, method) => {
                tracing::debug!(method = %method, id = %msg.id, "received request (unhandled in skeleton)");
                let response = IpcMessage::error_response(
                    &msg.id,
                    method,
                    "UNHANDLED",
                    format!("no handler registered for method '{method}'"),
                );
                write_message(writer, &response)?;
            }
            (MessageType::Response, _) => {
                tracing::debug!(id = %msg.id, method = %msg.method, "received response (no pending requests in skeleton)");
            }
            (MessageType::Event, method) => {
                tracing::debug!(method = %method, "received event (ignored in skeleton)");
            }
            _ => {
                tracing::warn!(msg_type = ?msg.msg_type, method = %msg.method, "unexpected message");
            }
        }
    }
}
