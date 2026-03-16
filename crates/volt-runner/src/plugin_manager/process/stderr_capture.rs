use std::io::Read;

const MAX_STDERR_CAPTURE_BYTES: usize = 256 * 1024;

pub(super) fn read_bounded_stderr(stderr: &mut impl Read) -> String {
    let mut captured = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 8192];
    let mut truncated = false;

    loop {
        match stderr.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                let remaining = MAX_STDERR_CAPTURE_BYTES.saturating_sub(captured.len());
                if remaining == 0 {
                    truncated = true;
                    break;
                }
                let to_copy = read.min(remaining);
                captured.extend_from_slice(&chunk[..to_copy]);
                if to_copy < read {
                    truncated = true;
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let mut text = String::from_utf8_lossy(&captured).into_owned();
    if truncated {
        text.push_str("\n[volt] plugin stderr truncated at 262144 bytes");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_stderr_reader_caps_capture_size() {
        let oversized = vec![b'x'; MAX_STDERR_CAPTURE_BYTES + 1024];
        let mut reader = std::io::Cursor::new(oversized);
        let captured = read_bounded_stderr(&mut reader);

        assert!(captured.len() >= MAX_STDERR_CAPTURE_BYTES);
        assert!(captured.contains("plugin stderr truncated"));
    }
}
