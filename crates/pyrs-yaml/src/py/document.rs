//! YamlDocument — round-trip editable YAML document.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

use crate::ast::CustomNode;
use crate::parser::yaml::Schema;
use crate::serializer::{SerializeOptions, to_yaml_with_options};
use crate::splice::SpliceState;

use crate::py::convert::{
    format_i18n_error, node_to_pyobject_resolving_anchors, node_to_pyobject_simple, parse_schema,
};
use crate::py::editing;
use crate::py::editing::segment_py::SegmentExt;
use crate::py::parse_error_to_py_err;
use crate::py::python_types::pyobject_to_node;
use crate::py::walk_helpers::{walk_ast, walk_scalars};

#[derive(Clone)]
pub(crate) struct DocumentSnapshot {
    pub(crate) ast: CustomNode,
    pub(crate) splice: Option<SpliceState>,
    pub(crate) source: Option<Arc<str>>,
    pub(crate) revision: u64,
    pub(crate) source_dirty: bool,
    pub(crate) splice_checked: bool,
}

#[pyclass]
/// Round-trip editable YAML document with transaction support, path editing, and source preservation.
pub struct YamlDocument {
    pub(crate) ast: CustomNode,
    pub(crate) schema: Schema,
    pub(crate) source: Option<Arc<str>>,
    pub(crate) version: String,
    pub(crate) revision: u64, // bumped on every mutation (Node invalidation)
    pub(crate) source_dirty: bool, // lazy source re-serialization flag
    /// Segment-splice accumulator for default-layout documents; `None`
    /// when the doc is not splice-eligible or the state was consumed.
    pub(crate) splice: Option<SpliceState>,
    /// Whether splice eligibility has been computed (lazily, on first edit).
    pub(crate) splice_checked: bool,
    /// Transaction snapshot stack. `__enter__` pushes, `__exit__` pops.
    /// Grows only as deep as `with` nesting; `None` means empty stack.
    pub(crate) snapshot: Vec<DocumentSnapshot>,
}

/// Serialize a document's AST with the given options, flushing any pending
/// splice edits first. Shared by `to_yaml_with_options` and streaming
/// write. The serialize itself runs detached from the GIL.
pub(crate) fn serialize_document(
    doc: &mut YamlDocument,
    py: Python,
    options: &SerializeOptions,
) -> PyResult<String> {
    doc.flush_source(py)?;
    py.detach(|| to_yaml_with_options(&doc.ast, options))
        .map_err(|e| {
            if e.to_string().contains("max depth exceeded") {
                YamlMaxDepthError::new_err(format_i18n_error(
                    "max-depth-exceeded",
                    &[("max_depth", &options.max_depth.to_string())],
                ))
            } else {
                YamlSerializeError::new_err(format_i18n_error(
                    "yaml-serialize-error",
                    &[("detail", &e.to_string())],
                ))
            }
        })
}

impl YamlDocument {
    /// Uniform constructor for a freshly parsed document: all entries start
    /// at revision 0 with splice eligibility uncomputed. Every parse entry
    /// point (`parse`, `parse_file`, `parse_all_docs`, `YAML` variants) must
    /// go through here so derived state stays in sync.
    pub(crate) fn new(ast: CustomNode, schema: Schema, source: Arc<str>) -> Self {
        YamlDocument {
            ast,
            schema,
            source: Some(source),
            version: "1.2".to_string(),
            revision: 0,
            source_dirty: false,
            splice: None,
            splice_checked: false,
            snapshot: vec![],
        }
    }

    /// Non-`#[pymethod]` constructor used by benches and integration tests.
    /// Splice eligibility is computed lazily on first edit via
    /// [`ensure_splice`], so `splice` starts `None`.
    #[allow(dead_code)] // pub but struct is not pub — used by benches (separate crate)
    pub fn from_ast(ast: CustomNode, source: Arc<str>) -> Self {
        YamlDocument::new(ast, crate::parser::yaml::Schema::Core, source)
    }

    /// Lazily compute splice eligibility on first edit. `splice_checked`
    /// distinguishes "not yet computed" from "consumed or ineligible" —
    /// the single-burst model must NOT re-create state after materialize.
    ///
    /// Field-reference form so edit closures keep disjoint field captures.
    pub(crate) fn ensure_splice(
        splice: &mut Option<SpliceState>,
        checked: &mut bool,
        ast: &CustomNode,
        source: &Option<Arc<str>>,
    ) {
        if *checked || splice.is_some() {
            return;
        }
        *checked = true;
        if let Some(src) = source
            && crate::parser::check_default_layout(ast, src)
        {
            *splice = Some(SpliceState::new(src.clone()));
        }
    }
}

#[pymethods]
impl YamlDocument {
    /// Lazily re-serialize the AST into `source` if edits have occurred.
    fn flush_source(&mut self, py: Python) -> PyResult<()> {
        if self.source_dirty {
            // Splice path: replay the accumulated segments against the
            // original source. Runs at most once — the state is consumed
            // (set to None) whether or not it materializes, so a later
            // burst falls back to a full serialize.
            let spliced = py.detach(|| self.splice.as_ref().and_then(|s| s.materialize()));
            let new_yaml = match spliced {
                Some(text) => text,
                None => py
                    .detach(|| {
                        crate::serializer::to_yaml_with_options(
                            &self.ast,
                            &SerializeOptions::default(),
                        )
                    })
                    .map_err(|e| {
                        YamlSerializeError::new_err(format_i18n_error(
                            "yaml-serialize-error",
                            &[("detail", &e.to_string())],
                        ))
                    })?,
            };
            self.source = Some(Arc::from(new_yaml));
            self.splice = None;
            self.source_dirty = false;
        }
        Ok(())
    }

    /// Serialize the document to a YAML string (default 2-space indent).
    #[allow(clippy::wrong_self_convention)] // needs &mut self to flush lazy source
    fn to_yaml(&mut self, py: Python) -> PyResult<String> {
        self.flush_source(py)?;
        self.to_yaml_with_options(py, 2, false, false, false, 1000, 80, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::wrong_self_convention)]
    #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false, max_depth: "int" = 1000, width: "int" = 80, indent_mapping: "int | None" = None, indent_sequence: "int | None" = None, indent_offset: "int | None" = None) -> "str")]
    /// Serialize with customizable indent, sorting, and explicit start/end markers.
    fn to_yaml_with_options(
        &mut self,
        py: Python,
        indent_size: usize,
        explicit_start: bool,
        explicit_end: bool,
        sort_keys: bool,
        max_depth: usize,
        width: usize,
        indent_mapping: Option<usize>,
        indent_sequence: Option<usize>,
        indent_offset: Option<usize>,
    ) -> PyResult<String> {
        self.flush_source(py)?;
        let options = SerializeOptions {
            indent_size,
            explicit_start,
            explicit_end,
            sort_keys,
            max_depth,
            width,
            indent_mapping: indent_mapping.unwrap_or(indent_size),
            indent_sequence: indent_sequence.unwrap_or(indent_size),
            indent_offset: indent_offset.unwrap_or(0),
        };
        to_yaml_with_options(&self.ast, &options).map_err(|e| {
            if e.to_string().contains("max depth exceeded") {
                YamlMaxDepthError::new_err(format_i18n_error(
                    "max-depth-exceeded",
                    &[("max_depth", &max_depth.to_string())],
                ))
            } else {
                YamlSerializeError::new_err(format_i18n_error(
                    "yaml-serialize-error",
                    &[("detail", &e.to_string())],
                ))
            }
        })
    }

    /// Convert the document to a Python dict/list, resolving anchor references.
    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let has_anchors = self.source.as_deref().is_none_or(|s| s.contains('&'));
        node_to_pyobject_resolving_anchors(&self.ast, py, &self.schema, has_anchors)
    }

    /// Access a value by top-level mapping key, returning `default` if not found.
    /// Path-based access is available via `find()` / `node()`.
    #[pyo3(signature = (key: "str", default: "Any" = None) -> "Any")]
    fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                let key_node = CustomNode::plain_scalar(key);
                if let Some(value) = pairs.get(&key_node) {
                    Ok(node_to_pyobject_simple(value, py, &self.schema)?)
                } else {
                    Ok(default.unwrap_or_else(|| py.None()))
                }
            }
            _ => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Return the root node type: `scalar`/`mapping`/`sequence`/`null`/`alias`.
    fn root_type(&self) -> String {
        match &self.ast {
            CustomNode::Scalar { .. } => "scalar".to_string(),
            CustomNode::Mapping { .. } => "mapping".to_string(),
            CustomNode::Sequence { .. } => "sequence".to_string(),
            CustomNode::Null { .. } => "null".to_string(),
            CustomNode::Alias { .. } => "alias".to_string(),
        }
    }

    /// Return a `YamlDocument(...)` representation.
    fn __repr__(&mut self, py: Python) -> String {
        format!("YamlDocument({})", self.to_yaml(py).unwrap_or_default())
    }

    /// Return the YAML string representation.
    fn __str__(&mut self, py: Python) -> String {
        self.to_yaml(py)
            .unwrap_or_else(|_| "YamlDocument(error)".to_string())
    }

    /// Check if a key exists in the mapping.
    fn __contains__(&self, key: &str) -> bool {
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => pairs.contains_key(&CustomNode::plain_scalar(key)),
            _ => false,
        }
    }

    /// Return the number of mapping entries or sequence length.
    fn __len__(&self) -> usize {
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => pairs.len(),
            CustomNode::Sequence { items, .. } => items.len(),
            _ => 0,
        }
    }

    /// Return an iterator over keys (mapping) or values (sequence).
    fn __iter__<'py>(&self, _py: Python<'py>) -> PyResult<Py<PyAny>> {
        let schema = self.schema.clone();
        Python::attach(|py| match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                let keys: Vec<Py<PyAny>> = pairs
                    .keys()
                    .map(|k| node_to_pyobject_simple(k, py, &schema))
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(keys.into_pyobject(py)?.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let values: Vec<Py<PyAny>> = items
                    .iter()
                    .map(|v| node_to_pyobject_simple(v, py, &schema))
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(values.into_pyobject(py)?.into_any().unbind())
            }
            _ => Ok(Vec::<Py<PyAny>>::new()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
        })
    }

    /// Access a child node by key (mapping) or index (sequence).
    fn __getitem__<'py>(&self, py: Python<'py>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let schema = &self.schema;
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                if let Ok(key_str) = key.bind(py).extract::<String>() {
                    let key_node = CustomNode::plain_scalar(key_str.clone());
                    if let Some(value) = pairs.get(&key_node) {
                        Ok(node_to_pyobject_simple(value, py, schema)?)
                    } else {
                        Err(pyo3::exceptions::PyKeyError::new_err(format_i18n_error(
                            "key-not-found",
                            &[("key", key_str.as_str())],
                        )))
                    }
                } else {
                    Err(YamlTypeError::new_err(format_i18n_error(
                        "key-not-string",
                        &[],
                    )))
                }
            }
            CustomNode::Sequence { items, .. } => {
                if let Ok(idx) = key.bind(py).extract::<usize>() {
                    if idx < items.len() {
                        Ok(node_to_pyobject_simple(&items[idx], py, schema)?)
                    } else {
                        Err(pyo3::exceptions::PyIndexError::new_err(format_i18n_error(
                            "index-out-of-range",
                            &[
                                ("index", &idx.to_string()),
                                ("len", &items.len().to_string()),
                            ],
                        )))
                    }
                } else {
                    Err(YamlTypeError::new_err(format_i18n_error(
                        "index-not-integer",
                        &[],
                    )))
                }
            }
            _ => Err(YamlTypeError::new_err(format_i18n_error(
                "not-subscriptable",
                &[],
            ))),
        }
    }

    /// Return the current YAML source string.
    fn source(&mut self, py: Python) -> PyResult<String> {
        self.flush_source(py)?;
        Ok(self.source.as_deref().unwrap_or("").to_string())
    }

    /// Walk the AST depth-first, returning a list of path tuples.
    /// Each path tuple contains strings (mapping keys) and ints (sequence indices).
    /// The first element is always the root node (empty path).
    fn _walk_paths(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
        let mut paths = Vec::new();
        let mut path = Vec::new();
        walk_ast(&self.ast, &mut path, &mut paths, py)?;
        Ok(paths)
    }

    /// Walk only scalar/null nodes, returning their path tuples.
    fn _scalar_paths(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
        let mut paths = Vec::new();
        let mut path = Vec::new();
        walk_scalars(&self.ast, &mut path, &mut paths, py)?;
        Ok(paths)
    }

    /// Set a value by path (internal, called by `__setitem__`).
    #[pyo3(signature = (segments: "list", value: "Any", create_missing: "bool" = false) -> "None")]
    fn _set_path(
        &mut self,
        py: Python,
        segments: Vec<Py<PyAny>>,
        value: Py<PyAny>,
        create_missing: bool,
    ) -> PyResult<()> {
        let segs: Vec<editing::Segment<'_>> = segments
            .iter()
            .map(|s| editing::Segment::from_py(s.bind(py)))
            .collect::<Result<Vec<_>, pyo3::PyErr>>()
            .map_err(|e| YamlEditError::new_err(e.to_string()))?;
        let new_node = pyobject_to_node(py, &value)?;
        py.detach(|| -> Result<(), String> {
            let src = self.source.as_deref().unwrap_or("");
            Self::ensure_splice(
                &mut self.splice,
                &mut self.splice_checked,
                &self.ast,
                &self.source,
            );
            let unit = {
                let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                editing::set_path(
                    &mut self.ast,
                    &segs,
                    new_node,
                    true,
                    src,
                    offsets,
                    create_missing,
                )?
            };
            if let Some(state) = self.splice.as_mut()
                && state.apply(&unit).is_err()
            {
                self.splice = None;
            }
            Ok(())
        })
        .map_err(|e| YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)])))?;
        self.revision = self.revision.wrapping_add(1);
        self.source_dirty = true;
        Ok(())
    }

    /// Insert a value by path (internal, inserts at a sequence position).
    #[pyo3(signature = (segments: "list", index: "int", value: "Any") -> "None")]
    fn _insert_path(
        &mut self,
        py: Python,
        segments: Vec<Py<PyAny>>,
        index: i64,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        let segs: Vec<editing::Segment<'_>> = segments
            .iter()
            .map(|s| editing::Segment::from_py(s.bind(py)))
            .collect::<Result<Vec<_>, pyo3::PyErr>>()
            .map_err(|e| YamlEditError::new_err(e.to_string()))?;
        let new_node = pyobject_to_node(py, &value)?;
        py.detach(|| -> Result<(), String> {
            let src = self.source.as_deref().unwrap_or("");
            Self::ensure_splice(
                &mut self.splice,
                &mut self.splice_checked,
                &self.ast,
                &self.source,
            );
            let unit = {
                let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                editing::insert_path(&mut self.ast, &segs, index, new_node, src, offsets)?
            };
            if let Some(state) = self.splice.as_mut()
                && state.apply(&unit).is_err()
            {
                self.splice = None;
            }
            Ok(())
        })
        .map_err(|e| YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)])))?;
        self.revision = self.revision.wrapping_add(1);
        self.source_dirty = true;
        Ok(())
    }

    /// Append a value by path (internal, appends to a sequence).
    #[pyo3(signature = (segments: "list", value: "Any") -> "None")]
    fn _append_path(
        &mut self,
        py: Python,
        segments: Vec<Py<PyAny>>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        let segs: Vec<editing::Segment<'_>> = segments
            .iter()
            .map(|s| editing::Segment::from_py(s.bind(py)))
            .collect::<Result<Vec<_>, pyo3::PyErr>>()
            .map_err(|e| YamlEditError::new_err(e.to_string()))?;
        let new_node = pyobject_to_node(py, &value)?;
        py.detach(|| -> Result<(), String> {
            let src = self.source.as_deref().unwrap_or("");
            Self::ensure_splice(
                &mut self.splice,
                &mut self.splice_checked,
                &self.ast,
                &self.source,
            );
            let unit = {
                let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                editing::append_path(&mut self.ast, &segs, new_node, src, offsets)?
            };
            if let Some(state) = self.splice.as_mut()
                && state.apply(&unit).is_err()
            {
                self.splice = None;
            }
            Ok(())
        })
        .map_err(|e| YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)])))?;
        self.revision = self.revision.wrapping_add(1);
        self.source_dirty = true;
        Ok(())
    }

    /// Delete a node by path (internal, called by `__delitem__`).
    #[pyo3(signature = (segments: "list") -> "None")]
    fn _delete_path(&mut self, py: Python, segments: Vec<Py<PyAny>>) -> PyResult<()> {
        let segs: Vec<editing::Segment<'_>> = segments
            .iter()
            .map(|s| editing::Segment::from_py(s.bind(py)))
            .collect::<Result<Vec<_>, pyo3::PyErr>>()
            .map_err(|e| YamlEditError::new_err(e.to_string()))?;
        py.detach(|| -> Result<(), String> {
            let src = self.source.as_deref().unwrap_or("");
            Self::ensure_splice(
                &mut self.splice,
                &mut self.splice_checked,
                &self.ast,
                &self.source,
            );
            let unit = {
                let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                editing::delete_path(&mut self.ast, &segs, src, offsets)?
            };
            if let Some(state) = self.splice.as_mut()
                && state.apply(&unit).is_err()
            {
                self.splice = None;
            }
            Ok(())
        })
        .map_err(|e| YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)])))?;
        self.revision = self.revision.wrapping_add(1);
        self.source_dirty = true;
        Ok(())
    }

    /// Rename a mapping key by path (internal).
    #[pyo3(signature = (segments: "list", new_key: "str") -> "None")]
    fn _rename_path(
        &mut self,
        py: Python,
        segments: Vec<Py<PyAny>>,
        new_key: &str,
    ) -> PyResult<()> {
        let segs: Vec<editing::Segment<'_>> = segments
            .iter()
            .map(|s| editing::Segment::from_py(s.bind(py)))
            .collect::<Result<Vec<_>, pyo3::PyErr>>()
            .map_err(|e| YamlEditError::new_err(e.to_string()))?;
        py.detach(|| -> Result<(), String> {
            let src = self.source.as_deref().unwrap_or("");
            Self::ensure_splice(
                &mut self.splice,
                &mut self.splice_checked,
                &self.ast,
                &self.source,
            );
            let unit = {
                let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                editing::rename_path(&mut self.ast, &segs, new_key, src, offsets)?
            };
            if let Some(state) = self.splice.as_mut()
                && state.apply(&unit).is_err()
            {
                self.splice = None;
            }
            Ok(())
        })
        .map_err(|e| YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)])))?;
        self.revision = self.revision.wrapping_add(1);
        self.source_dirty = true;
        Ok(())
    }

    /// Set the value for a mapping key, `doc['key'] = value`.
    fn __setitem__(&mut self, py: Python, key: String, value: Py<PyAny>) -> PyResult<()> {
        self._set_path(
            py,
            vec![key.into_pyobject(py)?.into_any().unbind()],
            value,
            false,
        )
    }

    /// Delete a mapping key, `del doc['key']`.
    fn __delitem__(&mut self, py: Python, key: String) -> PyResult<()> {
        self._delete_path(py, vec![key.into_pyobject(py)?.into_any().unbind()])
    }

    /// Return the current revision number (incremented on each edit).
    fn _revision(&self) -> u64 {
        self.revision
    }

    /// Return the YAML version string.
    fn version(&self) -> &str {
        &self.version
    }

    /// Reparse the current source, optionally changing merge behavior and schema.
    #[pyo3(signature = (resolve_merges: "bool" = true, schema: "str" = "core") -> "None")]
    fn reparse(&mut self, py: Python, resolve_merges: bool, schema: &str) -> PyResult<()> {
        self.flush_source(py)?;
        let source = self.source.as_ref().ok_or_else(|| {
            YamlTypeError::new_err(format_i18n_error("no-source-to-reparse", &[]))
        })?;
        let schema_enum = parse_schema(schema)?;
        let schema_clone = schema_enum.clone();
        let new_ast = py.detach(|| {
            crate::parser::parse_with_options(source, resolve_merges, schema_clone, 1000, false)
                .map_err(|e| parse_error_to_py_err(e, source, 1000))
        })?;
        self.ast = new_ast;
        self.schema = schema_enum;
        self.splice = None;
        self.splice_checked = false;
        Ok(())
    }

    /// Serialize to a JSON string (via Python `json.dumps`).
    #[pyo3(signature = (indent: "int" = 2) -> "str")]
    fn to_json(&self, py: Python, indent: usize) -> PyResult<String> {
        let obj = self.to_dict(py)?;
        let json_module = py.import("json")?;
        let kw = PyDict::new(py);
        kw.set_item("indent", indent)?;
        let s = json_module
            .call_method("dumps", (obj,), Some(&kw))?
            .extract()?;
        Ok(s)
    }

    /// Validate the document against a JSON Schema.
    #[pyo3(signature = (schema: "str | dict[str, Any]") -> "None")]
    fn validate(&self, py: Python, schema: &Bound<'_, PyAny>) -> PyResult<()> {
        let instance = self.to_dict(py)?;
        let schema_obj: Bound<'_, PyAny> = if let Ok(schema_str) = schema.extract::<String>() {
            let json_module = py.import("json")?;
            json_module.call_method("loads", (schema_str,), None)?
        } else {
            schema.clone()
        };
        let jsonschema = py.import("jsonschema")?;
        let validate_fn = jsonschema.getattr("validate")?;
        let kw = PyDict::new(py);
        kw.set_item("instance", instance)?;
        kw.set_item("schema", schema_obj)?;
        validate_fn.call((), Some(&kw)).map_err(|e| {
            let msg = e
                .value(py)
                .getattr("message")
                .ok()
                .and_then(|m| m.extract::<String>().ok())
                .unwrap_or_else(|| e.to_string());
            YamlValidateError::new_err(msg)
        })?;
        Ok(())
    }

    /// Enter a transaction scope: snapshot AST + splice state. `with doc:` exits cleanly
    /// preserving edits; exceptions roll back the snapshot.
    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let snap = DocumentSnapshot {
            ast: slf.ast.clone(),
            splice: slf.splice.clone(),
            source: slf.source.clone(),
            revision: slf.revision,
            source_dirty: slf.source_dirty,
            splice_checked: slf.splice_checked,
        };
        slf.snapshot.push(snap);
        slf
    }

    #[pyo3(signature = (exc_type: "Any | None" = None, exc_value: "Any | None" = None, tb: "Any | None" = None) -> "bool")]
    fn __exit__(
        &mut self,
        exc_type: Option<Bound<'_, PyAny>>,
        exc_value: Option<Bound<'_, PyAny>>,
        tb: Option<Bound<'_, PyAny>>,
    ) -> bool {
        let _ = (exc_value, tb);
        if let Some(snap) = self.snapshot.pop()
            && exc_type.is_some()
        {
            self.ast = snap.ast;
            self.splice = snap.splice;
            self.source = snap.source;
            self.revision = snap.revision;
            self.source_dirty = snap.source_dirty;
            self.splice_checked = snap.splice_checked;
        }
        false // never swallow exceptions
    }
}

use crate::YamlEditError;
use crate::YamlMaxDepthError;
use crate::YamlParseError;
use crate::YamlSerializeError;
use crate::YamlTagError;
use crate::YamlTypeError;
use crate::YamlValidateError;

pub(crate) fn parse_document(
    py: Python,
    yaml: &Bound<'_, PyAny>,
    resolve_merges: bool,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<YamlDocument> {
    let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
        s
    } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
        String::from_utf8(bytes).map_err(|e| {
            YamlParseError::new_err(format_i18n_error(
                "invalid-utf8",
                &[("detail", &e.to_string())],
            ))
        })?
    } else {
        return Err(YamlTypeError::new_err(format_i18n_error(
            "expected-str-or-bytes",
            &[],
        )));
    };

    let schema_enum = parse_schema(schema)?;
    let schema_clone = schema_enum.clone();
    let mut ast = py.detach(|| {
        crate::parser::parse_with_options(
            &yaml_str,
            resolve_merges,
            schema_clone,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, &yaml_str, max_depth))
    })?;
    resolve_tags(&mut ast, py)?;
    let source: Arc<str> = Arc::from(yaml_str);
    Ok(YamlDocument::new(ast, schema_enum, source))
}

pub(crate) fn resolve_tags(node: &mut crate::ast::CustomNode, py: Python<'_>) -> PyResult<()> {
    use crate::py::tag_registry::get_handlers;
    if crate::py::tag_registry::is_empty() {
        return Ok(());
    }
    match node {
        crate::ast::CustomNode::Scalar {
            value,
            meta: crate::ast::NodeMeta { tag: Some(t), .. },
            ..
        } => {
            let tag_name = t.to_string();
            if let Some(handlers) = get_handlers(&tag_name, py) {
                for (_priority, handler) in handlers {
                    match handler.call1(py, (value.as_ref(),)) {
                        Ok(result) => match result.extract::<String>(py) {
                            Ok(s) => {
                                *value = Arc::from(s);
                                break;
                            }
                            Err(_) => {
                                return Err(YamlTagError::new_err(format!(
                                    "Tag handler '{}' must return a string",
                                    tag_name
                                )));
                            }
                        },
                        Err(e) => {
                            if e.is_instance_of::<crate::YamlTagSkip>(py) {
                                continue;
                            }
                            return Err(YamlTagError::new_err(format!(
                                "Tag handler '{}' failed: {}",
                                tag_name, e
                            )));
                        }
                    }
                }
            }
            Ok(())
        }
        crate::ast::CustomNode::Mapping { pairs, .. } => {
            for (_, v) in pairs.iter_mut() {
                resolve_tags(v, py)?;
            }
            Ok(())
        }
        crate::ast::CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                resolve_tags(item, py)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
