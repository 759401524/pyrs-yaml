---

title: Installation
lang: ko

## 설치

### 시스템 요구사항

- **Python** ≥ 3.8 (CPython)
- **플랫폼**: Linux, macOS, Windows

### 소스에서 설치

패키지는 아직 PyPI에 게시되지 않았습니다. 소스에서 설치:

```bash
uv run --frozen maturin develop --release
```

패키지는 **ABI3 휠**로 빌드되며, 단일 휠로 Python 3.8부터 3.15까지 지원 — 재컴파일 불필요.

### 개발용 설치

소스에서 설치 (개발 또는 테스트용):

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

### 설치 확인

```python
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
