//! Window state: the gesture, the workers, and the saved settings.

use std::collections::{HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::audio::AudioEngine;
use crate::bus::WorkerEvent;
use crate::config::{self, Command, Config, Gesture, JoyBind, Mode, SttEngine};
use crate::cues::{CueKind, CuePlayer};
use crate::hotkey::{HotkeyHub, KeyRole};
use crate::inject::InjectRequest;
use crate::keys;
use crate::phase::{self, Effect, Phase};
use crate::plan::{self, PlanInput};
use crate::presets::merge_starter;
use crate::resample;
use crate::sticks::StickHub;
use crate::stt::{self, SttCommand};
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Operate,
    Chat,
    Commands,
    Setup,
}

#[derive(Clone, Copy)]
pub(crate) enum LogKind {
    System,
    Sent,
    Cancel,
    Problem,
    Heard,
}

pub(crate) struct LogEntry {
    pub time: String,
    pub kind: LogKind,
    pub message: String,
}

pub(crate) struct DownloadView {
    pub label: String,
    pub bytes: u64,
    pub total: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickCapture {
    Talk,
    Command,
    Channel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Press {
    Screen,
    TalkStick,
    CommandStick,
    TalkKey,
    CommandKey,
}

pub struct CallsignApp {
    pub(crate) config: Config,
    pub(crate) phase: Phase,
    pub(crate) page: Page,
    pub(crate) events: VecDeque<LogEntry>,
    pub(crate) last_headline: String,
    pub(crate) last_detail: String,
    pub(crate) try_line: String,
    pub(crate) command_test: String,
    pub(crate) import_path: String,
    pub(crate) import_note: String,
    pub(crate) command_query: String,
    pub(crate) command_category: String,
    pub(crate) command_open: HashSet<u64>,
    pub(crate) confirm_clear_commands: bool,
    pub(crate) listen_stick: Option<StickCapture>,
    pub(crate) download: Option<DownloadView>,
    pub(crate) mic_names: Vec<String>,
    pub(crate) save_error: Option<String>,
    pub(crate) audio: AudioEngine,
    pub(crate) cues: CuePlayer,
    pub(crate) sticks: StickHub,
    pub(crate) hotkeys: HotkeyHub,
    epoch: Arc<AtomicU64>,
    utterance: u64,
    gesture_mode: Option<Mode>,
    /// Gesture style captured when this recording started.
    active_gesture: Gesture,
    started: Instant,
    last_down_ms: u64,
    capture_started: Option<Instant>,
    /// The control that is physically down for the current press.
    /// A different control releasing must not send or cancel that press.
    press: Option<Press>,
    talk_stick_down: bool,
    command_stick_down: bool,
    screen_down: bool,
    applied_ptt: String,
    applied_command_ptt: String,
    applied_channel_ptt: String,
    channel_stick_down: bool,
    applied_audio: Option<String>,
    edit_snapshot: String,
    dirty: bool,
    last_edit: Instant,
    paint_while_hidden: bool,
    event_tx: Sender<WorkerEvent>,
    event_rx: Receiver<WorkerEvent>,
    stt_tx: Sender<SttCommand>,
    inject_tx: Sender<InjectRequest>,
}

impl CallsignApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let (config, warning) = Config::load();
        let epoch = Arc::new(AtomicU64::new(1));
        let (event_tx, event_rx) = mpsc::channel();
        let (stt_tx, stt_rx) = mpsc::channel();
        let (inject_tx, inject_rx) = mpsc::channel();
        {
            let tx = event_tx.clone();
            thread::spawn(move || stt::worker(stt_rx, tx));
        }
        {
            let tx = event_tx.clone();
            let epoch = epoch.clone();
            thread::spawn(move || crate::inject::worker(inject_rx, tx, epoch));
        }

        let mut audio = AudioEngine::new();
        audio.open(config.audio_device.as_deref());
        let mut hotkeys = HotkeyHub::new();
        hotkeys.set_talk(keys::parse_ptt(&config.ptt));
        hotkeys.set_command(command_binding(&config.command_ptt));
        hotkeys.set_channel(command_binding(&config.channel_ptt));
        let applied_ptt = config.ptt.clone();
        let applied_command_ptt = config.command_ptt.clone();
        let applied_channel_ptt = config.channel_ptt.clone();
        let applied_audio = config.audio_device.clone();
        let mic_names = crate::audio::list_inputs();
        let edit_snapshot = serde_json::to_string(&config).unwrap_or_default();
        let mut app = Self {
            config,
            phase: Phase::Idle,
            page: Page::Operate,
            events: VecDeque::new(),
            last_headline: "Nothing said yet".into(),
            last_detail: "A transcript lands here.".into(),
            try_line: String::new(),
            command_test: String::new(),
            import_path: String::new(),
            import_note: String::new(),
            command_query: String::new(),
            command_category: String::new(),
            command_open: HashSet::new(),
            confirm_clear_commands: false,
            listen_stick: None,
            download: None,
            mic_names,
            save_error: None,
            audio,
            cues: CuePlayer::new(),
            sticks: StickHub::new(),
            hotkeys,
            epoch,
            utterance: 0,
            gesture_mode: None,
            active_gesture: Gesture::Tap,
            started: Instant::now(),
            last_down_ms: 0,
            capture_started: None,
            press: None,
            talk_stick_down: false,
            command_stick_down: false,
            screen_down: false,
            applied_ptt,
            applied_command_ptt,
            applied_channel_ptt,
            channel_stick_down: false,
            applied_audio,
            edit_snapshot,
            dirty: false,
            last_edit: Instant::now(),
            paint_while_hidden: false,
            event_tx,
            event_rx,
            stt_tx,
            inject_tx,
        };
        if let Some(warning) = warning {
            app.push_log(LogKind::Problem, warning);
        }
        let mode_line = if app.config.send_keys {
            "Key output is on. Callsign will type into a window titled IL-2.".into()
        } else {
            "Practice mode. Nothing is typed into IL-2 until key output is turned on.".into()
        };
        app.push_log(LogKind::System, mode_line);
        if app.config.stt.engine == SttEngine::Local
            && !stt::model_is_ready(&app.config.resolved_model_path())
        {
            app.push_log(
                LogKind::System,
                "The local speech model isn't downloaded yet. Typed lines still work.".into(),
            );
        }
        app
    }

    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub(crate) fn note_screen_button(&mut self, down: bool) {
        if down && !self.screen_down {
            self.edge_down(Press::Screen, false);
        } else if !down && self.screen_down {
            self.edge_up(Press::Screen);
        }
        self.screen_down = down;
    }

    fn edge_down(&mut self, press: Press, command: bool) {
        if self.press.is_some() {
            return;
        }
        let now = self.now_ms();
        if now.saturating_sub(self.last_down_ms) < 45 {
            return;
        }
        let consumes = matches!(
            self.phase,
            Phase::Idle | Phase::Listening { opener_down: false } | Phase::Transcribing
        );
        if !consumes {
            return;
        }
        self.last_down_ms = now;
        self.press = Some(press);
        let was_idle = self.phase == Phase::Idle;
        let (phase, effect) = phase::on_down(self.phase, now);
        if was_idle && effect == Effect::BeginCapture {
            self.active_gesture = self.capture_gesture(command);
            self.gesture_mode = Some(if command {
                Mode::Command
            } else {
                self.config.mode
            });
        }
        self.phase = phase;
        self.apply(effect);
    }

    fn edge_up(&mut self, press: Press) {
        if self.press != Some(press) {
            return;
        }
        self.press = None;
        let now = self.now_ms();
        let opener = matches!(self.phase, Phase::Listening { opener_down: true });
        let hold = self.active_gesture == Gesture::Hold;
        let (phase, effect) = phase::on_up(self.phase, now, self.config.long_press_ms, hold);
        self.phase = phase;
        if opener && matches!(self.phase, Phase::Listening { opener_down: false }) {
            self.cue(CueKind::Ready);
        }
        self.apply(effect);
    }

    pub(crate) fn force_send(&mut self) {
        if phase::is_capturing(self.phase) {
            self.begin_transcribe();
        }
    }

    pub(crate) fn cancel_gesture(&mut self) {
        let effect = match self.phase {
            Phase::Idle | Phase::Dispatching => return,
            Phase::Transcribing => Effect::CancelInFlight,
            _ => Effect::DiscardCapture,
        };
        self.phase = Phase::Idle;
        self.apply(effect);
    }

    pub(crate) fn run_try_line(&mut self) {
        if self.phase != Phase::Idle {
            return;
        }
        let line = self.try_line.clone();
        if line.trim().is_empty() {
            return;
        }
        self.dispatch_text(&line, None, false);
    }

    pub(crate) fn radio_check(&mut self) {
        if self.phase != Phase::Idle {
            return;
        }
        self.try_line = "radio check".into();
        self.dispatch_text("radio check", Some(Mode::Chat), false);
    }

    pub(crate) fn run_command_test(&mut self) {
        if self.phase != Phase::Idle {
            return;
        }
        let line = self.command_test.clone();
        if line.trim().is_empty() {
            return;
        }
        self.dispatch_text(&line, Some(Mode::Command), false);
    }

    pub(crate) fn add_command(&mut self) {
        let id = self.config.alloc_id();
        self.config.commands.push(Command {
            id,
            name: "New command".into(),
            group: "Flight".into(),
            phrases: vec!["phrase".into()],
            sequence: String::new(),
            enabled: false,
        });
    }

    pub(crate) fn clear_commands(&mut self) {
        let count = self.config.commands.len();
        self.config.commands.clear();
        self.command_open.clear();
        self.command_category.clear();
        self.confirm_clear_commands = false;
        self.push_log(LogKind::System, format!("Removed {count} commands."));
    }

    pub(crate) fn add_starter(&mut self) {
        let added = merge_starter(&mut self.config.commands, &mut self.config.next_id);
        self.push_log(
            LogKind::System,
            if added == 0 {
                "Starter commands are already in the list.".into()
            } else {
                format!("Added {added} starter commands.")
            },
        );
    }

    pub(crate) fn export_commands(&mut self) {
        match config::export_commands(&self.config.commands) {
            Ok(path) => self.push_log(
                LogKind::System,
                format!("Exported commands to {}.", path.display()),
            ),
            Err(err) => self.push_log(LogKind::Problem, err),
        }
    }

    pub(crate) fn browse_commands(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Command list", &["json"])
            .set_title("Import Callsign commands");
        let parent = std::path::Path::new(self.import_path.trim()).parent();
        if let Some(parent) = parent.filter(|path| path.is_dir()) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(path) = dialog.pick_file() {
            self.import_path = path.display().to_string();
            self.import_commands();
        }
    }

    pub(crate) fn import_commands(&mut self) {
        let path = self.import_path.trim().to_string();
        if path.is_empty() {
            let message = "Choose a command file with Browse, or paste its path.".to_string();
            self.import_note = message.clone();
            self.push_log(LogKind::Problem, message);
            return;
        }
        match config::import_commands(std::path::Path::new(&path)) {
            Ok(incoming) => {
                let mut replaced = 0;
                let mut added = 0;
                let mut unreadable = Vec::new();
                for command in incoming.commands {
                    if !command.sequence.trim().is_empty()
                        && keys::parse_sequence(&command.sequence).is_err()
                    {
                        unreadable.push(command.name.clone());
                    }
                    if let Some(existing) = self
                        .config
                        .commands
                        .iter_mut()
                        .find(|current| current.name == command.name)
                    {
                        existing.group = command.group;
                        existing.phrases = command.phrases;
                        existing.sequence = command.sequence;
                        existing.enabled = command.enabled;
                        replaced += 1;
                    } else {
                        let mut command = command;
                        command.id = self.config.alloc_id();
                        self.config.commands.push(command);
                        added += 1;
                    }
                }
                let mut message = format!("Imported {added} new commands and updated {replaced}.");
                if incoming.repaired {
                    message.push_str(" A few commas in the file were repaired while reading it.");
                }
                self.push_log(LogKind::System, message.clone());
                if !unreadable.is_empty() {
                    let note = format!(
                        "{message} {} of them use a mouse control Callsign can't press, starting with {}. Those stay in the list and are marked in red.",
                        unreadable.len(),
                        unreadable[0]
                    );
                    self.import_note = note.clone();
                    self.push_log(LogKind::Problem, note);
                } else {
                    self.import_note = message;
                }
            }
            Err(err) => {
                self.import_note = err.clone();
                self.push_log(LogKind::Problem, err);
            }
        }
    }

    pub(crate) fn refresh_mics(&mut self) {
        self.mic_names = crate::audio::list_inputs();
    }

    pub(crate) fn reopen_mic(&mut self) {
        self.audio.open(self.config.audio_device.as_deref());
        self.applied_audio = self.config.audio_device.clone();
        self.refresh_mics();
    }

    pub(crate) fn start_download(&mut self, label: &str, url: &str, filename: &str) {
        if self.download.is_some() {
            return;
        }
        let dest = config::data_dir().join("models").join(filename);
        self.download = Some(DownloadView {
            label: label.into(),
            bytes: 0,
            total: None,
        });
        let tx = self.event_tx.clone();
        let url = url.to_string();
        thread::spawn(move || {
            let tx_progress = tx.clone();
            let result = stt::download_model(&url, &dest, move |bytes, total| {
                let _ = tx_progress.send(WorkerEvent::DownloadProgress { bytes, total });
            });
            let _ = tx.send(WorkerEvent::DownloadFinished {
                result: result.map(|()| dest),
            });
        });
    }

    fn pump(&mut self, ctx: &egui::Context) {
        if !self.paint_while_hidden {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            self.paint_while_hidden = true;
        }
        if ctx.input(|input| input.viewport().close_requested()) && self.dirty {
            let _ = self.config.save();
            self.dirty = false;
        }
        self.hotkeys.set_suspended(ctx.egui_wants_keyboard_input());
        if self.applied_ptt != self.config.ptt {
            self.applied_ptt = self.config.ptt.clone();
            self.hotkeys.set_talk(keys::parse_ptt(&self.config.ptt));
        }
        if self.applied_command_ptt != self.config.command_ptt {
            self.applied_command_ptt = self.config.command_ptt.clone();
            self.hotkeys
                .set_command(command_binding(&self.config.command_ptt));
        }
        if self.applied_channel_ptt != self.config.channel_ptt {
            self.applied_channel_ptt = self.config.channel_ptt.clone();
            self.hotkeys
                .set_channel(command_binding(&self.config.channel_ptt));
        }
        if self.applied_audio != self.config.audio_device {
            self.applied_audio = self.config.audio_device.clone();
            self.audio.open(self.config.audio_device.as_deref());
        }
        self.poll_sticks();
        let edges = self.hotkeys.poll();
        for edge in edges {
            if edge.role == KeyRole::Channel {
                if edge.down {
                    self.toggle_channel();
                }
                continue;
            }
            let command = edge.role == KeyRole::Command;
            let press = if command {
                Press::CommandKey
            } else {
                Press::TalkKey
            };
            if edge.down {
                self.edge_down(press, command);
            } else {
                self.edge_up(press);
            }
        }
        self.poll_events();
        self.tick_phase();
        self.tick_limit();
        if let Some(err) = self.audio.take_stream_error() {
            self.push_log(LogKind::Problem, format!("Microphone: {err}"));
        }
        self.note_edits();
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn poll_sticks(&mut self) {
        let edges = self.sticks.poll();
        if let Some(target) = self.listen_stick {
            if let Some(edge) = edges.into_iter().next() {
                let message = match target {
                    StickCapture::Talk => {
                        format!("Talk button is {} button {}.", edge.name, edge.button + 1)
                    }
                    StickCapture::Command => {
                        format!(
                            "Commands button is {} button {}.",
                            edge.name,
                            edge.button + 1
                        )
                    }
                    StickCapture::Channel => {
                        format!(
                            "Chat channel button is {} button {}.",
                            edge.name,
                            edge.button + 1
                        )
                    }
                };
                let bind = JoyBind {
                    guid: edge.guid,
                    name: edge.name,
                    button: edge.button,
                };
                match target {
                    StickCapture::Talk => self.config.joystick = Some(bind),
                    StickCapture::Command => self.config.command_joystick = Some(bind),
                    StickCapture::Channel => self.config.channel_joystick = Some(bind),
                }
                self.listen_stick = None;
                self.push_log(LogKind::System, message);
            }
            return;
        }
        self.follow_stick(false);
        self.follow_stick(true);
        self.follow_channel();
    }

    fn follow_channel(&mut self) {
        let blocked = same_bind(&self.config.joystick, &self.config.channel_joystick)
            || same_bind(&self.config.command_joystick, &self.config.channel_joystick);
        if blocked {
            self.channel_stick_down = false;
            return;
        }
        let bind = self.config.channel_joystick.clone();
        let down = bind
            .as_ref()
            .is_some_and(|bind| self.sticks.is_held(&bind.guid, &bind.name, bind.button));
        if down && !self.channel_stick_down {
            self.toggle_channel();
        }
        self.channel_stick_down = down;
    }

    fn toggle_channel(&mut self) {
        let Some(next) = self.config.chat.next_profile() else {
            self.push_log(
                LogKind::Problem,
                "Add a second chat channel before that button can switch.".into(),
            );
            return;
        };
        self.config.chat.profile = next;
        let name = self
            .config
            .chat
            .active()
            .map(|profile| profile.name.clone())
            .unwrap_or_else(|| "Chat".into());
        let friendly = name.to_lowercase().contains("friend");
        self.cue(if friendly {
            CueKind::Friendly
        } else {
            CueKind::AllPilots
        });
        self.push_log(LogKind::System, format!("Chat channel is {name}."));
    }

    fn follow_stick(&mut self, command: bool) {
        if command && same_command_stick(&self.config) {
            self.command_stick_down = false;
            return;
        }
        let bind = if command {
            self.config.command_joystick.clone()
        } else {
            self.config.joystick.clone()
        };
        let down = bind
            .as_ref()
            .is_some_and(|bind| self.sticks.is_held(&bind.guid, &bind.name, bind.button));
        let was = if command {
            self.command_stick_down
        } else {
            self.talk_stick_down
        };
        let press = if command {
            Press::CommandStick
        } else {
            Press::TalkStick
        };
        if down && !was {
            self.edge_down(press, command);
        } else if !down && was {
            self.edge_up(press);
        }
        if command {
            self.command_stick_down = down;
        } else {
            self.talk_stick_down = down;
        }
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                WorkerEvent::Transcript { epoch, result } => self.on_transcript(epoch, result),
                WorkerEvent::Injected {
                    epoch,
                    cue,
                    headline,
                    detail,
                    result,
                } => self.on_injected(epoch, cue, headline, detail, result),
                WorkerEvent::DownloadProgress { bytes, total } => {
                    if let Some(download) = &mut self.download {
                        download.bytes = bytes;
                        download.total = total;
                    }
                }
                WorkerEvent::DownloadFinished { result } => {
                    self.download = None;
                    match result {
                        Ok(path) => {
                            self.config.stt.model_path = path.display().to_string();
                            self.config.stt.engine = SttEngine::Local;
                            self.push_log(
                                LogKind::System,
                                format!("Speech model saved to {}.", path.display()),
                            );
                        }
                        Err(err) => self.push_log(LogKind::Problem, err),
                    }
                }
            }
        }
    }

    fn tick_phase(&mut self) {
        let now = self.now_ms();
        let hold = self.active_gesture == Gesture::Hold;
        let (phase, effect) = phase::on_tick(self.phase, now, self.config.long_press_ms, hold);
        self.phase = phase;
        self.apply(effect);
    }

    fn tick_limit(&mut self) {
        let Some(started) = self.capture_started else {
            return;
        };
        if !phase::is_capturing(self.phase) {
            return;
        }
        if started.elapsed() >= Duration::from_millis(self.config.max_record_ms) {
            self.push_log(
                LogKind::System,
                "Hit the recording limit. Sending what I heard.".into(),
            );
            self.force_send();
        }
    }

    fn apply(&mut self, effect: Effect) {
        match effect {
            Effect::None => {}
            Effect::BeginCapture => {
                self.utterance = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
                self.audio.start_capture();
                self.capture_started = Some(Instant::now());
                self.cue(CueKind::Listen);
            }
            Effect::DiscardCapture => {
                self.epoch.fetch_add(1, Ordering::SeqCst);
                self.utterance = 0;
                self.gesture_mode = None;
                self.audio.discard();
                self.capture_started = None;
                self.phase = Phase::Idle;
                self.cue(CueKind::Cancel);
                self.set_outcome("Cancelled", "Nothing was sent.");
                self.push_log(LogKind::Cancel, "Cancelled.".into());
            }
            Effect::EndCapture => self.begin_transcribe(),
            Effect::CancelInFlight => {
                self.epoch.fetch_add(1, Ordering::SeqCst);
                self.utterance = 0;
                self.gesture_mode = None;
                self.audio.discard();
                self.capture_started = None;
                self.phase = Phase::Idle;
                self.cue(CueKind::Cancel);
                self.set_outcome("Cancelled", "The transcription was dropped.");
                self.push_log(LogKind::Cancel, "Cancelled during transcription.".into());
            }
        }
    }

    fn begin_transcribe(&mut self) {
        let (samples, rate) = self.audio.stop_capture();
        self.capture_started = None;
        let lead = if self.config.cues { 90 } else { 0 };
        let samples = resample::trim_lead(samples, rate, lead);
        let min_samples = (rate as usize / 4).max(1);
        if samples.len() < min_samples {
            self.fail_listen("That clip was too short.");
            return;
        }
        let level = resample::rms(&samples);
        if level < self.config.silence_rms {
            self.fail_listen(&format!(
                "Too quiet (level {level:.3}). Move closer, or lower the silence gate in Setup."
            ));
            return;
        }
        let mut samples_16k = resample::resample_linear(&samples, rate, 16_000);
        let cap = 16_000 * 45;
        if samples_16k.len() > cap {
            samples_16k.truncate(cap);
        }
        self.phase = Phase::Transcribing;
        self.cue(CueKind::Working);
        let settings = stt::settings_from(&self.config);
        let epoch = self.utterance;
        if self
            .stt_tx
            .send(SttCommand::Transcribe {
                epoch,
                samples_16k,
                settings,
            })
            .is_err()
        {
            self.fail_listen("The speech worker has stopped.");
        }
    }

    fn on_transcript(&mut self, epoch: u64, result: Result<String, String>) {
        if epoch != self.utterance || epoch != self.epoch.load(Ordering::SeqCst) {
            return;
        }
        if self.phase != Phase::Transcribing {
            return;
        }
        match result {
            Ok(text) => self.dispatch_text(&text, None, true),
            Err(err) => self.fail_listen(&err),
        }
    }

    fn dispatch_text(&mut self, text: &str, mode_override: Option<Mode>, from_mic: bool) {
        if !from_mic {
            self.utterance = self.epoch.load(Ordering::SeqCst);
        }
        let mode =
            mode_override.unwrap_or_else(|| self.gesture_mode.take().unwrap_or(self.config.mode));
        let dispatch = plan::plan_action(&PlanInput {
            mode,
            text,
            chat: &self.config.chat,
            commands: &self.config.commands,
            token_floor: self.config.token_floor,
            command_max_extra: self.config.command_max_extra,
            mixed_max_extra: self.config.mixed_max_extra,
            command_gap_ms: self.config.command_gap_ms,
            command_lead_ms: self.config.command_lead_ms,
            practice: !self.config.send_keys,
        });
        let send = dispatch.ok && self.config.send_keys && !dispatch.steps.is_empty();
        if !send {
            self.phase = Phase::Idle;
            self.cue(dispatch.cue);
            self.set_outcome(&dispatch.headline, &dispatch.detail);
            let kind = if dispatch.ok {
                LogKind::Heard
            } else {
                LogKind::Problem
            };
            self.push_log(kind, format!("{} — {}", dispatch.headline, dispatch.detail));
            return;
        }
        let abort = if dispatch.cue == CueKind::ChatSent {
            self.config
                .chat
                .active()
                .and_then(|profile| keys::parse_sequence(&profile.abort).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        self.phase = Phase::Dispatching;
        self.set_outcome(&dispatch.headline, "Sending keys…");
        let request = InjectRequest {
            epoch: self.utterance,
            steps: dispatch.steps,
            abort,
            char_delay_ms: self.config.chat.char_delay_ms,
            focus_title: self.config.focus_title.clone(),
            require_focus: self.config.require_focus,
            headline: dispatch.headline,
            detail: dispatch.detail,
            cue: dispatch.cue,
        };
        if self.inject_tx.send(request).is_err() {
            self.phase = Phase::Idle;
            self.fail_listen("The key worker has stopped.");
        }
    }

    fn on_injected(
        &mut self,
        epoch: u64,
        cue: CueKind,
        headline: String,
        detail: String,
        result: Result<(), String>,
    ) {
        if epoch != self.utterance {
            return;
        }
        self.phase = Phase::Idle;
        match result {
            Ok(()) => {
                let detail = format!("Sent. {detail}");
                self.cue(cue);
                self.set_outcome(&headline, &detail);
                self.push_log(LogKind::Sent, format!("{headline} — {detail}"));
            }
            Err(err) => {
                self.cue(CueKind::Problem);
                self.set_outcome("Not sent", &err);
                self.push_log(LogKind::Problem, err);
            }
        }
    }

    fn fail_listen(&mut self, message: &str) {
        self.phase = Phase::Idle;
        self.gesture_mode = None;
        self.cue(CueKind::Problem);
        self.set_outcome("Didn't catch that", message);
        self.push_log(LogKind::Problem, message.to_string());
    }

    fn cue(&mut self, kind: CueKind) {
        if self.config.cues {
            self.cues.play(kind, self.config.cue_volume);
        }
    }

    fn set_outcome(&mut self, headline: &str, detail: &str) {
        self.last_headline = headline.to_string();
        self.last_detail = detail.to_string();
    }

    fn push_log(&mut self, kind: LogKind, message: String) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.events.push_back(LogEntry {
            time: time.clone(),
            kind,
            message: message.clone(),
        });
        while self.events.len() > 200 {
            self.events.pop_front();
        }
        let path = config::log_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{time}  {message}");
        }
    }

    fn note_edits(&mut self) {
        let snapshot = serde_json::to_string(&self.config).unwrap_or_default();
        if snapshot != self.edit_snapshot {
            self.edit_snapshot = snapshot;
            self.dirty = true;
            self.last_edit = Instant::now();
        }
        if self.dirty && self.last_edit.elapsed() > Duration::from_millis(400) {
            match self.config.save() {
                Ok(()) => {
                    self.dirty = false;
                    self.save_error = None;
                }
                Err(err) => {
                    self.save_error = Some(err);
                    self.last_edit = Instant::now();
                }
            }
        }
    }
}

impl eframe::App for CallsignApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::draw(self, ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
    }
}

impl CallsignApp {
    /// Talk uses the chat gesture, except when the header is already Commands.
    /// The commands button always uses the command gesture.
    fn capture_gesture(&self, command: bool) -> Gesture {
        if command || self.config.mode == Mode::Command {
            self.config.command_gesture
        } else {
            self.config.chat_gesture
        }
    }

    pub(crate) fn shown_gesture(&self) -> Gesture {
        if self.phase == Phase::Idle {
            self.capture_gesture(false)
        } else {
            self.active_gesture
        }
    }
}

fn same_command_stick(config: &Config) -> bool {
    same_bind(&config.joystick, &config.command_joystick)
}

fn same_bind(left: &Option<JoyBind>, right: &Option<JoyBind>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.button == right.button && (left.guid == right.guid || left.name == right.name)
        }
        _ => false,
    }
}

fn command_binding(text: &str) -> Option<Result<crate::keys::Chord, String>> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(keys::parse_ptt(text))
    }
}
