//! Joystick buttons by index, so a HOTAS hat or side button can be push-to-talk.
//! SDL is opened with a dummy video driver (set in main) so it doesn't open a window.

use std::collections::HashMap;

use sdl2::joystick::Joystick;
use sdl2::JoystickSubsystem;
use sdl2::Sdl;

pub struct Edge {
    pub guid: String,
    pub name: String,
    pub button: u32,
}

pub struct StickHub {
    inner: Option<Inner>,
    error: Option<String>,
}

struct Inner {
    open: Vec<OpenStick>,
    prev: HashMap<(usize, u32), bool>,
    last_count: u32,
    subsystem: JoystickSubsystem,
    _sdl: Sdl,
}

struct OpenStick {
    guid: String,
    name: String,
    joystick: Joystick,
}

impl StickHub {
    pub fn new() -> Self {
        match connect() {
            Ok(inner) => Self {
                inner: Some(inner),
                error: None,
            },
            Err(err) => Self {
                inner: None,
                error: Some(err),
            },
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn summary(&self) -> Vec<String> {
        let Some(inner) = &self.inner else {
            return Vec::new();
        };
        inner
            .open
            .iter()
            .map(|stick| format!("{} · {} buttons", stick.name, stick.joystick.num_buttons()))
            .collect()
    }

    pub fn is_held(&self, guid: &str, name: &str, button: u32) -> bool {
        let Some(inner) = &self.inner else {
            return false;
        };
        inner.open.iter().enumerate().any(|(index, stick)| {
            (stick.guid == guid || stick.name == name)
                && inner.prev.get(&(index, button)).copied().unwrap_or(false)
        })
    }

    pub fn poll(&mut self) -> Vec<Edge> {
        let Some(inner) = self.inner.as_mut() else {
            return Vec::new();
        };
        inner.subsystem.update();
        let count = inner.subsystem.num_joysticks().unwrap_or(0);
        if count != inner.last_count {
            inner.last_count = count;
            reopen(inner);
        }
        let mut edges = Vec::new();
        for (index, stick) in inner.open.iter().enumerate() {
            let buttons = stick.joystick.num_buttons();
            for button in 0..buttons {
                let down = match stick.joystick.button(button) {
                    Ok(down) => down,
                    Err(_) => inner.prev.get(&(index, button)).copied().unwrap_or(false),
                };
                let was = inner.prev.get(&(index, button)).copied().unwrap_or(false);
                if down && !was {
                    edges.push(Edge {
                        guid: stick.guid.clone(),
                        name: stick.name.clone(),
                        button,
                    });
                }
                inner.prev.insert((index, button), down);
            }
        }
        edges
    }
}

fn connect() -> Result<Inner, String> {
    let _ = sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
    let sdl = sdl2::init().map_err(|err| err.to_string())?;
    let subsystem = sdl.joystick().map_err(|err| err.to_string())?;
    let mut inner = Inner {
        open: Vec::new(),
        prev: HashMap::new(),
        last_count: 0,
        subsystem,
        _sdl: sdl,
    };
    let count = inner.subsystem.num_joysticks().unwrap_or(0);
    inner.last_count = count;
    reopen(&mut inner);
    Ok(inner)
}

fn reopen(inner: &mut Inner) {
    inner.open.clear();
    inner.prev.clear();
    let count = inner.subsystem.num_joysticks().unwrap_or(0);
    for index in 0..count {
        let Ok(joystick) = inner.subsystem.open(index) else {
            continue;
        };
        let name = joystick.name();
        let guid = joystick.guid().string();
        inner.open.push(OpenStick {
            guid,
            name,
            joystick,
        });
    }
}
