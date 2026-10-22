//! Streaming reader: chunked char iterator feeding saphyr `Parser`.
//!
//! `ChunkCharIter` adapts a Python file-like object or a `BufReader<File>` into
//! a `char` iterator that reads in fixed-size chunks, tracking UTF-8 boundary
//! state across chunk boundaries. Read failures / invalid UTF-8 set an error
//! flag (`take_error`), surfaced by `YamlStream` as a `PyErr`.

use granit_parser::{BufferedInput, Parser as SaphyrParser, Span};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::{Arc, Mutex};

use crate::ast::NodeMap;
use crate::parser::stream::event_to_stream_event;
use crate::parser::{StreamEvent, StreamEventType};
use crate::py::stream_events::stream_event_to_py_dict;

/// Char-index → byte-offset marks for a stream that is never held in memory.
///
/// `granit` reports spans as char indices, and the string-path receiver converts them with a table
/// built over the whole source. The incremental reader has no source to build a table from - that
/// is the point of it - so the iterator records one mark per multi-byte character, the only place
/// where the char→byte distance changes. Between two marks every character is ASCII, so a byte
/// offset is a char offset plus a constant; an ASCII-only document records nothing at all and a
/// lookup is one comparison.
///
/// Entries are dropped once a lookup has moved past them, so the table holds the span between the
/// last reported event and the parser's read-ahead, never the whole stream: a 100 MB document of
/// CJK costs the marks its largest token needs, not one per character. `load_stream`'s memory
/// bound is the reason this pruning exists, and it is the only reason it can be dropped.
#[derive(Default)]
pub(crate) struct ByteMarks {
    /// `(char index just past a multi-byte character, byte offset just past it)`, absolute.
    marks: VecDeque<(usize, usize)>,
    /// Absolute `(chars, bytes)` of the text behind the oldest retained mark. Between this point
    /// and that mark every character is ASCII, so a lookup there is the char index plus a constant.
    base: (usize, usize),
    chars: usize,
    bytes: usize,
}

impl ByteMarks {
    fn record_ascii(&mut self, len: usize) {
        self.chars += len;
        self.bytes += len;
    }

    fn record(&mut self, text: &str) {
        for c in text.chars() {
            self.chars += 1;
            self.bytes += c.len_utf8();
            if c.len_utf8() > 1 {
                self.marks.push_back((self.chars, self.bytes));
            }
        }
    }

    /// The byte offset of `char_index`, read from the last mark at or before it.
    fn byte_at(&self, char_index: usize) -> usize {
        debug_assert!(char_index >= self.base.0);
        let at = match self
            .marks
            .binary_search_by(|&(chars, _)| chars.cmp(&char_index))
        {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        match at {
            0 => self.base.1 + char_index.saturating_sub(self.base.0),
            at => {
                let (chars, bytes) = self.marks[at - 1];
                bytes + (char_index - chars)
            }
        }
    }

    /// Forget every mark at or before `char_index`, whose byte offset is `byte`.
    ///
    /// `granit` reports spans in source order, so no later event can name text that starts behind
    /// this one; the dropped prefix is answered by `base` instead. Called with each event's own
    /// start, never its end: a flow collection's span can cover its children, and pruning to the
    /// end would put the base past the children still to be reported.
    fn advance(&mut self, char_index: usize, byte: usize) {
        if char_index < self.base.0 {
            return;
        }
        while let Some(&(chars, _)) = self.marks.front() {
            if chars > char_index {
                break;
            }
            self.marks.pop_front();
        }
        self.base = (char_index, byte);
    }
}

/// Source of bytes for [`ChunkCharIter`].
pub(crate) enum InputSrc {
    /// Python file-like object (`Py<PyAny>`, Ungil) — `read(chunk_size)` per fill.
    PyObj(Py<PyAny>),
    /// Local file handle.
    File(std::io::BufReader<std::fs::File>),
}

/// Default chunk size for streaming reads (64 KiB).
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Char iterator that reads from [`InputSrc`] in chunks, splitting on UTF-8
/// boundaries and reporting read/decode failures via [`ChunkCharIter::take_error`].
pub(crate) struct ChunkCharIter {
    src: InputSrc,
    pending: Vec<u8>, // ≤3 字节的不完整 UTF-8 尾部
    buf: VecDeque<char>,
    chunk_size: usize,
    eof: bool,
    error: Option<String>,
    /// Shared error slot — YamlStream reads reader failures (invalid UTF-8,
    /// read errors) here once the parser sees a clean EOF.
    error_slot: Option<Arc<Mutex<Option<String>>>>,
    /// Shared char→byte marks: the iterator records them as it decodes, and
    /// YamlStream reads them to give a span a byte extent.
    byte_marks: Arc<Mutex<ByteMarks>>,
}

impl ChunkCharIter {
    pub(crate) fn new(src: InputSrc, chunk_size: usize) -> Self {
        Self {
            src,
            pending: Vec::new(),
            buf: VecDeque::with_capacity(chunk_size),
            chunk_size,
            eof: false,
            error: None,
            error_slot: None,
            byte_marks: Arc::new(Mutex::new(ByteMarks::default())),
        }
    }

    /// Attach a shared error slot; subsequent reader errors are mirrored there.
    fn attach_error_slot(&mut self, slot: Arc<Mutex<Option<String>>>) {
        self.error_slot = Some(slot);
    }

    fn set_error(&mut self, msg: String) {
        self.eof = true;
        self.error = Some(msg.clone());
        if let Some(slot) = &self.error_slot
            && let Ok(mut guard) = slot.lock()
        {
            *guard = Some(msg);
        }
    }

    fn fill(&mut self) {
        if self.eof || self.error.is_some() {
            return;
        }
        match &mut self.src {
            InputSrc::File(reader) => {
                let mut chunk = vec![0u8; self.chunk_size];
                match reader.read(&mut chunk) {
                    Ok(0) => self.eof = true,
                    Ok(n) => self.push_bytes(&chunk[..n]),
                    Err(e) => {
                        self.set_error(format!("Failed to read file: {}", e));
                    }
                }
            }
            InputSrc::PyObj(_) => self.fill_pyobj(),
        }
    }

    fn fill_pyobj(&mut self) {
        enum ReadResult {
            Bytes(Vec<u8>),
            BadType,
            Failed(String),
        }
        let chunk_size = self.chunk_size;
        // 闭包内仅借用 self.src（借用在 attach 返回后结束），避免在
        // self.push_bytes 期间持有对 self 的借用。
        let result = Python::attach(|py| {
            let obj = match &self.src {
                InputSrc::PyObj(o) => o,
                InputSrc::File(_) => return ReadResult::BadType,
            };
            match obj.call_method1(py, "read", (chunk_size,)) {
                Ok(val) => {
                    if let Ok(bytes) = val.extract::<Vec<u8>>(py) {
                        ReadResult::Bytes(bytes)
                    } else if let Ok(s) = val.extract::<String>(py) {
                        ReadResult::Bytes(s.into_bytes())
                    } else {
                        ReadResult::BadType
                    }
                }
                Err(e) => ReadResult::Failed(e.to_string()),
            }
        });
        match result {
            ReadResult::Bytes(bytes) => {
                if bytes.is_empty() {
                    // Python 文件对象 read() 在 EOF 返回 b"" / ""——同 File::read
                    // 的 Ok(0)，必须置 eof 否则 next() 无限循环。
                    self.eof = true;
                } else {
                    self.push_bytes(&bytes);
                }
            }
            ReadResult::BadType => {
                self.set_error("Expected str or bytes from file object read()".to_string());
            }
            ReadResult::Failed(msg) => {
                self.set_error(format!("file read failed: {}", msg));
            }
        }
    }

    /// Hand a decoded run to the parser and index where its bytes sit.
    fn push_chars(&mut self, text: &str) {
        self.buf.extend(text.chars());
        let mut marks = match self.byte_marks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if text.is_ascii() {
            marks.record_ascii(text.len());
        } else {
            marks.record(text);
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        if self.error.is_some() {
            return;
        }
        let mut combined = Vec::with_capacity(self.pending.len() + bytes.len());
        combined.extend_from_slice(&self.pending);
        combined.extend_from_slice(bytes);
        self.pending.clear();

        match std::str::from_utf8(&combined) {
            Ok(s) => self.push_chars(s),
            Err(e) => {
                let valid = e.valid_up_to();
                // valid_up_to 保证 [..valid] 是合法 UTF-8 前缀。
                let valid_str = std::str::from_utf8(&combined[..valid]).unwrap_or("");
                self.push_chars(valid_str);
                let rest = &combined[valid..];
                match e.error_len() {
                    // 完整非法序列（拼接更多字节也无法修复）→ 报错。
                    Some(_) => {
                        self.set_error(format!("Invalid UTF-8: {:?}", rest));
                    }
                    // 不完整多字节序列尾部（≤3 字节）→ 留待下一 chunk 拼接。
                    None => self.pending = rest.to_vec(),
                }
            }
        }
    }
}

impl Iterator for ChunkCharIter {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        loop {
            if let Some(c) = self.buf.pop_front() {
                return Some(c);
            }
            if !self.pending.is_empty() && self.eof {
                // EOF 时遗留的不完整 UTF-8 尾部 → 截断错误。
                self.set_error(format!("Invalid UTF-8 (truncated): {:?}", self.pending));
                self.pending.clear();
                return None;
            }
            if self.eof || self.error.is_some() {
                return None;
            }
            self.fill();
        }
    }
}

/// Lazy, constant-memory stream of YAML documents from a [`ChunkCharIter`].
///
/// `__next__` parses exactly one event per call (driving the granit parser
/// incrementally), so a large file is never fully buffered. Errors surface
/// exactly once as a `YamlParseError`; after an error or `close()` the
/// iterator returns `None` (StopIteration).
#[pyclass(module = "pyrs_yaml")]
pub(crate) struct YamlStream {
    /// SAFETY: `'static` lifetime is a lie — the parser's lifetime parameter is
    /// for deserialization (which we don't use). The `ChunkCharIter` is owned
    /// by `BufferedInput`, which is owned by the parser, so no dangling refs.
    parser: Option<SaphyrParser<'static, BufferedInput<ChunkCharIter>>>,
    anchor_map: NodeMap<usize, String>,
    finished: bool,
    pending_error: Option<String>,
    /// Shared reader-error slot (mirrors ChunkCharIter.error). The iterator is
    /// moved into the parser, so this is how YamlStream observes reader
    /// failures (invalid UTF-8 / read errors) at clean EOF.
    error_slot: Option<Arc<Mutex<Option<String>>>>,
    /// Shared char→byte marks, for the same reason: the iterator decoded the
    /// text and dropped it, so this is where its byte positions survive.
    byte_marks: Arc<Mutex<ByteMarks>>,
    /// Whether the Python file object was closed by Drop (for repr).
    dropped: bool,
}

impl YamlStream {
    pub(crate) fn new(chars: ChunkCharIter) -> Self {
        let error_slot = Arc::new(Mutex::new(None));
        let mut chars = chars;
        chars.attach_error_slot(error_slot.clone());
        let byte_marks = chars.byte_marks.clone();
        let parser = SaphyrParser::new_from_iter(chars);
        Self {
            parser: Some(parser),
            anchor_map: NodeMap::default(),
            finished: false,
            pending_error: None,
            error_slot: Some(error_slot),
            byte_marks,
            dropped: false,
        }
    }

    /// The core loop: pull the next event, or `None` at EOF or after an error.
    ///
    /// Called from `__next__` inside `py.detach`. The event detaches here as well, because the
    /// incremental reader rotates the buffer a borrowed event would point into — a borrow of
    /// `self` would outlive exactly the call that is allowed to move it.
    fn next_event_impl(&mut self) -> Option<(StreamEvent<'static>, Span)> {
        let parser = self.parser.as_mut()?;
        loop {
            match parser.next_event() {
                None => {
                    self.finished = true;
                    // A reader error reads as a clean EOF here: the message sits in the shared
                    // slot and is lifted into `pending_error` (unless a scan error got there first).
                    self.pull_reader_error();
                    return None;
                }
                Some(Err(e)) => {
                    self.finished = true;
                    self.pending_error = Some(format!("YAML parse error: {}", e));
                    return None;
                }
                Some(Ok((event, span))) => {
                    // The parser reports char indices; the marks turn both ends into byte
                    // offsets. One uncontended lock per event, and nothing recorded for an
                    // ASCII source, whose two indices are already byte offsets.
                    let mut marks = match self.byte_marks.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    let offset = marks.byte_at(span.start.index());
                    let end = marks.byte_at(span.end.index());
                    marks.advance(span.start.index(), offset);
                    drop(marks);
                    let stream_event = event_to_stream_event(
                        event,
                        span,
                        offset,
                        end,
                        &mut self.anchor_map,
                        &mut |id| Some(format!("anchor_{}", id)),
                    );
                    // D5: per-document reset — every anchor of this document has been consumed, so
                    // clear after the conversion and let the next one register from scratch.
                    if matches!(
                        stream_event.as_ref().map(|e| &e.event_type),
                        Some(StreamEventType::DocumentEnd)
                    ) {
                        self.anchor_map.clear();
                    }
                    if let Some(ev) = stream_event {
                        return Some((ev.into_owned(), span));
                    }
                    // granit is `non_exhaustive`: an event this adapter does not map → keep pulling.
                }
            }
        }
    }

    fn pull_reader_error(&mut self) {
        if self.pending_error.is_some() {
            return;
        }
        if let Some(slot) = &self.error_slot
            && let Ok(mut guard) = slot.lock()
            && let Some(msg) = guard.take()
        {
            self.pending_error = Some(format!("YAML parse error: {}", msg));
        }
    }
}

#[pymethods]
impl YamlStream {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'a>(&mut self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDict>>> {
        if self.finished {
            return Ok(None);
        }
        // 错误在首次 next_event 时取出并抛出一次。
        let result = py.detach(|| self.next_event_impl());
        if let Some(err_msg) = self.pending_error.take() {
            self.finished = true;
            return Err(crate::YamlParseError::new_err(err_msg));
        }
        let Some((event, _span)) = result else {
            self.finished = true;
            return Ok(None);
        };
        Ok(Some(stream_event_to_py_dict(py, &event)?))
    }

    /// 提前终止：停止读取后续 chunk。幂等。
    fn close(&mut self) {
        self.finished = true;
        self.parser = None;
    }

    fn __repr__(&self) -> String {
        if self.finished || self.dropped {
            "<YamlStream finished>".to_string()
        } else {
            "<YamlStream running>".to_string()
        }
    }

    fn __del__(&mut self) {
        self.finished = true;
        self.parser = None;
        self.dropped = true;
    }
}

impl Drop for YamlStream {
    fn drop(&mut self) {
        self.parser = None;
        self.dropped = true;
    }
}
