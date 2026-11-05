//! Push-to-talk gesture.
//!
//! The first press starts listening, but a second press inside the double-tap
//! window cancels it — that is how an accidental tap is undone. Once that
//! window has passed, one press arms a send and a second press (another tap
//! inside the same window) cancels instead. Releasing the arming window with
//! no second press is the confirm.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Listening, and a second tap still means "cancel this press".
    StartWindow {
        until_ms: u64,
    },
    Listening,
    /// A confirm tap landed. A second tap before `until_ms` cancels.
    Arming {
        until_ms: u64,
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

pub fn on_press(phase: Phase, now_ms: u64, double_ms: u64) -> (Phase, Effect) {
    let window = double_ms.max(1);
    match phase {
        Phase::Idle => (
            Phase::StartWindow {
                until_ms: now_ms.saturating_add(window),
            },
            Effect::BeginCapture,
        ),
        Phase::StartWindow { .. } => (Phase::Idle, Effect::DiscardCapture),
        Phase::Listening => (
            Phase::Arming {
                until_ms: now_ms.saturating_add(window),
            },
            Effect::None,
        ),
        Phase::Arming { .. } => (Phase::Idle, Effect::DiscardCapture),
        Phase::Transcribing => (Phase::Idle, Effect::CancelInFlight),
        Phase::Dispatching => (phase, Effect::None),
    }
}

pub fn on_tick(phase: Phase, now_ms: u64) -> (Phase, Effect) {
    match phase {
        Phase::StartWindow { until_ms } if now_ms >= until_ms => (Phase::Listening, Effect::None),
        Phase::Arming { until_ms } if now_ms >= until_ms => {
            (Phase::Transcribing, Effect::EndCapture)
        }
        _ => (phase, Effect::None),
    }
}

pub fn is_capturing(phase: Phase) -> bool {
    matches!(
        phase,
        Phase::StartWindow { .. } | Phase::Listening | Phase::Arming { .. }
    )
}

pub fn title(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Ready",
        Phase::StartWindow { .. } | Phase::Listening => "Listening",
        Phase::Arming { .. } => "Confirming",
        Phase::Transcribing => "Transcribing",
        Phase::Dispatching => "Sending",
    }
}

pub fn hint(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Press once and speak.",
        Phase::StartWindow { .. } => "Tap again now only if that press was a mistake.",
        Phase::Listening => "Press once to send. Press twice to cancel.",
        Phase::Arming { .. } => "Tap again to cancel.",
        Phase::Transcribing => "Working through the recording.",
        Phase::Dispatching => "Sending keys to the sim.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOUBLE: u64 = 320;

    #[test]
    fn first_press_opens_a_cancel_window_and_starts_capture() {
        let (phase, effect) = on_press(Phase::Idle, 1_000, DOUBLE);
        assert_eq!(phase, Phase::StartWindow { until_ms: 1_320 });
        assert_eq!(effect, Effect::BeginCapture);
    }

    #[test]
    fn second_press_inside_the_opening_window_cancels() {
        let (phase, _) = on_press(Phase::Idle, 1_000, DOUBLE);
        let (phase, effect) = on_press(phase, 1_100, DOUBLE);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::DiscardCapture);
    }

    #[test]
    fn opening_window_expires_into_listening_without_stopping_audio() {
        let (phase, _) = on_press(Phase::Idle, 1_000, DOUBLE);
        let (phase, effect) = on_tick(phase, 1_320);
        assert_eq!(phase, Phase::Listening);
        assert_eq!(effect, Effect::None);
        assert!(is_capturing(phase));
    }

    #[test]
    fn single_press_while_listening_arms_a_send() {
        let (phase, effect) = on_press(Phase::Listening, 5_000, DOUBLE);
        assert_eq!(phase, Phase::Arming { until_ms: 5_320 });
        assert_eq!(effect, Effect::None);
    }

    #[test]
    fn second_press_while_arming_cancels() {
        let (phase, _) = on_press(Phase::Listening, 5_000, DOUBLE);
        let (phase, effect) = on_press(phase, 5_100, DOUBLE);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::DiscardCapture);
    }

    #[test]
    fn arming_timer_starts_transcription() {
        let (phase, _) = on_press(Phase::Listening, 5_000, DOUBLE);
        let (phase, effect) = on_tick(phase, 5_319);
        assert_eq!(phase, Phase::Arming { until_ms: 5_320 });
        assert_eq!(effect, Effect::None);
        let (phase, effect) = on_tick(phase, 5_320);
        assert_eq!(phase, Phase::Transcribing);
        assert_eq!(effect, Effect::EndCapture);
    }

    #[test]
    fn press_while_transcribing_cancels() {
        let (phase, effect) = on_press(Phase::Transcribing, 9_000, DOUBLE);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(effect, Effect::CancelInFlight);
    }

    #[test]
    fn press_while_sending_is_ignored() {
        let (phase, effect) = on_press(Phase::Dispatching, 9_000, DOUBLE);
        assert_eq!(phase, Phase::Dispatching);
        assert_eq!(effect, Effect::None);
    }
}
