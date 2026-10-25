//! `load_toml` built from the TOML IR, with no `CustomNode` in between.
//!
//! The engine's own AST route projects the scanner's tree onto the shared node model and then walks
//! that model into Python objects. A caller that only wants values pays for the model twice: once
//! to build it (an `IndexMap<CustomNode, CustomNode>` per table, with a hash per key) and once to
//! un-build it. This sink takes the same projected scalars the AST route stores and puts them
//! straight into a `dict`, so a TOML document reaches Python through one walk instead of two.
//!
//! Parity is by construction, not by a suite chasing it: the leaf answers come from
//! `pyrs_toml::scalar_projection`, which is the single function the AST projection and the event
//! stream already share, and the (text, style, tag) to object step calls the same helpers
//! `node_to_pyobject_inner` calls. What this route does not do is `resolve_tags` - a registered tag
//! handler rewrites a tagged scalar on the node, which is an edit - so a document read while any
//! handler is installed takes the AST route.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::ast::ScalarStyle;
use crate::parser::yaml::Schema;
use crate::py::convert::{scalar_to_pyobject, try_custom_type};
use crate::py::tags::tagged_scalar;
use pyrs_toml::{ScalarProjection, TomlReadError, TomlSink};

enum Frame<'py> {
    /// A mapping under construction, with the key whose value has not arrived yet.
    Map {
        dict: Bound<'py, PyDict>,
        key: Option<Bound<'py, PyAny>>,
    },
    Seq {
        list: Bound<'py, PyList>,
    },
}

/// The value side of the IR turned into Python objects.
pub(crate) struct PythonValues<'py> {
    py: Python<'py>,
    schema: &'py Schema,
    stack: Vec<Frame<'py>>,
    root: Option<Bound<'py, PyAny>>,
}

impl<'py> PythonValues<'py> {
    fn new(py: Python<'py>, schema: &'py Schema) -> Self {
        Self {
            py,
            schema,
            stack: Vec::new(),
            root: None,
        }
    }

    /// Hand a finished value to the container that is waiting for it.
    fn deliver(&mut self, value: Bound<'py, PyAny>) -> PyResult<()> {
        match self.stack.last_mut() {
            Some(Frame::Seq { list }) => list.append(value),
            Some(Frame::Map { dict, key }) => match key.take() {
                // The IR writes a name before the thing it names, so a value with no key is a walk
                // that lost its place, not a shape a document can have.
                None => Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "TOML reader delivered a value before its key",
                )),
                Some(k) => dict.set_item(k, value),
            },
            None => {
                self.root = Some(value);
                Ok(())
            }
        }
    }
}

impl TomlSink for PythonValues<'_> {
    type Error = PyErr;

    fn open_table(&mut self) -> Result<(), PyErr> {
        self.stack.push(Frame::Map {
            dict: PyDict::new(self.py),
            key: None,
        });
        Ok(())
    }

    fn open_array(&mut self) -> Result<(), PyErr> {
        self.stack.push(Frame::Seq {
            list: PyList::empty(self.py),
        });
        Ok(())
    }

    fn close_table(&mut self) -> Result<(), PyErr> {
        match self.stack.pop() {
            Some(Frame::Map { dict, .. }) => self.deliver(dict.into_any()),
            _ => Err(pyo3::exceptions::PyRuntimeError::new_err(
                "TOML reader closed a sequence as a table",
            )),
        }
    }

    fn close_array(&mut self) -> Result<(), PyErr> {
        match self.stack.pop() {
            Some(Frame::Seq { list }) => self.deliver(list.into_any()),
            _ => Err(pyo3::exceptions::PyRuntimeError::new_err(
                "TOML reader closed a table as a sequence",
            )),
        }
    }

    /// A member's name, resolved exactly as the AST route resolves a plain key.
    fn key(&mut self, name: std::sync::Arc<str>, style: ScalarStyle) -> Result<(), PyErr> {
        let key = scalar_to_pyobject(self.py, &name, &style, self.schema)
            .map(|object| object.into_bound(self.py))?;
        match self.stack.last_mut() {
            Some(Frame::Map { key: pending, .. }) => {
                *pending = Some(key);
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyRuntimeError::new_err(
                "TOML reader wrote a name outside a mapping",
            )),
        }
    }

    /// A leaf, through the same three-step reading the node walk applies: the tag the document
    /// states wins, then the schema resolves plain text, then a registered type gets its turn.
    fn scalar(&mut self, value: ScalarProjection) -> Result<(), PyErr> {
        let py = self.py;
        let text = value.text.as_str();
        let tagged = match value.tag.as_ref() {
            Some(t) => tagged_scalar(py, &t.to_string(), text)?,
            None => None,
        };
        let object = match tagged {
            Some(object) => object,
            None => {
                let plain = scalar_to_pyobject(py, text, &value.style, self.schema)?;
                match value.tag.as_ref() {
                    Some(t) => {
                        try_custom_type(py, &t.to_string(), plain.bind(py))?.unwrap_or(plain)
                    }
                    None => plain,
                }
            }
        };
        self.deliver(object.into_bound(py))
    }
}

/// Read TOML straight into Python objects.
///
/// The caller maps the two failure kinds the way the AST route maps its own: a parse refusal
/// becomes the localized `toml-parse-error`, a sink error is already a Python error.
pub(crate) fn read_values(
    py: Python<'_>,
    toml_str: &str,
    schema: &Schema,
) -> Result<Py<PyAny>, TomlReadError<PyErr>> {
    let mut sink = PythonValues::new(py, schema);
    pyrs_toml::read_toml(toml_str, &mut sink)?;
    match sink.root {
        Some(root) => Ok(root.unbind()),
        None => Err(TomlReadError::Sink(
            pyo3::exceptions::PyRuntimeError::new_err("TOML reader produced no document"),
        )),
    }
}
