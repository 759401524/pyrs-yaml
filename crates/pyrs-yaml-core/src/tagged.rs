//! The lexical grammar of YAML's standard `!!` tags, apart from any object to build.
//!
//! An explicit tag states the type its author meant, and each tag brings its own lexemes: YAML 1.1's
//! `tag:yaml.org,2002:*` definitions say `!!int 0x1F` is 31, `!!float 1` is `1.0`, and `!!binary aGk=`
//! is `b"hi"`. Those readings are engine semantics rather than Python marshalling, so they live here -
//! the binding's job starts where a value has to become a `Py<PyAny>`, and it may disagree with Python
//! without this file noticing.
//!
//! The split is also what makes the grammar testable: measured 2026-10-15, the six tests below ran only
//! against functions whose signatures name no `Python<'_>`, yet they linked libpython because they sat
//! in the crate that owns the PyO3 dependency. Here they run in the Python-free lane
//! (`cargo nextest run -p pyrs-yaml-core`), which is the lane a contributor without an interpreter
//! develops in.
//!
//! Measured against PyYAML and ruamel - which agree with each other on every case in the tests -
//! `!!str 1.20` is the string `"1.20"`, `!!float 1` is `1.0`, `!!bool yes` is `true`, and
//! `!!binary aGk=` is `b'hi'`. Before this grammar existed, the loader resolved those texts as if the
//! tag were absent, which returned `1.2` (the type changed *and* the trailing zero was lost), `1`,
//! `"yes"` and `"aGk="`.

use alloc::string::String;
use alloc::vec::Vec;

/// The integer spellings the `!!int` tag defines: an optional sign, then base-10 digits or a `0x` /
/// `0o` / `0b` prefixed literal, plus YAML's legacy `07` octal.
///
/// `None` means "not a 64-bit integer in a spelling the tag allows", which the caller reads as either a
/// Python `int` of arbitrary width or a type error - the width is not this function's question.
pub fn integer_i64(text: &str) -> Option<i64> {
    let (sign, body) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, text.strip_prefix('+').unwrap_or(text)),
    };
    if body.is_empty() {
        return None;
    }
    let (radix, digits) = if let Some(rest) = strip_prefix_ci(body, "0x") {
        (16, rest)
    } else if let Some(rest) = strip_prefix_ci(body, "0o") {
        (8, rest)
    } else if let Some(rest) = strip_prefix_ci(body, "0b") {
        (2, rest)
    } else if body.len() > 1 && body.starts_with('0') && body.bytes().all(|b| b.is_ascii_digit()) {
        // Legacy octal: `017` is 15 in YAML 1.1, and both reference libraries read it that way.
        (8, &body[1..])
    } else {
        (10, body)
    };
    if digits.is_empty() {
        return None;
    }
    i64::from_str_radix(digits, radix)
        .ok()
        .map(|n| n.saturating_mul(sign))
}

/// The numbers the `!!float` tag names. Read apart from Python so the boundary is testable.
pub fn float_value(text: &str) -> Option<f64> {
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    match strip_sign_ci(&digits).as_str() {
        // The infinities and the not-a-number of YAML's own float resolution, spelled the way the
        // 1.1 tag defines them - `.inf`, `.Inf`, `.INF`, with the sign the implicit path also takes.
        ".inf" => Some(f64::INFINITY),
        "-.inf" => Some(f64::NEG_INFINITY),
        ".nan" => Some(f64::NAN),
        // A float has to carry a digit: Rust's `parse` also accepts `inf`, `infinity` and `NaN`,
        // which are Python's spellings rather than YAML's. A bare integer is allowed - becoming a
        // float is the whole point of the tag, so `!!float 1` is `1.0` in both references.
        _ if digits.bytes().any(|byte| byte.is_ascii_digit()) => {
            digits.parse::<f64>().ok().filter(|value| value.is_finite())
        }
        _ => None,
    }
}

/// Base64 as the `!!binary` tag uses it: whitespace is layout and is dropped, and the padding is read
/// leniently (`aGk===` decodes, as both reference libraries decode it).
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut sextets: Vec<u8> = Vec::with_capacity(text.len());
    let mut padding = 0usize;
    for byte in text.bytes() {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => continue,
            b'=' => padding += 1,
            _ => {
                if padding > 0 {
                    // Only trailing `=` is legal; anything after it makes the body unreadable.
                    return None;
                }
                sextets.push(base64_value(byte)?);
            }
        }
    }
    if sextets.is_empty() {
        return Some(Vec::new());
    }
    let mut out: Vec<u8> = Vec::with_capacity(sextets.len() * 3 / 4);
    let mut index = 0usize;
    while index + 4 <= sextets.len() {
        let group = &sextets[index..index + 4];
        let n = u32::from(group[0]) << 18
            | u32::from(group[1]) << 12
            | u32::from(group[2]) << 6
            | u32::from(group[3]);
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
        index += 4;
    }
    match sextets.len() - index {
        0 => {}
        2 => {
            let n = u32::from(sextets[index]) << 18 | u32::from(sextets[index + 1]) << 12;
            out.push((n >> 16) as u8);
        }
        3 => {
            let n = u32::from(sextets[index]) << 18
                | u32::from(sextets[index + 1]) << 12
                | u32::from(sextets[index + 2]) << 6;
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
        }
        // A single leftover sextet cannot name a byte; that is a broken body, not a short one.
        _ => return None,
    }
    // The trailing group may declare padding; extra `=` is tolerated because both reference
    // libraries tolerate it, and refusing it breaks a document nobody meant to break.
    let _ = padding;
    Some(out)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

fn strip_sign_ci(text: &str) -> String {
    match text.strip_prefix('-') {
        Some(rest) => format!("-{}", rest.to_ascii_lowercase()),
        None => text.strip_prefix('+').unwrap_or(text).to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_decoder_reads_known_answers() {
        // Every pair here is text another part of this library produced with Python's `b64encode`, or
        // a body both reference libraries decode to the bytes named - so the decoder is checked
        // against the encoders in use, not against itself.
        for (encoded, expected) in [
            ("aGk=", b"hi".as_slice()),
            ("YQ==", b"a".as_slice()),
            ("", b"".as_slice()),
            (
                "AAECAwQFBgcI",
                b"\x00\x01\x02\x03\x04\x05\x06\x07\x08".as_slice(),
            ),
        ] {
            assert_eq!(
                base64_decode(encoded).as_deref(),
                Some(expected),
                "{encoded:?}"
            );
        }
    }

    #[test]
    fn the_tagged_spellings_of_a_known_gif_decode() {
        // Multi-line and wrapped; PyYAML, ruamel and Python's own `b64decode` all give 44 bytes
        // for this body, which is the number measured rather than counted by hand.
        let text = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIB\n  RAID7wA=";
        let decoded = base64_decode(text).expect("decodes");
        assert_eq!(&decoded[..6], b"GIF89a");
        assert_eq!(decoded.len(), 44);
    }

    #[test]
    fn padding_is_read_the_way_the_references_read_it() {
        assert_eq!(base64_decode("aGk=").as_deref(), Some("hi".as_bytes()));
        assert_eq!(base64_decode("aG k=").as_deref(), Some("hi".as_bytes()));
        assert_eq!(base64_decode("aGk===").as_deref(), Some("hi".as_bytes()));
        assert_eq!(base64_decode("").as_deref(), Some(&[][..]));
    }

    #[test]
    fn a_body_that_is_not_base64_is_rejected() {
        assert_eq!(base64_decode("!!!"), None);
        assert_eq!(base64_decode("a"), None, "one sextet names no byte");
    }

    #[test]
    fn integer_spellings_cover_the_1_1_tag() {
        assert_eq!(integer_i64("31"), Some(31));
        assert_eq!(integer_i64("0x1F"), Some(31));
        assert_eq!(integer_i64("0X1f"), Some(31));
        assert_eq!(integer_i64("0o17"), Some(15));
        assert_eq!(integer_i64("017"), Some(15), "legacy octal");
        assert_eq!(integer_i64("0b101"), Some(5));
        assert_eq!(integer_i64("-0"), Some(0));
        assert_eq!(integer_i64(""), None);
        assert_eq!(integer_i64("1.0"), None);
    }

    #[test]
    fn a_float_needs_a_digit_and_takes_yamls_own_infinities() {
        assert_eq!(float_value("1"), Some(1.0));
        assert_eq!(float_value("1e3"), Some(1000.0));
        assert_eq!(float_value("1_0.5"), Some(10.5));
        assert_eq!(float_value("-.5"), Some(-0.5));
        assert_eq!(float_value("1."), Some(1.0), "the references read this too");
        assert_eq!(float_value(".inf"), Some(f64::INFINITY));
        assert_eq!(float_value("-.Inf"), Some(f64::NEG_INFINITY));
        assert!(float_value(".nan").is_some_and(|value| value.is_nan()));
        // Rust's `parse` accepts these; YAML's tag does not, so neither may load.
        assert_eq!(float_value("inf"), None);
        assert_eq!(float_value("NaN"), None);
        assert_eq!(float_value("hello"), None);
    }
}
