---

title: 例外
lang: ja

## 例外

pyrs-yaml はエラーハンドリング用にカスタム例外クラスを定義しています。

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
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
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
    result = pyrs_yaml.safe_dump(float("inf"))
except pyrs_yaml.YamlSerializeError as e:
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
    result = pyrs_yaml.safe_dump(object())  # 変換不可な型
except pyrs_yaml.YamlTypeError as e:
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
    doc = pyrs_yaml.parse("age: not_a_number")
    doc.validate(schema={"type": "object", "properties": {"age": {"type": "number"}}})
except pyrs_yaml.YamlValidateError as e:
    print(f"検証エラー: {e}")
```

### YamlEditError

インプレース編集を適用できない場合にスローされます：サポートされない値型（`tuple`）、負のインデックス、エイリアス経由の編集、ルートまたは複合キーのリネーム、スカラーへのナビゲーション、インデックス範囲外。

```python
class YamlEditError(ValueError):
    """インプレース編集エラー（ValueError を継承）。"""
```

**継承元:** `ValueError`

**例:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")

try:
    doc.set("$.a.b.c", 2)  # スカラーへのナビゲーション
except pyrs_yaml.YamlEditError as e:
    print(f"編集エラー: {e}")
```

### YamlPathError

JSONPath スタイルのパスが不正または編集不可の場合にスローされます：`$` で始まらないパス、編集操作でのワイルドカード（`[*]`）またはディープスキャン（`..`）セグメントの使用。

```python
class YamlPathError(ValueError):
    """YAML パスエラー（ValueError を継承）。"""
```

**継承元:** `ValueError`

**例:**

```python
doc = pyrs_yaml.parse("items: [1, 2]")

try:
    doc.set("$.items[*]", 3)  # ワイルドカードは編集不可
except pyrs_yaml.YamlPathError as e:
    print(f"パスエラー: {e}")
```

### YamlDocumentError

`Node` が陳腐化した場合にスローされます — ノード作成後にドキュメントが変更（またはリリース）された。

```python
class YamlDocumentError(Exception):
    """ノードの親 YamlDocument が陳腐化したときにスロー。"""
```

**継承元:** `Exception`

**例:**

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # ドキュメントのリビジョンを増加
node.set_value(99)  # RuntimeWarning + YamlDocumentError
```

### エラーメッセージ形式

すべてのエラーメッセージにはコンテキスト情報が含まれます。

| フィールド | 説明 |
|-----------|------|
| メッセージ | 人間が読めるエラーの説明 |
| Line | エラーが発生した行番号 |
| Column | エラーが発生した列番号 |
| offset | 行内のバイトオフセット |

**編集エラー形式:**

| エラー | 形式 |
|-------|------|
| 編集失敗 | `YAML edit error: <detail>` |
| パス不正 | `YAML path error: <detail>` |

### i18n サポート

エラーメッセージはローカライズできます：

```python
import pyrs_yaml

pyrs_yaml.set_language("zh-CN")  # 中国語
try:
    pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(e)  # 中国語のエラーメッセージ
```

### ベストプラクティス

```python
# 具体的な例外をキャッチ
try:
    doc = pyrs_yaml.parse(yaml_content)
except pyrs_yaml.YamlParseError as e:
    logger.error(f"YAML パースエラー: {e}")
    # エラーメッセージの解析
    error_str = str(e)  # "Invalid YAML: line 1, column 15: ..."
except pyrs_yaml.YamlTypeError as e:
    logger.error(f"型エラー: {e}")
```

**注意:** すべてのカスタム例外は `ValueError` を継承しているため、`except ValueError` で一括キャッチできます。ただし、エラーハンドリングを細かく制御する場合は、具体的な例外クラスを使用してください。
