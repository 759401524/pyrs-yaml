//! The byte scanner: `&str` -> a flat token stream.
//!
//! Every `<` is classified *before* its content is read. That is the security
//! boundary: `<!DOCTYPE` and `<!ENTITY` are refused at this step, so there is
//! no internal-subset buffer, no declaration table and no expansion phase for
//! a billion-laughs bomb to run in. There is no I/O here — the scanner takes
//! `&str` and returns tokens or a refusal, nothing else.
//!
//! Entity decoding happens once, in text and attribute values, on the raw
//! slice between markup. The decoded characters are appended to a fresh
//! `String` and never re-scanned, so `&lt;b&gt;` is the string `<b>` and never
//! becomes an element.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::XmlError;

/// Attributes in document order, with values already decoded.
type Attributes = Vec<(String, String)>;

/// A start tag's name, attributes and self-closing flag.
type StartTag = (String, Attributes, bool);

/// One lexical token. `byte_range` is into the BOM-stripped input, and
/// `line`/`column` are 1-based (column counted in characters).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Half-open byte range in the BOM-stripped input.
    pub byte_range: core::ops::Range<usize>,
    /// 1-based line the token starts on.
    pub line: usize,
    /// 1-based column the token starts on.
    pub column: usize,
}

/// The token vocabulary of the XML subset this engine reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// `<name ...>` or `<name .../>`.
    Start {
        /// The QName, prefixes kept verbatim.
        name: String,
        /// Attributes in document order, values entity-decoded.
        attrs: Vec<(String, String)>,
        /// Whether the tag ended with `/>`.
        self_closing: bool,
    },
    /// `</name>`.
    End {
        /// The QName, prefixes kept verbatim.
        name: String,
    },
    /// Entity-decoded character data.
    Text(String),
    /// Comment body (the `<!--` and `-->` are not included).
    Comment(String),
    /// Processing instruction target and data.
    ProcessingInstruction {
        /// The PI target.
        target: String,
        /// Everything after the target and its separating whitespace.
        data: String,
    },
    /// CDATA body, emitted as character data (delimiters normalised away).
    CData(String),
}

/// Scan an XML document into tokens. A leading UTF-8 BOM is accepted and
/// stripped; an XML declaration is validated and not emitted as a token.
///
/// DTD and entity declarations are refused here, at the `<!` boundary, before
/// a single byte of their body is read.
pub fn scan(text: &str) -> Result<Vec<Token>, XmlError> {
    Scanner::new(text).scan_all()
}

/// 1-based `(line, column)` of a byte offset, column counted in characters.
/// Used on demand so the scanner keeps a single source of truth for position.
fn position_of(text: &str, offset: usize) -> (usize, usize) {
    let prefix = &text[..offset.min(text.len())];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = prefix[line_start..].chars().count() + 1;
    (line, column)
}

/// XML 1.0 whitespace, deliberately the four characters of the production.
pub(crate) fn is_xml_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

fn is_xml_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// XML 1.0 (5th ed.) `NameStartChar`.
pub(crate) fn is_name_start(c: char) -> bool {
    let u = c as u32;
    c == ':'
        || c == '_'
        || c.is_ascii_alphabetic()
        || matches!(u,
            0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x2FF | 0x370..=0x37D |
            0x37F..=0x1FFF | 0x200C..=0x200D | 0x2070..=0x218F |
            0x2C00..=0x2FEF | 0x3001..=0xD7FF | 0xF900..=0xFDCF |
            0xFDF0..=0xFFFD | 0x10000..=0xEFFFF)
}

/// XML 1.0 (5th ed.) `NameChar`.
pub(crate) fn is_name_char(c: char) -> bool {
    let u = c as u32;
    is_name_start(c) || matches!(u, 45 | 46 | 48..=57 | 0xB7 | 0x300..=0x36F | 0x203F..=0x2040)
}

fn is_valid_xml_char(code: u32) -> bool {
    code == 0x9
        || code == 0xA
        || code == 0xD
        || (0x20..=0xD7FF).contains(&code)
        || (0xE000..=0xFFFD).contains(&code)
        || (0x10000..=0x10FFFF).contains(&code)
}

fn predefined_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Decode the five predefined and numeric references in one pass. `base` is
/// the absolute byte offset of `s` inside the full document, so a refusal can
/// name the real line/column. Decoded text is never re-scanned.
fn decode_entities(s: &str, base: usize, full: &str) -> Result<String, XmlError> {
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < s.len() {
        let ch = s[i..]
            .chars()
            .next()
            .expect("non-empty slice has a first char");
        if ch != '&' {
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        let (line, column) = position_of(full, base + i);
        let after = &s[i + 1..];
        let Some(semi) = after.find(';') else {
            return Err(XmlError::UndeclaredEntity {
                name: String::new(),
                line,
                column,
            });
        };
        let name = &after[..semi];
        if let Some(body) = name.strip_prefix('#') {
            let (radix, digits) = match body.strip_prefix('x').or_else(|| body.strip_prefix('X')) {
                Some(hex) => (16u32, hex),
                None => (10u32, body),
            };
            let code = u32::from_str_radix(digits, radix).ok();
            let decoded = code
                .filter(|&n| is_valid_xml_char(n))
                .and_then(char::from_u32);
            match decoded {
                Some(c) => out.push(c),
                None => {
                    return Err(XmlError::InvalidCharacterReference {
                        text: alloc::format!("&{name};"),
                        line,
                        column,
                    });
                }
            }
        } else if !name.is_empty() && name.chars().all(is_name_char) {
            match predefined_entity(name) {
                Some(c) => out.push(c),
                None => {
                    return Err(XmlError::UndeclaredEntity {
                        name: name.to_string(),
                        line,
                        column,
                    });
                }
            }
        } else {
            return Err(XmlError::UndeclaredEntity {
                name: name.to_string(),
                line,
                column,
            });
        }
        i += 1 + semi + 1;
    }
    Ok(out)
}

struct Scanner<'a> {
    /// The document with any leading BOM removed.
    text: &'a str,
    /// Current byte offset into `text`.
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        Self { text, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.rest().starts_with(needle)
    }

    fn byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn at(&self, offset: usize) -> (usize, usize) {
        position_of(self.text, offset)
    }

    fn here(&self) -> (usize, usize) {
        self.at(self.pos)
    }

    fn skip_whitespace(&mut self) {
        while self.byte().is_some_and(is_xml_whitespace_byte) {
            self.pos += 1;
        }
    }

    fn scan_all(mut self) -> Result<Vec<Token>, XmlError> {
        let mut tokens = Vec::new();
        while self.pos < self.text.len() {
            if self.byte() != Some(b'<') {
                self.scan_text(&mut tokens)?;
                continue;
            }
            let start = self.pos;
            let (line, column) = self.here();

            // The declaration is only a declaration at offset zero. Anywhere
            // else `xml` is just a PI target, which §4.3 refuses.
            if start == 0
                && self.starts_with("<?xml")
                && self
                    .text
                    .as_bytes()
                    .get(5)
                    .is_some_and(|b| is_xml_whitespace_byte(*b))
            {
                self.scan_declaration()?;
                continue;
            }
            if self.starts_with("<!--") {
                let body = self.scan_comment(start)?;
                tokens.push(Token {
                    kind: TokenKind::Comment(body),
                    byte_range: start..self.pos,
                    line,
                    column,
                });
            } else if self.starts_with("<![CDATA[") {
                let body = self.scan_cdata(start)?;
                tokens.push(Token {
                    kind: TokenKind::CData(body),
                    byte_range: start..self.pos,
                    line,
                    column,
                });
            } else if self.starts_with("</") {
                let name = self.scan_end_tag(start)?;
                tokens.push(Token {
                    kind: TokenKind::End { name },
                    byte_range: start..self.pos,
                    line,
                    column,
                });
            } else if self.starts_with("<?") {
                let (target, data) = self.scan_pi(start)?;
                tokens.push(Token {
                    kind: TokenKind::ProcessingInstruction { target, data },
                    byte_range: start..self.pos,
                    line,
                    column,
                });
            } else if self.starts_with("<!") {
                // Classification before content: no DTD body is ever read.
                let after = &self.text[start + 2..];
                if after.len() >= 7 && after.as_bytes()[..7].eq_ignore_ascii_case(b"DOCTYPE") {
                    return Err(XmlError::DtdDeclared { line, column });
                }
                if after.len() >= 6 && after.as_bytes()[..6].eq_ignore_ascii_case(b"ENTITY") {
                    return Err(XmlError::EntityDeclaration { line, column });
                }
                return Err(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: "unsupported markup declaration".to_string(),
                });
            } else {
                let (name, attrs, self_closing) = self.scan_start_tag(start)?;
                tokens.push(Token {
                    kind: TokenKind::Start {
                        name,
                        attrs,
                        self_closing,
                    },
                    byte_range: start..self.pos,
                    line,
                    column,
                });
            }
        }
        Ok(tokens)
    }

    fn scan_text(&mut self, tokens: &mut Vec<Token>) -> Result<(), XmlError> {
        let start = self.pos;
        let (line, column) = self.here();
        let end = self
            .rest()
            .find('<')
            .map(|i| self.pos + i)
            .unwrap_or(self.text.len());
        let raw = &self.text[start..end];
        let decoded = decode_entities(raw, start, self.text)?;
        self.pos = end;
        if !decoded.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Text(decoded),
                byte_range: start..end,
                line,
                column,
            });
        }
        Ok(())
    }

    /// Consume `<!-- ... -->`, refusing `--` inside the body.
    fn scan_comment(&mut self, start: usize) -> Result<String, XmlError> {
        let (line, column) = self.at(start);
        self.pos += 4;
        let end =
            self.rest()
                .find("-->")
                .map(|i| self.pos + i)
                .ok_or(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: "unterminated comment".to_string(),
                })?;
        let body = &self.text[self.pos..end];
        if body.contains("--") {
            return Err(XmlError::MalformedMarkup {
                line,
                column,
                detail: "double hyphen inside comment".to_string(),
            });
        }
        let body = body.to_string();
        self.pos = end + 3;
        Ok(body)
    }

    /// Consume `<![CDATA[ ... ]]>`; the body is character data as written.
    fn scan_cdata(&mut self, start: usize) -> Result<String, XmlError> {
        let (line, column) = self.at(start);
        self.pos += 9;
        let end =
            self.rest()
                .find("]]>")
                .map(|i| self.pos + i)
                .ok_or(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: "unterminated CDATA section".to_string(),
                })?;
        let body = self.text[self.pos..end].to_string();
        self.pos = end + 3;
        Ok(body)
    }

    /// Consume `</name>`, with no whitespace allowed before the name.
    fn scan_end_tag(&mut self, start: usize) -> Result<String, XmlError> {
        let (line, column) = self.at(start);
        self.pos += 2;
        let name = self.read_name().ok_or(XmlError::MalformedMarkup {
            line,
            column,
            detail: "end tag without a name".to_string(),
        })?;
        self.skip_whitespace();
        if !self.starts_with(">") {
            return Err(XmlError::MalformedMarkup {
                line,
                column,
                detail: "unterminated end tag".to_string(),
            });
        }
        self.pos += 1;
        Ok(name)
    }

    /// Consume `<?target data?>`, refusing `xml` as a PI target in any case.
    fn scan_pi(&mut self, start: usize) -> Result<(String, String), XmlError> {
        let (line, column) = self.at(start);
        self.pos += 2;
        let end =
            self.rest()
                .find("?>")
                .map(|i| self.pos + i)
                .ok_or(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: "unterminated processing instruction".to_string(),
                })?;
        let inner = &self.text[self.pos..end];
        let target_len = inner
            .chars()
            .take_while(|&c| is_name_char(c))
            .map(char::len_utf8)
            .sum::<usize>();
        let target = &inner[..target_len];
        if target.is_empty() {
            return Err(XmlError::MalformedMarkup {
                line,
                column,
                detail: "processing instruction without a target".to_string(),
            });
        }
        if target.eq_ignore_ascii_case("xml") {
            return Err(XmlError::MalformedMarkup {
                line,
                column,
                detail: "reserved processing instruction target xml".to_string(),
            });
        }
        let data = inner[target_len..]
            .trim_start_matches(is_xml_whitespace)
            .to_string();
        self.pos = end + 2;
        Ok((target.to_string(), data))
    }

    /// Consume the prolog declaration, accepting only 1.0 / UTF-8 and emitting
    /// nothing — all six lexical normalisations drop it from the infoset.
    fn scan_declaration(&mut self) -> Result<(), XmlError> {
        let start = self.pos;
        let (line, _) = self.at(start);
        let end =
            self.rest()
                .find("?>")
                .map(|i| self.pos + i)
                .ok_or(XmlError::MalformedMarkup {
                    line,
                    column: 1,
                    detail: "unterminated XML declaration".to_string(),
                })?;
        let inner = &self.text[start + 5..end];
        let mut i = 0usize;
        while i < inner.len() {
            while inner
                .as_bytes()
                .get(i)
                .is_some_and(|b| is_xml_whitespace_byte(*b))
            {
                i += 1;
            }
            if i >= inner.len() {
                break;
            }
            let name_start = i;
            while inner
                .get(i..)
                .and_then(|s| s.chars().next())
                .is_some_and(is_name_char)
            {
                i += inner[i..].chars().next().expect("checked above").len_utf8();
            }
            if i == name_start {
                return Err(XmlError::MalformedMarkup {
                    line,
                    column: 1,
                    detail: "bad XML declaration".to_string(),
                });
            }
            let name = &inner[name_start..i];
            while inner
                .as_bytes()
                .get(i)
                .is_some_and(|b| is_xml_whitespace_byte(*b))
            {
                i += 1;
            }
            if inner.as_bytes().get(i) != Some(&b'=') {
                return Err(XmlError::MalformedMarkup {
                    line,
                    column: 1,
                    detail: "bad XML declaration".to_string(),
                });
            }
            i += 1;
            while inner
                .as_bytes()
                .get(i)
                .is_some_and(|b| is_xml_whitespace_byte(*b))
            {
                i += 1;
            }
            let quote = inner.as_bytes().get(i).copied();
            if !matches!(quote, Some(b'"') | Some(b'\'')) {
                return Err(XmlError::MalformedMarkup {
                    line,
                    column: 1,
                    detail: "unquoted XML declaration value".to_string(),
                });
            }
            let quote = quote.expect("checked above");
            i += 1;
            let value_start = i;
            let value_end =
                inner[i..]
                    .find(quote as char)
                    .map(|n| i + n)
                    .ok_or(XmlError::MalformedMarkup {
                        line,
                        column: 1,
                        detail: "unterminated XML declaration value".to_string(),
                    })?;
            let value = &inner[value_start..value_end];
            match name {
                "version" if value != "1.0" => {
                    return Err(XmlError::UnsupportedDeclaration {
                        detail: alloc::format!("unsupported version {value}; only 1.0 is read"),
                    });
                }
                "encoding" if !value.eq_ignore_ascii_case("UTF-8") => {
                    return Err(XmlError::UnsupportedDeclaration {
                        detail: alloc::format!("unsupported encoding {value}; only UTF-8 is read"),
                    });
                }
                _ => {}
            }
            i = value_end + 1;
        }
        self.pos = end + 2;
        Ok(())
    }

    /// Read a Name at the current position, advancing past it.
    fn read_name(&mut self) -> Option<String> {
        let first = self.rest().chars().next()?;
        if !is_name_start(first) {
            return None;
        }
        let mut len = first.len_utf8();
        for c in self.rest()[len..].chars() {
            if !is_name_char(c) {
                break;
            }
            len += c.len_utf8();
        }
        let name = self.text[self.pos..self.pos + len].to_string();
        self.pos += len;
        Some(name)
    }

    /// Consume `<name attr="value" ...>`, decoding attribute values once.
    fn scan_start_tag(&mut self, start: usize) -> Result<StartTag, XmlError> {
        let (line, column) = self.at(start);
        self.pos += 1;
        let name = self.read_name().ok_or(XmlError::MalformedMarkup {
            line,
            column,
            detail: "start tag without a name".to_string(),
        })?;
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            self.skip_whitespace();
            if self.starts_with("/>") {
                self.pos += 2;
                self_closing = true;
                break;
            }
            if self.starts_with(">") {
                self.pos += 1;
                break;
            }
            if self.pos >= self.text.len() {
                return Err(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: "unterminated start tag".to_string(),
                });
            }
            let attr_pos = self.pos;
            let (attr_line, attr_column) = self.at(attr_pos);
            let attr_name = self.read_name().ok_or(XmlError::MalformedMarkup {
                line: attr_line,
                column: attr_column,
                detail: "bad attribute name".to_string(),
            })?;
            self.skip_whitespace();
            if self.byte() != Some(b'=') {
                return Err(XmlError::MalformedMarkup {
                    line: attr_line,
                    column: attr_column,
                    detail: "attribute without a value".to_string(),
                });
            }
            self.pos += 1;
            self.skip_whitespace();
            let quote = self.byte();
            if !matches!(quote, Some(b'"') | Some(b'\'')) {
                return Err(XmlError::MalformedMarkup {
                    line: attr_line,
                    column: attr_column,
                    detail: "unquoted attribute value".to_string(),
                });
            }
            let quote = quote.expect("checked above");
            self.pos += 1;
            let value_start = self.pos;
            let value_end = self
                .rest()
                .find(quote as char)
                .map(|i| self.pos + i)
                .ok_or(XmlError::MalformedMarkup {
                    line: attr_line,
                    column: attr_column,
                    detail: "unterminated attribute value".to_string(),
                })?;
            let value =
                decode_entities(&self.text[value_start..value_end], value_start, self.text)?;
            self.pos = value_end + 1;
            if attrs.iter().any(|(seen, _)| seen == &attr_name) {
                return Err(XmlError::DuplicateAttribute {
                    name: attr_name,
                    line: attr_line,
                    column: attr_column,
                });
            }
            attrs.push((attr_name, value));
        }
        Ok((name, attrs, self_closing))
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, is_name_char, is_name_start, scan};
    use crate::error::XmlError;
    use alloc::vec::Vec;

    fn kinds(text: &str) -> Vec<TokenKind> {
        scan(text).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn classifies_the_five_markup_forms() {
        let k = kinds("<a x='1'>t<!--c--><?p d?><![CDATA[z]]></a>");
        assert!(
            matches!(k[0], TokenKind::Start { ref name, self_closing: false, .. } if name == "a")
        );
        assert!(matches!(k[1], TokenKind::Text(ref t) if t == "t"));
        assert!(matches!(k[2], TokenKind::Comment(ref c) if c == "c"));
        assert!(
            matches!(k[3], TokenKind::ProcessingInstruction { ref target, ref data } if target == "p" && data == "d")
        );
        assert!(matches!(k[4], TokenKind::CData(ref c) if c == "z"));
        assert!(matches!(k[5], TokenKind::End { ref name } if name == "a"));
    }

    #[test]
    fn doctype_and_entity_are_refused_at_the_bang_boundary() {
        let dtd = scan("<!DOCTYPE root [<!ENTITY x 'y'>]><root/>").unwrap_err();
        assert_eq!(dtd.code(), "E-XML-001");
        match dtd {
            XmlError::DtdDeclared { line, column } => assert_eq!((line, column), (1, 1)),
            other => panic!("expected DtdDeclared, got {other:?}"),
        }
        let entity = scan("<root>&x;</root>").unwrap_err();
        assert_eq!(entity.code(), "E-XML-003");
        let standalone = scan("<!ENTITY x 'y'>").unwrap_err();
        assert_eq!(standalone.code(), "E-XML-002");
    }

    #[test]
    fn only_predefined_and_numeric_references_decode() {
        let k = kinds("<a>&amp;&lt;&gt;&quot;&apos;&#65;&#x42;</a>");
        assert!(matches!(k[1], TokenKind::Text(ref t) if t == "&<>\"'AB"));
        let bad = scan("<a>&foo;</a>").unwrap_err();
        assert!(matches!(bad, XmlError::UndeclaredEntity { ref name, .. } if name == "foo"));
    }

    #[test]
    fn numeric_references_outside_xml_ranges_are_refused() {
        assert_eq!(scan("<a>&#xD800;</a>").unwrap_err().code(), "E-XML-010");
        assert_eq!(scan("<a>&#0;</a>").unwrap_err().code(), "E-XML-010");
    }

    #[test]
    fn decoded_markup_is_never_rescanned() {
        let k = kinds("<a>&lt;b&gt;</a>");
        assert!(matches!(k[1], TokenKind::Text(ref t) if t == "<b>"));
        assert_eq!(k.len(), 3, "no Start token for b");
    }

    #[test]
    fn name_productions_and_comments_are_checked() {
        assert!(is_name_start(':'));
        assert!(is_name_start('_'));
        assert!(!is_name_start('1'));
        assert!(is_name_char('-'));
        assert!(is_name_char('1'));
        assert_eq!(scan("<!-- a -- b -->").unwrap_err().code(), "E-XML-004");
        let styles = scan("<?xml-stylesheet?>").unwrap();
        assert!(matches!(
            styles[0].kind,
            TokenKind::ProcessingInstruction { .. }
        ));
        assert_eq!(scan("<?XML?>").unwrap_err().code(), "E-XML-004");
    }

    #[test]
    fn only_1_0_utf8_declarations_are_accepted() {
        assert!(scan("<?xml version='1.0' encoding='UTF-8'?><a/>").is_ok());
        assert!(scan("<?xml version='1.1'?><a/>").is_err());
        assert!(scan("<?xml encoding='ISO-8859-1'?><a/>").is_err());
    }

    #[test]
    fn duplicate_attributes_are_refused() {
        let e = scan("<a x='1' x='2'/>").unwrap_err();
        assert!(matches!(e, XmlError::DuplicateAttribute { ref name, .. } if name == "x"));
    }

    #[test]
    fn a_bom_is_stripped_and_positions_are_tracked() {
        let tokens = scan("\u{feff}<a>\n<b/></a>").unwrap();
        let b = tokens
            .iter()
            .find(|t| matches!(&t.kind, TokenKind::Start { name, .. } if name == "b"))
            .unwrap();
        assert_eq!((b.line, b.column), (2, 1));
    }
}
