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
                for (i, c) in line[anchor_start..].char_indices() {
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
