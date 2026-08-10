---
title: Installation
description: Install pyrs-yaml, including requirements, free-threaded Python notes, and a quick install verification.
tags:
  - docs
status: new
---

## Installation

### Requirements

- **Python** ≥ 3.8 (CPython)
- **Platform**: Linux, macOS, Windows

### Install from Source

The package is not yet published on PyPI. To install from source:

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

The package is built as an **ABI3 wheel**, meaning a single wheel works across Python 3.8 through 3.15 — no recompilation needed.

### Free-Threaded Python (cp314t)

Free-threaded (no-GIL) wheels for CPython 3.14t are built with `--no-default-features`, so the NumPy integration is **not included**: `safe_dump` on a `numpy.ndarray` raises `YamlTypeError` on free-threaded builds. GIL builds (Python 3.8–3.15) keep full ndarray serialization support.

!!! warning "Free-threaded builds exclude NumPy"
    On free-threaded (cp314t) wheels the NumPy integration is not compiled in,
    so calling `safe_dump` on a `numpy.ndarray` raises `YamlTypeError`. GIL
    builds (Python 3.8–3.15) keep full ndarray serialization support.

### Quick Check

???+ tip "Verifying your install"
    Run the snippet below to confirm the module imports, the version is
    reported, and a basic parse/round-trip works. A successful run prints
    `✓ Installation verified`.

```python
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```

### Run Tests

```bash
# Rust tests
cargo test

# Python tests
uv run --frozen pytest tests/
```
