//! Decide what a transcript should do: type chat, press a binding, or stop.

use crate::config::{ChatSettings, Command, Mode};
use crate::cues::CueKind;
use crate::keys::{self, Step};
use crate::match_cmd::{self, Entry, MatchOpts};
use crate::sanitize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    pub headline: String,
    pub detail: String,
    pub steps: Vec<Step>,
    pub cue: CueKind,
    pub ok: bool,
}

pub struct PlanInput<'a> {
    pub mode: Mode,
    pub text: &'a str,
    pub chat: &'a ChatSettings,
    pub commands: &'a [Command],
    pub token_floor: f64,
    pub command_max_extra: usize,
    pub mixed_max_extra: usize,
    pub command_gap_ms: u64,
    pub command_lead_ms: u64,
    pub practice: bool,
}

pub fn plan_action(input: &PlanInput<'_>) -> Dispatch {
    match input.mode {
        Mode::Chat => plan_chat(input),
        Mode::Command => plan_command(input, true),
        Mode::Mixed => {
            let command = plan_command(input, false);
            if command.headline.starts_with("No command") {
                plan_chat(input)
            } else {
                command
            }
        }
    }
}

fn plan_chat(input: &PlanInput<'_>) -> Dispatch {
    let text = sanitize::sanitize_chat(input.text, input.chat.max_chars);
    if text.is_empty() {
        return fail("Didn't catch words", "Nothing in that line can be typed.");
    }
    let Some(profile) = input.chat.active() else {
        return fail("No chat profile", "Add an all-pilots or friendly profile.");
    };
    let steps = match chat_steps(profile, &text, input.chat.after_open_ms) {
        Ok(steps) => steps,
        Err(err) => return fail("Chat keys", &err),
    };
    let practice = if input.practice { "Practice. " } else { "" };
    Dispatch {
        headline: text,
        detail: format!(
            "{practice}{} · {}",
            profile.name,
            keys::describe_sequence(&steps)
        ),
        steps,
        cue: CueKind::ChatSent,
        ok: true,
    }
}

fn chat_steps(
    profile: &crate::config::ChatProfile,
    text: &str,
    after_open_ms: u64,
) -> Result<Vec<Step>, String> {
    let mut steps = keys::parse_sequence(&profile.open)?;
    if after_open_ms > 0 {
        steps.push(Step::Delay { ms: after_open_ms });
    }
    steps.push(Step::Text {
        text: text.to_string(),
    });
    steps.extend(keys::parse_sequence(&profile.send)?);
    Ok(steps)
}

fn plan_command(input: &PlanInput<'_>, suggest: bool) -> Dispatch {
    let max_extra = match input.mode {
        Mode::Mixed => input.mixed_max_extra,
        _ => input.command_max_extra,
    };
    let entries: Vec<Entry<'_>> = input
        .commands
        .iter()
        .map(|command| Entry {
            name: &command.name,
            phrases: &command.phrases,
            enabled: command.enabled,
        })
        .collect();
    let opts = MatchOpts {
        token_floor: input.token_floor,
        max_extra,
    };
    let Some(hit) = match_cmd::best_match(&entries, input.text, opts) else {
        if suggest {
            return suggest_or_fail(input, &entries);
        }
        return fail("No command", "No command matched.");
    };
    let command = &input.commands[hit.index];
    let steps = match keys::parse_sequence(&command.sequence) {
        Ok(steps) => steps,
        Err(err) => return fail(&command.name, &err),
    };
    let steps = pace_command(steps, input.command_lead_ms, input.command_gap_ms);
    if steps.is_empty() {
        return Dispatch {
            headline: format!("{} isn't bound", command.name),
            detail: format!("Heard \"{}\". Add the keys, then it can run.", hit.phrase),
            steps,
            cue: CueKind::Problem,
            ok: false,
        };
    }
    let practice = if input.practice { "Practice. " } else { "" };
    Dispatch {
        headline: command.name.clone(),
        detail: format!(
            "{practice}Press {} · heard \"{}\"",
            keys::describe_sequence(&steps),
            input.text.trim()
        ),
        steps,
        cue: CueKind::CommandSent,
        ok: true,
    }
}

/// A radio macro such as Tab, 0, 8 needs a beat between keys. An explicit
/// `delay:` in the sequence is left alone. The lead wait sits in front.
fn pace_command(steps: Vec<Step>, lead_ms: u64, gap_ms: u64) -> Vec<Step> {
    if steps.is_empty() {
        return steps;
    }
    let mut paced = Vec::with_capacity(steps.len() * 2 + 1);
    if lead_ms > 0 {
        paced.push(Step::Delay { ms: lead_ms });
    }
    for (index, step) in steps.iter().enumerate() {
        let chord = matches!(step, Step::Chord { .. });
        paced.push(step.clone());
        if gap_ms == 0 || !chord {
            continue;
        }
        if matches!(steps.get(index + 1), Some(Step::Chord { .. })) {
            paced.push(Step::Delay { ms: gap_ms });
        }
    }
    paced
}

fn suggest_or_fail(input: &PlanInput<'_>, entries: &[Entry<'_>]) -> Dispatch {
    let loose = MatchOpts {
        token_floor: input.token_floor,
        max_extra: input.command_max_extra.max(8),
    };
    if let Some(hit) = match_cmd::best_match(entries, input.text, loose) {
        if hit.score >= 0.92 {
            let name = entries[hit.index].name;
            return fail(
                "No command",
                &format!("No command matched. Closest is {name}."),
            );
        }
    }
    fail(
        "No command",
        &format!("No command matched \"{}\".", input.text.trim()),
    )
}

fn fail(headline: &str, detail: &str) -> Dispatch {
    Dispatch {
        headline: headline.to_string(),
        detail: detail.to_string(),
        steps: Vec::new(),
        cue: CueKind::Problem,
        ok: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChatSettings, Command, Config};

    fn input<'a>(mode: Mode, text: &'a str, config: &'a Config, practice: bool) -> PlanInput<'a> {
        PlanInput {
            mode,
            text,
            chat: config.chat(),
            commands: config.commands(),
            token_floor: config.token_floor,
            command_max_extra: config.command_max_extra,
            mixed_max_extra: config.mixed_max_extra,
            command_gap_ms: config.command_gap_ms,
            command_lead_ms: config.command_lead_ms,
            practice,
        }
    }

    #[test]
    fn chat_types_between_enter_presses() {
        let config = Config::default();
        let dispatch = plan_action(&input(Mode::Chat, " bandits high ", &config, true));
        assert!(dispatch.ok);
        assert_eq!(dispatch.headline, "bandits high");
        assert!(dispatch
            .steps
            .iter()
            .any(|step| matches!(step, Step::Text { text } if text == "bandits high")));
        assert!(dispatch.detail.starts_with("Practice."));
    }

    #[test]
    fn command_presses_gear_and_refuses_weapons() {
        let config = Config::default();
        let gear = plan_action(&input(Mode::Command, "gear up", &config, false));
        assert!(gear.ok);
        assert_eq!(gear.headline, "Landing gear");

        let bombs = plan_action(&input(Mode::Command, "drop bombs", &config, false));
        assert!(!bombs.ok);
    }

    #[test]
    fn mixed_mode_keeps_sentences_as_chat() {
        let config = Config::default();
        let sentence = plan_action(&input(
            Mode::Mixed,
            "please put the gear up now",
            &config,
            false,
        ));
        assert!(sentence.ok);
        assert_eq!(sentence.cue, CueKind::ChatSent);

        let command = plan_action(&input(Mode::Mixed, "flaps up", &config, false));
        assert_eq!(command.headline, "Flaps up");
    }

    #[test]
    fn command_macro_waits_between_keys_and_before_the_first() {
        let mut config = Config::default();
        config.command_lead_ms = 40;
        config.command_gap_ms = 80;
        config.commands_mut().push(Command {
            id: 9_000,
            name: "Request landing".into(),
            group: "Radio".into(),
            phrases: vec!["request landing".into()],
            sequence: "tab,0,8".into(),
            enabled: true,
        });
        let dispatch = plan_action(&input(Mode::Command, "request landing", &config, false));
        assert!(dispatch.ok);
        let delays: Vec<u64> = dispatch
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Delay { ms } => Some(*ms),
                _ => None,
            })
            .collect();
        assert_eq!(delays, vec![40, 80, 80]);

        config.commands_mut().last_mut().unwrap().sequence = "tab, delay:120, 0".into();
        let spaced = plan_action(&input(Mode::Command, "request landing", &config, false));
        let delays: Vec<u64> = spaced
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Delay { ms } => Some(*ms),
                _ => None,
            })
            .collect();
        assert_eq!(delays, vec![40, 120]);
    }

    #[test]
    fn friendly_profile_uses_right_control() {
        let mut config = Config::default();
        config.chat_mut().profile = 1;
        let dispatch = plan_action(&input(Mode::Chat, "on me", &config, false));
        assert!(dispatch.ok);
        let open = &dispatch.steps[0];
        match open {
            Step::Chord { modifiers, key } => {
                assert_eq!(modifiers, &vec![crate::keys::NamedKey::RCtrl]);
                assert_eq!(*key, crate::keys::NamedKey::Enter);
            }
            other => panic!("unexpected {other:?}"),
        }
        let _ = ChatSettings::il2();
    }
}
