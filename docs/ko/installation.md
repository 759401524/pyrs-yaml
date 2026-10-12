---
title: Installation
description: pyrs-yaml의 시스템 요구사항, 소스 설치, 프리-스레디드 빌드 및 설치 확인 방법
tags:
  - docs
status: new
---

## 시스템 요구사항

| :material-language-python: 요구사항 | 세부 정보 |
|---|---|
| **Python** | ≥ 3.8 (CPython), free-threaded 3.14t와 3.15t 포함 |
| :material-monitor: **플랫폼** | Linux, macOS, Windows |
| :material-hammer-wrench: **빌드** | Rust 툴체인 (소스 빌드에만 필요) |

## PyPI에서 설치

패키지는 PyPI에 게시되었습니다. pip로 설치:

```bash title="PyPI에서 설치"
pip install pyrs-yaml
```

패키지는 **ABI3 휠**로 빌드되며, 단일 휠로 Python 3.8부터 3.15까지 지원 — 재컴파일 불필요.

## 소스에서 설치

개발용 또는 최신 미출시 변경용:

```bash title="소스에서 설치"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 프리-스레디드 Python (cp314t과 cp315t)

CPython 3.14t와 3.15t용 프리-스레디드(no-GIL) 휠은 NumPy 통합을 포함합니다. 환경에 NumPy가 설치된 경우 `safe_dump` / `from_dict`는 `numpy.ndarray` 값을 정상적으로 직렬화합니다. NumPy가 없는 경우 통합은 비활성화되며 호출은 기본 객체 핸들러로 폴스루됩니다. GIL 빌드(Python 3.8–3.15)에서는 전체 ndarray 직렬화를 지원합니다.

두 프리-스레디드 휠은 만드는 방식이 다르고, 그래서 둘 다 필요합니다. 프리-스레디드 빌드의 안정 ABI(`abi3t`)는 3.15부터 시작하므로 `cp315t` 이후는 ABI 휠로 덮입니다. 3.14t는 그 ABI보다 앞서므로 전용 휠이 필요하고, 버전별 산출물로 배포됩니다.

!!! note "NumPy는 런타임에 자동 감지됩니다"
    NumPy 통합은 모든 휠(GIL 및 프리-스레디드)에 컴파일되어 있지만, NumPy를 임포트할 수 있을 때만 활성화됩니다. NumPy가 설치되어 있지 않으면 `numpy.ndarray`에 `safe_dump`를 호출하면 `YamlTypeError`가 발생합니다(값이 인식된 타입이 아님).

## 개발용 설치

소스에서 설치 (개발 또는 테스트용):

```bash title="소스에서 설치"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 설치 확인

???+ tip "설치 확인"
    `pyrs_yaml.__version__`을 출력하고 간단한 파싱 테스트로 설치를 검증할 수 있습니다.

```python title="설치 확인"
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
