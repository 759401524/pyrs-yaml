# Features

pyyaml-rs is designed to be a **drop-in replacement** for PyYAML while adding powerful features that PyYAML lacks.

## YAML 1.2 Compliance

Powered by **saphyr-parser**, pyyaml-rs achieves **98.1% pass rate** on the YAML Test Suite.

## Perfect Round-Trip

Unlike PyYAML, pyyaml-rs **preserves all formatting and metadata**:

- **Comments** — standalone and inline
- **Anchors** (`&name`) and **aliases** (`*name`)
- **Tags** (`!!str`, `!!int`, etc.)
- **Chomping indicators** (`\|-`, `\|+`, `>-`, `>+`)
- **Scalar styles** (plain, single-quoted, double-quoted, literal, folded)
- **Flow/block formatting** — `[]`/`{}` vs block style preserved

## Performance

Rust backend delivers **25–40× speedup** over PyYAML:

| Operation | pyyaml-rs | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.08 ms | 2.96 ms |
| Round-trip | 0.08 ms | 2.98 ms |

## Custom AST

The **CustomNode** AST gives you full control over YAML structure:

- Inspect and modify nodes programmatically
- Add custom metadata (comments, anchors, tags)
- Build YAML from scratch with full formatting control
- Advanced use cases: template engines, config generators, code formatters

## PyYAML Compatibility

Drop-in replacement with familiar API:

```python
import pyyaml_rs as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

## Async I/O

Non-blocking serialization and parsing via `asyncio`:

```python
import asyncio
import pyyaml_rs

async def main():
    yaml = await pyyaml_rs.safe_dump_async({"a": 1})
    data = await pyyaml_rs.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

Available functions: `safe_dump_async`, `safe_load_async`, `safe_loads_async`.

## JSON Schema Validation

Validate parsed YAML documents against JSON Schema:

```python
doc = pyyaml_rs.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

Raises `YamlValidateError` on validation failure.

## Incremental Re-parse

Re-parse stored source text in place with different options:

```python
doc = pyyaml_rs.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

## NumPy ndarray Support

pyyaml-rs can serialize `numpy.ndarray` objects of any dimension directly to YAML:

```python
import numpy as np
import pyyaml_rs

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyyaml_rs.safe_dump(arr)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyyaml_rs.safe_dump(matrix)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip
loaded = pyyaml_rs.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### Supported dtypes

| Type | Rust Backend | YAML Output |
|------|-------------|-------------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | Plain integer (quoted if negative) |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | Plain integer |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | Plain float (quoted if negative) |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` string |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

### Notes

- **Zero-copy**: Uses the `numpy` Rust crate's `PyUntypedArray` for type-erased array access, then dispatches to the correct typed `PyArrayDyn<T>` for zero-copy slice iteration
- **GIL released**: Slice iteration runs outside the GIL for maximum performance on large arrays
- **Negative numbers**: YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative values are automatically quoted and correctly parsed back during round-trip
- **0-D arrays**: Reshaped to 1-D and serialized as a single-item list
- **Complex numbers**: YAML has no native complex type; serialized as `(re+imj)` strings. `safe_load` returns them as strings, not Python `complex`
- **Markdown frontmatter extraction** — `read_markdown()` for blog/content tools
- **JSON ↔ YAML conversion** — `from_json()` / `from_dict()`
- **Multi-document parsing** — `parse_all_docs()`
- **i18n error messages** — `set_language("zh-CN")` for bilingual errors
- **Type hints** — PEP 561 typed package marker (`py.typed`) for mypy support

## Supported YAML Constructs

| Feature | Support |
|---------|---------|
| YAML 1.2 spec | ✅ Full |
| Comments (standalone) | ✅ Preserved |
| Comments (inline) | ✅ Preserved |
| Anchors & aliases | ✅ Preserved |
| Tags (explicit) | ✅ Preserved |
| Block scalars (`\|`, `>`) | ✅ Preserved |
| Chomping indicators | ✅ Preserved |
| Flow collections (`{}`, `[]`) | ✅ Preserved |
| Merge keys (`<<`) | ✅ Resolved |
| Complex keys | ✅ Supported |
| Escape sequences | ✅ Supported |
| Multi-document | ✅ Supported |
| **Async I/O** | **✅ `safe_*_async`** |
| **JSON Schema validation** | **✅ `doc.validate()`** |
| **Incremental re-parse** | **✅ `doc.reparse()`** |
| **JSON export** | **✅ `doc.to_json()`** |
