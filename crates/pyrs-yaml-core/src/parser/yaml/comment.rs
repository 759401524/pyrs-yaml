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

/// Check if a character can continue an unquoted anchor name.
///
/// Mirrors granit's `is_anchor_char` — printable, not `s-space`/break, not a flow
/// indicator, not `z` — so the harvested run is the maximal run the reader
/// stored. `char::is_whitespace` is the wrong vocabulary here: it also matches
/// NBSP, which granit treats as ordinary name content, so a Unicode trim would
/// cut `&a<NBSP>b` down to `a` and silently rename the anchor on re-emission.
/// `is_yaml_document_char` stays on top because the AST must hold text our own
/// reader accepts inside a document (U+FEFF and the noncharacters, #262).
fn is_valid_anchor_char(c: char) -> bool {
    // ASCII fast path, which is every real anchor name: printable ASCII minus the
    // flow indicators and the blanks. The flow indicators are all ASCII, so the
    // non-ASCII arm below needs no test for them.
    if c.is_ascii() {
        return matches!(c, '!'..='~') && !matches!(c, '\t' | '{' | '}' | '[' | ']' | ',');
    }
    pyrs_schema::is_yaml_document_char(c) && !pyrs_schema::is_yaml_blank(c)
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
        // A tag token (`!` followed by ns-uri-char*) may legally contain `&`, and
        // `&` also follows `-` (from `- &a v`) and `:` - so the boundary test alone
        // cannot tell an anchor from an ampersand sitting inside a preceding tag.
        // Walk the whitespace-delimited run leftwards from here: if it opens with
        // `!`, this `&` belongs to that tag and granit never meant it as an anchor.
        // Tags and anchors are always space-separated, so the run's first byte is
        // exactly the token's first byte.
        let mut run = i;
        while run > 0 && !bytes[run - 1].is_ascii_whitespace() {
            run -= 1;
        }
        if bytes[run] == b'!' {
            continue;
        }
        // A `#` that opens a comment anywhere earlier on this line puts everything
        // after it — including this `&` — inside comment text, which granit never
        // treats as an anchor. The earlier form of this guard compared only the
        // token immediately before the `&`, so `# !! &?` donated its `&?` as the
        // name across a comment line sitting between `bg: &b` and the node's content:
        // the anchor was renamed on re-read and every alias pointing at the old name
        // was orphaned (libFuzzer `yaml_roundtrip` crash-68adf94c — the same class as
        // crash-04fddeb8, whose `# &l` shape this still refuses).
        if comment_opens_before_on_line(bytes, i) {
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

/// True when a comment starts on `byte`'s line before `byte` itself, which makes
/// `byte` comment text rather than node syntax.
///
/// YAML opens a comment at a `#` in the first column or after a blank, so a `#`
/// embedded in a scalar (`x#y`) does not count and cannot hide an anchor. A quoted
/// `#` earlier on the line is the one shape this would over-refuse, and it has no
/// reachable form: node properties (`&anchor`, `!tag`) always precede the node
/// value, so nothing that granit can mark as an anchor ever follows a quoted
/// scalar on the same line.
fn comment_opens_before_on_line(bytes: &[u8], byte: usize) -> bool {
    let mut line = 0usize;
    for at in (0..byte).rev() {
        if matches!(bytes[at], b'\n' | b'\r') {
            line = at + 1;
            break;
        }
    }
    (line..byte)
        .any(|at| bytes[at] == b'#' && (at == line || matches!(bytes[at - 1], b' ' | b'\t')))
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

    /// A tag URI may legally contain `&` (it is an ns-uri-char), and the writer
    /// emits properties as `&anchor !tag`, so the rightmost boundary `&` left of a
    /// node can sit *inside the tag* instead of being the anchor (libFuzzer
    /// `yaml_roundtrip`, crash-f44eca1d minimised to 12 bytes): `&F !-&l ` read back
    /// as anchor `l`, renaming the anchor every round and orphaning any `*F` alias,
    /// so emission never reached a fixed point. Rejecting a `&` whose token opens
    /// with `!` makes the scanner find the real `&F`.
    #[test]
    fn anchor_name_before_skips_ampersand_inside_tag_token() {
        let s = "&F !-&l \n";
        // granit's empty-scalar span for the anchored node starts at byte 7.
        assert_eq!(anchor_name_before(s, 7), Some("F".to_string()));

        // The reverse property order worked before and must keep working.
        let s = "!-&l &F";
        assert_eq!(anchor_name_before(s, s.len()), Some("F".to_string()));

        // A tag carrying the only `&`, with no anchor at all, must yield nothing
        // rather than harvesting the tag's innards.
        let s = "key: !a&b ";
        assert_eq!(anchor_name_before(s, s.len() - 1), None);
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

    /// A `&` anywhere inside comment text is never an anchor — not only one that
    /// follows the `#` directly. `bg: &b` + `# !! &?` + content re-read as `&?`,
    /// renaming the anchor and orphaning every alias to it (libFuzzer
    /// `yaml_roundtrip` crash-68adf94c, the shape that escaped the immediate-token
    /// version of this guard).
    #[test]
    fn anchor_name_before_ignores_ampersand_anywhere_in_comment_text() {
        let s = "bg: &b\n  # !! &?\n  ~: ~\n";
        let b = s.find("~: ~").expect("content present");
        assert_eq!(anchor_name_before(s, b), Some("b".to_string()));
        assert!(
            comment_opens_before_on_line(s.as_bytes(), s.find("&?").expect("comment amp")),
            "the ampersand inside the comment body is comment text"
        );
        assert!(
            !comment_opens_before_on_line(s.as_bytes(), s.find("&b").expect("real anchor")),
            "the real anchor precedes any comment on its line"
        );
        // A `#` embedded in a plain scalar opens nothing.
        let plain = "k: x#y &a v\n";
        assert!(
            !comment_opens_before_on_line(plain.as_bytes(), plain.find("&a").expect("anchor")),
            "a mid-token `#` is not a comment opener"
        );
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

    /// End-to-end: the document that renamed its anchor by harvesting a `&?` out of
    /// the comment line below it now keeps the name and settles in one step. Bytes
    /// are read from the committed seed so the test cannot drift from the input that
    /// actually crashed (libFuzzer `yaml_roundtrip` crash-68adf94c).
    #[test]
    fn anchor_keeps_its_name_across_a_comment_line_holding_an_ampersand() {
        let raw =
            include_bytes!("../../../../../fuzz/seeds/yaml_roundtrip/former-crash-68adf94c.seed");
        let src = String::from_utf8(raw.to_vec()).expect("seed is utf-8");
        let node = crate::parser::parse(&src, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
        let once = crate::serializer::to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
        assert!(
            once.starts_with("bg: &b\n"),
            "the anchor token stays `&b`, not the comment's `&?`: {once:?}"
        );
        assert!(
            once.contains("# !! &?"),
            "the comment keeps its own ampersand as text: {once:?}"
        );
        assert_eq!(
            crate::serializer::to_yaml(&again),
            once,
            "{src:?} must settle in one step: {once:?}"
        );
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
