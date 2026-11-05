//! One global key. It is released while a text field is focused so that
//! field can still be typed into.

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::keys::{self, Chord};

pub struct HotkeyHub {
    manager: Option<GlobalHotKeyManager>,
    registered: Option<HotKey>,
    id: Option<u32>,
    error: Option<String>,
    suspended: bool,
    binding: Option<Chord>,
}

impl HotkeyHub {
    pub fn new() -> Self {
        match GlobalHotKeyManager::new() {
            Ok(manager) => Self {
                manager: Some(manager),
                registered: None,
                id: None,
                error: None,
                suspended: false,
                binding: None,
            },
            Err(err) => Self {
                manager: None,
                registered: None,
                id: None,
                error: Some(format!("Global keys aren't available. {err}")),
                suspended: false,
                binding: None,
            },
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_binding(&mut self, binding: Result<Chord, String>) {
        if self.manager.is_none() {
            self.binding = binding.ok();
            return;
        }
        match binding {
            Ok(chord) => {
                self.binding = Some(chord);
                self.error = None;
            }
            Err(err) => {
                self.binding = None;
                self.error = Some(err);
            }
        }
        if self.suspended {
            self.unregister();
        } else {
            self.register_current();
        }
    }

    pub fn set_suspended(&mut self, suspended: bool) {
        if suspended == self.suspended {
            return;
        }
        self.suspended = suspended;
        if suspended {
            self.unregister();
        } else {
            self.register_current();
        }
    }

    pub fn poll_pressed(&self) -> bool {
        let mut pressed = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if self.suspended {
                continue;
            }
            if event.state == HotKeyState::Pressed && self.id == Some(event.id) {
                pressed = true;
            }
        }
        pressed
    }

    fn register_current(&mut self) {
        self.unregister();
        let (Some(manager), Some(chord)) = (self.manager.as_ref(), self.binding.clone()) else {
            return;
        };
        match keys::chord_to_hotkey(&chord) {
            Ok((mods, code)) => {
                let hotkey = HotKey::new(mods, code);
                let id = hotkey.id();
                match manager.register(hotkey) {
                    Ok(()) => {
                        self.registered = Some(hotkey);
                        self.id = Some(id);
                        self.error = None;
                    }
                    Err(err) => {
                        self.error = Some(format!(
                            "Couldn't claim that key ({err}). Pick another, or use the on-screen button."
                        ));
                    }
                }
            }
            Err(err) => self.error = Some(err),
        }
    }

    fn unregister(&mut self) {
        if let (Some(manager), Some(hotkey)) = (self.manager.as_ref(), self.registered.take()) {
            let _ = manager.unregister(hotkey);
        }
        self.id = None;
    }
}
