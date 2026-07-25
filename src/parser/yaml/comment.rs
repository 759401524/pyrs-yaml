use crate::ast::Comment;

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
                let comment_text = line[col_idx + 1..].trim().to_string();
                let is_standalone = line[..col_idx].trim().is_empty();
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
                let anchor_start = col_idx + 1;
                let mut anchor_name = String::new();
                for (_i, c) in line[anchor_start..].char_indices() {
                    if c.is_alphanumeric() || c == '_' || c == '-' {
                        anchor_name.push(c);
                    } else {
                        break;
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

/// Find an inline comment on the same line at a column after `after_col`
pub fn find_inline_comment(
    comments: &[RawComment],
    start_idx: &mut usize,
    line: usize,
    after_col: usize,
) -> Option<Comment> {
    while *start_idx < comments.len() {
        let c = &comments[*start_idx];
        if c.line > line {
            return None;
        }
        if c.line < line {
            *start_idx += 1;
            continue;
        }
        // Same line
        if c.col > after_col && !c.standalone {
            let comment = Comment {
                text: c.text.clone(),
                standalone: false,
            };
            *start_idx += 1;
            return Some(comment);
        }
        *start_idx += 1;
    }
    None
}

/// Find the next standalone comment before a given line
pub fn find_standalone_comment_before(
    comments: &[RawComment],
    start_idx: &mut usize,
    before_line: usize,
) -> Option<Comment> {
    let mut result = None;
    while *start_idx < comments.len() {
        let c = &comments[*start_idx];
        if c.line >= before_line {
            break;
        }
        if c.standalone {
            result = Some(Comment {
                text: c.text.clone(),
                standalone: true,
            });
        }
        *start_idx += 1;
    }
    result
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
    fn test_find_inline_comment() {
        let comments = vec![
            RawComment { line: 0, col: 10, text: "inline".to_string(), standalone: false },
            RawComment { line: 1, col: 0, text: "standalone".to_string(), standalone: true },
        ];
        let mut idx = 0;
        let result = find_inline_comment(&comments, &mut idx, 0, 5);
        assert!(result.is_some());
        let c = result.unwrap();
        assert_eq!(c.text, "inline");
        assert!(!c.standalone);
    }

    #[test]
    fn test_find_standalone_comment_before() {
        let comments = vec![
            RawComment { line: 0, col: 0, text: "top".to_string(), standalone: true },
            RawComment { line: 2, col: 0, text: "bottom".to_string(), standalone: true },
        ];
        let mut idx = 0;
        let result = find_standalone_comment_before(&comments, &mut idx, 2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "top");
    }
}
