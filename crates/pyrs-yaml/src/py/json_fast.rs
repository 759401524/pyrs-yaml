//! Direct strict-JSON -> Python-object fast path for `load_jsonc`.
//!
//! The general load path parses text into the shared `CustomNode` AST and then
//! converts it to Python objects. For plain, canonical JSON (arrays, objects,
//! `i64`-range integers, JSON floats, booleans, `null`, and strings with no
//! escape sequences) that intermediate AST is pure overhead, so this scanner
//! walks the bytes and builds `PyList` / `PyDict` / scalars directly. Floats go
//! through Rust's correctly-rounded `f64` parse, which yields the identical
//! double to the `float()` that CPython's `json.loads` uses, so parity holds by
//! construction (overflow to `+/-inf` matches too).
//!
//! Correctness is guaranteed by *falling back*, never by reimplementing edge
//! cases: any input outside the narrow canonical subset — a malformed number,
//! leading `+`, out-of-range integer, any `\` escape, a comment, trailing
//! comma, control byte, non-canonical token, or over-deep nesting — makes the
//! scanner give up (`None`) and the caller routes the whole document through
//! the battle-tested AST path, preserving pyrs's exact JSON5/JSONC number
//! spellings and surrogate rules. So the fast path can only ever produce the
//! same value the general path would, for a strict subset.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyList, PyNone, PyString};

/// Owned reference to any Python object (pyo3 0.29 dropped the `PyObject`
/// alias from the prelude).
type PyObject = Py<PyAny>;

/// Depth cap matching the JSON parser's default; deeper input falls back to the
/// general path (which raises the identical depth error).
const MAX_DEPTH: usize = 1000;

struct Scanner<'a> {
    src: &'a [u8],
    text: &'a str,
    pos: usize,
}

type Bail<T> = Option<T>;

/// True for a digit run that is a multi-digit number with a leading `0` (e.g.
/// `01`, `007`) — invalid strict JSON, so the fast path bails and lets the AST
/// path reject/report it. A lone `0` (or `-0`) is valid and returns false.
fn bytes_eq_lead_zero_multi(digits: &[u8]) -> bool {
    digits.len() > 1 && digits.first() == Some(&b'0')
}

impl<'a> Scanner<'a> {
    fn new(s: &'a str) -> Self {
        Scanner {
            src: s.as_bytes(),
            text: s,
            pos: 0,
        }
    }

    #[inline]
    fn skip_ws(&mut self) {
        while let Some(&b) = self.src.get(self.pos) {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    /// Parse the top-level value; returns `None` to signal "use the AST path".
    fn value(&mut self, py: Python<'a>, depth: usize) -> Bail<PyObject> {
        if depth > MAX_DEPTH {
            return None;
        }
        match self.peek()? {
            b'{' => self.object(py, depth),
            b'[' => self.array(py, depth),
            b'"' => self.string(py),
            b't' => self.lit(
                b"true",
                PyBool::new(py, true).to_owned().into_any().unbind(),
            ),
            b'f' => self.lit(
                b"false",
                PyBool::new(py, false).to_owned().into_any().unbind(),
            ),
            b'n' => self.lit(b"null", PyNone::get(py).to_owned().into_any().unbind()),
            b'-' | b'0'..=b'9' => self.number(py),
            _ => None,
        }
    }

    fn lit(&mut self, word: &[u8], out: PyObject) -> Bail<PyObject> {
        if self.src[self.pos..].starts_with(word) {
            self.pos += word.len();
            Some(out)
        } else {
            None
        }
    }

    fn string(&mut self, py: Python<'a>) -> Bail<PyObject> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.pos += 1; // opening quote
        let start = self.pos;
        let bytes = self.src;
        let mut i = self.pos;
        // Scan to the closing quote, recording the first escape. Decode the
        // eight two-byte JSON escapes inline (\n, \t, \", \\, ... - common in
        // configs/logs); bail on \u (surrogate-pair combining) and any other or
        // invalid escape, and on a raw control byte, so the AST path owns those
        // cases (and their errors) identically. Bytes >= 0x80 are UTF-8
        // lead/continuation and pass through untouched.
        let mut first_escape: Option<usize> = None;
        loop {
            let b = *bytes.get(i)?;
            match b {
                b'"' => break,
                b'\\' => {
                    if first_escape.is_none() {
                        first_escape = Some(i);
                    }
                    match bytes.get(i + 1) {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => i += 2,
                        _ => return None, // \u or invalid -> AST path
                    }
                }
                0x00..=0x1f => return None,
                _ => i += 1,
            }
        }
        let end = i;
        // No escapes: copy the input subslice zero-copy (`get` keeps a
        // pathological non-boundary a bail rather than a panic).
        let Some(esc) = first_escape else {
            let s = self.text.get(start..end)?;
            self.pos = end + 1; // consume closing quote
            return Some(PyString::new(py, s).into_any().unbind());
        };
        // Decode the simple escapes into an owned buffer, bulk-copying each
        // escapable-free run between them.
        let mut buf = String::with_capacity(end - start);
        buf.push_str(self.text.get(start..esc)?);
        let mut j = esc;
        while j < end {
            if bytes.get(j) == Some(&b'\\') {
                let c = match bytes.get(j + 1) {
                    Some(b'"') => '"',
                    Some(b'\\') => '\\',
                    Some(b'/') => '/',
                    Some(b'b') => '\u{8}',
                    Some(b'f') => '\u{c}',
                    Some(b'n') => '\n',
                    Some(b'r') => '\r',
                    Some(b't') => '\t',
                    _ => return None,
                };
                buf.push(c);
                j += 2;
            } else {
                let run = j;
                while j < end && bytes[j] != b'\\' {
                    j += 1;
                }
                buf.push_str(self.text.get(run..j)?);
            }
        }
        self.pos = end + 1; // consume closing quote
        Some(PyString::new(py, &buf).into_any().unbind())
    }

    fn number(&mut self, py: Python<'a>) -> Bail<PyObject> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == digits_start {
            self.pos = start;
            return None;
        }
        // Enforce the JSON integer-part grammar (`-?(0|[1-9][0-9]*)`): a
        // multi-digit run may not start with `0` (invalid JSON -> AST reports it).
        if bytes_eq_lead_zero_multi(&self.src[digits_start..self.pos]) {
            self.pos = start;
            return None;
        }
        // A fraction and/or exponent make a float; validate the full JSON number
        // grammar so any malformed form (`1.`, `1e`, `1e+`) bails and the AST
        // path raises the identical error.
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            let frac_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == frac_start {
                self.pos = start;
                return None;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == exp_start {
                self.pos = start;
                return None;
            }
        }
        if is_float {
            // Rust's f64 parse is correctly rounded, so for a syntactically valid
            // JSON number it yields the same double as the float() that
            // json.loads uses; overflow to +/-inf matches too. A parse failure
            // bails to the AST path.
            let token = self.text.get(start..self.pos)?;
            let Ok(f) = token.parse::<f64>() else {
                self.pos = start;
                return None;
            };
            return Some(PyFloat::new(py, f).to_owned().into_any().unbind());
        }
        // Integer: manual checked accumulation avoids `str::parse` and a UTF-8
        // re-validation pass per number (the orjson-style tight path). Any
        // overflow (out-of-i64) bails so the AST path yields pyrs's canonical
        // form for the value.
        let negative = self.src[start] == b'-';
        let mut acc: i64 = 0;
        for &b in &self.src[digits_start..self.pos] {
            acc = acc.checked_mul(10)?.checked_add(i64::from(b - b'0'))?;
        }
        let n = if negative { -acc } else { acc };
        Some(n.into_pyobject(py).ok()?.into_any().unbind())
    }

    fn object(&mut self, py: Python<'a>, depth: usize) -> Bail<PyObject> {
        self.pos += 1; // '{'
        let dict = PyDict::new(py);
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Some(dict.into_any().unbind());
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return None; // non-string / unquoted key -> AST path
            }
            let key = self.string(py)?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return None;
            }
            self.pos += 1;
            self.skip_ws();
            let val = self.value(py, depth + 1)?;
            // Duplicate keys: last wins (Python dict semantics match the JSON
            // parser's last-value-wins rule).
            dict.set_item(key.bind(py), val).ok()?;
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Some(dict.into_any().unbind());
                }
                _ => return None,
            }
        }
    }

    fn array(&mut self, py: Python<'a>, depth: usize) -> Bail<PyObject> {
        self.pos += 1; // '['
        let mut items: Vec<PyObject> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            let empty: Vec<PyObject> = Vec::new();
            return Some(PyList::new(py, empty).ok()?.into_any().unbind());
        }
        loop {
            self.skip_ws();
            let val = self.value(py, depth + 1)?;
            items.push(val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    let list = PyList::new(py, items).ok()?;
                    return Some(list.into_any().unbind());
                }
                _ => return None,
            }
        }
    }
}

/// Attempt the fast path. `Some(obj)` = produced the canonical value; `None` =
/// caller must use the AST path (comments, floats, escapes, trailing commas,
/// out-of-range ints, over-deep nesting, trailing junk, any non-canonical byte).
pub(crate) fn try_load(py: Python<'_>, json_str: &str) -> Option<Py<PyAny>> {
    let mut sc = Scanner::new(json_str);
    sc.skip_ws();
    let val = sc.value(py, 0)?;
    sc.skip_ws();
    // Reject trailing content: the AST path validates the document fully.
    if sc.pos != sc.src.len() {
        return None;
    }
    Some(val)
}
