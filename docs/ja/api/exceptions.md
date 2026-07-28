---

title: 例外
lang: ja

## 例外

pyyaml-rs はエラーハンドリング用に3つのカスタム例外クラスを定義しています。

### YamlParseError

YAML パースに失敗した場合にスローされます。

```python
class YamlParseError(ValueError):
    """YAML パースエラー（ValueError を継承）。"""
```

**継承元:** `ValueError`

**例:**

```python
try:
    doc = pyyaml_rs.parse("invalid: yaml: [")
except pyyaml_rs.YamlParseError as e:
    print(f"パースエラー: {e}")
```

**エラーメッセージ例:**

- `Invalid YAML: line 1, column 15: did not find expected key`
- `YAML parse error at line 2, column 1: mapping values are not allowed here`

### YamlSerializeError

YAML シリアライズに失敗した場合にスローされます。

```python
class YamlSerializeError(ValueError):
    """YAML シリアライズエラー（ValueError を継承）。"""
```

**継承元:** `ValueError`

**例:**

```python
try:
    result = pyyaml_rs.safe_dump(float('inf'))
except pyyaml_rs.YamlSerializeError as e:
    print(f"シリアライズエラー: {e}")
```

### YamlTypeError

型変換エラーが発生した場合にスローされます。

```python
class YamlTypeError(TypeError):
    """型変換エラー（TypeError を継承）。"""
```

**継承元:** `TypeError`

**例:**

```python
try:
    result = pyyaml_rs.safe_dump(object())  # 変換不可な型
except pyyaml_rs.YamlTypeError as e:
    print(f"型エラー: {e}")
```

### YamlValidateError

JSON Schema 検証が失敗した場合にスローされます。

```python
class YamlValidateError(ValueError):
    """JSON Schema 検証エラー（ValueError を継承）。"""
```

**継承元:** `ValueError`

**例:**

```python
try:
    doc = pyyaml_rs.parse("age: not_a_number")
    doc.validate(schema={"type": "object", "properties": {"age": {"type": "number"}}})
except pyyaml_rs.YamlValidateError as e:
    print(f"検証エラー: {e}")
```

### エラーメッセージ形式

すべてのエラーメッセージにはコンテキスト情報が含まれます。

| フィールド | 説明 |
|-----------|------|
| メッセージ | 人間が読めるエラーの説明 |
| Line | エラーが発生した行番号 |
| Column | エラーが発生した列番号 |
| offset | 行内のバイトオフセット |

### i18n サポート

エラーメッセージはローカライズできます：

```python
import pyyaml_rs

pyyaml_rs.set_language("zh-CN")  # 中国語
try:
    pyyaml_rs.parse("invalid: yaml: [")
except pyyaml_rs.YamlParseError as e:
    print(e)  # 中国語のエラーメッセージ
```

### ベストプラクティス

```python
# 具体的な例外をキャッチ
try:
    doc = pyyaml_rs.parse(yaml_content)
except pyyaml_rs.YamlParseError as e:
    logger.error(f"YAML パースエラー: {e}")
    # エラーメッセージの解析
    error_str = str(e)  # "Invalid YAML: line 1, column 15: ..."
except pyyaml_rs.YamlTypeError as e:
    logger.error(f"型エラー: {e}")
```

**注意:** すべてのカスタム例外は `ValueError` を継承しているため、`except ValueError` で一括キャッチできます。ただし、エラーハンドリングを細かく制御する場合は、具体的な例外クラスを使用してください。
