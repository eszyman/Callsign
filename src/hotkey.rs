//! Talk, command, and chat-channel keys. They are released while a text
//! field is focused so that field can still be typed into.

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::keys::{self, Chord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRole {
    Talk,
    Command,
    Channel,
}

pub struct KeyEdge {
    pub role: KeyRole,
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
    channel: Slot,
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
                channel: Slot::empty(),
                suspended: false,
                manager_error: None,
            },
            Err(err) => Self {
                manager: None,
                talk: Slot::empty(),
                command: Slot::empty(),
                channel: Slot::empty(),
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

    pub fn channel_error(&self) -> Option<&str> {
        self.channel.error.as_deref()
    }

    pub fn set_talk(&mut self, binding: Result<Chord, String>) {
        self.set_slot(KeyRole::Talk, Some(binding));
    }

    pub fn set_command(&mut self, binding: Option<Result<Chord, String>>) {
        self.set_slot(KeyRole::Command, binding);
    }

    pub fn set_channel(&mut self, binding: Option<Result<Chord, String>>) {
        self.set_slot(KeyRole::Channel, binding);
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
            let role = if self.talk.id == Some(event.id) {
                KeyRole::Talk
            } else if self.command.id == Some(event.id) {
                KeyRole::Command
            } else if self.channel.id == Some(event.id) {
                KeyRole::Channel
            } else {
                continue;
            };
            let down = event.state == HotKeyState::Pressed;
            if !down && event.state != HotKeyState::Released {
                continue;
            }
            edges.push(KeyEdge { role, down });
        }
        edges
    }

    fn set_slot(&mut self, role: KeyRole, binding: Option<Result<Chord, String>>) {
        let parsed = match binding {
            None => None,
            Some(Ok(chord)) => Some(chord),
            Some(Err(err)) => {
                let slot = self.slot_mut(role);
                slot.binding = None;
                slot.error = Some(err);
                self.sync();
                return;
            }
        };
        {
            let slot = self.slot_mut(role);
            slot.binding = parsed;
            slot.error = None;
        }
        if self.manager.is_none() {
            return;
        }
        self.sync();
    }

    fn sync(&mut self) {
        let talk_key = Self::take_registered(&mut self.talk);
        let command_key = Self::take_registered(&mut self.command);
        let channel_key = Self::take_registered(&mut self.channel);
        if let Some(manager) = self.manager.as_ref() {
            for hotkey in [talk_key, command_key, channel_key].into_iter().flatten() {
                let _ = manager.unregister(hotkey);
            }
        }
        if self.suspended || self.manager.is_none() {
            return;
        }
        self.note_duplicate(
            KeyRole::Command,
            self.same_binding(KeyRole::Talk, KeyRole::Command),
            "Use a different key from the talk button.",
        );
        let channel_clash = if self.same_binding(KeyRole::Talk, KeyRole::Channel) {
            Some("Use a different key from the talk button.")
        } else if self.same_binding(KeyRole::Command, KeyRole::Channel) {
            Some("Use a different key from the commands button.")
        } else {
            None
        };
        self.note_duplicate(
            KeyRole::Channel,
            channel_clash.is_some(),
            channel_clash.unwrap_or(""),
        );
        self.register(KeyRole::Talk);
        if !self.same_binding(KeyRole::Talk, KeyRole::Command) {
            self.register(KeyRole::Command);
        }
        if channel_clash.is_none() {
            self.register(KeyRole::Channel);
        }
    }

    fn same_binding(&self, left: KeyRole, right: KeyRole) -> bool {
        match (
            self.slot(left).binding.as_ref(),
            self.slot(right).binding.as_ref(),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }

    fn note_duplicate(&mut self, role: KeyRole, duplicate: bool, message: &str) {
        let slot = self.slot_mut(role);
        if duplicate {
            slot.error = Some(message.to_string());
        } else if slot
            .error
            .as_deref()
            .is_some_and(|err| err.starts_with("Use a different key"))
        {
            slot.error = None;
        }
    }

    fn register(&mut self, role: KeyRole) {
        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        let Some(chord) = self.slot(role).binding.clone() else {
            return;
        };
        match keys::chord_to_hotkey(&chord) {
            Ok((mods, code)) => {
                let hotkey = HotKey::new(mods, code);
                let id = hotkey.id();
                match manager.register(hotkey) {
                    Ok(()) => {
                        let slot = self.slot_mut(role);
                        slot.hotkey = Some(hotkey);
                        slot.id = Some(id);
                        slot.error = None;
                    }
                    Err(err) => {
                        self.slot_mut(role).error = Some(format!(
                            "Couldn't claim that key ({err}). Pick another, or use the on-screen button."
                        ));
                    }
                }
            }
            Err(err) => {
                self.slot_mut(role).error = Some(err);
            }
        }
    }

    fn slot(&self, role: KeyRole) -> &Slot {
        match role {
            KeyRole::Talk => &self.talk,
            KeyRole::Command => &self.command,
            KeyRole::Channel => &self.channel,
        }
    }

    fn slot_mut(&mut self, role: KeyRole) -> &mut Slot {
        match role {
            KeyRole::Talk => &mut self.talk,
            KeyRole::Command => &mut self.command,
            KeyRole::Channel => &mut self.channel,
        }
    }

    fn take_registered(slot: &mut Slot) -> Option<HotKey> {
        slot.id = None;
        slot.hotkey.take()
    }
}
