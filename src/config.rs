//! Settings live in the user config directory, not in the game folder.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::presets::starter_commands;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Chat,
    Command,
    Mixed,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Chat
    }
}

/// How a press becomes a recording.
///
/// Tap is press, speak, press again. Hold is press and speak, then release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    Tap,
    Hold,
}

impl Default for Gesture {
    fn default() -> Self {
        Self::Tap
    }
}

impl Gesture {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tap => "Press twice",
            Self::Hold => "Hold to talk",
        }
    }
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Chat",
            Self::Command => "Commands",
            Self::Mixed => "Mixed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SttEngine {
    Local,
    Http,
}

impl Default for SttEngine {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SttConfig {
    pub engine: SttEngine,
    pub model_path: String,
    pub language: String,
    pub api_base: String,
    pub api_key: String,
    pub api_model: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            engine: SttEngine::Local,
            model_path: String::new(),
            language: "en".into(),
            api_base: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            api_model: "whisper-1".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatProfile {
    pub name: String,
    pub open: String,
    pub send: String,
    pub abort: String,
}

impl Default for ChatProfile {
    fn default() -> Self {
        Self {
            name: "All pilots".into(),
            open: "enter".into(),
            send: "enter".into(),
            abort: "escape".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatSettings {
    pub profile: usize,
    pub profiles: Vec<ChatProfile>,
    pub char_delay_ms: u64,
    pub after_open_ms: u64,
    pub max_chars: usize,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self::il2()
    }
}

impl ChatSettings {
    pub fn il2() -> Self {
        Self {
            profile: 0,
            profiles: vec![
                ChatProfile {
                    name: "All pilots".into(),
                    open: "enter".into(),
                    send: "enter".into(),
                    abort: "escape".into(),
                },
                ChatProfile {
                    name: "Friendly".into(),
                    open: "rctrl+enter".into(),
                    send: "enter".into(),
                    abort: "escape".into(),
                },
            ],
            char_delay_ms: 12,
            after_open_ms: 160,
            max_chars: 160,
        }
    }

    pub fn active(&self) -> Option<&ChatProfile> {
        self.profiles
            .get(self.profile)
            .or_else(|| self.profiles.first())
    }

    /// The next channel after a press of the chat-channel button.
    /// Two channels swap. More than two step forward and wrap.
    pub fn next_profile(&self) -> Option<usize> {
        let count = self.profiles.len();
        if count < 2 {
            return None;
        }
        Some((self.profile + 1) % count)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoyBind {
    pub guid: String,
    pub name: String,
    pub button: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: u64,
    pub name: String,
    pub group: String,
    pub phrases: Vec<String>,
    pub sequence: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub mode: Mode,
    #[serde(default = "default_ptt")]
    pub ptt: String,
    pub joystick: Option<JoyBind>,
    /// Keyboard key that always starts a command, even in Chat mode.
    #[serde(default)]
    pub command_ptt: String,
    pub command_joystick: Option<JoyBind>,
    /// Key that flips the chat line between All pilots and Friendly.
    #[serde(default)]
    pub channel_ptt: String,
    pub channel_joystick: Option<JoyBind>,
    /// How long the second press must be held before the line is dropped.
    #[serde(default = "default_long_press")]
    pub long_press_ms: u64,
    /// Talk button gesture in Chat and Mixed.
    #[serde(default = "default_chat_gesture")]
    pub chat_gesture: Gesture,
    /// Commands button gesture, and the talk button while the header is Commands.
    #[serde(default = "default_command_gesture")]
    pub command_gesture: Gesture,
    /// Wait inserted between command keys that are not already spaced.
    #[serde(default = "default_command_gap")]
    pub command_gap_ms: u64,
    /// Wait before the first key of a command.
    #[serde(default)]
    pub command_lead_ms: u64,
    pub send_keys: bool,
    #[serde(default = "default_focus_title")]
    pub focus_title: String,
    #[serde(default = "default_true")]
    pub require_focus: bool,
    #[serde(default = "default_true")]
    pub cues: bool,
    #[serde(default = "default_cue_volume")]
    pub cue_volume: f32,
    pub chat: ChatSettings,
    pub commands: Vec<Command>,
    pub next_id: u64,
    pub audio_device: Option<String>,
    #[serde(default = "default_silence")]
    pub silence_rms: f32,
    #[serde(default = "default_max_record")]
    pub max_record_ms: u64,
    pub stt: SttConfig,
    #[serde(default = "default_floor")]
    pub token_floor: f64,
    #[serde(default = "default_command_extra")]
    pub command_max_extra: usize,
    #[serde(default = "default_mixed_extra")]
    pub mixed_max_extra: usize,
}

fn default_ptt() -> String {
    "scrolllock".into()
}
fn default_long_press() -> u64 {
    700
}
fn default_chat_gesture() -> Gesture {
    Gesture::Tap
}
fn default_command_gesture() -> Gesture {
    Gesture::Hold
}
fn default_command_gap() -> u64 {
    100
}
fn default_focus_title() -> String {
    "IL-2".into()
}
fn default_true() -> bool {
    true
}
fn default_cue_volume() -> f32 {
    0.12
}
fn default_silence() -> f32 {
    0.012
}
fn default_max_record() -> u64 {
    30_000
}
fn default_floor() -> f64 {
    0.84
}
fn default_command_extra() -> usize {
    4
}
fn default_mixed_extra() -> usize {
    1
}

impl Default for Config {
    fn default() -> Self {
        let commands = starter_commands();
        let next_id = commands.iter().map(|command| command.id).max().unwrap_or(0) + 1;
        Self {
            version: 1,
            mode: Mode::Chat,
            ptt: default_ptt(),
            joystick: None,
            command_ptt: String::new(),
            command_joystick: None,
            channel_ptt: String::new(),
            channel_joystick: None,
            long_press_ms: default_long_press(),
            chat_gesture: default_chat_gesture(),
            command_gesture: default_command_gesture(),
            command_gap_ms: default_command_gap(),
            command_lead_ms: 0,
            send_keys: false,
            focus_title: default_focus_title(),
            require_focus: true,
            cues: true,
            cue_volume: default_cue_volume(),
            chat: ChatSettings::il2(),
            commands,
            next_id,
            audio_device: None,
            silence_rms: default_silence(),
            max_record_ms: default_max_record(),
            stt: SttConfig::default(),
            token_floor: default_floor(),
            command_max_extra: default_command_extra(),
            mixed_max_extra: default_mixed_extra(),
        }
    }
}

impl Config {
    pub fn load() -> (Self, Option<String>) {
        let path = config_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => return (Self::default(), None),
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(mut config) => {
                config.normalize();
                (config, None)
            }
            Err(err) => {
                let backup = path.with_extension("json.bak");
                let _ = fs::rename(&path, &backup);
                let warning = format!(
                    "Settings file could not be read ({err}). Started fresh. The old file is at {}.",
                    backup.display()
                );
                (Self::default(), Some(warning))
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp).map_err(|err| err.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
            }
            file.write_all(json.as_bytes())
                .map_err(|err| err.to_string())?;
            file.sync_all().map_err(|err| err.to_string())?;
        }
        fs::rename(&tmp, &path).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn normalize(&mut self) {
        let max_id = self
            .commands
            .iter()
            .map(|command| command.id)
            .max()
            .unwrap_or(0);
        if self.next_id <= max_id {
            self.next_id = max_id + 1;
        }
        if self.chat.profiles.is_empty() {
            self.chat = ChatSettings::il2();
        }
        if self.chat.profile >= self.chat.profiles.len() {
            self.chat.profile = 0;
        }
        self.long_press_ms = self.long_press_ms.clamp(300, 1_500);
        self.command_gap_ms = self.command_gap_ms.min(400);
        self.command_lead_ms = self.command_lead_ms.min(600);
        self.max_record_ms = self.max_record_ms.clamp(3_000, 60_000);
        self.cue_volume = self.cue_volume.clamp(0.0, 0.4);
        self.silence_rms = self.silence_rms.clamp(0.0, 0.2);
        self.token_floor = self.token_floor.clamp(0.5, 0.99);
        if self.chat.max_chars == 0 {
            self.chat.max_chars = 160;
        }
        if self.version == 0 {
            self.version = 1;
        }
    }

    pub fn resolved_model_path(&self) -> PathBuf {
        if self.stt.model_path.trim().is_empty() {
            default_model_path()
        } else {
            PathBuf::from(&self.stt.model_path)
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "Callsign", "Callsign")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("callsign-data"))
}

pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "Callsign", "Callsign")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("callsign-data"))
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn log_path() -> PathBuf {
    config_dir().join("callsign.log")
}

pub fn default_model_path() -> PathBuf {
    data_dir().join("models").join("ggml-tiny.en.bin")
}

pub fn export_commands(commands: &[Command]) -> Result<PathBuf, String> {
    let path = config_dir().join("callsign-commands.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let file = CommandFile {
        commands: commands.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|err| err.to_string())?;
    fs::write(&path, json).map_err(|err| err.to_string())?;
    Ok(path)
}

pub struct ImportedCommands {
    pub commands: Vec<Command>,
    pub repaired: bool,
}

pub fn import_commands(path: &Path) -> Result<ImportedCommands, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("Couldn't read that file. {err}"))?;
    parse_command_list(&text)
        .map_err(|err| format!("That file isn't a Callsign command list. {err}"))
}

fn parse_command_list(text: &str) -> Result<ImportedCommands, String> {
    if let Some(commands) = decode_commands(text) {
        return Ok(ImportedCommands {
            commands,
            repaired: false,
        });
    }
    let repaired = repair_command_json(text);
    match decode_commands(&repaired) {
        Some(commands) => Ok(ImportedCommands {
            commands,
            repaired: true,
        }),
        None => Err(decode_commands_error(&repaired)),
    }
}

fn decode_commands(text: &str) -> Option<Vec<Command>> {
    if let Ok(file) = serde_json::from_str::<CommandFile>(text) {
        return Some(file.commands);
    }
    serde_json::from_str::<Vec<Command>>(text).ok()
}

fn decode_commands_error(text: &str) -> String {
    serde_json::from_str::<CommandFile>(text)
        .err()
        .map(|err| err.to_string())
        .unwrap_or_else(|| "The list could not be read.".into())
}

/// Hand-edited lists often have a trailing comma, a missing comma between
/// phrases, or a sequence written as several strings (`"tab","0","8"`).
fn repair_command_json(text: &str) -> String {
    let joined = join_split_sequences(text);
    let with_commas = insert_missing_commas(&joined);
    let without_strays = drop_stray_array_numbers(&with_commas);
    strip_trailing_commas(&without_strays)
}

/// A phrase list sometimes starts with a stray number, as in `"phrases": [3 "requesting landing"]`.
fn drop_stray_array_numbers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((_, end)) = read_json_string(text, i) {
            out.push_str(&text[i..end]);
            i = end;
            continue;
        }
        if bytes[i] == b'[' {
            out.push('[');
            let mut cursor = skip_ws(text, i + 1);
            if cursor < bytes.len() && (bytes[cursor].is_ascii_digit() || bytes[cursor] == b'-') {
                let number_end = skip_number(text, cursor);
                let after = skip_ws(text, number_end);
                if after < bytes.len() && (bytes[after] == b'"' || bytes[after] == b',') {
                    i = if bytes[after] == b',' {
                        after + 1
                    } else {
                        after
                    };
                    continue;
                }
            }
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn skip_number(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start;
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn join_split_sequences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((value, next)) = read_json_string(text, i) {
            out.push_str(&text[i..next]);
            if value == "sequence" {
                let cursor = skip_ws(text, next);
                if cursor < text.len() && text.as_bytes()[cursor] == b':' {
                    let after_colon = skip_ws(text, cursor + 1);
                    if let Some((parts, end)) = read_sequence_parts(text, after_colon) {
                        if parts.len() > 1 {
                            out.push_str(&text[next..=cursor]);
                            out.push('"');
                            out.push_str(&escape_json(&parts.join(",")));
                            out.push('"');
                            i = end;
                            continue;
                        }
                    }
                }
            }
            i = next;
            continue;
        }
        out.push(text[i..].chars().next().unwrap());
        i += text[i..].chars().next().unwrap().len_utf8();
    }
    out
}

fn read_sequence_parts(text: &str, start: usize) -> Option<(Vec<String>, usize)> {
    let (first, mut end) = read_json_string(text, start)?;
    let mut parts = vec![first];
    loop {
        let after = skip_ws(text, end);
        if after >= text.len() || text.as_bytes()[after] != b',' {
            break;
        }
        let next = skip_ws(text, after + 1);
        let (value, value_end) = read_json_string(text, next)?;
        let after_value = skip_ws(text, value_end);
        if after_value < text.len() && text.as_bytes()[after_value] == b':' {
            break;
        }
        parts.push(value);
        end = value_end;
    }
    Some((parts, end))
}

fn insert_missing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if let Some((_, end)) = read_json_string(text, i) {
            out.push_str(&text[i..end]);
            let after = skip_ws(text, end);
            if read_json_string(text, after).is_some() {
                out.push(',');
            }
            i = end;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((_, end)) = read_json_string(text, i) {
            out.push_str(&text[i..end]);
            i = end;
            continue;
        }
        if bytes[i] == b',' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b']' || bytes[j] == b'}') {
                i += 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn read_json_string(text: &str, start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if start >= bytes.len() || bytes[start] != b'"' {
        return None;
    }
    let mut value = String::new();
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((value, i + 1)),
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    return None;
                }
                match bytes[i] {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'/' => value.push('/'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    _ => return None,
                }
                i += 1;
            }
            byte => {
                let ch = text[i..].chars().next()?;
                value.push(ch);
                i += ch.len_utf8();
                let _ = byte;
            }
        }
    }
    None
}

fn skip_ws(text: &str, mut i: usize) -> usize {
    let bytes = text.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn escape_json(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Serialize, Deserialize)]
struct CommandFile {
    commands: Vec<Command>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_repairs_the_commas_in_a_hand_edited_list() {
        let raw = r#"{
          "commands": [
            {
              "id": 1,
              "name": "Accept",
              "group": "Flight",
              "phrases": ["accept",],
              "sequence": "space",
              "enabled": true
            },
            {
              "id": 2,
              "name": "Request landing",
              "group": "Flight",
              "phrases": ["requesting landing"],
              "sequence": "tab","0","8",
              "enabled": false
            },
            {
              "id": 3,
              "name": "Nearest beacon",
              "group": "Flight",
              "phrases": [3
                "select nearest airbase"
                "select nearest"
              ],
              "sequence": "ralt+insert",
              "enabled": true
            }
          ],
        }"#;
        let path = std::env::temp_dir().join("callsign-import-repair.json");
        std::fs::write(&path, raw).unwrap();
        let imported = import_commands(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(imported.repaired);
        assert_eq!(imported.commands.len(), 3);
        assert_eq!(imported.commands[1].sequence, "tab,0,8");
        assert_eq!(
            imported.commands[2].phrases,
            vec!["select nearest airbase", "select nearest"]
        );
    }

    #[test]
    fn korea_command_file_imports_and_its_keys_parse() {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("commands/il2-korea.json");
        let imported = import_commands(&path).expect("Korea command file");
        assert!(!imported.repaired);
        assert_eq!(imported.commands.len(), 57);
        let mut names = std::collections::BTreeSet::new();
        for command in &imported.commands {
            assert!(
                names.insert(command.name.clone()),
                "duplicate name {}",
                command.name
            );
            assert!(!command.phrases.is_empty(), "{}", command.name);
            if command.group == "Weapons" || command.group == "Gunner" {
                assert!(!command.enabled, "{} starts enabled", command.name);
            }
            assert!(
                !command.sequence.trim().is_empty(),
                "{} has no keys",
                command.name
            );
            crate::keys::parse_sequence(&command.sequence)
                .unwrap_or_else(|err| panic!("{} ({}) {err}", command.name, command.sequence));
        }
        let by_name = |name: &str| {
            imported
                .commands
                .iter()
                .find(|command| command.name == name)
                .unwrap_or_else(|| panic!("missing {name}"))
        };
        assert_eq!(by_name("Autopilot").sequence, "lalt+a");
        assert_eq!(by_name("Landing gear").sequence, "g");
        assert_eq!(by_name("Air brakes").sequence, "b");
        assert_eq!(by_name("Drop bombs").sequence, "lshift+space");
        assert_eq!(by_name("Master arm").sequence, "rctrl+'");
        assert!(by_name("Landing gear").enabled);
        for name in [
            "Drop bombs",
            "Launch rockets",
            "Dump fuel",
            "Drop tanks",
            "Jet boosters",
            "Jettison boosters",
            "Cease fire",
        ] {
            assert!(!by_name(name).enabled, "{name}");
        }
        assert!(imported.commands.iter().all(|command| {
            let name = command.name.to_lowercase();
            !name.contains("bail") && !name.contains("eject") && !name.contains("trigger")
        }));
    }

    #[test]
    fn channel_button_swaps_all_pilots_and_friendly() {
        let mut chat = ChatSettings::il2();
        assert_eq!(chat.active().unwrap().name, "All pilots");
        chat.profile = chat.next_profile().unwrap();
        assert_eq!(chat.active().unwrap().name, "Friendly");
        chat.profile = chat.next_profile().unwrap();
        assert_eq!(chat.active().unwrap().name, "All pilots");
        chat.profiles.truncate(1);
        assert!(chat.next_profile().is_none());
    }
}
