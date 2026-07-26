
/// 从原始 YAML 文本中提取的注释信息。
#[derive(Debug, Clone)]
pub struct RawComment {
    /// 注释所在行（0 起始）
    pub line: usize,
    /// 注释起始列（0 起始，`#` 字符的位置）
    pub col: usize,
    /// 注释文本（不含 `#` 前缀和前导空格）
    pub text: String,
    /// `true` 表示独立行注释（行首仅有注释），`false` 表示行尾注释
    pub standalone: bool,
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

/// 从原始 YAML 文本中逐行扫描提取所有注释。
///
/// 正确处理单引号和双引号内的 `#` 字符（视为字符串内容而非注释）。
///
/// # Arguments
/// * `yaml` - 原始 YAML 文本。
///
/// # Returns
/// 按行号和列号排序的 `RawComment` 列表。
pub fn extract_comments(yaml: &str) -> Vec<RawComment> {
    let mut comments = Vec::new();

    for (line_idx, line) in yaml.lines().enumerate() {
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for (col_idx, ch) in line.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' && (in_single_quote || in_double_quote) {
                escaped = true;
                continue;
            }
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
                continue;
            }
            if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
                continue;
            }
            if ch == '#' && !in_single_quote && !in_double_quote {
                let comment_text = line.get(col_idx + 1..).unwrap_or("").trim().to_string();
                let is_standalone = line.get(..col_idx).unwrap_or("").trim().is_empty();
                comments.push(RawComment {
                    line: line_idx,
                    col: col_idx,
                    text: comment_text,
                    standalone: is_standalone,
                });
                break;
            }
        }
    }

    comments
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
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' && (in_single_quote || in_double_quote) {
                escaped = true;
                continue;
            }
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
                continue;
            }
            if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
                continue;
            }
            if ch == '&' && !in_single_quote && !in_double_quote {
                // Found an anchor
                let anchor_start_byte = col_idx + ch.len_utf8();
                let rest = &line[anchor_start_byte..];

                let mut anchor_name = String::new();
                let mut chars = rest.char_indices();

                // Check for quoted anchor name: &"name"
                if let Some((_, first)) = chars.next() {
                    if first == '"' {
                        // Quoted anchor: scan until closing quote
                        for (_, c) in chars.by_ref() {
                            if c == '"' {
                                break;
                            }
                            anchor_name.push(c);
                        }
                    } else if is_valid_anchor_char(first) {
                        // Unquoted anchor: scan until invalid character
                        anchor_name.push(first);
                        for (_, c) in chars.by_ref() {
                            if is_valid_anchor_char(c) {
                                anchor_name.push(c);
                            } else {
                                break;
                            }
                        }
                    }
                }

                if !anchor_name.is_empty() {
                    anchors.push(RawAnchor {
                        line: line_idx,
                        col: col_idx,
                        name: anchor_name,
                    });
                }
            }
        }
    }

    anchors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_inline_comment() {
        let yaml = "key: value  # comment";
        let comments = extract_comments(yaml);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "comment");
        assert!(!comments[0].standalone);
        assert_eq!(comments[0].line, 0);
    }

    #[test]
    fn test_extract_standalone_comment() {
        let yaml = "# standalone\nkey: value";
        let comments = extract_comments(yaml);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "standalone");
        assert!(comments[0].standalone);
        assert_eq!(comments[0].line, 0);
    }

    #[test]
    fn test_comment_in_string_ignored() {
        let yaml = "key: 'has # inside'";
        let comments = extract_comments(yaml);
        assert!(comments.is_empty());
    }

    #[test]
    fn test_comment_in_double_quoted_string_ignored() {
        let yaml = r#"key: "has # inside""#;
        let comments = extract_comments(yaml);
        assert!(comments.is_empty());
    }

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
