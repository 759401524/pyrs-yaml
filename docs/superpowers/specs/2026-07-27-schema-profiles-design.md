# Design: YAML Schema Resolution Profiles

## Overview

Add configurable schema resolution to `parse()` / `safe_load()` etc. that controls how implicit (unquoted) scalar values are typed. Three built-in schemas per the YAML 1.2 spec: `failsafe`, `json`, `core`. Plus `yaml1.1` for legacy compatibility.

## Standard reference

Per [YAML 1.2 Spec §7.3](https://yaml.org/spec/1.2/spec.html#id2785970):

| Schema tag | Behavior | Implicit types resolved |
|------------|----------|------------------------|
| `tag:yaml.org,2002:failsafe` | No type resolution | None — all scalars are strings |
| `tag:yaml.org,2002:json` | JSON-compatible | null, bool, int, float |
| `tag:yaml.org,2002:` (core) | Full YAML 1.2 | null, bool, int, float, inf, nan, hex, octal |
| `tag:yaml.org,2002:` with 1.1 rules | Legacy | null (incl. `~`), bool (incl. `yes`/`no`), int, float, inf, nan, hex, octal |

This is the same schema hierarchy used by **Ruamel.yaml** and documented in **PyYAML**'s Loader classes.

## Decision

| Decision | Choice |
|----------|--------|
| Profile names | `failsafe`, `json`, `core`, `yaml1.1` |
| Python API | `pyyaml_rs.parse(yaml, schema="core")` string parameter |
| Explicit tags | Always honored, override schema — `!!str "42"` → string even in `json` schema |
| Schema inheritance | `failsafe` ⊂ `json` ⊂ `core` ⊂ `yaml1.1` — each adds more resolution |
| Default | `core` — current behavior, no change |

## Python API

```python
# Core (YAML 1.2 full resolution — current default)
pyyaml_rs.parse("x: true\ny: 42", schema="core")
# → {"x": True, "y": 42}

# JSON — only null/bool/int/float
pyyaml_rs.parse("x: true\ny: 42", schema="json")
# → {"x": True, "y": 42}

# JSON rejects non-JSON types like inf, nan, hex, octal
pyyaml_rs.parse("x: .inf", schema="json")
# → {"x": ".inf"} (string, not float)

# Failsafe — no implicit resolution at all
pyyaml_rs.parse("x: true\ny: 42", schema="failsafe")
# → {"x": "true", "y": "42"}

# YAML 1.1 — adds yes/no/on/off bools, ~ as null
pyyaml_rs.parse("x: yes\ny: ~", schema="yaml1.1")
# → {"x": True, "y": None}
```

## Resolution per schema

| Literal | core | json | failsafe | yaml1.1 |
|---------|------|------|----------|---------|
| `null`, `Null`, `NULL`, `~` | None | None | `"null"` / `"~"` | None |
| `true`, `True`, `TRUE` | True | True | `"true"` | True |
| `false`, `False`, `FALSE` | False | False | `"false"` | False |
| `yes`, `no`, `y`, `n`, `on`, `off` | string | string | string | bool |
| `~` (standalone) | None | None | `"~"` | None |
| `42`, `-17` | 42, -17 | 42, -17 | "42" | 42, -17 |
| `0o17`, `0x1F` | 15, 31 | string | "0o17" | 15, 31 |
| `3.14`, `.5`, `1e3` | 3.14, 0.5, 1000.0 | 3.14, 0.5, 1000.0 | "3.14" | 3.14, 0.5, 1000.0 |
| `.inf`, `.Inf`, `-inf` | inf, -inf | string | ".inf" | inf, -inf |
| `.nan`, `.NaN` | nan | string | ".nan" | nan |

## Rust implementation

### New type: `YamlSchema` enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlSchema {
    Failsafe,  // no implicit resolution
    Json,      // null, bool, int, float only
    Core,      // full YAML 1.2 (current default)
    Yaml11,    // YAML 1.1 legacy
}
```

### Refactor `resolve_yaml_type` → parameterized

```rust
pub fn resolve_yaml_type(value: &str, schema: YamlSchema) -> YamlType {
    match schema {
        YamlSchema::Failsafe => YamlType::Str(value.to_string()),
        YamlSchema::Json => resolve_json_type(value),
        YamlSchema::Core => resolve_core_type(value),
        YamlSchema::Yaml11 => resolve_yaml11_type(value),
    }
}
```

Where:

- `resolve_core_type` = current `resolve_yaml_type` (move into it)
- `resolve_json_type` = core minus inf/nan/hex/octal/0o
- `resolve_yaml11_type` = core plus `yes`/`no`/`on`/`off`/`y`/`n`

### Thread schema through the call chain

1. `parse_with_options(yaml, resolve_merges, schema)` — add `YamlSchema` param
2. `YamlDocument` struct stores `schema: YamlSchema`
3. `node_to_pyobject_inner()` receives `schema` and passes to `resolve_yaml_type()`
4. **All** PyO3 parse functions accept `schema` param
5. `parse_stream()` also accepts `schema` param

### Explicit tag behavior

When a `CustomNode::Scalar` has a non-null `tag` field (e.g. `!!int`, `!!str`, `!custom`), the schema **does not affect** resolution:

- `!!str` on "42" → always Python `str("42")`
- `!!int` on "42" → always Python `int(42)`
- `!!int` on "hello" → raises `YamlParseError`

## Python function signatures

```rust
#[pyfunction]
#[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
fn parse(py: Python, yaml: &Bound<'_, PyAny>, resolve_merges: bool, schema: &str) -> PyResult<YamlDocument>
```

`schema` parsing:

- `"core"`, `"yaml.org,2002"`, `"YamlOrg2002"` → `Core`
- `"json"`, `"yaml.org,2002:json"` → `Json`
- `"failsafe"`, `"yaml.org,2002:failsafe"` → `Failsafe`
- `"yaml1.1"`, `"yaml.org,2002:yaml1.1"`, `"1.1"` → `Yaml11`
- Any other → `YamlTypeError`

## Error behavior

| Scenario | Behavior |
|----------|----------|
| `schema="unknown"` | `YamlTypeError: "unknown schema profile: 'unknown'"` |
| `!!int` on "hello" | `YamlParseError: "cannot parse 'hello' as !!int"` |
| `~` in `core` schema | Python `None` |
| `~` in `json` schema | Python `None` |
| `~` in `failsafe` | Python `str("~")` |
| `yes` in `core` schema | Python `str("yes")` |
| `yes` in `yaml1.1` schema | Python `bool(True)` |

## Files affected

- **New**: `src/parser/yaml/schema.rs` — `YamlSchema`, `resolve_json_type`, `resolve_yaml11_type`
- **Modified**: `src/parser/yaml/types.rs` — refactor `resolve_yaml_type` into `resolve_core_type` + parameterized dispatcher
- **Modified**: `src/parser/mod.rs` — add `YamlSchema` param to `parse_with_options`, `parse_all_with_options`
- **Modified**: `src/lib.rs` — add `schema` param to all parse/safe_load functions, store in `YamlDocument`
- **Modified**: `python/pyyaml_rs/__init__.py` — expose `schema` param
- **New**: `tests/test_schema.rs` — Rust unit tests for all 4 schemas
- **New**: `tests/test_schema_profiles.py` — Python integration tests
