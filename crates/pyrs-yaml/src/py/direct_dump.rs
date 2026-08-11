//! Direct writer: Python objects → YAML string with no intermediate AST.
//!
//! `safe_dump` / `from_dict` / `dump_file` previously built a `CustomNode`
//! tree (`pyobject_to_node`) and then ran the core serializer over it. This
//! module does the same job in one pass while the GIL is held, emitting output
//! that is byte-identical to the old path except for the intentional quirk
//! fixes (see [`needs_quotes`]) and two crash fixes:
//!
//! - Deep nesting (≥ `max_depth` levels) raises `YamlMaxDepthError` instead of
//!   panicking through `to_yaml`'s `.expect()`.
//! - Width-wrapping of multi-byte scalars no longer panics on a mid-character
//!   byte split.
//!
//! The scalar/layout rules below mirror `pyrs_yaml_core::serializer`
//! (`write_plain_scalar`, `write_mapping_pair`, `write_sequence_item`,
//! `write_double_quoted_scalar`) restricted to nodes reachable from Python
//! objects (plain scalars, nulls, block mappings/sequences; no comments,
//! anchors, tags, flow style, or complex `? ` keys — dict/list keys are
//! unhashable and cannot reach the converter).

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

#[cfg(feature = "numpy")]
use crate::YamlSerializeError;
use crate::parser::yaml::schema::needs_quotes;
use crate::py::convert::format_i18n_error;
#[cfg(feature = "numpy")]
use crate::py::ndarray::ndarray_to_node;
use crate::py::python_types::{float_to_yaml_string, py_string_to_arc};
use crate::py::type_registry;
#[cfg(feature = "numpy")]
use crate::serializer::{SerializeOptions, to_yaml_with_options};
use crate::serializer::{write_double_quoted_scalar, write_plain_scalar};
use crate::{YamlMaxDepthError, YamlTypeError};

const MAX_DEPTH: usize = 1000;
const WIDTH: usize = 80;

/// Coarse classification of a Python object's YAML node kind, used to decide
/// inline-vs-block layout without allocating any string.
enum Kind {
    /// `None` → null node.
    Null,
    /// bool / int / float / str → plain-or-quoted scalar.
    Scalar,
    /// dict / list → block container.
    Block,
    /// ndarray or an unsupported object (resolved at write time).
    Other,
}

/// The extracted scalar value after classification, before formatting.
/// String is handled separately because quoting differs between values and keys.
#[derive(Debug)]
enum ScalarValue {
    Bool(bool),
    Int(i64),
    Float(f64),
}

/// Extract a non-string scalar from a Python object. Returns `None` if the
/// object is a string, dict, list, or null — those are handled by their own
/// dedicated paths.
fn extract_scalar_value(obj: &Bound<'_, PyAny>) -> PyResult<Option<ScalarValue>> {
    if obj.cast::<PyString>().is_ok() || obj.is_none() {
        return Ok(None);
    }
    if obj.cast::<PyDict>().is_ok() || obj.cast::<PyList>().is_ok() {
        return Ok(None);
    }
    // bool is checked before PyInt because bool is a subclass of int; this
    // ensures a bool extraction (lossy for plain ints) only matches actual
    // bools / numpy bools.
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Some(ScalarValue::Bool(b)));
    }
    // Fast path: native Python int/float (exact type check, cheaper than extract).
    if obj.cast::<pyo3::types::PyInt>().is_ok()
        && let Ok(n) = obj.extract::<i64>()
    {
        return Ok(Some(ScalarValue::Int(n)));
    }
    if obj.cast::<pyo3::types::PyFloat>().is_ok()
        && let Ok(f) = obj.extract::<f64>()
    {
        return Ok(Some(ScalarValue::Float(f)));
    }
    // Fallback via extract for protocol-convertible types (numpy scalars,
    // Python big ints that overflow i64, etc.).
    if let Ok(n) = obj.extract::<i64>() {
        return Ok(Some(ScalarValue::Int(n)));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Some(ScalarValue::Float(f)));
    }
    Ok(None)
}

fn classify(obj: &Bound<'_, PyAny>) -> Kind {
    if obj.is_none() {
        return Kind::Null;
    }
    if obj.cast::<PyDict>().is_ok() {
        return Kind::Block;
    }
    if obj.cast::<PyList>().is_ok() {
        return Kind::Block;
    }
    // String case: cheap cast check.
    if obj.cast::<PyString>().is_ok() {
        return Kind::Scalar;
    }
    // Numeric / bool: try extract_scalar_value.
    if extract_scalar_value(obj).ok().flatten().is_some() {
        return Kind::Scalar;
    }
    Kind::Other
}

/// Serialize a Python object to a YAML string (one document, no `---`).
pub(crate) fn direct_dump(py: Python, obj: &Py<PyAny>) -> PyResult<String> {
    direct_dump_with_options(py, obj, false)
}

/// Serialize a Python object to a YAML string, optionally sorting mapping keys.
/// Key sorting mirrors the core serializer's block-mapping sort (byte order on
/// the raw scalar value); compact `- key: value` items are never sorted.
pub(crate) fn direct_dump_with_options(
    py: Python,
    obj: &Py<PyAny>,
    sort_keys: bool,
) -> PyResult<String> {
    let mut w = DirectWriter {
        output: String::new(),
        indent_cache: vec![String::new()],
        sort_keys,
    };
    w.write_node(py, obj.bind(py), 0, 0)?;
    Ok(w.output)
}

struct DirectWriter {
    output: String,
    indent_cache: Vec<String>,
    sort_keys: bool,
}

impl DirectWriter {
    fn write_indent(&mut self, width: usize) {
        if self.indent_cache.len() <= width {
            self.indent_cache.resize(width + 1, String::new());
        }
        if self.indent_cache[width].is_empty() {
            self.indent_cache[width] = " ".repeat(width);
        }
        self.output.push_str(&self.indent_cache[width]);
    }

    fn depth_error(&self) -> PyErr {
        YamlMaxDepthError::new_err(format_i18n_error(
            "max-depth-exceeded",
            &[("max_depth", &MAX_DEPTH.to_string())],
        ))
    }

    /// Mirror of `Serializer::serialize_node_internal` for Python objects.
    /// Depth check is delegated to [`write_node_with_kind`].
    fn write_node(
        &mut self,
        py: Python,
        obj: &Bound<'_, PyAny>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        let kind = classify(obj);
        self.write_node_with_kind(py, obj, kind, indent_width, depth)
    }

    /// Like [`write_node`] but takes a pre-classified [`Kind`] to avoid
    /// re-classifying when the caller already knows the kind.
    fn write_node_with_kind(
        &mut self,
        py: Python,
        obj: &Bound<'_, PyAny>,
        kind: Kind,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        if depth >= MAX_DEPTH {
            return Err(self.depth_error());
        }
        match kind {
            Kind::Null => {
                self.write_indent(indent_width);
                self.output.push_str("null");
                self.output.push('\n');
            }
            Kind::Scalar => self.write_scalar_node(py, obj, indent_width)?,
            Kind::Block => {
                if obj.is_instance_of::<PyDict>() {
                    self.write_mapping(py, obj.cast().unwrap(), indent_width, depth)?;
                } else {
                    self.write_sequence(py, obj.cast().unwrap(), indent_width, depth)?;
                }
            }
            Kind::Other => self.write_other(py, obj, indent_width)?,
        }
        Ok(())
    }

    /// Mirror of the serializer's plain/`DoubleQuoted` scalar branches plus the
    /// conversion-layer `needs_quotes` guard.
    fn write_scalar_node(
        &mut self,
        py: Python,
        obj: &Bound<'_, PyAny>,
        indent_width: usize,
    ) -> PyResult<()> {
        self.write_indent(indent_width);
        if let Ok(s) = obj.cast::<PyString>() {
            let text = py_string_to_arc(s)?;
            if needs_quotes(&text) {
                self.write_double_quoted(&text);
            } else {
                self.write_plain_scalar(&text, WIDTH);
            }
        } else if let Some(v) = extract_scalar_value(obj)? {
            match v {
                ScalarValue::Bool(b) => self.output.push_str(if b { "true" } else { "false" }),
                ScalarValue::Int(n) => self.output.push_str(&n.to_string()),
                ScalarValue::Float(f) => {
                    if !type_registry::is_empty()
                        && let Some(result) = type_registry::try_to_yaml(py, &obj.clone().unbind())
                    {
                        let (tag_name, yaml_str) = result?;
                        self.output
                            .push_str(&format!("!{} ", tag_name.trim_start_matches('!')));
                        self.output.push_str(&yaml_str);
                    } else {
                        self.output.push_str(&float_to_yaml_string(f));
                    }
                }
            }
        } else {
            return Err(self.unsupported_type());
        }
        self.output.push('\n');
        Ok(())
    }

    /// Mirror of `Serializer::write_scalar_for_key` (keys never wrap).
    fn write_key(&mut self, key: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(s) = key.cast::<PyString>() {
            let text = py_string_to_arc(s)?;
            if needs_quotes(&text) {
                self.write_double_quoted(&text);
            } else {
                self.write_key_text(&text);
            }
            return Ok(());
        }
        match classify(key) {
            Kind::Null => self.output.push_str("null"),
            Kind::Scalar => match extract_scalar_value(key)? {
                Some(ScalarValue::Bool(b)) => {
                    self.output.push_str(if b { "true" } else { "false" })
                }
                Some(ScalarValue::Int(n)) => self.output.push_str(&n.to_string()),
                Some(ScalarValue::Float(f)) => self.output.push_str(&float_to_yaml_string(f)),
                None => return Err(self.unsupported_type()),
            },
            // dict/list keys are unhashable and can never reach the converter.
            Kind::Block | Kind::Other => return Err(self.unsupported_type()),
        }
        Ok(())
    }

    /// Mirror of `Serializer::write_mapping_pair` for each `(key, value)`.
    fn write_mapping(
        &mut self,
        py: Python,
        dict: &Bound<'_, PyDict>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        if self.sort_keys {
            let mut pairs: Vec<(String, Bound<'_, PyAny>, Bound<'_, PyAny>)> = dict
                .iter()
                .map(|(key, value)| Ok((self.key_sort_string(&key)?, key, value)))
                .collect::<PyResult<_>>()?;
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, key, value) in pairs {
                self.write_mapping_pair(py, &key, &value, indent_width, depth)?;
            }
        } else {
            for (key, value) in dict.iter() {
                self.write_mapping_pair(py, &key, &value, indent_width, depth)?;
            }
        }
        Ok(())
    }

    /// Write one `key: value` pair, inlining scalars and nulls.
    fn write_mapping_pair(
        &mut self,
        py: Python,
        key: &Bound<'_, PyAny>,
        value: &Bound<'_, PyAny>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        self.write_indent(indent_width);
        self.write_key(key)?;
        self.output.push(':');
        let vkind = classify(value);
        match vkind {
            Kind::Null | Kind::Scalar => {
                self.output.push(' ');
                self.write_node_with_kind(py, value, vkind, 0, depth + 1)?;
            }
            Kind::Block | Kind::Other => {
                self.output.push('\n');
                self.write_node_with_kind(py, value, vkind, indent_width + 2, depth + 1)?;
            }
        }
        Ok(())
    }

    /// Raw key content used for `sort_keys`, mirroring the serializer's
    /// comparator (the unquoted scalar value of the key node).
    fn key_sort_string(&self, key: &Bound<'_, PyAny>) -> PyResult<String> {
        if let Ok(s) = key.cast::<PyString>() {
            return py_string_to_arc(s).map(|a| a.to_string());
        }
        match classify(key) {
            Kind::Null => Ok("null".to_string()),
            Kind::Scalar => {
                if let Ok(b) = key.extract::<bool>() {
                    Ok(if b { "true" } else { "false" }.to_string())
                } else if let Ok(n) = key.extract::<i64>() {
                    Ok(n.to_string())
                } else if let Ok(f) = key.extract::<f64>() {
                    Ok(float_to_yaml_string(f))
                } else {
                    Err(self.unsupported_type())
                }
            }
            // dict/list keys are unhashable and can never reach the converter.
            Kind::Block | Kind::Other => Ok(String::new()),
        }
    }

    /// Mirror of `Serializer::write_sequence_item`.
    fn write_sequence(
        &mut self,
        py: Python,
        list: &Bound<'_, PyList>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        for item in list.iter() {
            self.write_indent(indent_width);
            self.output.push_str("- ");
            if let Ok(dict) = item.cast::<PyDict>() {
                if self.is_compact_mapping(dict) {
                    for (pi, (key, value)) in dict.iter().enumerate() {
                        if pi > 0 {
                            self.write_indent(indent_width + 2);
                        }
                        self.write_key(&key)?;
                        self.output.push(':');
                        self.output.push(' ');
                        self.write_node(py, &value, 0, depth + 1)?;
                    }
                } else {
                    self.output.push('\n');
                    self.write_mapping(py, dict, indent_width + 2, depth + 1)?;
                }
            } else if item.is_instance_of::<PyList>() {
                self.output.push('\n');
                self.write_sequence(py, item.cast().unwrap(), indent_width + 2, depth + 1)?;
            } else {
                // Scalars and nulls stay on the dash line; ndarray (block) and
                // unsupported objects go on their own indented block.
                match classify(&item) {
                    Kind::Scalar | Kind::Null => {
                        self.write_node(py, &item, 0, depth + 1)?;
                    }
                    _ => {
                        self.output.push('\n');
                        self.write_node(py, &item, indent_width + 2, depth + 1)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Mirror of `Serializer::is_compact_item` for Python-derived mappings:
    /// non-empty, no metadata, every value inlineable (scalar or null).
    /// ndarray values always produce a block sequence, so they are never
    /// compact — matching `inlineable_value` on the node form.
    fn is_compact_mapping(&self, dict: &Bound<'_, PyDict>) -> bool {
        if dict.is_empty() {
            return false;
        }
        dict.iter()
            .all(|(_, v)| matches!(classify(&v), Kind::Null | Kind::Scalar))
    }

    /// ndarray subtree (serialized via the core serializer with the current
    /// indent spliced in) or an unsupported Python type.
    #[cfg_attr(not(feature = "numpy"), allow(unused_variables))]
    fn write_other(
        &mut self,
        py: Python,
        obj: &Bound<'_, PyAny>,
        indent_width: usize,
    ) -> PyResult<()> {
        #[cfg(feature = "numpy")]
        {
            // numpy is only probed at runtime: free-threaded (Py_GIL_DISABLED)
            // builds compile with the default "numpy" feature but ship without
            // the module installed, and numpy's capsule access would panic.
            if py.import("numpy").is_ok()
                && let Some(node) = ndarray_to_node(py, obj)
            {
                let yaml = to_yaml_with_options(
                    &node,
                    &SerializeOptions {
                        indent_offset: indent_width,
                        ..Default::default()
                    },
                )
                .map_err(|e| {
                    if e.to_string().contains("max depth exceeded") {
                        self.depth_error()
                    } else {
                        YamlSerializeError::new_err(format_i18n_error(
                            "yaml-serialize-error",
                            &[("detail", &e.to_string())],
                        ))
                    }
                })?;
                self.output.push_str(&yaml);
                return Ok(());
            }
        }
        // Check registered CustomTypes for serialization
        if let Some(result) = type_registry::try_to_yaml(py, &obj.clone().unbind()) {
            let (tag_name, yaml_str) = result?;
            self.write_indent(indent_width);
            self.output
                .push_str(&format!("!{} ", tag_name.trim_start_matches('!')));
            self.output.push_str(&yaml_str);
            self.output.push('\n');
            return Ok(());
        }
        Err(self.unsupported_type())
    }

    fn unsupported_type(&self) -> PyErr {
        YamlTypeError::new_err(format_i18n_error("unsupported-type", &[]))
    }

    /// Mirror of `Serializer::write_scalar_for_key`'s plain branch, minus the
    /// `needs_quotes` guard (callers dispatch quoted keys before reaching here).
    fn write_key_text(&mut self, value: &str) {
        if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
            self.output.push_str(value);
        } else {
            self.write_plain_scalar(value, 0);
        }
    }

    /// Mirror of `Serializer::write_plain_scalar` (`remaining` = wrap width;
    /// 0 disables wrapping). Byte-identical to the serializer except that
    /// Delegate to the shared `serializer::write_plain_scalar`. `WIDTH` is the
    /// full wrap width used for continuations; `remaining` is the remaining width
    /// on the current line (0 disables the first-line wrap).
    fn write_plain_scalar(&mut self, value: &str, remaining: usize) {
        write_plain_scalar(&mut self.output, value, remaining, WIDTH);
    }

    /// Delegate to the shared `serializer::write_double_quoted_scalar`.
    fn write_double_quoted(&mut self, value: &str) {
        write_double_quoted_scalar(&mut self.output, value);
    }
}
