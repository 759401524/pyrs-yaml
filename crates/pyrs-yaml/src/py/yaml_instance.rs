//! YAML — configured parser instance (rt / safe / full).

use pyo3::prelude::*;
use std::sync::Arc;

use crate::parser::yaml::registry;
use crate::py::convert::{format_i18n_error, node_to_pyobject_resolving_anchors, parse_schema};
use crate::py::document::{YamlDocument, parse_document};
use crate::py::parse_error_to_py_err;
use crate::py::streaming::{ChunkCharIter, DEFAULT_CHUNK_SIZE, InputSrc, YamlStream};
use crate::py::writing::{OutputSink, SinkWriter, dump_iterable, dump_options};

use crate::YamlTypeError;

/// Configured YAML parser instance (rt / safe / full).
#[pyclass]
#[allow(clippy::upper_case_acronyms)]
pub(crate) struct YAML {
    yaml_type: String,
    schema: String,
    max_depth: usize,
    allow_duplicate_keys: bool,
}

#[pymethods]
impl YAML {
    #[new]
    /// Create a YAML instance; `typ` can be `rt`/`safe`/`full`.
    #[pyo3(signature = (typ: "str" = "rt", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false))]
    fn new(
        typ: &str,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<Self> {
        let valid_types = ["rt", "safe", "full"];
        if !valid_types.contains(&typ) {
            return Err(YamlTypeError::new_err(format!(
                "Invalid YAML type: '{}'. Valid types: rt, safe, full",
                typ
            )));
        }
        if !registry::exists(schema) {
            return Err(YamlTypeError::new_err(format!(
                "Invalid schema: '{}'. Valid schemas: core, yaml1.1, failsafe, json (plus any registered custom schemas)",
                schema
            )));
        }
        Ok(YAML {
            yaml_type: typ.to_string(),
            schema: schema.to_string(),
            max_depth,
            allow_duplicate_keys,
        })
    }

    /// Parse a YAML string and return a `YamlDocument`.
    #[pyo3(signature = (yaml: "str | bytes") -> "YamlDocument")]
    fn parse(&self, py: Python, yaml: &Bound<'_, PyAny>) -> PyResult<YamlDocument> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        parse_document(
            py,
            yaml,
            resolve_merges,
            &self.schema,
            self.max_depth,
            self.allow_duplicate_keys,
        )
    }

    /// Parse YAML into a dict/list (resolves anchors and merges).
    #[pyo3(signature = (yaml: "str") -> "dict[str, Any] | list[Any]")]
    fn safe_load(&self, py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
        let schema_enum = parse_schema(&self.schema)?;
        let schema_clone = schema_enum.clone();
        let ast = py.detach(|| {
            crate::parser::parse_with_options(
                yaml,
                true,
                schema_clone,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        node_to_pyobject_resolving_anchors(&ast, py, &schema_enum, yaml.bytes().any(|b| b == b'&'))
    }

    /// Parse multi-document YAML into a list of dicts/lists.
    #[pyo3(signature = (yaml: "str") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(&self, py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
        let schema_enum = parse_schema(&self.schema)?;
        let schema_clone = schema_enum.clone();
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                true,
                schema_clone,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        let has_anchors = yaml.bytes().any(|b| b == b'&');
        let mut results = Vec::with_capacity(asts.len());
        for ast in asts {
            let obj = node_to_pyobject_resolving_anchors(&ast, py, &schema_enum, has_anchors)?;
            results.push(obj);
        }
        Ok(results)
    }

    /// Parse a YAML file and return a `YamlDocument`.
    #[pyo3(signature = (path: "str") -> "YamlDocument")]
    fn parse_file(&self, py: Python, path: &str) -> PyResult<YamlDocument> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let schema_enum = parse_schema(&self.schema)?;
        let schema_clone = schema_enum.clone();
        let content = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let ast = py.detach(|| {
            crate::parser::parse_with_options(
                &content,
                resolve_merges,
                schema_clone,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, &content, self.max_depth))
        })?;
        let source: Arc<str> = Arc::from(content);
        Ok(YamlDocument::new(ast, schema_enum, source))
    }

    /// Parse multi-document YAML and return a list of `YamlDocument`.
    #[pyo3(signature = (yaml: "str") -> "list[YamlDocument]")]
    fn parse_all_docs(&self, py: Python, yaml: &str) -> PyResult<Vec<YamlDocument>> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let schema_enum = parse_schema(&self.schema)?;
        let schema_clone = schema_enum.clone();
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                resolve_merges,
                schema_clone,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        let source: Arc<str> = Arc::from(yaml);
        Ok(asts
            .into_iter()
            .map(|ast| YamlDocument::new(ast, schema_enum.clone(), source.clone()))
            .collect())
    }

    /// Lazy event iterator: incrementally read from `file_obj` (read() returns str or bytes).
    #[pyo3(signature = (file_obj: "Any") -> "YamlStream")]
    fn load_stream(&self, _py: Python, file_obj: Bound<'_, PyAny>) -> PyResult<YamlStream> {
        if file_obj.getattr("read").is_err() {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-readable",
                &[],
            )));
        }
        let src = InputSrc::PyObj(file_obj.unbind());
        Ok(YamlStream::new(ChunkCharIter::new(src, DEFAULT_CHUNK_SIZE)))
    }

    /// Lazy event iterator: incrementally read from file path (Rust File, no GIL blocking).
    #[pyo3(signature = (path: "str") -> "YamlStream")]
    fn load_stream_file(&self, _py: Python, path: &str) -> PyResult<YamlStream> {
        let file = std::fs::File::open(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let src = InputSrc::File(std::io::BufReader::new(file));
        Ok(YamlStream::new(ChunkCharIter::new(src, DEFAULT_CHUNK_SIZE)))
    }

    /// Streaming writer: serialize documents to `file_obj` (write(str)), constant memory.
    #[pyo3(signature = (file_obj: "Any", iterable: "Any", explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "None")]
    fn dump_stream(
        &self,
        py: Python,
        file_obj: Bound<'_, PyAny>,
        iterable: Bound<'_, PyAny>,
        explicit_start: bool,
        explicit_end: bool,
        sort_keys: bool,
    ) -> PyResult<()> {
        if file_obj.getattr("write").is_err() {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-writable",
                &[],
            )));
        }
        let mut writer = SinkWriter::new(
            OutputSink::PyObj(file_obj.unbind()),
            crate::py::writing::DEFAULT_CHUNK_SIZE,
        );
        dump_iterable(
            py,
            &mut writer,
            iterable,
            &dump_options(sort_keys),
            explicit_start,
            explicit_end,
        )
    }

    /// Streaming writer: serialize documents to `path` (Rust File, no GIL blocking).
    #[pyo3(signature = (path: "str", iterable: "Any", explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "None")]
    fn dump_file(
        &self,
        py: Python,
        path: &str,
        iterable: Bound<'_, PyAny>,
        explicit_start: bool,
        explicit_end: bool,
        sort_keys: bool,
    ) -> PyResult<()> {
        let file = std::fs::File::create(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-write-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let mut writer = SinkWriter::new(
            OutputSink::File(std::io::BufWriter::new(file)),
            crate::py::writing::DEFAULT_CHUNK_SIZE,
        );
        dump_iterable(
            py,
            &mut writer,
            iterable,
            &dump_options(sort_keys),
            explicit_start,
            explicit_end,
        )
    }
}
