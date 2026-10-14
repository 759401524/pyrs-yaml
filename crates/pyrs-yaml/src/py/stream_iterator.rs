use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::parser::StreamEvent;
use crate::py::stream_events::stream_event_to_py_dict;

#[pyclass]
/// YAML event stream iterator, yielding parsed events one by one.
pub(crate) struct StreamIterator {
    // The events own their text: a `#[pyclass]` cannot carry a borrow of the source, so this is
    // the one stream path that detaches (see `py::functions::parse_stream`). It is a plain comment
    // rather than a doc comment because a pyclass's docstring is the published contract — the
    // committed `.pyi` carries it, and `check_stub_drift.py` compares what users see.
    pub(crate) events: Vec<StreamEvent<'static>>,
    pub(crate) index: usize,
}

#[pymethods]
impl StreamIterator {
    /// Return self (iterator protocol).
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Yield the next event dict; return `None` when the stream ends.
    fn __next__<'a>(&mut self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDict>>> {
        if self.index < self.events.len() {
            let event = &self.events[self.index];
            self.index += 1;
            Ok(Some(stream_event_to_py_dict(py, event)?))
        } else {
            Ok(None)
        }
    }
}
