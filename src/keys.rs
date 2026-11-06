//! Key sequences written the way a pilot would say the binding.
//!
//! `rctrl+enter` is one chord. `grave, delay:120, 1` is a radio menu:
//! open it, wait, then press 1. Names are case-insensitive.

use std::str::FromStr;

use global_hotkey::hotkey::{Code, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Char(char),
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Grave,
    Slash,
    Comma,
    Period,
    Minus,
    Equal,
    F(u8),
    ScrollLock,
    Pause,
    LCtrl,
    RCtrl,
    LShift,
    RShift,
    LAlt,
    RAlt,
    LWin,
    RWin,
    Numpad(u8),
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDecimal,
    NumpadDivide,
    NumpadEnter,
    PrintScreen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Chord {
        modifiers: Vec<NamedKey>,
        key: NamedKey,
    },
    Delay {
        ms: u64,
    },
    Text {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: Vec<NamedKey>,
    pub key: NamedKey,
}

pub fn is_modifier(key: NamedKey) -> bool {
    matches!(
        key,
        NamedKey::LCtrl
            | NamedKey::RCtrl
            | NamedKey::LShift
            | NamedKey::RShift
            | NamedKey::LAlt
            | NamedKey::RAlt
            | NamedKey::LWin
            | NamedKey::RWin
    )
}

pub fn parse_sequence(input: &str) -> Result<Vec<Step>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut steps = Vec::new();
    for raw in trimmed.split(',') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(rest) = strip_prefix_ignore_case(part, "delay:") {
            let ms: u64 = rest
                .trim()
                .parse()
                .map_err(|_| format!("\"{part}\" needs a number of milliseconds."))?;
            steps.push(Step::Delay { ms });
            continue;
        }
        let (modifiers, key) = split_chord(part)?;
        steps.push(Step::Chord { modifiers, key });
    }
    Ok(steps)
}

pub fn parse_ptt(input: &str) -> Result<Chord, String> {
    let steps = parse_sequence(input)?;
    match steps.as_slice() {
        [Step::Chord { modifiers, key }] => Ok(Chord {
            modifiers: modifiers.clone(),
            key: *key,
        }),
        _ => Err("Push-to-talk is one key, like scrolllock or ctrl+f13.".into()),
    }
}

/// Modifiers are peeled from the left, so a key such as `numpad+` keeps its plus.
fn split_chord(part: &str) -> Result<(Vec<NamedKey>, NamedKey), String> {
    let mut rest = part.trim();
    if rest.is_empty() {
        return Err(format!("\"{part}\" is not a key."));
    }
    let mut modifiers = Vec::new();
    while let Some((modifier, after)) = take_modifier(rest) {
        modifiers.push(modifier);
        rest = after.trim();
        if rest.is_empty() {
            return Err(format!("\"{part}\" is missing the key after the modifier."));
        }
    }
    let key = parse_key(rest)?;
    if is_modifier(key) && !modifiers.is_empty() {
        return Err("The last part of a chord has to be the key that gets pressed.".into());
    }
    Ok((modifiers, key))
}

fn take_modifier(input: &str) -> Option<(NamedKey, &str)> {
    const MODIFIERS: &[(&str, NamedKey)] = &[
        ("leftcontrol", NamedKey::LCtrl),
        ("rightcontrol", NamedKey::RCtrl),
        ("leftctrl", NamedKey::LCtrl),
        ("rightctrl", NamedKey::RCtrl),
        ("leftshift", NamedKey::LShift),
        ("rightshift", NamedKey::RShift),
        ("leftalt", NamedKey::LAlt),
        ("rightalt", NamedKey::RAlt),
        ("leftwin", NamedKey::LWin),
        ("rightwin", NamedKey::RWin),
        ("lcontrol", NamedKey::LCtrl),
        ("rcontrol", NamedKey::RCtrl),
        ("lshift", NamedKey::LShift),
        ("rshift", NamedKey::RShift),
        ("lsuper", NamedKey::LWin),
        ("rsuper", NamedKey::RWin),
        ("lctrl", NamedKey::LCtrl),
        ("rctrl", NamedKey::RCtrl),
        ("lmeta", NamedKey::LWin),
        ("rmeta", NamedKey::RWin),
        ("lalt", NamedKey::LAlt),
        ("ralt", NamedKey::RAlt),
        ("altgr", NamedKey::RAlt),
        ("control", NamedKey::LCtrl),
        ("shift", NamedKey::LShift),
        ("super", NamedKey::LWin),
        ("ctrl", NamedKey::LCtrl),
        ("meta", NamedKey::LWin),
        ("lwin", NamedKey::LWin),
        ("rwin", NamedKey::RWin),
        ("alt", NamedKey::LAlt),
        ("win", NamedKey::LWin),
    ];
    let trimmed = input.trim_start();
    for (name, key) in MODIFIERS {
        if trimmed.len() >= name.len() && trimmed[..name.len()].eq_ignore_ascii_case(name) {
            let after = trimmed[name.len()..].trim_start();
            if let Some(after) = after.strip_prefix('+') {
                return Some((*key, after));
            }
        }
    }
    None
}

pub fn parse_key(token: &str) -> Result<NamedKey, String> {
    let t = token.trim().to_lowercase();
    let key = match t.as_str() {
        "enter" | "return" => NamedKey::Enter,
        "esc" | "escape" => NamedKey::Escape,
        "tab" => NamedKey::Tab,
        "space" => NamedKey::Space,
        "backspace" | "bksp" => NamedKey::Backspace,
        "delete" | "del" => NamedKey::Delete,
        "insert" | "ins" => NamedKey::Insert,
        "left" => NamedKey::Left,
        "right" => NamedKey::Right,
        "up" => NamedKey::Up,
        "down" => NamedKey::Down,
        "home" => NamedKey::Home,
        "end" => NamedKey::End,
        "pageup" | "pgup" => NamedKey::PageUp,
        "pagedown" | "pgdn" => NamedKey::PageDown,
        "grave" | "tilde" | "backquote" => NamedKey::Grave,
        "slash" | "/" => NamedKey::Slash,
        "comma" | "," => NamedKey::Comma,
        "period" | "dot" | "." => NamedKey::Period,
        "minus" | "dash" | "-" => NamedKey::Minus,
        "equal" | "equals" | "=" => NamedKey::Equal,
        "numpad+" | "numpadplus" | "kp+" => NamedKey::NumpadAdd,
        "numpad-" | "numpadminus" | "kp-" => NamedKey::NumpadSubtract,
        "numpad*" | "numpadstar" | "kp*" => NamedKey::NumpadMultiply,
        "numpad." | "numpaddot" | "kp." => NamedKey::NumpadDecimal,
        "numpad/" | "numpadslash" | "kp/" => NamedKey::NumpadDivide,
        "numpad_enter" | "numpadenter" | "kpenter" => NamedKey::NumpadEnter,
        "printscreen" | "prtsc" | "prtscn" | "snapshot" => NamedKey::PrintScreen,
        "scrolllock" | "scroll" => NamedKey::ScrollLock,
        "pause" | "break" => NamedKey::Pause,
        "lctrl" | "lcontrol" | "leftctrl" => NamedKey::LCtrl,
        "rctrl" | "rcontrol" | "rightctrl" => NamedKey::RCtrl,
        "ctrl" | "control" => NamedKey::LCtrl,
        "lshift" | "leftshift" => NamedKey::LShift,
        "rshift" | "rightshift" => NamedKey::RShift,
        "shift" => NamedKey::LShift,
        "lalt" | "leftalt" => NamedKey::LAlt,
        "ralt" | "rightalt" | "altgr" => NamedKey::RAlt,
        "alt" => NamedKey::LAlt,
        "lwin" | "leftwin" | "lsuper" | "lmeta" => NamedKey::LWin,
        "rwin" | "rightwin" | "rsuper" | "rmeta" => NamedKey::RWin,
        "win" | "super" | "meta" => NamedKey::LWin,
        _ => {
            if let Some(n) = t.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                if (1..=24).contains(&n) {
                    return Ok(NamedKey::F(n));
                }
            }
            if let Some(rest) = t.strip_prefix("numpad").or_else(|| t.strip_prefix("kp")) {
                if let Ok(n) = rest.parse::<u8>() {
                    if n <= 9 {
                        return Ok(NamedKey::Numpad(n));
                    }
                }
            }
            let mut chars = t.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() {
                    return Ok(NamedKey::Char(c));
                }
            }
            return Err(format!("Unknown key \"{token}\"."));
        }
    };
    Ok(key)
}

pub fn describe_key(key: NamedKey) -> String {
    match key {
        NamedKey::Char(c) => c.to_ascii_uppercase().to_string(),
        NamedKey::Enter => "Enter".into(),
        NamedKey::Escape => "Escape".into(),
        NamedKey::Tab => "Tab".into(),
        NamedKey::Space => "Space".into(),
        NamedKey::Backspace => "Backspace".into(),
        NamedKey::Delete => "Delete".into(),
        NamedKey::Insert => "Insert".into(),
        NamedKey::Left => "Left".into(),
        NamedKey::Right => "Right".into(),
        NamedKey::Up => "Up".into(),
        NamedKey::Down => "Down".into(),
        NamedKey::Home => "Home".into(),
        NamedKey::End => "End".into(),
        NamedKey::PageUp => "Page Up".into(),
        NamedKey::PageDown => "Page Down".into(),
        NamedKey::Grave => "Tilde".into(),
        NamedKey::Slash => "Slash".into(),
        NamedKey::Comma => "Comma".into(),
        NamedKey::Period => "Period".into(),
        NamedKey::Minus => "Minus".into(),
        NamedKey::Equal => "Equals".into(),
        NamedKey::F(n) => format!("F{n}"),
        NamedKey::ScrollLock => "Scroll Lock".into(),
        NamedKey::Pause => "Pause".into(),
        NamedKey::LCtrl => "Left Ctrl".into(),
        NamedKey::RCtrl => "Right Ctrl".into(),
        NamedKey::LShift => "Left Shift".into(),
        NamedKey::RShift => "Right Shift".into(),
        NamedKey::LAlt => "Left Alt".into(),
        NamedKey::RAlt => "Right Alt".into(),
        NamedKey::LWin => "Left Win".into(),
        NamedKey::RWin => "Right Win".into(),
        NamedKey::Numpad(n) => format!("Numpad {n}"),
        NamedKey::NumpadAdd => "Numpad +".into(),
        NamedKey::NumpadSubtract => "Numpad -".into(),
        NamedKey::NumpadMultiply => "Numpad *".into(),
        NamedKey::NumpadDecimal => "Numpad .".into(),
        NamedKey::NumpadDivide => "Numpad /".into(),
        NamedKey::NumpadEnter => "Numpad Enter".into(),
        NamedKey::PrintScreen => "Print Screen".into(),
    }
}

pub fn describe_chord(modifiers: &[NamedKey], key: NamedKey) -> String {
    if modifiers.is_empty() {
        return describe_key(key);
    }
    let mut parts: Vec<String> = modifiers.iter().copied().map(describe_key).collect();
    parts.push(describe_key(key));
    parts.join(" + ")
}

pub fn describe_sequence(steps: &[Step]) -> String {
    if steps.is_empty() {
        return "nothing".into();
    }
    steps
        .iter()
        .map(|step| match step {
            Step::Chord { modifiers, key } => describe_chord(modifiers, *key),
            Step::Delay { ms } => format!("wait {ms} ms"),
            Step::Text { text } => format!("type \"{text}\""),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn keyboard_code_name(key: NamedKey) -> String {
    match key {
        NamedKey::Char(c) if c.is_ascii_alphabetic() => {
            format!("Key{}", c.to_ascii_uppercase())
        }
        NamedKey::Char(c) if c.is_ascii_digit() => format!("Digit{c}"),
        NamedKey::Char(_) => "Unidentified".into(),
        NamedKey::Enter => "Enter".into(),
        NamedKey::Escape => "Escape".into(),
        NamedKey::Tab => "Tab".into(),
        NamedKey::Space => "Space".into(),
        NamedKey::Backspace => "Backspace".into(),
        NamedKey::Delete => "Delete".into(),
        NamedKey::Insert => "Insert".into(),
        NamedKey::Left => "ArrowLeft".into(),
        NamedKey::Right => "ArrowRight".into(),
        NamedKey::Up => "ArrowUp".into(),
        NamedKey::Down => "ArrowDown".into(),
        NamedKey::Home => "Home".into(),
        NamedKey::End => "End".into(),
        NamedKey::PageUp => "PageUp".into(),
        NamedKey::PageDown => "PageDown".into(),
        NamedKey::Grave => "Backquote".into(),
        NamedKey::Slash => "Slash".into(),
        NamedKey::Comma => "Comma".into(),
        NamedKey::Period => "Period".into(),
        NamedKey::Minus => "Minus".into(),
        NamedKey::Equal => "Equal".into(),
        NamedKey::F(n) => format!("F{n}"),
        NamedKey::ScrollLock => "ScrollLock".into(),
        NamedKey::Pause => "Pause".into(),
        NamedKey::LCtrl => "ControlLeft".into(),
        NamedKey::RCtrl => "ControlRight".into(),
        NamedKey::LShift => "ShiftLeft".into(),
        NamedKey::RShift => "ShiftRight".into(),
        NamedKey::LAlt => "AltLeft".into(),
        NamedKey::RAlt => "AltRight".into(),
        NamedKey::LWin => "MetaLeft".into(),
        NamedKey::RWin => "MetaRight".into(),
        NamedKey::Numpad(n) => format!("Numpad{n}"),
        NamedKey::NumpadAdd => "NumpadAdd".into(),
        NamedKey::NumpadSubtract => "NumpadSubtract".into(),
        NamedKey::NumpadMultiply => "NumpadMultiply".into(),
        NamedKey::NumpadDecimal => "NumpadDecimal".into(),
        NamedKey::NumpadDivide => "NumpadDivide".into(),
        NamedKey::NumpadEnter => "NumpadEnter".into(),
        NamedKey::PrintScreen => "PrintScreen".into(),
    }
}

pub fn chord_to_hotkey(chord: &Chord) -> Result<(Option<Modifiers>, Code), String> {
    let mut mods = Modifiers::empty();
    for modifier in &chord.modifiers {
        mods |= modifier_bit(*modifier)?;
    }
    let name = keyboard_code_name(chord.key);
    let code = Code::from_str(&name).map_err(|_| format!("Can't register the {name} key."))?;
    let mods = if mods.is_empty() { None } else { Some(mods) };
    Ok((mods, code))
}

fn modifier_bit(key: NamedKey) -> Result<Modifiers, String> {
    Ok(match key {
        NamedKey::LCtrl | NamedKey::RCtrl => Modifiers::CONTROL,
        NamedKey::LShift | NamedKey::RShift => Modifiers::SHIFT,
        NamedKey::LAlt | NamedKey::RAlt => Modifiers::ALT,
        NamedKey::LWin | NamedKey::RWin => Modifiers::META,
        _ => return Err(format!("{} is not a hotkey modifier.", describe_key(key))),
    })
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_team_chat_and_a_radio_menu() {
        let steps = parse_sequence("rctrl+enter, delay:120, 1").unwrap();
        assert_eq!(
            steps,
            vec![
                Step::Chord {
                    modifiers: vec![NamedKey::RCtrl],
                    key: NamedKey::Enter,
                },
                Step::Delay { ms: 120 },
                Step::Chord {
                    modifiers: vec![],
                    key: NamedKey::Char('1'),
                },
            ]
        );
        assert_eq!(
            describe_sequence(&steps),
            "Right Ctrl + Enter, wait 120 ms, 1"
        );
    }

    #[test]
    fn push_to_talk_rejects_a_pause() {
        assert!(parse_ptt("scrolllock").is_ok());
        assert!(parse_ptt("ctrl+f13").is_ok());
        assert!(parse_ptt("delay:10").is_err());
        assert!(parse_ptt("enter, f1").is_err());
    }

    #[test]
    fn unknown_key_is_an_error() {
        assert!(parse_sequence("notakey").is_err());
        assert!(parse_sequence("mouse_left").is_err());
        assert!(parse_sequence("mousex").is_err());
    }

    #[test]
    fn cockpit_punctuation_and_numpad_math_are_keys() {
        for sequence in [
            "@",
            "\\",
            "'",
            ";",
            "[",
            "]",
            "numpad+",
            "rshift+numpad+",
            "rshift+numpad-",
            "numpad*",
            "numpad.",
            "numpad_enter",
            "printscreen",
            "rctrl+'",
            "tab,0,8",
        ] {
            parse_sequence(sequence).unwrap_or_else(|err| panic!("{sequence}: {err}"));
        }
    }

    #[test]
    fn hotkey_names_match_the_keyboard_codes() {
        let samples = [
            NamedKey::Enter,
            NamedKey::ScrollLock,
            NamedKey::F(13),
            NamedKey::Char('g'),
            NamedKey::Char('1'),
            NamedKey::Grave,
            NamedKey::RCtrl,
            NamedKey::LShift,
            NamedKey::Slash,
            NamedKey::Numpad(0),
        ];
        for key in samples {
            let name = keyboard_code_name(key);
            assert!(
                Code::from_str(&name).is_ok(),
                "keyboard code {name} did not parse"
            );
        }
    }
}
