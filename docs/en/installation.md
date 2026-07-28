# Installation

## Requirements

- **Python** ≥ 3.8 (CPython)
- **Platform**: Linux, macOS, Windows

## Install from Source

The package is not yet published on PyPI. To install from source:

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs
uv run --frozen maturin develop --release
```

The package is built as an **ABI3 wheel**, meaning a single wheel works across Python 3.8 through 3.15 — no recompilation needed.

## Quick Check

```python
import pyyaml_rs

# Check version
print(pyyaml_rs.__version__)

# Quick smoke test
doc = pyyaml_rs.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```

## Run Tests

```bash
# Rust tests
cargo test

# Python tests
uv run --frozen pytest tests/
```
