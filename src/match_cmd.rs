//! Match a transcript to a cockpit command.
//!
//! Tokens have to line up in order. Short words such as "up" and "down" must
//! match exactly, so "flaps up" cannot fire "flaps down". Extra words are
//! allowed up to a limit: command mode tolerates "please" and "now", mixed
//! mode does not, because a long sentence is probably chat.

#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    pub name: &'a str,
    pub phrases: &'a [String],
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct MatchOpts {
    pub token_floor: f64,
    pub max_extra: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub index: usize,
    pub phrase: String,
    pub score: f64,
}

pub fn best_match(entries: &[Entry<'_>], transcript: &str, opts: MatchOpts) -> Option<Hit> {
    let heard = tokenize(transcript);
    if heard.is_empty() {
        return None;
    }
    let mut best: Option<Hit> = None;
    let mut best_len = 0usize;
    for (index, entry) in entries.iter().enumerate() {
        if !entry.enabled {
            continue;
        }
        for phrase in entry.phrases {
            let needle = tokenize(phrase);
            if needle.is_empty() {
                continue;
            }
            let Some(score) = align(&heard, &needle, opts) else {
                continue;
            };
            let replace = match &best {
                None => true,
                Some(current) => {
                    needle.len() > best_len
                        || (needle.len() == best_len && score > current.score + 0.0001)
                }
            };
            if replace {
                best_len = needle.len();
                best = Some(Hit {
                    index,
                    phrase: phrase.clone(),
                    score,
                });
            }
        }
    }
    best
}

fn align(heard: &[String], needle: &[String], opts: MatchOpts) -> Option<f64> {
    let extra = heard.len().saturating_sub(needle.len());
    if extra > opts.max_extra {
        return None;
    }
    let mut cursor = 0;
    let mut scores = Vec::with_capacity(needle.len());
    for token in needle {
        let mut found = None;
        while cursor < heard.len() {
            let score = token_score(&heard[cursor], token, opts.token_floor);
            cursor += 1;
            if score >= opts.token_floor {
                found = Some(score);
                break;
            }
        }
        scores.push(found?);
    }
    Some(scores.iter().sum::<f64>() / scores.len() as f64)
}

fn token_score(heard: &str, want: &str, floor: f64) -> f64 {
    if heard == want {
        return 1.0;
    }
    if heard.len() < 4 || want.len() < 4 {
        return 0.0;
    }
    let score = strsim::jaro_winkler(heard, want);
    if score < floor {
        return 0.0;
    }
    // "down" and "darn" should not sneak through on a shared first letter only
    // when the rest is weak. jaro-winkler already rewards the prefix; the
    // floor does the rest.
    score
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(normalize_token)
        .filter(|token| !token.is_empty())
        .collect()
}

fn normalize_token(token: &str) -> String {
    match token {
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" | "niner" => "9",
        "ten" => "10",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> Vec<(String, Vec<String>, bool)> {
        vec![
            (
                "Landing gear".into(),
                vec!["gear up".into(), "gear down".into(), "landing gear".into()],
                true,
            ),
            (
                "Flaps down".into(),
                vec!["flaps down".into(), "lower flaps".into()],
                true,
            ),
            (
                "Flaps up".into(),
                vec!["flaps up".into(), "raise flaps".into()],
                true,
            ),
            ("Radio 1".into(), vec!["radio one".into()], true),
            ("Radio 2".into(), vec!["radio two".into()], true),
            (
                "Level autopilot".into(),
                vec!["level autopilot".into()],
                true,
            ),
            ("Autopilot".into(), vec!["autopilot".into()], true),
            ("Drop bombs".into(), vec!["drop bombs".into()], false),
        ]
    }

    fn entries(book: &[(String, Vec<String>, bool)]) -> Vec<Entry<'_>> {
        book.iter()
            .map(|(name, phrases, enabled)| Entry {
                name,
                phrases,
                enabled: *enabled,
            })
            .collect()
    }

    fn opts(max_extra: usize) -> MatchOpts {
        MatchOpts {
            token_floor: 0.84,
            max_extra,
        }
    }

    #[test]
    fn exact_fuzzy_and_numbers() {
        let book = book();
        let entries = entries(&book);
        let gear = best_match(&entries, "gear up", opts(4)).unwrap();
        assert_eq!(gear.index, 0);

        let typo = best_match(&entries, "geer up", opts(4)).unwrap();
        assert_eq!(typo.index, 0);

        let radio = best_match(&entries, "radio 1", opts(1)).unwrap();
        assert_eq!(entries[radio.index].name, "Radio 1");
    }

    #[test]
    fn up_does_not_match_down_and_disabled_is_skipped() {
        let book = book();
        let entries = entries(&book);
        let hit = best_match(&entries, "flaps down", opts(2)).unwrap();
        assert_eq!(entries[hit.index].name, "Flaps down");
        assert!(best_match(&entries, "drop bombs", opts(2)).is_none());
    }

    #[test]
    fn longer_phrase_wins_and_extra_words_are_limited() {
        let book = book();
        let entries = entries(&book);
        let hit = best_match(&entries, "level autopilot", opts(2)).unwrap();
        assert_eq!(entries[hit.index].name, "Level autopilot");

        let loose = best_match(&entries, "please put the gear up now", opts(4)).unwrap();
        assert_eq!(entries[loose.index].name, "Landing gear");
        assert!(best_match(&entries, "please put the gear up now", opts(1)).is_none());
    }
}
