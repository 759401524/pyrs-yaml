---

title: 序列化
lang: zh

# 序列化

将 Python 对象和 `YamlDocument` 实例转换为 YAML 字符串。

## 基本序列化

### YamlDocument.to_yaml()

```python
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()
print(yaml_str)  # key: value\n
```

### YamlDocument.to_yaml_with_options()

```python
doc = pyrs_yaml.parse("key: value")

# 自定义缩进和文档标记
yaml_str = doc.to_yaml_with_options(
    indent_size=4,  # 每级缩进 4 个空格
    explicit_start=True,  # 在开头添加 "---"
    explicit_end=True,  # 在结尾添加 "..."
    sort_keys=True,  # 按字母顺序排序键
)
```

### PyYAML 兼容序列化

```python
# 将 dict 转换为 YAML 字符串
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432}})

# safe_dumps（别名）也可用
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

## 将 Python 对象转换为 YAML

### from_dict()

```python
yaml_str = pyrs_yaml.from_dict({"name": "Alice", "age": 30, "tags": ["admin", "user"]})
```

### from_json()

```python
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

### dump_file()

```python
# 将 Python 对象直接写入 YAML 文件
pyrs_yaml.dump_file({"config": {"debug": True, "log_level": "info"}}, "output.yaml")
```

## 支持的输入类型

| Python 类型 | YAML 输出 |
|-------------|-----------|
| `dict` | YAML 映射 |
| `list` | YAML 序列 |
| `str` | Plain 或引号标量 |
| `int` | Plain 整数 |
| `float` | Plain 浮点数 |
| `bool` | `true` / `false` |
| `None` | `null` |

## 往返

```python
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
