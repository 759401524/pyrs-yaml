---
title: 테스트 실행
description: pyrs-yaml의 Rust 및 Python 테스트 실행 방법 — 테스트 커버리지, CI 테스트, 새 테스트 추가
tags:
  - docs
status: new
---

pyrs-yaml는 Rust 유닛 테스트와 Python 통합 테스트를 모두 보유하고 있습니다.

## Rust 테스트

```bash title="Rust 테스트 실행"
# 모든 Rust 테스트를 nextest로 실행 (권장)
cargo nextest run --all

# 모든 Rust 테스트를 cargo test로 실행
cargo test --all

# 순수 Rust 코어 테스트 실행 (Python 런타임 불필요)
cargo test --all --no-default-features

# 출력과 함께 실행
cargo test --all -- --nocapture
```

### 테스트 커버리지

- **`crates/pyrs-yaml-core/src/ast.rs`** — 노드 구성, 메타데이터, 동등성
- **`crates/pyrs-yaml-core/src/parser/`** — 다양한 YAML 구조 파싱
- **`crates/pyrs-yaml-core/src/serializer.rs`** — 직렬화 순환 보존
- **`crates/pyrs-yaml-core/src/editing/`** — 편집 프리미티브 (navigate, region, dirty, metadata)
- **`crates/pyrs-yaml-core/src/integration/`** — YAML Test Suite 통합
- **`crates/pyrs-yaml/src/fidelity.rs`** — 속성 기반 퍼즈 테스트

## Python 테스트

```bash title="Python 테스트 실행"
# 모든 Python 테스트 실행
uv run pytest tests/ -v

# 특정 테스트 파일 실행
uv run pytest tests/test_edit.py -v

# 특정 테스트 클래스 실행
uv run pytest tests/test_node_api.py::TestDocWalk -v

# 커버리지와 함께 실행
uv run pytest tests/ -v --cov=pyrs_yaml

# 규정 준수 스위트 실행
uv run pytest tests/test_yaml_suite.py -v

# 벤치마크 실행
uv run pytest tests/ --codspeed
```

### 테스트 파일

| 파일 | 커버리지 |
|------|---------|
| `test_parse.py` | 파싱, 데이터 타입, 특수 문자 |
| `test_serialize.py` | 직렬화, 순환 보존 |
| `test_edge_cases.py` | 엣지 케이스, 오류 처리 |
| `test_errors.py` | 사용자 정의 예외 타입, 파일 I/O |
| `test_features.py` | Markdown Front Matter, from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 변환 |
| `test_tabs.py` | 탭 처리 |
| `test_yaml_suite.py` | YAML Test Suite 통합 |
| `test_performance.py` | 성능 건전성 검사 |
| **`test_numpy.py`** | **NumPy ndarray 직렬화 (0차원~N차원, 모든 dtype)** |

## Maturin 빌드

```bash title="빌드 및 설치"
# 빌드 및 설치 (모노레포 manifest-path 사용)
uv run maturin develop --release

# .pyi 파일용 스텁 생성
uv run maturin build --release --generate-stubs
```

## CI 테스트

GitHub Actions가 모든 푸시와 PR에서 실행:

- **Rust**: `cargo nextest run --all`, `cargo clippy --all -- -D warnings`, `cargo fmt --check`
- **Python**: 4개 Python 버전 × 3개 OS에서 `uv run pytest tests/`
- **Maturin**: 각 Python 버전용 wheel 빌드 (`crates/pyrs-yaml/Cargo.toml` 경로 사용)

## 새 테스트 추가

### Rust 테스트

```rust title="Rust 테스트 템플릿"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // 테스트를 여기에 작성
    }
}
```

#### Python 테스트

```python title="Python 테스트 템플릿"
import pyrs_yaml
import pytest


class TestNewFeature:
    def test_basic(self):
        result = pyrs_yaml.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # 엣지 케이스 테스트
        pass
```

## 테스트 카테고리

- **유닛 테스트** — 개별 함수, 작은 입력
- **통합 테스트** — 완전한 파싱 → 직렬화 순환 보존
- **엣지 케이스 테스트** — 특수 문자, 빈 입력, 잘못된 형식의 YAML
- **성능 테스트** — 건전성 검사 (벤치마크 아님)
- **YAML Test Suite** — YAML 호환성을 위한 외부 테스트 스위트

## YAML Test Suite 알려진 차이점

스위트 통과율은 **95%**로 제한됩니다 (`test_compliance_report` 참조). 소수의 케이스는 의존적으로 추구하지 않습니다. 이들을 거부하는 것이 사양에 맞으며 참조 파서(특히 PyYAML/libyaml)와 일치하기 때문입니다:

| ID | 입력 | 차이점으로 수락된 이유 |
|:---|:-----|:----------------------|
| `ZYU8` | `%YAML 1.1 1.2` | 버전 지시어 뒤에 오는 콘텐츠는 YAML 1.2 문법(`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`)에 따라 **유효하지 않습니다**. PyYAML도 거부합니다. 스위트 자체의 참고 사항에서 이러한 지시어 변형은 "전혀 유용하게 유효하지 않으며" 이를 지원하는 것이 권장되지 않습니다. |

다른 모든 스위트 케이스는 통과합니다 (현재 405/406 = 99.75%).
