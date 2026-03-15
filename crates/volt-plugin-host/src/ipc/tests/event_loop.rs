use std::io::{BufReader, Cursor};

use crate::ipc::{IpcMessage, run_ipc_loop, write_message};

use super::super::framing::read_message;
use super::super::message::MessageType;

#[test]
fn ipc_loop_heartbeat_and_deactivate() {
    let mut input = Vec::new();
    write_message(&mut input, &IpcMessage::signal("hb-1", "heartbeat")).unwrap();
    write_message(&mut input, &IpcMessage::signal("deact-1", "deactivate")).unwrap();

    let mut reader = BufReader::new(Cursor::new(input));
    let mut output = Vec::new();
    run_ipc_loop(&mut reader, &mut output).unwrap();

    let mut out_reader = BufReader::new(Cursor::new(output));
    let ack = read_message(&mut out_reader).unwrap().unwrap();
    assert_eq!(ack.msg_type, MessageType::Signal);
    assert_eq!(ack.method, "heartbeat-ack");
    assert_eq!(ack.id, "hb-1");
    assert!(read_message(&mut out_reader).unwrap().is_none());
}

#[test]
fn ipc_loop_unhandled_request() {
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
    let response = read_message(&mut out_reader).unwrap().unwrap();
    assert_eq!(response.msg_type, MessageType::Response);
    assert_eq!(response.id, "req-1");
    assert_eq!(response.error.as_ref().unwrap().code, "UNHANDLED");
}

#[test]
fn ipc_loop_exits_on_eof() {
    let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
    let mut output = Vec::new();
    run_ipc_loop(&mut reader, &mut output).unwrap();
    assert!(output.is_empty());
}

#[test]
fn ipc_loop_activate_signal() {
    let mut input = Vec::new();
    write_message(&mut input, &IpcMessage::signal("act-1", "activate")).unwrap();
    write_message(&mut input, &IpcMessage::signal("d", "deactivate")).unwrap();

    let mut reader = BufReader::new(Cursor::new(input));
    let mut output = Vec::new();
    run_ipc_loop(&mut reader, &mut output).unwrap();

    let mut out_reader = BufReader::new(Cursor::new(output));
    let response = read_message(&mut out_reader).unwrap().unwrap();
    assert_eq!(response.msg_type, MessageType::Response);
    assert_eq!(response.method, "activate");
    assert_eq!(response.id, "act-1");
    assert!(response.payload.is_none());
    assert!(read_message(&mut out_reader).unwrap().is_none());
}
