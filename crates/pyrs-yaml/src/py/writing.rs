//! Streaming writer: document-level serialization to a Python file-like
//! object or a local file. Mirrors `streaming.rs`'s dual-input design.

use std::io::Write;

use pyo3::prelude::*;

use crate::py::convert::format_i18n_error;
use crate::py::direct_dump::direct_dump_with_options;
use crate::py::pyrs_yaml::{YamlDocument, serialize_document};
use crate::serializer::SerializeOptions;

/// Destination for streaming writes.
pub(crate) enum OutputSink {
    /// Python file-like object (`Py<PyAny>`, Ungil) — `write(str)` per flush.
    PyObj(Py<PyAny>),
    /// Local file handle.
    File(std::io::BufWriter<std::fs::File>),
}

/// Default chunk size for streaming writes (64 KiB).
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Buffered chunk writer. Accumulates output and flushes to the sink when the
/// pending buffer reaches `chunk_size`; `flush` writes the remainder.
pub(crate) struct SinkWriter {
    sink: OutputSink,
    chunk_size: usize,
    pending: String,
}

impl SinkWriter {
    pub(crate) fn new(sink: OutputSink, chunk_size: usize) -> Self {
        Self {
            sink,
            chunk_size,
            pending: String::new(),
        }
    }

    /// Append `text`; flush when the pending buffer reaches the chunk size.
    pub(crate) fn write(&mut self, py: Python, text: &str) -> PyResult<()> {
        self.pending.push_str(text);
        if self.pending.len() >= self.chunk_size {
            self.flush(py)?;
        }
        Ok(())
    }

    /// Write the pending buffer to the sink. PyObj writes run under the GIL;
    /// File writes are fully detached (`py.detach`).
    pub(crate) fn flush(&mut self, py: Python) -> PyResult<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let text = std::mem::take(&mut self.pending);
        match &mut self.sink {
            OutputSink::PyObj(obj) => {
                obj.call_method1(py, "write", (text,))
                    .map(|_| ())
                    .map_err(|e| {
                        pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                            "file-write-error",
                            &[("detail", &e.to_string())],
                        ))
                    })
            }
            OutputSink::File(w) => py.detach(|| w.write_all(text.as_bytes())).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                    "file-write-error",
                    &[("detail", &e.to_string())],
                ))
            }),
        }
    }
}

/// Normalize a serialized document to end with exactly one `\n`.
pub(crate) fn normalize_doc(text: &str) -> String {
    let trimmed = text.trim_end_matches('\n');
    format!("{}\n", trimmed)
}

/// Serialize options for streaming write: fixed shape, `sort_keys` the only
/// user-visible toggle. Separator flags (`---`/`...`) are NOT set here —
/// `dump_iterable` handles them programmatically, and the serializer must not
/// emit them inline.
pub(crate) fn dump_options(sort_keys: bool) -> SerializeOptions {
    SerializeOptions {
        indent_size: 2,
        explicit_start: false,
        explicit_end: false,
        sort_keys,
        max_depth: 1000,
        width: 80,
        indent_mapping: 2,
        indent_sequence: 2,
        indent_offset: 0,
    }
}

/// Pull items from `iterable`, serialize each (document-level) and write to
/// the sink. Separator policy: `---\n` before every document after the first;
/// `explicit_start` adds a leading `---\n`; `explicit_end` adds a trailing
/// `...\n`. An empty iterable writes zero bytes.
pub(crate) fn dump_iterable(
    py: Python,
    writer: &mut SinkWriter,
    iterable: Bound<'_, PyAny>,
    options: &SerializeOptions,
    explicit_start: bool,
    explicit_end: bool,
) -> PyResult<()> {
    let mut iter = iterable.try_iter()?;
    let mut first = true;
    loop {
        let item = match iter.next() {
            Some(Ok(i)) => i,
            Some(Err(e)) => {
                // Keep the partial output produced before the failure.
                let _ = writer.flush(py);
                return Err(e);
            }
            None => break,
        };
        let result: PyResult<String> = (|| {
            if let Ok(doc) = item.cast::<YamlDocument>() {
                let mut doc = doc.try_borrow_mut()?;
                serialize_document(&mut doc, py, options)
            } else {
                // Direct path: typed PyErrs (YamlMaxDepthError, YamlTypeError,
                // YamlSerializeError) propagate as-is, matching what
                // pyobject_to_node raised directly before serialization.
                direct_dump_with_options(py, &item.unbind(), options.sort_keys)
            }
        })();
        let yaml = match result {
            Ok(y) => y,
            Err(e) => {
                // Flush pending before propagating the error so partial output is
                // preserved. If the flush itself fails that error is more important.
                writer.flush(py)?;
                return Err(e);
            }
        };
        if !first || explicit_start {
            writer.write(py, "---\n")?;
        }
        first = false;
        writer.write(py, &normalize_doc(&yaml))?;
    }
    if explicit_end && !first {
        writer.write(py, "...\n")?;
    }
    writer.flush(py)?;
    Ok(())
}
