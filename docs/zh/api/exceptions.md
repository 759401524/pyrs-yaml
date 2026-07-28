---

title: 异常
lang: zh

## 异常

pyrs-yaml 定义了三个自定义异常类用于错误处理。

### YamlParseError

YAML 解析失败时引发。

```python
class YamlParseError(ValueError):
    """YAML 解析错误（继承自 ValueError）。"""
```

**继承自:** `ValueError`

**示例:**

```python
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"解析错误: {e}")
```

**错误消息示例:**

- `Invalid YAML: line 1, column 15: did not find expected key`
- `YAML parse error at line 2, column 1: mapping values are not allowed here`

### YamlSerializeError

YAML 序列化失败时引发。

```python
class YamlSerializeError(ValueError):
    """YAML 序列化错误（继承自 ValueError）。"""
```

**继承自:** `ValueError`

**示例:**

```python
try:
    result = pyrs_yaml.safe_dump(float('inf'))
except pyrs_yaml.YamlSerializeError as e:
    print(f"序列化错误: {e}")
```

### YamlTypeError

类型转换错误时引发。

```python
class YamlTypeError(TypeError):
    """类型转换错误（继承自 TypeError）。"""
```

**继承自:** `TypeError`

**示例:**

```python
try:
    result = pyrs_yaml.safe_dump(object())  # 不可转换的类型
except pyrs_yaml.YamlTypeError as e:
    print(f"类型错误: {e}")
```

### YamlValidateError

JSON Schema 验证失败时引发。

```python
class YamlValidateError(ValueError):
    """JSON Schema 验证错误（继承自 ValueError）。"""
```

**继承自:** `ValueError`

**示例:**

```python
try:
    doc = pyrs_yaml.parse("age: not_a_number")
    doc.validate(schema={"type": "object", "properties": {"age": {"type": "number"}}})
except pyrs_yaml.YamlValidateError as e:
    print(f"验证错误: {e}")
```

### 错误消息格式

所有错误消息都包含上下文信息：

| 字段 | 说明 |
|-----|------|
| 消息 | 人类可读的错误描述 |
| Line | 错误发生的行号 |
| Column | 错误发生的列号 |
| offset | 行内的字节偏移量 |

### i18n 支持

错误消息可以本地化：

```python
import pyrs_yaml

pyrs_yaml.set_language("zh-CN")  # 中文
try:
    pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(e)  # 中文错误消息
```

### 最佳实践

```python
# 捕获具体的异常
try:
    doc = pyrs_yaml.parse(yaml_content)
except pyrs_yaml.YamlParseError as e:
    logger.error(f"YAML 解析错误: {e}")
    # 错误消息的解析
    error_str = str(e)  # "Invalid YAML: line 1, column 15: ..."
except pyrs_yaml.YamlTypeError as e:
    logger.error(f"类型错误: {e}")
```

**注意:** 所有自定义异常都继承自 `ValueError`，因此可以用 `except ValueError` 批量捕获。但为了更细粒度的错误处理，建议使用具体的异常类。
