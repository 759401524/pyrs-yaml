---

title: Features
lang: ja

## 機能

pyrs-yaml は PyYAML の**直接置換**として設計されており、PyYAML にない強力な機能を追加しています。

### YAML 1.2 準拠

**saphyr-parser** により駆動され、YAML テストスイートで **98.1% の合格率**を達成。

### 完璧なラウンドトリップ

PyYAML と異なり、pyrs-yaml は**すべてのフォーマットとメタデータを保持**します：

- **コメント** — 独立コメントとインラインコメント
- **アンカー** (`&name`) と **エイリアス** (`*name`)
- **タグ** (`!!str`、`!!int` など)
- **チョーピングインジケーター** (`|-`、`|+`、`>-`、`>+`)
- **スカラースタイル**（プレーン、シングルクォート、ダブルクォート、リテラル、フォールド）
- **フロー/ブロックフォーマット** — `[]`/`{}` とブロックスタイルを保持

### パフォーマンス

Rust バックエンドは PyYAML より **25–40 倍高速**：

| Operation | pyrs-yaml | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.07 ms | 2.92 ms |
| Round-trip | 0.07 ms | 2.90 ms |

### カスタム AST

**CustomNode** AST は YAML 構造を完全に制御できます：

- プログラムでノードを検査・修正
- カスタムメタデータ（コメント、アンカー、タグ）を追加
- フォーマットを完全に制御して YAML をゼロから構築
- 高度なユースケース：テンプレートエンジン、設定ジェネレーター、コードフォーマッター

### PyYAML 互換性

使い慣れた API で直接置き換え可能：

```python
import pyrs_yaml as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

### 非同期 I/O

`asyncio` を使用した非ブロッキングシリアライズとパース：

```python
import asyncio
import pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dump_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

利用可能な関数：`safe_dump_async`、`safe_load_async`、`safe_loads_async`。

### JSON Schema 検証

JSON Schema に基づいてパースされた YAML ドキュメントを検証：

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

検証に失敗した場合、`YamlValidateError` をスローします。

### インクリメンタル再パース

異なるオプションで保存されたソーステキストをその場で再パース：

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### インプレース編集

解析済みドキュメントを**フォーマットメタデータを一切失わずに**編集します — コメント、アンカー、タグ、スカラースタイル、フロー/ブロックスタイルはすべて保持されます：

```python
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")     # パスで置換
doc.insert("$.server.ports", 0, 80)     # シーケンスに挿入
doc.append("$.server.ports", 443)       # シーケンスに追加
doc.rename("$.server", "srv")           # マッピングキーをリネーム
del doc["server"]                       # または: doc.delete("$.server")
```

- **パス API** — JSONPath スタイルのパス（`$.a.b[0]`）、ルート用糖衣構文（`doc["k"] = v`、`del doc["k"]`）
- **ノード API** — `doc.node().find(path)` は `Node` オブジェクトを返し、`set_value` / `insert` / `append` / `delete` / `rename` とツリー走査（`parent`、`children`、`walk`、`filter`）をサポート
- **原子性** — 失敗した編集はドキュメント（リビジョンを含む）を変更しません
- **メタデータ保持** — 置換されたスカラーはコメント/アンカー/タグ/クォートを保持；リネームされたキーは位置とコメントを保持
- **エイリアス対応** — エイリアス自身のパスへの設定はその場で置換；エイリアス*経由*の編集は `YamlEditError` をスロー

詳細は [インプレース編集ガイド](guides/editing.md) を参照してください。

### NumPy ndarray サポート

pyrs-yaml は任意次元の `numpy.ndarray` オブジェクトを直接 YAML にシリアライズできます：

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

#### サポートされるデータ型

| 型 | Rust バックエンド | YAML 出力 |
|----|------------------|----------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | プレーン整数（負の場合は引用符付き） |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | プレーン整数 |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | プレーン浮動小数点（負の場合は引用符付き） |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` 文字列 |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

#### 注記

- **ゼロコピー**：`numpy` Rust クレートの `PyUntypedArray` を使用して型消去された配列アクセスを行い、正しい型付き `PyArrayDyn<T>` にディスパッチしてゼロコピー切片反復を実行
- **GIL リリース**：切片反復は GIL 外で実行され、大きな配列で最大のパフォーマンスを発揮
- **負の数**：YAML 1.2 ブロックシーケンスには `-` で始まるプレーンスカラーを含めることはできません。負の値は自動的に引用符で囲まれ、ラウンドトリップ時に正しくパースされます
- **0 次元配列**：1 次元にリシェイプされ、単一アイテムリストとしてシリアライズされます
- **複数**：YAML にはネイティブの複数型がありません。(re+imj)` 文字列としてシリアライズされます。`safe_load` は Python `complex` ではなく文字列として返します
- **Markdown frontmatter 抽出** — `read_markdown()` ブログ/コンテンツツール用
- **JSON ↔ YAML 変換** — `from_json()` / `from_dict()`
- **複数ドキュメントパース** — `parse_all_docs()`
- **国際化エラーメッセージ** — `set_language("ja")` バイリンガルエラー用
- **型ヒント** — IDE サポート用の完全な `.pyi` スタブ

### サポートされる YAML 構造

| 機能 | サポート |
|------|---------|
| YAML 1.2 仕様 | ✅ 完全 |
| コメント（独立） | ✅ 保持 |
| コメント（インライン） | ✅ 保持 |
| アンカーとエイリアス | ✅ 保持 |
| タグ（明示的） | ✅ 保持 |
| ブロックスカラー（`|`、`>`） | ✅ 保持 |
| チョーピングインジケーター | ✅ 保持 |
| フローコレクション（`{}`、`[]`） | ✅ 保持 |
| マージキー（`<<`） | ✅ 解決 |
| 複合キー | ✅ サポート |
| エスケープシーケンス | ✅ サポート |
| 複数ドキュメント | ✅ サポート |
| **非同期 I/O** | **✅ `safe_*_async`** |
| **JSON Schema 検証** | **✅ `doc.validate()`** |
| **インクリメンタル再パース** | **✅ `doc.reparse()`** |
| **インプレース編集** | **✅ `doc.set()` / `insert()` / `append()` / `delete()` / `rename()`** |
| **JSON エクスポート** | **✅ `doc.to_json()`** |
