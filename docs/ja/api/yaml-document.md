---
title: YamlDocument クラス
description: pyrs-yaml のコアクラス YamlDocument の API リファレンス。プロパティ、編集、ダンダーメソッドをカバーします。
tags:
  - docs
status: new
---

## 概要

`YamlDocument` は pyrs-yaml のコアクラスで、解析済みの YAML ドキュメントを保持します。`IndexMap` を使用したカスタム AST により、**100% ラウンドトリップ**、**完全なキー順序保持**、**ネストされたコメントの保持**、**詳細なメタデータ**を実現します。

```python
class YamlDocument:
    """pyrs-yaml のコアクラス。"""

    # ... C 拡張で実装 ...
```

## コンストラクター

### `YamlDocument()`

内部コンストラクター。ユーザーが直接呼び出すことはありません。`pyrs_yaml.parse()` から返されます。

## プロパティ

- `version` — YAML ドキュメントバージョン
- `schema` — スキーマ（`core`, `failsafe`, `json`）
- `tags` — タグ一覧
- `anchors` — アンカー一覧
- `source` — YAML ソーステキスト

## メソッド

### `to_yaml()`

ドキュメントを YAML 文字列に変換します。

```python
to_yaml(
    indent: int = 2,
    allow_unicode: bool = True,
    default_flow_style: bool = False,
    sort_keys: bool = False,
    width: int = 80,
    resolve_aliases: bool = True,
    strip_comments: bool = False,
    preserve_quotes: bool = True,
) -> str
```

**パラメータ:**

- `indent` — インデントスペース数（デフォルト: 2）
- `allow_unicode` — Unicode 文字を許可（デフォルト: True）
- `default_flow_style` — デフォルトでフロースタイルを使用（デフォルト: False）
- `sort_keys` — キーをソート（デフォルト: False）
- `width` — 折り返し幅（デフォルト: 80）
- `resolve_aliases` — エイリアスを解決（デフォルト: True）
- `strip_comments` — コメントを除去（デフォルト: False）
- `preserve_quotes` — クォートを保持（デフォルト: True）

**戻り値:** YAML 文字列

**例:**

```python
doc = pyrs_yaml.parse("key: value\n# comment")
yaml_str = doc.to_yaml()
```

### `to_yaml_with_options()`

カスタムオプションで YAML に変換します。

```python
to_yaml_with_options(
    indent_size: int = 2,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
    note_placement: str = "note_first",
) -> str
```

**パラメータ:**

- `indent_size` — インデントレベルごとのスペース数 (デフォルト: 2)
- `explicit_start` — ドキュメント先頭に `---` を追加 (デフォルト: False)
- `explicit_end` — ドキュメント末尾に `...` を追加 (デフォルト: False)
- `sort_keys` — キーをアルファベット順にソート (デフォルト: False)
- `note_placement` — 備考が自分の行に乗れず、直上が `&a` / `!tag` ヘッダー行になるときの位置: `note_first`（既定）はヘッダーの上に書きます。読み戻しもそこへまとめるので、最初の出力で収束します。`header_first` はヘッダーと本文の間に留めます。それ以外の値は拒否されます

**例:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

### `to_dict()`

Python dict/list に変換します。エイリアス参照を解決し、ネイティブ Python タイプを返します。同じ解決は mapping の key にも等しく適用されます — `~: 1` の key は `None`、`1: a` の key は整数 1。同じテキストなら `:` のどちら側にあっても同じ型になります。

```python
to_dict() -> dict[Any, Any] | list[Any]
```

**戻り値:** 辞書またはリスト

**例:**

```python
doc = pyrs_yaml.parse("key: value")
data = doc.to_dict()  # {'key': 'value'}
```

### `get()`

トップレベルのマッピングキーによるリテラルキー検索で値を取得します（`__getitem__`/`__setitem__` と一貫）。キーは `.`、`[`、`]`、`$` を含めても常に文字どおりのキーとして扱われ、パスとして解釈されません。

```python
get(key: str, default: Any = None) -> Any
```

**戻り値:** 値、見つからない場合はデフォルト

!!! note "パスアクセス"
    JSONPath スタイルのアクセスには [`find()`](#find) / [`node()`](#node) /
    [`set()`](#set) を使ってください（`$.a.b`、`$.items[-1]` 等）。

### `root_type()`

ルートノードの型を文字列で取得します。

```python
root_type() -> str
```

**戻り値:** `"scalar"`、`"mapping"`、`"sequence"`、`"null"` のいずれか。

`"alias"` はバインディングがエイリアスルート用に携えている綴りですが、どの文書の読み取りでも
そこには届きません：エイリアスには同じ文書内のより前にそのアンカーが必要なので、ルートの
エイリアスはパーサーに拒否されます。`pyrs_yaml.parse("*x\n")` は
`YamlParseError: YAML parse error: while parsing node, found unknown anchor at char 0 line 1 column 1`
と答え、文書をまたぐ形 `"--- &a 1\n--- *a\n"` は 2 行目で同じように拒否されます。

**例:**

```python
print(doc.root_type())  # "mapping"
```

### `to_json()`

ドキュメントを JSON 文字列にシリアライズします。

```python
to_json(indent: int = 2) -> str
```

**戻り値:** JSON 文字列

### `validate()`

JSON Schema に基づいてドキュメントの内容を検証します。

```python
validate(schema: str | dict[str, Any]) -> None
```

**スロー:**

- `YamlValidateError` — ドキュメントがスキーマに適合しません。`str` でもマッピングでもない
  スキーマも同じくここに届き、エンジンの文をそのテキストにします：`doc.validate(3)` は
  `TypeError: argument of type 'int' is not iterable` と答えます

このクラスの外から来る二つの拒否があります。評価は Python の `jsonschema` パッケージへ
委譲され、本プロジェクトが公開するどの extra もそれをインストールしないためです：

- `jsonschema` が import できないと、`doc.validate({"type": "object"})` は裸の
  `ModuleNotFoundError: No module named 'jsonschema'` で答えます
- `str` のスキーマは評価器の前に `json.loads` を通るので、YAML の綴りのスキーマのテキストは
  `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` と答えます——
  `jsonschema` のインストールの有無にかかわらず同じです

両方とも再現とともに[例外 — YamlValidateError](exceptions.md#yamlvalidateerror)に記録しています。

### `flush_source()`

```python
flush_source() -> None
```

編集が発生したあと、AST を `source` へ遅延再シリアライズして現在のツリーを反映します。変更のたびにフル ダンプを強制しません。

### `reparse()`

保存されたソーステキストをその場で再パースし、スキーマやマージ動作の変更を可能にします。

```python
reparse(resolve_merges: bool = True, schema: str | None = None, honor_version_directive: bool | None = None) -> None
```

**パラメータ:**

- `resolve_merges` — `<<: *alias` マージキーを解決するかどうか (デフォルト: `True`)
- `schema` — 型解決スキーマ: `"core"`, `"json"`, `"failsafe"`, または `"yaml1.1"`。`None` は文書を読み込
  んだときのスキーマのままです。
- `honor_version_directive` — 宣言された `%YAML` のバージョン交渉。`True` なら `%YAML 1.1` を宣言する文
  書は 1.1 プロファイルで解決され、ディレクティブも再出力されます。`False` は宣言を無視して選んだ
  スキーマを保ちます。`None` は文書の現在の設定を保ちます。

**スロー:**

- `TypeError` — ソーステキストが保存されていない
- `YamlParseError` — 再パースに失敗

**例:**

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### `source()`

このドキュメントの作成に使用された元の YAML ソーステキストを返します。ドキュメントがその場で編集された場合、ソースは最初のアクセス時に現在のツリーから遅延して再シリアライズされます。

```python
source() -> str
```

**戻り値:** YAML 文字列。`parse()` で作成されていない場合（例: `from_dict()`）は空文字列。

**例:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.source())  # "key: value"
```

## 編集メソッド

!!! note "アトミック編集"
    pyrs-yaml の編集操作はすべてアトミックです。失敗した場合、ドキュメント（リビジョンを含む）は変更されません。

ドキュメントをその場で編集し、すべてのメタデータ（コメント、アンカー、タグ、スタイル）を保持します。編集は JSONPath スタイルのパス（`$.a.b`、`$.items[0]`）でノードを特定します。

### `set()`

パスで値を置き換えます。

```python
set(path: str, value: Any, create_missing: bool = False) -> None
```

- スカラー、`dict`、`list`、`tuple` をサポート — `tuple` は受け入れられ、`list` と同じくシーケンスとして書き出されます
- 既存のスカラーを置換する場合、対象のメタデータが保持されます；パスが存在しない場合はマッピングの末尾に新しいキーを追加
- 空のドキュメント（`""` からパース）にパスを設定すると、マッピングルートが自動的に作成されます
- `create_missing=True` を指定すると、不足している中間マッピングキーが自動的に作成されます

**スロー:**

- `YamlPathError` — 不正なパス（ワイルドカード/`..` は拒否されます）
- `YamlEditError` — 適用できないパスのステップで、メッセージがそれを名乗ります：エイリアスを
  経由するステップは `edit failed: cannot-edit-alias`、スカラーや形の違うコンテナに入る
  ステップは `create-needs-mapping`、`create_missing=True` がスカラーと出会う場合は
  `cannot-descend-into-scalar:<segment>`、書き込むキーより**前に**欠けているキーなら
  `missing-path:<key>`、シーケンスの向こうのインデックスなら `index-out-of-range-edit:<index>`。
  **最後の**キーの欠落は拒否ではなく——`parse("a: 1\n").set("$.zz", 3)` は `a: 1\nzz: 3\n` を
  出力します——生きているアンカーの書き込みも拒否ではありません：`base: &x 1\nuse: *x\n` での
  `set("$.base", 2)` は `base: &x 2\nuse: *x\n` を再出力し、`&x` をそのエイリアスがいまだ
  見つける位置に保ちます
- `YamlTypeError` — 書き手が知らない Python 型の値。`tuple` は受け入れられ、シーケンスとして
  書き出され、`bytes` は `!!binary` として受け入れられます

呼び出しごとに実測した完全な語彙は[編集理由トークン](exceptions.md#edit-reason-tokens)にあります。

**例:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)
doc.set("$.a.c", True)  # 新しいキーを追加
doc.set("$", {"x": 1})  # ルート全体を置換

# create_missing で不足キーを自動作成
doc.set("$.b.c.d", 2, create_missing=True)
```

### `walk()`

AST の深さ優先・先行順の走査で、`Node` オブジェクトを生成します。

```python
walk() -> Generator[Node, None, None]
```

`Node.walk()` とは異なり、このメソッドは**Rust バックエンド**です — AST を Python dict に変換せずに直接走査するため、大規模ドキュメントで大幅に高速です。

**生成するもの:** ルートを含む、ドキュメントツリー内のすべてのノードの `Node` オブジェクト。

**例:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
for node in doc.walk():
    print(node._path, node.root_type)
# ()       mapping
# ('a',)   mapping
# ('a', 'b') scalar
# ('a', 'c') scalar
```

### `scalars()`

`walk()` と同様ですが、スカラー/null ノードのみを生成します。

```python
scalars() -> Generator[Node, None, None]
```

**生成するもの:** ドキュメントツリー内のすべてのスカラーまたは null ノードの `Node` オブジェクト。

**例:**

```python
doc = pyrs_yaml.parse("a: hello\nb: null\n")
for node in doc.scalars():
    print(node._path, node.value)
# ('a',) hello
# ('b',) None
```

### `insert()`

シーケンスの指定インデックスに値を挿入します。

```python
insert(path: str, index: int, value: Any) -> None
```

`index` はシーケンスの現在の長さまで指定できます（`len` への挿入は追加と同等）。負のインデックスは末尾から数えます（`-1` は最後の要素の前に挿入）。パスはシーケンスノードに解決される必要があります。

**スロー:**

- `YamlEditError` — パスがマッピングまたはスカラーに解決されるなら `edit failed: not-a-sequence`、
  `len` より上か `-len` より下のインデックスなら `edit failed: index-out-of-range-edit`、解決しない
  パスなら `edit failed: missing-path:<key>`、エイリアスを経由するパスなら
  `edit failed: cannot-edit-alias`、スカラー値の下にあるパスなら
  `edit failed: cannot-descend-into-scalar:<segment>`

### `append()`

シーケンスの末尾に値を追加します。

```python
append(path: str, value: Any) -> None
```

**スロー:** `insert()` と同じ `YamlEditError` の理由で、シーケンスでない対象は
`edit failed: not-a-sequence`、エイリアスの親は `edit failed: cannot-edit-alias` と実測しました。

### `delete()`

パスでノードを削除します。マッピングの順序は保持されます。

```python
delete(path: str, on_duplicate: str = "one") -> None
```

**スロー:**

- `YamlEditError` — 文書が持たないキーなら `edit failed: missing-path`（`del doc["zz"]` もこれに
  答えます）、シーケンスの外側のインデックスなら `edit failed: index-out-of-range-edit`、ノードの
  アンカーがまだ参照されているなら `edit failed: anchor-is-referenced`、`on_duplicate="error"` で
  そのキーをつづる組が二つ以上あるなら `edit failed: duplicate-key-target`、それ以外の
  `on_duplicate` なら
  `edit failed: unknown on_duplicate "<value>"; expected one, all or error`、エイリアスを経由する
  パスなら `edit failed: cannot-edit-alias`、`delete("$")` なら `edit failed: edit-error`——
  ルートには取り除くセグメントが残っていません

### `rename()`

マッピングキーをその場でリネームします（位置とメタデータを保持）。

```python
rename(path: str, new_key: str) -> None
```

**スロー:**

- `YamlEditError` — `$` なら `edit failed: cannot-rename-root`、複合キーまたはシーケンス要素なら
  `edit failed: cannot-rename-complex-key`、マッピングがすでに持つ `new_key` なら
  `edit failed: rename-key-exists`、解決しないパスなら `edit failed: missing-path`、エイリアスを
  経由するパスなら `edit failed: cannot-edit-alias`

キーをそれ自身へリネームするのは拒否ではありません：`a: 1` で実測すると、`rename("$.a", "a")` は文書を `a: 1` のまま残します。

### `node()`

ドキュメントルートの `Node` を返します。

```python
node() -> Node
```

### `find()`

パスでノードを検索します。ワイルドカード（`[*]`）とディープスキャン（`..`）をサポート — その場合、ノードのリストを返します。

```python
find(path: str) -> Node | list[Node]
```

**スロー:**

- `YamlPathError` — パスが不正、または編集パスでワイルドカード/`..` を使用
- `YamlEditError` — 編集を適用できない（エイリアス経由の編集、ルート/複合キーのリネーム、スカラーへのナビゲーション、インデックス範囲外）
- `YamlTypeError` — 書き手が知らない Python 型の値 — `tuple` は**これに当たりません**（シーケンスとして書き出されます）
- `YamlDocumentError` — ドキュメント編集後に陳腐化した `Node` を使用

**参照:** [インプレース編集ガイド](../guides/editing.md)

**例:**

```python
doc = pyrs_yaml.parse("items: [1, 2, 3]")
doc.set("$.items[1]", "two")
doc.insert("$.items", 1, "x")  # items: [1, x, 2, 3]
doc.append("$.items", 4)
doc.rename("$.items", "list")  # マッピングキーをリネーム
del doc["list"]  # doc.delete("$.list") と同等
```

## ダンダー メソッド

### `__getitem__()`

キー（マッピング）またはインデックス（シーケンス）でアクセスします。

```python
doc = pyrs_yaml.parse("key: value")
value = doc["key"]  # 'value'
```

**スロー:**

- `KeyError` — マッピングにキーが見つかりません。実測したテキストは `Key 'zz' not found` で、
  これは組み込みの `KeyError` なので `str(exc)` は外側の引用符付きの
  `"Key 'zz' not found"` を見せます
- `IndexError` — シーケンスのインデックスが範囲外。
  `pyrs_yaml.parse("- a\n- b\n")[99]` で実測すると `Index 99 out of range (len=2)`
- `TypeError` — キーの種類が違う場合、またはそもそも添字にできないルートの
  `YamlTypeError` 節で、それぞれ実測しました：`parse("- a\n")["k"]` →
  `Index must be an integer`、`parse("a: 1\n")[99]` → `Key must be a string`、
  `parse("just a scalar")[0]` → `YamlDocument is not subscriptable`

上のテキストはすべて `pyrs_yaml.set_language()` に追従します——`zh-CN` の下で実測すると、
四つの節は `索引必须是整数`、`键必须是字符串`、`YamlDocument 不支持下标操作`、
`索引 99 超出范围（长度=2）` と答えます。[i18n サポート](exceptions.md#i18n-support) を
参照してください。

### `__setitem__()`

ルートマッピングキーを設定します（`doc.set()` のルート用糖衣構文）。

```python
doc["key"] = value
```

### `__delitem__()`

ルートマッピングキーを削除します（`doc.delete()` のルート用糖衣構文）。

```python
del doc["key"]
```

### `__contains__()`

キーが存在するか確認します。

```python
"key" in doc  # True
```

### `__len__()`

アイテム数を取得します。

```python
len(doc)
```

### `__iter__()`

キー（マッピング）または値（シーケンス）を反復します。

```python
for key in doc:
    print(key)
```

### `__repr__()`

デバッグ表現。

```python
repr(doc)  # "YamlDocument({key: value})"
```

### `__str__()`

文字列表現。

```python
str(doc)  # "YamlDocument({key: value})"
```

### `__eq__()`

等値比較。2つの `YamlDocument` が同じ内容を持つ場合、true を返します。

```python
doc1 == doc2  # True or False
```

## 使用例

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
name: Alice
age: 30
""")

print(doc.get("name"))  # Alice
print(doc.root_type())  # mapping
print(len(doc))  # 2
print("name" in doc)  # True
for key in doc:
    print(key, doc[key])  # name Alice, age 30
```

## 自動生成の API リファレンス

以下の内容は `mkdocstrings` がライブラリ自身の docstring と生成された型スタブから描画するため、署名はコード由来であり書き写しではありません。

::: pyrs_yaml.YamlDocument
