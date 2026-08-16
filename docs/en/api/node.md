---
title: Node Class
description: Reference for the Node class providing a borrowed view into a YamlDocument's AST for tree traversal, query, and mutation.
tags:
  - docs
status: new
---

## Node Class

The `Node` class provides a borrowed view into a `YamlDocument`'s AST, enabling tree traversal, query, and mutation operations. A node is created via `doc.node()`, `doc.find("$.path")`, or `doc.walk()`.

### Overview

```python
class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path."""
```

Each `Node` stores a reference to its parent `YamlDocument` and a path tuple that navigates to the target node within the document's AST. Nodes become stale when the document is modified or released.

### Constructor

#### `Node.__init__()`

```python
Node.__init__(document: YamlDocument, path: tuple = ()) -> None
```

**Parameters:**

- `document` — The parent `YamlDocument`
- `path` — A tuple of path segments (keys/indexes) navigating to the target node

### Properties

#### `value`

Get the scalar value of this node.

```python
value -> Any | None
```

Returns `None` for non-scalar nodes (mappings, sequences).

#### `root_type`

Get the type of this node.

```python
root_type -> str
```

Returns one of `"scalar"`, `"mapping"`, `"sequence"`, `"null"`.

#### `_path`

The path tuple that navigates to this node within the document's AST.

```python
_path -> tuple
```

#### `children`

Get the child nodes of this node.

```python
children -> list[Node]
```

Returns an empty list for scalar/null nodes.

#### `parent`

Get the parent `Node`, or `None` if this is the root.

```python
parent -> Node | None
```

#### `comment`

Get this node's comment text, or `None` if it has no comment.

```python
comment -> str | None
```

#### `anchor`

Get this node's anchor name, or `None` if it has no anchor.

```python
anchor -> str | None
```

#### `tag`

Get this node's YAML tag string (e.g. `!!str`), or `None` if it has no tag.

```python
tag -> str | None
```

#### `scalar_style`

Get the scalar style (`"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`), or `None` for non-scalar nodes.

```python
scalar_style -> str | None
```

#### `flow_style`

Get the flow style (`True` = flow `{}`/`[]`, `False` = block), or `None` for non-container nodes.

```python
flow_style -> bool | None
```

#### `chomping`

Get the chomping indicator (`"strip"`, `"clip"`, `"keep"`), or `None` for non-scalar nodes.

```python
chomping -> str | None
```

### Methods

#### `find()`

Find a node by JSONPath-like path.

```python
find(path: str) -> Node | list[Node]
```

**Supported path syntax:**

| Pattern | Description |
| --- | --- |
| `$.key` | Root key |
| `$.key.subkey` | Nested key |
| `$.arr[0]` | Index into sequence |
| `$.arr[*]` | All items in sequence |
| `$..key` | Deep search for key at any depth |
| `$..*` | All descendant nodes |

**Returns:** A single `Node` for exact paths, or a `list[Node]` for wildcard/deep-scan queries.

#### `walk()`

Walk all descendant nodes (depth-first pre-order).

```python
walk() -> Iterator[Node]
```

**Yields:** The node itself, then all descendants recursively.

#### `filter()`

Filter descendant nodes by a predicate function.

```python
filter(predicate: Callable[[Node], bool]) -> list[Node]
```

**Parameters:**

- `predicate` — A function taking a `Node` and returning `bool`

**Example:**

```python
scalars = root.filter(lambda n: n.root_type == "scalar")
```

#### `set_value()`

Replace this node's value, preserving its metadata (comment, anchor, tag, style).

```python
set_value(value: Any, create_missing: bool = False) -> None
```

With `create_missing=True`, missing intermediate mapping keys along the path are created as nested mappings. Index segments that miss are still an error.

#### `append()`

Append a value to a sequence node.

```python
append(value: Any) -> None
```

#### `insert()`

Insert into a sequence node at an index.

```python
insert(index: int, value: Any) -> None
```

#### `delete()`

Remove this node and its comments. The node becomes stale afterwards.

```python
delete() -> None
```

#### `rename()`

Rename this node's mapping key. The node must be a mapping value.

```python
rename(new_key: str) -> None
```

#### `set_comment()`

Set (or replace) this node's comment. With `standalone=True` (default) the
comment is emitted on its own line above the node; with `standalone=False`
it is emitted inline after the node.

```python
set_comment(text: str, standalone: bool = True) -> None
```

#### `remove_comment()`

Remove this node's comment.

```python
remove_comment() -> None
```

#### `set_anchor()`

Set (or replace) this node's anchor.

```python
set_anchor(name: str) -> None
```

#### `remove_anchor()`

Remove this node's anchor.

```python
remove_anchor() -> None
```

#### `set_tag()`

Set (or replace) this node's YAML tag. `"!custom"` produces a local tag,
`"!!int"` produces a primary (`!!`) tag, and `"!<tag:yaml.org,2002:str>"`
produces a verbatim tag.

```python
set_tag(tag: str) -> None
```

#### `remove_tag()`

Remove this node's YAML tag.

```python
remove_tag() -> None
```

#### `set_scalar_style()`

Set (or replace) this node's scalar style. No-op on non-scalar nodes. Recognized values: `"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`.

```python
set_scalar_style(style: str) -> None
```

#### `set_flow_style()`

Set (or replace) this node's flow style. `True` emits flow (`{}`/`[]`), `False` emits block. No-op on non-container nodes.

```python
set_flow_style(flow: bool) -> None
```

#### `set_chomping()`

Set (or replace) this node's chomping indicator. Recognized values: `"strip"` (`-`), `"clip"` (default), `"keep"` (`+`). No-op on non-scalar nodes.

```python
set_chomping(chomp: str) -> None
```

#### `to_yaml()`

Serialize this subtree to a YAML string.

```python
to_yaml() -> str
```

#### `is_valid()`

Check if the parent document is still alive and unmodified.

```python
is_valid() -> bool
```

#### `release()`

Release the reference to the parent document, marking this node as stale.

```python
release() -> None
```

After calling `release()`, any access to this node will emit a `RuntimeWarning` and raise `YamlDocumentError`.

### Dunder Methods

#### `__repr__()`

```python
__repr__() -> str
```

Returns `Node(root_type=<type>, path=<path>)` for valid nodes, `Node(released)` for released nodes, or `Node(invalid)` for stale nodes.

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

Two `Node` instances are equal if they share the same document, path, and alive state.

### Stale Node Behavior

!!! warning "Stale nodes"
    A `Node` is tied to the document's revision at creation time. Any document
    edit bumps the revision, so previously obtained nodes become stale. Always
    re-find a node after editing the document.

A node becomes stale when:

- The parent `YamlDocument` is garbage collected
- `release()` is called explicitly
- The document is modified after the node was created

Accessing a stale node emits a `RuntimeWarning` and raises `YamlDocumentError`:

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
