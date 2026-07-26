# Roadmap

## Released

### v0.2.0 — 2025-07-26 ✅
- Flow collections (`{}`/`[]`) round-trip
- `parse()` accepts `str` and `bytes`; `resolve_merges` option
- `to_yaml_with_options()` with indent, start/end markers, sort_keys
- `get()` default values, `dump_file()`, `parse_all_docs()`
- Criterion benchmarks, CI matrix, i18n, `read_markdown`, `from_json`, `from_dict`
- Inline `#[pymodule]` for `pyo3-introspection` compatibility
- All `#[pyo3(signature)]` attributes with Python type annotations
- `experimental-inspect` feature enabled
- Full `.pyi` stubs with type annotations
- Anchor name parsing expanded to full YAML 1.2 spec

## Upcoming

### v0.3.0 — In Progress
- [ ] Multi-document YAML support (`parse_all_docs()`) — partial, refine error handling
- [ ] Performance improvements — reduce serialization overhead for large documents
- [ ] Additional i18n language packs — expand beyond zh-CN and en

### v0.4.0 — Planned
- [ ] **NumPy array support** — native `numpy.ndarray` serialization/deserialization via `__array_interface__` or `__array_struct__` hook
  - Parse YAML with `!!numpy/ndarray` tag to reconstruct arrays
  - Serialize NumPy arrays preserving dtype, shape, and memory layout
  - Support for array slicing, strided views, and structured dtypes
  - [Reference](https://numpy.org/doc/stable/reference/arrays.interface.html)

### v0.5.0 — Future
- [ ] Schema validation — JSON Schema-like validation for YAML documents
- [ ] Streaming parser — low-memory parsing for multi-GB YAML files
- [ ] Custom tag ecosystem — plugin system for user-defined YAML tags

## Research
- [ ] WebAssembly build target — run pyyaml-rs in browser/WASM environments
- [ ] Async serialization — non-blocking `to_yaml_async()` for large documents
- [ ] YAML 1.2 schema profiles — restrict/extend type resolution per use case