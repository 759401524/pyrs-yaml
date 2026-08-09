use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::ast::CustomNode;

/// Walk the AST depth-first, collecting path tuples of keys (str) and indices (int).
pub(crate) fn walk_ast<'a>(
    node: &CustomNode,
    path: &mut Vec<Bound<'a, PyAny>>,
    paths: &mut Vec<Py<PyAny>>,
    py: Python<'a>,
) -> PyResult<()> {
    paths.push(
        PyTuple::new(py, path.iter().map(|p| p as &Bound<'_, PyAny>))?
            .into_any()
            .unbind(),
    );
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (k, v) in pairs.iter() {
                let key_str = match k {
                    CustomNode::Scalar { value, .. } => value.as_ref(),
                    _ => continue,
                };
                let item = key_str.into_pyobject(py)?.into_any();
                path.push(item);
                walk_ast(v, path, paths, py)?;
                path.pop();
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                let idx = (i as i64).into_pyobject(py)?.into_any();
                path.push(idx);
                walk_ast(item, path, paths, py)?;
                path.pop();
            }
        }
        _ => {}
    }
    Ok(())
}

/// Walk only scalar/null nodes, collecting their path tuples.
pub(crate) fn walk_scalars<'a>(
    node: &CustomNode,
    path: &mut Vec<Bound<'a, PyAny>>,
    paths: &mut Vec<Py<PyAny>>,
    py: Python<'a>,
) -> PyResult<()> {
    match node {
        CustomNode::Scalar { .. } | CustomNode::Null { .. } => {
            paths.push(
                PyTuple::new(py, path.iter().map(|p| p as &Bound<'_, PyAny>))?
                    .into_any()
                    .unbind(),
            );
        }
        CustomNode::Mapping { pairs, .. } => {
            for (k, v) in pairs.iter() {
                let key_str = match k {
                    CustomNode::Scalar { value, .. } => value.as_ref(),
                    _ => continue,
                };
                let item = key_str.into_pyobject(py)?.into_any();
                path.push(item);
                walk_scalars(v, path, paths, py)?;
                path.pop();
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                let idx = (i as i64).into_pyobject(py)?.into_any();
                path.push(idx);
                walk_scalars(item, path, paths, py)?;
                path.pop();
            }
        }
        _ => {}
    }
    Ok(())
}
