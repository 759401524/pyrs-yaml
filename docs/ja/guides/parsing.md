---

title: YAML のパース
lang: ja

## YAML のパース

このガイドでは、pyyaml-rs で YAML をパースするすべての方法を説明します。

### 基本パース

#### YAML 文字列のパース

```python
import pyyaml_rs

doc = pyyaml_rs.parse("key: value")
print(doc.get("key"))  # value
```

#### オプション付きパース

```python
# マージキーの解決を無効化（<<: *alias をそのまま保持）
doc = pyyaml_rs.parse(yaml_text, resolve_merges=False)
```

#### YAML ファイルのパース

```python
doc = pyyaml_rs.parse_file("config.yaml")
print(doc.get("name"))
```

#### 複数ドキュメントのパース

```python
# --- 区切りの YAML
yaml_text = """
---

name: first
---
name: second
"""

docs = pyyaml_rs.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

## PyYAML 互換パース

```python
# ネイティブ Python 型を返す（dict, list, str, int など）
data = pyyaml_rs.safe_load("key: value")
print(data)  # {'key': 'value'}

# 複数ドキュメント
docs = pyyaml_rs.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

### 受け付ける入力型

- `str` — 標準 YAML 文字列
- `bytes` — 有効な UTF-8 エンコードバイト列
- `str` に BOM あり — 正しく処理される

```python
# str と bytes の両方を受け付ける
doc1 = pyyaml_rs.parse("key: value")
doc2 = pyyaml_rs.parse(b"key: value")
```

### エラーハンドリング

```python
try:
    doc = pyyaml_rs.parse("invalid: yaml: [")
except pyyaml_rs.YamlParseError as e:
    print(f"パースエラー: {e}")
```

### サポートされるデータ型

pyyaml-rs はすべての YAML 1.2 スカラータイプを正しくパースします。

| 型 | 例 | Python 型 |
|---|-----|----------|
| 文字列 | `hello` | `str` |
| 整数 | `42`, `0x1A`, `0o77` | `int` |
| 浮動小数点 | `3.14`, `1.23e-4` | `float` |
| ブーリアン | `true`, `false` | `bool` |
| Null | `null`, `~` | `None` |
| 無限大 | `.inf`, `-.inf` | `float` |
| NaN | `.nan` | `float` |
