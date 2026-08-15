---
title: Node クラス
description: Node クラスの API リファレンス。プロパティ、メソッド、期限切れノードの動作をカバーします。
tags:
  - docs
status: new
---

## Node クラス

`Node` クラスは、`YamlDocument` の AST への借用ビューを提供し、ツリーの走査、クエリ、および変更操作を可能にします。ノードは `doc.node()`、`doc.find("$.path")`、または `doc.walk()` で作成されます。

### Overview

```python
class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path."""
```

各 `Node` は、親 `YamlDocument` への参照と、ドキュメントの AST 内のターゲットノードに移動するパス タプルを保持します。ノードは、ドキュメントが変更または解放されると期限切れになります。

### Constructor

#### `Node.__init__()`

```python
Node.__init__(document: YamlDocument, path: tuple = ()) -> None
```

**Parameters:**

- `document` — 親 `YamlDocument`
- `path` — ターゲットノードに移動するパスセグメント（キー/インデックス）のタプル

### Properties

#### `value`

このノードのスカラー値を取得します。

```python
value -> Any | None
```

非スカラーノード（マッピング、シーケンス）の場合は `None` を返します。

#### `root_type`

このノードの型を取得します。

```python
root_type -> str
```

`"scalar"`、`"mapping"`、`"sequence"`、`"null"` のいずれかを返します。

#### `_path`

このノードをドキュメントの AST 内に位置付けるパス タプルです。

```python
_path -> tuple
```

#### `children`

このノードの子ノードを取得します。

```python
children -> list[Node]
```

スカラー/Null ノードの場合は空のリストを返します。

#### `parent`

親 `Node` を取得します。ルートの場合は `None` を返します。

```python
parent -> Node | None
```

#### `comment`

このノードのコメントテキストを取得します。コメントがない場合は `None` を返します。

```python
comment -> str | None
```

#### `anchor`

このノードのアンカー名を取得します。アンカーがない場合は `None` を返します。

```python
anchor -> str | None
```

#### `tag`

このノードの YAML タグ文字列（例: `!!str`）を取得します。タグがない場合は `None` を返します。

```python
tag -> str | None
```

### Methods

#### `find()`

JSONPath ライクなパスでノードを検索します。

```python
find(path: str) -> Node | list[Node]
```

**対応するパス構文:**

| Pattern | Description |
| --- | --- |
| `$.key` | ルートキー |
| `$.key.subkey` | ネストされたキー |
| `$.arr[0]` | シーケンスへのインデックス |
| `$.arr[*]` | シーケンス内のすべてのアイテム |
| `$..key` | 任意の深さでキーを検索 |
| `$..*` | すべての子孫ノード |

**戻り値:** 厳密なパスには単一の `Node`、ワイルドカード/深層検索クエリには `list[Node]`。

#### `walk()`

すべての子孫ノードを走査します（深さ優先の先行順）。

```python
walk() -> Iterator[Node]
```

**生成:** ノード自体、次にすべての子孫を再帰的に生成します。

#### `filter()`

述語関数で子孫ノードをフィルタリングします。

```python
filter(predicate: Callable[[Node], bool]) -> list[Node]
```

**Parameters:**

- `predicate` — `Node` を受け取り `bool` を返す関数

**Example:**

```python
scalars = root.filter(lambda n: n.root_type == "scalar")
```

#### `set_value()`

このノードの値を置き換え、メタデータ（コメント、アンカー、タグ、スタイル）を保持します。

```python
set_value(value: Any, create_missing: bool = False) -> None
```

`create_missing=True` の場合、パスに沿った欠落している中間マッピングキーがネストされたマッピングとして作成されます。インデックスセグメントが欠落している場合は引き続きエラーになります。

#### `append()`

シーケンスノードに値を追加します。

```python
append(value: Any) -> None
```

#### `insert()`

シーケンスノードのインデックスに挿入します。

```python
insert(index: int, value: Any) -> None
```

#### `delete()`

このノードとそのコメントを削除します。その後、ノードは期限切れになります。

```python
delete() -> None
```

#### `rename()`

このノードのマッピングキーの名前を変更します。ノードはマッピングの値である必要があります。

```python
rename(new_key: str) -> None
```

#### `set_comment()`

このノードのコメントを設定（または置換）します。`standalone=True`（デフォルト）ではコメントがノードの上の独立した行に出力され、`standalone=False` ではノードの後ろにインラインで出力されます。

```python
set_comment(text: str, standalone: bool = True) -> None
```

#### `remove_comment()`

このノードのコメントを削除します。

```python
remove_comment() -> None
```

#### `set_anchor()`

このノードのアンカーを設定（または置換）します。

```python
set_anchor(name: str) -> None
```

#### `remove_anchor()`

このノードのアンカーを削除します。

```python
remove_anchor() -> None
```

#### `set_tag()`

このノードの YAML タグを設定（または置換）します。`"!custom"` はローカルタグ、`"!!int"` はプライマリ（`!!`）タグになります。

```python
set_tag(tag: str) -> None
```

#### `remove_tag()`

このノードの YAML タグを削除します。

```python
remove_tag() -> None
```

#### `to_yaml()`

このサブツリーを YAML 文字列にシリアライズします。

```python
to_yaml() -> str
```

#### `is_valid()`

親ドキュメントがまだ有効で変更されていないかどうかを確認します。

```python
is_valid() -> bool
```

#### `release()`

親ドキュメントへの参照を解放し、このノードを期限切れとしてマークします。

```python
release() -> None
```

`release()` を呼び出した後、このノードへのアクセスは `RuntimeWarning` を発行し、`YamlDocumentError` を発生させます。

### Dunder Methods

#### `__repr__()`

```python
__repr__() -> str
```

有効なノードの場合は `Node(root_type=<type>, path=<path>)`、解放されたノードの場合は `Node(released)`、期限切れのノードの場合は `Node(invalid)` を返します。

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

2 つの `Node` インスタンスは、同じドキュメント、パス、および有効状態を共有する場合に等しいとみなされます。

### Stale Node Behavior

!!! warning "期限切れノード"
    `Node` はドキュメントのリビジョンに結び付けられています。ドキュメントが編集されるとリビジョンが増加し、以前に取得したノードは期限切れになり、`YamlDocumentError` をスローします。編集後はノードを再取得してください。

ノードは次の場合に期限切れになります:

- 親 `YamlDocument` がガベージコレクションされた
- `release()` が明示的に呼び出された
- ノード作成後にドキュメントが変更された

期限切れのノードにアクセスすると、`RuntimeWarning` が発行され、`YamlDocumentError` が発生します:

```python
>>> node = doc.node()
>>> doc.set("$.key", "new_value")
>>> node.value
RuntimeWarning: Node is stale: the document was modified after this node was created
YamlDocumentError: document has been modified; re-find the node
```

### Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
a:
  b: 1
  c: [2, 3, 4]
d: hello
""")

# Get root node
root = doc.node()
print(root.root_type)  # "mapping"

# Navigate
node = root.find("$.a.c[1]")
print(node.value)  # 3

# Walk
for n in root.walk():
    print(n._path, n.root_type)

# Filter
numbers = root.filter(lambda n: n.root_type == "scalar" and isinstance(n.value, int))
for n in numbers:
    print(n._path, n.value)  # ('a', 'b') 1, ('a', 'c', 0) 2, ...

# Mutate
root.find("$.a.b").set_value(42)
root.find("$.a.c").append(5)
root.find("$.d").rename("greeting")
root.find("$.a.c[0]").delete()

print(doc.to_yaml())
# a:
#   b: 42
#   c: [3, 4, 5]
# greeting: hello
```
