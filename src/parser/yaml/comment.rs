
/// A comment extracted from the raw YAML text
#[derive(Debug, Clone)]
pub struct RawComment {
    pub line: usize,
    pub col: usize,
    pub text: String,
    pub standalone: bool,
}

/// An anchor extracted from the raw YAML text
#[derive(Debug, Clone)]
pub struct RawAnchor {
    pub line: usize,
    pub col: usize,
    pub name: String,
}

/// Extract comments from raw YAML text by scanning line by line
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

/// Extract anchors from raw YAML text
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
