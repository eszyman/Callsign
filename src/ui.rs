//! The desk. Operate is the thing you glance at in the sim. The other pages
//! are how you set the keys and the microphone before you get in.

use egui::{Color32, CornerRadius, Layout, RichText, Stroke, TextStyle, Vec2};

use crate::app::{CallsignApp, LogKind, Page};
use crate::config::{Mode, SttEngine};
use crate::cues::CueKind;
use crate::keys;
use crate::phase::{self, Phase};
use crate::stt;
use crate::theme;

pub fn draw(app: &mut CallsignApp, ui: &mut egui::Ui) {
    let wide = ui.available_width() >= 860.0;
    header(app, ui, wide);
    if wide {
        egui::Panel::left("nav")
            .exact_size(188.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(nav_frame())
            .show(ui, |ui| nav(app, ui, true));
    }
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .inner_margin(egui::Margin::symmetric(22, 18)),
        )
        .show(ui, |ui| match app.page {
            Page::Operate => operate(app, ui),
            Page::Chat => chat_page(app, ui),
            Page::Commands => commands_page(app, ui),
            Page::Setup => setup_page(app, ui),
        });
}

fn header(app: &mut CallsignApp, ui: &mut egui::Ui, wide: bool) {
    egui::Panel::top("header")
        .show_separator_line(false)
        .frame(nav_frame().inner_margin(egui::Margin::symmetric(20, 12)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("CALLSIGN")
                            .font(theme::display_font(26.0))
                            .extra_letter_spacing(1.8)
                            .color(theme::TEXT),
                    );
                    ui.label(RichText::new("IL-2 voice").size(12.5).color(theme::FAINT));
                });
                ui.add_space(18.0);
                segment(ui, &mut app.config.mode);
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let live = app.config.send_keys;
                    let (label, fill, ink) = if live {
                        (
                            "Live keys",
                            theme::AMBER,
                            Color32::from_rgb(0x1a, 0x14, 0x04),
                        )
                    } else {
                        ("Practice", theme::SURFACE_2, theme::MUTED)
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).color(ink).size(13.0))
                                .fill(fill)
                                .corner_radius(20)
                                .min_size(Vec2::new(96.0, 30.0)),
                        )
                        .on_hover_text("Opens Setup. This does not switch key output by itself.")
                        .clicked()
                    {
                        app.page = Page::Setup;
                    }
                    ui.add_space(10.0);
                    lamp(ui, lamp_color(app.phase), 6.0);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(phase::title(app.phase))
                            .color(theme::TEXT)
                            .size(14.0),
                    );
                });
            });
            if !wide {
                ui.add_space(8.0);
                ui.horizontal(|ui| nav(app, ui, false));
            }
        });
}

fn nav(app: &mut CallsignApp, ui: &mut egui::Ui, stacked: bool) {
    let pages = [
        (Page::Operate, "Desk"),
        (Page::Chat, "Chat"),
        (Page::Commands, "Commands"),
        (Page::Setup, "Setup"),
    ];
    if stacked {
        ui.add_space(8.0);
        for (page, label) in pages {
            nav_button(app, ui, page, label, true);
            ui.add_space(4.0);
        }
        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Runs beside the sim, on the PC. Not an overlay inside the headset.")
                    .size(12.0)
                    .color(theme::FAINT),
            );
        });
    } else {
        for (page, label) in pages {
            nav_button(app, ui, page, label, false);
        }
    }
}

fn nav_button(app: &mut CallsignApp, ui: &mut egui::Ui, page: Page, label: &str, wide: bool) {
    let selected = app.page == page;
    let fill = if selected {
        theme::AMBER_DIM
    } else {
        Color32::TRANSPARENT
    };
    let ink = if selected { theme::AMBER } else { theme::MUTED };
    let width = if wide { ui.available_width() } else { 0.0 };
    let button = egui::Button::new(RichText::new(label).color(ink).size(14.5))
        .fill(fill)
        .stroke(Stroke::NONE)
        .corner_radius(8)
        .min_size(Vec2::new(width, 34.0));
    if ui.add(button).clicked() {
        app.page = page;
    }
}

fn operate(app: &mut CallsignApp, ui: &mut egui::Ui) {
    let wide = ui.available_width() > 760.0;
    if wide {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                status_column(app, ui);
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                outcome_column(app, ui);
            });
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            status_column(app, ui);
            ui.add_space(14.0);
            outcome_column(app, ui);
        });
    }
}

fn status_column(app: &mut CallsignApp, ui: &mut egui::Ui) {
    theme::card().show(ui, |ui| {
        ui.horizontal(|ui| {
            lamp(ui, lamp_color(app.phase), 9.0);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(phase::title(app.phase))
                        .font(theme::display_font(28.0))
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new(phase::hint(app.phase))
                        .color(theme::MUTED)
                        .size(13.5),
                );
            });
        });
        ui.add_space(14.0);
        meter(ui, app.audio.rms());
        ui.add_space(4.0);
        let device = app.audio.device_name().to_string();
        ui.label(
            RichText::new(device)
                .size(12.0)
                .color(theme::FAINT)
                .monospace(),
        );
        if let Some(err) = app.audio.error() {
            ui.label(RichText::new(err).size(12.5).color(theme::RED));
        }
        ui.add_space(14.0);
        let (label, fill, ink, enabled) = talk_style(app.phase);
        let width = ui.available_width();
        let button = egui::Button::new(
            RichText::new(label)
                .color(ink)
                .size(18.0)
                .font(theme::display_font(22.0))
                .extra_letter_spacing(0.8),
        )
        .fill(fill)
        .corner_radius(12)
        .min_size(Vec2::new(width, 56.0));
        let response = ui.add_enabled(enabled, button);
        if response.clicked() {
            app.on_ptt();
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let capturing = phase::is_capturing(app.phase);
            if ui
                .add_enabled(capturing, egui::Button::new("Send now"))
                .clicked()
            {
                app.force_send();
            }
            let can_cancel = !matches!(app.phase, Phase::Idle | Phase::Dispatching);
            if ui
                .add_enabled(can_cancel, egui::Button::new("Cancel"))
                .clicked()
            {
                app.cancel_gesture();
            }
        });
        ui.add_space(16.0);
        gesture_strip(ui);
    });
    ui.add_space(12.0);
    theme::card().show(ui, |ui| {
        let binding = match keys::parse_ptt(&app.config.ptt) {
            Ok(chord) => keys::describe_chord(&chord.modifiers, chord.key),
            Err(_) => app.config.ptt.clone(),
        };
        ui.label(
            RichText::new("TALK BUTTON")
                .size(11.0)
                .color(theme::FAINT)
                .extra_letter_spacing(1.1),
        );
        ui.label(RichText::new(binding).color(theme::TEXT).size(16.0));
        if let Some(joy) = &app.config.joystick {
            ui.label(
                RichText::new(format!("or {} button {}", joy.name, joy.button + 1))
                    .color(theme::MUTED)
                    .size(13.0),
            );
        }
        if let Some(err) = app.hotkeys.error() {
            ui.label(RichText::new(err).size(12.5).color(theme::RED));
        }
        ui.add_space(10.0);
        if app.config.send_keys {
            let title = if app.config.focus_title.trim().is_empty() {
                "the focused window".to_string()
            } else {
                format!("a window titled {}", app.config.focus_title)
            };
            ui.label(
                RichText::new(format!("Live. Keys go to {title}."))
                    .color(theme::AMBER)
                    .size(13.0),
            );
        } else {
            ui.label(
                RichText::new("Practice mode. Nothing is typed into IL-2.")
                    .color(theme::GREEN)
                    .size(13.0),
            );
        }
    });
}

fn outcome_column(app: &mut CallsignApp, ui: &mut egui::Ui) {
    ui.label(
        RichText::new(&app.last_headline)
            .font(theme::display_font(34.0))
            .color(theme::TEXT),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(&app.last_detail)
            .color(theme::MUTED)
            .size(15.0),
    );
    ui.add_space(16.0);
    if app.config.stt.engine == SttEngine::Local
        && !stt::model_is_ready(&app.config.resolved_model_path())
    {
        theme::card().show(ui, |ui| {
            ui.label(RichText::new("No local speech model yet").color(theme::TEXT).size(15.0));
            ui.label(
                RichText::new("Typed lines below still run. Speaking needs the small English model, about 75 MB.")
                    .color(theme::MUTED)
                    .size(13.0),
            );
            ui.add_space(8.0);
            download_row(app, ui);
        });
        ui.add_space(12.0);
    }
    ui.label(
        RichText::new("Try a line without speaking")
            .size(12.5)
            .color(theme::FAINT),
    );
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 78.0).max(120.0);
        let edit = egui::TextEdit::singleline(&mut app.try_line)
            .hint_text("bandits high, two o'clock")
            .desired_width(width)
            .font(TextStyle::Body);
        let response = ui.add(edit);
        let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let idle = app.phase == Phase::Idle;
        if (ui.add_enabled(idle, egui::Button::new("Run")).clicked() || enter) && idle {
            app.run_try_line();
        }
    });
    ui.add_space(14.0);
    ui.label(
        RichText::new("LOG")
            .size(11.0)
            .color(theme::FAINT)
            .extra_letter_spacing(1.1),
    );
    ui.add_space(4.0);
    let height = ui.available_height().max(160.0);
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::LINE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(12)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(height - 28.0)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if app.events.is_empty() {
                        ui.label(RichText::new("The log is empty.").color(theme::FAINT));
                    }
                    for entry in &app.events {
                        let color = match entry.kind {
                            LogKind::System => theme::MUTED,
                            LogKind::Sent => theme::GREEN,
                            LogKind::Cancel => theme::AMBER,
                            LogKind::Problem => theme::RED,
                            LogKind::Heard => theme::TEXT,
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(&entry.time)
                                    .monospace()
                                    .size(12.0)
                                    .color(theme::FAINT),
                            );
                            ui.label(RichText::new(&entry.message).size(13.0).color(color));
                        });
                    }
                });
        });
}

fn chat_page(app: &mut CallsignApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Chat channels");
        ui.label(
            RichText::new(
                "All pilots opens with Enter. Friendly chat opens with Right Ctrl+Enter. Type the sides as lctrl and rctrl — a single ctrl would hit the wrong key, and the recorder cannot tell left from right on its own.",
            )
            .color(theme::MUTED),
        );
        ui.add_space(12.0);
        let count = app.config.chat.profiles.len();
        for index in 0..count {
            let selected = app.config.chat.profile == index;
            let frame = theme::card().stroke(Stroke::new(
                if selected { 1.5 } else { 1.0 },
                if selected { theme::AMBER } else { theme::LINE },
            ));
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let name = app.config.chat.profiles[index].name.clone();
                    if ui
                        .selectable_label(selected, RichText::new(format!("Use {name}")).color(theme::AMBER))
                        .clicked()
                    {
                        app.config.chat.profile = index;
                    }
                });
                ui.add_space(8.0);
                labeled(ui, "Name", &mut app.config.chat.profiles[index].name);
                labeled(ui, "Open", &mut app.config.chat.profiles[index].open);
                labeled(ui, "Send", &mut app.config.chat.profiles[index].send);
                labeled(ui, "Abort", &mut app.config.chat.profiles[index].abort);
                sequence_preview(ui, &app.config.chat.profiles[index].open);
            });
            ui.add_space(10.0);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut app.config.chat.char_delay_ms, 0u64..=40).text("ms between letters"));
            ui.add(egui::Slider::new(&mut app.config.chat.after_open_ms, 40u64..=400).text("ms after opening chat"));
        });
        ui.add(egui::Slider::new(&mut app.config.chat.max_chars, 40usize..=240).text("character cap"));
        ui.add_space(8.0);
        let idle = app.phase == Phase::Idle;
        if ui
            .add_enabled(idle, egui::Button::new("Send a radio check"))
            .on_hover_text("Runs the words \"radio check\" through the active chat channel.")
            .clicked()
        {
            app.radio_check();
        }
    });
}

fn commands_page(app: &mut CallsignApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Voice to keys");
        ui.label(
            RichText::new(
                "A phrase matches when its words show up in order. Short words have to be exact. These keys are the printed IL-2 Great Battles defaults — open Controls in the sim and check them before you turn key output on. Weapons start off: a misheard word should not drop ordnance. Eject and the gun trigger are not in this list.",
            )
            .color(theme::MUTED),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 92.0).max(140.0);
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.command_test)
                    .hint_text("gear up")
                    .desired_width(width),
            );
            let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let idle = app.phase == Phase::Idle;
            if (ui.add_enabled(idle, egui::Button::new("Test")).clicked() || enter) && idle {
                app.run_command_test();
            }
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Add command").clicked() {
                app.add_command();
            }
            if ui.button("Add starter set").clicked() {
                app.add_starter();
            }
            if ui.button("Export").clicked() {
                app.export_commands();
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.import_path)
                    .hint_text("Path to a command file")
                    .desired_width((ui.available_width() - 88.0).max(160.0)),
            );
            if ui.button("Import").clicked() {
                app.import_commands();
            }
        });
        ui.add_space(14.0);
        let mut groups: Vec<String> = Vec::new();
        for command in &app.config.commands {
            if !groups.iter().any(|group| group == &command.group) {
                groups.push(command.group.clone());
            }
        }
        groups.sort_by(|a, b| group_rank(a).cmp(&group_rank(b)).then(a.cmp(b)));
        let mut remove: Option<u64> = None;
        for group in groups {
            ui.add_space(6.0);
            ui.label(
                RichText::new(group.to_uppercase())
                    .font(theme::display_font(16.0))
                    .extra_letter_spacing(1.2)
                    .color(theme::FAINT),
            );
            ui.add_space(4.0);
            let indexes: Vec<usize> = app
                .config
                .commands
                .iter()
                .enumerate()
                .filter(|(_, command)| command.group == group)
                .map(|(index, _)| index)
                .collect();
            for index in indexes {
                let id = app.config.commands[index].id;
                ui.push_id(id, |ui| {
                    theme::card().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut app.config.commands[index].enabled, "Enabled");
                            if ui.button("Remove").clicked() {
                                remove = Some(id);
                            }
                        });
                        labeled(ui, "Name", &mut app.config.commands[index].name);
                        labeled(ui, "Group", &mut app.config.commands[index].group);
                        ui.label(RichText::new("Phrases, one per line").size(12.0).color(theme::FAINT));
                        let mut blob = app.config.commands[index].phrases.join("\n");
                        let changed = ui
                            .add(
                                egui::TextEdit::multiline(&mut blob)
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed();
                        if changed {
                            let phrases: Vec<String> = if blob.is_empty() {
                                vec![String::new()]
                            } else {
                                blob.split('\n').map(|line| line.to_string()).collect()
                            };
                            app.config.commands[index].phrases = phrases;
                        }
                        labeled_mono(ui, "Keys", &mut app.config.commands[index].sequence);
                        sequence_preview(ui, &app.config.commands[index].sequence);
                        if app.config.commands[index].sequence.trim().is_empty() {
                            ui.label(
                                RichText::new("Unbound. A match is reported and nothing is typed.")
                                    .size(12.5)
                                    .color(theme::AMBER),
                            );
                        }
                    });
                });
                ui.add_space(8.0);
            }
        }
        if let Some(id) = remove {
            app.config.commands.retain(|command| command.id != id);
        }
    });
}

fn setup_page(app: &mut CallsignApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Talk button");
        ui.label(
            RichText::new(
                "Scroll Lock is the default because IL-2 rarely wants it. The key is claimed globally, and released while you are typing in this window. On a HOTAS, pick a button that is not also bound in the sim. Linux may need your user in the input group before SDL can see the stick.",
            )
            .color(theme::MUTED),
        );
        ui.add_space(8.0);
        labeled(ui, "Keyboard", &mut app.config.ptt);
        match keys::parse_ptt(&app.config.ptt) {
            Ok(chord) => {
                ui.label(
                    RichText::new(keys::describe_chord(&chord.modifiers, chord.key))
                        .color(theme::GREEN)
                        .size(13.0),
                );
            }
            Err(err) => {
                ui.label(RichText::new(err).color(theme::RED).size(13.0));
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let listen = if app.listen_joy {
                "Listening for a button…"
            } else {
                "Listen for a stick button"
            };
            if ui.button(listen).clicked() {
                app.listen_joy = !app.listen_joy;
            }
            if ui.button("Clear stick").clicked() {
                app.config.joystick = None;
                app.listen_joy = false;
            }
        });
        if let Some(joy) = &app.config.joystick {
            ui.label(
                RichText::new(format!("{} · button {}", joy.name, joy.button + 1)).color(theme::TEXT),
            );
        }
        if let Some(err) = app.sticks.error() {
            ui.label(RichText::new(err).color(theme::RED).size(13.0));
        } else {
            let devices = app.sticks.summary();
            if devices.is_empty() {
                ui.label(RichText::new("No joystick or HOTAS detected.").color(theme::FAINT).size(13.0));
            } else {
                for device in devices {
                    ui.label(RichText::new(device).color(theme::MUTED).size(13.0));
                }
            }
        }
        ui.add_space(18.0);
        section(ui, "Microphone");
        ui.label(
            RichText::new(
                "The mic stays open so the level meter can move. Samples are kept only while a talk gesture is running.",
            )
            .color(theme::MUTED),
        );
        ui.add_space(8.0);
        let names = app.mic_names.clone();
        let selected = app
            .config
            .audio_device
            .clone()
            .unwrap_or_else(|| "System default".into());
        egui::ComboBox::from_id_salt("mic")
            .selected_text(selected)
            .width(320.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.config.audio_device.is_none(), "System default")
                    .clicked()
                {
                    app.config.audio_device = None;
                }
                for name in names {
                    let on = app.config.audio_device.as_deref() == Some(name.as_str());
                    if ui.selectable_label(on, &name).clicked() {
                        app.config.audio_device = Some(name);
                    }
                }
            });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Reopen microphone").clicked() {
                app.reopen_mic();
            }
            if ui.button("Refresh list").clicked() {
                app.refresh_mics();
            }
        });
        meter(ui, app.audio.rms());
        ui.label(
            RichText::new(format!(
                "{} · {} Hz",
                app.audio.device_name(),
                app.audio.sample_rate()
            ))
            .size(12.0)
            .color(theme::FAINT)
            .monospace(),
        );
        if let Some(err) = app.audio.error() {
            ui.label(RichText::new(err).color(theme::RED).size(13.0));
        }
        ui.add_space(18.0);
        section(ui, "Cues");
        ui.checkbox(&mut app.config.cues, "Play tones on the default output");
        ui.add(
            egui::Slider::new(&mut app.config.cue_volume, 0.0..=0.40)
                .fixed_decimals(2)
                .text("volume"),
        );
        if ui.button("Play the listen tone").clicked() {
            app.cues.play(CueKind::Listen, app.config.cue_volume.max(0.08));
        }
        if let Some(err) = app.cues.error() {
            ui.label(RichText::new(err).color(theme::RED).size(13.0));
        }
        ui.label(
            RichText::new("SteamVR usually mirrors the desktop output into the headset.")
                .size(12.5)
                .color(theme::FAINT),
        );
        ui.add_space(18.0);
        section(ui, "Speech");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.config.stt.engine, SttEngine::Local, "On this machine");
            ui.selectable_value(&mut app.config.stt.engine, SttEngine::Http, "Speech server");
        });
        #[cfg(not(feature = "whisper"))]
        ui.label(
            RichText::new("This build has no local model compiled in. Use a speech server.")
                .color(theme::AMBER),
        );
        ui.add_space(6.0);
        labeled(ui, "Model file", &mut app.config.stt.model_path);
        let path = app.config.resolved_model_path();
        ui.label(
            RichText::new(format!("{} · {}", path.display(), model_size(&path)))
                .size(12.0)
                .color(theme::FAINT)
                .monospace(),
        );
        ui.label(
            RichText::new("Leave the path empty to use the default file under the data folder. Language en, de, ru, or auto. The English model only understands English.")
                .size(12.5)
                .color(theme::MUTED),
        );
        labeled(ui, "Language", &mut app.config.stt.language);
        ui.add_space(6.0);
        download_row(app, ui);
        ui.add_space(10.0);
        labeled(ui, "Server", &mut app.config.stt.api_base);
        ui.label(RichText::new("API key").size(12.0).color(theme::FAINT));
        ui.add(
            egui::TextEdit::singleline(&mut app.config.stt.api_key)
                .password(true)
                .desired_width(f32::INFINITY),
        );
        labeled(ui, "Server model", &mut app.config.stt.api_model);
        ui.add_space(18.0);
        section(ui, "Injection");
        ui.checkbox(&mut app.config.send_keys, "Send keys to IL-2");
        ui.label(
            RichText::new(
                "Off until you turn it on. Callsign tries to focus a window whose title contains the text below, then checks it still has focus before typing.",
            )
            .color(theme::MUTED)
            .size(13.0),
        );
        labeled(ui, "Window title", &mut app.config.focus_title);
        ui.checkbox(
            &mut app.config.require_focus,
            "Refuse to type if that window is not focused",
        );
        ui.add_space(8.0);
        egui::CollapsingHeader::new("Timing and matching")
            .default_open(false)
            .show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut app.config.double_tap_ms, 140u64..=600)
                        .text("double-tap window, ms"),
                );
                let mut seconds = app.config.max_record_ms as f32 / 1000.0;
                ui.add(egui::Slider::new(&mut seconds, 5.0..=30.0).suffix(" s").text("max recording"));
                app.config.max_record_ms = (seconds * 1000.0).round() as u64;
                ui.add(
                    egui::Slider::new(&mut app.config.silence_rms, 0.002..=0.08)
                        .fixed_decimals(3)
                        .text("silence gate"),
                );
                ui.add(
                    egui::Slider::new(&mut app.config.token_floor, 0.70..=0.98)
                        .fixed_decimals(2)
                        .text("word match floor"),
                );
                ui.add(
                    egui::Slider::new(&mut app.config.command_max_extra, 0usize..=8)
                        .text("extra words, command mode"),
                );
                ui.add(
                    egui::Slider::new(&mut app.config.mixed_max_extra, 0usize..=4)
                        .text("extra words, mixed mode"),
                );
            });
        ui.add_space(16.0);
        if let Some(err) = &app.save_error {
            ui.label(RichText::new(err).color(theme::RED));
        } else {
            ui.label(
                RichText::new(format!("Settings save to {}", crate::config::config_path().display()))
                    .size(12.0)
                    .color(theme::FAINT)
                    .monospace(),
            );
        }
    });
}

fn download_row(app: &mut CallsignApp, ui: &mut egui::Ui) {
    if let Some(download) = &app.download {
        let fraction = download
            .total
            .filter(|total| *total > 0)
            .map(|total| download.bytes as f32 / total as f32)
            .unwrap_or(0.08);
        ui.add(
            egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).text(format!(
                "{} · {}",
                download.label,
                format_bytes(download.bytes)
            )),
        );
        return;
    }
    let busy = false;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new("Download English model"))
            .clicked()
        {
            app.start_download("English model", stt::MODEL_EN_URL, "ggml-tiny.en.bin");
        }
        if ui
            .add_enabled(!busy, egui::Button::new("Download multilingual model"))
            .clicked()
        {
            app.start_download("Multilingual model", stt::MODEL_MULTI_URL, "ggml-tiny.bin");
        }
    });
}

fn gesture_strip(ui: &mut egui::Ui) {
    let steps = [
        ("1", "Press and speak"),
        ("2", "Press to send"),
        ("3", "Press twice to cancel"),
    ];
    ui.horizontal(|ui| {
        for (number, label) in steps {
            ui.label(
                RichText::new(number)
                    .font(theme::display_font(18.0))
                    .color(theme::AMBER),
            );
            ui.label(RichText::new(label).size(12.5).color(theme::MUTED));
            ui.add_space(8.0);
        }
    });
    ui.label(
        RichText::new(
            "A short tick means the next press will send. A second tap inside the window cancels.",
        )
        .size(12.0)
        .color(theme::FAINT),
    );
}

fn sequence_preview(ui: &mut egui::Ui, sequence: &str) {
    if sequence.trim().is_empty() {
        return;
    }
    match keys::parse_sequence(sequence) {
        Ok(steps) => {
            ui.label(
                RichText::new(keys::describe_sequence(&steps))
                    .size(12.5)
                    .color(theme::GREEN),
            );
        }
        Err(err) => {
            ui.label(RichText::new(err).size(12.5).color(theme::RED));
        }
    }
}

fn labeled(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.add_space(4.0);
    ui.label(RichText::new(label).size(12.0).color(theme::FAINT));
    ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
}

fn labeled_mono(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.add_space(4.0);
    ui.label(RichText::new(label).size(12.0).color(theme::FAINT));
    ui.add(
        egui::TextEdit::singleline(text)
            .desired_width(f32::INFINITY)
            .font(TextStyle::Monospace),
    );
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .font(theme::display_font(26.0))
            .color(theme::TEXT),
    );
    ui.add_space(2.0);
}

fn segment(ui: &mut egui::Ui, mode: &mut Mode) {
    ui.horizontal(|ui| {
        for choice in [Mode::Chat, Mode::Mixed, Mode::Command] {
            let on = *mode == choice;
            let fill = if on { theme::AMBER_DIM } else { theme::SURFACE };
            let ink = if on { theme::AMBER } else { theme::MUTED };
            if ui
                .add(
                    egui::Button::new(RichText::new(choice.label()).color(ink).size(13.5))
                        .fill(fill)
                        .stroke(Stroke::new(
                            1.0,
                            if on { theme::AMBER } else { theme::LINE },
                        ))
                        .corner_radius(8)
                        .min_size(Vec2::new(78.0, 30.0)),
                )
                .clicked()
            {
                *mode = choice;
            }
        }
    });
}

fn meter(ui: &mut egui::Ui, rms: f32) {
    let gained = (rms * 8.0).clamp(0.0, 1.0);
    let level = gained.powf(0.65);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::SURFACE_2);
    if level > 0.01 {
        let mut fill = rect;
        fill.set_width((rect.width() * level).max(4.0));
        let color = if level > 0.92 {
            theme::RED
        } else {
            theme::GREEN
        };
        painter.rect_filled(fill, CornerRadius::same(4), color);
    }
}

fn lamp(ui: &mut egui::Ui, color: Color32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 4.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

fn lamp_color(phase: Phase) -> Color32 {
    match phase {
        Phase::Idle => theme::FAINT,
        Phase::StartWindow { .. } | Phase::Listening | Phase::Arming { .. } => theme::AMBER,
        Phase::Transcribing | Phase::Dispatching => theme::BLUE,
    }
}

fn talk_style(phase: Phase) -> (&'static str, Color32, Color32, bool) {
    let ink = Color32::from_rgb(0x1a, 0x14, 0x04);
    match phase {
        Phase::Idle => ("Talk", theme::AMBER, ink, true),
        Phase::StartWindow { .. } | Phase::Listening => (
            "Send",
            theme::GREEN,
            Color32::from_rgb(0x14, 0x18, 0x0c),
            true,
        ),
        Phase::Arming { .. } => (
            "Cancel",
            theme::RED,
            Color32::from_rgb(0x1c, 0x0e, 0x0a),
            true,
        ),
        Phase::Transcribing => ("Working", theme::SURFACE_2, theme::TEXT, true),
        Phase::Dispatching => ("Sending", theme::SURFACE_2, theme::MUTED, false),
    }
}

fn nav_frame() -> egui::Frame {
    egui::Frame::new().fill(theme::NAV).inner_margin(12)
}

fn group_rank(name: &str) -> u8 {
    match name.to_ascii_lowercase().as_str() {
        "radio" => 0,
        "flight" => 1,
        "weapons" => 2,
        _ => 3,
    }
}

fn model_size(path: &std::path::Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() > 1_000_000 => format!("{:.0} MB", meta.len() as f64 / 1_048_576.0),
        Ok(_) => "too small to use".into(),
        Err(_) => "not downloaded".into(),
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
