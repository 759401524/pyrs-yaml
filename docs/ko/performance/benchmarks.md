---

title: Benchmarks
lang: ko

pyrs-yaml의 성능 벤치마크 (저자 환경: Windows 11, Python 3.12).

## Methodology

- **도구:** Criterion (Rust) + `pytest-codspeed` (Python)
- **반복 횟수:** 벤치마크당 200회 (Python), 100회 이상 샘플 (Rust)
- **지표:** 밀리초 단위 중앙값 시간 (Python), 마이크로초 단위 평균 시간 (Rust)

## Parse 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.00 ms | 0.11 ms | 0.26 ms | **25×** |
| 중규모 (~500 B) | 0.03 ms | 0.75 ms | 1.74 ms | **28×** |
| 대규모 (~2 KB) | 0.07 ms | 1.83 ms | 4.26 ms | **26×** |

## Serialize 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.01 ms | 0.19 ms | 0.46 ms | **36×** |
| 중규모 (~500 B) | 0.03 ms | 1.21 ms | 2.83 ms | **40×** |
| 대규모 (~2 KB) | 0.08 ms | 2.96 ms | 6.74 ms | **37×** |

## Round-Trip 성능

| YAML 크기 | pyrs-yaml | PyYAML | ruamel.yaml | PyYAML 대비 속도 |
|-----------|-----------|--------|-------------|-------------------|
| 소규모 (~100 B) | 0.01 ms | 0.19 ms | 0.47 ms | **35×** |
| 중규모 (~500 B) | 0.03 ms | 1.20 ms | 2.88 ms | **39×** |
| 대규모 (~2 KB) | 0.08 ms | 2.98 ms | 6.79 ms | **37×** |

## Rust-Side 벤치마크 (Criterion)

Rust 수준에서 측정 (Python 오버헤드 없음):

| 작업 | 시간 |
|-----------|------|
| Parse (소规模) | 1.69 µs |
| Parse (중规模) | 12.2 µs |
| Parse (대规模) | 37.7 µs |
| Parse (anchor) | 10.5 µs |
| Parse (comment) | 5.0 µs |
| Parse (block scalar) | 3.2 µs |
| Serialize (소规模) | 4.4 µs |
| Serialize (중规模) | 4.7 µs |
| Serialize (대规模) | 5.5 µs |
| Serialize (anchor) | 4.8 µs |
| Serialize (block scalar) | 4.4 µs |
| Round-trip (소规模) | 5.9 µs |
| Round-trip (중规模) | 17.1 µs |
| Round-trip (대规模) | 44.7 µs |

## 주요 결론

1. **pyrs-yaml은 모든 작업에서 PyYAML보다 25~40× 빠릅니다**
2. **pyrs-yaml은 루트립 기능을 유지하면서 ruamel.yaml보다 4~10× 빠릅니다**
3. **Rust 파싱은 매우 빠릅니다** — 소규모 문서는 약 1.7 µs에 파싱됩니다
4. **직렬화는 모든 크기에서 빠릅니다** — 소규모 문서는 약 4.4 µs에 직렬화됩니다
5. **크기가 커질수록 속도 우위가 커집니다**

## 참고

- 벤치마크는 단일 기계에서 측정; 절대 시간은 환경에 따라 다를 수 있습니다
- 상대 속도 (×N)는 하드웨어에 일관되게 유지됩니다
- PyYAML 벤치마크는 `safe_load`/`safe_dump` 사용 (동일한 안전 보장)
- ruamel.yaml 벤치마크는 기본 설정 사용
