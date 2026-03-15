use std::io::{self, BufReader, Read, Write};

use super::message::IpcMessage;

pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Write a framed message to the given writer (stdout).
///
/// Format: `<4-byte LE length><JSON bytes>\n`
pub fn write_message<W: Write>(writer: &mut W, msg: &IpcMessage) -> io::Result<()> {
    let json = serde_json::to_string(msg)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
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

    writer.write_all(&(body_bytes.len() as u32).to_le_bytes())?;
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
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
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

    let raw = String::from_utf8(body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let trimmed = raw.trim_end_matches('\n');
    let msg = serde_json::from_str(trimmed)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(msg))
}
