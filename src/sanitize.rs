//! Clean a transcript before it is typed into IL-2 chat.

pub fn sanitize_chat(raw: &str, max_chars: usize) -> String {
    let without_tags = strip_angle_tags(raw);
    let filtered: String = without_tags
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some(' ')
            } else if c.is_control() {
                None
            } else {
                Some(c)
            }
        })
        .collect();
    let collapsed = filtered.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim().trim_matches('"').trim();
    if is_blank_artifact(trimmed) {
        return String::new();
    }
    if max_chars == 0 {
        return String::new();
    }
    trimmed.chars().take(max_chars).collect()
}

fn strip_angle_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '<' {
            for next in chars.by_ref() {
                if next == '>' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn is_blank_artifact(text: &str) -> bool {
    if text.is_empty() || !text.chars().any(|c| c.is_alphanumeric()) {
        return true;
    }
    let bare = text
        .trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')' | '.' | '-' | '_' | ' '))
        .to_lowercase();
    matches!(
        bare.as_str(),
        "blank_audio" | "blank audio" | "silence" | "music" | "noise" | "inaudible" | "null"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace_and_drops_controls() {
        let text = sanitize_chat("  bandits\nhigh \u{0007} ", 160);
        assert_eq!(text, "bandits high");
    }

    #[test]
    fn drops_whisper_silence_and_truncates() {
        assert_eq!(sanitize_chat("[BLANK_AUDIO]", 160), "");
        assert_eq!(sanitize_chat("...", 160), "");
        assert_eq!(sanitize_chat("abcdef", 3), "abc");
    }

    #[test]
    fn strips_special_tokens() {
        assert_eq!(sanitize_chat("<|en|>gear up", 160), "gear up");
    }
}
