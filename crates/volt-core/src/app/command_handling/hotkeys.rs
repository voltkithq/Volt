use global_hotkey::hotkey::{Code, HotKey, Modifiers};

pub(super) fn parse_hotkey_accelerator(accelerator: &str) -> Result<HotKey, String> {
    let mut modifiers = Modifiers::empty();
    let mut key: Option<Code> = None;

    for part in accelerator.split('+') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }

        match token.to_ascii_lowercase().as_str() {
            "cmdorctrl" => {
                if cfg!(target_os = "macos") {
                    modifiers |= Modifiers::META;
                } else {
                    modifiers |= Modifiers::CONTROL;
                }
            }
            "cmd" | "command" | "meta" | "super" => modifiers |= Modifiers::META,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => {
                if key.is_some() {
                    return Err(format!("Multiple keys in accelerator '{accelerator}'"));
                }
                key = Some(parse_key_code(token)?);
            }
        }
    }

    let code = key.ok_or_else(|| format!("Missing key in accelerator '{accelerator}'"))?;
    Ok(HotKey::new(Some(modifiers), code))
}

fn parse_key_code(token: &str) -> Result<Code, String> {
    let upper = token.trim().to_ascii_uppercase();
    let key = match upper.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "ESC" | "ESCAPE" => Code::Escape,
        "TAB" => Code::Tab,
        "UP" => Code::ArrowUp,
        "DOWN" => Code::ArrowDown,
        "LEFT" => Code::ArrowLeft,
        "RIGHT" => Code::ArrowRight,
        _ => {
            if let Some(stripped) = upper.strip_prefix('F')
                && let Ok(value) = stripped.parse::<u8>()
            {
                return match value {
                    1 => Ok(Code::F1),
                    2 => Ok(Code::F2),
                    3 => Ok(Code::F3),
                    4 => Ok(Code::F4),
                    5 => Ok(Code::F5),
                    6 => Ok(Code::F6),
                    7 => Ok(Code::F7),
                    8 => Ok(Code::F8),
                    9 => Ok(Code::F9),
                    10 => Ok(Code::F10),
                    11 => Ok(Code::F11),
                    12 => Ok(Code::F12),
                    13 => Ok(Code::F13),
                    14 => Ok(Code::F14),
                    15 => Ok(Code::F15),
                    16 => Ok(Code::F16),
                    17 => Ok(Code::F17),
                    18 => Ok(Code::F18),
                    19 => Ok(Code::F19),
                    20 => Ok(Code::F20),
                    21 => Ok(Code::F21),
                    22 => Ok(Code::F22),
                    23 => Ok(Code::F23),
                    24 => Ok(Code::F24),
                    _ => Err(format!("Unsupported function key: F{value}")),
                };
            }
            return Err(format!("Unsupported key token: {token}"));
        }
    };
    Ok(key)
}
