//! Plugin-side IPC protocol implementation.
//!
//! Framed JSON messages over stdin/stdout with 4-byte LE length prefix.
//! Handles message reading, writing, heartbeat responses, and lifecycle signals.

use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, Read, Write};

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Message types in the plugin-host IPC protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Request,
    Response,
    Event,
    Signal,
}

/// Error payload in an IPC message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

/// A single IPC message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub id: String,
    pub method: String,
    pub payload: Option<serde_json::Value>,
    pub error: Option<IpcError>,
}

impl IpcMessage {
    pub fn signal(id: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Signal,
            id: id.into(),
            method: method.into(),
            payload: None,
            error: None,
        }
    }

    pub fn response(
        id: impl Into<String>,
        method: impl Into<String>,
        payload: Option<serde_json::Value>,
    ) -> Self {
        Self {
            msg_type: MessageType::Response,
            id: id.into(),
            method: method.into(),
            payload,
            error: None,
        }
    }

    pub fn error_response(
        id: impl Into<String>,
        method: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            msg_type: MessageType::Response,
            id: id.into(),
            method: method.into(),
            payload: None,
            error: Some(IpcError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

/// Write a framed message to the given writer (stdout).
///
/// Format: `<4-byte LE length><JSON bytes>\n`
pub fn write_message<W: Write>(writer: &mut W, msg: &IpcMessage) -> io::Result<()> {
    let json =
        serde_json::to_string(msg).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let body = format!("{json}\n");
    let body_bytes = body.as_bytes();
    if body_bytes.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "frame too large: {} bytes exceeds {} byte limit",
                body_bytes.len(),
                MAX_FRAME_SIZE
            ),
        ));
    }
    let len = body_bytes.len() as u32;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(body_bytes)?;
    writer.flush()
}

/// Read a single framed message from the given reader (stdin).
///
/// Returns `None` on EOF (stdin closed).
pub fn read_message<R: Read>(reader: &mut BufReader<R>) -> io::Result<Option<IpcMessage>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid frame length: {len}"),
        ));
    }

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;

    let raw = String::from_utf8(body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let trimmed = raw.trim_end_matches('\n');
    let msg: IpcMessage =
        serde_json::from_str(trimmed).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(msg))
}

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
            Some(m) => m,
            None => {
                tracing::info!("stdin EOF — exiting IPC loop");
                return Ok(());
            }
        };

        match (&msg.msg_type, msg.method.as_str()) {
            (MessageType::Signal, "heartbeat") => {
                let ack = IpcMessage::signal(&msg.id, "heartbeat-ack");
                write_message(writer, &ack)?;
            }
            (MessageType::Signal, "deactivate") => {
                tracing::info!("received deactivate signal — exiting");
                return Ok(());
            }
            (MessageType::Signal, "cancel") => {
                tracing::debug!(id = %msg.id, "received cancel signal (no-op in skeleton)");
            }
            (MessageType::Signal, "activate") => {
                tracing::info!("received activate signal");
                let resp = IpcMessage::response(&msg.id, "activate", None);
                write_message(writer, &resp)?;
            }
            (MessageType::Request, method) => {
                tracing::debug!(method = %method, id = %msg.id, "received request (unhandled in skeleton)");
                let resp = IpcMessage::error_response(
                    &msg.id,
                    method,
                    "UNHANDLED",
                    format!("no handler registered for method '{method}'"),
                );
                write_message(writer, &resp)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn roundtrip(msg: &IpcMessage) -> IpcMessage {
        let mut buf = Vec::new();
        write_message(&mut buf, msg).unwrap();
        let mut reader = BufReader::new(Cursor::new(buf));
        read_message(&mut reader).unwrap().unwrap()
    }

    #[test]
    fn test_signal_roundtrip() {
        let msg = IpcMessage::signal("test-id", "ready");
        let parsed = roundtrip(&msg);
        assert_eq!(parsed.msg_type, MessageType::Signal);
        assert_eq!(parsed.id, "test-id");
        assert_eq!(parsed.method, "ready");
        assert!(parsed.payload.is_none());
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_response_roundtrip() {
        let msg = IpcMessage::response(
            "resp-1",
            "getData",
            Some(serde_json::json!({"key": "value"})),
        );
        let parsed = roundtrip(&msg);
        assert_eq!(parsed.msg_type, MessageType::Response);
        assert_eq!(parsed.method, "getData");
        assert_eq!(parsed.payload.unwrap(), serde_json::json!({"key": "value"}));
    }

    #[test]
    fn test_error_response_roundtrip() {
        let msg = IpcMessage::error_response("e-1", "doStuff", "FAILED", "something broke");
        let parsed = roundtrip(&msg);
        let err = parsed.error.unwrap();
        assert_eq!(err.code, "FAILED");
        assert_eq!(err.message, "something broke");
    }

    #[test]
    fn test_read_eof() {
        let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn test_multiple_messages_in_sequence() {
        let msgs = vec![
            IpcMessage::signal("1", "heartbeat"),
            IpcMessage::signal("2", "heartbeat-ack"),
            IpcMessage::signal("3", "ready"),
        ];
        let mut buf = Vec::new();
        for m in &msgs {
            write_message(&mut buf, m).unwrap();
        }
        let mut reader = BufReader::new(Cursor::new(buf));
        for expected in &msgs {
            let parsed = read_message(&mut reader).unwrap().unwrap();
            assert_eq!(parsed.id, expected.id);
            assert_eq!(parsed.method, expected.method);
        }
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn test_ipc_loop_heartbeat_and_deactivate() {
        // Build input: heartbeat then deactivate
        let mut input = Vec::new();
        write_message(&mut input, &IpcMessage::signal("hb-1", "heartbeat")).unwrap();
        write_message(&mut input, &IpcMessage::signal("deact-1", "deactivate")).unwrap();

        let mut reader = BufReader::new(Cursor::new(input));
        let mut output = Vec::new();
        run_ipc_loop(&mut reader, &mut output).unwrap();

        // Parse the output — should have exactly one heartbeat-ack
        let mut out_reader = BufReader::new(Cursor::new(output));
        let ack = read_message(&mut out_reader).unwrap().unwrap();
        assert_eq!(ack.msg_type, MessageType::Signal);
        assert_eq!(ack.method, "heartbeat-ack");
        assert_eq!(ack.id, "hb-1");
        // No more messages
        assert!(read_message(&mut out_reader).unwrap().is_none());
    }

    #[test]
    fn test_ipc_loop_unhandled_request() {
        let mut input = Vec::new();
        let req = IpcMessage {
            msg_type: MessageType::Request,
            id: "req-1".into(),
            method: "unknown.method".into(),
            payload: Some(serde_json::json!({"x": 1})),
            error: None,
        };
        write_message(&mut input, &req).unwrap();
        write_message(&mut input, &IpcMessage::signal("d", "deactivate")).unwrap();

        let mut reader = BufReader::new(Cursor::new(input));
        let mut output = Vec::new();
        run_ipc_loop(&mut reader, &mut output).unwrap();

        let mut out_reader = BufReader::new(Cursor::new(output));
        let resp = read_message(&mut out_reader).unwrap().unwrap();
        assert_eq!(resp.msg_type, MessageType::Response);
        assert_eq!(resp.id, "req-1");
        assert_eq!(resp.error.as_ref().unwrap().code, "UNHANDLED");
    }

    #[test]
    fn test_ipc_loop_exits_on_eof() {
        let input = Vec::new();
        let mut reader = BufReader::new(Cursor::new(input));
        let mut output = Vec::new();
        run_ipc_loop(&mut reader, &mut output).unwrap();
        // Should exit cleanly with no output
        assert!(output.is_empty());
    }

    #[test]
    fn test_request_message_serde() {
        let msg = IpcMessage {
            msg_type: MessageType::Request,
            id: "r-1".into(),
            method: "test.method".into(),
            payload: Some(serde_json::json!({"a": [1, 2, 3]})),
            error: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"request\""));
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, MessageType::Request);
        assert_eq!(parsed.method, "test.method");
    }

    #[test]
    fn test_event_message_serde() {
        let msg = IpcMessage {
            msg_type: MessageType::Event,
            id: "ev-1".into(),
            method: "stream:start".into(),
            payload: Some(serde_json::json!({"streamId": "s-1"})),
            error: None,
        };
        let parsed = roundtrip(&msg);
        assert_eq!(parsed.msg_type, MessageType::Event);
        assert_eq!(parsed.method, "stream:start");
    }

    #[test]
    fn test_ipc_loop_activate_signal() {
        let mut input = Vec::new();
        write_message(&mut input, &IpcMessage::signal("act-1", "activate")).unwrap();
        write_message(&mut input, &IpcMessage::signal("d", "deactivate")).unwrap();

        let mut reader = BufReader::new(Cursor::new(input));
        let mut output = Vec::new();
        run_ipc_loop(&mut reader, &mut output).unwrap();

        let mut out_reader = BufReader::new(Cursor::new(output));
        let resp = read_message(&mut out_reader).unwrap().unwrap();
        assert_eq!(resp.msg_type, MessageType::Response);
        assert_eq!(resp.method, "activate");
        assert_eq!(resp.id, "act-1");
        assert!(resp.payload.is_none());
        // No more messages
        assert!(read_message(&mut out_reader).unwrap().is_none());
    }

    #[test]
    fn test_read_rejects_oversized_frame() {
        let mut buf = Vec::new();
        let oversized_len = (MAX_FRAME_SIZE as u32) + 1;
        buf.extend_from_slice(&oversized_len.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; oversized_len as usize]);

        let mut reader = BufReader::new(Cursor::new(buf));
        let result = read_message(&mut reader);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("invalid frame length"));
    }

    #[test]
    fn test_read_rejects_zero_length_frame() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&0u32.to_le_bytes());

        let mut reader = BufReader::new(Cursor::new(buf));
        let result = read_message(&mut reader);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("invalid frame length"));
    }

    #[test]
    fn test_write_rejects_oversized_frame() {
        // Create a message with a payload large enough to exceed MAX_FRAME_SIZE
        let big_string = "x".repeat(MAX_FRAME_SIZE + 1);
        let msg = IpcMessage {
            msg_type: MessageType::Request,
            id: "big".into(),
            method: "test".into(),
            payload: Some(serde_json::json!(big_string)),
            error: None,
        };
        let mut buf = Vec::new();
        let result = write_message(&mut buf, &msg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("frame too large"));
    }
}
