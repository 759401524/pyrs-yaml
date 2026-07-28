---

title: Running Tests
lang: ko-KR

## 테스트 실행

pyyaml-rs has both Rust unit tests and Python integration tests.

### Rust Tests

```bash
# Run all Rust tests
cargo test

# Run tests for a specific module
cargo test ast
cargo test parser
cargo test serializer

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration
```

#### Test Coverage

- **`src/ast.rs`** — Node construction, metadata, equality
- **`src/parser/`** — Parsing various YAML constructs
- **`src/serializer.rs`** — 직렬화 round-trips
- **`src/integration/`** — YAML Test Suite integration

### Python Tests

```bash
# Run all Python tests
pytest tests/

# Run with verbose output
pytest tests/ -v

# Run a specific test file
pytest tests/test_parse.py

# Run tests matching a pattern
pytest tests/ -k "comment"

# Run with coverage
pytest tests/ --cov=pyyaml_rs --cov-report=term-missing

# Run benchmarks
pytest tests/ --benchmark-only --benchmark-json=results.json
```

#### Test Files

| File | Coverage |
|------|----------|
| `test_parse.py` | Parsing, data types, special chars |
| `test_serialize.py` | 직렬화, round-trips |
| `test_edge_cases.py` | Edge cases, error handling |
| `test_errors.py` | Custom exception types, file I/O |
| `test_features.py` | Markdown frontmatter, from_dict/from_json |
| `test_json.py` | JSON ↔ YAML conversion |
| `test_tabs.py` | Tab handling |
| `test_yaml_suite.py` | YAML Test Suite integration |
| `test_performance.py` | 성능 sanity checks |
| **`test_numpy.py`** | **NumPy ndarray serialization (0-D through N-D, all dtypes)** |

### CI Testing

GitHub Actions runs on every push and PR:

- **Rust**: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- **Python**: `pytest tests/` on 4 Python versions × 3 OSes
- **Maturin**: Build wheel for each Python version

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
import pyyaml_rs
import pytest

class TestNewFeature:
    def test_basic(self):
        result = pyyaml_rs.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # Edge case test
        pass
```

### Test Categories

- **Unit tests** — Individual functions, small inputs
- **Integration tests** — Full parse → serialize round-trips
- **Edge case tests** — Special characters, empty input, malformed YAML
- **성능 tests** — Sanity checks (not benchmarks)
- **YAML Test Suite** — External test suite for YAML compliance
