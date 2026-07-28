---

title: 테스트 실행
lang: ko

## 테스트 실행

pyrs-yaml는 Rust 유닛 테스트와 Python 통합 테스트를 모두 보유하고 있습니다.

### Rust 테스트

```bash
# 모든 Rust 테스트 실행
cargo test

# 특정 모듈의 테스트 실행
cargo test ast
cargo test parser
cargo test serializer

# 출력과 함께 실행
cargo test -- --nocapture

# 통합 테스트만 실행
cargo test --test integration
```

#### 테스트 커버리지

- **`src/ast.rs`** — 노드 구성, 메타데이터, 동등성
- **`src/parser/`** — 다양한 YAML 구조 파싱
- **`src/serializer.rs`** — 직렬화 순환 보존
- **`src/integration/`** — YAML Test Suite 통합

### Python 테스트

```bash
# 모든 Python 테스트 실행
pytest tests/

# 상세 출력으로 실행
pytest tests/ -v

# 특정 테스트 파일 실행
pytest tests/test_parse.py

# 패턴과 일치하는 테스트 실행
pytest tests/ -k "comment"

# 커버리지와 함께 실행
pytest tests/ --cov=pyrs_yaml --cov-report=term-missing

# 벤치마크 실행
pytest tests/ --benchmark-only --benchmark-json=results.json
```

#### 테스트 파일

| 파일 | 커버리지 |
|------|---------|
| `test_parse.py` | 파싱, 데이터 타입, 특수 문자 |
| `test_serialize.py` | 직렬화, 순환 보존 |
| `test_edge_cases.py` | 엣지 케이스, 오류 처리 |
| `test_errors.py` | 사용자 정의 예외 타입, 파일 I/O |
| `test_features.py` | Markdown 프론트메터, from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 변환 |
| `test_tabs.py` | 탭 처리 |
| `test_yaml_suite.py` | YAML Test Suite 통합 |
| `test_performance.py` | 성능 건전성 검사 |
| **`test_numpy.py`** | **NumPy ndarray 직렬화 (0차원~N차원, 모든 dtype)** |

### CI 테스트

GitHub Actions가 모든 푸시와 PR에서 실행:

- **Rust**: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- **Python**: 4개 Python 버전 × 3개 OS에서 `pytest tests/`
- **Maturin**: 각 Python 버전용 wheel 빌드

### 새 테스트 추가

#### Rust 테스트

```rust
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

```python
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

### 테스트 카테고리

- **유닛 테스트** — 개별 함수, 작은 입력
- **통합 테스트** — 완전한 파싱 → 직렬화 순환 보존
- **엣지 케이스 테스트** — 특수 문자, 빈 입력, 잘못된 형식의 YAML
- **성능 테스트** — 건전성 검사 (벤치마크 아님)
- **YAML Test Suite** — YAML 호환성을 위한 외부 테스트 스위트
