---

title: Features
lang: zh

## 功能特性

pyrs-yaml 旨在成为 PyYAML 的**直接替代品**，同时添加 PyYAML 缺少的强大功能。

### YAML 1.2 合规

由 **saphyr-parser** 驱动，pyrs-yaml 在 YAML 测试套件中达到 **98.1% 的通过率**。

### 完美的往返解析

与 PyYAML 不同，pyrs-yaml **保留所有格式和元数据**：

- **注释** — 独立注释和行内注释
- **锚点** (`&name`) 和 **别名** (`*name`)
- **标签** (`!!str`、`!!int` 等)
- **修剪指示符** (`|-`、`|+`、`>-`、`>+`)
- **标量样式**（无引号、单引号、双引号、字面量、折叠）
- **流式/块式格式** — 保留 `[]`/`{}` 与块式风格

### 性能

Rust 后端比 PyYAML 快 **25–40 倍**：

| Operation | pyrs-yaml | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.07 ms | 2.92 ms |
| Round-trip | 0.07 ms | 2.90 ms |

### 自定义 AST

**CustomNode** AST 让您完全控制 YAML 结构：

- 以编程方式检查和修改节点
- 添加自定义元数据（注释、锚点、标签）
- 从头构建 YAML，完全控制格式
- 高级用例：模板引擎、配置生成器、代码格式化器

### 与 PyYAML 兼容

直接替换，API 熟悉易用：

```python
import pyrs_yaml as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

### 异步 I/O

通过 `asyncio` 进行非阻塞序列化和解析：

```python
import asyncio
import pyrs_yaml

async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

可用函数：`safe_dump_async`、`safe_load_async`、`safe_loads_async`。

### JSON Schema 验证

根据 JSON Schema 验证解析后的 YAML 文档：

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

验证失败时抛出 `YamlValidateError`。

### 增量重新解析

使用不同选项就地重新解析存储的源文本：

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### NumPy ndarray 支持

pyrs-yaml 可以将任意维度的 `numpy.ndarray` 对象直接序列化为 YAML：

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

#### 支持的数据类型

| 类型 | Rust 后端 | YAML 输出 |
|------|----------|----------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | 普通整数（负数时加引号） |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | 普通整数 |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | 普通浮点数（负数时加引号） |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` 字符串 |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

#### 说明

- **零拷贝**：使用 `numpy` Rust crate 的 `PyUntypedArray` 进行类型擦除的数组访问，然后调度到正确的类型化 `PyArrayDyn<T>` 进行零拷贝切片迭代
- **GIL 释放**：切片迭代在 GIL 外运行，以在大数组上获得最大性能
- **负数**：YAML 1.2 块序列不能包含以 `-` 开头的普通标量；负数值会自动加引号并在往返解析时正确解析回
- **0 维数组**：重塑为 1 维并序列化为单元素列表
- **复数**：YAML 没有原生复数类型；序列化为 `(re+imj)` 字符串。`safe_load` 返回字符串而非 Python `complex`
- **Markdown frontmatter 提取** — `read_markdown()` 用于博客/内容工具
- **JSON ↔ YAML 转换** — `from_json()` / `from_dict()`
- **多文档解析** — `parse_all_docs()`
- **国际化错误消息** — `set_language("zh")` 支持双语错误报告
- **类型提示** — 完整的 `.pyi` 存根文件，支持 IDE

### 支持的 YAML 构造

| 功能 | 支持情况 |
|------|---------|
| YAML 1.2 规范 | ✅ 完全支持 |
| 注释（独立） | ✅ 保留 |
| 注释（行内） | ✅ 保留 |
| 锚点和别名 | ✅ 保留 |
| 标签（显式） | ✅ 保留 |
| 块标量（`|`、`>`） | ✅ 保留 |
| 修剪指示符 | ✅ 保留 |
| 流式集合（`{}`、`[]`） | ✅ 保留 |
| 合并键（`<<`） | ✅ 解析 |
| 复杂键 | ✅ 支持 |
| 转义序列 | ✅ 支持 |
| 多文档 | ✅ 支持 |
| **异步 I/O** | **✅ `safe_*_async`** |
| **JSON Schema 验证** | **✅ `doc.validate()`** |
| **增量重新解析** | **✅ `doc.reparse()`** |
| **JSON 导出** | **✅ `doc.to_json()`** |
