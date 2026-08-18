---
title: 快速开始
description: 帮助您在几分钟内快速上手 pyrs-yaml，涵盖安装、解析、序列化、往返保留和就地编辑。
tags:
  - docs
status: new
---

## 快速开始

本指南将帮助您在几分钟内快速上手 pyrs-yaml。

### 1. 安装

该包尚未发布到 PyPI。从源码安装：

```bash title="从源码安装"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

### 2. 解析 YAML

```python title="解析并访问值"
import pyrs_yaml

# 解析 YAML 字符串
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# 访问值
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

### 3. 转换为 Python 对象

```python title="使用 safe_load 获取原生类型"
# 使用 safe_load 获得 PyYAML 兼容行为
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# 返回原生 Python 类型（dict, list, str, int 等）
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

### 4. 序列化为 YAML

```python title="使用 safe_dump 序列化 dict"
# 将 Python dict 转换回 YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

### 5. 保留格式（往返）

```python title="注释和锚点在往返中保留"
# pyrs-yaml 的核心优势
original = """
# 服务器配置
server:
  host: 0.0.0.0
  port: 8080

# 数据库设置
database: &db
  host: localhost
  port: 5432

# 使用数据库锚点
api:
  <<: *db
  endpoint: /api/v1
"""

# 解析并重新序列化 — 注释和锚点被保留
doc = pyrs_yaml.parse(original)  # (1)!
output = doc.to_yaml()  # (2)!

# 输出与输入匹配（或语义等价）
assert "# 服务器配置" in output  # (3)!
assert "&db" in output  # (4)!
```

1. :material-arrow-down: `parse` 构建 `YamlDocument`，保留每条注释、锚点、标签和样式。
2. :material-arrow-down: `to_yaml` 从 AST 重新序列化，保留格式 — 无需字符串操作。
3. :material-arrow-down: 独立注释完整保留。
4. :material-arrow-down: 锚点（`&db`）、别名（`*db`）和 merge 键（`<<`）均被保留。

### 6. 就地编辑

```python title="按 JSONPath 编辑"
# 编辑已解析的文档，不丢失注释或格式
doc = pyrs_yaml.parse("""
server:
  host: localhost  # 绑定地址
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # 按路径替换
doc.append("$.server.ports", 443)  # 向序列追加

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # 绑定地址
#   ports:
#     - 8080
#     - 443
```

参见 [就地编辑指南](guides/editing.md) 了解完整 API。

### 7. 从文件读取 YAML

```python title="parse_file"
# 直接解析 YAML 文件
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### 8. 多文档解析

```python title="parse_all_docs"
# 解析多个 YAML 文档
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

### 9. NumPy ndarray 支持

??? note "可选：需要 NumPy"

    pyrs-yaml 可以将 `numpy.ndarray` 对象直接序列化为 YAML。这对于保存科学数据、模型权重或任何多维数组为人类可读格式非常有用。

    ```python
    import numpy as np
    import pyrs_yaml

    # 一维数组
    arr = np.array([1, 2, 3], dtype="int32")
    yaml_str = pyrs_yaml.safe_dump(arr)
    print(yaml_str)
    # - 1
    # - 2
    # - 3

    # 二维矩阵
    matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
    yaml_str = pyrs_yaml.safe_dump(matrix)
    print(yaml_str)
    # -
    #   - 1.0
    #   - 2.0
    # -
    #   - 3.0
    #   - 4.0

    # 往返保留值
    loaded = pyrs_yaml.safe_load(yaml_str)
    assert loaded == [[1.0, 2.0], [3.0, 4.0]]
    ```

    #### 支持的 NumPy 数据类型

    | NumPy 数据类型 | YAML 输出 | 备注 |
    |----------------|-----------|------|
    | `int8/16/32/64` | Plain 整数 | 负数时加引号 |
    | `uint8/16/32/64` | Plain 整数 | — |
    | `float32/64` | Plain 浮点数 | 负数时加引号 |
    | `complex64/128` | `(re+imj)` 字符串 | 无原生 YAML 复数类型 |
    | `bool` | `true` / `false` | — |

### 10. 操控元数据（comment, anchor, tag）

```python title="设置注释、锚点、标签"
doc = pyrs_yaml.parse("key: value")
node = doc.node().find("$.key")
node.set_comment("a note")
node.set_anchor("cfg")
node.set_tag("!custom")
print(doc.to_yaml())
# key: &cfg !custom value  # a note
```

### 11. 控制格式（scalar style、flow style、chomping）

```python title="样式、流式、chomping"
doc = pyrs_yaml.parse("key: value")
doc.node().find("$.key").set_scalar_style("single_quoted")
print(doc.to_yaml())  # key: 'value'
```

### 12. 使用 Schema 校验

??? note "可选：YAML Schema Language"

    ```python
    schema = """\
    name: app
    extends: core
    validate:
      - path: $.port
        type: int
        required: true
    """
    pyrs_yaml.validate_against_schema("port: 8080\n", schema)
    ```

### 13. 深度编辑（批量设置、排序、移动、复制）

```python title="批量、排序、移动、复制"
doc.set_many({"$.items[*].active": False})
doc.sort_keys()
```

### 下一步

<div class="grid cards" markdown>

- :material-feature-search: **[功能特性](features.md)** — 探索所有支持的 YAML 功能
- :material-file-search: **[解析指南](guides/parsing.md)** — 高级解析选项
- :material-pencil: **[就地编辑](guides/editing.md)** — 编辑文档而不丢失格式
- :material-book-open-variant: **[配置管理教程](guides/tutorial-config-management.md)** — 端到端实战
- :material-code-braces: **[API 参考](api/reference.md)** — 完整的 API 文档

</div>
