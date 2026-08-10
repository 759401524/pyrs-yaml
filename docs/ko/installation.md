---
title: Installation
description: pyrs-yaml의 시스템 요구사항, 소스 설치, 프리-스레디드 빌드 및 설치 확인 방법
tags:
  - docs
status: new
---

## 시스템 요구사항

- **Python** ≥ 3.8 (CPython)
- **플랫폼**: Linux, macOS, Windows

## 소스에서 설치

패키지는 아직 PyPI에 게시되지 않았습니다. 소스에서 설치:

```bash
uv run --frozen maturin develop --release
```

패키지는 **ABI3 휠**로 빌드되며, 단일 휠로 Python 3.8부터 3.15까지 지원 — 재컴파일 불필요.

## 프리-스레디드 Python (cp314t)

!!! warning "프리-스레디드 빌드는 NumPy 미포함"
    프리-스레디드 빌드에서 `numpy.ndarray`에 `safe_dump`를 호출하면 `YamlTypeError`가 발생합니다.

CPython 3.14t용 프리-스레디드(no-GIL) 휠은 `--no-default-features`로 빌드되어 NumPy 통합이 **포함되지 않습니다**: 프리-스레디드 빌드에서 `numpy.ndarray`에 `safe_dump`를 호출하면 `YamlTypeError`가 발생합니다. GIL 빌드(Python 3.8–3.15)에서는 전체 ndarray 직렬화를 지원합니다.

## 개발용 설치

소스에서 설치 (개발 또는 테스트용):

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 설치 확인

???+ tip "설치 확인"
    `pyrs_yaml.__version__`을 출력하고 간단한 파싱 테스트로 설치를 검증할 수 있습니다.

```python
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
