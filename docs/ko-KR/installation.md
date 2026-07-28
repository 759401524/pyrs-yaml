# ---
---

title: Installation
lang: ko-KR

# 설치

## Requirements

- **Python** ≥ 3.9 (CPython)
- **Platform**: Linux, macOS, Windows

## pip install

```bash
pip install pyyaml-rs
```

The package is published as an **ABI3 wheel**, meaning a single wheel works across Python 3.9 through 3.13 — no recompilation needed.

## Development 설치

To install from source (for development or testing):

```bash
git clone https://github.com/MuLong/pyyaml-rs.git
cd pyyaml-rs
pip install maturin
maturin develop --release
```

## Verify 설치

```python
import pyyaml_rs

# Check version
print(pyyaml_rs.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyyaml_rs.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
