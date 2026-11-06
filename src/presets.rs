//! Starter bindings from the IL-2 Great Battles printed defaults.
//!
//! Korea uses a different keyboard. Import `commands/il2-korea.json` for that
//! map. Weapon releases and the numbered radio slots start disabled: a near
//! miss should not drop ordnance, and an unbound radio slot should not swallow
//! chat.

use crate::config::Command;

pub fn starter_commands() -> Vec<Command> {
    let mut id = 1u64;
    let mut next =
        |name: &str, group: &str, phrases: &[&str], sequence: &str, enabled: bool| -> Command {
            let command = Command {
                id,
                name: name.to_string(),
                group: group.to_string(),
                phrases: phrases.iter().map(|phrase| (*phrase).to_string()).collect(),
                sequence: sequence.to_string(),
                enabled,
            };
            id += 1;
            command
        };

    vec![
        next("Radio 1", "Radio", &["radio one", "channel one"], "", false),
        next("Radio 2", "Radio", &["radio two", "channel two"], "", false),
        next(
            "Radio 3",
            "Radio",
            &["radio three", "channel three"],
            "",
            false,
        ),
        next(
            "Radio 4",
            "Radio",
            &["radio four", "channel four"],
            "",
            false,
        ),
        next(
            "Command menu",
            "Radio",
            &["command menu", "radio menu"],
            "grave",
            true,
        ),
        next(
            "Next radio channel",
            "Radio",
            &["next channel", "radio channel", "switch channel"],
            "lalt+c",
            true,
        ),
        next(
            "Landing gear",
            "Flight",
            &["gear up", "gear down", "landing gear", "toggle gear"],
            "g",
            true,
        ),
        next(
            "Flaps down",
            "Flight",
            &["flaps down", "lower flaps"],
            "f",
            true,
        ),
        next(
            "Flaps up",
            "Flight",
            &["flaps up", "raise flaps"],
            "lshift+f",
            true,
        ),
        next(
            "Canopy",
            "Flight",
            &["canopy", "open canopy", "close canopy"],
            "ralt+c",
            true,
        ),
        next(
            "Engine",
            "Flight",
            &["toggle engine", "start engine", "cut engine", "stop engine"],
            "e",
            true,
        ),
        next(
            "Landing lights",
            "Flight",
            &["landing lights"],
            "rshift+l",
            true,
        ),
        next(
            "Navigation lights",
            "Flight",
            &["nav lights", "navigation lights"],
            "rctrl+l",
            true,
        ),
        next("Cockpit lights", "Flight", &["cockpit lights"], "l", true),
        next(
            "Air brakes",
            "Flight",
            &["air brakes", "speed brakes"],
            "ralt+b",
            true,
        ),
        next("Tail wheel", "Flight", &["tail wheel"], "lshift+g", true),
        next(
            "Parking brake",
            "Flight",
            &["parking brake"],
            "lshift+slash",
            true,
        ),
        next(
            "Reset trims",
            "Flight",
            &["reset trims", "center trims"],
            "lctrl+t",
            true,
        ),
        next("Autopilot", "Flight", &["autopilot"], "a", true),
        next(
            "Level autopilot",
            "Flight",
            &["level autopilot", "level flight"],
            "lshift+a",
            true,
        ),
        next(
            "Drop bombs",
            "Weapons",
            &["drop bombs", "release bombs"],
            "b",
            false,
        ),
        next(
            "Launch rockets",
            "Weapons",
            &["launch rockets", "fire rockets"],
            "r",
            false,
        ),
        next(
            "Bomb doors",
            "Weapons",
            &["bomb doors", "open bomb bay", "close bomb bay"],
            "n",
            false,
        ),
    ]
}

pub fn merge_starter(commands: &mut Vec<Command>, next_id: &mut u64) -> usize {
    let mut added = 0;
    for mut starter in starter_commands() {
        if commands.iter().any(|command| command.name == starter.name) {
            continue;
        }
        starter.id = *next_id;
        *next_id += 1;
        commands.push(starter);
        added += 1;
    }
    added
}
