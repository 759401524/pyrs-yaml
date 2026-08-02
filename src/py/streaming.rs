//! Streaming reader: chunked char iterator feeding saphyr `Parser`.
//!
//! `ChunkCharIter` adapts a Python file-like object or a `BufReader<File>` into
//! a `char` iterator that reads in fixed-size chunks, tracking UTF-8 boundary
//! state across chunk boundaries. Read failures / invalid UTF-8 set an error
//! flag (`take_error`), surfaced by `YamlStream` as a `PyErr`.

use pyo3::prelude::*;
use std::collections::VecDeque;
use std::io::Read;

/// Source of bytes for [`ChunkCharIter`].
#[allow(dead_code)] // Task 3 提供基础设施；Task 4 YamlStream 消费后移除
pub(crate) enum InputSrc {
    /// Python file-like object (`Py<PyAny>`, Ungil) — `read(chunk_size)` per fill.
    PyObj(Py<PyAny>),
    /// Local file handle.
    File(std::io::BufReader<std::fs::File>),
}

/// Default chunk size for streaming reads (64 KiB).
#[allow(dead_code)] // Task 3 提供基础设施；Task 4 YamlStream 消费后移除
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Char iterator that reads from [`InputSrc`] in chunks, splitting on UTF-8
/// boundaries and reporting read/decode failures via [`ChunkCharIter::take_error`].
#[allow(dead_code)] // Task 3 提供基础设施；Task 4 YamlStream 消费后移除
pub(crate) struct ChunkCharIter {
    src: InputSrc,
    pending: Vec<u8>, // ≤3 字节的不完整 UTF-8 尾部
    buf: VecDeque<char>,
    chunk_size: usize,
    eof: bool,
    error: Option<String>,
}

#[allow(dead_code)] // Task 3 提供基础设施；Task 4 YamlStream 消费后移除
impl ChunkCharIter {
    pub(crate) fn new(src: InputSrc, chunk_size: usize) -> Self {
        Self {
            src,
            pending: Vec::new(),
            buf: VecDeque::with_capacity(chunk_size),
            chunk_size,
            eof: false,
            error: None,
        }
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
        }
    }

    /// Take the pending error (once).
    pub(crate) fn take_error(&mut self) -> Option<String> {
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
                        self.eof = true;
                        self.error = Some(format!("Failed to read file: {}", e));
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
            ReadResult::Bytes(bytes) => self.push_bytes(&bytes),
            ReadResult::BadType => {
                self.eof = true;
                self.error = Some("Expected str or bytes from file object read()".to_string());
            }
            ReadResult::Failed(msg) => {
                self.eof = true;
                self.error = Some(format!("file read failed: {}", msg));
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
                        self.eof = true;
                        self.error = Some(format!("Invalid UTF-8: {:?}", rest));
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
                self.error = Some(format!("Invalid UTF-8 (truncated): {:?}", self.pending));
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
}
