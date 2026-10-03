use crate::ast::Chomping;

/// 反转义双引号 YAML 字符串中的转义序列。
///
/// 支持的转义序列：`\n`, `\r`, `\t`, `\\`, `\"`, `\/`, `\0`, `\a`, `\b`,
/// `\f`, `\e`, `\ `（空格），行续接（`\` + 换行），`\uXXXX`（Unicode），`\UXXXXXXXX`（Unicode），`\xXX`（十六进制）。
///
/// # Arguments
/// * `s` - 包含转义序列的字符串（不含外层双引号）。
///
/// # Returns
/// 反转义后的字符串。
///
/// # Examples
/// ```rust
/// use pyrs_yaml_core::parser::yaml::scalar::unescape_double_quoted;
/// assert_eq!(unescape_double_quoted(r#"hello\nworld"#), "hello\nworld");
/// assert_eq!(unescape_double_quoted(r"\u0041"), "A");
/// ```
pub fn unescape_double_quoted(s: &str) -> String {
    // Every transformation below keys off a backslash; with none present the
    // output equals the input, so skip the per-char state machine entirely
    // (common case: quoted scalars without escapes).
    if !s.contains('\\') {
        return s.to_string();
    }
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
                    while let Some(&next) = chars.clone().peekable().peek() {
                        if next.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    result.push_str(&unescape_unicode_hex(&hex, 4));
                }
                Some('U') => {
                    let hex: String = chars.by_ref().take(8).collect();
                    result.push_str(&unescape_unicode_hex(&hex, 8));
                }
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    result.push_str(&unescape_hex_escape(&hex));
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

/// Decode a hex string as a Unicode code point and return the character.
fn unescape_unicode_hex(hex: &str, expected_len: usize) -> String {
    if hex.len() != expected_len {
        return String::new();
    }
    match u32::from_str_radix(hex, 16) {
        Ok(code_point) => char::from_u32(code_point).map_or(String::new(), |c| c.to_string()),
        Err(_) => String::new(),
    }
}

/// Decode a two-character hex escape and return the character.
fn unescape_hex_escape(hex: &str) -> String {
    if hex.len() != 2 {
        return String::new();
    }
    match u8::from_str_radix(hex, 16) {
        Ok(byte) => (byte as char).to_string(),
        Err(_) => String::new(),
    }
}

/// 从原始 YAML 文本中检测块标量的 chomping 指示符。
///
/// 从 `content_line`（0 起始）向上扫描，查找 `|-`、`|+`、`>-`、`>+` 或 `|`、`>` 模式。
///
/// # Arguments
/// * `yaml` - 原始 YAML 文本。
/// * `content_line` - 块标量内容起始行号（**0 起始**）。
///
/// # Returns
/// 检测到的 `Chomping` 值：`Strip`（`-`）、`Keep`（`+`）或默认 `Clip`。
pub fn detect_chomping(yaml: &str, content_line: usize) -> Chomping {
    // Look at the line before the content for the block scalar indicator
    // The indicator could be on the same line as the key or on a previous
    // line. Lines are pulled lazily (nth) instead of materializing the whole
    // document into a Vec on every block scalar.
    for check_line in (0..=content_line).rev() {
        let Some(line_text) = yaml.lines().nth(check_line) else {
            continue;
        };

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

/// 块标量头的探测结果：`|`/`>` 后可以带 chomping（`-`/`+`）与显式缩进
/// 指示器（`1`-`9`），两者可任意组合（如 `|-2`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockHeader {
    /// Chomping 指示符。
    pub chomping: Chomping,
    /// 显式缩进指示器（`|2` 中的 `2`）；`None` 表示源码未显式给出。
    pub indent: Option<u8>,
}

/// 从原始 YAML 文本中检测块标量头部的 chomping 与显式缩进指示器。
///
/// 与 [`detect_chomping`] 同一套行扫描策略（从 `content_line` 向上找到最近
/// 的 `|`/`>` 头），只是把缩进数字也一并带出来。合成扫描而不是两次调用，
/// 避免对同一个块标量重复走一遍行迭代器。
///
/// # Arguments
/// * `yaml` - 原始 YAML 文本。
/// * `content_line` - 块标量内容起始行号（**0 起始**）。
///
/// # Returns
/// 探测到的块标量头；未找到 `|`/`>` 时返回默认值（`Clip` + 无缩进指示器）。
///
/// # Examples
/// ```rust
/// use pyrs_yaml_core::parser::yaml::scalar::detect_block_header;
/// let h = detect_block_header("yaml: |2\n  x\n", 1);
/// assert_eq!(h.indent, Some(2));
/// ```
pub fn detect_block_header(yaml: &str, content_line: usize) -> BlockHeader {
    // Indentation (leading-whitespace width) of the block's first content line.
    // The header line always sits at the *parent* indentation, strictly shallower
    // than the block content, so a valid header candidate must be less indented
    // than this. Without the guard the upward scan can land on a *content* line
    // that itself contains `|`/`>`: granit counts only `\n` as a line break, so a
    // source `\r` keeps `key: |2` and a `|`-bearing content line on one logical
    // line, and normalizing `\r`->`\n` on emit shifts which line the scan hits -
    // flipping `|2`/`|` between rounds (libFuzzer `yaml_roundtrip` crash-bdf3f15f).
    let content_indent = yaml
        .lines()
        .nth(content_line)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    for check_line in (0..=content_line).rev() {
        let Some(line_text) = yaml.lines().nth(check_line) else {
            continue;
        };
        // Lines at or deeper than the block content are content (including the
        // first content line itself); the header is strictly shallower.
        if line_text.len() - line_text.trim_start().len() >= content_indent {
            continue;
        }

        for (i, ch) in line_text.char_indices() {
            if ch == '|' || ch == '>' {
                let mut chomping = Chomping::Clip;
                let mut indent = None;
                // Walk the (at most two) indicator characters after the
                // header. A digit is the explicit indentation indicator; a
                // `-`/`+` is chomping. Order is irrelevant because each digit
                // is a single byte in the 1-9 range.
                for ind_ch in line_text[i + 1..].chars() {
                    match ind_ch {
                        '-' => chomping = Chomping::Strip,
                        '+' => chomping = Chomping::Keep,
                        '1'..='9' => indent = Some(ind_ch as u8 - b'0'),
                        // End of the header: a space, a comment, or EOL.
                        _ => return BlockHeader { chomping, indent },
                    }
                }
                // Header ran to the end of the line.
                return BlockHeader { chomping, indent };
            }
        }
    }

    BlockHeader::default()
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
        assert_eq!(unescape_double_quoted("hello\\\n  world"), "helloworld");
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
