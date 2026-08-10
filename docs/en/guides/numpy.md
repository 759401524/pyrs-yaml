---
title: NumPy ndarray Serialization Guide
description: Serialize NumPy arrays to YAML lists with zero-copy Rust processing, including multi-dimensional arrays and special values.
tags:
  - docs
status: new
---

## NumPy ndarray Serialization Guide

!!! warning "Free-threaded builds exclude NumPy"
    On free-threaded (cp314t) wheels the `numpy` feature is disabled, so
    `safe_dump` on a `numpy.ndarray` raises `YamlTypeError`. GIL builds
    (Python 3.8–3.15) keep full ndarray serialization support.

!!! note "Complex numbers"
    YAML has no native complex type. Complex numbers are serialized as
    `(re+imj)` strings. `safe_load` returns them as Python strings, not
    `complex` objects.

Serialize NumPy arrays to YAML lists with zero-copy Rust processing.

### Basic Usage

```python
import numpy as np
import pyrs_yaml as y

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = y.safe_dump(arr)
# Output:
# - 1
# - 2
# - 3

# Round-trip back to Python list
data = y.safe_load(yaml_str)
assert data == [1, 2, 3]
```

### Multi-dimensional Arrays

```python
# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = y.safe_dump(matrix)
data = y.safe_load(yaml_str)
assert data == [[1.0, 2.0], [3.0, 4.0]]

# 3-D cube
cube = np.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], dtype="int64")
data = y.safe_load(y.safe_dump(cube))
assert data == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
```

### Supported dtypes

| NumPy dtype | YAML output | Example |
|-------------|-------------|---------|
| `int8/16/32/64` | Integer | `42` |
| `uint8/16/32/64` | Integer | `42` |
| `float32/64` | Float | `3.14` |
| `bool` | Boolean | `true` / `false` |
| `complex64/128` | String | `(1+2j)` |

### Special Values

```python
# NaN
arr = np.array([1.0, float("nan"), 3.0])
data = y.safe_load(y.safe_dump(arr))
assert str(data[1]) == "nan"

# Infinity
arr = np.array([float("inf"), -float("inf")])
data = y.safe_load(y.safe_dump(arr))
assert data[0] == float("inf")
assert data[1] == float("-inf")
```

### Negative Numbers

YAML 1.2 does not allow plain scalars starting with `-` in block sequences. Negative values are automatically quoted for correct round-trip:

```python
arr = np.array([-100, 200], dtype="int16")
data = y.safe_load(y.safe_dump(arr))
assert data == [-100, 200]  # round-trip correct
```

### 0-D Scalar Arrays

0-D arrays are reshaped to 1-D before serialization, producing a single-element list:

```python
scalar = np.array(42, dtype="int32")
data = y.safe_load(y.safe_dump(scalar))
assert data == [42]
```

### Nested in Containers

NumPy arrays can be embedded in dicts or lists:

```python
data = {"matrix": np.array([[1, 2], [3, 4]]), "label": "test"}
yaml_str = y.safe_dump(data)
loaded = y.safe_load(yaml_str)
assert loaded["matrix"] == [[1, 2], [3, 4]]
```

### Unsupported Types

The following types raise `YamlTypeError`:

- String arrays
- Object arrays
- Structured arrays
- Non-numeric custom dtypes

### Performance

- Zero-copy dtype dispatch via `PyUntypedArray`
- Zero-copy slice iteration via `PyArrayDyn<T>`
- Python GIL released during slice traversal
- Arbitrary dimensions supported with no extra allocation
