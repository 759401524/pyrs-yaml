# Roadmap

All versions are tracked here with release dates and delivered features.

---

## Released

### v0.4.0 — 2026-07-27 ✅

- **Version unification** — `Cargo.toml` is the single source of truth; `pyproject.toml` uses `dynamic = ["version"]`; `__version__` uses `importlib.metadata`
- **132 gap-filling tests** across all public APIs (i18n, bytes, unicode, flow collections, merge keys, anchor on non-scalar nodes, etc.)
- **i18n function tests** — `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`
- **`parse_all_docs`** dedicated test suite and test coverage for `parse_file`, `to_yaml_with_options`, `to_dict`, `YamlDocument` dunder methods
- **`safe_load`/`safe_loads`** feature coverage — anchors, merge keys, block scalars, flow collections, special floats, type resolution
- **YAML Test Suite individual case tests** — octal, hex, scientific notation, NaN, infinity, merge keys, block scalar strip, flow collections
- **Pre-commit hooks** — `ruff`, `cargo fmt`, `cargo clippy`, markdown linting via `prek`

### v0.3.0 — 2025-07-27 ✅

- **NumPy ndarray serialization** — `safe_dump()`/`safe_dumps()`/`from_dict()`/`dump_file()` support `numpy.ndarray` of all dimensions (0-D through N-D)
  - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
  - Zero-copy Rust dispatch via `numpy` crate (`PyUntypedArray` + `PyArrayDyn`)
  - GIL released during slice iteration for large arrays
  - Complex numbers serialized as `(re+imj)` strings
- **Flow collections** (`{}`/`[]`) round-trip with `flow_style` field on AST nodes
- **Quoted scalars** — `quoted_scalar()` constructor, negative number round-trip fixed
- **`parse()` accepts `bytes`**, `parse_all_docs()`, `to_yaml_with_options()`
- **`resolve_merges`** parameter to control merge key expansion
- **Comprehensive NumPy test suite** — 42 tests across all dtypes and dimensions
- **CI matrix** (3 OS × 4 Python versions), Criterion benchmarks, full `.pyi` stubs

### v0.2.0 — 2025-07-26 ✅

- Flow collections round-trip, `parse()` accepts `str`/`bytes`, `resolve_merges` option
- `to_yaml_with_options()` with indent/start/end markers/sort_keys
- `get()` default values, `dump_file()`, `parse_all_docs()`
- Criterion benchmarks, i18n (`zh-CN`, `en`), `read_markdown`, `from_json`, `from_dict`
- All `#[pyo3(signature)]` attributes, `py.typed` PEP 561 marker

### v0.1.0 — 2025-07-25 ✅

- Initial release with saphyr-parser (YAML 1.2, 98.1% test suite pass rate)
- Custom AST with full round-trip metadata (comments, anchors, tags, chomping, scalar styles)
- PyYAML-compatible API (`safe_load`/`safe_dump`), `from_dict`/`from_json`, `read_markdown`
- Block scalars with chomping, escape sequences, complex keys, merge key resolution

---

## In Progress

### v0.6.0 — Planned

- **Custom tag ecosystem** — plugin system for user-defined YAML tags with Python callbacks
- **WASM build target** — `wasm32-unknown-unknown` compilation via maturin, enabling browser/WASM execution
- **Async serialization** — non-blocking `safe_dump_async()` for large documents using Python coroutines
- **JSON Schema ↔ YAML schema validation** — declarative document validation layer
- **Incremental re-parse** — modify a subset of a parsed document without re-serializing the whole tree
- **Faster round-trip** — optimize serializer hot-path for block-style output (target: 50× over PyYAML)

### v0.7.0 — Research / Future

---

## Research

- [ ] YAML 1.2 schema profiles — feasibility explored via saphyr-parser's tag resolution hooks
- [ ] WASM build target — maturin supports `wasm32-unknown-unknown`; requires `pyo3` 0.21+
- [ ] Async serialization — Python `asyncio` integration via `py.allow_threads` on GIL release
- [ ] Schema validation plugins — custom tag ecosystem can serve as validation infrastructure
