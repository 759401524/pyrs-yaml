title: インプレース編集
lang: ja

# インプレース編集

pyrs-yaml では、**パース済みドキュメントをその場で編集**でき、すべてのフォーマットメタデータ（コメント、アンカー、タグ、スカラースタイル、フロー/ブロックスタイル）を保持します — 手作業による文字列加工は不要で、忠実性の損失もありません。

## 概要

編集は、ドキュメントツリーへの **JSONPath スタイルのパス** で表現します：

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
db:
  host: localhost
  port: 5432
""")

doc.set("$.db.host", "db.example.com")  # set by path
doc.set("$.db.port", 5433)
print(doc.to_yaml())
# db:
#   host: db.example.com
#   port: 5433
```

すべての編集メソッドは**アトミック**です：失敗した場合、ドキュメント（リビジョンを含む）は何も変更されません。成功するとドキュメントはダーティとしてマークされ、次の `source()` / `to_yaml()` / `to_yaml_with_options()` / `reparse()` 呼び出しで更新されたツリーから再シリアライズされます。

## パス構文

パスは `$` で始まり、ドット区切りのキー（マッピング）または `[N]` インデックス（シーケンス）が続きます：

| パス | 意味 |
|------|------|
| `$.host` | ルートマッピングのキー `host` |
| `$.a.b.c` | ネストされたキー |
| `$.items[0]` | シーケンス `items` の最初の要素 |
| `$` | ルートノード自体 |

- **負のインデックス**（`[-1]`、`[-2]`、...）は**サポートされています** — シーケンスの末尾から数えます（Python と同じセマンティクス：`-1` は最後の要素）。範囲外の負のインデックスは `YamlEditError` を発生させます
- キーは**値でマッチ**します（メタデータには依存しません）。そのため、クォート付きキー `"host"` はプレーンキー `host` にマッチします

編集パスは正確に 1 つのノードを対象とする必要があります — **ワイルドカード**（`[*]`）と**ディープスキャン**（`..`）は `YamlPathError` を発生させます。（クエリ専用の `find()` ではこれらを使用できます。[`find()` によるクエリ](#find) を参照してください。）

**発生する例外:** 不正なパスでは `YamlPathError` が、パスのステップを適用できない場合（スカラーへのナビゲーションやエイリアス経由の編集など）は `YamlEditError` が発生します。

## 値の設定

### `set()` — パスによる置換

```python
set(path: str, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("a:\n  b: 1\nitems: [1, 2, 3]")

doc.set("$.a.b", 42)  # scalar → scalar, metadata preserved
doc.set("$.items[1]", "two")  # sequence index
doc.set("$.a.c", True)  # add a new key to a mapping (last position)
doc.set("$", {"x": 1})  # replace the entire root
```

値の変換ルール：

| Python 値 | YAML ノード |
|-----------|-------------|
| `str`, `int`, `float`, `bool`, `None` | 新しいスカラー（値は*再パースされません*） |
| `dict` | 新しいマッピング（プレーンスタイル） |
| `list` | 新しいシーケンス（プレーンスタイル） |
| `tuple` | ❌ サポートされません — `YamlEditError` を発生 |

既存のスカラーを置換する場合、対象のメタデータ（インラインコメント、アンカー、タグ、クォートスタイル）は**保持**されます — ただし、新しい値がマッピング/シーケンスの場合は、新しいノード自身のフォーマットが採用されます。

### `__setitem__` — ルート用糖衣構文

```python
doc["b"] = 2  # equivalent to doc.set("$.b", 2)
```

### `Node.set_value()` — ノード経由の編集

```python
node = doc.node().find("$.a.b")  # see "Working with Nodes"
node.set_value(42)
```

## 挿入と追加

どちらも**シーケンスのみ**を対象とします。パスはシーケンスノードに解決される必要があります。

### `insert()` — インデックス位置への挿入

```python
insert(path: str, index: int, value: Any) -> None
```

`index` は現在の長さまで指定できます（`len` に挿入すると末尾への追加になります）。それより大きい場合は `YamlEditError` を発生します。負のインデックスは末尾から数えます（`-1` は最後の要素の前に挿入、`-len` は先頭に挿入）。

```python
doc = pyrs_yaml.parse("items:\n  - a\n  - c")

doc.insert("$.items", 1, "b")  # items: [a, b, c]
doc.insert("$.items", 0, "first")
doc.insert("$.items", 3, "last")  # index == len appends
doc.insert("$.items", -1, "before-last")  # items: [a, before-last, c]
```

### `append()` — 末尾に追加

```python
append(path: str, value: Any) -> None
```

```python
doc.append("$.items", "d")
```

### `Node.append()` / `Node.insert()`

同じ操作を `Node` オブジェクトでも利用できます：

```python
node = doc.node().find("$.items")
node.append("d")
node.insert(1, "x")
```

## 削除

### `delete()` — パスによる削除

```python
delete(path: str) -> None
```

```python
doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
doc.delete("$.b")
print(doc.to_yaml())  # a: 1\nc: 3\n — order preserved
```

マッピングの順序は常に保持されます。シーケンスの削除では隙間が詰められます。

### `__delitem__` — ルート用糖衣構文

```python
del doc["b"]  # equivalent to doc.delete("$.b")
```

### `Node.delete()`

```python
node = doc.node().find("$.b")
node.delete()
```

## リネーム

### `rename()` — マッピングキーのその場リネーム

```python
rename(path: str, new_key: str) -> None
```

パスは**マッピングキー**を指している必要があります（値はそのキーの下にあり、メタデータを保持します）：

```python
doc = pyrs_yaml.parse("old: value  # keep me\nnext: 1")
doc.rename("$.old", "new")
print(doc.to_yaml())  # new: value  # keep me\nnext: 1
```

- **位置は保持** — リネームされたキーは元の位置に留まります
- **メタデータは保持** — キーのインラインコメント、スタイル、アンカーはリネームとともに移動します
- ルートや複合（非スカラー）キーのリネーム、および**既存キーへの**リネームは `YamlEditError` を発生させます（同一キーへのリネームは何も行いません）

### `Node.rename()`

```python
node = doc.node().find("$.old")
node.rename("new")
```

## ノードの操作

`doc.node()` はドキュメントルートの `Node` を返し、`Node.find(path)` はサブツリーに移動します：

```python
node = doc.node()  # root node
node = doc.node().find("$.db.host")  # navigate by path
print(node.value)  # "localhost"
node.set_value("other")  # edit through the node
print(node.root_type)  # "scalar" | "mapping" | "sequence" | "null"
```

ノードはツリー API を公開しています：`node.parent`、`node.children`、`node.walk()`（深さ優先イテレーター）、`node.filter(predicate)`、`node.to_yaml()`。

### `find()` によるクエリ

`find()` は**読み取り指向**で、ワイルドカードとディープスキャンをサポートします — パスが複数のノードを選択する場合はリストを返します：

```python
doc.node().find("$.items[*]")  # all items of a sequence (list of Nodes)
doc.node().find("$..timeout")  # deep search for any key named "timeout"
```

ワイルドカード/ディープスキャンの結果は**直接編集できません** — パスの特定に使用し、編集は `set()` / `insert()` などで行ってください。

## エイリアスとマージキー

エイリアスノード（`*name`）は、その自身のパスが設定されると**その場で**置き換えられます：

```python
yaml = "defaults: &defaults\n  timeout: 30\nprod: *defaults\n"
doc = pyrs_yaml.YAML(typ="safe").parse(yaml)  # resolve_merges=false keeps the alias node

doc.set("$.prod", {"timeout": 99})  # replaces the alias node — prod.timeout: 99
```

- エイリアス**経由**の設定（`*defaults` を通過してマージされたキーに到達する）は `YamlEditError` を発生させます — 参照先ノードは別の場所にあります
- マージキーを解決した場合（デフォルト）、マージ展開されたキーはクローンです。それらを編集してもクローンのみが編集されます
- アンカー付きノードの削除は許容されます（アンカーが参照されなくなるだけです）

## ビューと AST

`doc.get()` / `doc.to_dict()` は**ビュー**（解決された値）を返します。編集は常に**AST**に対して行われます：

```python
doc = pyrs_yaml.parse("on: yes")
print(doc.get("on"))  # True   — view (core schema resolution)
doc.set("$.on", "off")  #         — edits the AST scalar
print(doc.to_yaml())  # on: off — serialized verbatim, no re-resolution
```

編集された値は**そのまま**出力されます。ビューはアクティブなスキーマに従ってそれを解決します。

## 陳腐化したノード

`Node` はドキュメントの**リビジョン**に結び付けられており、ノードの作成時に記録されます。ドキュメントの編集（別のノード経由でも）はリビジョンを増加させるため、以前に取得したノードは陳腐化します：

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # bumps the revision
node.set_value(99)  # RuntimeWarning + YamlDocumentError (stale)
```

編集後はノードを再取得して作業を続けてください。`node.is_valid()` は生存性をチェックし、`node.release()` はノードをドキュメントから明示的に切り離します。

## エラーハンドリング

| エラー | 発生時 |
|-------|--------|
| `YamlPathError` | 不正なパス、編集パスでのワイルドカード/`..` の使用 |
| `YamlEditError` | サポートされない値型（`tuple`）、エイリアス経由の編集、ルート/複合/既存キーのリネーム、スカラーへのナビゲーション、インデックス範囲外 |
| `YamlDocumentError` | ドキュメント編集後に陳腐化した `Node` を使用 |

すべての編集はアトミックです — 失敗した編集はドキュメント（とそのリビジョン）に影響を与えません。

## 完全な例

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
# server config
server:
  host: localhost  # bind address
  ports:
    - 8080
    - 9090
""")

doc.set("$.server.host", "0.0.0.0")
doc.insert("$.server.ports", 0, 80)
doc.append("$.server.ports", 443)
doc.rename("$.server", "srv")

print(doc.to_yaml())
# server config
# srv:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 80
#     - 8080
#     - 9090
#     - 443
```

コメント、アンカー、タグ、スカラースタイル、フロー/ブロックフォーマットはすべて保持されます。

## パフォーマンス

**デフォルトレイアウト**のドキュメント（ブロックスタイル、2スペースインデント、CRLF/BOMなし）では、編集は**バイトレベルのスプライス**として適用されます — タッチされた領域のみ再生成され、未変更テキストはそのままコピーされます。これにより、編集+フラッシュが大規模ドキュメントでの全再シリアライズと比較して**最大100倍高速**になります。

**フォールバック**（全再シリアライズ）は以下で発生します：

- 編集されたノードまたはその祖先が**フロースタイル**（`{...}`、`[...]`）を使用
- ドキュメントが**非デフォルトレイアウト**（CRLF行末、BOM、非標準インデント）
- ドキュメントに**マージキー**（`<<: *anchor`）が含まれる
- 単一文字列から複数ドキュメントが解析された
- スプライス状態が以前の materialize で**消費**された（シングルバーストモデル）

すべてのフォールバックケースで、正確性は保証されます — パフォーマンス上の利点のみが失われます。

### ベンチマーク

```text
Benchmark                   Median
serialize_10mb             17 ms
edit_flush_set_10mb       110 ms
edit_flush_burst5_10mb    119 ms
```

500グループ×838キーの合成10MBブロックマッピングドキュメントで測定。比率はASTクローンコスト（56ms）が支配的；実際の編集+materializeは約54ms（シリアライズの3倍）。コメント、アンカー、タグを含む複雑なドキュメントでは、スプライスの利点が大幅に拡大します。
