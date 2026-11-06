//! Whether a talk key is physically down.
//!
//! The global hotkey watcher on Windows can report a release as soon as its
//! first poll misses the key. Hold-to-talk asks the key again here, and keeps
//! the recording open while the high bit is set.

use crate::keys::NamedKey;

pub fn is_down(key: NamedKey) -> Option<bool> {
    #[cfg(windows)]
    {
        let vk = vk_code(key)?;
        // Negative means the high bit is set: the key is down right now.
        let state = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) };
        Some(state < 0)
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        None
    }
}

#[cfg(windows)]
fn vk_code(key: NamedKey) -> Option<i32> {
    Some(match key {
        NamedKey::Char(c) if c.is_ascii_alphabetic() => c.to_ascii_uppercase() as i32,
        NamedKey::Char(c) if c.is_ascii_digit() => c as i32,
        NamedKey::Char(',') => 0xBC,
        NamedKey::Char('.') => 0xBE,
        NamedKey::Char('/') => 0xBF,
        NamedKey::Char('-') => 0xBD,
        NamedKey::Char('=') => 0xBB,
        NamedKey::Char('`') => 0xC0,
        NamedKey::Char(';') => 0xBA,
        NamedKey::Char('\'') => 0xDE,
        NamedKey::Char('[') => 0xDB,
        NamedKey::Char(']') => 0xDD,
        NamedKey::Char('\\') => 0xDC,
        NamedKey::Enter | NamedKey::NumpadEnter => 0x0D,
        NamedKey::Escape => 0x1B,
        NamedKey::Tab => 0x09,
        NamedKey::Space => 0x20,
        NamedKey::Backspace => 0x08,
        NamedKey::Delete => 0x2E,
        NamedKey::Insert => 0x2D,
        NamedKey::Left => 0x25,
        NamedKey::Up => 0x26,
        NamedKey::Right => 0x27,
        NamedKey::Down => 0x28,
        NamedKey::Home => 0x24,
        NamedKey::End => 0x23,
        NamedKey::PageUp => 0x21,
        NamedKey::PageDown => 0x22,
        NamedKey::Grave => 0xC0,
        NamedKey::Slash => 0xBF,
        NamedKey::Comma => 0xBC,
        NamedKey::Period => 0xBE,
        NamedKey::Minus => 0xBD,
        NamedKey::Equal => 0xBB,
        NamedKey::F(n) if (1..=24).contains(&n) => 0x70 + i32::from(n) - 1,
        NamedKey::ScrollLock => 0x91,
        NamedKey::Pause => 0x13,
        NamedKey::LCtrl => 0xA2,
        NamedKey::RCtrl => 0xA3,
        NamedKey::LShift => 0xA0,
        NamedKey::RShift => 0xA1,
        NamedKey::LAlt => 0xA4,
        NamedKey::RAlt => 0xA5,
        NamedKey::LWin => 0x5B,
        NamedKey::RWin => 0x5C,
        NamedKey::Numpad(n) if n <= 9 => 0x60 + i32::from(n),
        NamedKey::NumpadAdd => 0x6B,
        NamedKey::NumpadSubtract => 0x6D,
        NamedKey::NumpadMultiply => 0x6A,
        NamedKey::NumpadDecimal => 0x6E,
        NamedKey::NumpadDivide => 0x6F,
        NamedKey::PrintScreen => 0x2C,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_cannot_poll_the_key_so_hold_uses_the_release_event() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(is_down(NamedKey::ScrollLock), None);
    }
}
