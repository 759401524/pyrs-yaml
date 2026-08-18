---
title: NumPy ndarray 직렬화 가이드
description: NumPy 배열을 YAML 리스트로 직렬화 — 제로 복사 Rust 처리, 지원되는 dtype, 음수 처리, 성능
tags:
  - docs
status: new
---

!!! warning "프리-스레디드 빌드는 NumPy 미포함"
    프리-스레디드(cp314t) 빌드에서 `numpy.ndarray`에 `safe_dump`를 호출하면 `YamlTypeError`가 발생합니다. GIL 빌드(Python 3.8–3.15)에서는 전체 ndarray 직렬화를 지원합니다.

NumPy 배열을 YAML 리스트로 직렬화합니다. 제로 복사 Rust 처리를 지원합니다.

## 기본 사용법

```python title="1차원 배열 직렬화"
import numpy as np
import pyrs_yaml as y

# 1차원 배열
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = y.safe_dump(arr)
# 출력:
# - 1
# - 2
# - 3

# Python 리스트로 복원
data = y.safe_load(yaml_str)
assert data == [1, 2, 3]
```

## 다차원 배열

```python title="2차원·3차원 배열"
# 2차원 행렬
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = y.safe_dump(matrix)
data = y.safe_load(yaml_str)
assert data == [[1.0, 2.0], [3.0, 4.0]]

# 3차원 큐브
cube = np.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], dtype="int64")
data = y.safe_load(y.safe_dump(cube))
assert data == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
```

## 지원되는 dtype

| NumPy dtype | YAML 출력 | 예시 |
|-------------|-----------|------|
| :material-numeric: `int8/16/32/64` | 정수 | `42` |
| :material-numeric: `uint8/16/32/64` | 정수 | `42` |
| :material-decimal: `float32/64` | 부동소수점 | `3.14` |
| :material-toggle-switch: `bool` | 불리언 | `true` / `false` |
| :material-format-text: `complex64/128` | 문자열 | `(1+2j)` |

## 특수 값

```python title="NaN과 무한대"
# NaN
arr = np.array([1.0, float("nan"), 3.0])
data = y.safe_load(y.safe_dump(arr))
assert str(data[1]) == "nan"

# 무한대
arr = np.array([float("inf"), -float("inf")])
data = y.safe_load(y.safe_dump(arr))
assert data[0] == float("inf")
assert data[1] == float("-inf")
```

## 음수 처리

YAML 1.2 사양에서는 블록 시퀀스에 `-`로 시작하는 일반 스칼라를 포함할 수 없습니다. 음수 값은 자동으로 단일 따옴표로 감싸지며, 순환 파싱 시 올바르게 파싱됩니다:

```python title="음수 값"
arr = np.array([-100, 200], dtype="int16")
data = y.safe_load(y.safe_dump(arr))
assert data == [-100, 200]  # 순환 파싱 정상 처리
```

## 0차원 스칼라 배열

0차원 배열은 1차원으로 리셰이프된 후 직렬화되며, 단일 항목 리스트가 됩니다:

```python title="0차원 스칼라"
scalar = np.array(42, dtype="int32")
data = y.safe_load(y.safe_dump(scalar))
assert data == [42]
```

## 구조체 내 중첩

NumPy 배열은 dict나 list에 포함될 수 있습니다:

```python title="dict에 중첩"
data = {"matrix": np.array([[1, 2], [3, 4]]), "label": "test"}
yaml_str = y.safe_dump(data)
loaded = y.safe_load(yaml_str)
assert loaded["matrix"] == [[1, 2], [3, 4]]
```

!!! note "복소수"
    YAML에는 네이티브 복소수 타입이 없습니다. 복소수는 `(re+imj)` 문자열로 직렬화되며, `safe_load`는 Python `complex`가 아닌 문자열로 반환합니다.

## 지원되지 않는 타입

다음 타입은 `YamlTypeError`를 발생시킵니다:

- :material-alert: 문자열 배열
- :material-alert: 객체 배열
- :material-alert: 구조화 배열
- :material-alert: 비숫자 커스텀 dtype

## 성능

- :material-bolt: `PyUntypedArray`를 사용한 제로 복사 dtype 디스패치
- :material-bolt: `PyArrayDyn<T>`를 사용한 제로 복사 슬라이스 반복
- :material-bolt: 슬라이스 이터레이션 중 Python GIL 해제
- :material-bolt: 추가 할당 없이 모든 차원 지원
