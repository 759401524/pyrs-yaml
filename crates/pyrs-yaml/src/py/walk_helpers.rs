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
    let all_filter = |_: &CustomNode| true;
    walk_inner(node, path, paths, py, &all_filter)
}

/// Walk only scalar/null nodes, collecting their path tuples.
pub(crate) fn walk_scalars<'a>(
    node: &CustomNode,
    path: &mut Vec<Bound<'a, PyAny>>,
    paths: &mut Vec<Py<PyAny>>,
    py: Python<'a>,
) -> PyResult<()> {
    let scalar_filter =
        |n: &CustomNode| matches!(n, CustomNode::Scalar { .. } | CustomNode::Null { .. });
    walk_inner(node, path, paths, py, &scalar_filter)
}

/// Shared depth-first walk. `filter` gates which nodes are emitted as paths;
/// containers are always recursed into.
fn walk_inner<'a, F>(
    node: &CustomNode,
    path: &mut Vec<Bound<'a, PyAny>>,
    paths: &mut Vec<Py<PyAny>>,
    py: Python<'a>,
    filter: &F,
) -> PyResult<()>
where
    F: Fn(&CustomNode) -> bool,
{
    if filter(node) {
        paths.push(
            PyTuple::new(py, path.iter().map(|p| p as &Bound<'_, PyAny>))?
                .into_any()
                .unbind(),
        );
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (k, v) in pairs.iter() {
                let key_str = match k {
                    CustomNode::Scalar { value, .. } => value.as_ref(),
                    _ => continue,
                };
                let item = key_str.into_pyobject(py)?.into_any();
                path.push(item);
                walk_inner(v, path, paths, py, filter)?;
                path.pop();
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                let idx = (i as i64).into_pyobject(py)?.into_any();
                path.push(idx);
                walk_inner(item, path, paths, py, filter)?;
                path.pop();
            }
        }
        _ => {}
    }
    Ok(())
}
