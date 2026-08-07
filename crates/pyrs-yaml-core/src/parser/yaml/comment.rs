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
    let mut anchors = Vec::new();

    for (line_idx, line) in yaml.lines().enumerate() {
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for (col_idx, ch) in line.char_indices() {
            // 锚点提取：引号外 `&` 视为锚点
            if !in_single_quote && !in_double_quote && ch == '&' {
                let rest = &line[col_idx + 1..];
                if let Some(anchor_name) = scan_anchor_name(rest) {
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
    let mut chars = rest.char_indices();
    let first = chars.next()?.1;

    let mut anchor_name = String::new();
    if first == '"' {
        for (_, c) in chars {
            if c == '"' {
                break;
            }
            anchor_name.push(c);
        }
    } else if is_valid_anchor_char(first) {
        anchor_name.push(first);
        for (_, c) in chars {
            if is_valid_anchor_char(c) {
                anchor_name.push(c);
            } else {
                break;
            }
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
    fn test_extract_anchor_with_colon() {
        let yaml = "key: &anchor:name value";
        let anchors = extract_anchors(yaml);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "anchor:name");
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
