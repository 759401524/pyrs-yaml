//! The Python surface of a line-delimited JSON stream: one value per line, as it is asked for.

use std::sync::Arc;

use pyo3::prelude::*;

use crate::YamlParseError;
use crate::py::convert::{format_i18n_error, node_to_pyobject_resolving_anchors, parse_schema};
use crate::py::document::resolve_tags;
use crate::py::json_fast;

/// The dialect the line walk reads. One dialect per stream, so a record cannot change what the
/// next record is.
const STRICT: pyrs_json::JsonParseOptions = pyrs_json::JsonParseOptions::STRICT;

#[pyclass]
/// Iterator over the values of a line-delimited JSON stream, one document per line.
pub(crate) struct JsonLines {
    /// Owned, because a `#[pyclass]` cannot borrow the string it was handed. The walk resumes from
    /// the two numbers below, which is what `pyrs_json::Lines` reports for each line.
    text: Arc<str>,
    /// The first byte of the line not yet returned.
    at: usize,
    /// How many lines were returned before it, which is what an error's position is measured against.
    number: usize,
}

impl JsonLines {
    pub(crate) fn new(text: &str) -> Self {
        JsonLines {
            text: Arc::from(text),
            at: 0,
            number: 0,
        }
    }

    /// Resolve one line to a Python value, taking the same two routes `load_json` takes for one
    /// document: canonical JSON builds the object straight from the bytes, and anything else goes
    /// through the AST so the dialect's number and string semantics stay in one place.
    ///
    /// No `py.detach` here. The dominant cost of a line is building its Python objects, which needs
    /// the GIL, and the per-line detach would add an acquire-release on top of the work it was meant
    /// to overlap — the same reason `load_json` is not detached. Other threads get the GIL at every
    /// yield point the iterator already has.
    fn resolve(&self, py: Python<'_>, number: usize, line: pyrs_json::Line) -> PyResult<Py<PyAny>> {
        let text = &self.text[line.0..line.1];
        if let Some(value) = json_fast::try_load(py, text) {
            return Ok(value);
        }
        let mut ast =
            pyrs_json::document_from_line(&self.text, line, number, STRICT).map_err(|error| {
                YamlParseError::new_err(format_i18n_error(
                    "json-parse-error",
                    &[("detail", &error.to_string())],
                ))
            })?;
        resolve_tags(&mut ast, py)?;
        node_to_pyobject_resolving_anchors(&ast, py, &parse_schema("json")?, false)
    }
}

#[pymethods]
impl JsonLines {
    /// Return self (iterator protocol).
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Yield the next line's value; `None` once the stream is exhausted.
    ///
    /// A line the stream refuses raises with its position in the whole input, and the cursor does
    /// not move: asking again reports the same record rather than the one after it, because a
    /// record this format defines is not one to lose silently.
    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let Some((number, line)) =
            pyrs_json::Lines::resume(&self.text, self.at, self.number).next()
        else {
            return Ok(None);
        };
        let value = self.resolve(py, number, line)?;
        self.at = line.1 + 1;
        self.number = number + 1;
        Ok(Some(value))
    }
}
