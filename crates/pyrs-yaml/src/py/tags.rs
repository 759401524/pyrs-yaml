//! The standard `!!` tags the loader must honour, and the values they name.
//!
//! An explicit tag states the type its author meant, so the document's implicit schema cannot answer it
//! on its own. Measured against PyYAML and ruamel - which agree with each other on every case below -
//! `!!str 1.20` is the string `"1.20"`, `!!float 1` is `1.0`, `!!bool yes` is `true`, and
//! `!!binary aGk=` is `b'hi'`. Before this module the loader resolved those texts as if the tag were
//! absent, which returned `1.2` (the type changed *and* the trailing zero was lost), `1`, `"yes"` and
//! `"aGk="`: four silent disagreements with both reference libraries, none of them reported.
//!
//! The lexical grammar itself - which digits a tag accepts, how its `!!binary` body is read - lives in
//! `pyrs_yaml_core::tagged`, because it is engine semantics rather than object marshalling.
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

use pyrs_yaml_core::tagged::{base64_decode, float_value, integer_i64};

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
        // The non-specific tag `!` is not a standard tag, but for a scalar it states the same
        // reading: YAML 1.2 §6.8 leaves the node's tag as `!` and §10.5.2 restricts application
        // resolution rules to the `?` tag, so the scalar is its own text. Spec Example 6.28 and the
        // suite's `S4JQ` spell it as a string; PyYAML and ruamel resolve it implicitly instead
        // (measured on this tree), so the divergence is recorded rather than smoothed over.
        "!!str" | "tag:yaml.org,2002:str" | "!" => {
            Ok(Some(PyString::new(py, text).into_any().unbind()))
        }
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

fn tagged_float(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let value = float_value(text).ok_or_else(|| mismatch("!!float", text))?;
    Ok(value.into_pyobject(py)?.into_any().unbind())
}

fn tagged_binary(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let bytes = base64_decode(text).ok_or_else(|| mismatch("!!binary", text))?;
    Ok(PyBytes::new(py, &bytes).into_any().unbind())
}
