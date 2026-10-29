//! NumPy ndarray → `CustomNode`, asked through the array's own Python methods.
//!
//! Nothing here reaches into numpy's C API, and that is the point. The route used to cast the object
//! to `PyUntypedArray` and read it through `numpy`'s typed slices, which is only sound while the
//! extension is linked the way numpy expects; the `cp315-abi3t` wheel - one binary that serves both
//! the GIL and the free-threaded 3.15 - is not. Measured on the published 0.18.3 wheel:
//! `safe_dump(numpy.arange(8))` on 3.15 GIL died with `Fatal Python error: Segmentation fault`
//! (exit 139 on Linux, `Windows fatal exception: access violation` on Windows), and the same bytes on
//! 3.15t answered `YamlTypeError`, while 3.12, 3.14 and 3.14t - built against a non-limited ABI -
//! dumped it correctly. It was deterministic and single-threaded, and a zero-length array crashed too,
//! so the fault sat at the first read of array internals rather than in any race.
//!
//! `tobytes()` and `tolist()`-free arithmetic on the returned buffer is the whole route now: numpy
//! does the copy inside the interpreter, under the GIL, and everything after that is Rust reading
//! bytes it owns. One behaviour follows from that change and it is a fix rather than a cost:
//! `tobytes()` emits *logical* C order, so a Fortran-ordered or transposed array now writes what the
//! array means. It used to write what its memory held - measured on 0.18.3,
//! `safe_dump(numpy.arange(4).reshape(2, 2).T)` returned the un-transposed `[[0, 1], [2, 3]]`.
//!
//! The accepted dtypes are exactly the set the typed route accepted, by `dtype.kind` and
//! `dtype.itemsize`: signed and unsigned ints up to 64 bits, `float32`/`float64`, `bool`,
//! `complex64`/`complex128`. `float16`, strings, bytes and object arrays still return `None`, so the
//! callers fall through to the refusal they give today, and the spellings of every accepted value are
//! unchanged (Rust's own `Display` for the same width, `NaN` for a NaN float, `(re+imj)` for a
//! complex, including today's `(1+-2j)` for a negative imaginary part).
//!
//! Reaching that route is [`is_dimensioned_array`]'s business: both converters refuse to read a
//! dimensional array through Python's number protocol, which answers for a one-element array and
//! drops its dimension. The 0-D case keeps the protocol's answer, so a 0-D array stays a scalar and
//! a 1-D one-element array stays a sequence, on every numpy the package supports.

use crate::ast::CustomNode;
use pyo3::prelude::*;
use pyo3::types::PyType;

/// What one element of the buffer holds: which family, and how wide it is in bytes.
#[derive(Copy, Clone)]
enum Family {
    Signed,
    Unsigned,
    Float,
    Bool,
    Complex,
}

impl Family {
    /// The pair `dtype.kind` and `dtype.itemsize` maps to a supported element, or `None` for a dtype
    /// this route has never emitted. Derived from the two, not from `dtype` equality, because the
    /// widths are what the buffer reader needs anyway.
    fn of(kind: &str, width: usize) -> Option<Family> {
        match (kind, width) {
            ("i", 1 | 2 | 4 | 8) => Some(Family::Signed),
            ("u", 1 | 2 | 4 | 8) => Some(Family::Unsigned),
            ("f", 4 | 8) => Some(Family::Float),
            ("b", 1) => Some(Family::Bool),
            ("c", 8 | 16) => Some(Family::Complex),
            _ => None,
        }
    }

    fn scalar(&self, chunk: &[u8], little: bool) -> CustomNode {
        let bits = read_unsigned(chunk, little);
        let text = match self {
            Family::Signed => match chunk.len() {
                1 => (bits as i8).to_string(),
                2 => (bits as i16).to_string(),
                4 => (bits as i32).to_string(),
                _ => (bits as i64).to_string(),
            },
            Family::Unsigned => match chunk.len() {
                1 => (bits as u8).to_string(),
                2 => (bits as u16).to_string(),
                4 => (bits as u32).to_string(),
                _ => bits.to_string(),
            },
            Family::Float => float_text(bits, chunk.len()),
            Family::Bool => if chunk[0] == 0 { "false" } else { "true" }.to_string(),
            Family::Complex => {
                let half = chunk.len() / 2;
                let re = float_text(read_unsigned(&chunk[..half], little), half);
                let im = float_text(read_unsigned(&chunk[half..], little), half);
                format!("({re}+{im}j)")
            }
        };
        CustomNode::plain_scalar(text)
    }
}

/// The spelling the typed route gave a float of this width: `NaN` for a NaN, Rust's own rendering
/// otherwise. The width is part of the question, not a detail - a `float32` array holding 1e30
/// printed `1000000000000000000000000000000` through `rust-numpy` (an `f32`'s shortest decimal that
/// round-trips as f32) and `1000000015047466200000000000000` once the bits were widened to `f64`
/// first, measured on the same wheel. Same reason `complex64` keeps its 4-byte halves.
fn float_text(bits: u64, width: usize) -> String {
    if width == 4 {
        let value = f32::from_bits(bits as u32);
        if value.is_nan() {
            "NaN".to_string()
        } else {
            value.to_string()
        }
    } else {
        let value = f64::from_bits(bits);
        if value.is_nan() {
            "NaN".to_string()
        } else {
            value.to_string()
        }
    }
}

fn read_unsigned(chunk: &[u8], little: bool) -> u64 {
    chunk.iter().enumerate().fold(0u64, |acc, (index, byte)| {
        let shift = if little {
            8 * index
        } else {
            8 * (chunk.len() - 1 - index)
        };
        acc | ((*byte as u64) << shift)
    })
}

/// The array type numpy publishes, or `None` when numpy is not installed.
fn ndarray_type(py: Python<'_>) -> Option<Bound<'_, PyType>> {
    let ty = py
        .import("numpy")
        .ok()?
        .getattr("ndarray")
        .ok()?
        .cast::<PyType>()
        .ok()?
        .clone();
    Some(ty)
}

/// Whether `obj` is a NumPy array carrying at least one dimension.
///
/// Both converters ask this before they let the number protocol near an object, because that
/// protocol answers *for* an array: `int(numpy.array([42]))` is 42, `float(numpy.array([1e30],
/// dtype="float32"))` is the double widened from the float, and the dimension is gone in both.
/// numpy enforced that as a warning up to 2.2 and made it an error in 2.3 ("only 0-dimensional
/// arrays can be converted to Python scalars"), so which spelling a one-element array left the
/// package with depended on the numpy the caller happened to have installed. Asking the question
/// numpy settled on makes the emitted shape a property of this crate. Measured on 0.18.3 with
/// numpy 2.2: `safe_dump(numpy.array([1e30], dtype="float32"))` answered
/// `1000000015047466200000000000000.0` - a scalar, the f64 widening, and no sequence - and
/// `numpy.array([True])` answered `1`, which is the spelling `docs/en/guides/numpy.md` tells
/// readers to use *for* a boolean round-trip.
///
/// A 0-D array is deliberately left to the protocol, which every numpy version answers with the
/// element itself; that is the scalar spelling `TestNumpyScalar` and the guide's 0-D section pin.
pub(crate) fn is_dimensioned_array(py: Python, obj: &Bound<'_, PyAny>) -> bool {
    let Some(ndarray) = ndarray_type(py) else {
        return false;
    };
    if !obj.is_instance(&ndarray).unwrap_or(false) {
        return false;
    }
    obj.getattr("ndim")
        .and_then(|ndim| ndim.extract::<usize>())
        .is_ok_and(|ndim| ndim > 0)
}

/// 将 NumPy ndarray 序列化为嵌套 YAML 列表。
pub(crate) fn ndarray_to_node(py: Python, obj: &Bound<'_, PyAny>) -> Option<CustomNode> {
    let ndarray = ndarray_type(py)?;
    if !obj.is_instance(&ndarray).ok()? {
        return None;
    }
    let dtype = obj.getattr("dtype").ok()?;
    let kind = dtype.getattr("kind").ok()?.extract::<String>().ok()?;
    let width = dtype.getattr("itemsize").ok()?.extract::<usize>().ok()?;
    let family = Family::of(&kind, width)?;
    let shape = obj.getattr("shape").ok()?.extract::<Vec<usize>>().ok()?;
    if shape.iter().product::<usize>() == 0 {
        return Some(CustomNode::plain_sequence(Vec::new()));
    }

    // The snapshot happens here, inside the interpreter and with the GIL held: `tobytes` copies the
    // array into a bytes object this function then owns, so a concurrent write by another thread
    // cannot appear in what follows - the invariant `tests/test_numpy.py::TestNumpyConcurrency` pins.
    let bytes = obj
        .call_method0("tobytes")
        .ok()?
        .extract::<Vec<u8>>()
        .ok()?;
    let little = py
        .import("sys")
        .ok()?
        .getattr("byteorder")
        .ok()?
        .extract::<String>()
        .ok()?
        == "little";

    // The element loop is pure Rust over a buffer nothing else can write, which is the case
    // `py.detach` is for; the Python-side work is already finished above.
    let flat = py.detach(|| {
        bytes
            .chunks_exact(width)
            .map(|chunk| family.scalar(chunk, little))
            .collect::<Vec<CustomNode>>()
    });
    let mut result = flat;
    for &dim in shape.iter().skip(1).rev() {
        result = nest_ndarray_sequence(result, dim);
    }
    Some(CustomNode::plain_sequence(result))
}

/// 将展平的 `Vec<CustomNode>` 按 `dim` 嵌套一层。
pub(crate) fn nest_ndarray_sequence(flat: Vec<CustomNode>, dim: usize) -> Vec<CustomNode> {
    if dim == 0 {
        return vec![CustomNode::plain_sequence(Vec::new())];
    }
    flat.chunks(dim)
        .map(|chunk| CustomNode::plain_sequence(chunk.to_vec()))
        .collect()
}
