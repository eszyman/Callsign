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

/// Which sim Callsign is talking to.
///
/// Korea and Great Battles keep separate chat channels and command lists.
/// The talk button, microphone, and speech engine stay shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Korea,
    Gb,
}

impl Default for Game {
    fn default() -> Self {
        Self::Korea
    }
}

impl Game {
    pub fn label(self) -> &'static str {
        match self {
            Self::Korea => "IL-2 Series: Korea",
            Self::Gb => "IL-2 Great Battles",
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Korea => "korea",
            Self::Gb => "gb",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Korea => Self::Gb,
            Self::Gb => Self::Korea,
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "korea" | "il2-korea" | "series-korea" => Some(Self::Korea),
            "gb" | "great-battles" | "greatbattles" | "battles" => Some(Self::Gb),
            _ => None,
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

/// Chat channels, command list, and the sim window title for one game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameProfile {
    pub focus_title: String,
    pub chat: ChatSettings,
    pub commands: Vec<Command>,
    pub next_id: u64,
}

impl GameProfile {
    pub fn fresh(game: Game) -> Self {
        let commands = starter_commands(game);
        let next_id = commands.iter().map(|command| command.id).max().unwrap_or(0) + 1;
        Self {
            focus_title: default_focus_title(),
            chat: ChatSettings::il2(),
            commands,
            next_id,
        }
    }
}

impl Default for GameProfile {
    fn default() -> Self {
        Self::fresh(Game::Korea)
    }
}

fn default_version() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    pub game: Game,
    pub korea: GameProfile,
    pub gb: GameProfile,
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
    #[serde(default = "default_true")]
    pub require_focus: bool,
    #[serde(default = "default_true")]
    pub cues: bool,
    #[serde(default = "default_cue_volume")]
    pub cue_volume: f32,
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
        Self {
            version: default_version(),
            game: Game::Korea,
            korea: GameProfile::fresh(Game::Korea),
            gb: GameProfile::fresh(Game::Gb),
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
            require_focus: true,
            cues: true,
            cue_volume: default_cue_volume(),
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

/// What `load` wants the desk to say, besides the settings themselves.
pub struct LoadNotes {
    pub problem: Option<String>,
    pub migrated: Option<String>,
}

impl Config {
    pub fn load(launch: Option<Game>) -> (Self, LoadNotes) {
        let path = config_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => {
                return (
                    Self::for_launch(launch),
                    LoadNotes {
                        problem: None,
                        migrated: None,
                    },
                )
            }
        };
        match parse_config(&text, launch) {
            Ok((config, migrated)) => (
                config,
                LoadNotes {
                    problem: None,
                    migrated,
                },
            ),
            Err(err) => {
                let backup = path.with_extension("json.bak");
                let _ = fs::rename(&path, &backup);
                let warning = format!(
                    "Settings file could not be read ({err}). Started fresh. The old file is at {}.",
                    backup.display()
                );
                (
                    Self::for_launch(launch),
                    LoadNotes {
                        problem: Some(warning),
                        migrated: None,
                    },
                )
            }
        }
    }

    fn for_launch(launch: Option<Game>) -> Self {
        let mut config = Self::default();
        if let Some(game) = launch {
            config.game = game;
        }
        config
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
        normalize_profile(&mut self.korea);
        normalize_profile(&mut self.gb);
        self.long_press_ms = self.long_press_ms.clamp(300, 1_500);
        self.command_gap_ms = self.command_gap_ms.min(400);
        self.command_lead_ms = self.command_lead_ms.min(600);
        self.max_record_ms = self.max_record_ms.clamp(3_000, 60_000);
        self.cue_volume = self.cue_volume.clamp(0.0, 0.4);
        self.silence_rms = self.silence_rms.clamp(0.0, 0.2);
        self.token_floor = self.token_floor.clamp(0.5, 0.99);
        if self.version < 2 {
            self.version = 2;
        }
    }

    pub fn set_game(&mut self, game: Game) {
        self.game = game;
    }

    pub fn active(&self) -> &GameProfile {
        match self.game {
            Game::Korea => &self.korea,
            Game::Gb => &self.gb,
        }
    }

    pub fn active_mut(&mut self) -> &mut GameProfile {
        match self.game {
            Game::Korea => &mut self.korea,
            Game::Gb => &mut self.gb,
        }
    }

    pub fn chat(&self) -> &ChatSettings {
        &self.active().chat
    }

    pub fn chat_mut(&mut self) -> &mut ChatSettings {
        &mut self.active_mut().chat
    }

    pub fn commands(&self) -> &[Command] {
        &self.active().commands
    }

    pub fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.active_mut().commands
    }

    pub fn focus_title(&self) -> &str {
        &self.active().focus_title
    }

    pub fn focus_title_mut(&mut self) -> &mut String {
        &mut self.active_mut().focus_title
    }

    pub fn resolved_model_path(&self) -> PathBuf {
        if self.stt.model_path.trim().is_empty() {
            default_model_path()
        } else {
            PathBuf::from(&self.stt.model_path)
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let profile = self.active_mut();
        let id = profile.next_id;
        profile.next_id += 1;
        id
    }
}

fn normalize_profile(profile: &mut GameProfile) {
    if profile.chat.profiles.is_empty() {
        profile.chat = ChatSettings::il2();
    }
    if profile.chat.profile >= profile.chat.profiles.len() {
        profile.chat.profile = 0;
    }
    if profile.chat.max_chars == 0 {
        profile.chat.max_chars = 160;
    }
    let max_id = profile
        .commands
        .iter()
        .map(|command| command.id)
        .max()
        .unwrap_or(0);
    if profile.next_id <= max_id {
        profile.next_id = max_id + 1;
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        parse_value(value, None)
            .map(|(config, _)| config)
            .map_err(serde::de::Error::custom)
    }
}

pub fn parse_config(text: &str, launch: Option<Game>) -> Result<(Config, Option<String>), String> {
    let value = serde_json::from_str(text).map_err(|err| err.to_string())?;
    parse_value(value, launch)
}

fn parse_value(
    value: serde_json::Value,
    launch: Option<Game>,
) -> Result<(Config, Option<String>), String> {
    let raw: RawConfig = serde_json::from_value(value).map_err(|err| err.to_string())?;
    let legacy = raw.korea.is_none() && raw.gb.is_none();
    let mut game = raw.game.unwrap_or(Game::Korea);
    let mut korea = raw.korea.unwrap_or_else(|| GameProfile::fresh(Game::Korea));
    let mut gb = raw.gb.unwrap_or_else(|| GameProfile::fresh(Game::Gb));
    let mut migrated = None;
    if legacy {
        let target = launch.unwrap_or(game);
        let carried = GameProfile {
            focus_title: raw.focus_title.clone().unwrap_or_else(default_focus_title),
            chat: raw.chat.clone().unwrap_or_else(ChatSettings::il2),
            commands: raw
                .commands
                .clone()
                .unwrap_or_else(|| starter_commands(target)),
            next_id: raw.next_id.unwrap_or(0),
        };
        match target {
            Game::Korea => korea = carried,
            Game::Gb => gb = carried,
        }
        game = target;
        let other = target.other();
        migrated = Some(format!(
            "Saved commands and chat are now the {} profile. {} starts from its own starter list. Switch the game in Setup if those saved commands were for {}.",
            target.label(),
            other.label(),
            other.label()
        ));
    } else if let Some(launch) = launch {
        game = launch;
    }
    let mut config = Config {
        version: raw.version,
        game,
        korea,
        gb,
        mode: raw.mode,
        ptt: raw.ptt,
        joystick: raw.joystick,
        command_ptt: raw.command_ptt,
        command_joystick: raw.command_joystick,
        channel_ptt: raw.channel_ptt,
        channel_joystick: raw.channel_joystick,
        long_press_ms: raw.long_press_ms,
        chat_gesture: raw.chat_gesture,
        command_gesture: raw.command_gesture,
        command_gap_ms: raw.command_gap_ms,
        command_lead_ms: raw.command_lead_ms,
        send_keys: raw.send_keys,
        require_focus: raw.require_focus,
        cues: raw.cues,
        cue_volume: raw.cue_volume,
        audio_device: raw.audio_device,
        silence_rms: raw.silence_rms,
        max_record_ms: raw.max_record_ms,
        stt: raw.stt,
        token_floor: raw.token_floor,
        command_max_extra: raw.command_max_extra,
        mixed_max_extra: raw.mixed_max_extra,
    };
    config.normalize();
    Ok((config, migrated))
}

#[derive(Deserialize)]
struct RawConfig {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    game: Option<Game>,
    #[serde(default)]
    korea: Option<GameProfile>,
    #[serde(default)]
    gb: Option<GameProfile>,
    #[serde(default)]
    chat: Option<ChatSettings>,
    #[serde(default)]
    commands: Option<Vec<Command>>,
    #[serde(default)]
    next_id: Option<u64>,
    #[serde(default)]
    focus_title: Option<String>,
    #[serde(default)]
    mode: Mode,
    #[serde(default = "default_ptt")]
    ptt: String,
    #[serde(default)]
    joystick: Option<JoyBind>,
    #[serde(default)]
    command_ptt: String,
    #[serde(default)]
    command_joystick: Option<JoyBind>,
    #[serde(default)]
    channel_ptt: String,
    #[serde(default)]
    channel_joystick: Option<JoyBind>,
    #[serde(default = "default_long_press")]
    long_press_ms: u64,
    #[serde(default = "default_chat_gesture")]
    chat_gesture: Gesture,
    #[serde(default = "default_command_gesture")]
    command_gesture: Gesture,
    #[serde(default = "default_command_gap")]
    command_gap_ms: u64,
    #[serde(default)]
    command_lead_ms: u64,
    #[serde(default)]
    send_keys: bool,
    #[serde(default = "default_true")]
    require_focus: bool,
    #[serde(default = "default_true")]
    cues: bool,
    #[serde(default = "default_cue_volume")]
    cue_volume: f32,
    #[serde(default)]
    audio_device: Option<String>,
    #[serde(default = "default_silence")]
    silence_rms: f32,
    #[serde(default = "default_max_record")]
    max_record_ms: u64,
    #[serde(default)]
    stt: SttConfig,
    #[serde(default = "default_floor")]
    token_floor: f64,
    #[serde(default = "default_command_extra")]
    command_max_extra: usize,
    #[serde(default = "default_mixed_extra")]
    mixed_max_extra: usize,
}

/// How the process was started. `-mode` picks the sim. `--transcribe` stays a tool.
#[derive(Debug)]
pub enum Startup {
    Gui { game: Option<Game> },
    Transcribe { file: Option<String> },
}

pub fn parse_startup<I>(args: I) -> Result<Startup, String>
where
    I: IntoIterator<Item = String>,
{
    let mut game = None;
    let mut transcribe = false;
    let mut file = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--transcribe" {
            transcribe = true;
            file = args.next();
            continue;
        }
        if arg == "-mode" || arg == "--mode" {
            let Some(value) = args.next() else {
                return Err(usage("Missing the game after -mode. Use korea or gb."));
            };
            game = Some(parse_mode(&value)?);
            continue;
        }
        if let Some(value) = arg
            .strip_prefix("--mode=")
            .or_else(|| arg.strip_prefix("-mode="))
        {
            game = Some(parse_mode(value)?);
            continue;
        }
        return Err(usage(&format!("Unknown argument '{arg}'.")));
    }
    if transcribe {
        if game.is_some() {
            return Err(usage("-mode cannot be combined with --transcribe."));
        }
        return Ok(Startup::Transcribe { file });
    }
    Ok(Startup::Gui { game })
}

fn parse_mode(value: &str) -> Result<Game, String> {
    Game::parse(value).ok_or_else(|| usage(&format!("Unknown game '{value}'. Use korea or gb.")))
}

fn usage(detail: &str) -> String {
    format!("{detail}\nUsage: callsign [-mode korea|gb]\n       callsign --transcribe <file.wav>")
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

pub fn export_commands(game: Game, commands: &[Command]) -> Result<PathBuf, String> {
    let path = config_dir().join(format!("callsign-commands-{}.json", game.cli_name()));
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
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("commands/Korea-commands.json");
        let imported = import_commands(&path).expect("Korea command file");
        assert!(!imported.repaired);
        assert_eq!(imported.commands.len(), 321);
        for command in &imported.commands {
            if command.sequence.trim().is_empty() {
                continue;
            }
            crate::keys::parse_sequence(&command.sequence)
                .unwrap_or_else(|err| panic!("{} ({}) {err}", command.name, command.sequence));
        }
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

    fn sequence<'a>(commands: &'a [Command], name: &str) -> &'a str {
        commands
            .iter()
            .find(|command| command.name == name)
            .unwrap()
            .sequence
            .as_str()
    }

    #[test]
    fn starters_keep_a_different_command_menu_for_each_game() {
        let config = Config::default();
        assert_eq!(config.game, Game::Korea);
        assert_eq!(sequence(config.commands(), "Command menu"), "tab");
        assert_eq!(sequence(&config.gb.commands, "Command menu"), "grave");
        assert_eq!(sequence(config.commands(), "Landing gear"), "g");
        assert_eq!(sequence(&config.gb.commands, "Landing gear"), "g");
    }

    #[test]
    fn a_partial_file_keeps_the_old_defaults() {
        let (config, _) = parse_config("{}", None).unwrap();
        assert_eq!(config.ptt, "scrolllock");
        assert_eq!(config.command_gesture, Gesture::Hold);
        assert_eq!(config.chat_gesture, Gesture::Tap);
        assert!(config.require_focus);
        assert!(config.cues);
        assert_eq!(config.command_gap_ms, 100);
        assert_eq!(config.long_press_ms, 700);
        assert!((config.cue_volume - 0.12).abs() < 0.001);
        assert_eq!(config.command_max_extra, 4);
        assert_eq!(config.mixed_max_extra, 1);
    }

    #[test]
    fn legacy_settings_stay_on_the_launched_game() {
        let raw = r#"{
            "version": 1,
            "mode": "command",
            "ptt": "f13",
            "send_keys": true,
            "focus_title": "My IL-2",
            "chat": {
                "profile": 1,
                "profiles": [
                    {"name": "All pilots", "open": "enter", "send": "enter", "abort": "escape"},
                    {"name": "Squad", "open": "rctrl+enter", "send": "enter", "abort": "escape"}
                ],
                "char_delay_ms": 12,
                "after_open_ms": 160,
                "max_chars": 160
            },
            "commands": [
                {"id": 7, "name": "Gear", "group": "Flight", "phrases": ["gear"], "sequence": "g", "enabled": true}
            ],
            "next_id": 8
        }"#;
        let (korea, note) = parse_config(raw, None).unwrap();
        assert_eq!(korea.game, Game::Korea);
        assert_eq!(korea.mode, Mode::Command);
        assert_eq!(korea.ptt, "f13");
        assert!(korea.send_keys);
        assert_eq!(korea.focus_title(), "My IL-2");
        assert_eq!(korea.chat().active().unwrap().name, "Squad");
        assert_eq!(korea.commands().len(), 1);
        assert_eq!(korea.commands()[0].name, "Gear");
        assert_eq!(sequence(&korea.gb.commands, "Command menu"), "grave");
        assert!(note.unwrap().contains("IL-2 Series: Korea"));

        let (gb, _) = parse_config(raw, Some(Game::Gb)).unwrap();
        assert_eq!(gb.game, Game::Gb);
        assert_eq!(gb.commands().len(), 1);
        assert_eq!(gb.commands()[0].name, "Gear");
        assert_eq!(gb.focus_title(), "My IL-2");
        assert_eq!(sequence(&gb.korea.commands, "Command menu"), "tab");
        assert!(gb
            .korea
            .commands
            .iter()
            .any(|command| command.name == "Landing gear"));
    }

    #[test]
    fn games_keep_separate_chat_and_commands_across_a_save() {
        let mut config = Config::default();
        config.chat_mut().profiles[0].name = "Everyone".into();
        config.commands_mut()[0].sequence = "x".into();
        config.set_game(Game::Gb);
        assert_eq!(config.chat().profiles[0].name, "All pilots");
        config.chat_mut().profiles[0].name = "All GB".into();
        config.commands_mut()[0].sequence = "y".into();
        config.set_game(Game::Korea);
        assert_eq!(config.chat().profiles[0].name, "Everyone");
        assert_eq!(config.commands()[0].sequence, "x");

        let json = serde_json::to_value(&config).unwrap();
        assert!(json.get("chat").is_none());
        assert!(json.get("commands").is_none());
        assert!(json.get("focus_title").is_none());
        let (loaded, note) = parse_config(&json.to_string(), Some(Game::Gb)).unwrap();
        assert!(note.is_none());
        assert_eq!(loaded.game, Game::Gb);
        assert_eq!(loaded.chat().profiles[0].name, "All GB");
        assert_eq!(loaded.commands()[0].sequence, "y");
        assert_eq!(loaded.korea.chat.profiles[0].name, "Everyone");
        assert_eq!(loaded.korea.commands[0].sequence, "x");
    }

    #[test]
    fn startup_flag_selects_the_game() {
        assert!(matches!(
            parse_startup(None::<String>).unwrap(),
            Startup::Gui { game: None }
        ));
        match parse_startup(["-mode".into(), "korea".into()]).unwrap() {
            Startup::Gui {
                game: Some(Game::Korea),
            } => {}
            other => panic!("unexpected {other:?}"),
        }
        match parse_startup(["--mode".into(), "gb".into()]).unwrap() {
            Startup::Gui {
                game: Some(Game::Gb),
            } => {}
            other => panic!("unexpected {other:?}"),
        }
        match parse_startup(["-mode=gb".into()]).unwrap() {
            Startup::Gui {
                game: Some(Game::Gb),
            } => {}
            other => panic!("unexpected {other:?}"),
        }
        match parse_startup(["--mode=Korea".into()]).unwrap() {
            Startup::Gui {
                game: Some(Game::Korea),
            } => {}
            other => panic!("unexpected {other:?}"),
        }
        assert!(parse_startup(["-mode".into(), "dcs".into()]).is_err());
        assert!(parse_startup(["-mode".into()]).is_err());
        assert!(parse_startup(["--nope".into()]).is_err());
        match parse_startup(["--transcribe".into(), "sample.wav".into()]).unwrap() {
            Startup::Transcribe { file: Some(path) } => assert_eq!(path, "sample.wav"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(parse_startup([
            "-mode".into(),
            "korea".into(),
            "--transcribe".into(),
            "a.wav".into()
        ])
        .is_err());
    }
}
