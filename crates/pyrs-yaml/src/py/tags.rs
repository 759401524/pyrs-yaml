//! The standard `!!` tags the loader must honour, and the base64 body of `!!binary`.
//!
//! An explicit tag states the type its author meant, so the document's implicit schema cannot answer it
//! on its own. Measured against PyYAML and ruamel - which agree with each other on every case below -
//! `!!str 1.20` is the string `"1.20"`, `!!float 1` is `1.0`, `!!bool yes` is `true`, and
//! `!!binary aGk=` is `b'hi'`. Before this module the loader resolved those texts as if the tag were
//! absent, which returned `1.2` (the type changed *and* the trailing zero was lost), `1`, `"yes"` and
//! `"aGk="`: four silent disagreements with both reference libraries, none of them reported.
//!
//! The tag carries its own lexical set - YAML 1.1's `tag:yaml.org,2002:*` definitions - which is why
//! `!!bool yes` is true under the Core schema too: it is the tag that makes it a boolean, not the
//! schema. Single-letter `y` / `n` are accepted on the strength of that spec and of ruamel; PyYAML
//! refuses them here, the one place the two references disagree.
//!
//! Unknown tags are deliberately left alone. Both reference libraries raise on `!!weird`, but turning
//! that into an error would break documents that carry application tags this engine forwards to a
//! registered plugin, and the tag survives a round trip either way.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyString};

use crate::YamlTypeError;

/// The AST tag a registered plugin name stands for.
///
/// The name carries its own handle: `!!binary` is YAML's standard tag, `!custom` is a local one.
/// Collapsing every name to a single `!` downgraded the standard tags to local ones, so a document
/// this library wrote could not be read by the libraries whose tag it had borrowed.
pub(crate) fn tag_of(registered: &str) -> crate::ast::Tag {
    crate::ast::Tag {
        handle: if registered.starts_with("!!") {
            "!!"
        } else {
            "!"
        }
        .to_string(),
        suffix: registered.trim_start_matches('!').to_string(),
    }
}

/// The value an explicit standard tag asks for, or `None` when the tag is not one of ours.
///
/// A text that cannot be read as the tagged type is an error rather than a fall-through: silently
/// returning a string for `!!int hello` is how the loader ends up agreeing with nothing.
pub(crate) fn tagged_scalar(py: Python<'_>, tag: &str, text: &str) -> PyResult<Option<Py<PyAny>>> {
    // The same tag reaches the AST in two spellings: the shorthand `!!str`, and the verbatim
    // `!<tag:yaml.org,2002:str>` a document may write or a caller may set through `Node.set_tag`.
    // Both name one type, so both must be honoured - answering for only the first would leave
    // `!<tag:yaml.org,2002:str> 1.20` loading as the float it plainly says it is not.
    let name = match tag
        .strip_prefix("!<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        Some(uri) => uri,
        None => tag,
    };
    match name {
        "!!str" | "tag:yaml.org,2002:str" => Ok(Some(PyString::new(py, text).into_any().unbind())),
        "!!int" | "tag:yaml.org,2002:int" => tagged_int(py, text).map(Some),
        "!!float" | "tag:yaml.org,2002:float" => tagged_float(py, text).map(Some),
        "!!bool" | "tag:yaml.org,2002:bool" => tagged_bool(py, text).map(Some),
        "!!binary" | "tag:yaml.org,2002:binary" => tagged_binary(py, text).map(Some),
        _ => Ok(None),
    }
}

fn mismatch(tag: &str, text: &str) -> PyErr {
    YamlTypeError::new_err(format!("value tagged {tag} is not a {tag}: {text:?}"))
}

/// YAML 1.1's boolean lexemes, case-insensitively.
fn tagged_bool(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let value = match text.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "y" => true,
        "false" | "no" | "off" | "n" => false,
        _ => return Err(mismatch("!!bool", text)),
    };
    Ok(PyBool::new(py, value).to_owned().into_any().unbind())
}

fn tagged_int(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    if let Some(value) = integer_i64(&digits) {
        return Ok(value.into_pyobject(py)?.into_any().unbind());
    }
    // `!!int` has no width limit, so a value past i64 is still an integer - Python's `int(text, 0)`
    // reads the same 0x / 0o / 0b spellings and arbitrary precision is exactly what the tag allows.
    let builtins = py.import("builtins")?;
    let int = builtins.getattr("int")?;
    if let Ok(value) = int.call1((&digits, 0)) {
        return Ok(value.unbind());
    }
    Err(mismatch("!!int", text))
}

/// The integer spellings the tag defines: an optional sign, then base-10 digits or a `0x` / `0o` /
/// `0b` prefixed literal, plus YAML's legacy `07` octal.
fn integer_i64(text: &str) -> Option<i64> {
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

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

fn tagged_float(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let value = float_value(text).ok_or_else(|| mismatch("!!float", text))?;
    Ok(value.into_pyobject(py)?.into_any().unbind())
}

/// The numbers the `!!float` tag names. Read apart from Python so the boundary is testable.
fn float_value(text: &str) -> Option<f64> {
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

fn strip_sign_ci(text: &str) -> String {
    match text.strip_prefix('-') {
        Some(rest) => format!("-{}", rest.to_ascii_lowercase()),
        None => text.strip_prefix('+').unwrap_or(text).to_ascii_lowercase(),
    }
}

fn tagged_binary(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let bytes = base64_decode(text).ok_or_else(|| mismatch("!!binary", text))?;
    Ok(PyBytes::new(py, &bytes).into_any().unbind())
}

/// Base64 as the tag uses it: whitespace is layout and is dropped, and the padding is read
/// leniently (`aGk===` decodes, as both reference libraries decode it).
fn base64_decode(text: &str) -> Option<Vec<u8>> {
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
