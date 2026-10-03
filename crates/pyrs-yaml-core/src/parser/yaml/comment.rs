/// Pre-compute the byte offset of the start of each line in the given text.
///
/// The returned vector has one entry per line, where `line_offsets[line]`
/// is the byte offset of the first byte of that line in `yaml`.
///
/// # Arguments
/// * `yaml` - The raw YAML text.
///
/// # Returns
/// A vector of byte offsets, one per line. The length is `number_of_lines + 1`
/// (the extra entry is the offset past the final character, for convenience).
pub fn compute_line_offsets(yaml: &str) -> Vec<usize> {
    scan_yaml(yaml).line_offsets
}

/// Result of a single full-text scan over the input.
///
/// Collects everything the parser needs from the raw text in one pass,
/// avoiding separate `contains('#')`/`contains('&')`/`is_ascii()`/line-offset
/// traversals (4 full scans on comment-free documents before the parser runs).
#[derive(Debug)]
pub struct YamlScan {
    /// Byte offset of the start of each line (`len = lines + 1`).
    pub line_offsets: Vec<usize>,
    /// Whether the text contains a `#` (possible comment).
    pub has_hash: bool,
    /// Whether the text contains an `&` (possible anchor).
    pub has_amp: bool,
    /// Whether the text is entirely ASCII.
    pub is_ascii: bool,
}

/// Single pass over `yaml` collecting line offsets and marker presence.
pub fn scan_yaml(yaml: &str) -> YamlScan {
    let mut offsets = Vec::with_capacity(yaml.len() / 16 + 1);
    offsets.push(0);
    let mut has_hash = false;
    let mut has_amp = false;
    let mut is_ascii = true;
    for (i, byte) in yaml.bytes().enumerate() {
        match byte {
            b'\n' => offsets.push(i + 1),
            b'#' => has_hash = true,
            b'&' => has_amp = true,
            0x00..=0x7f => {}
            _ => is_ascii = false,
        }
    }
    YamlScan {
        line_offsets: offsets,
        has_hash,
        has_amp,
        is_ascii,
    }
}

/// Check if a character is a valid unquoted anchor name character.
/// YAML 1.2 allows any character except whitespace and flow indicators: `{}[],`
fn is_valid_anchor_char(c: char) -> bool {
    !c.is_whitespace() && c != '{' && c != '}' && c != '[' && c != ']' && c != ','
}

/// Resolve the anchor name granit attached to a node whose content begins at
/// byte offset `byte_start`, without re-scanning the whole document.
///
/// granit's event stream marks a node as anchored (`anchor_id != 0`) but hands
/// back only the numeric id, never the `&name` text — so the display name for
/// round-trip re-emission must be recovered from the source. The id *is*
/// granit's authoritative decision about **which** tokens are anchors, so the
/// recovery does not need to make that decision again: it only reads the name
/// back at the exact site granit pointed to.
///
/// We scan left from `byte_start` for the nearest `&` that (a) sits at a
/// node-start boundary and (b) is followed by a non-empty `is_valid_anchor_char`
/// run ending at or before `byte_start`. That run reproduces the maximal name
/// granit's own `scan_anchor` stored in `TokenType::Anchor`. The lookup is
/// *position-isolated*: because granit tells us an anchored node begins here and
/// its `&name` is the nearest qualifying `&` to the left, a misread of any one
/// byte class can only affect this node's name — it can never shift the naming
/// of any *other* anchor. That shift was the failure mode of the retired whole-
/// text quote-state pre-scan + counter pairing, which regenerated the same drift
/// family every round (libFuzzer `yaml_roundtrip`: apostrophe #68da2420, embedded
/// `&` #83cc68c6, single-quote backslash #12f01ee0).
///
/// Handles every anchored-node shape granit emits:
/// - plain/quoted scalar (`&x v`): the name run ends before the value offset;
/// - anchored null or block collection (`&x` then an empty span): the run ends
///   exactly at `byte_start`;
/// - block scalar (`&x |`): the anchor lives on the header line while
///   `byte_start` is on the content line, and the nearest boundary `&` to its
///   left is still that header anchor.
pub fn anchor_name_before(yaml: &str, byte_start: usize) -> Option<String> {
    let bytes = yaml.as_bytes();
    let b = byte_start.min(yaml.len());
    // `&` is 0x26 and can never be a UTF-8 continuation byte (those are
    // 0x80..=0xBF), so a byte scan locates candidate starts without risking a
    // split multibyte sequence; the maximal name run is then read as chars.
    let mut i = b;
    while i > 0 {
        i -= 1;
        if bytes[i] != b'&' {
            continue;
        }
        // A `&` only begins an anchor where a node may begin: start of input or
        // after whitespace / a value indicator / a flow opener. Interior `&`
        // characters of a name run (`&&&`) fail this and are skipped, so the
        // leftmost `&` of the token wins and its full run is the name.
        let boundary = i == 0
            || matches!(
                bytes[i - 1],
                b' ' | b'\t' | b'\n' | b'\r' | b':' | b',' | b'[' | b'{' | b'-'
            );
        if !boundary {
            continue;
        }
        let name: String = yaml[i + 1..]
            .chars()
            .take_while(|c| is_valid_anchor_char(*c))
            .collect();
        if name.is_empty() {
            // `&` followed by whitespace / EOL / flow indicator is not an anchor
            // (matches granit's empty-run rejection).
            continue;
        }
        // The name run must terminate at or before the node's own content start;
        // otherwise this `&` is not the anchor granit pointed to.
        if i + 1 + name.len() <= b {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The byte offset immediately after an `&name` token — the `byte_start` a
    /// granit event would carry for an anchored collection/null whose span is
    /// empty right after the name.
    fn after_anchor(s: &str, anchor: &str) -> usize {
        s.find(anchor).expect("anchor token present") + anchor.len()
    }

    /// Position-isolated name recovery for the plain, same-line case.
    #[test]
    fn anchor_name_before_reads_maximal_run() {
        assert_eq!(
            anchor_name_before(
                "defaults: &defaults\n  key: value",
                after_anchor("defaults: &defaults\n  key: value", "&defaults")
            ),
            Some("defaults".to_string())
        );
        // Colon, dot and hash are ordinary name chars (granit grammar).
        assert_eq!(
            anchor_name_before("key: &a:b value", after_anchor("key: &a:b value", "&a:b")),
            Some("a:b".to_string())
        );
        assert_eq!(
            anchor_name_before(
                "key: &anchor.name v",
                after_anchor("key: &anchor.name v", "&anchor.name")
            ),
            Some("anchor.name".to_string())
        );
        assert_eq!(
            anchor_name_before(
                "key: &anchor#name v",
                after_anchor("key: &anchor#name v", "&anchor#name")
            ),
            Some("anchor#name".to_string())
        );
        // A `"` is a name char; the run stops at the following space.
        assert_eq!(
            anchor_name_before("key: &\"q value", after_anchor("key: &\"q value", "&\"q")),
            Some("\"q".to_string())
        );
    }

    /// The name run ends at a flow indicator, not swallowing the flow opener.
    #[test]
    fn anchor_name_before_stops_at_flow_indicator() {
        let s = "key: &anchor{sub}";
        assert_eq!(
            anchor_name_before(s, after_anchor(s, "&anchor")),
            Some("anchor".to_string())
        );
        let s = "key: &anchor, next";
        assert_eq!(
            anchor_name_before(s, after_anchor(s, "&anchor")),
            Some("anchor".to_string())
        );
    }

    /// `&&&&:` names the anchor `&&&:` (colon is a name char): the interior `&`
    /// characters are not at a node boundary, so the leftmost `&` of the token
    /// wins and its full run is the name.
    #[test]
    fn anchor_name_of_run_of_ampersands() {
        let s = "a: &&&&:\n";
        // value is a null node whose empty span starts right after `&&&&:`.
        let b = s.find("&&&&:").unwrap() + "&&&&:".len();
        assert_eq!(anchor_name_before(s, b), Some("&&&:".to_string()));
    }

    /// A `&` embedded in a plain *key* (`sbb&e`) is literal content — granit
    /// never marks that node anchored, and even if a byte_start were handed in
    /// it must not harvest the interior `&` (fails the boundary test) but the
    /// real `&b` that granit did anchor.
    #[test]
    fn anchor_name_before_ignores_embedded_ampersand_in_key() {
        let s = "sbb&e: &b v\n";
        // anchored scalar value `v`; nearest boundary `&` to its left is `&b`.
        let b = s.find(" v").unwrap();
        assert_eq!(anchor_name_before(s, b), Some("b".to_string()));
    }

    /// A bare apostrophe in a plain key (`bas'e`) is not a quote open, so a
    /// later anchored node still resolves its name — the family bug #68da2420.
    #[test]
    fn anchor_after_plain_apostrophe_key_resolves() {
        let s = "bas'e: &b v\n";
        let b = s.find(" v").unwrap();
        assert_eq!(anchor_name_before(s, b), Some("b".to_string()));
    }

    /// A single-quoted key ending in a backslash (`'a\'`) does not hide a later
    /// anchor's name recovery — the family bug #12f01ee0.
    #[test]
    fn anchor_after_single_quoted_backslash_key_resolves() {
        let s = "'a\\': &b v\n";
        let b = s.find(" v").unwrap();
        assert_eq!(anchor_name_before(s, b), Some("b".to_string()));
    }

    /// A block scalar's anchor sits on the header line while granit reports the
    /// content start; the nearest boundary `&` to the left is that header anchor.
    #[test]
    fn anchor_name_before_reaches_block_scalar_header() {
        let s = "a: &x |\n  line1\n  line2\n";
        let b = s.find("line1").unwrap();
        assert_eq!(anchor_name_before(s, b), Some("x".to_string()));
    }

    /// The name run must end at or before the node's own content start, so an
    /// empty offset region yields no name (never a spurious one).
    #[test]
    fn anchor_name_before_is_none_without_anchor() {
        assert_eq!(anchor_name_before("key: value", 0), None);
        assert_eq!(anchor_name_before("key: value", 4), None);
    }

    /// End-to-end: an anchor whose document contains a bare-apostrophe key
    /// must survive the parse -> serialize -> re-parse -> serialize loop the
    /// fuzz target exercises.
    #[test]
    fn anchor_survives_plain_apostrophe_key_roundtrip() {
        for input in [
            "bas'e: &b\n hhhbase: &b\n ild:\n  <<: *b\n  y___: 2  # inline\n",
            "a': &x\n  b: 1\nc: &x\n  d: 2\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {input:?}: {once:?}"
            );
        }
    }

    /// End-to-end idempotence on the embedded-`&` crash input + a minimal pair
    /// (the family bug #83cc68c6).
    #[test]
    fn ampersand_in_plain_key_roundtrip() {
        for input in [
            "base]]]]]]]]]]]]]]]]]]] 0]] ] ]]]:  a\nsbb&e: &b\n  be: &b\n   ",
            "sbb&e: &b\n  be: &b\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {input:?}: {once:?}"
            );
        }
    }

    /// End-to-end idempotence on the single-quote-backslash crash input (the
    /// family bug #12f01ee0).
    #[test]
    fn backslash_in_single_quote_roundtrip() {
        let crash: &[u8] = &[
            0x79, 0x61, 0x20, 0x7c, 0x32, 0x30, 0x20, 0x20, 0x66, 0x69, 0x72, 0x3a, 0x5c, 0x5c,
            0x5c, 0x3a, 0x20, 0x26, 0x62, 0x0a, 0x20, 0x21, 0x78,
        ];
        let input = std::str::from_utf8(crash).unwrap();
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
        let once = crate::serializer::to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
        assert_eq!(
            once,
            crate::serializer::to_yaml(&again),
            "drift for {input:?}: {once:?}"
        );
    }

    /// A name that would cross a line break must not be harvested — granit ends
    /// an anchor token at a line break.
    #[test]
    fn anchor_name_never_crosses_line_break() {
        let yaml = "&\"X-\r:&\"X-\r";
        let node = crate::parser::parse(yaml, pyrs_schema::types::Schema::Core).unwrap();
        let once = crate::serializer::to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap();
        let twice = crate::serializer::to_yaml(&again);
        assert_eq!(once, twice, "not idempotent: {once:?} vs {twice:?}");
    }

    /// Grammar regression round-trips: colon/dot/hash names, quoted-leading
    /// names and the value-indicator-colon case must all re-emit and re-parse
    /// to themselves (the emit is bare `&name `, a maximal run re-scans equal).
    #[test]
    fn anchor_grammar_shapes_round_trip() {
        for input in [
            "key: &a:b value\n",
            "key: &anchor.name value\n",
            "key: &anchor#name value\n",
            "key: &anchor{sub}\n",
            "a: &x: 1\n",
            "a: &x:y 1\n",
            "&\"X-\r:",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {input:?}: {once:?}"
            );
        }
    }
}
