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
//! anchors, tags or flow style). A complex `? ` key is the one shape this writer
//! does not spell at all: the loader projects a YAML sequence key to a Python
//! tuple, so such a key really does reach the converter, and the pair is handed
//! to the core serializer for that case rather than mirroring the spelling twice.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use crate::YamlSerializeError;
use crate::ast::{CustomNode, NodeMap};
use crate::parser::yaml::schema::needs_quotes;
use crate::py::convert::format_i18n_error;
#[cfg(feature = "numpy")]
use crate::py::ndarray::{is_dimensioned_array, ndarray_to_node};
use crate::py::python_types::pyobject_to_node;
use crate::py::python_types::{
    float_to_yaml_string, is_decimal_digits, key_is_readable_shape, py_string_to_arc,
};
use crate::py::type_registry;
use crate::serializer::{SerializeOptions, to_yaml_with_options};
use crate::serializer::{
    is_short_alphanumeric, is_yaml_noncharacter, write_double_quoted_scalar, write_plain_scalar,
    write_single_quoted_scalar,
};
use crate::{YamlMaxDepthError, YamlTypeError};
use std::sync::Arc;

const MAX_DEPTH: usize = 1000;
const WIDTH: usize = 80;

/// Coarse classification of a Python object's YAML node kind, used to decide
/// inline-vs-block layout without allocating any string.
#[derive(Debug, Clone, Copy)]
enum Kind {
    /// `None` → null node.
    Null,
    /// bool / int / float / str → plain-or-quoted scalar.
    Scalar,
    /// dict → block mapping.
    Block,
    /// list / tuple → block sequence. A tuple is the spelling the loader hands back for a YAML
    /// sequence key, so refusing it here broke `safe_dump(safe_load(text))` on a document the
    /// reader accepted.
    Sequence,
    /// ndarray or an unsupported object (resolved at write time).
    Other,
}

/// The extracted scalar value after classification, before formatting.
/// String is handled separately because quoting differs between values and keys.
#[derive(Debug)]
enum ScalarValue {
    Bool(bool),
    Int(i64),
    /// A Python int too wide for `i64`, kept as the exact decimal text `str()` produced.
    BigInt(Arc<str>),
    Float(f64),
}

/// Extract a non-string scalar from a Python object. Returns `None` if the
/// object is a string, dict, list, or null — those are handled by their own
/// dedicated paths.
fn extract_scalar_value(obj: &Bound<'_, PyAny>) -> PyResult<Option<ScalarValue>> {
    if obj.cast::<PyString>().is_ok() || obj.is_none() {
        return Ok(None);
    }
    if obj.cast::<PyDict>().is_ok() || obj.cast::<PyList>().is_ok() || obj.cast::<PyTuple>().is_ok()
    {
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
    // A native int that did not fit `i64` must not reach the `f64` fallback below: `extract::<f64>()`
    // succeeds on *every* `PyLong`, so it captured wide integers and emitted a float spelling of a
    // different value (`123456789012345678901234567890` left as `1.2345678901234568e+29`, which also
    // silently re-typed wide integer *keys*). YAML integers have no width limit on the writer side.
    if obj.cast::<pyo3::types::PyInt>().is_ok() {
        let digits = py_string_to_arc(&obj.str()?)?;
        if is_decimal_digits(&digits) {
            return Ok(Some(ScalarValue::BigInt(digits)));
        }
    }
    // A dimensional NumPy array is a shape, not a number, and the protocol calls below answer *for*
    // one: `float(numpy.array([1e30], dtype="float32"))` succeeds on every numpy up to 2.2 and
    // returns the double widened from the float with the dimension dropped. Declining here leaves
    // the classification as `Kind::Other`, which is the branch that asks the array route and either
    // emits the sequence or refuses. Placed after the exact-type casts so no native int, float or
    // bool pays for the import.
    #[cfg(feature = "numpy")]
    if is_dimensioned_array(obj.py(), obj) {
        return Ok(None);
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
        return Kind::Sequence;
    }
    // String case: cheap cast check.
    if obj.cast::<PyString>().is_ok() {
        return Kind::Scalar;
    }
    // Numeric / bool: try extract_scalar_value.
    if extract_scalar_value(obj).ok().flatten().is_some() {
        return Kind::Scalar;
    }
    // Checked after the scalars so a string or number never pays for the extra cast: a tuple is
    // only ever the shape the loader hands back for a sequence key.
    if obj.cast::<PyTuple>().is_ok() {
        return Kind::Sequence;
    }
    Kind::Other
}

/// Whether a key is a container and so cannot be spelled on the key's own line.
///
/// Three casts, never the full [`classify`]: this question sits on the recursive path, and an
/// inlined `classify` brings `extract_scalar_value`'s locals into the caller's frame. `MAX_DEPTH`
/// lets that frame repeat a thousand times, which is more stack than a Windows main thread
/// commits, so the refusal a deep document should get arrives as a interpreter-killing overflow
/// instead. Measured: the same 1,100-level document overflowed at the 891st level with `classify`
/// here and reached the guard with this predicate.
fn key_is_complex(obj: &Bound<'_, PyAny>) -> bool {
    obj.cast::<PyDict>().is_ok() || obj.cast::<PyList>().is_ok() || obj.cast::<PyTuple>().is_ok()
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
    let mut output = String::new();
    direct_dump_into(py, obj.bind(py), sort_keys, &mut output)?;
    Ok(output)
}

/// Serialize `obj` *into* a caller-provided buffer (cleared first). Lets a
/// multi-document loop reuse one allocation instead of creating a fresh
/// `String` per document; the buffer's capacity survives across calls.
pub(crate) fn direct_dump_into(
    py: Python,
    obj: &Bound<'_, PyAny>,
    sort_keys: bool,
    output: &mut String,
) -> PyResult<()> {
    let mut w = DirectWriter {
        output: std::mem::take(output),
        indent_cache: vec![String::new()],
        sort_keys,
    };
    let result = w.write_node(py, obj, 0, 0);
    *output = std::mem::take(&mut w.output);
    result
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
                if let Ok(dict) = obj.cast::<PyDict>() {
                    self.write_mapping(py, dict, indent_width, depth)?;
                }
            }
            Kind::Sequence => {
                if let Ok(list) = obj.cast::<PyList>() {
                    self.write_sequence(py, list, indent_width, depth)?;
                } else if let Ok(tuple) = obj.cast::<PyTuple>() {
                    self.write_tuple_sequence(py, tuple, indent_width, depth)?;
                }
            }
            Kind::Other => self.write_other(py, obj, indent_width)?,
        }
        Ok(())
    }

    /// The tag name and text a registered `CustomType` produces for this object. `None` means
    /// nothing is registered or nothing claims its class, which is every native value.
    ///
    /// The text is what the node route puts in the key node's scalar slot, so `key_sort_string`
    /// sorts on it: the serializer's comparator reads that slot and nothing else.
    fn registered_scalar(
        &self,
        py: Python,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<Option<(String, String)>> {
        if type_registry::is_empty() {
            return Ok(None);
        }
        match type_registry::try_to_yaml(py, &obj.clone().unbind()) {
            Some(result) => result.map(Some),
            None => Ok(None),
        }
    }

    /// A registered `CustomType` spells this scalar if one claims its class. `false` means nothing
    /// is registered or nothing matched, which is every native value.
    ///
    /// The caller keeps the indentation it already wrote; this appends the tag handle and the
    /// handler's text only.
    fn write_registered_scalar(&mut self, py: Python, obj: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Some((tag_name, yaml_str)) = self.registered_scalar(py, obj)? else {
            return Ok(false);
        };
        let tag = crate::py::tags::tag_of(&tag_name);
        self.output.push_str(&tag.handle);
        self.output.push_str(&tag.suffix);
        self.output.push(' ');
        self.output.push_str(&yaml_str);
        Ok(true)
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
                self.write_quoted(&text);
            } else {
                self.write_plain_scalar(&text, WIDTH);
            }
        } else if let Some(v) = extract_scalar_value(obj)? {
            // Asked for every scalar shape, not just the float one: a registered handler used to
            // lose to Python's number protocol for any class carrying `__index__`, which is what
            // Decimal always escaped. An `int` or `int` subclass keeps the integer spelling - that
            // is what a reader resolves - and the one cast is what keeps the common value in a real
            // document off the registry path, which snapshots the handlers and calls `isinstance`
            // per entry. Same predicate as `python_types::pyobject_to_node`, where the routes have
            // to agree.
            if obj.cast::<pyo3::types::PyInt>().is_err() && self.write_registered_scalar(py, obj)? {
                self.output.push('\n');
                return Ok(());
            }
            match v {
                ScalarValue::Bool(b) => self.output.push_str(if b { "true" } else { "false" }),
                ScalarValue::Int(n) => self.output.push_str(&n.to_string()),
                ScalarValue::BigInt(digits) => self.output.push_str(&digits),
                ScalarValue::Float(f) => self.output.push_str(&float_to_yaml_string(f)),
            }
        } else {
            return Err(self.unsupported_type());
        }
        self.output.push('\n');
        Ok(())
    }

    /// Mirror of `Serializer::write_scalar_for_key` (keys never wrap).
    fn write_key(&mut self, py: Python, key: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(s) = key.cast::<PyString>() {
            let text = py_string_to_arc(s)?;
            if needs_quotes(&text) {
                self.write_quoted(&text);
            } else {
                self.write_key_text(&text);
            }
            return Ok(());
        }
        match classify(key) {
            Kind::Null => self.output.push_str("null"),
            Kind::Scalar => {
                // The same question `write_scalar_node` asks, asked here too: a key whose class a
                // handler claims would otherwise be spelled as the integer its protocol answers
                // with, while the route a key takes through `pyobject_to_node` spells it as the
                // tagged text.
                if key.cast::<pyo3::types::PyInt>().is_err()
                    && self.write_registered_scalar(py, key)?
                {
                    return Ok(());
                }
                match extract_scalar_value(key)? {
                    Some(ScalarValue::Bool(b)) => {
                        self.output.push_str(if b { "true" } else { "false" })
                    }
                    Some(ScalarValue::Int(n)) => self.output.push_str(&n.to_string()),
                    Some(ScalarValue::BigInt(digits)) => self.output.push_str(&digits),
                    Some(ScalarValue::Float(f)) => self.output.push_str(&float_to_yaml_string(f)),
                    None => return Err(self.unsupported_type()),
                }
            }
            // A dict key cannot exist in a Python mapping, and a sequence key (the tuple the
            // loader hands back) is delegated before reaching here: `write_complex_pair`.
            Kind::Sequence | Kind::Block | Kind::Other => return Err(self.unsupported_type()),
        }
        Ok(())
    }

    /// One pair whose key is not a scalar, handed to the core serializer.
    ///
    /// A YAML sequence key reaches the writer as the tuple the reader projected (`? [a, b]` →
    /// `("a", "b")`), and the explicit-key spelling — `?` on its own line, the key as a flow
    /// body, then `:` — is the node serializer's, not this writer's. Mirroring it here would
    /// put a second copy of that shape where the route-parity table has to keep them equal; the
    /// delegation is the same move `write_other` already makes for an ndarray subtree, and it
    /// buys byte-identity with `from_dict(...).to_yaml()` by construction.
    ///
    /// Out-of-line on purpose: its locals (a `NodeMap`, two `pyobject_to_node` results, a
    /// serialized `String`) would otherwise join `write_mapping_pair`'s frame, and that frame is
    /// live once per nesting level under the `MAX_DEPTH` guard. See [`key_is_complex`].
    #[inline(never)]
    fn write_complex_pair(
        &mut self,
        py: Python,
        key: &Bound<'_, PyAny>,
        value: &Bound<'_, PyAny>,
        indent_width: usize,
    ) -> PyResult<()> {
        let mut pairs = NodeMap::default();
        pairs.insert(
            pyobject_to_node(py, &key.clone().unbind())?,
            pyobject_to_node(py, &value.clone().unbind())?,
        );
        let yaml = to_yaml_with_options(
            &CustomNode::plain_mapping(pairs),
            &SerializeOptions {
                indent_offset: indent_width,
                sort_keys: self.sort_keys,
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
        Ok(())
    }

    /// Mirror of `Serializer::is_empty_container` for Python objects.
    ///
    /// `{}` and `[]` have no block spelling. Putting one on a line of its own re-reads as
    /// a *flow* node, so the next dump inlines it: the data is right and the text is not
    /// settled, which is the invariant the round-trip tier asserts. `#287` applied this at
    /// the dash line in both writers; the route-parity table over the two writers
    /// (`tests/test_route_parity.py`) found the same decision missing at the mapping-value
    /// site here, where the node writer had already inlined it.
    fn is_empty_container(obj: &Bound<'_, PyAny>) -> bool {
        if let Ok(dict) = obj.cast::<PyDict>() {
            return dict.is_empty();
        }
        if let Ok(list) = obj.cast::<PyList>() {
            return list.is_empty();
        }
        if let Ok(tuple) = obj.cast::<PyTuple>() {
            return tuple.is_empty();
        }
        false
    }

    /// Mirror of `Serializer::write_mapping_pair` for each `(key, value)`.
    fn write_mapping(
        &mut self,
        py: Python,
        dict: &Bound<'_, PyDict>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        if dict.is_empty() {
            self.write_indent(indent_width);
            self.output.push_str("{}\n");
            return Ok(());
        }
        if self.sort_keys {
            let mut pairs: Vec<(String, Bound<'_, PyAny>, Bound<'_, PyAny>)> = dict
                .iter()
                .map(|(key, value)| Ok((self.key_sort_string(py, &key)?, key, value)))
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
        if key_is_complex(key) {
            if !key_is_readable_shape(key) {
                return Err(self.unsupported_type());
            }
            return self.write_complex_pair(py, key, value, indent_width);
        }
        self.write_indent(indent_width);
        self.write_key(py, key)?;
        self.output.push(':');
        let vkind = classify(value);
        match vkind {
            Kind::Null | Kind::Scalar => {
                self.output.push(' ');
                self.write_node_with_kind(py, value, vkind, 0, depth + 1)?;
            }
            Kind::Block | Kind::Sequence | Kind::Other => {
                if Self::is_empty_container(value) {
                    // An empty container rides the key's line, exactly as a scalar does;
                    // `write_node*` would indent it onto the next line, where it re-reads
                    // as flow and the following dump inlines it again.
                    self.output.push(' ');
                    self.output.push_str(if value.cast::<PyDict>().is_ok() {
                        "{}\n"
                    } else {
                        "[]\n"
                    });
                } else {
                    self.output.push('\n');
                    self.write_node_with_kind(py, value, vkind, indent_width + 2, depth + 1)?;
                }
            }
        }
        Ok(())
    }

    /// Raw key content used for `sort_keys`, mirroring the serializer's
    /// comparator (the unquoted scalar value of the key node).
    fn key_sort_string(&self, py: Python, key: &Bound<'_, PyAny>) -> PyResult<String> {
        if let Ok(s) = key.cast::<PyString>() {
            return py_string_to_arc(s).map(|a| a.to_string());
        }
        match classify(key) {
            Kind::Null => Ok("null".to_string()),
            Kind::Scalar => {
                // The handler's text, not the number the protocol answers with, because that text
                // is the key node's scalar value on the route this mirrors: two `Money` keys sort
                // `5c` against `500c` there, and sorting `5` against `500` here would order the
                // same mapping the other way.
                if key.cast::<pyo3::types::PyInt>().is_err()
                    && let Some((_tag, text)) = self.registered_scalar(py, key)?
                {
                    return Ok(text);
                }
                if let Ok(b) = key.extract::<bool>() {
                    Ok(if b { "true" } else { "false" }.to_string())
                } else if let Ok(n) = key.extract::<i64>() {
                    Ok(n.to_string())
                } else if key.cast::<pyo3::types::PyInt>().is_ok()
                    && let Ok(text) = key.str()
                    && is_decimal_digits(&text.to_string())
                {
                    // A wide integer sorts on the digits it will be written as, which is what the
                    // node-side comparator sees; ordering it by the `f64` fallback would sort on a
                    // rounded value the output never contains. Non-int objects keep the old route,
                    // because every Python object has a `str`.
                    Ok(text.to_string())
                } else if let Ok(f) = key.extract::<f64>() {
                    Ok(float_to_yaml_string(f))
                } else {
                    Err(self.unsupported_type())
                }
            }
            // A complex key has no scalar text to sort on, which is what the node writer's
            // comparator does for the same node (`_ => ""`), so a sequence key keeps the pair
            // first among equals rather than the direct writer inventing an ordering.
            Kind::Sequence | Kind::Block | Kind::Other => Ok(String::new()),
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
        if list.is_empty() {
            self.write_indent(indent_width);
            self.output.push_str("[]\n");
            return Ok(());
        }
        for item in list.iter() {
            self.write_indent(indent_width);
            self.output.push_str("- ");
            if let Ok(dict) = item.cast::<PyDict>() {
                if dict.is_empty() {
                    // Mirror of the node writer's exemption: `{}` has no block spelling, so
                    // putting it on a line of its own re-reads as a flow node and the next
                    // dump inlines it — the dash line is where the text stops moving.
                    self.output.push_str("{}\n");
                } else if self.is_compact_mapping(dict) {
                    for (pi, (key, value)) in dict.iter().enumerate() {
                        if pi > 0 {
                            self.write_indent(indent_width + 2);
                        }
                        self.write_key(py, &key)?;
                        self.output.push(':');
                        self.output.push(' ');
                        self.write_node(py, &value, 0, depth + 1)?;
                    }
                } else {
                    self.output.push('\n');
                    self.write_mapping(py, dict, indent_width + 2, depth + 1)?;
                }
            } else if let Ok(list) = item.cast::<PyList>() {
                if list.is_empty() {
                    // Same for `[]`: no block spelling, so no line of its own.
                    self.output.push_str("[]\n");
                } else {
                    self.output.push('\n');
                    self.write_sequence(py, list, indent_width + 2, depth + 1)?;
                }
            } else if let Ok(tuple) = item.cast::<PyTuple>() {
                if tuple.is_empty() {
                    self.output.push_str("[]\n");
                } else {
                    self.output.push('\n');
                    self.write_tuple_sequence(py, tuple, indent_width + 2, depth + 1)?;
                }
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

    /// A tuple is written as the sequence the loader projected it from: borrow it into a list
    /// once, then the same writer. The alternative was a second copy of the dash-line rules, and
    /// tuples are the cold shape here — no fixture reaches them except through a complex key.
    fn write_tuple_sequence(
        &mut self,
        py: Python,
        tuple: &Bound<'_, PyTuple>,
        indent_width: usize,
        depth: usize,
    ) -> PyResult<()> {
        if tuple.is_empty() {
            self.write_indent(indent_width);
            self.output.push_str("[]\n");
            return Ok(());
        }
        self.write_sequence(py, &tuple.to_list(), indent_width, depth)
    }

    /// Mirror of `Serializer::is_compact_item` for Python-derived mappings:
    /// non-empty, no metadata, every value inlineable (scalar or null).
    /// ndarray values always produce a block sequence, so they are never
    /// compact — matching `inlineable_value` on the node form. An empty container is
    /// inlineable too (`{}` / `[]` have no block spelling), and without that the whole
    /// item loses its compact form as well: `{"a": {}, "b": 1}` was spelled
    /// `-\n  a:\n    {}\n  b: 1` where the node writer emits `- a: {}\n  b: 1`.
    fn is_compact_mapping(&self, dict: &Bound<'_, PyDict>) -> bool {
        if dict.is_empty() {
            return false;
        }
        dict.iter().all(|(key, value)| {
            // A sequence key is not inlineable: the pair goes through `write_complex_pair`, and
            // that is the dash line's own shape, so the item cannot stay compact. Same cheap
            // question as `write_mapping_pair` asks, for the same frame-size reason.
            !key_is_complex(&key)
                && (matches!(classify(&value), Kind::Null | Kind::Scalar)
                    || Self::is_empty_container(&value))
        })
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
            // The route itself answers `None` when numpy is not installed or the object is not an
            // array of a dtype it emits, so an array of strings still reaches the refusal below.
            if let Some(node) = ndarray_to_node(py, obj) {
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
        if let Some((tag_name, yaml_str)) = self.registered_scalar(py, obj)? {
            self.write_indent(indent_width);
            let tag = crate::py::tags::tag_of(&tag_name);
            self.output.push_str(&tag.handle);
            self.output.push_str(&tag.suffix);
            self.output.push(' ');
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
        if is_short_alphanumeric(value) {
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

    /// Mirror the serializer's double-quote escape: granit-parser mishandles
    /// `\\<escape-letter>` inside double-quoted scalars (e.g. `\\0` collapses to
    /// NUL). Single-quoted scalars keep backslashes literal, so prefer them
    /// whenever a quoted value contains a backslash, no newline (which
    /// single-quoted cannot represent), and no control/noncharacter (which
    /// single-quoted cannot escape).
    fn write_quoted(&mut self, value: &str) {
        if value.contains('\\')
            && !value.contains('\n')
            && !value.chars().any(char::is_control)
            && !value.chars().any(is_yaml_noncharacter)
        {
            write_single_quoted_scalar(&mut self.output, value);
        } else {
            write_double_quoted_scalar(&mut self.output, value);
        }
    }
}
