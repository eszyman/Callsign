//! Callsign is a desktop companion for IL-2 Sturmovik.
//! Press a button, speak, then press again to send chat or a cockpit command.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod bus;
mod config;
mod cues;
mod focus;
mod hotkey;
mod icon_file;
mod inject;
mod keys;
mod match_cmd;
mod phase;
mod plan;
mod presets;
mod resample;
mod sanitize;
mod sticks;
mod stt;
mod theme;
mod ui;
mod wav;

fn main() -> eframe::Result<()> {
    // SDL is only here for joystick buttons. A real video driver would open a
    // second window and fight winit for the display connection.
    unsafe {
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
    }

    if std::env::args().nth(1).as_deref() == Some("--transcribe") {
        let file = std::env::args().nth(2);
        match file {
            Some(file) => match stt::transcribe_wav_file(std::path::Path::new(&file)) {
                Ok(text) => println!("{text}"),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            },
            None => {
                eprintln!("Usage: callsign --transcribe <file.wav>");
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    let icon = theme::app_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([920.0, 640.0])
            .with_title("Callsign")
            .with_icon(icon),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Callsign",
        options,
        Box::new(|cc| Ok(Box::new(app::CallsignApp::new(cc)))),
    )
}
