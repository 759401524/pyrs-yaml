---
title: Installation
description: Install pyrs-yaml, including requirements, free-threaded Python notes, and a quick install verification.
tags:
  - docs
status: new
---

## Installation

### Requirements

| :material-language-python: Requirement | Details |
|---|---|
| **Python** | ≥ 3.8 (CPython), including 3.14t free-threaded |
| :material-monitor: **Platform** | Linux, macOS, Windows |
| :material-hammer-wrench: **Build** | Rust toolchain (for source builds only) |

### Install from PyPI

The package is published on PyPI. Install with pip:

```bash title="Install from PyPI"
pip install pyrs-yaml
```

The package is built as an **ABI3 wheel**, meaning a single wheel works across Python 3.8 through 3.15 — no recompilation needed.

### Install from Source

For development or the latest unreleased changes:

```bash title="Install from source"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

### Free-Threaded Python (cp314t)

Free-threaded (no-GIL) wheels for CPython 3.14t include the NumPy integration. When NumPy is installed in the environment, `safe_dump` / `from_dict` serialize `numpy.ndarray` values normally; when NumPy is absent, the integration is inert and calls fall through to the default object handler. GIL builds (Python 3.8–3.15) keep full ndarray serialization support.

!!! note "NumPy is auto-detected at runtime"
    The NumPy integration is compiled in on every wheel (GIL and free-threaded)
    but only activates when NumPy is importable. If NumPy is not installed,
    `safe_dump` on a `numpy.ndarray` raises `YamlTypeError` (the value is not a
    recognized type).

### Quick Check

???+ tip "Verifying your install"
    Run the snippet below to confirm the module imports, the version is
    reported, and a basic parse/round-trip works. A successful run prints
    `✓ Installation verified`.

```python title="Verify your install"
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```

### Run Tests

=== "Rust"

    ```bash
    cargo nextest run --all
    ```

=== "Python"

    ```bash
    uv run --frozen pytest tests/
    ```
