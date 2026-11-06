//! Talk and command keys. Both are released while a text field is focused
//! so that field can still be typed into.

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::keys::{self, Chord};

pub struct KeyEdge {
    pub command: bool,
    pub down: bool,
}

struct Slot {
    binding: Option<Chord>,
    hotkey: Option<HotKey>,
    id: Option<u32>,
    error: Option<String>,
}

impl Slot {
    fn empty() -> Self {
        Self {
            binding: None,
            hotkey: None,
            id: None,
            error: None,
        }
    }
}

pub struct HotkeyHub {
    manager: Option<GlobalHotKeyManager>,
    talk: Slot,
    command: Slot,
    suspended: bool,
    manager_error: Option<String>,
}

impl HotkeyHub {
    pub fn new() -> Self {
        match GlobalHotKeyManager::new() {
            Ok(manager) => Self {
                manager: Some(manager),
                talk: Slot::empty(),
                command: Slot::empty(),
                suspended: false,
                manager_error: None,
            },
            Err(err) => Self {
                manager: None,
                talk: Slot::empty(),
                command: Slot::empty(),
                suspended: false,
                manager_error: Some(format!("Global keys aren't available. {err}")),
            },
        }
    }

    pub fn talk_error(&self) -> Option<&str> {
        self.manager_error.as_deref().or(self.talk.error.as_deref())
    }

    pub fn command_error(&self) -> Option<&str> {
        self.command.error.as_deref()
    }

    pub fn set_talk(&mut self, binding: Result<Chord, String>) {
        self.set_slot(false, binding);
    }

    pub fn set_command(&mut self, binding: Option<Result<Chord, String>>) {
        match binding {
            None => {
                self.command.binding = None;
                self.command.error = None;
                self.sync();
            }
            Some(result) => self.set_slot(true, result),
        }
    }

    pub fn set_suspended(&mut self, suspended: bool) {
        if suspended == self.suspended {
            return;
        }
        self.suspended = suspended;
        self.sync();
    }

    pub fn poll(&self) -> Vec<KeyEdge> {
        let mut edges = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if self.suspended {
                continue;
            }
            let command = if self.command.id == Some(event.id) {
                true
            } else if self.talk.id == Some(event.id) {
                false
            } else {
                continue;
            };
            let down = event.state == HotKeyState::Pressed;
            if !down && event.state != HotKeyState::Released {
                continue;
            }
            edges.push(KeyEdge { command, down });
        }
        edges
    }

    fn set_slot(&mut self, command: bool, binding: Result<Chord, String>) {
        let slot = if command {
            &mut self.command
        } else {
            &mut self.talk
        };
        if self.manager.is_none() {
            slot.binding = binding.ok();
            return;
        }
        match binding {
            Ok(chord) => {
                slot.binding = Some(chord);
                slot.error = None;
            }
            Err(err) => {
                slot.binding = None;
                slot.error = Some(err);
            }
        }
        self.sync();
    }

    fn sync(&mut self) {
        let talk_key = Self::take_registered(&mut self.talk);
        let command_key = Self::take_registered(&mut self.command);
        if let Some(manager) = self.manager.as_ref() {
            if let Some(hotkey) = talk_key {
                let _ = manager.unregister(hotkey);
            }
            if let Some(hotkey) = command_key {
                let _ = manager.unregister(hotkey);
            }
        }
        if self.suspended || self.manager.is_none() {
            return;
        }
        let duplicate = match (&self.talk.binding, &self.command.binding) {
            (Some(talk), Some(command)) => talk == command,
            _ => false,
        };
        if duplicate {
            self.command.error = Some("Use a different key from the talk button.".into());
        } else if self.command.error.as_deref() == Some("Use a different key from the talk button.")
        {
            self.command.error = None;
        }
        self.register(false);
        if !duplicate {
            self.register(true);
        }
    }

    fn register(&mut self, command: bool) {
        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        let binding = if command {
            self.command.binding.clone()
        } else {
            self.talk.binding.clone()
        };
        let Some(chord) = binding else {
            return;
        };
        match keys::chord_to_hotkey(&chord) {
            Ok((mods, code)) => {
                let hotkey = HotKey::new(mods, code);
                let id = hotkey.id();
                match manager.register(hotkey) {
                    Ok(()) => {
                        let slot = if command {
                            &mut self.command
                        } else {
                            &mut self.talk
                        };
                        slot.hotkey = Some(hotkey);
                        slot.id = Some(id);
                        slot.error = None;
                    }
                    Err(err) => {
                        let slot = if command {
                            &mut self.command
                        } else {
                            &mut self.talk
                        };
                        slot.error = Some(format!(
                            "Couldn't claim that key ({err}). Pick another, or use the on-screen button."
                        ));
                    }
                }
            }
            Err(err) => {
                let slot = if command {
                    &mut self.command
                } else {
                    &mut self.talk
                };
                slot.error = Some(err);
            }
        }
    }

    fn take_registered(slot: &mut Slot) -> Option<HotKey> {
        slot.id = None;
        slot.hotkey.take()
    }
}
