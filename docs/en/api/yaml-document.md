---
title: YamlDocument Class
description: Reference for the YamlDocument class with full metadata preservation, editing methods, and dunder methods.
tags:
  - docs
status: new
---

## YamlDocument Class

The `YamlDocument` class represents a parsed YAML document with full metadata preservation.

### Overview

```python
class YamlDocument:
    """A parsed YAML document with perfect round-trip support."""
```

### Methods

#### `to_yaml()`

Convert the document back to a YAML string.

```python
to_yaml() -> str
```

**Returns:** The complete YAML document string, ending with a newline.

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n
```

#### `to_yaml_with_options()`

Convert to YAML with custom options.

```python
to_yaml_with_options(
    indent_size: int = 2,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
    note_placement: str = "note_first",
) -> str
```

**Parameters:**

- `indent_size` — Spaces per indent level (default: 2)
- `explicit_start` — Add `---` at document start (default: False)
- `explicit_end` — Add `...` at document end (default: False)
- `sort_keys` — Sort keys alphabetically (default: False)
- `note_placement` — Where a note goes when it cannot ride its line and an `&a` / `!tag` header line sits above it: `note_first` (default) writes it above the header, where a read files it, so the first emission settles; `header_first` keeps it inside the gap. Any other value is refused

**Example:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

#### `to_dict()`

Convert to a Python dict/list, resolving alias references.

```python
to_dict() -> dict[Any, Any] | list[Any]
```

**Returns:** Native Python types. Anchors (`&name`) are inlined, aliases (`*name`) are replaced with actual values. Scalars are converted to Python native types (bool/int/float/str/None). The same resolution applies to mapping keys, so `~: 1` loads with a `None` key and `1: a` with the integer 1 - one text, one type, whichever side of the `:` it sits on.

**Example:**

```python
data = doc.to_dict()
print(data["key"])  # value
print(type(data))  # <class 'dict'>
```

#### `get()`

Get a value by top-level mapping key (literal key lookup, consistent with
`__getitem__`/`__setitem__`).

```python
get(key: str, default: Any = None) -> Any
```

**Parameters:**

- `key` — The literal mapping key to look up. Keys may legally contain `.`,
  `[`, `]` or `$`; they are never interpreted as paths.
- `default` — Value to return if the key is not found (default: None)

**Returns:** The value, or `default` if not found (or if root is not a mapping).

!!! note "Path access"
    For JSONPath-style access, use [`find()`](#find) / [`node()`](#node) /
    [`set()`](#set) instead — they resolve paths such as `$.a.b` and
    `$.items[-1]`.

**Example:**

```python
value = doc.get("key")
value = doc.get("a.b")  # literal dotted key
value = doc.get("missing", "fallback")
```

#### `root_type()`

Get the root node type as a string.

```python
root_type() -> str
```

**Returns:** One of `"scalar"`, `"mapping"`, `"sequence"`, `"null"`.

`"alias"` is a spelling the binding carries for an alias root, but no document read reaches it: an
alias needs its anchor earlier in the same document, so a root alias is refused by the parser.
`pyrs_yaml.parse("*x\n")` answers `YamlParseError: YAML parse error: while parsing node, found
unknown anchor at char 0 line 1 column 1`, and the cross-document form `"--- &a 1\n--- *a\n"` is
refused the same way at line 2.

**Example:**

```python
print(doc.root_type())  # "mapping"
```

#### `to_json()`

Serialize the document to a JSON string.

```python
to_json(indent: int = 2) -> str
```

**Parameters:**

- `indent` — JSON indentation spaces (default: 2)

**Returns:** A JSON string of the document contents.

**Example:**

```python
doc = pyrs_yaml.parse("a: 1\nb: hello")
json_str = doc.to_json()  # '{"a": 1, "b": "hello"}'
```

#### `validate()`

Validate the document contents against a JSON Schema.

```python
validate(schema: str | dict[str, Any]) -> None
```

**Parameters:**

- `schema` — JSON Schema as either a JSON string or a Python dict

**Returns:** `None` on success.

**Raises:**

- `YamlValidateError` — Document does not conform to the schema. A schema that is neither `str` nor a mapping arrives here too, with the engine's sentence as its text: `doc.validate(3)` answers `TypeError: argument of type 'int' is not iterable`

Two refusals come from outside this class, because evaluation is delegated to the Python
`jsonschema` package and no extra this project publishes installs it:

- With `jsonschema` unimportable, `doc.validate({"type": "object"})` answers a bare `ModuleNotFoundError: No module named 'jsonschema'`
- A `str` schema runs `json.loads` before the evaluator, so a YAML-spelled schema text answers `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` — the same whether or not `jsonschema` is installed

Both are recorded with their reproduction in [Exceptions — YamlValidateError](exceptions.md#yamlvalidateerror).

**Example:**

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# From JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

#### `source()`

Return the original YAML source text used to create this document. If the document has been edited in place, the source is lazily re-serialized from the current tree on first access.

```python
source() -> str
```

**Returns:** The YAML string. Empty string if the document was not created via `parse()` (e.g. from `from_dict()`).

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.source())  # "key: value"
```

#### `flush_source()`

```python
flush_source() -> None
```

Lazily re-serializes the AST into `source` when edits have occurred, so `source` reflects the current tree without a full dump after every mutation.

#### `reparse()`

Re-parse the stored source text in place, allowing schema or merge behavior changes.

```python
reparse(resolve_merges: bool = True, schema: str | None = None, honor_version_directive: bool | None = None) -> None
```

**Parameters:**

- `resolve_merges` — Whether to resolve `<<: *alias` merge keys (default: `True`)
- `schema` — Type resolution schema: `"core"`, `"json"`, `"failsafe"`, or `"yaml1.1"`. `None`
  keeps the schema the document was read with.
- `honor_version_directive` — Version negotiation for a declared `%YAML`. `True` resolves a
  document declaring `%YAML 1.1` under the 1.1 profile and re-emits the directive; `False` ignores
  the declaration and keeps the selected schema. `None` keeps the document's current setting.

**Raises:**

- `TypeError` — No source text stored
- `YamlParseError` — Re-parsing failed

**Example:**

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### Editing Methods

!!! note "Atomic edits"
    All edits are atomic — a failed edit leaves the document (and its
    revision) untouched. On success the stored source is marked dirty and
    the next `source()` / `to_yaml()` / `to_yaml_with_options()` / `reparse()`
    call re-serializes from the updated tree. See the
    [In-Place Editing guide](../guides/editing.md) for the full path syntax
    and semantics.

#### `set()`

Set a value by JSONPath path.

```python
set(path: str, value: Any, create_missing: bool = False) -> None
```

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)  # replace existing
doc.set("$.a.c", True)  # create new key
doc.set("$", {"x": 1})  # replace the root

empty = pyrs_yaml.parse("")
empty.set("$.a", 1)  # auto-creates a mapping root: {a: 1}
```

**Raises:**

- `YamlPathError` — Malformed path (wildcards/`..` are rejected)
- `YamlEditError` — A path step that cannot be applied, named in the message: `edit failed: cannot-edit-alias` for a step through an alias, `create-needs-mapping` for a step into a scalar or onto the wrong container shape, `cannot-descend-into-scalar:<segment>` when `create_missing=True` meets a scalar, `missing-path:<key>` for a key that is missing **before** the one being written, `index-out-of-range-edit:<index>` for an index past the sequence. A missing **final** key is not a refusal — `parse("a: 1\n").set("$.zz", 3)` emits `a: 1\nzz: 3\n` — and neither is writing the source of a live anchor: `set("$.base", 2)` on `base: &x 1\nuse: *x\n` re-emits `base: &x 2\nuse: *x\n`, keeping `&x` where its alias still finds it
- `YamlTypeError` — Value of a Python type the writer does not know; a `tuple` is accepted and written as a sequence, and `bytes` is accepted as `!!binary`

The full measured vocabulary, call by call, is [Edit reason tokens](exceptions.md#edit-reason-tokens).

#### `walk()`

Depth-first, pre-order traversal of the AST, yielding `Node` objects.

```python
walk() -> Generator[Node, None, None]
```

Unlike `Node.walk()`, this method is **Rust-backed** — it traverses the AST directly without converting to Python dicts, making it significantly faster for large documents.

**Yields:** `Node` objects for every node in the document tree, including the root.

**Example:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
for node in doc.walk():
    print(node._path, node.root_type)
# ()       mapping
# ('a',)   mapping
# ('a', 'b') scalar
# ('a', 'c') scalar
```

#### `scalars()`

Like `walk()`, but yields only scalar/null nodes.

```python
scalars() -> Generator[Node, None, None]
```

**Yields:** `Node` objects for every scalar or null node in the document tree.

**Example:**

```python
doc = pyrs_yaml.parse("a: hello\nb: null\n")
for node in doc.scalars():
    print(node._path, node.value)
# ('a',) hello
# ('b',) None
```

#### `insert()`

Insert a value into a sequence at an index. The path must resolve to a sequence. Negative indexes count from the end (`-1` inserts before the last element).

```python
insert(path: str, index: int, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("items: [a, c]")
doc.insert("$.items", 1, "b")  # items: [a, b, c]
doc.insert("$.items", -1, "x")  # items: [a, b, x, c]
```

**Raises:**

- `YamlEditError` — `edit failed: not-a-sequence` when the path resolves to a mapping or a scalar, `edit failed: index-out-of-range-edit` for an index above `len` or below `-len`, `edit failed: missing-path:<key>` for a path that does not resolve, `edit failed: cannot-edit-alias` for a path through an alias, `edit failed: cannot-descend-into-scalar:<segment>` for a path under a scalar value

#### `append()`

Append a value to a sequence. The path must resolve to a sequence.

```python
append(path: str, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("items: [a, b]")
doc.append("$.items", "c")
```

**Raises:** the same `YamlEditError` reasons as `insert()`, measured for a non-sequence target as `edit failed: not-a-sequence` and for an alias parent as `edit failed: cannot-edit-alias`.

#### `delete()`

Delete the node at a path, preserving mapping/sequence order.

```python
delete(path: str, on_duplicate: str = "one") -> None
```

```python
doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
doc.delete("$.b")
# a: 1
# c: 3
```

**Raises:**

- `YamlEditError` — `edit failed: missing-path` for a key the document does not hold (this is what `del doc["zz"]` answers too), `edit failed: index-out-of-range-edit` for an index outside the sequence, `edit failed: anchor-is-referenced` when the node's anchor is still referenced, `edit failed: duplicate-key-target` when `on_duplicate="error"` and more than one pair spells the key, `edit failed: unknown on_duplicate "<value>"; expected one, all or error` for any other `on_duplicate`, `edit failed: cannot-edit-alias` for a path through an alias, and `edit failed: edit-error` for `delete("$")` — the root has no segment left to remove

#### `rename()`

Rename a mapping key in place, preserving position and key metadata.

```python
rename(path: str, new_key: str) -> None
```

```python
doc = pyrs_yaml.parse("old: 1  # comment")
doc.rename("$.old", "new")
# new: 1  # comment
```

**Raises:**

- `YamlEditError` — `edit failed: cannot-rename-root` for `$`, `edit failed: cannot-rename-complex-key` for a complex key or a sequence element, `edit failed: rename-key-exists` for a `new_key` the mapping already holds, `edit failed: missing-path` for a path that does not resolve, `edit failed: cannot-edit-alias` for a path through an alias

Renaming a key to itself is not a refusal: measured on `a: 1`, `rename("$.a", "a")` leaves the
document as `a: 1`.

#### `node()`

Get the root `Node` of the document for tree navigation and editing.

```python
node() -> Node
```

```python
node = doc.node().find("$.a.b")
node.set_value(42)
```

#### `find()`

Query the document by JSONPath-like path. Supports wildcards (`[*]`) and deep-scan (`..`), returning a list when multiple nodes match.

```python
find(path: str) -> Node | list[Node]
```

#### `__setitem__()` / `__delitem__()` — root sugar

```python
doc["key"] = value  # equivalent to doc.set("$.key", value)
del doc["key"]  # equivalent to doc.delete("$.key")
```

### Dunder Methods

#### `__getitem__()`

Access by key (mapping) or index (sequence).

```python
doc["key"]  # For mappings
doc[0]  # For sequences
```

**Raises:**

- `KeyError` — Key not found in mapping. Measured text is `Key 'zz' not found`, and because this is the builtin `KeyError`, `str(exc)` shows it quoted: `"Key 'zz' not found"`
- `IndexError` — Index out of range for sequence. Measured on `pyrs_yaml.parse("- a\n- b\n")[99]`: `Index 99 out of range (len=2)`
- `TypeError` — the `YamlTypeError` arm for the wrong key kind or a root that cannot be subscripted at all, each measured: `parse("- a\n")["k"]` → `Index must be an integer`, `parse("a: 1\n")[99]` → `Key must be a string`, `parse("just a scalar")[0]` → `YamlDocument is not subscriptable`

Every text above follows `pyrs_yaml.set_language()` — measured under `zh-CN`, which answers
`索引必须是整数`, `键必须是字符串`, `YamlDocument 不支持下标操作` and
`索引 99 超出范围（长度=2）` for the four arms. See [i18n Support](exceptions.md#i18n-support).

#### `__contains__()`

Check if a key exists.

```python
"key" in doc  # Returns bool
```

#### `__len__()`

Get the number of items.

```python
len(doc)  # Number of keys (mapping) or items (sequence)
```

#### `__iter__()`

Iterate over keys (mapping) or values (sequence).

```python
for key in doc:
    print(key, doc[key])
```

#### `__repr__()`

Debug representation.

```python
repr(doc)  # "YamlDocument(<yaml>...)"
```

#### `__str__()`

String representation.

```python
str(doc)  # Same as doc.to_yaml()
```

### Example

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

## Generated API reference

Rendered by `mkdocstrings` from the library's own docstrings and generated type stub, so the signatures below come out of the code rather than being retyped.

::: pyrs_yaml.YamlDocument
