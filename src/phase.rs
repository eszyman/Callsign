//! Push-to-talk gesture.
//!
//! The first press starts listening. A short second press sends. Holding that
//! second press cancels the line before it is transcribed, which is the way
//! out of a stumble.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Capturing. `opener_down` is the first press, which does not send.
    Listening {
        opener_down: bool,
    },
    /// The second press is down. A quick release sends; holding cancels.
    Holding {
        since_ms: u64,
    },
    Transcribing,
    Dispatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    BeginCapture,
    DiscardCapture,
    EndCapture,
    CancelInFlight,
}

pub fn on_down(phase: Phase, now_ms: u64) -> (Phase, Effect) {
    match phase {
        Phase::Idle => (Phase::Listening { opener_down: true }, Effect::BeginCapture),
        Phase::Listening { opener_down: true } => (phase, Effect::None),
        Phase::Listening { opener_down: false } => {
            (Phase::Holding { since_ms: now_ms }, Effect::None)
        }
        Phase::Holding { .. } => (phase, Effect::None),
        Phase::Transcribing => (Phase::Idle, Effect::CancelInFlight),
        Phase::Dispatching => (phase, Effect::None),
    }
}

pub fn on_up(phase: Phase, now_ms: u64, long_ms: u64, hold: bool) -> (Phase, Effect) {
    if hold {
        return match phase {
            Phase::Listening { .. } | Phase::Holding { .. } => {
                (Phase::Transcribing, Effect::EndCapture)
            }
            _ => (phase, Effect::None),
        };
    }
    let long_ms = long_ms.max(1);
    match phase {
        Phase::Listening { opener_down: true } => {
            (Phase::Listening { opener_down: false }, Effect::None)
        }
        Phase::Holding { since_ms } => {
            if now_ms.saturating_sub(since_ms) >= long_ms {
                (Phase::Idle, Effect::DiscardCapture)
            } else {
                (Phase::Transcribing, Effect::EndCapture)
            }
        }
        _ => (phase, Effect::None),
    }
}

pub fn on_tick(phase: Phase, now_ms: u64, long_ms: u64, hold: bool) -> (Phase, Effect) {
    if hold {
        return (phase, Effect::None);
    }
    let long_ms = long_ms.max(1);
    match phase {
        Phase::Holding { since_ms } if now_ms.saturating_sub(since_ms) >= long_ms => {
            (Phase::Idle, Effect::DiscardCapture)
        }
        _ => (phase, Effect::None),
    }
}

/// A hold-to-talk release has to stay up this long before the recording ends.
/// Contact bounce and a one-frame drop in a stick reading are shorter than this.
pub const HOLD_SETTLE_MS: u64 = 120;

/// `true` when hold-to-talk should end the recording.
/// The control dipping and coming back clears the timer and keeps the line open.
pub fn hold_should_send(up_since: &mut Option<u64>, now_ms: u64, still_down: bool) -> bool {
    if still_down {
        *up_since = None;
        return false;
    }
    let since = *up_since.get_or_insert(now_ms);
    now_ms.saturating_sub(since) >= HOLD_SETTLE_MS
}

/// Whether a keyboard hold is still down.
///
/// `physical` is a direct poll of the key. `None` means this machine can't
/// poll it, so a release event is trusted after a short grace. That grace
/// covers the Windows hotkey watcher, which can report a release before the
/// key has ever looked down.
pub fn key_held(
    physical: Option<bool>,
    seen_down: &mut bool,
    release_seen: bool,
    elapsed_ms: u64,
) -> bool {
    match physical {
        Some(true) => {
            *seen_down = true;
            true
        }
        Some(false) if *seen_down => false,
        _ => !release_seen || elapsed_ms < 250,
    }
}

pub fn is_capturing(phase: Phase) -> bool {
    matches!(phase, Phase::Listening { .. } | Phase::Holding { .. })
}

pub fn title(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Ready",
        Phase::Listening { .. } => "Listening",
        Phase::Holding { .. } => "Holding",
        Phase::Transcribing => "Transcribing",
        Phase::Dispatching => "Sending",
    }
}

pub fn hint(phase: Phase, hold: bool) -> &'static str {
    if hold {
        return match phase {
            Phase::Idle => "Hold to speak. Release to send.",
            Phase::Listening { .. } | Phase::Holding { .. } => "Release to send.",
            Phase::Transcribing => "Working through the recording.",
            Phase::Dispatching => "Sending keys to the sim.",
        };
    }
    match phase {
        Phase::Idle => "Press once and speak.",
        Phase::Listening { opener_down: true } => "Release, then press again to send.",
        Phase::Listening { opener_down: false } => "Press to send. Hold that press to cancel.",
        Phase::Holding { .. } => "Release to send. Keep holding to cancel.",
        Phase::Transcribing => "Working through the recording.",
        Phase::Dispatching => "Sending keys to the sim.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: u64 = 700;
    const TAP: bool = false;

    #[test]
    fn first_press_starts_capture_and_its_release_keeps_listening() {
        let (phase, effect) = on_down(Phase::Idle, 1_000);
        assert_eq!(phase, Phase::Listening { opener_down: true });
        assert_eq!(effect, Effect::BeginCapture);
        let (phase, effect) = on_up(phase, 1_080, LONG, TAP);
        assert_eq!(phase, Phase::Listening { opener_down: false });
        assert_eq!(effect, Effect::None);
        assert!(is_capturing(phase));
    }

    #[test]
    fn short_second_press_sends() {
        let (phase, _) = on_down(Phase::Idle, 1_000);
        let (phase, _) = on_up(phase, 1_080, LONG, TAP);
        let (phase, effect) = on_down(phase, 2_000);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_tick(phase, 2_400, LONG, TAP);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_up(phase, 2_400, LONG, TAP);
        assert_eq!(phase, Phase::Transcribing);
        assert_eq!(effect, Effect::EndCapture);
    }

    #[test]
    fn holding_the_second_press_cancels_before_release() {
        let (phase, _) = on_down(Phase::Idle, 1_000);
        let (phase, _) = on_up(phase, 1_080, LONG, TAP);
        let (phase, _) = on_down(phase, 2_000);
        let (phase, effect) = on_tick(phase, 2_699, LONG, TAP);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_tick(phase, 2_700, LONG, TAP);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::DiscardCapture);
        let (phase, effect) = on_up(phase, 2_900, LONG, TAP);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::None);
    }

    #[test]
    fn release_after_the_hold_still_cancels_if_the_tick_has_not_run() {
        let (phase, _) = on_down(Phase::Listening { opener_down: false }, 2_000);
        let (phase, effect) = on_up(phase, 2_700, LONG, TAP);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::DiscardCapture);
    }

    #[test]
    fn press_while_transcribing_cancels() {
        let (phase, effect) = on_down(Phase::Transcribing, 9_000);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::CancelInFlight);
    }

    #[test]
    fn press_while_sending_is_ignored() {
        let (phase, effect) = on_down(Phase::Dispatching, 9_000);
        assert_eq!(phase, Phase::Dispatching);
        assert_eq!(effect, Effect::None);
    }

    #[test]
    fn hold_ignores_a_short_drop_and_sends_only_after_the_button_stays_up() {
        let mut up = None;
        assert!(!hold_should_send(&mut up, 1_000, true));
        assert!(!hold_should_send(&mut up, 1_016, false));
        assert!(!hold_should_send(&mut up, 1_032, true));
        assert!(up.is_none());
        assert!(!hold_should_send(&mut up, 2_000, false));
        assert!(!hold_should_send(&mut up, 2_119, false));
        assert!(hold_should_send(&mut up, 2_120, false));
    }

    #[test]
    fn key_hold_ignores_a_release_until_the_key_has_actually_come_up() {
        let mut seen = false;
        assert!(key_held(Some(false), &mut seen, true, 40));
        assert!(!seen);
        assert!(key_held(Some(true), &mut seen, true, 50));
        assert!(seen);
        assert!(!key_held(Some(false), &mut seen, true, 80));
        let mut unseen = false;
        assert!(key_held(None, &mut unseen, false, 5_000));
        assert!(key_held(None, &mut unseen, true, 40));
        assert!(!key_held(None, &mut unseen, true, 250));
    }

    #[test]
    fn hold_to_talk_sends_on_release_and_does_not_cancel_while_held() {
        let (phase, effect) = on_down(Phase::Idle, 1_000);
        assert_eq!(effect, Effect::BeginCapture);
        let (phase, effect) = on_tick(phase, 4_000, LONG, true);
        assert_eq!(phase, Phase::Listening { opener_down: true });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_up(phase, 4_100, LONG, true);
        assert_eq!(phase, Phase::Transcribing);
        assert_eq!(effect, Effect::EndCapture);
    }
}
