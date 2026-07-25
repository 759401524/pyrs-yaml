use crate::ast::Chomping;

/// Unescape a double-quoted YAML string
pub fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('/') => result.push('/'),
                Some('0') => result.push('\0'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('e') => result.push('\x1B'),
                Some(' ') => result.push(' '),
                Some('\n') => {
                    // Line continuation - skip whitespace
                    while let Some(&next) = chars.clone().peekable().peek() {
                        if next.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                Some('u') => {
                    // Unicode escape: \uXXXX
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                    }
                }
                Some('U') => {
                    // Unicode escape: \UXXXXXXXX
                    let hex: String = chars.by_ref().take(8).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                    }
                }
                Some('x') => {
                    // Hex escape: \xXX
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(code_point) = u8::from_str_radix(&hex, 16) {
                        result.push(code_point as char);
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Detect chomping indicator from raw YAML text
/// The chomping indicator appears on the line before the block scalar content
/// Looks for |- or |+ or >- or >+ patterns
pub fn detect_chomping(yaml: &str, content_line: usize) -> Chomping {
    let lines: Vec<&str> = yaml.lines().collect();

    // Look at the line before the content for the block scalar indicator
    // The indicator could be on the same line as the key or on a previous line
    for check_line in (0..=content_line).rev() {
        if check_line >= lines.len() {
            continue;
        }
        let line_text = lines[check_line];

        // Look for | or > followed by - or +
        for (i, ch) in line_text.char_indices() {
            if ch == '|' || ch == '>' {
                let remaining = &line_text[i + 1..];
                if remaining.starts_with('-') {
                    return Chomping::Strip;
                } else if remaining.starts_with('+') {
                    return Chomping::Keep;
                }
                // Found the indicator without chomping, stop looking
                if remaining.is_empty() || remaining.starts_with(|c: char| c.is_whitespace()) {
                    return Chomping::Clip;
                }
            }
        }
    }

    Chomping::Clip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_newlines() {
        assert_eq!(unescape_double_quoted(r#"hello\nworld"#), "hello\nworld");
    }

    #[test]
    fn test_unescape_tabs() {
        assert_eq!(unescape_double_quoted(r"hello\tworld"), "hello\tworld");
    }

    #[test]
    fn test_unescape_unicode() {
        assert_eq!(unescape_double_quoted(r"\u0041"), "A");
    }

    #[test]
    fn test_unescape_hex() {
        assert_eq!(unescape_double_quoted(r"\x41"), "A");
    }

    #[test]
    fn test_unescape_backslash() {
        assert_eq!(unescape_double_quoted(r"hello\\world"), r"hello\world");
    }

    #[test]
    fn test_unescape_double_quote() {
        assert_eq!(unescape_double_quoted(r#"hello\"world"#), r#"hello"world"#);
    }

    #[test]
    fn test_unescape_line_continuation() {
        assert_eq!(
            unescape_double_quoted("hello\\\n  world"),
            "helloworld"
        );
    }

    #[test]
    fn test_detect_chomping_strip() {
        let yaml = "key: |-\n  content";
        let chomping = detect_chomping(yaml, 1);
        assert_eq!(chomping, Chomping::Strip);
    }

    #[test]
    fn test_detect_chomping_keep() {
        let yaml = "key: |+\n  content";
        let chomping = detect_chomping(yaml, 1);
        assert_eq!(chomping, Chomping::Keep);
    }

    #[test]
    fn test_detect_chomping_clip() {
        let yaml = "key: |\n  content";
        let chomping = detect_chomping(yaml, 1);
        assert_eq!(chomping, Chomping::Clip);
    }

    #[test]
    fn test_detect_chomping_folded_strip() {
        let yaml = "key: >-\n  content";
        let chomping = detect_chomping(yaml, 1);
        assert_eq!(chomping, Chomping::Strip);
    }

    #[test]
    fn test_detect_chomping_default() {
        let yaml = "key: value";
        let chomping = detect_chomping(yaml, 0);
        assert_eq!(chomping, Chomping::Clip);
    }
}
