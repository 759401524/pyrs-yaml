//! Streaming writer: document-level serialization to a Python file-like
//! object or a local file. Mirrors `streaming.rs`'s dual-input design.

use std::io::Write;

use pyo3::prelude::*;

use crate::py::convert::format_i18n_error;
use crate::py::pyrs_yaml::{serialize_document, YamlDocument};
use crate::py::python_types::pyobject_to_node;
use crate::serializer::{to_yaml_with_options, SerializeOptions};

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
                let node = pyobject_to_node(py, &item.unbind())?;
                py.detach(|| to_yaml_with_options(&node, options))
                    .map_err(|e| {
                        if e.contains("max depth exceeded") {
                            crate::YamlMaxDepthError::new_err(format_i18n_error(
                                "max-depth-exceeded",
                                &[("max_depth", &options.max_depth.to_string())],
                            ))
                        } else {
                            crate::YamlSerializeError::new_err(format_i18n_error(
                                "yaml-serialize-error",
                                &[("detail", &e)],
                            ))
                        }
                    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};
    use std::sync::Once;

    static PY_INIT: Once = Once::new();

    fn init_python() {
        // The crate does not enable pyo3's auto-initialize feature; tests that
        // need a Python token initialize the interpreter once, explicitly.
        PY_INIT.call_once(Python::initialize);
    }

    #[test]
    fn normalize_doc_ensures_single_trailing_newline() {
        assert_eq!(normalize_doc("a: 1"), "a: 1\n");
        assert_eq!(normalize_doc("a: 1\n"), "a: 1\n");
        assert_eq!(normalize_doc("a: 1\n\n"), "a: 1\n");
        assert_eq!(normalize_doc("a: 1\n...\n"), "a: 1\n...\n");
        assert_eq!(normalize_doc(""), "\n");
    }

    #[test]
    fn sink_writer_file_flushes_on_chunk_threshold() {
        init_python();
        let mut f = tempfile::tempfile().unwrap();
        let mut writer = SinkWriter::new(
            OutputSink::File(std::io::BufWriter::new(f.try_clone().unwrap())),
            14,
        );
        Python::attach(|py| {
            writer.write(py, "a: 1\n").unwrap(); // < 14B → 暂存
            assert_eq!(writer.pending.len(), 5);
            writer.write(py, "b: 2\nc: 3\n").unwrap(); // 累计 ≥14B → 落盘
            assert!(writer.pending.is_empty());
            writer.flush(py).unwrap();
        });
        drop(writer);
        f.seek(SeekFrom::Start(0)).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "a: 1\nb: 2\nc: 3\n");
    }

    #[test]
    fn sink_writer_file_flush_on_finish() {
        init_python();
        let mut f = tempfile::tempfile().unwrap();
        let mut writer = SinkWriter::new(
            OutputSink::File(std::io::BufWriter::new(f.try_clone().unwrap())),
            1024,
        );
        Python::attach(|py| {
            writer.write(py, "a: 1\n").unwrap();
            writer.flush(py).unwrap();
        });
        drop(writer);
        f.seek(SeekFrom::Start(0)).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "a: 1\n");
    }
}
