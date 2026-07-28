# ---

---

title: NumPy ndarray 序列化指南
lang: zh-CN

## NumPy ndarray 序列化指南

将 NumPy 数组序列化为 YAML 列表，支持零拷贝 Rust 处理。

### 基本用法

```python
import numpy as np
import pyyaml_rs as y

# 1-D 数组
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = y.safe_dump(arr)
# 输出:
# - 1
# - 2
# - 3

# 还原回 Python 列表
data = y.safe_load(yaml_str)
assert data == [1, 2, 3]
```

### 多维数组

```python
# 2-D 矩阵
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = y.safe_dump(matrix)
data = y.safe_load(yaml_str)
assert data == [[1.0, 2.0], [3.0, 4.0]]

# 3-D 立方体
cube = np.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], dtype="int64")
data = y.safe_load(y.safe_dump(cube))
assert data == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
```

### 支持的 dtype

| NumPy dtype | YAML 输出 | 示例 |
|-------------|-----------|------|
| `int8/16/32/64` | 整数 | `42` |
| `uint8/16/32/64` | 整数 | `42` |
| `float32/64` | 浮点数 | `3.14` |
| `bool` | 布尔 | `true` / `false` |
| `complex64/128` | 字符串 | `(1+2j)` |

### 特殊值

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

### 负数处理

YAML 1.2 规范不允许在块序列中以 `-` 开头的 plain 标量。负数会自动用单引号包裹以确保正确的往返：

```python
arr = np.array([-100, 200], dtype="int16")
data = y.safe_load(y.safe_dump(arr))
assert data == [-100, 200]  # 往返正确
```

### 0-D 标量数组

0-D 数组会被 reshape 为 1-D 后序列化，结果是一个单元素列表：

```python
scalar = np.array(42, dtype="int32")
data = y.safe_load(y.safe_dump(scalar))
assert data == [42]
```

### 嵌套在结构体中

NumPy 数组可以包含在 dict 或 list 中：

```python
data = {"matrix": np.array([[1, 2], [3, 4]]), "label": "test"}
yaml_str = y.safe_dump(data)
loaded = y.safe_load(yaml_str)
assert loaded["matrix"] == [[1, 2], [3, 4]]
```

### 不支持的类型

以下类型会抛出 `YamlTypeError`：

- 字符串数组
- 对象数组
- 结构化数组
- 非数值自定义 dtype

### 性能

- 使用 `PyUntypedArray` 进行零拷贝 dtype 分派
- 使用 `PyArrayDyn<T>` 进行零拷贝切片迭代
- 遍历切片时释放 Python GIL
- 支持任意维度无需额外分配
