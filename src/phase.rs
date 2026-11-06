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

pub fn on_up(phase: Phase, now_ms: u64, long_ms: u64) -> (Phase, Effect) {
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

pub fn on_tick(phase: Phase, now_ms: u64, long_ms: u64) -> (Phase, Effect) {
    let long_ms = long_ms.max(1);
    match phase {
        Phase::Holding { since_ms } if now_ms.saturating_sub(since_ms) >= long_ms => {
            (Phase::Idle, Effect::DiscardCapture)
        }
        _ => (phase, Effect::None),
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

pub fn hint(phase: Phase) -> &'static str {
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

    #[test]
    fn first_press_starts_capture_and_its_release_keeps_listening() {
        let (phase, effect) = on_down(Phase::Idle, 1_000);
        assert_eq!(phase, Phase::Listening { opener_down: true });
        assert_eq!(effect, Effect::BeginCapture);
        let (phase, effect) = on_up(phase, 1_080, LONG);
        assert_eq!(phase, Phase::Listening { opener_down: false });
        assert_eq!(effect, Effect::None);
        assert!(is_capturing(phase));
    }

    #[test]
    fn short_second_press_sends() {
        let (phase, _) = on_down(Phase::Idle, 1_000);
        let (phase, _) = on_up(phase, 1_080, LONG);
        let (phase, effect) = on_down(phase, 2_000);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_tick(phase, 2_400, LONG);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_up(phase, 2_400, LONG);
        assert_eq!(phase, Phase::Transcribing);
        assert_eq!(effect, Effect::EndCapture);
    }

    #[test]
    fn holding_the_second_press_cancels_before_release() {
        let (phase, _) = on_down(Phase::Idle, 1_000);
        let (phase, _) = on_up(phase, 1_080, LONG);
        let (phase, _) = on_down(phase, 2_000);
        let (phase, effect) = on_tick(phase, 2_699, LONG);
        assert_eq!(phase, Phase::Holding { since_ms: 2_000 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_tick(phase, 2_700, LONG);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::DiscardCapture);
        let (phase, effect) = on_up(phase, 2_900, LONG);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::None);
    }

    #[test]
    fn release_after_the_hold_still_cancels_if_the_tick_has_not_run() {
        let (phase, _) = on_down(Phase::Listening { opener_down: false }, 2_000);
        let (phase, effect) = on_up(phase, 2_700, LONG);
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
}
