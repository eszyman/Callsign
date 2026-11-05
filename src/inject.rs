//! Play a key sequence into the focused window.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::bus::WorkerEvent;
use crate::cues::CueKind;
use crate::focus;
use crate::keys::{NamedKey, Step};

pub struct InjectRequest {
    pub epoch: u64,
    pub steps: Vec<Step>,
    pub abort: Vec<Step>,
    pub char_delay_ms: u64,
    pub focus_title: String,
    pub require_focus: bool,
    pub headline: String,
    pub detail: String,
    pub cue: CueKind,
}

pub fn worker(rx: Receiver<InjectRequest>, tx: Sender<WorkerEvent>, epoch: Arc<AtomicU64>) {
    let mut enigo = Enigo::new(&Settings::default());
    while let Ok(job) = rx.recv() {
        let cue = job.cue;
        let headline = job.headline.clone();
        let detail = job.detail.clone();
        let job_epoch = job.epoch;
        let result = match enigo.as_mut() {
            Ok(enigo) => run_job(&job, enigo, &epoch),
            Err(err) => Err(format!("Key output isn't available. {err}")),
        };
        let _ = tx.send(WorkerEvent::Injected {
            epoch: job_epoch,
            cue,
            headline,
            detail,
            result,
        });
    }
}

fn run_job(job: &InjectRequest, enigo: &mut Enigo, epoch: &AtomicU64) -> Result<(), String> {
    if epoch.load(Ordering::SeqCst) != job.epoch {
        return Err("Cancelled.".into());
    }
    let needle = job.focus_title.trim();
    if !needle.is_empty() {
        let _ = focus::focus_title(needle);
        thread::sleep(Duration::from_millis(90));
    }
    if job.require_focus && !needle.is_empty() {
        match focus::foreground_title() {
            Ok(title) if focus::title_matches(&title, needle) => {}
            Ok(title) => {
                return Err(format!(
                    "The focused window is \"{title}\", not one titled with \"{needle}\". Nothing was typed."
                ));
            }
            Err(err) => {
                return Err(format!(
                    "Couldn't check the focused window ({err}). Nothing was typed."
                ));
            }
        }
    }
    let mut out = EnigoOut {
        enigo,
        held: Vec::new(),
        epoch,
        expected: job.epoch,
    };
    let played = play(&job.steps, &mut out, job.char_delay_ms);
    if played.is_err() && !job.abort.is_empty() {
        let _ = play(&job.abort, &mut out, 0);
    }
    out.release_all();
    played
}

fn play(steps: &[Step], out: &mut dyn Out, char_delay_ms: u64) -> Result<(), String> {
    for step in steps {
        if out.cancelled() {
            return Err("Cancelled.".into());
        }
        match step {
            Step::Delay { ms } => out.wait(*ms),
            Step::Text { text } => {
                for ch in text.chars() {
                    if out.cancelled() {
                        return Err("Cancelled.".into());
                    }
                    out.text(&ch.to_string())?;
                    if char_delay_ms > 0 {
                        out.wait(char_delay_ms);
                    }
                }
            }
            Step::Chord { modifiers, key } => {
                for modifier in modifiers {
                    out.down(*modifier)?;
                }
                if !modifiers.is_empty() {
                    out.wait(15);
                }
                out.down(*key)?;
                out.wait(25);
                out.up(*key)?;
                if !modifiers.is_empty() {
                    out.wait(10);
                }
                for modifier in modifiers.iter().rev() {
                    out.up(*modifier)?;
                }
            }
        }
    }
    Ok(())
}

trait Out {
    fn down(&mut self, key: NamedKey) -> Result<(), String>;
    fn up(&mut self, key: NamedKey) -> Result<(), String>;
    fn text(&mut self, text: &str) -> Result<(), String>;
    fn wait(&mut self, ms: u64);
    fn cancelled(&self) -> bool;
}

struct EnigoOut<'a> {
    enigo: &'a mut Enigo,
    held: Vec<NamedKey>,
    epoch: &'a AtomicU64,
    expected: u64,
}

impl EnigoOut<'_> {
    fn release_all(&mut self) {
        for key in self.held.drain(..).rev() {
            if let Ok(mapped) = to_enigo(key) {
                let _ = self.enigo.key(mapped, Direction::Release);
            }
        }
    }
}

impl Out for EnigoOut<'_> {
    fn down(&mut self, key: NamedKey) -> Result<(), String> {
        self.enigo
            .key(to_enigo(key)?, Direction::Press)
            .map_err(|err| err.to_string())?;
        self.held.push(key);
        Ok(())
    }

    fn up(&mut self, key: NamedKey) -> Result<(), String> {
        self.enigo
            .key(to_enigo(key)?, Direction::Release)
            .map_err(|err| err.to_string())?;
        self.held.retain(|held| *held != key);
        Ok(())
    }

    fn text(&mut self, text: &str) -> Result<(), String> {
        self.enigo.text(text).map_err(|err| err.to_string())
    }

    fn wait(&mut self, ms: u64) {
        if ms > 0 {
            thread::sleep(Duration::from_millis(ms));
        }
    }

    fn cancelled(&self) -> bool {
        self.epoch.load(Ordering::SeqCst) != self.expected
    }
}

pub fn to_enigo(key: NamedKey) -> Result<Key, String> {
    Ok(match key {
        NamedKey::Char(ch) => Key::Unicode(ch),
        NamedKey::Enter => Key::Return,
        NamedKey::Escape => Key::Escape,
        NamedKey::Tab => Key::Tab,
        NamedKey::Space => Key::Space,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::Insert => Key::Insert,
        NamedKey::Left => Key::LeftArrow,
        NamedKey::Right => Key::RightArrow,
        NamedKey::Up => Key::UpArrow,
        NamedKey::Down => Key::DownArrow,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::Grave => Key::Unicode('`'),
        NamedKey::Slash => Key::Unicode('/'),
        NamedKey::Comma => Key::Unicode(','),
        NamedKey::Period => Key::Unicode('.'),
        NamedKey::Minus => Key::Unicode('-'),
        NamedKey::Equal => Key::Unicode('='),
        NamedKey::F(n) => function_key(n)?,
        NamedKey::ScrollLock => scroll_lock(),
        NamedKey::Pause => Key::Pause,
        NamedKey::LCtrl => Key::LControl,
        NamedKey::RCtrl => Key::RControl,
        NamedKey::LShift => Key::LShift,
        NamedKey::RShift => Key::RShift,
        NamedKey::LAlt => alt_left(),
        NamedKey::RAlt => alt_right(),
        NamedKey::LWin => Key::Meta,
        NamedKey::RWin => win_right(),
        NamedKey::Numpad(n) => numpad(n)?,
    })
}

fn alt_left() -> Key {
    #[cfg(target_os = "macos")]
    {
        Key::Option
    }
    #[cfg(not(target_os = "macos"))]
    {
        Key::LMenu
    }
}

fn alt_right() -> Key {
    // Right Alt is a different virtual key on each OS. IL-2 binds friendly
    // chat and a few cockpit keys to that side specifically.
    #[cfg(target_os = "windows")]
    {
        Key::RMenu
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Key::Other(0xFFEA) // XK_Alt_R
    }
    #[cfg(target_os = "macos")]
    {
        Key::ROption
    }
}

fn win_right() -> Key {
    #[cfg(target_os = "windows")]
    {
        Key::RWin
    }
    #[cfg(not(target_os = "windows"))]
    {
        Key::Meta
    }
}

fn scroll_lock() -> Key {
    #[cfg(target_os = "windows")]
    {
        Key::Scroll
    }
    #[cfg(not(target_os = "windows"))]
    {
        Key::ScrollLock
    }
}

fn function_key(n: u8) -> Result<Key, String> {
    Ok(match n {
        1 => Key::F1,
        2 => Key::F2,
        3 => Key::F3,
        4 => Key::F4,
        5 => Key::F5,
        6 => Key::F6,
        7 => Key::F7,
        8 => Key::F8,
        9 => Key::F9,
        10 => Key::F10,
        11 => Key::F11,
        12 => Key::F12,
        13 => Key::F13,
        14 => Key::F14,
        15 => Key::F15,
        16 => Key::F16,
        17 => Key::F17,
        18 => Key::F18,
        19 => Key::F19,
        20 => Key::F20,
        21 => Key::F21,
        22 => Key::F22,
        23 => Key::F23,
        24 => Key::F24,
        _ => return Err(format!("F{n} is out of range.")),
    })
}

fn numpad(n: u8) -> Result<Key, String> {
    Ok(match n {
        0 => Key::Numpad0,
        1 => Key::Numpad1,
        2 => Key::Numpad2,
        3 => Key::Numpad3,
        4 => Key::Numpad4,
        5 => Key::Numpad5,
        6 => Key::Numpad6,
        7 => Key::Numpad7,
        8 => Key::Numpad8,
        9 => Key::Numpad9,
        _ => return Err(format!("Numpad {n} is out of range.")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Recording {
        events: Vec<String>,
    }

    impl Out for Recording {
        fn down(&mut self, key: NamedKey) -> Result<(), String> {
            self.events
                .push(format!("down:{}", crate::keys::describe_key(key)));
            Ok(())
        }
        fn up(&mut self, key: NamedKey) -> Result<(), String> {
            self.events
                .push(format!("up:{}", crate::keys::describe_key(key)));
            Ok(())
        }
        fn text(&mut self, text: &str) -> Result<(), String> {
            self.events.push(format!("text:{text}"));
            Ok(())
        }
        fn wait(&mut self, ms: u64) {
            self.events.push(format!("wait:{ms}"));
        }
        fn cancelled(&self) -> bool {
            false
        }
    }

    #[test]
    fn chord_then_text_lifts_modifiers_before_typing() {
        let mut steps = crate::keys::parse_sequence("rctrl+enter").unwrap();
        steps.push(Step::Text { text: "hi".into() });
        let mut out = Recording { events: Vec::new() };
        play(&steps, &mut out, 0).unwrap();
        let keys: Vec<_> = out
            .events
            .into_iter()
            .filter(|event| !event.starts_with("wait:"))
            .collect();
        assert_eq!(
            keys,
            vec![
                "down:Right Ctrl",
                "down:Enter",
                "up:Enter",
                "up:Right Ctrl",
                "text:h",
                "text:i",
            ]
        );
    }

    #[test]
    fn preset_keys_map_into_enigo() {
        for key in [
            NamedKey::Char('g'),
            NamedKey::Enter,
            NamedKey::Grave,
            NamedKey::RAlt,
            NamedKey::RCtrl,
            NamedKey::LShift,
            NamedKey::Slash,
            NamedKey::F(13),
        ] {
            to_enigo(key).unwrap();
        }
    }
}
