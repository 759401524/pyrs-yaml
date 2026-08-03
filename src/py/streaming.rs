//! Streaming reader: chunked char iterator feeding saphyr `Parser`.
//!
//! `ChunkCharIter` adapts a Python file-like object or a `BufReader<File>` into
//! a `char` iterator that reads in fixed-size chunks, tracking UTF-8 boundary
//! state across chunk boundaries. Read failures / invalid UTF-8 set an error
//! flag (`take_error`), surfaced by `YamlStream` as a `PyErr`.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use saphyr_parser::{BufferedInput, Parser as SaphyrParser, Span};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::{Arc, Mutex};

use crate::parser::stream::event_to_stream_event;
use crate::parser::{StreamEvent, StreamEventType};
use crate::py::stream_events::stream_event_to_py_dict;

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
        }
    }

    /// Attach a shared error slot; subsequent reader errors are mirrored there.
    fn attach_error_slot(&mut self, slot: Arc<Mutex<Option<String>>>) {
        self.error_slot = Some(slot);
    }

    #[cfg(test)]
    fn with_error(msg: &str) -> Self {
        // 测试注入：error flag 已置位，src 不会被读取。
        Self {
            src: InputSrc::File(std::io::BufReader::new(tempfile::tempfile().unwrap())),
            pending: Vec::new(),
            buf: VecDeque::new(),
            chunk_size: 4,
            eof: true,
            error: Some(msg.to_string()),
            error_slot: None,
        }
    }

    fn set_error(&mut self, msg: String) {
        self.eof = true;
        self.error = Some(msg.clone());
        if let Some(slot) = &self.error_slot {
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(msg);
            }
        }
    }

    /// Take the pending error (once). Reader errors now surface to YamlStream
    /// via the shared slot; this accessor is test-only.
    #[cfg(test)]
    fn take_error(&mut self) -> Option<String> {
        self.error.take()
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

    fn push_bytes(&mut self, bytes: &[u8]) {
        if self.error.is_some() {
            return;
        }
        let mut combined = Vec::with_capacity(self.pending.len() + bytes.len());
        combined.extend_from_slice(&self.pending);
        combined.extend_from_slice(bytes);
        self.pending.clear();

        match std::str::from_utf8(&combined) {
            Ok(s) => self.buf.extend(s.chars()),
            Err(e) => {
                let valid = e.valid_up_to();
                // valid_up_to 保证 [..valid] 是合法 UTF-8 前缀。
                let valid_str = std::str::from_utf8(&combined[..valid]).unwrap_or("");
                self.buf.extend(valid_str.chars());
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
/// `__next__` parses exactly one event per call (driving the saphyr parser
/// incrementally), so a large file is never fully buffered. Errors surface
/// exactly once as a `YamlParseError`; after an error or `close()` the
/// iterator returns `None` (StopIteration).
#[pyclass(module = "pyrs_yaml")]
pub(crate) struct YamlStream {
    /// SAFETY: `'static` lifetime is a lie — the parser's lifetime parameter is
    /// for deserialization (which we don't use). The `ChunkCharIter` is owned
    /// by `BufferedInput`, which is owned by the parser, so no dangling refs.
    parser: Option<SaphyrParser<'static, BufferedInput<ChunkCharIter>>>,
    anchor_map: HashMap<usize, String>,
    finished: bool,
    pending_error: Option<String>,
    /// Shared reader-error slot (mirrors ChunkCharIter.error). The iterator is
    /// moved into the parser, so this is how YamlStream observes reader
    /// failures (invalid UTF-8 / read errors) at clean EOF.
    error_slot: Option<Arc<Mutex<Option<String>>>>,
    /// Whether the Python file object was closed by Drop (for repr).
    dropped: bool,
}

impl YamlStream {
    pub(crate) fn new(chars: ChunkCharIter) -> Self {
        let error_slot = Arc::new(Mutex::new(None));
        let mut chars = chars;
        chars.attach_error_slot(error_slot.clone());
        let parser = SaphyrParser::new_from_iter(chars);
        Self {
            parser: Some(parser),
            anchor_map: HashMap::new(),
            finished: false,
            pending_error: None,
            error_slot: Some(error_slot),
            dropped: false,
        }
    }

    /// 核心循环：取下一事件。返回 `(StreamEvent, Span)` 或 None（EOF/错误）。
    /// 在 `__next__` 内以 `py.detach(|| ...)` 调用。
    fn next_event_impl(&mut self) -> Option<(StreamEvent, Span)> {
        let parser = self.parser.as_mut()?;
        loop {
            match parser.next_event() {
                None => {
                    self.finished = true;
                    // 解析器遇 reader 错误视为干净 EOF——错误存于共享槽，
                    // 在此提取转为 pending_error（若尚未由扫描错误触发）。
                    self.pull_reader_error();
                    return None;
                }
                Some(Err(e)) => {
                    self.finished = true;
                    self.pending_error = Some(format!("YAML parse error: {}", e));
                    return None;
                }
                Some(Ok((event, span))) => {
                    let stream_event =
                        event_to_stream_event(event, span, &mut self.anchor_map, &mut |id| {
                            Some(format!("anchor_{}", id))
                        });
                    // D5: 文档级清空 —— 当前文档的 anchor 已全部消费完毕，
                    // 在转换之后清空，下个文档从头注册。
                    if matches!(
                        stream_event.as_ref().map(|e| &e.event_type),
                        Some(StreamEventType::DocumentEnd)
                    ) {
                        self.anchor_map.clear();
                    }
                    if let Some(ev) = stream_event {
                        return Some((ev, span));
                    }
                    // Event::Nothing → 继续循环
                }
            }
        }
    }

    fn pull_reader_error(&mut self) {
        if self.pending_error.is_some() {
            return;
        }
        if let Some(slot) = &self.error_slot {
            if let Ok(mut guard) = slot.lock() {
                if let Some(msg) = guard.take() {
                    self.pending_error = Some(format!("YAML parse error: {}", msg));
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    fn iter_from_bytes(data: &[u8]) -> ChunkCharIter {
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(data).unwrap();
        f.flush().unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        ChunkCharIter::new(InputSrc::File(std::io::BufReader::new(f)), 4)
    }

    fn collect(chars: ChunkCharIter) -> (String, Option<String>) {
        let mut it = chars;
        let mut s = String::new();
        while let Some(c) = it.next() {
            s.push(c);
        }
        let err = it.take_error();
        (s, err)
    }

    #[test]
    fn chunk_utf8_across_boundary() {
        // 多字节字符跨 chunk 边界（chunk=4 字节强制切分）
        let (s, err) = collect(iter_from_bytes("a: 你好\nb: 世界\n".as_bytes()));
        assert_eq!(err, None);
        assert_eq!(s, "a: 你好\nb: 世界\n");
    }

    #[test]
    fn chunk_invalid_utf8_sets_error() {
        let (s, err) = collect(iter_from_bytes(b"\xff\xfe"));
        let err = err.expect("invalid UTF-8 must set error");
        assert!(err.starts_with("Invalid UTF-8:"), "got: {}", err);
        assert!(s.is_empty());
    }

    #[test]
    fn chunk_eof_returns_none_cleanly() {
        let mut it = iter_from_bytes("a".as_bytes());
        assert_eq!(it.next(), Some('a'));
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);
        assert_eq!(it.take_error(), None);
    }

    #[test]
    fn chunk_read_error_sets_flag() {
        // 注入 error flag（目录路径模拟 read 失败不可靠，计划允许 with_error 构造器）
        let mut it = ChunkCharIter::with_error("injected read failure");
        assert_eq!(it.next(), None);
        assert_eq!(it.take_error(), Some("injected read failure".to_string()));
        assert_eq!(it.take_error(), None); // 一次性
    }

    #[test]
    fn yaml_stream_produces_events_from_chars() {
        let chars = iter_from_bytes("key: value\n".as_bytes());
        let mut ys = YamlStream::new(chars);
        let mut types = Vec::new();
        while let Some((ev, _)) = ys.next_event_impl() {
            types.push(format!("{:?}", ev.event_type));
        }
        // SS/DS/MS/SC/ME/DE/SE
        assert!(types.len() >= 6, "got {:?}", types);
        assert!(types.iter().any(|t| t.starts_with("Scalar")), "{:?}", types);
        assert_eq!(ys.pending_error, None);
        assert!(ys.finished);
    }

    #[test]
    fn yaml_stream_document_end_clears_anchor_map() {
        // 多文档：doc1 定义 &x，doc2 无 anchor —— DocumentEnd 后 anchor_map 为空。
        let chars = iter_from_bytes("a: &x 1\n---\nb: 2\n".as_bytes());
        let mut ys = YamlStream::new(chars);
        while let Some((_ev, _)) = ys.next_event_impl() {
            if ys.anchor_map.is_empty() {
                // 任何时刻 anchor_map 为空都合法（doc2 无 anchor）
            }
        }
        // 迭代结束后 anchor_map 应为空（最后一次 DocumentEnd 已清空）。
        assert!(ys.anchor_map.is_empty());
        assert_eq!(ys.pending_error, None);
    }

    #[test]
    fn yaml_stream_error_then_finished() {
        // 非法 YAML → pending_error 置位 + finished
        let chars = iter_from_bytes("key: {unclosed\n".as_bytes());
        let mut ys = YamlStream::new(chars);
        loop {
            let ev = ys.next_event_impl();
            if ev.is_none() {
                break;
            }
        }
        assert!(ys.pending_error.is_some(), "expected a parse error");
        assert!(ys.finished);
    }

    #[test]
    fn yaml_stream_close_stops_reading() {
        let chars = iter_from_bytes("key: value\n".as_bytes());
        let mut ys = YamlStream::new(chars);
        ys.close();
        assert!(ys.finished);
        assert!(ys.next_event_impl().is_none());
        assert!(ys.pending_error.is_none());
    }
}
