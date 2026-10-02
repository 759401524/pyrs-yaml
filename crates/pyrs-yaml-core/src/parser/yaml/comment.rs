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
            if !in_single_quote && !in_double_quote && ch == '&' && col_idx >= token_end {
                let rest = &line[col_idx + 1..];
                if let Some(anchor_name) = scan_anchor_name(rest) {
                    token_end = col_idx + 1 + anchor_name.len();
                    anchors.push(RawAnchor {
                        line: line_idx,
                        col: col_idx,
                        name: anchor_name,
                    });
                }
            }
            if is_string_char(&mut in_single_quote, &mut in_double_quote, &mut escaped, ch) {
                continue;
            }
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
/// Handles both quoted (`&"name"`) and unquoted (`&name`) forms.
fn scan_anchor_name(rest: &str) -> Option<String> {
    let mut it = rest.chars();
    let first = it.next()?;

    let mut anchor_name = String::new();
    if first == '"' {
        if rest[1..].contains('"') {
            // True quoted form (`&"name"`): the closing quote bounds the name,
            // spaces included.
            for c in it {
                if c == '"' {
                    break;
                }
                anchor_name.push(c);
            }
        } else {
            // Unterminated quote: granit never reads this as a quoted anchor,
            // so collecting to end-of-line smuggled CRs and colons into names
            // the serializer emitted verbatim — `&"X-<CR>:` yielded anchor
            // `X-\r:`, which re-parsed to anchor `X-` and broke round-trip
            // idempotence (libFuzzer `yaml_roundtrip`). Stop where granit's
            // own unquoted anchor token would stop.
            for c in it {
                if c == '"' || !is_valid_anchor_char(c) {
                    break;
                }
                anchor_name.push(c);
            }
        }
    } else if is_valid_anchor_char(first) {
        // ':' continues the name only when a non-space follows: `key:
        // &anchor:name value` carries it inside the name (granit agrees),
        // while `:` + space/EOL is the value indicator — taking it produced
        // names like `&&&:` that the emitted text cannot survive: re-parse
        // reads `&&&&: v` as anchor `&&&` plus an indicator, drifting one
        // character per serialize round (libFuzzer `yaml_roundtrip`, 12 bytes
        // `&&&&:<LF>#&&&:&`). Scan by byte offset so the lookahead can peek
        // at the raw remainder without moving the iterator.
        let mut pos = first.len_utf8();
        if first == ':' && next_is_space_or_end(&rest[pos..]) {
            return None;
        }
        anchor_name.push(first);
        while let Some(c) = rest[pos..].chars().next() {
            if !is_valid_anchor_char(c) {
                break;
            }
            let after = pos + c.len_utf8();
            if c == ':' && next_is_space_or_end(&rest[after..]) {
                break;
            }
            anchor_name.push(c);
            pos = after;
        }
    } else {
        return None;
    }

    if anchor_name.is_empty() {
        None
    } else {
        Some(anchor_name)
    }
}

/// True when the anchor scan has no more name material at this position: the
/// remainder is empty (end of line) or starts with a space/tab.
fn next_is_space_or_end(rest: &str) -> bool {
    rest.is_empty() || rest.starts_with([' ', '\t'])
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
    fn test_extract_quoted_anchor() {
        let yaml = r#"key: &"quoted anchor" value"#;
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "quoted anchor");
    }

    #[test]
    fn unterminated_quote_never_swallows_line_end() {
        // libFuzzer `yaml_roundtrip` crash (6 bytes): the quoted scan ran to
        // end of line without a closing quote, naming the anchor `X-\r:`;
        // the verbatim `&X-\r:` emission then re-parsed as `X-` — the
        // serialize/re-parse loop drifted. Same schema entry the fuzz target
        // uses (`Schema::Core`), asserting output idempotence end to end.
        let yaml = "&\"X-\r:";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "X-");
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
    fn test_extract_anchor_with_colon() {
        let yaml = "key: &anchor:name value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor:name");
    }

    #[test]
    fn value_indicator_colon_is_not_anchor_name() {
        // libFuzzer `yaml_roundtrip` (12 bytes): `&&&&:` + LF — the colon is
        // followed by end-of-line, so it is the value indicator, not name
        // material. Taking it produced `&&&:` which the emitted text cannot
        // re-parse as one token, drifting a character per serialize round.
        let anchors = extract_anchors("&&&&:\n#&&&:&");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "&&&");
        // Colon + space ends the name the same way…
        let anchors = extract_anchors("a: &x: 1");
        assert_eq!(anchors[0].name, "x");
        // …while a non-space after the colon keeps it inside the name
        // (covered by `test_extract_anchor_with_colon` too).
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
