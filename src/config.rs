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
    #[serde(default = "default_double_tap")]
    pub double_tap_ms: u64,
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
fn default_double_tap() -> u64 {
    320
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
            double_tap_ms: default_double_tap(),
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
        self.double_tap_ms = self.double_tap_ms.clamp(140, 600);
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

pub fn import_commands(path: &Path) -> Result<Vec<Command>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("Couldn't read that file. {err}"))?;
    if let Ok(file) = serde_json::from_str::<CommandFile>(&text) {
        return Ok(file.commands);
    }
    serde_json::from_str::<Vec<Command>>(&text)
        .map_err(|_| "That file isn't a Callsign command list.".into())
}

#[derive(Serialize, Deserialize)]
struct CommandFile {
    commands: Vec<Command>,
}
