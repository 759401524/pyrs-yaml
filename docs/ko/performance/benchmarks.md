---
title: Benchmarks
description: pyrs-yaml의 성능 벤치마크 — 파싱, 직렬화, 순환 성능, Rust 측 divan 벤치마크
tags:
  - docs
status: new
---

!!! note "벤치마크 환경"
    CodSpeed CI는 **시뮬레이션** 모드(`mode: simulation`)로 실행되어 실제 시간 대신 명령 수를 측정합니다. 아래 밀리초 값은 2026-08-17에 단일 머신에서 기록한 실제 시간 측정치이며 CodSpeed 출력이 아니고, 이후 재측정되지 않았습니다. 배율은 참고용이며 인터프리터, 하드웨어, 문서에 따라 달라집니다.

pyrs-yaml의 성능 벤치마크. CodSpeed의 시뮬레이션 수치는 상대 추세 추적용이며, 아래 실제 시간 표는 단일 머신에서 기록한 과거 측정치입니다.

## Methodology

- **도구:** divan (Rust) + `pytest-codspeed` (Python)
- **반복:** CI는 CodSpeed 시뮬레이션을 실행하며, Python 표는 실제 시간 측정치이지 시뮬레이션 출력이 아닙니다
- **지표:** 밀리초 (Python, 실제 시간), 마이크로초 (Rust, 시뮬레이션)

## Parse 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.18 ms | 3.8 ms | 8.8 ms | **21×** |
| 중규모 (~690 B) | 0.56 ms | 24.2 ms | 56.1 ms | **43×** |
| 대규모 (~2 KB) | 1.5 ms | 57.7 ms | 127.9 ms | **38×** |

## Serialize 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.04 ms | 2.2 ms | 4.9 ms | **55×** |
| 중규모 (~690 B) | 0.08 ms | 12.6 ms | 28.1 ms | **159×** |
| 대규모 (~2 KB) | 0.17 ms | 30.2 ms | 63.1 ms | **177×** |

## Round-Trip 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.22 ms | 6.0 ms[^1] | 13.7 ms[^1] | **28×** |
| 중규모 (~690 B) | 0.63 ms | 36.8 ms[^1] | 84.2 ms[^1] | **59×** |
| 대규모 (~2 KB) | 1.6 ms | 87.9 ms[^1] | 191.0 ms[^1] | **55×** |

[^1]: PyYAML/ruamel의 라운드트립 시간은 동일 벤치마크의 파싱·직렬화 합계로 추정한 값입니다.

## Rust-Side 벤치마크 (divan)

Rust CodSpeed 레그(`cargo codspeed`, 시뮬레이션 모드)에서 측정한 값입니다. 네이티브 실제 시간이 아닌 계측 시간이며, 픽스처는 `pyrs_yaml_core::bench_inputs`에서 옵니다. "small"은 19 B로 Python 표의 95 B 문서와 다르므로 같은 이름의 행은 직접 비교할 수 없습니다.

| 작업 | 시간 |
|-----------|------|
| Parse (소규모) | 85.6 µs |
| Parse (중규모) | 277.4 µs |
| Parse (대규모) | 840.6 µs |
| Parse (anchor) | 254.3 µs |
| Parse (comment) | 164.9 µs |
| Parse (block scalar) | 123.8 µs |
| Serialize (소규모) | 7.9 µs |
| Serialize (중규모) | 32.2 µs |
| Serialize (대규모) | 76.1 µs |
| Serialize (anchor) | 27.0 µs |
| Serialize (block scalar) | 15.4 µs |
| Round-trip (소규모) | 91.7 µs |
| Round-trip (중규모) | 303.1 µs |
| Round-trip (대규모) | 910.0 µs |

## 주요 결론

1. :material-trending-up: **pyrs-yaml은 PyYAML보다 파싱 21~43×, 직렬화 55~177× 빠릅니다**
2. :material-trending-up: **pyrs-yaml은 루트립 기능을 유지하면서 ruamel.yaml보다 파싱 48~100×, 직렬화 123~371× 빠릅니다**
3. :material-bolt: **Rust 파싱**은 Python 호출 오버헤드가 없으며, 시뮬레이션 레그가 작업별 비용을 분해합니다
4. :material-bolt: **직렬화**는 같은 시뮬레이션 레그에서 항상 파싱 비용보다 낮습니다
5. :material-chart-line: **속도 우위는 모든 문서 크기에서 일관됩니다**

## 참고

- Python 표: 2026-08-17에 기록한 실제 시간 측정치이며 CodSpeed 시뮬레이션 출력이 아닙니다
- CodSpeed CI는 시뮬레이션 모드로 명령 수를 보고하며 상대 추세 추적에 사용합니다
- 배율은 참고용이며 인터프리터, 하드웨어, 문서 크기에 따라 달라집니다
- PyYAML 벤치마크는 `safe_load`/`safe_dump` 사용 (동일한 안전 보장)
- ruamel.yaml 벤치마크는 기본 설정 사용
- Python 픽스처는 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
