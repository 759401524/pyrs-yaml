//! Typo hints for the two command surfaces `pyq` exposes.
//!
//! The two surfaces have different vocabularies and must not be mixed: a
//! misspelled native subcommand gets pyq's "unknown command" wording, while a
//! near-miss inside the jq/yq shell gets the supported form it resembles. Both
//! are computed here without a new dependency: the vocabularies are tiny.

/// Levenshtein distance over Unicode scalars. Counting scalars rather than
/// bytes keeps a stray non-ASCII argv from being scored as many edits.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = if ca == cb {
                prev[j]
            } else {
                1 + prev[j].min(prev[j + 1]).min(cur[j])
            };
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// The unique candidate within `max` edits, or `None`. A tie is not a hint:
/// guessing between two commands is worse than the plain refusal.
pub fn closest<'a>(word: &str, candidates: &'a [&'a str], max: usize) -> Option<&'a str> {
    let mut best: Option<(&'a str, usize)> = None;
    let mut tied = false;
    for &candidate in candidates {
        let d = edit_distance(word, candidate);
        if d > max {
            continue;
        }
        match best {
            None => best = Some((candidate, d)),
            Some((_, prev)) if d < prev => {
                best = Some((candidate, d));
                tied = false;
            }
            Some((_, prev)) if d > prev => {
                let _ = prev; // a worse candidate is not a tie
            }
            Some(_) => tied = true,
        }
    }
    if tied {
        None
    } else {
        best.map(|(candidate, _)| candidate)
    }
}

/// A token that can be a misspelled subcommand. A jq/yq expression starts with
/// `.`, `$` or `[`, contains a call or an operator, or is a flag; suggesting a
/// verb for any of those would be a guess about a different language.
pub fn looks_like_command(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with(['-', '.', '$', '['])
        && !word.contains(['(', ')', '{', '}', '|', '=', '?', ';', ' '])
        && !word.contains("//")
        && !word.contains("..")
}

/// Native "unknown command" line, without the `pyq: ` prefix the entry point
/// adds. `to-xml` is deliberately absent in this read-only-XML slice: it is two
/// edits from `to-toml`, so it needs an explicit exemption or a hint would
/// break the documented refusal instead of preserving it.
pub fn native_command(word: &str, commands: &[&str]) -> Option<String> {
    if word == "to-xml" || !looks_like_command(word) {
        return None;
    }
    closest(word, commands, 2)
        .map(|best| format!("unknown command '{word}'; did you mean '{best}'?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_counts_scalars_and_edits() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("from-xml", "from-xml"), 0);
        assert_eq!(edit_distance("fro-xml", "from-xml"), 1);
        assert_eq!(edit_distance("from-xm", "from-xml"), 1);
        assert_eq!(edit_distance("frmo-xml", "from-xml"), 2);
        assert_eq!(edit_distance("get", "set"), 1);
        assert_eq!(edit_distance("é", "e"), 1);
    }

    #[test]
    fn closest_refuses_a_tie() {
        assert_eq!(closest("se", &["set", "get", "fmt"], 2), Some("set"));
        assert_eq!(closest("x", &["a", "b"], 1), None);
        assert_eq!(closest("get", &["get", "set"], 2), Some("get"));
    }

    #[test]
    fn only_bare_words_are_command_typos() {
        let cmds = ["get", "set", "fmt", "to-toml"];
        assert!(
            native_command("gte", &cmds)
                .unwrap()
                .contains("did you mean 'get'")
        );
        assert_eq!(native_command(".a.b", &cmds), None);
        assert_eq!(native_command("--help", &cmds), None);
        assert_eq!(native_command("del(.a)", &cmds), None);
        assert_eq!(native_command("to-xml", &cmds), None);
    }
}
