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
            // 锚点提取：引号外 `&` 视为锚点
            if !in_single_quote && !in_double_quote && ch == '&' {
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
            is_string_char(&mut in_single_quote, &mut in_double_quote, &mut escaped, ch);
        }
    }

    anchors
}

/// Advance quote/escape state machine for a single character.
/// Returns `true` if the character was consumed (quote toggle or escape start).
fn is_string_char(
    in_single_quote: &mut bool,
    in_double_quote: &mut bool,
    escaped: &mut bool,
    ch: char,
) -> bool {
    if *escaped {
        *escaped = false;
        return true;
    }
    if ch == '\\' && (*in_single_quote || *in_double_quote) {
        *escaped = true;
        return true;
    }
    if ch == '\'' && !*in_double_quote {
        *in_single_quote = !*in_single_quote;
        return true;
    }
    if ch == '"' && !*in_single_quote {
        *in_double_quote = !*in_double_quote;
        return true;
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
