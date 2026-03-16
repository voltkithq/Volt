use std::io::{BufReader, Read, Write};

use super::wire::WireMessage;

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

pub(super) fn write_wire_message<W: Write>(
    writer: &mut W,
    message: &WireMessage,
) -> std::io::Result<()> {
    let json = serde_json::to_string(message)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let body = format!("{json}\n");
    let bytes = body.as_bytes();
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {}", bytes.len()),
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()
}

pub(super) fn read_wire_message<R: Read>(
    reader: &mut BufReader<R>,
) -> std::io::Result<Option<WireMessage>> {
    let mut len_buf = [0_u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }

    let length = u32::from_le_bytes(len_buf) as usize;
    if length == 0 || length > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid frame length: {length}"),
        ));
    }

    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    let raw = String::from_utf8(body)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let trimmed = raw.trim_end_matches('\n');
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}
