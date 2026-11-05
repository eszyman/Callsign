//! Bring the sim forward before typing, and refuse to type into the wrong window.

pub fn title_matches(title: &str, needle: &str) -> bool {
    let needle = needle.trim();
    !needle.is_empty() && title.to_lowercase().contains(&needle.to_lowercase())
}

pub fn focus_title(needle: &str) -> Result<String, String> {
    platform::focus_title(needle)
}

pub fn foreground_title() -> Result<String, String> {
    platform::foreground_title()
}

#[cfg(target_os = "linux")]
mod platform {
    use std::collections::HashSet;

    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        Atom, AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt, EventMask,
        StackMode, Window,
    };
    use x11rb::CURRENT_TIME;

    pub fn foreground_title() -> Result<String, String> {
        let (conn, root, names) = connect()?;
        let window = active_window(&conn, root, names.active)?;
        window_title(&conn, window, names.wm_name, names.utf8)
            .ok_or_else(|| "The focused window has no title.".into())
    }

    pub fn focus_title(needle: &str) -> Result<String, String> {
        let (conn, root, names) = connect()?;
        let mut found = Vec::new();
        let mut seen = HashSet::new();
        walk(&conn, root, needle, &names, 0, &mut seen, &mut found);
        let target = found
            .into_iter()
            .next()
            .ok_or_else(|| format!("No window title contains \"{needle}\"."))?;
        let aux = ConfigureWindowAux::new().stack_mode(StackMode::ABOVE);
        let _ = conn.configure_window(target, &aux);
        let _ = conn.map_window(target);
        let event = ClientMessageEvent::new(32, target, names.active, [1, CURRENT_TIME, 0, 0, 0]);
        conn.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .map_err(|err| err.to_string())?;
        conn.flush().map_err(|err| err.to_string())?;
        window_title(&conn, target, names.wm_name, names.utf8)
            .ok_or_else(|| format!("Raised a window matching \"{needle}\", but it has no title."))
    }

    struct Names {
        active: Atom,
        wm_name: Atom,
        utf8: Atom,
    }

    fn connect() -> Result<(impl Connection, Window, Names), String> {
        let (conn, screen_num) = x11rb::connect(None).map_err(|err| err.to_string())?;
        let root = conn.setup().roots[screen_num].root;
        let names = Names {
            active: intern(&conn, b"_NET_ACTIVE_WINDOW")?,
            wm_name: intern(&conn, b"_NET_WM_NAME")?,
            utf8: intern(&conn, b"UTF8_STRING")?,
        };
        Ok((conn, root, names))
    }

    fn intern(conn: &impl Connection, name: &[u8]) -> Result<Atom, String> {
        Ok(conn
            .intern_atom(false, name)
            .map_err(|err| err.to_string())?
            .reply()
            .map_err(|err| err.to_string())?
            .atom)
    }

    fn active_window(conn: &impl Connection, root: Window, atom: Atom) -> Result<Window, String> {
        let reply = conn
            .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
            .map_err(|err| err.to_string())?
            .reply()
            .map_err(|err| err.to_string())?;
        reply
            .value32()
            .and_then(|mut values| values.next())
            .filter(|window| *window != 0)
            .ok_or_else(|| "No focused window.".into())
    }

    fn window_title(
        conn: &impl Connection,
        window: Window,
        net_wm_name: Atom,
        utf8: Atom,
    ) -> Option<String> {
        if let Some(title) = property_string(conn, window, net_wm_name, utf8) {
            if !title.is_empty() {
                return Some(title);
            }
        }
        property_string(
            conn,
            window,
            AtomEnum::WM_NAME.into(),
            AtomEnum::STRING.into(),
        )
    }

    fn property_string(
        conn: &impl Connection,
        window: Window,
        property: Atom,
        type_: Atom,
    ) -> Option<String> {
        let reply = conn
            .get_property(false, window, property, type_, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        let text = String::from_utf8(reply.value).ok()?;
        let text = text.trim_matches('\u{0}').trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn walk(
        conn: &impl Connection,
        window: Window,
        needle: &str,
        names: &Names,
        depth: usize,
        seen: &mut HashSet<Window>,
        found: &mut Vec<Window>,
    ) {
        if depth > 24 || !seen.insert(window) {
            return;
        }
        if let Some(title) = window_title(conn, window, names.wm_name, names.utf8) {
            if super::title_matches(&title, needle) {
                found.push(window);
            }
        }
        let Ok(cookie) = conn.query_tree(window) else {
            return;
        };
        let Ok(tree) = cookie.reply() else {
            return;
        };
        for child in tree.children {
            walk(conn, child, needle, names, depth + 1, seen, found);
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    use super::title_matches;

    struct Search {
        needle: String,
        found: HWND,
    }

    pub fn foreground_title() -> Result<String, String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            window_text(hwnd).ok_or_else(|| "The focused window has no title.".into())
        }
    }

    pub fn focus_title(needle: &str) -> Result<String, String> {
        let mut search = Search {
            needle: needle.to_string(),
            found: HWND(std::ptr::null_mut()),
        };
        unsafe {
            EnumWindows(
                Some(enum_windows),
                LPARAM(&mut search as *mut Search as isize),
            )
            .map_err(|err| err.to_string())?;
            if search.found.0.is_null() {
                return Err(format!("No window title contains \"{needle}\"."));
            }
            raise(search.found);
            window_text(search.found).ok_or_else(|| {
                format!("Raised a window matching \"{needle}\", but it has no title.")
            })
        }
    }

    unsafe extern "system" fn enum_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        if IsWindowVisible(hwnd).0 != 0 {
            if let Some(title) = window_text(hwnd) {
                if title_matches(&title, &search.needle) {
                    search.found = hwnd;
                    return BOOL(0);
                }
            }
        }
        BOOL(1)
    }

    unsafe fn window_text(hwnd: HWND) -> Option<String> {
        if hwnd.0.is_null() {
            return None;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return None;
        }
        let text = String::from_utf16_lossy(&buf[..copied as usize]);
        let text = text.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    unsafe fn raise(hwnd: HWND) {
        let foreground = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground, None);
        let current = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current
            && AttachThreadInput(current, foreground_thread, true).0 != 0;
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(current, foreground_thread, false);
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    pub fn focus_title(needle: &str) -> Result<String, String> {
        let _ = needle;
        Err("Focusing the sim isn't implemented on this system. Focus IL-2 yourself.".into())
    }

    pub fn foreground_title() -> Result<String, String> {
        Err("Can't read the focused window on this system.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::title_matches;

    #[test]
    fn matches_case_insensitive_substring() {
        assert!(title_matches("IL-2 Sturmovik: Korea", "il-2"));
        assert!(!title_matches("Notepad", "IL-2"));
        assert!(!title_matches("IL-2", "  "));
    }
}
