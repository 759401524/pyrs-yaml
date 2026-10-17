---
title: NumPy ndarray 序列化指南
description: 将 NumPy 数组序列化为 YAML 格式的完整指南，支持零拷贝 Rust 处理。
tags:
  - docs
status: new
---

将 NumPy 数组序列化为 YAML 列表，支持零拷贝 Rust 处理。

!!! note "自由线程 wheel 包含 NumPy"
    所有 wheel（GIL 与自由线程 cp314t）都编译了 NumPy 集成。NumPy 可导入时该集成自动激活；若未安装 NumPy，对 `numpy.ndarray` 调用 `safe_dump` 会抛出 `YamlTypeError`（值不是受支持类型）。

## 基本用法

```python title="序列化一维数组"
import numpy as np
import pyrs_yaml as y

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

## 多维数组

```python title="二维与三维数组"
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

## 支持的 dtype

| NumPy dtype | YAML 输出 | 示例 |
|-------------|-----------|------|
| :material-numeric: `int8/16/32/64` | 整数 | `42` |
| :material-numeric: `uint8/16/32/64` | 整数 | `42` |
| :material-decimal: `float32/64` | 浮点数 | `3.14` |
| :material-toggle-switch: `bool` | 布尔 | `true` / `false` |
| :material-format-text: `complex64/128` | 字符串 | `(1+2j)` |

!!! note "复数支持"
    YAML 没有原生复数类型，复数会序列化为 `(re+imj)` 字符串。`safe_load` 返回字符串而非 Python `complex` 类型。

## 特殊值

```python title="NaN 与 Infinity"
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

## 负数处理

YAML 1.2 规范不允许在块序列中以 `-` 开头的 plain 标量。负数会自动用单引号包裹以确保正确的往返：

```python title="负数值"
arr = np.array([-100, 200], dtype="int16")
data = y.safe_load(y.safe_dump(arr))
assert data == [-100, 200]  # 往返正确
```

## 0-D 标量数组

0-D 数组直接序列化为标量值，不会包装成列表：

```python title="0-D 标量"
scalar = np.array(42, dtype="int32")
data = y.safe_load(y.safe_dump(scalar))
assert data == 42
```

## 嵌套在结构体中

NumPy 数组可以包含在 dict 或 list 中：

```python title="嵌套在 dict 中"
data = {"matrix": np.array([[1, 2], [3, 4]]), "label": "test"}
yaml_str = y.safe_dump(data)
loaded = y.safe_load(yaml_str)
assert loaded["matrix"] == [[1, 2], [3, 4]]
```

## 不支持的类型

以下类型会抛出 `YamlTypeError`：

- :material-alert: 字符串数组
- :material-alert: 对象数组
- :material-alert: 结构化数组
- :material-alert: 非数值自定义 dtype

## 性能

- :material-bolt: 使用 `PyUntypedArray` 进行零拷贝 dtype 分派
- :material-bolt: 使用 `PyArrayDyn<T>` 进行零拷贝切片迭代
- :material-bolt: 遍历切片时释放 Python GIL
- :material-bolt: 支持任意维度无需额外分配

---

### 另请参阅

- [序列化](serialization.md) — 序列化普通 Python 对象
- [NumPy 特性](../features.md) — NumPy ndarray 支持概览
- [NumPy API 参考](../api/reference.md) — 序列化模块函数
