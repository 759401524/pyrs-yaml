---
title: Running Tests
description: How to run Rust and Python tests for pyrs-yaml, including nextest, pytest, and the YAML Test Suite.
tags:
  - docs
status: new
---

## Running Tests

pyrs-yaml has both Rust unit tests and Python integration tests.

### Rust Tests

```bash
# Run all Rust tests with nextest (preferred)
cargo nextest run --all

# Run all Rust tests with cargo test
cargo test --all

# Run pure Rust core tests (no Python runtime needed)
cargo test --all --no-default-features

# Run with output
cargo test --all -- --nocapture
```

#### Test Coverage

- **`crates/pyrs-yaml-core/src/ast.rs`** — Node construction, metadata, equality
- **`crates/pyrs-yaml-core/src/parser/`** — Parsing various YAML constructs
- **`crates/pyrs-yaml-core/src/serializer.rs`** — Serialization round-trips
- **`crates/pyrs-yaml-core/src/editing/`** — Edit primitives (navigate, region, dirty, metadata)
- **`crates/pyrs-yaml-core/src/integration/`** — YAML Test Suite integration
- **`crates/pyrs-yaml/src/fidelity.rs`** — Property-based fuzz tests

### Python Tests

```bash
# Run all Python tests
uv run pytest tests/ -v

# Run a specific test file
uv run pytest tests/test_edit.py -v

# Run a specific test class
uv run pytest tests/test_node_api.py::TestDocWalk -v

# Run with coverage
uv run pytest tests/ -v --cov=pyrs_yaml

# Run compliance suite
uv run pytest tests/test_yaml_suite.py -v

# Run benchmarks
uv run pytest tests/ --codspeed
```

### Maturin Build

```bash
# Build and install (uses monorepo manifest-path)
uv run maturin develop --release

# Generate stubs for .pyi files
uv run maturin build --release --generate-stubs
```

#### Test Files

| File | Coverage |
|------|----------|
| `test_parse.py` | Parsing, data types, special chars |
| `test_serialize.py` | Serialization, round-trips |
| `test_edge_cases.py` | Edge cases, error handling |
| `test_errors.py` | Custom exception types, file I/O |
| `test_features.py` | Markdown frontmatter, from_dict/from_json |
| `test_json.py` | JSON ↔ YAML conversion |
| `test_tabs.py` | Tab handling |
| `test_yaml_suite.py` | YAML Test Suite integration |
| `test_performance.py` | Performance sanity checks |
| **`test_numpy.py`** | **NumPy ndarray serialization (0-D through N-D, all dtypes)** |

### CI Testing

GitHub Actions runs on every push and PR:

- **Rust**: `cargo nextest run --all`, `cargo clippy --all -- -D warnings`, `cargo fmt --check`
- **Python**: `uv run pytest tests/` on 4 Python versions × 3 OSes
- **Maturin**: Build wheel for each Python version (via `crates/pyrs-yaml/Cargo.toml`)

### Adding New Tests

#### Rust Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // Your test here
    }
}
```

#### Python Test

```python
import pyrs_yaml
import pytest


class TestNewFeature:
    def test_basic(self):
        result = pyrs_yaml.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # Edge case test
        pass
```

### Test Categories

- **Unit tests** — Individual functions, small inputs
- **Integration tests** — Full parse → serialize round-trips
- **Edge case tests** — Special characters, empty input, malformed YAML
- **Performance tests** — Sanity checks (not benchmarks)
- **YAML Test Suite** — External test suite for YAML compliance

### YAML Test Suite Known Deviations

The suite pass rate is gated at **95%** (see `test_compliance_report`). A small
number of cases are intentionally not chased because rejecting them is
spec-correct and matches reference parsers (notably PyYAML/libyaml):

| ID | Input | Why accepted as a deviation |
|:---|:------|:----------------------------|
| `ZYU8` | `%YAML 1.1 1.2` | Version directive with trailing content is **invalid** per the YAML 1.2 grammar (`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`). PyYAML rejects it too. The suite's own note says these directive variants are "not at all usefully valid" and that supporting them is not encouraged. |

All other suite cases pass (currently 405/406 = 99.75%).
