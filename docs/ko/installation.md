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
| **Python** | ≥ 3.8 (CPython), free-threaded 3.14t 및 3.15t(사전 빌드 휠) 포함 |
| :material-monitor: **플랫폼** | Linux, macOS, Windows |
| :material-hammer-wrench: **빌드** | Rust 툴체인 (소스 빌드에만 필요) |

## PyPI에서 설치

패키지는 PyPI에 게시되었습니다. pip로 설치:

```bash title="PyPI에서 설치"
pip install pyrs-yaml
```

GIL 빌드는 단일 **`cp38-abi3` 휠**로 제공되므로 Python 3.8부터 3.15까지 재컴파일 없이 동작합니다.

## 소스에서 설치

개발용 또는 최신 미출시 변경용:

```bash title="소스에서 설치"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 프리-스레디드 Python (cp314t과 cp315t)

CPython 3.14t와 3.15t용 프리-스레디드(no-GIL) 빌드는 NumPy 통합을 포함합니다. 환경에 NumPy가 설치된 경우 `safe_dump` / `from_dict`는 `numpy.ndarray` 값을 정상적으로 직렬화합니다. NumPy가 없는 경우 통합은 비활성화되며 호출은 기본 객체 핸들러로 폴스루됩니다. GIL 빌드(Python 3.8–3.15)에서는 전체 ndarray 직렬화를 지원합니다.

프리-스레디드 빌드의 안정 ABI(`abi3t`)는 3.15부터 시작하므로 3.14t와 3.15t는 하나의 휠을 공유할 수 없습니다. 공개된 프리-스레디드 휠은 3.14t용 `cp314t`와 3.15t 이상용 `cp315-abi3.abi3t`이며, Windows(x64/x86/arm64), macOS(x86_64/arm64), Linux glibc(x86_64/arm64)를 대상으로 합니다. musllinux와 나머지 Linux 아키텍처는 소스 빌드(`uv run --frozen maturin develop --release`)로 지원되며, CI가 이 경로를 실행합니다.

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
print(pyrs_yaml.__version__)

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
