//! Sourcing a document's text from a file, optionally without copying it into
//! the heap first.
//!
//! The default build reads with [`std::fs::read_to_string`], exactly as the
//! binding always did. With the `mmap` feature a regular, non-empty file is
//! mapped read-only instead, so the parser reads straight through the page
//! cache and the only whole-file heap allocation left on the path is the
//! `Arc<str>` source an editable document keeps for round-tripping — that one
//! cannot be elided, because the document's lifetime must not ride on a mapping
//! of a file the caller can truncate from under it (a truncated map faults
//! `SIGBUS`).
//!
//! That lifetime constraint is also why the mapping is the *narrow* case. Empty
//! files, directories and other non-regular paths fall back to
//! `read_to_string` rather than being mapped, which keeps their error bytes —
//! `Is a directory (os error 21)`, for one — identical to the non-`mmap` build
//! instead of replacing them with a mapping error (`Mmap::map` on a directory
//! reports a different kind, and on an empty file errors outright). A caller
//! that gets a [`FileText::Mapped`] still has to validate it: [`FileText::as_str`]
//! returns the same `InvalidData` message `read_to_string` raises on a
//! non-UTF-8 file, so the two paths fail a bad byte the same way.

/// A file's text, owned or memory-mapped, kept alive so [`FileText::as_str`] can
/// hand out a borrow of it.
pub enum FileText {
    /// A copy made by `std::fs::read_to_string`.
    Read(alloc::string::String),
    /// A read-only memory map, present only with the `mmap` feature.
    #[cfg(feature = "mmap")]
    Mapped(memmap2::Mmap),
}

impl FileText {
    /// The text as a `&str`, validating it is UTF-8.
    ///
    /// A [`FileText::Read`] is already validated (so this never fails); a
    /// [`FileText::Mapped`] is checked here, and a bad byte is reported with the
    /// same `InvalidData` message `read_to_string` would have produced, so a
    /// caller maps the two paths' errors identically.
    pub fn as_str(&self) -> Result<&str, std::io::Error> {
        match self {
            FileText::Read(s) => Ok(s.as_str()),
            #[cfg(feature = "mmap")]
            FileText::Mapped(m) => std::str::from_utf8(m).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "stream did not contain valid UTF-8",
                )
            }),
        }
    }
}

/// Read a file's text for parsing.
///
/// With the `mmap` feature, a regular file of non-zero length is mapped and
/// returned as [`FileText::Mapped`]; every other path — empty file, directory,
/// special file, and the whole function in a non-`mmap` build — goes through
/// [`std::fs::read_to_string`] so its IO errors are the ones the caller already
/// expects.
pub fn read_file_text(path: &str) -> Result<FileText, std::io::Error> {
    #[cfg(feature = "mmap")]
    {
        let file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;
        if metadata.is_file() && metadata.len() > 0 {
            // SAFETY: a read-only map of a regular file the caller is about to
            // parse. It is unsound only if the file is truncated or otherwise
            // shrinks while mapped, which would fault `SIGBUS` mid-read — the
            // reason this path is an opt-in feature rather than the default.
            let mmap = unsafe { memmap2::Mmap::map(&file)? };
            return Ok(FileText::Mapped(mmap));
        }
        // Empty, a directory or a non-regular file: `read_to_string` is what the
        // caller's error handling already describes, so fall through to it.
    }
    Ok(FileText::Read(std::fs::read_to_string(path)?))
}

#[cfg(all(test, feature = "mmap"))]
mod tests {
    use super::*;
    // A `#![no_std]` crate gets no std prelude even under `cfg(test)`, so the
    // trait that powers `.to_string()` on the errors compared below must be in
    // scope explicitly.
    use std::io::Write;
    use std::string::ToString;

    use proptest::prelude::*;

    fn tempdir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("pyrs_file_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_regular_file_is_mapped() {
        let dir = tempdir("mapped");
        let path = dir.join("doc.yaml");
        std::fs::write(&path, b"a: 1\n").unwrap();
        match read_file_text(path.to_str().unwrap()).unwrap() {
            FileText::Mapped(_) => {}
            FileText::Read(_) => panic!("expected a memory map for a regular file"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_mapped_text_equals_the_read_text() {
        let dir = tempdir("parity");
        let path = dir.join("doc.yaml");
        std::fs::write(&path, "# n\na: 1\n  b: [2, 3]\n").unwrap();
        let p = path.to_str().unwrap();
        let mapped = read_file_text(p).unwrap();
        assert_eq!(
            mapped.as_str().unwrap(),
            std::fs::read_to_string(p).unwrap()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_empty_file_falls_back_to_the_read_path() {
        let dir = tempdir("empty");
        let path = dir.join("doc.yaml");
        std::fs::write(&path, b"").unwrap();
        match read_file_text(path.to_str().unwrap()).unwrap() {
            FileText::Read(s) => assert!(s.is_empty()),
            FileText::Mapped(_) => panic!("an empty file must not be mapped"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_nonexistent_path_reports_the_same_error_as_read_to_string() {
        let dir = tempdir("missing");
        let path = dir.join("absent.yaml");
        let p = path.to_str().unwrap();
        let mmap_err = read_file_text(p).map(|_| ()).unwrap_err().to_string();
        let std_err = std::fs::read_to_string(p)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert_eq!(mmap_err, std_err);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_directory_reports_the_same_error_as_read_to_string() {
        let dir = tempdir("dir");
        // A directory is not a regular file, so the fallback must carry
        // `read_to_string`'s own error rather than `Mmap::map`'s.
        let p = dir.to_str().unwrap();
        let mmap_err = read_file_text(p).map(|_| ()).err().map(|e| e.to_string());
        let std_err = std::fs::read_to_string(p)
            .map(|_| ())
            .err()
            .map(|e| e.to_string());
        // Both are errors (neither may succeed), and the message matches.
        assert!(mmap_err.is_some(), "a directory must not read as text");
        assert_eq!(mmap_err, std_err);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_bad_byte_fails_with_the_read_to_string_message() {
        let dir = tempdir("utf8");
        let path = dir.join("doc.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"a: \xff\xfe\n").unwrap();
        drop(f);
        let p = path.to_str().unwrap();
        let mapped = read_file_text(p).unwrap();
        let as_str_err = mapped.as_str().err().map(|e| e.to_string());
        let std_err = std::fs::read_to_string(p).err().map(|e| e.to_string());
        assert_eq!(
            as_str_err, std_err,
            "the invalid-UTF-8 message must match read_to_string"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    proptest! {
        // The whole point of the seam is that a mapped read is indistinguishable
        // from `read_to_string`. Six example inputs prove that shape; this proves
        // it over arbitrary UTF-8 — including the empty case that must take the
        // fallback, and multi-byte content whose length crosses page boundaries.
        #[test]
        fn mapped_read_matches_std_read_for_any_utf8(content in ".{0,400}") {
            let dir = tempdir("prop");
            let path = dir.join("doc.yaml");
            std::fs::write(&path, content.as_bytes()).unwrap();
            let p = path.to_str().unwrap();
            let ft = read_file_text(p).unwrap();
            prop_assert_eq!(ft.as_str().unwrap(), content.as_str());
            if content.is_empty() {
                prop_assert!(matches!(ft, FileText::Read(_)), "an empty file must use the fallback");
            } else {
                prop_assert!(matches!(ft, FileText::Mapped(_)), "a non-empty regular file must be mapped");
            }
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
