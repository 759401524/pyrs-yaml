---
title: Comparison with Other Libraries
description: pyrs-yaml을 PyYAML 및 ruamel.yaml과 비교 — 성능, 기능, 마이그레이션 경로
tags:
  - docs
status: new
---

pyrs-yaml을 가장 인기 있는 두 Python YAML 라이브러리와 비교합니다.

## 성능 비교

### Parse 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **1.5 ms** | — |
| PyYAML | 57.7 ms | 38× 느림 |
| ruamel.yaml | 127.9 ms | 85× 느림 |

#### Serialize 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **0.17 ms** | — |
| PyYAML | 30.2 ms | 177× 느림 |
| ruamel.yaml | 63.1 ms | 371× 느림 |

#### Round-Trip 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **1.6 ms** | — |
| PyYAML | 87.9 ms[^1] | 55× 느림 |
| ruamel.yaml | 191.0 ms[^1] | 119× 느림 |

[^1]: PyYAML/ruamel의 라운드트립 시간은 동일 벤치마크의 파싱·직렬화 합계로 추정한 값입니다.

## 기능 비교

| 기능 | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2 준수** | :material-check: | :material-check: | :material-check: |
| **주석 (standalone)** | :material-check: | :material-close: | :material-check: |
| **주석 (inline)** | :material-check: | :material-close: | :material-check: |
| **Anchor/alias** | :material-check: | :material-close: | :material-check: |
| **Tag (explicit)** | :material-check: | :material-close: | :material-check: |
| **Block scalar** | :material-check: | :material-check: | :material-check: |
| **Flow collection** | :material-check: | :material-check: | :material-check: |
| **Merge key (<<)** | :material-check: | :material-close: | :material-check: |
| **복잡한 키** | :material-check: | :material-check: | :material-check: |
| **Round-trip 보존** | :material-check: | :material-close: | :material-check: |
| **Python 바인딩** | :material-check: | :material-check: | :material-check: |
| **ABI3 (py3.8+)** | :material-check: | :material-close: | :material-close: |
| **Type stubs (.pyi)** | :material-check: | :material-check: | :material-close: |
| **i18n 오류 메시지** | :material-check: | :material-close: | :material-close: |
| **Rust 백엔드** | :material-check: | :material-close: | :material-close: |
| **성능** | :material-rocket-launch: 가장 빠름 | :material-snail: 느림 | :material-snail: 느림 |

## 요약

### pyrs-yaml을 선택할 때

- **성능이 중요합니다** — PyYAML보다 파싱 21~43×, 직렬화 55~177× 빠름
- **Round-trip 보존이 중요합니다** — 주석, anchor, tag 보존
- **PyYAML 호환성을 원합니다** — 즉시 교체 가능한 API
- **Type hints가 필요합니다** — 완전한 `.pyi` stubs
- **단일 wheel을 원합니다** — ABI3로 Python 3.8~3.15 지원

#### PyYAML을 선택할 때

- 이미 사용하고 있으며 round-trip 보존이 필요하지 않은 경우
- 기존 코드와의 최대 호환성이 필요한 경우
- 성능이 중요한 고려사항이 아닌 경우

#### ruamel.yaml을 선택할 때

- 가장 기능 완성도가 높은 YAML 파서가 필요한 경우
- 복잡한 YAML 조작을 수행하는 경우
- 성능이 중요한 고려사항이 아닌 경우 (가장 느린 옵션)

## 마이그레이션 경로

```python
# Step 1: Install
pip install pyrs-yaml

# Step 2: Replace import
# Before:
import yaml

# After:
import pyrs_yaml as yaml

# Step 3: Test
# Run your existing tests to verify compatibility
```

대부분의 코드는 변경 없이 작동합니다. 주요 차이점:

1. Round-trip 출력은 주석과 서식을 보존합니다
2. 오류 메시지가 더 상세하며 지역화할 수 있습니다
3. 성능이 크게 향상됩니다
