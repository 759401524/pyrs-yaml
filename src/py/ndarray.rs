//! NumPy ndarray → `CustomNode` conversion via `__array_interface__`.

use crate::ast::CustomNode;

use numpy::{
    dtype, Complex32, Complex64, PyArrayDescrMethods, PyArrayDyn, PyArrayMethods, PyUntypedArray,
    PyUntypedArrayMethods,
};
use pyo3::prelude::*;

/// 将 NumPy ndarray 序列化为嵌套 YAML 列表。
pub fn ndarray_to_node(py: Python, obj: &Bound<'_, PyAny>) -> Option<CustomNode> {
    let arr = obj.cast::<PyUntypedArray>().ok()?;

    if arr.ndim() == 0 {
        let reshape = obj.getattr("reshape").ok()?;
        let tuple_cls = py.import("builtins").ok()?.getattr("tuple").ok()?;
        let shape_arg = tuple_cls.call1((1usize,)).ok()?;
        let reshaped = reshape.call1((shape_arg,)).ok()?;
        return ndarray_to_node(py, &reshaped);
    }
    let shape = arr.shape();
    let total = shape.iter().product::<usize>();

    if total == 0 {
        return Some(CustomNode::plain_sequence(Vec::new()));
    }

    macro_rules! dispatch_dtype {
        ($ty:ty, $to_scalar:expr) => {
            if arr.dtype().is_equiv_to(&dtype::<$ty>(py)) {
                let typed = arr.cast::<PyArrayDyn<$ty>>().ok()?;
                let slice = unsafe { typed.as_slice() }.ok()?;
                let flat = py.detach(|| slice.iter().map($to_scalar).collect::<Vec<CustomNode>>());
                let mut result = flat;
                for &dim in shape[1..].iter().rev() {
                    result = nest_ndarray_sequence(result, dim);
                }
                return Some(CustomNode::plain_sequence(result));
            }
        };
    }

    dispatch_dtype!(i8, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(i16, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(i32, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(i64, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(u8, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(u16, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(u32, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(u64, |v| CustomNode::plain_scalar(v.to_string()));
    dispatch_dtype!(f32, |v| CustomNode::plain_scalar(if v.is_nan() {
        "NaN".to_string()
    } else {
        v.to_string()
    }));
    dispatch_dtype!(f64, |v| CustomNode::plain_scalar(if v.is_nan() {
        "NaN".to_string()
    } else {
        v.to_string()
    }));
    dispatch_dtype!(bool, |v| CustomNode::plain_scalar(if *v {
        "true"
    } else {
        "false"
    }));
    dispatch_dtype!(Complex64, |c| CustomNode::plain_scalar(format!(
        "({}+{}j)",
        c.re, c.im
    )));
    dispatch_dtype!(Complex32, |c| CustomNode::plain_scalar(format!(
        "({}+{}j)",
        c.re, c.im
    )));

    None
}

/// 将展平的 `Vec<CustomNode>` 按 `dim` 嵌套一层。
pub fn nest_ndarray_sequence(flat: Vec<CustomNode>, dim: usize) -> Vec<CustomNode> {
    if dim == 0 {
        return vec![CustomNode::plain_sequence(Vec::new())];
    }
    flat.chunks(dim)
        .map(|chunk| CustomNode::plain_sequence(chunk.to_vec()))
        .collect()
}
