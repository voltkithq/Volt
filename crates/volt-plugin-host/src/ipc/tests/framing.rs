use std::io::{self, BufReader, Cursor};

use crate::ipc::{IpcMessage, write_message};

use super::super::framing::{MAX_FRAME_SIZE, read_message};
use super::super::message::MessageType;

#[test]
fn read_eof() {
    let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
    assert!(read_message(&mut reader).unwrap().is_none());
}

#[test]
fn multiple_messages_in_sequence() {
    let msgs = vec![
        IpcMessage::signal("1", "heartbeat"),
        IpcMessage::signal("2", "heartbeat-ack"),
        IpcMessage::signal("3", "ready"),
    ];
    let mut buf = Vec::new();
    for msg in &msgs {
        write_message(&mut buf, msg).unwrap();
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
fn read_rejects_oversized_frame() {
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
fn read_rejects_zero_length_frame() {
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
fn write_rejects_oversized_frame() {
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
    assert!(result.unwrap_err().to_string().contains("frame too large"));
}
