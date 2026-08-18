---
title: Benchmarks
description: pyrs-yaml의 성능 벤치마크 — 파싱, 직렬화, 순환 성능, Rust 측 divan 벤치마크
tags:
  - docs
status: new
---

!!! note "벤치마크 환경"
    벤치마크는 CodSpeed CI(`pytest-codspeed`, WallTime 모드)에서 측정된 결과입니다. 절대 시간은 환경에 따라 다를 수 있습니다.

pyrs-yaml의 성능 벤치마크 (CodSpeed CI, `pytest-codspeed` WallTime 모드).

## Methodology

- **도구:** divan (Rust) + `pytest-codspeed` (Python)
- **반복 횟수:** WallTime 모드, 적응형 샘플링 (Python), 100회 이상 샘플 (Rust)
- **지표:** 밀리초 단위 중앙값 시간 (Python), 마이크로초 단위 평균 시간 (Rust)

## Parse 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.18 ms | 3.8 ms | 8.8 ms | **21×** |
| 중규모 (~500 B) | 0.56 ms | 24.2 ms | 56.1 ms | **43×** |
| 대규모 (~2 KB) | 1.5 ms | 57.7 ms | 127.9 ms | **38×** |

## Serialize 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.04 ms | 2.2 ms | 4.9 ms | **55×** |
| 중규모 (~500 B) | 0.08 ms | 12.6 ms | 28.1 ms | **159×** |
| 대규모 (~2 KB) | 0.17 ms | 30.2 ms | 63.1 ms | **177×** |

## Round-Trip 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.22 ms | 6.0 ms[^1] | 13.7 ms[^1] | **28×** |
| 중규모 (~500 B) | 0.63 ms | 36.8 ms[^1] | 84.2 ms[^1] | **59×** |
| 대규모 (~2 KB) | 1.6 ms | 87.9 ms[^1] | 191.0 ms[^1] | **55×** |

[^1]: PyYAML/ruamel의 라운드트립 시간은 동일 벤치마크의 파싱·직렬화 합계로 추정한 값입니다.

## Rust-Side 벤치마크 (divan)

Rust 수준에서 측정 (Python 오버헤드 없음):

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
3. :material-bolt: **Rust 파싱은 매우 빠릅니다** — 소규모 문서는 약 86 µs에 파싱됩니다
4. :material-bolt: **직렬화는 모든 크기에서 빠릅니다** — 소규모 문서는 약 8 µs에 직렬화됩니다
5. :material-chart-line: **속도 우위는 모든 문서 크기에서 일관됩니다**

## 참고

- 벤치마크는 CodSpeed CI에서 측정; 절대 시간은 환경에 따라 다를 수 있습니다
- 상대 속도 (×N)는 하드웨어에 일관되게 유지됩니다
- PyYAML 벤치마크는 `safe_load`/`safe_dump` 사용 (동일한 안전 보장)
- ruamel.yaml 벤치마크는 기본 설정 사용
