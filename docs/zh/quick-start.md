---

title: Quick Start
lang: zh

## 快速开始

本指南将帮助您在几分钟内快速上手 pyrs-yaml。

### 1. 安装

该包尚未发布到 PyPI。从源码安装：

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

### 2. 解析 YAML

```python
import pyrs_yaml

# Parse a YAML string
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# Access values
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

### 3. 转换为 Python 对象

```python
# Use safe_load for PyYAML-compatible behavior
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# Returns native Python types (dict, list, str, int, etc.)
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

### 4. 序列化为 YAML

```python
# Convert a Python dict back to YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

### 5. 保留格式（往返解析）

```python
# The key advantage of pyrs-yaml
original = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080

# Database settings
database: &db
  host: localhost
  port: 5432

# Use the database anchor
api:
  <<: *db
  endpoint: /api/v1
"""

# Parse and re-serialize — comments and anchors preserved
doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output
assert "&db" in output
```

### 6. 就地编辑

```python
# 编辑已解析的文档，不丢失注释或格式
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")   # 按路径替换
doc.append("$.server.ports", 443)     # 向序列追加

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 8080
#     - 443
```

参见 [就地编辑指南](guides/editing.md) 了解完整 API。

### 7. 从文件读取 YAML

```python
# Parse a YAML file directly
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### 8. 多文档解析

```python
# Parse multiple YAML documents
yaml_text = """
---

name: config1
value: 1
---
name: config2
value: 2
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # config1
```

## 9. NumPy ndarray 支持

pyrs-yaml 可以将 `numpy.ndarray` 对象直接序列化为 YAML。这对于保存科学数据、模型权重或任何多维数组为人类可读格式非常有用。

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip preserves values
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### 支持的 NumPy 数据类型

| NumPy dtype | YAML output | Notes |
|-------------|-------------|-------|
| `int8/16/32/64` | Plain integer | Quoted if negative |
| `uint8/16/32/64` | Plain integer | — |
| `float32/64` | Plain float | Quoted if negative |
| `complex64/128` | `(re+imj)` string | No native YAML complex type |
| `bool` | `true` / `false` | — |

### 下一步

- **[功能特性](features.md)** — 探索所有支持的 YAML 功能
- **[解析指南](guides/parsing.md)** — 高级解析选项
- **[就地编辑](guides/editing.md)** — 编辑文档而不丢失格式
- **[API 参考](api/reference.md)** — 完整的 API 文档
