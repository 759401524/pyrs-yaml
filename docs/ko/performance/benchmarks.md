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

<!-- bench: layout=sizes format=yaml operation=parse sizes=small,medium,large baseline=PyYAML -->
| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML 대비 속도 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 소규모 (~100 B) | 4.4 µs | 151 µs | 347 µs | 1.9 µs | 5.7 µs | **35×** |
| 중규모 (~690 B) | 27 µs | 976 µs | 2.3 ms | 10 µs | 33 µs | **37×** |
| 대규모 (~2 KB) | 72 µs | 2.3 ms | 5.4 ms | 27 µs | 86 µs | **32×** |

## Serialize 성능

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML 대비 속도 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 소규모 (~100 B) | 0.42 µs | 86 µs | 227 µs | 0.97 µs | 3.1 µs | **204×** |
| 중규모 (~690 B) | 1.6 µs | 568 µs | 1.4 ms | 6.1 µs | 17 µs | **347×** |
| 대규모 (~2 KB) | 5.4 µs | 1.4 ms | 3.3 ms | 17 µs | 42 µs | **261×** |

## Round-Trip 성능

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML 대비 속도 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 소규모 (~100 B) | 5.7 µs | 265 µs | 619 µs | 3.1 µs | 9.7 µs | **47×** |
| 중규모 (~690 B) | 30 µs | 1.7 ms | 3.6 ms | 17 µs | 51 µs | **55×** |
| 대규모 (~2 KB) | 82 µs | 3.9 ms | 9.1 ms | 48 µs | 135 µs | **48×** |

### JSON

같은 JSON 텍스트를 세 리더로 읽습니다. 큰 문서에서는 `orjson`이 더 빠릅니다. 이 줄은 보기 좋으려고 싣는 것이 아니라 측정한 값이라 싣습니다.

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| 문서 | pyrs-yaml | json | orjson | json 대비 속도 |
| --- | ---: | ---: | ---: | ---: |
| 설정 (~123 B) | 0.56 µs | 1.3 µs | 0.45 µs | **2.3×** |
| 큰 (~1.9 KB) | 7.8 µs | 9.8 µs | 4.5 µs | **1.3×** |

### TOML

TOML 텍스트를 `pyrs_yaml.load_toml`, 표준 라이브러리 `tomllib`, `toml-rs` 바인딩인 `rtoml`에 각각 통합니다. `rtoml`은 사전(딕셔너리)으로 바로 디코딩하므로 두 픽스처에서 여전히 약 1.5× 빠르게 읽습니다. 문제의 비용이던 노드 모델은 이제 내지 않습니다 — `load_toml`이 스캐너의 테이블 트리를 곧바로 Python 조립으로 옮깁니다 — 남는 격차는 그 트리입니다. 같은 기기 측정에서는 탐색과 트리 구성만으로 `rtoml` 전체 해석과 거의 같은 값을 쓰므로, 다음 목표는 트리를 아예 만들지 않는 것입니다.

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=rtoml -->
| 문서 | pyrs-yaml | tomllib | rtoml | rtoml 대비 속도 |
| --- | ---: | ---: | ---: | ---: |
| 핫 (~1.8 KB) | 29 µs | 212 µs | 20 µs | **0.7×** |
| 설정 (~2.1 KB) | 45 µs | 358 µs | 31 µs | **0.7×** |

### JSON5

작은 JSON5 문서 하나를 `pyjson5`와 겨룹니다. 독립적인 JSON5 리더는 이것이 유일합니다. 여기서는 우리가 느리고, 그래서 JSON5 읽기는 남은 최적화 대상입니다.

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| 문서 | pyrs-yaml | pyjson5 | pyjson5 대비 속도 |
| --- | ---: | ---: | ---: |
| 설정 (~83 B) | 2.0 µs | 0.82 µs | **0.4×** |

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

1. :material-trending-up: **pyrs-yaml은 PyYAML보다 파싱 32~37×, 직렬화 204~347× 빠릅니다**
2. :material-trending-up: **pyrs-yaml은 루트립 기능을 유지하면서 ruamel.yaml보다 파싱 75~85×, 직렬화 540~832× 빠릅니다**
3. :material-trending-up: **루트립은 PyYAML보다 47~55×, ruamel.yaml보다 109~119× 빠릅니다**
4. :material-scale-balance: **Rust 기반 상대는 결과를 둘로 나눕니다** — `yaml-rs`는 루트립 트리가 아닌 일반 객체를 반환해 파싱 2.3~2.7×, 루트립 1.7~1.8× 빠르고, 대신 pyrs-yaml의 직렬화가 2.3~3.7× 빠릅니다
5. :material-bolt: **Rust 파싱**은 Python 호출 오버헤드가 없으며, 시뮬레이션 레그가 작업별 비용을 분해합니다
6. :material-bolt: **직렬화**는 같은 시뮬레이션 레그에서 항상 파싱 비용보다 낮습니다
7. :material-chart-line: **속도 우위는 모든 문서 크기에서 일관됩니다**

## 참고

- Python 표: 2026-10-25에 `scripts/bench_snapshot.py`가 기록한 배치 측정 실제 시간 값이며, 스냅샷(`.ci/perf-snapshot.json`)에 환경, 라이브러리 버전, 픽스처 크기를 기록
- CodSpeed CI는 시뮬레이션 모드로 명령 수를 보고하며 상대 추세 추적에 사용합니다
- 배율은 참고용이며 인터프리터, 하드웨어, 문서 크기에 따라 달라집니다
- PyYAML 벤치마크는 `safe_load`/`safe_dump` 사용 (동일한 안전 보장)
- ruamel.yaml 벤치마크는 기본 설정 사용
- `yaml-rs`와 `ryaml`이 돌려주는 것은 일반 Python 객체입니다. pyrs-yaml의 행이 만드는 것은 이후 편집이 원본 서식을 보존할 수 있는 문서이므로 같은 일이 아닙니다
- Python 픽스처는 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
