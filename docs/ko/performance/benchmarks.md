---
title: Benchmarks
description: pyrs-yaml의 성능 벤치마크 — 파싱, 직렬화, 순환 성능, Rust 측 divan 벤치마크
tags:
  - docs
status: new
---

!!! note "벤치마크 환경"
    CodSpeed CI는 **시뮬레이션** 모드(`mode: simulation`)로 실행되어 실제 시간 대신 명령 수를 측정합니다. 아래 실제 시간 값은 `scripts/bench_snapshot.py`가 생성합니다. 각 케이스를 배치 단위로 측정해 9개 배치 중 가장 빠른 것을 기록하며, 측정값은 측정 당시의 인터프리터, 라이브러리 버전, 픽스처 크기와 함께 `.ci/perf-snapshot.json`에 커밋됩니다. 배율은 참고용이며 인터프리터, 하드웨어, 문서에 따라 달라집니다.

pyrs-yaml의 성능 벤치마크. CodSpeed의 시뮬레이션 수치는 상대 추세 추적용이며, 아래 실제 시간 표는 단일 머신에서 배치로 측정한 값으로 `scripts/bench_snapshot.py`로 재현할 수 있습니다.

## Methodology

- **도구:** CI에서는 `codspeed-divan-compat`(Rust)와 `pytest-codspeed`(Python)를 사용하며, 실제 시간 표는 `scripts/bench_snapshot.py`가 생성
- **반복:** 각 케이스를 9개 배치로 측정하고 배치마다 0.1초 이상이 되도록 조정한 뒤 가장 빠른 배치를 보고
- **지표:** 마이크로초 (Python 실제 시간과 Rust 계측 값). CodSpeed는 시간이 아닌 명령 수를 보고

## Parse 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 4.6 µs | 134 µs | 311 µs | **29×** |
| 중규모 (~690 B) | 24 µs | 839 µs | 2.0 ms | **34×** |
| 대규모 (~2 KB) | 67 µs | 2.0 ms | 5.0 ms | **30×** |

## Serialize 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.37 µs | 83 µs | 201 µs | **223×** |
| 중규모 (~690 B) | 1.3 µs | 481 µs | 1.1 ms | **368×** |
| 대규모 (~2 KB) | 4.5 µs | 1.2 ms | 2.6 ms | **265×** |

## Round-Trip 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 5.3 µs | 238 µs | 545 µs | **45×** |
| 중규모 (~690 B) | 26 µs | 1.5 ms | 3.3 ms | **59×** |
| 대규모 (~2 KB) | 70 µs | 3.5 ms | 7.8 ms | **50×** |

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

1. :material-trending-up: **pyrs-yaml은 PyYAML보다 파싱 29~34×, 직렬화 223~368× 빠릅니다**
2. :material-trending-up: **pyrs-yaml은 루트립 기능을 유지하면서 ruamel.yaml보다 파싱 68~81×, 직렬화 541~876× 빠릅니다**
3. :material-trending-up: **루트립은 PyYAML보다 45~59×, ruamel.yaml보다 102~125× 빠릅니다**
4. :material-bolt: **Rust 파싱**은 Python 호출 오버헤드가 없으며, 시뮬레이션 레그가 작업별 비용을 분해합니다
5. :material-bolt: **직렬화**는 같은 시뮬레이션 레그에서 항상 파싱 비용보다 낮습니다
6. :material-chart-line: **속도 우위는 모든 문서 크기에서 일관됩니다**

## 참고

- Python 표: 2026-10-20에 `scripts/bench_snapshot.py`가 기록한 배치 측정 실제 시간 값이며, 스냅샷(`.ci/perf-snapshot.json`)에 환경, 라이브러리 버전, 픽스처 크기를 기록
- CodSpeed CI는 시뮬레이션 모드로 명령 수를 보고하며 상대 추세 추적에 사용합니다
- 배율은 참고용이며 인터프리터, 하드웨어, 문서 크기에 따라 달라집니다
- PyYAML 벤치마크는 `safe_load`/`safe_dump` 사용 (동일한 안전 보장)
- ruamel.yaml 벤치마크는 기본 설정 사용
- Python 픽스처는 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
