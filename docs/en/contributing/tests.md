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

```bash title="Run Rust tests"
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

```bash title="Run Python tests"
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

```bash title="Build and install"
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
- **Python**: `uv run pytest tests/` on 8 Python versions × 3 OSes — 12 (OS, version) cells
- **Maturin**: Build wheel for each Python version (via `crates/pyrs-yaml/Cargo.toml`)

### Adding New Tests

#### Rust Test

```rust title="Rust test template"
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

```python title="Python test template"
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

The suite is read by three named gates in `tests/test_yaml_suite.py` rather than by a percentage:
every valid case must parse except the one refusal this project makes on purpose, no invalid case
may parse, and every JSON comparison must match except a ledger of five cases, each recorded with
its cause. `test_compliance_report` floors the report's own number at 99.5%. A small number of
cases are intentionally not chased because rejecting them is spec-correct and matches reference
parsers (notably PyYAML/libyaml):

Cases are named the way the gates name them. A suite id is a *file*, and 18 of its 351 files carry
more than one case, so a name also carries the case's position inside that file.

| ID | Input | Why accepted as a deviation |
|:---|:------|:----------------------------|
| `ZYU8#3` | `%YAML 1.1 1.2` | Version directive with trailing content is **invalid** per the YAML 1.2 grammar (`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`). PyYAML rejects it too. The suite's own note says these directive variants are "not at all usefully valid" and that supporting them is not encouraged. |

All other suite cases pass (currently 405/406 = 99.75%).

The JSON ledger is two open defects and one comparison difference, and an entry leaves it by the
engine learning the rule, never by the set growing. The defects: a keep-chomped block whose content
lines are all empty keeps one line break too many (`JEF9#1`, `K858#1`). The difference is spelling:
the suite writes a `!!binary` node as the scalar's own source text, while the comparison writes the
loaded bytes as canonical base64 (`565N#1`) - and both spellings construct the same 185 bytes, which
is what `test_binary_spellings_construct_the_same_bytes` pins.

Two entries have left it that way. An alias used to bind the *last* anchor of its name instead of
the nearest preceding one, so `3GZX` read `Bar` where the suite's own event tree and ruamel both
read `Foo`; PyYAML refuses a reused anchor name outright (`found duplicate anchor`), so the corpus
and the library that accepts it are the authority, and
`test_a_reused_anchor_name_is_read_and_not_refused` pins the divergence. And a bare `!` used to be
resolved by the schema like an untagged scalar, so `! 12` loaded as an integer: YAML 1.2 §6.8 leaves
that node's tag as `!` and §10.5.2 restricts application resolution rules to the `?` tag, which
makes the scalar its own text - what spec Example 6.28 and the suite both spell as `"12"`. Both
Python libraries resolve it implicitly instead (ruamel has an open ticket, #573), so that
divergence is recorded with both answers, and the loader together with `to_json`, `to_jsonc`,
`to_json5` and `to_toml` is held to one answer by a single table in
`tests/test_cross_format_tags.py`.
