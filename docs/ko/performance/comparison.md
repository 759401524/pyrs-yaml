---

title: Comparison with Other Libraries
lang: ko

pyrs-yaml을 가장 인기 있는 두 Python YAML 라이브러리와 비교합니다.

## 성능 비교

### Parse 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **0.07 ms** | — |
| PyYAML | 1.83 ms | 26× 느림 |
| ruamel.yaml | 4.26 ms | 61× 느림 |

#### Serialize 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **0.07 ms** | — |
| PyYAML | 2.92 ms | 40× 느림 |
| ruamel.yaml | 6.73 ms | 93× 느림 |

#### Round-Trip 속도 (대규모 YAML, ~2 KB)

| 라이브러리 | 시간 | 속도 |
|---------|------|---------|
| **pyrs-yaml** | **0.07 ms** | — |
| PyYAML | 2.90 ms | 41× 느림 |
| ruamel.yaml | 6.57 ms | 91× 느림 |

## 기능 비교

| 기능 | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2 준수** | ✅ | ✅ | ✅ |
| **주석 (standalone)** | ✅ | ❌ | ✅ |
| **주석 (inline)** | ✅ | ❌ | ✅ |
| **Anchor/alias** | ✅ | ❌ | ✅ |
| **Tag (explicit)** | ✅ | ❌ | ✅ |
| **Block scalar** | ✅ | ✅ | ✅ |
| **Flow collection** | ✅ | ✅ | ✅ |
| **Merge key (<<)** | ✅ | ❌ | ✅ |
| **복잡한 키** | ✅ | ✅ | ✅ |
| **Round-trip 보존** | ✅ | ❌ | ✅ |
| **Python 바인딩** | ✅ | ✅ | ✅ |
| **ABI3 (py3.9+)** | ✅ | ❌ | ❌ |
| **Type stubs (.pyi)** | ✅ | ✅ | ❌ |
| **i18n 오류 메시지** | ✅ | ❌ | ❌ |
| **Rust 백엔드** | ✅ | ❌ | ❌ |
| **성능** | 🚀 가장 빠름 | 🐌 느림 | 🐌 느림 |

## 요약

### pyrs-yaml을 선택할 때

- **성능이 중요합니다** — PyYAML보다 25~40× 빠름
- **Round-trip 보존이 중요합니다** — 주석, anchor, tag 보존
- **PyYAML 호환성을 원합니다** — 즉시 교체 가능한 API
- **Type hints가 필요합니다** — 완전한 `.pyi` stubs
- **단일 wheel을 원합니다** — ABI3로 Python 3.9~3.13 지원

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
