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

Reading goes through the document's dict projection, and that is where it can refuse:
`KeyError` for a path the document does not hold, `YamlTypeError` when two distinct YAML
keys project to the same Python key. A projection refusal belongs to the document rather
than to the path — on such a document every `Node` read raises it, whichever path was used.
Writing does not read the projection: `set_value()` walks the AST, so on a colliding name
it lands on the first pair the reader folds to that name.

#### `root_type`

Get the type of this node.

```python
root_type -> str
```

Returns one of `"scalar"`, `"mapping"`, `"sequence"`, `"null"`.

An alias node reports its **target's** type, never a spelling of its own: measured on
`defaults: &d\n  timeout: 30\nprod: *d\n` and `src: &s\n  - a\nref: *s\n`, the nodes at `$.prod`
and `$.ref` answer `"mapping"` and `"sequence"`, and the scalar alias at `$.use` in
`base: &x 1\nuse: *x\n` answers `"scalar"`.

#### `_path`

The path tuple that navigates to this node within the document's AST.

```python
_path -> tuple
```

#### `path`

Get this node's path segments (tuple of keys and indices). Unlike `_path`,
this is the documented public accessor — useful for logging, error messages,
and re-finding a node after it becomes stale.

```python
path -> tuple
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

#### `find_first()`

Find the first matching node by JSONPath-like path. Always returns a single
`Node` or `None` (never a list) — for wildcard paths (`[*]`), returns the first
element.

```python
find_first(path: str) -> Node | None
```

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

Measured edges:

- A node `find()` could not resolve — the `Node(invalid)` object — still writes: `doc.node().find("$.zz").set_value(3)` adds `zz: 3`. Reading its `value` is the refusal (`KeyError`), not the write
- On an **alias** node the call is not a refusal either — it rewrites the alias in place: `parse("base: &x 1\nuse: *x\n").node().find("$.use").set_value(42)` emits `base: &x 1\nuse: 42\n`
- Writing **through** an alias is refused: `edit failed: cannot-edit-alias`
- On a stale node it emits a `RuntimeWarning` and raises `YamlDocumentError: document has been modified; re-find the node`
- A Python value the writer does not know is a `YamlTypeError` (`Unsupported Python type for YAML conversion`), not an edit error

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
delete(on_duplicate: str = "one") -> None
```

Measured refusals: the root node answers `YamlEditError: edit failed: edit-error`, a node whose
anchor an alias still references answers `edit failed: anchor-is-referenced`, and
`on_duplicate="error"` on a document where two pairs spell the key answers
`edit failed: duplicate-key-target`. Deleting an **alias** pair is not a refusal —
`find("$.prod").delete()` on `defaults: &d\n  timeout: 30\nprod: *d\n` leaves
`defaults: &d\n  timeout: 30\n`. The stale node answers `YamlDocumentError` on the next access but
`is_valid()` still answers `True`.

#### `rename()`

Rename this node's mapping key. The node must be a mapping value.

```python
rename(new_key: str) -> None
```

Measured refusals: the root answers `YamlEditError: edit failed: cannot-rename-root`, a sequence
element — which has no key — answers `edit failed: cannot-rename-complex-key`, a `new_key` the
mapping already holds answers `edit failed: rename-key-exists`, and a node whose key is a complex
node never reaches the engine: `parse("? [1, 2]\n: value\n").node().children[0].rename("b")` answers
`YamlEditError: ValueError: segment must be str or int`. Renaming to the same key is a no-op, and on
an alias pair the call renames the key rather than refusing — `find("$.prod").rename("prod2")` on
`defaults: &d\n  timeout: 30\nprod: *d\n` leaves `defaults: &d\n  timeout: 30\nprod2: *d\n`.

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

Refused with `YamlEditError: edit failed: anchor-is-referenced` while an alias still points at the
current anchor — measured on `base: &x 1\nuse: *x\n`, where `find("$.base").set_anchor("renamed")`
fails and, with the alias gone, the same call on `base: &x 1\n` emits `base: &renamed 1`. On an
alias node itself it answers `edit failed: cannot-edit-alias`.

#### `remove_anchor()`

Remove this node's anchor.

```python
remove_anchor() -> None
```

Same guard as `set_anchor()`: measured on `base: &x 1\nuse: *x\n`, `find("$.base").remove_anchor()`
answers `YamlEditError: edit failed: anchor-is-referenced`.

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

#### `sort_keys()`

Sort the keys of this mapping node in place. No-op on non-mapping nodes.

```python
sort_keys() -> None
```

#### `move()`

Move this subtree to a new path in the same document (copies the subtree to
`new_path`, then removes the source). `new_path` is an absolute JSONPath.

```python
move(new_path: str) -> None
```

Measured edges: a destination whose parent exists need not exist itself — `find("$.src").move("$.nope")`
on `src:\n  x: 1\n` leaves `nope:\n  x: 1\n` — while a destination with a missing intermediate answers
`YamlEditError: edit failed: missing-path:deep`. Replacing a scalar destination works: `dst: 2` becomes
`dst:\n  x: 1\n`. Moving the root answers `edit failed: edit-error`, moving a subtree onto its own path
empties the document (`find("$.src").move("$.src")` leaves `{}\n`), and a destination whose value still
carries a referencing alias answers `edit failed: anchor-is-referenced`.

#### `to_yaml()`

Serialize this subtree to a YAML string.

```python
to_yaml() -> str
```

#### `copy()`

Deep-copy this subtree as a standalone Python value (dict/list/scalar),
detached from the document. Useful for duplicating a subtree to paste
elsewhere via `set_value()`.

```python
copy() -> Any
```

#### `is_valid()`

Check if the parent document is still alive.

```python
is_valid() -> bool
```

This answers a question about the document, not about the path. A node whose path
resolves to nothing — an absent key, a name two YAML keys share, or one made stale by a
later edit — is still valid, so `is_valid()` is not a way to test whether `value` will
answer. `__repr__` carries that signal instead.

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

Returns `Node(root_type=<type>, path=<path>)` when the path resolves, `Node(released)`
once `release()` has run, or `Node(invalid)` when the path does not resolve — because the
document was modified, because it names nothing the document holds, or because the
projection refuses (see `value`).

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

Two `Node` instances are equal if they share the same document, path, and alive state.

#### `value_eq()`

Compare this node's resolved value with another node or Python value. Unlike
`__eq__` (reference identity), this compares the actual YAML value.

```python
value_eq(other: object) -> bool
```

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

```python title="Stale node example"
>>> node = doc.node()
>>> doc.set("$.key", "new_value")
>>> node.value  # (1)!
RuntimeWarning: Node is stale: the document was modified after this node was created
YamlDocumentError: document has been modified; re-find the node  # (2)!
```

1. :material-alert: Accessing a stale node triggers a `RuntimeWarning` first.
2. :material-close: Then raises `YamlDocumentError`. Re-acquire the node via `doc.node().find(path)`.

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

## Generated API reference

Rendered by `mkdocstrings` from the library's own docstrings and generated type stub, so the signatures below come out of the code rather than being retyped.

::: pyrs_yaml.Node
