---
title: 序列化
description: 将 Python 对象和 YamlDocument 实例转换为 YAML 字符串的完整指南。
tags:
  - docs
status: new
---

## 序列化

将 Python 对象和 `YamlDocument` 实例转换为 YAML 字符串。

### 基本序列化

#### YamlDocument.to_yaml()

```python title="to_yaml()"
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()  # (1)!
print(yaml_str)  # key: value\n
```

1. `to_yaml()` 序列化时保留所有注释、锚点和格式。

#### YamlDocument.to_yaml_with_options()

```python title="to_yaml_with_options()"
doc = pyrs_yaml.parse("key: value")

# 自定义缩进和文档标记
yaml_str = doc.to_yaml_with_options(
    indent_size=4,  # 每级缩进 4 个空格
    explicit_start=True,  # 在开头添加 "---"
    explicit_end=True,  # 在结尾添加 "..."
    sort_keys=True,  # 按字母顺序排序键
)
```

#### PyYAML 兼容序列化

```python title="PyYAML 兼容序列化"
# 将 dict 转换为 YAML 字符串
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432}})

# safe_dumps（别名）也可用
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

### 将 Python 对象转换为 YAML

#### from_dict()

```python title="from_dict()"
yaml_str = pyrs_yaml.from_dict({"name": "Alice", "age": 30, "tags": ["admin", "user"]})
```

#### from_json()

```python title="from_json()"
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

#### dump_file()

```python title="dump_file()"
# 将 Python 对象直接写入 YAML 文件
pyrs_yaml.dump_file({"config": {"debug": True, "log_level": "info"}}, "output.yaml")
```

### 输出格式

pyrs-yaml 可以序列化到不同的目标：

=== "字符串"

    ```python title="YAML 字符串"
    yaml_str = pyrs_yaml.safe_dump({"key": "value"})
    ```

=== "文件"

    ```python title="YAML 文件"
    pyrs_yaml.dump_file({"key": "value"}, "output.yaml")
    ```

=== "文档"

    ```python title="YamlDocument"
    doc = pyrs_yaml.parse("key: value")
    yaml_str = doc.to_yaml()
    ```

### 支持的输入类型

| Python 类型 | YAML 输出 |
|-------------|-----------|
| :material-language-python: `dict` | YAML 映射 |
| :material-format-list-numbered: `list` | YAML 序列 |
| :material-format-text: `str` | Plain 或引号标量 |
| :material-numeric: `int` | Plain 整数 |
| :material-decimal: `float` | Plain 浮点数 |
| :material-toggle-switch: `bool` | `true` / `false` |
| :material-null: `None` | `null` |

### 往返

```python title="往返保留"
# 核心优势：格式被保留
original = """
# 服务器配置
server:
  host: 0.0.0.0
  port: 8080  # 主端口

database: &db
  host: localhost

api:
  <<: *db
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# 注释、锚点和合并键被保留
assert "# 服务器配置" in output
assert "&db" in output
assert "<<: *db" in output
```

---

### 另请参阅

- [解析 YAML](parsing.md) — 解析字符串、文件和多个文档
- [往返保留](round-trip.md) — 注释和锚点在序列化中如何保留
- [PyYAML 兼容](pyyaml-compat.md) — 直接替换 API
