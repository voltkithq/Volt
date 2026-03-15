use std::io::{BufReader, Cursor};

use crate::ipc::{IpcMessage, write_message};

use super::super::framing::read_message;

pub(super) fn roundtrip(msg: &IpcMessage) -> IpcMessage {
    let mut buf = Vec::new();
    write_message(&mut buf, msg).unwrap();
    let mut reader = BufReader::new(Cursor::new(buf));
    read_message(&mut reader).unwrap().unwrap()
}
