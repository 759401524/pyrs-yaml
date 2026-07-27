# pyyaml-rs

A high-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.

## Features

- **YAML 1.2 compliant** - Uses saphyr-parser for full YAML 1.2 support
- **Perfect Round-Trip** - Preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **High Performance** - Rust backend, see [benchmarks](benches/yaml_bench.rs)
- **NumPy ndarray support** - `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` serialize `numpy.ndarray` of any dimension (0-D through N-D) with zero-copy Rust dispatch
- **Custom AST** - Extensible AST for advanced YAML manipulation
- **PyYAML Compatible** - Drop-in replacement with `safe_load`/`safe_dump` API

## Installation

```bash
pip install pyyaml-rs
```

## Quick Start

```python
import pyyaml_rs

# Parse YAML
doc = pyyaml_rs.parse("key: value")
print(doc.to_yaml())  # key: value

# PyYAML compatible API
data = pyyaml_rs.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyyaml_rs.parse(original)
assert doc.to_yaml() == original  # True
```

### NumPy ndarray support

```python
import numpy as np
import pyyaml_rs

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyyaml_rs.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyyaml_rs.safe_dump(matrix)
print(yaml_str)
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

## Features Supported

| Feature | Support |
|---------|---------|
| YAML 1.2 | Full |
| Comments (standalone + inline) | Preserved |
| Anchors (`&`) and aliases (`*`) | Preserved |
| Tags (`!!str`, `!!int`, etc.) | Preserved |
| Chomping (`\|-`, `\|+`, `>-`, `>+`) | Preserved |
| Complex keys (sequence/mapping as key) | Supported |
| Escape sequences (`\n`, `\t`, `\uXXXX`) | Supported |
| Flow collections (`{}`, `[]`) | Preserved |
| Block scalars (`\|`, `>`) | Preserved |
| Merge keys (`<<: *alias`) | Resolved (opt-out via `resolve_merges=False`) |
| **NumPy ndarray** | **Full (0-D through N-D)** |

## API Reference

### Core Functions

```python
# Parse YAML string (accepts str or bytes)
doc = pyyaml_rs.parse(yaml_str)
doc = pyyaml_rs.parse(yaml_bytes)

# Parse with options
doc = pyyaml_rs.parse(yaml_str, resolve_merges=False)

# Parse YAML file
doc = pyyaml_rs.parse_file("config.yaml")

# Parse multiple YAML documents
docs = pyyaml_rs.parse_all_docs(yaml_str)

# Convert to YAML string (with options)
yaml_str = doc.to_yaml()
yaml_str = doc.to_yaml_with_options(indent_size=4, explicit_start=True, sort_keys=True)

# Get value by key (with default)
value = doc.get("key")
value = doc.get("missing_key", "default")

# Get root type
doc.root_type()  # "mapping", "sequence", "scalar", "null"

# Check containment and length
"key" in doc
len(doc)

# Iterate
for key in doc:
    print(key, doc[key])
```

### PyYAML Compatible API

```python
# Load YAML to dict
data = pyyaml_rs.safe_load(yaml_str)

# Load multiple documents
docs = pyyaml_rs.safe_loads(yaml_str)

# Dump dict to YAML
yaml_str = pyyaml_rs.safe_dump(data)

# Convert dict to YAML
yaml_str = pyyaml_rs.from_dict(data)

# Convert JSON to YAML
yaml_str = pyyaml_rs.from_json(json_str)

# Dump to file
pyyaml_rs.dump_file(data, "output.yaml")

# Extract YAML frontmatter from markdown
frontmatter, content = pyyaml_rs.read_markdown("post.md")
frontmatter, content = pyyaml_rs.read_markdown_str(markdown_text)

# i18n language management
pyyaml_rs.set_language("zh-CN")
pyyaml_rs.get_language()  # "zh-CN"
pyyaml_rs.list_languages()  # ["en", "zh-CN"]
pyyaml_rs.detect_language()  # auto-detect from environment
pyyaml_rs.negotiate_language(["zh-CN", "en"], "en")  # "zh-CN"
```

## Performance

Criterion benchmarks in `benches/yaml_bench.rs`:

| Operation | Time |
|-----------|------|
| Parse (small, ~2 keys) | ~1.7 us |
| Parse (medium, ~30 keys) | ~20 us |
| Parse (large, ~60 keys) | ~93 us |
| Serialize (small) | ~200 ns |
| Serialize (medium) | ~1.7 us |
| Serialize (large) | ~4.9 us |
| Roundtrip (large) | ~99 us |

## Development

```bash
# Install dependencies
uv sync

# Build Python extension
maturin develop --release

# Run tests
cargo test
pytest tests/

# Run benchmarks
cargo bench

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
