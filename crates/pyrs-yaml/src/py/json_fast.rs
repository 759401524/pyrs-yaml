//! Direct strict-JSON -> Python-object fast path for `load_jsonc`.
//!
//! The general load path parses text into the shared `CustomNode` AST and then
//! converts it to Python objects. For plain, canonical JSON (arrays, objects,
//! `i64`-range integers, booleans, `null`, and strings with no escape
//! sequences) that intermediate AST is pure overhead, so this scanner walks the
//! bytes and builds `PyList` / `PyDict` / scalars directly.
//!
//! Correctness is guaranteed by *falling back*, never by reimplementing edge
//! cases: any input outside the narrow canonical subset — a float, exponent,
//! leading `+`, out-of-range integer, any `\` escape, a comment, trailing
//! comma, control byte, non-canonical token, or over-deep nesting — makes the
//! scanner give up (`None`) and the caller routes the whole document through
//! the battle-tested AST path, preserving pyrs's exact JSON5/JSONC number
//! spellings and surrogate rules. So the fast path can only ever produce the
//! same value the general path would, for a strict subset.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyNone, PyString};

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
        // Scan for the closing quote, rejecting any `\` escape or control byte
        // (fall back for those). Bytes >= 0x80 are UTF-8 continuation/lead bytes
        // and are copied verbatim below.
        loop {
            let b = *bytes.get(i)?;
            match b {
                b'"' => break,
                b'\\' => return None,
                0x00..=0x1f => return None,
                _ => i += 1,
            }
        }
        let end = i;
        // `text[start..end]` is a valid UTF-8 subslice of the input (multi-byte
        // lead/continuation bytes are >= 0x80, never a delimiter or control
        // byte), so slicing needs no re-validation. `get` keeps a pathological
        // non-boundary a bail to the AST path rather than a panic.
        let s = self.text.get(start..end)?;
        self.pos = end + 1; // consume closing quote
        Some(PyString::new(py, s).into_any().unbind())
    }

    fn number(&mut self, py: Python<'a>) -> Bail<PyObject> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        // Integer digits only; a `.` or `e` means a float (fall back to the AST
        // path, which resolves pyrs's exact JSON5/JSONC number spellings).
        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == digits_start {
            self.pos = start;
            return None;
        }
        // Enforce the JSON integer grammar (`-?(0|[1-9][0-9]*)`): a multi-digit
        // run may not start with `0` (that is invalid JSON), and `.`/`e` means a
        // float. Both bail so the AST path owns validation + exact spelling.
        if bytes_eq_lead_zero_multi(&self.src[digits_start..self.pos])
            || matches!(self.peek(), Some(b'.') | Some(b'e') | Some(b'E'))
        {
            self.pos = start;
            return None;
        }
        // Manual checked accumulation avoids `str::parse::<i64>` and a UTF-8
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
