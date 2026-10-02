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

/// 从原始 YAML 文本中提取的锚点信息。
#[derive(Debug, Clone)]
pub struct RawAnchor {
    /// 锚点所在行（0 起始）
    pub line: usize,
    /// 锚点起始列（0 起始，`&` 字符的位置）
    pub col: usize,
    /// 锚点名称（不含 `&` 前缀）
    pub name: String,
}

/// Check if a character is a valid unquoted anchor name character.
/// YAML 1.2 allows any character except whitespace and flow indicators: `{}[],`
fn is_valid_anchor_char(c: char) -> bool {
    !c.is_whitespace() && c != '{' && c != '}' && c != '[' && c != ']' && c != ','
}

/// 从原始 YAML 文本中逐行扫描提取所有锚点定义（`&name`）。
///
/// 支持非引号锚点名（字母、数字、`-`、`_`、`.`、`:`、`#` 等）和引号锚点名（`&"name"`）。
/// 锚点名在遇到空白或流指示符（`{}[],`）时终止。
///
/// # Arguments
/// * `yaml` - 原始 YAML 文本。
///
/// # Returns
/// 按出现顺序排列的 `RawAnchor` 列表。
pub fn extract_anchors(yaml: &str) -> Vec<RawAnchor> {
    // Cheap byte gate: every anchor literal contains `&`; skip the per-char
    // quote state machine entirely for documents without one (the common case).
    if !yaml.as_bytes().contains(&b'&') {
        return Vec::new();
    }
    let mut anchors = Vec::new();

    for (line_idx, line) in yaml.lines().enumerate() {
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;
        // End offset of the last accepted anchor token (`&` + name); scans
        // starting inside it are re-reading the same anchor's `&` characters
        // (`&&&&` would otherwise yield names `&&&`, `&&` and `&`).
        let mut token_end = 0usize;

        for (col_idx, ch) in line.char_indices() {
            // Inside a just-accepted anchor token: granit reads the name as one
            // atomic run of ns-chars, so a `"` / `#` / `&` that is part of the
            // name must NOT toggle quote state or start a comment for the rest of
            // the line — that desync shifted every later id-name pairing (the
            // root of the libFuzzer anchor family).
            if col_idx < token_end {
                continue;
            }
            // Comment starts (column 0 or preceded by blank, outside quotes)
            // end the anchor scan for this line: `#&&&:&` is comment text, and
            // harvesting anchors from it shifted every later id-name pairing
            // (libFuzzer `yaml_roundtrip` follow-up).
            if ch == '#'
                && !in_single_quote
                && !in_double_quote
                && (col_idx == 0 || line[..col_idx].ends_with([' ', '\t']))
            {
                break;
            }
            // 锚点提取：节点起始处（行首 / 空白 / `:,[]{}-` 之后）的 `&` 才是锚点。
            // plain 标量内部的 `&`（如裸键 `sbb&e`）是字面内容，granit 不视作锚点；
            // 误收会往有序 anchor_names 塞入幻名、错位 id->name 配对（crash-83cc68c6）。
            if !in_single_quote && !in_double_quote && ch == '&' && at_node_start(line, col_idx) {
                let rest = &line[col_idx + 1..];
                if let Some(anchor_name) = scan_anchor_name(rest) {
                    token_end = col_idx + 1 + anchor_name.len();
                    anchors.push(RawAnchor {
                        line: line_idx,
                        col: col_idx,
                        name: anchor_name,
                    });
                    continue;
                }
            }
            is_string_char(
                &mut in_single_quote,
                &mut in_double_quote,
                &mut escaped,
                ch,
                at_node_start(line, col_idx),
            );
        }
    }

    anchors
}

/// True when the byte at `col_idx` sits where a *node* may begin — line start
/// or after a structural/space token (` \t:,[]{}-`). Both a quoted scalar and
/// an anchor (`&`) only start at such a boundary: a `&` or `'`/`"` embedded in a
/// plain scalar (the `&` of a bare key like `sbb&e`, the `'` of `bas'e`) is
/// literal content, not a token start. Treating an embedded `&` as an anchor
/// added phantom names to the ordered `anchor_names` list and desynced the
/// id->name pairing in `register_anchor`, so real anchors got mislabeled and
/// drifted each round (libFuzzer `yaml_roundtrip` crash-83cc68c6; the quote case
/// is crash-68da2420). Closing a quote is always allowed and handled separately.
fn at_node_start(line: &str, col_idx: usize) -> bool {
    // O(1): only the single byte immediately before the char matters. The
    // boundary set is pure ASCII, and a UTF-8 continuation byte can never equal
    // one, so a char following a multibyte char is (correctly) mid-plain-scalar.
    match line.as_bytes().get(col_idx.wrapping_sub(1)) {
        None => true, // start of line
        Some(&b) => matches!(b, b' ' | b'\t' | b':' | b',' | b'[' | b'{' | b'-'),
    }
}

/// Advance quote/escape state machine for a single character.
/// `can_open` gates whether a quote may *start* a region here (see
/// [`quote_can_open`]); a quote that is already open always closes.
/// Returns `true` if the character was consumed (quote toggle or escape start).
fn is_string_char(
    in_single_quote: &mut bool,
    in_double_quote: &mut bool,
    escaped: &mut bool,
    ch: char,
    can_open: bool,
) -> bool {
    if *escaped {
        *escaped = false;
        return true;
    }
    if ch == '\\' && *in_double_quote {
        // Backslash is an escape lead ONLY in double-quoted scalars. A
        // single-quoted scalar has no escape processor (its only special
        // sequence is `''`), so a `\` there is literal. Treating it as an
        // escape ate the closing `'` of a backslash-terminated single-quoted
        // key like `'ya |20  fir:\\\'`, leaving the quote open and hiding every
        // later `&anchor` (libFuzzer `yaml_roundtrip` crash-12f01ee0).
        *escaped = true;
        return true;
    }
    if ch == '\'' && !*in_double_quote {
        if *in_single_quote {
            *in_single_quote = false;
            return true;
        }
        if can_open {
            *in_single_quote = true;
            return true;
        }
        return false;
    }
    if ch == '"' && !*in_single_quote {
        if *in_double_quote {
            *in_double_quote = false;
            return true;
        }
        if can_open {
            *in_double_quote = true;
            return true;
        }
        return false;
    }
    false
}

/// Scan an anchor name from the text after `&`.
///
/// Aligned with granit's authoritative anchor grammar (its scanner reads a
/// name as `while is_anchor_char(..)`): the name is the maximal run of
/// anchor-name characters after the `&`. Per YAML 1.2 `:`/`#`/`"`/`&` are
/// ordinary name characters, and the run ends only at whitespace, a line break
/// or a flow indicator (`{}[],`). There is deliberately NO quoted-anchor form
/// and NO value-indicator-colon special case — those hand-invented branches
/// were a second grammar drifting from granit and are the root of the entire
/// libFuzzer anchor family (#215/#218/#227/#228). An empty run (`&` followed by
/// whitespace / EOL / flow) is not an anchor, matching granit.
fn scan_anchor_name(rest: &str) -> Option<String> {
    let name: String = rest
        .chars()
        .take_while(|c| is_valid_anchor_char(*c))
        .collect();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_anchors() {
        let yaml = "defaults: &defaults\n  key: value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "defaults");
        assert_eq!(anchors[0].line, 0);
    }

    /// A `'` inside a *plain* scalar (a bare key like `bas'e`) is literal
    /// content, not a quote opening. The old state machine toggled
    /// `in_single_quote` on it, so every `&anchor` on later lines was read as
    /// quoted content and dropped — anchors vanished from the emit
    /// (libFuzzer `yaml_roundtrip` crash-68da2420). Opening is gated on a
    /// token boundary; only a real quoted scalar hides an anchor.
    #[test]
    fn apostrophe_in_plain_key_does_not_swallow_later_anchor() {
        // apostrophe mid-word: anchor after it still extracted
        let anchors = extract_anchors("bas'e: &b\nnext: &c 1\n");
        assert_eq!(
            anchors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["b", "c"]
        );
        // a genuinely single-quoted scalar still hides its `&`
        assert!(extract_anchors("key: 'a &b c'").is_empty());
        // double-quote opening at a boundary still hides its `&`
        assert!(extract_anchors(r#"key: "a &b c""#).is_empty());
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

    /// A `&` embedded in a *plain* scalar (a bare key like `sbb&e`) is literal
    /// content, not an anchor start. The old scan harvested it, adding a phantom
    /// name to the ordered `anchor_names` list; `register_anchor` pairs names to
    /// nodes by index, so the phantom shifted every later id->name binding and
    /// real anchors got mislabeled (`&b` emitted as `&e:`), drifting each round
    /// (libFuzzer `yaml_roundtrip` crash-83cc68c6). Anchor start is now gated on
    /// a node boundary, mirroring the quote case.
    #[test]
    fn ampersand_in_plain_key_is_not_an_anchor() {
        // `&` mid-key: no anchor harvested; the real `&b` after `: ` is
        let anchors = extract_anchors("sbb&e: &b\n");
        assert_eq!(
            anchors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["b"]
        );
        // end-to-end idempotence on the crash input + a minimal pair
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

    /// A backslash is an escape lead only in a *double*-quoted scalar; a
    /// single-quoted scalar has no escape processor (only `''`). The old scan
    /// treated `\` as an escape inside single quotes too, so a `'` following a
    /// backslash (the closing quote of `'a\'`) was swallowed as "escaped", the
    /// quote never closed, and every `&anchor` after it was hidden from
    /// `extract_anchors` — the value's anchor then vanished on re-parse
    /// (libFuzzer `yaml_roundtrip` crash-12f01ee0).
    #[test]
    fn backslash_in_single_quote_does_not_hide_later_anchor() {
        let anchors = extract_anchors("'a\\': &b v\n");
        assert_eq!(
            anchors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["b"]
        );
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

    #[test]
    fn test_extract_multiple_anchors() {
        let yaml = "a: &foo 1\nb: &bar 2";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].name, "foo");
        assert_eq!(anchors[1].name, "bar");
    }

    #[test]
    fn test_extract_anchor_with_dot() {
        let yaml = "key: &anchor.name value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor.name");
    }

    #[test]
    fn anchor_name_is_maximal_run_including_colon() {
        // granit's scanner has NO quoted-anchor form and treats `:` as an
        // ordinary name char (its `issue14_anchor_scanner_consumes_colon_as_
        // name_character`): the name is the maximal run of anchor chars, ending
        // only at whitespace / a line break / a flow indicator. The old
        // "quoted anchor"/value-indicator-colon branches were a second grammar
        // drifting from granit (the fuzz family's root) and are gone.
        let anchors = extract_anchors("key: &a:b value");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "a:b");
        // A `"` is just a name char too; the run stops at the following space.
        let anchors = extract_anchors(r#"key: &"q value"#);
        assert_eq!(anchors[0].name, "\"q");
        // An empty run (nothing valid after `&`) is not an anchor.
        assert!(extract_anchors("key: & value").is_empty());
    }

    #[test]
    fn unterminated_quote_anchor_name_stops_at_line_end() {
        // libFuzzer `yaml_roundtrip` crash input: under the granit-aligned
        // maximal-run grammar the name is `"X-` (leading `"` is a name char,
        // the run stops at the CR) — emitted bare as `&"X- ` it re-scans to the
        // same run, so serialize/re-parse is stable. The end-to-end idempotence
        // assertion below is the real contract.
        let yaml = "&\"X-\r:";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "\"X-");
        let node = crate::parser::parse(yaml, pyrs_schema::types::Schema::Core).unwrap();
        let once = crate::serializer::to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap();
        assert_eq!(
            once,
            crate::serializer::to_yaml(&again),
            "first output: {once:?}"
        );
    }

    #[test]
    fn quoted_anchor_closing_quote_must_be_on_same_line() {
        // libFuzzer `yaml_roundtrip` (11 bytes `&"X-<CR>:&"X-<CR>`): the quoted
        // scan saw a `"` later in the buffer and read the name across the CR,
        // producing anchor `X-\r:&`. The serializer emitted it verbatim and the
        // re-parse grew one round (`:&" ":&\"X-"` -> adds a `'` wrap each time).
        // granit ends an anchor token at a line break, so a closing quote past
        // CR/LF must not qualify as a quoted anchor.
        let yaml = "&\"X-\r:&\"X-\r";
        for a in extract_anchors(yaml) {
            assert!(
                !a.name.contains('\r') && !a.name.contains('\n'),
                "anchor name crossed a line break: {:?}",
                a.name
            );
        }
        let node = crate::parser::parse(yaml, pyrs_schema::types::Schema::Core).unwrap();
        let once = crate::serializer::to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap();
        let twice = crate::serializer::to_yaml(&again);
        assert_eq!(once, twice, "not idempotent: {once:?} vs {twice:?}");
    }

    #[test]
    fn test_extract_anchor_with_colon() {
        let yaml = "key: &anchor:name value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor:name");
    }

    #[test]
    fn anchor_name_consumes_value_indicator_colon() {
        // granit consumes `:` as an anchor name char (its issue-14 test), so
        // `&&&&:` names the anchor `&&&:` (the run stops at the newline) and
        // `&x: 1` names it `x:` (the run stops at the space). The emit is bare
        // (`&name `), and a maximal-run re-scan reproduces the name exactly, so
        // the drift the old value-indicator special case fought is gone by
        // construction. `test_extract_anchor_with_colon` covers `&x:y`.
        let anchors = extract_anchors("&&&&:\n#&&&:&");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "&&&:");
        let anchors = extract_anchors("a: &x: 1");
        assert_eq!(anchors[0].name, "x:");
        let anchors = extract_anchors("a: &x:y 1");
        assert_eq!(anchors[0].name, "x:y");
    }

    #[test]
    fn test_extract_anchor_with_hash() {
        let yaml = "key: &anchor#name value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor#name");
    }

    #[test]
    fn test_extract_anchor_stops_at_flow_indicator() {
        let yaml = "key: &anchor{sub}";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor");
    }

    #[test]
    fn test_extract_anchor_stops_at_comma() {
        let yaml = "key: &anchor, next";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor");
    }
}
