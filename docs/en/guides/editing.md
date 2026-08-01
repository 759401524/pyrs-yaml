# In-Place Editing

pyrs-yaml lets you **edit a parsed document in place** while preserving all formatting metadata (comments, anchors, tags, scalar styles, flow/block style) — no manual string surgery, no fidelity loss.

## Overview

Edits are expressed as **JSONPath-style paths** into the document tree:

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

All edit methods are **atomic**: on failure nothing changes, including the document revision. On success the document is marked dirty, and the next `source()` / `to_yaml()` / `to_yaml_with_options()` / `reparse()` call re-serializes from the updated tree.

## Path Syntax

Paths start with `$` followed by dot-separated keys (mapping) or `[N]` indices (sequence):

| Path | Meaning |
|------|---------|
| `$.host` | Key `host` of the root mapping |
| `$.a.b.c` | Nested keys |
| `$.items[0]` | First element of sequence `items` |
| `$` | The root node itself |

- **Negative indices** (`[-1]`) are **not supported** — they raise an error
- Keys are matched **by value** (metadata-insensitive), so a quoted key `"host"` matches the plain key `host`

Editing paths must target exactly one node — **wildcards** (`[*]`) and **deep-scan** (`..`) raise `YamlPathError`. (Query-only `find()` does support them; see [Querying with `find()`](#querying-with-find).)

**Raises** `YamlPathError` for malformed paths, and `YamlEditError` when a path step cannot be applied (e.g. navigating into a scalar, or editing through an alias).

## Setting Values

### `set()` — replace by path

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

Value conversion rules:

| Python value | YAML node |
|--------------|-----------|
| `str`, `int`, `float`, `bool`, `None` | New scalar (value is *not* re-parsed) |
| `dict` | New mapping (plain style) |
| `list` | New sequence (plain style) |
| `tuple` | ❌ not supported — raises `YamlEditError` |

When replacing an existing scalar, the target's metadata (inline comment, anchor, tag, quoting style) is **preserved** — unless the new value is a mapping/sequence, which adopts the new node's own formatting.

### `__setitem__` — root sugar

```python
doc["b"] = 2  # equivalent to doc.set("$.b", 2)
```

### `Node.set_value()` — edit through a Node

```python
node = doc.node().find("$.a.b")  # see "Working with Nodes"
node.set_value(42)
```

## Inserting and Appending

Both operate on **sequences** only; the path must resolve to a sequence node.

### `insert()` — insert at an index

```python
insert(path: str, index: int, value: Any) -> None
```

`index` may be up to the current length (inserting at `len` appends); anything larger raises `YamlEditError`.

```python
doc = pyrs_yaml.parse("items:\n  - a\n  - c")

doc.insert("$.items", 1, "b")  # items: [a, b, c]
doc.insert("$.items", 0, "first")
doc.insert("$.items", 3, "last")  # index == len appends
```

### `append()` — add at the end

```python
append(path: str, value: Any) -> None
```

```python
doc.append("$.items", "d")
```

### `Node.append()` / `Node.insert()`

The same operations are available on `Node` objects:

```python
node = doc.node().find("$.items")
node.append("d")
node.insert(1, "x")
```

## Deleting

### `delete()` — remove by path

```python
delete(path: str) -> None
```

```python
doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
doc.delete("$.b")
print(doc.to_yaml())  # a: 1\nc: 3\n — order preserved
```

Mapping order is always preserved; sequence deletion closes the gap.

### `__delitem__` — root sugar

```python
del doc["b"]  # equivalent to doc.delete("$.b")
```

### `Node.delete()`

```python
node = doc.node().find("$.b")
node.delete()
```

## Renaming

### `rename()` — rename a mapping key in place

```python
rename(path: str, new_key: str) -> None
```

The path must point at a **mapping key** (the value lives under it and keeps its metadata):

```python
doc = pyrs_yaml.parse("old: value  # keep me\nnext: 1")
doc.rename("$.old", "new")
print(doc.to_yaml())  # new: value  # keep me\nnext: 1
```

- **Position is preserved** — the renamed key stays in place
- **Metadata is preserved** — the key's inline comment, style, and anchor travel with the rename
- Renaming the root, a complex (non-scalar) key, or onto an **existing key** raises `YamlEditError` (renaming a key to itself is a no-op)

### `Node.rename()`

```python
node = doc.node().find("$.old")
node.rename("new")
```

## Working with Nodes

`doc.node()` returns a `Node` for the document root; `Node.find(path)` navigates to a subtree:

```python
node = doc.node()  # root node
node = doc.node().find("$.db.host")  # navigate by path
print(node.value)  # "localhost"
node.set_value("other")  # edit through the node
print(node.root_type)  # "scalar" | "mapping" | "sequence" | "null"
```

Nodes expose a tree API: `node.parent`, `node.children`, `node.walk()` (depth-first iterator), `node.filter(predicate)`, and `node.to_yaml()`.

### Querying with `find()`

`find()` is **read-oriented** and supports wildcards and deep scans — it returns a list when the path selects multiple nodes:

```python
doc.node().find("$.items[*]")  # all items of a sequence (list of Nodes)
doc.node().find("$..timeout")  # deep search for any key named "timeout"
```

Wildcard/deep-scan results are **not directly editable** — use them to locate paths, then edit with `set()`/`insert()`/etc.

## Aliases and Merge Keys

An alias node (`*name`) is replaced **in place** when its own path is set:

```python
yaml = "defaults: &defaults\n  timeout: 30\nprod: *defaults\n"
doc = pyrs_yaml.YAML(typ="safe").parse(yaml)  # resolve_merges=false keeps the alias node

doc.set("$.prod", {"timeout": 99})  # replaces the alias node — prod.timeout: 99
```

- Setting **through** an alias (navigating through `*defaults` to reach a merged key) raises `YamlEditError` — the referenced node lives elsewhere
- With merge keys resolved (default), merge-expanded keys are clones; editing them edits only the clone
- Deleting an anchored node is tolerated (the anchor simply stops being referenced)

## View vs. AST

`doc.get()` / `doc.to_dict()` return the **view** (resolved values). Editing always operates on the **AST**:

```python
doc = pyrs_yaml.parse("on: yes")
print(doc.get("on"))  # True   — view (core schema resolution)
doc.set("$.on", "off")  #         — edits the AST scalar
print(doc.to_yaml())  # on: off — serialized verbatim, no re-resolution
```

The edited value is emitted **as-is**; the view resolves it according to the active schema.

## Stale Nodes

A `Node` is tied to the document's **revision**, recorded when the node was created. Any document edit (even through a different node) bumps the revision, so previously obtained nodes become stale:

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # bumps the revision
node.set_value(99)  # RuntimeWarning + YamlDocumentError (stale)
```

Re-find the node after any edit to continue working. `node.is_valid()` checks liveness; `node.release()` detaches a node from its document explicitly.

## Error Handling

| Error | When |
|-------|------|
| `YamlPathError` | Malformed path, wildcard/`..` used in an edit path |
| `YamlEditError` | Unsupported value type (`tuple`), negative index, edit through alias, rename of root/complex/existing key, navigation into a scalar, index out of bounds |
| `YamlDocumentError` | Stale `Node` used after a document edit |

All edits are atomic — a failed edit leaves the document (and its revision) untouched.

## Full Example

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

Comments, anchors, tags, scalar styles, and flow/block formatting are preserved throughout.
