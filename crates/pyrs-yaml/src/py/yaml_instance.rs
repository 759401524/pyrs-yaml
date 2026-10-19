//! YAML — configured parser instance (rt / safe / full).

use pyo3::prelude::*;
use std::sync::Arc;

use crate::parser::yaml::registry;
use crate::py::convert::{
    directive_to_emit, format_i18n_error, negotiate_schema, node_to_pyobject_resolving_anchors,
    parse_schema,
};
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
    /// `None` means "no explicit schema": each document resolves under a
    /// honoured `%YAML` declaration, or Core when the flag is off.
    schema: Option<String>,
    honor_version_directive: bool,
    max_depth: usize,
    allow_duplicate_keys: bool,
}

#[pymethods]
impl YAML {
    #[new]
    /// Create a YAML instance; `typ` can be `rt`/`safe`/`full`.
    #[pyo3(signature = (typ: "str" = "rt", schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false))]
    fn new(
        typ: &str,
        schema: Option<&str>,
        max_depth: usize,
        allow_duplicate_keys: bool,
        honor_version_directive: bool,
    ) -> PyResult<Self> {
        let valid_types = ["rt", "safe", "full"];
        if !valid_types.contains(&typ) {
            return Err(YamlTypeError::new_err(format!(
                "Invalid YAML type: '{}'. Valid types: rt, safe, full",
                typ
            )));
        }
        if let Some(schema) = schema
            && !registry::exists(schema)
        {
            return Err(YamlTypeError::new_err(format!(
                "Invalid schema: '{}'. Valid schemas: core, yaml1.1, failsafe, json (plus any registered custom schemas)",
                schema
            )));
        }
        Ok(YAML {
            yaml_type: typ.to_string(),
            schema: schema.map(str::to_string),
            honor_version_directive,
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
            self.schema.as_deref(),
            self.max_depth,
            self.allow_duplicate_keys,
            self.honor_version_directive,
        )
    }

    /// Parse YAML into a dict/list (resolves anchors and merges).
    #[pyo3(signature = (yaml: "str") -> "dict[Any, Any] | list[Any]")]
    fn safe_load(&self, py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
        let explicit = self.schema.as_deref().map(parse_schema).transpose()?;
        let direct_schema = explicit
            .clone()
            .unwrap_or(crate::parser::yaml::Schema::Core);
        if !self.honor_version_directive {
            match crate::py::direct_load::try_direct_load(
                py,
                yaml,
                &direct_schema,
                self.max_depth,
                self.allow_duplicate_keys,
            ) {
                crate::py::direct_load::DirectOutcome::Done(v) => return Ok(v),
                crate::py::direct_load::DirectOutcome::Fail(e) => {
                    return Err(parse_error_to_py_err(e, yaml, self.max_depth));
                }
                crate::py::direct_load::DirectOutcome::Bail => {}
            }
        }
        let parsed = py.detach(|| {
            crate::parser::parse_document(
                yaml,
                true,
                direct_schema,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        let effective = negotiate_schema(
            explicit,
            parsed.version.as_deref(),
            self.honor_version_directive,
        );
        let ast = parsed.node;
        node_to_pyobject_resolving_anchors(&ast, py, &effective, yaml.bytes().any(|b| b == b'&'))
    }

    /// Parse multi-document YAML into a list of dicts/lists.
    #[pyo3(signature = (yaml: "str") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(&self, py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
        let explicit = self.schema.as_deref().map(parse_schema).transpose()?;
        let direct_schema = explicit
            .clone()
            .unwrap_or(crate::parser::yaml::Schema::Core);
        if !self.honor_version_directive && !yaml.trim().is_empty() {
            match crate::py::direct_load::try_direct_load(
                py,
                yaml,
                &direct_schema,
                self.max_depth,
                self.allow_duplicate_keys,
            ) {
                crate::py::direct_load::DirectOutcome::Done(v) => return Ok(vec![v]),
                crate::py::direct_load::DirectOutcome::Fail(e) => {
                    return Err(parse_error_to_py_err(e, yaml, self.max_depth));
                }
                crate::py::direct_load::DirectOutcome::Bail => {}
            }
        }
        let docs = py.detach(|| {
            crate::parser::parse_all_documents(
                yaml,
                true,
                direct_schema,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        let has_anchors = yaml.bytes().any(|b| b == b'&');
        let mut results = Vec::with_capacity(docs.len());
        for doc in docs {
            let effective = negotiate_schema(
                explicit.clone(),
                doc.version.as_deref(),
                self.honor_version_directive,
            );
            let obj = node_to_pyobject_resolving_anchors(&doc.node, py, &effective, has_anchors)?;
            results.push(obj);
        }
        Ok(results)
    }

    /// Parse a YAML file and return a `YamlDocument`.
    #[pyo3(signature = (path: "str") -> "YamlDocument")]
    fn parse_file(&self, py: Python, path: &str) -> PyResult<YamlDocument> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let explicit = self.schema.as_deref().map(parse_schema).transpose()?;
        let parse_schema_value = explicit
            .clone()
            .unwrap_or(crate::parser::yaml::Schema::Core);
        let content = crate::py::read_file_to_string(path)?;
        let parsed = py.detach(|| {
            crate::parser::parse_document(
                &content,
                resolve_merges,
                parse_schema_value,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, &content, self.max_depth))
        })?;
        let declared = parsed.version.as_deref();
        let effective = negotiate_schema(explicit, declared, self.honor_version_directive);
        let emit_version = directive_to_emit(self.honor_version_directive, declared, &effective);
        let source: Arc<str> = Arc::from(content);
        Ok(YamlDocument::new(
            parsed.node,
            effective,
            source,
            parsed.version,
            emit_version,
            self.honor_version_directive,
        ))
    }

    /// Parse multi-document YAML and return a list of `YamlDocument`.
    #[pyo3(signature = (yaml: "str") -> "list[YamlDocument]")]
    fn parse_all_docs(&self, py: Python, yaml: &str) -> PyResult<Vec<YamlDocument>> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let explicit = self.schema.as_deref().map(parse_schema).transpose()?;
        let parse_schema_value = explicit
            .clone()
            .unwrap_or(crate::parser::yaml::Schema::Core);
        let docs = py.detach(|| {
            crate::parser::parse_all_documents(
                yaml,
                resolve_merges,
                parse_schema_value,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| parse_error_to_py_err(e, yaml, self.max_depth))
        })?;
        let source: Arc<str> = Arc::from(yaml);
        Ok(docs
            .into_iter()
            .map(|doc| {
                let declared = doc.version.as_deref();
                let effective =
                    negotiate_schema(explicit.clone(), declared, self.honor_version_directive);
                let emit_version =
                    directive_to_emit(self.honor_version_directive, declared, &effective);
                YamlDocument::new(
                    doc.node,
                    effective,
                    source.clone(),
                    doc.version,
                    emit_version,
                    self.honor_version_directive,
                )
            })
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
        let file = crate::py::open_file(path)?;
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
