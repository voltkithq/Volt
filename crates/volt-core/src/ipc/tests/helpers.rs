pub(super) fn decode_js_single_quoted(escaped: &str) -> String {
    let mut out = String::with_capacity(escaped.len());
    let chars: Vec<char> = escaped.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if ch != '\\' {
            out.push(ch);
            index += 1;
            continue;
        }

        index += 1;
        if index >= chars.len() {
            out.push('\\');
            break;
        }

        match chars[index] {
            '\\' => out.push('\\'),
            '\'' => out.push('\''),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            'u' => {
                if index + 4 < chars.len() {
                    let code: String = chars[(index + 1)..=(index + 4)].iter().collect();
                    if let Ok(value) = u32::from_str_radix(&code, 16)
                        && let Some(decoded) = char::from_u32(value)
                    {
                        out.push(decoded);
                        index += 4;
                    } else {
                        out.push('u');
                    }
                } else {
                    out.push('u');
                }
            }
            other => out.push(other),
        }

        index += 1;
    }

    out
}

pub(super) fn extract_response_payload(script: &str) -> Option<String> {
    let prefix = "window.__volt_response__('";
    let suffix = "')";

    script
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .map(decode_js_single_quoted)
}

pub(super) fn extract_event_payload(script: &str) -> Option<(String, serde_json::Value)> {
    let prefix = "window.__volt_event__('";
    let rest = script.strip_prefix(prefix)?;
    let separator = "', JSON.parse('";
    let (escaped_name, rest) = rest.split_once(separator)?;
    let escaped_data = rest.strip_suffix("'))")?;

    let name = decode_js_single_quoted(escaped_name);
    let data_json = decode_js_single_quoted(escaped_data);
    let data = serde_json::from_str(&data_json).ok()?;
    Some((name, data))
}
