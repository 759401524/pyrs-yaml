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

/// 块标量头的探测结果：`|`/`>` 后可以带 chomping（`-`/`+`）与显式缩进
/// 指示器（`1`-`9`），两者可任意组合（如 `|-2`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockHeader {
    /// Chomping 指示符。
    pub chomping: Chomping,
    /// 显式缩进指示器（`|2` 中的 `2`）；`None` 表示源码未显式给出。
    pub indent: Option<u8>,
}

/// 解析紧跟块标量 sigil（`|`/`>`）之后的字节，判定它是否是一个合法的块头部，
/// 并从中取出 chomping 与缩进指示符。
///
/// 合法块头部的文法：sigil 后至多一个 `1`-`9` 数字与一个 `-`/`+`（顺序不限），
/// 其后只能再接空白或一个 `# 注释`，直至行末。任一不合规的字符都会让它被否定
/// （例如普通/带引号键里内嵌的 `|1:`，其 `1` 后面紧跟 `:` 便不合格）。
/// `from` 是 sigil 之后第一个字节，`line_end` 是该头部行的结束偏移（不含换行）。
fn parse_block_header_tail(bytes: &[u8], from: usize, line_end: usize) -> Option<BlockHeader> {
    let mut chomping = Chomping::Clip;
    let mut indent: Option<u8> = None;
    let mut tail = from;
    while tail < line_end {
        match bytes[tail] {
            b'1'..=b'9' if indent.is_none() => {
                indent = Some(bytes[tail] - b'0');
                tail += 1;
            }
            b'-' if matches!(chomping, Chomping::Clip) => {
                chomping = Chomping::Strip;
                tail += 1;
            }
            b'+' if matches!(chomping, Chomping::Clip) => {
                chomping = Chomping::Keep;
                tail += 1;
            }
            b' ' | b'\t' => tail += 1,
            // A `#` begins a trailing comment; everything after it is ignored,
            // so the sigil is a valid header.
            b'#' => return Some(BlockHeader { chomping, indent }),
            // Any other byte means this sigil is not a block header.
            _ => return None,
        }
    }
    Some(BlockHeader { chomping, indent })
}

/// 依据块标量**内容首字节**在源码中的位置，定位并解析它的头部（`|`/`>`），
/// 返回 chomping 与显式缩进指示符。
///
/// `content_start` 是 granit 报告给该块标量 span 的**内容首字节**偏移——块标量的
/// span 落在内容上而非头部行，头部 `|`/`>` 恒在内容之前的那一物理行。本函数**按
/// 字节**回退到头部所在物理行，再取该行上第一个尾部符合块头部文法的 `|`/`>`。
///
/// 相比早先“按解析器行号向上逐行扫描”的做法，这里有三个由构造保证的性质：
/// - **对 `\r` 免疫**：granit 只把 `\n` 计为换行，源码 `\r` 会把头部行与内容行折进
///   同一逻辑行，基于行号的向上扫描会落到错误的行、在 emit/重读之间翻转 `|N`↔`|`
///   （libFuzzer `yaml_roundtrip` crash-bdf3f15f）。按字节读取源码真实布局则不受影响。
/// - **绝不触碰内容行**：只看内容之前的那一行，内容行里出现的 `|`/`>` 不会被误判为
///   头部（同一 crash 家族的另一形态）。
/// - **尾部文法而非位置启发式**：带引号或普通键里内嵌的 `|`（如 `k:yam  |1: |2` 键内
///   的 `|1`，其后紧跟 `:`）因尾部不合规而被否定，唯一能延伸到行末（或注释）的合法
///   sigil 才是真头部（libFuzzer `yaml_roundtrip` crash-cad17b2b）。头部至多一个尾部
///   合法的 sigil，故“第一个”即“唯一一个”，注释里内嵌的 `|` 也不会被误取。
///
/// 时间复杂度为 O(头部行宽)，不再随块标量所在深度做二次方的 `lines().nth()` 扫描。
///
/// # Arguments
/// * `yaml` - 原始 YAML 文本。
/// * `content_start` - 块标量**内容首字节**的字节偏移。
///
/// # Returns
/// 探测到的块标量头；未找到合法 `|`/`>` 头部时返回默认值（`Clip` + 无缩进指示器）。
///
/// # Examples
/// ```rust
/// use pyrs_yaml_core::parser::yaml::scalar::detect_block_header;
/// // "yaml: |2\n  x\n"：内容首字节 `x` 位于偏移 10。
/// let h = detect_block_header("yaml: |2\n  x\n", 10);
/// assert_eq!(h.indent, Some(2));
/// ```
pub fn detect_block_header(yaml: &str, content_start: usize) -> BlockHeader {
    let bytes = yaml.as_bytes();
    let content_start = content_start.min(bytes.len());

    // Back up over the first content line's leading indentation, then over any
    // (entirely blank) lines above it, to the last non-blank byte before the
    // content — which is the end of the header line.
    let mut end = content_start;
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    while end > 0 && matches!(bytes[end - 1], b'\n' | b'\r') {
        end -= 1;
        while end > 0 && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
            end -= 1;
        }
    }

    // Start of the header's physical line: just after the preceding line break
    // (granit's `\n`-only rule aside, we split on either break for byte truth).
    let line_start = (0..end)
        .rev()
        .find(|&k| bytes[k] == b'\n' || bytes[k] == b'\r')
        .map_or(0, |k| k + 1);

    // `|` (0x7C) and `>` (0x3E) are ASCII and never occur inside a multi-byte
    // UTF-8 sequence, so a byte scan locates every candidate sigil safely.
    for idx in line_start..end {
        if bytes[idx] == b'|' || bytes[idx] == b'>' {
            // The first sigil whose tail reaches the line end (or a comment) as
            // a valid header is the block header — at most one can qualify.
            if let Some(header) = parse_block_header_tail(bytes, idx + 1, end) {
                return header;
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
    fn detect_block_header_literal_clip() {
        let s = "key: |\n  content";
        assert_eq!(
            detect_block_header(s, s.find("content").unwrap()).chomping,
            Chomping::Clip
        );
    }

    #[test]
    fn detect_block_header_strip() {
        let s = "key: |-\n  content";
        assert_eq!(
            detect_block_header(s, s.find("content").unwrap()).chomping,
            Chomping::Strip
        );
    }

    #[test]
    fn detect_block_header_keep() {
        let s = "key: |+\n  content";
        assert_eq!(
            detect_block_header(s, s.find("content").unwrap()).chomping,
            Chomping::Keep
        );
    }

    #[test]
    fn detect_block_header_folded_strip() {
        let s = "key: >-\n  content";
        assert_eq!(
            detect_block_header(s, s.find("content").unwrap()).chomping,
            Chomping::Strip
        );
    }

    #[test]
    fn detect_block_header_explicit_indent() {
        let s = "yaml: |2\n  x\n";
        assert_eq!(detect_block_header(s, s.find('x').unwrap()).indent, Some(2));
    }

    #[test]
    fn detect_block_header_indent_and_chomping() {
        let s = "k: |3-\n  v\n";
        let h = detect_block_header(s, s.find('v').unwrap());
        assert_eq!(h.indent, Some(3));
        assert_eq!(h.chomping, Chomping::Strip);
    }

    #[test]
    fn detect_block_header_skips_embedded_sigil_in_key() {
        // crash-cad17b2b shape: plain key `k:yam  |1`, real header `|2`. The
        // embedded `|1` fails the tail grammar (a `:` follows), the header wins.
        let s = "k:yam  |1: |2\r  x|&y";
        let h = detect_block_header(s, s.find('x').unwrap());
        assert_eq!(h.indent, Some(2));
        assert_eq!(h.chomping, Chomping::Clip);
    }

    #[test]
    fn detect_block_header_ignores_content_line_sigil() {
        // crash-bdf3f15f shape: header `|2` above a content line holding `|`.
        // Byte anchoring never scans the content line, so the `|` in `x|y` is
        // not mistaken for the header.
        let s = "k: |2\n  x|y\n";
        assert_eq!(detect_block_header(s, s.find('x').unwrap()).indent, Some(2));
    }

    #[test]
    fn detect_block_header_trailing_comment_not_confused() {
        // The `|2` sits in a comment; the real header is the first `|` (Clip,
        // no indent). Scanning stops at the first sigil with a valid tail.
        let s = "key: | # a |2\n  content";
        let h = detect_block_header(s, s.find("content").unwrap());
        assert_eq!(h.indent, None);
        assert_eq!(h.chomping, Chomping::Clip);
    }

    #[test]
    fn detect_block_header_quoted_key_pipe() {
        // crash-cad17b2b test shape: `|1` inside a double-quoted key, the real
        // header is the trailing `|` (Clip, no indent).
        let s = "\"k:yam  |1\": |\n  x";
        let h = detect_block_header(s, s.find('x').unwrap());
        assert_eq!(h.indent, None);
        assert_eq!(h.chomping, Chomping::Clip);
    }

    #[test]
    fn detect_block_header_blank_lines_before_content() {
        let s = "key: |-\n\n\n  content";
        assert_eq!(
            detect_block_header(s, s.find("content").unwrap()).chomping,
            Chomping::Strip
        );
    }

    #[test]
    fn detect_block_header_no_sigil_returns_default() {
        let s = "key: value";
        let h = detect_block_header(s, s.find("value").unwrap());
        assert_eq!(h, BlockHeader::default());
    }
}
