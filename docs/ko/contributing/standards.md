---

title: 코드 표준
lang: ko

## 코드 표준

pyrs-yaml에 기여할 때 다음 표준을 따르세요.

### Rust

#### 스타일

- 커밋 전에 `cargo fmt` 사용
- [Rust API 가이드라인](https://rust-lang.github.io/api-guidelines/) 준수
- `#[allow(unused_imports)]`는 필요한 경우에만 사용 (테스트, 피처 플래그)

#### 오류 처리

- **비즈니스 로직에서 `.unwrap()` 또는 `.expect()`를 절대 사용하지 마세요**
- 모든 Rust 오류를 Python 예외로 변환
- 실패할 수 있는 함수에 `PyResult<T>` 사용
- 특정 오류를 특정 Python 예외 타입으로 매핑

```rust
// 올바름
let content = std::fs::read_to_string(path)
    .map_err(|e| YamlParseError::new_err(format_i18n_error("file-read-error", ...)))?;

// 잘못됨
let content = std::fs::read_to_string(path).unwrap();
```

#### 문서

- 모든 공개 함수에 `///` doc 주석이 필요
- `# Arguments`, `# Returns`, `# Errors`, `# Examples` 섹션 포함
- doc 주석은 영어로 작성 (Rust 관례)
- 내부 함수의 doc 주석은 중국어 허용

```rust
/// YAML 문자열을 CustomNode AST로 파싱합니다.
///
/// # Arguments
/// * `yaml` — YAML 콘텐츠 문자열
///
/// # Returns
/// 파싱된 AST 루트 노드, 실패 시 `Err(String)`
///
/// # Errors
/// `"YAML parse error: line N, col M: <msg>"` 형식의 `Err(String)` 반환
///
/// # Examples
/// ```
/// let ast = pyrs_yaml::parser::parse("key: value").unwrap();
/// ```
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
```

#### PyO3 시그니처 주석

모든 `#[pyfunction]`와 `#[pymethods]`는 `#[pyo3(signature = "...")]`로 타입을 주석해야 합니다：

```rust
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
fn parse(...) -> YamlDocument { ... }
```

#### GIL 관리

- 부하가 높은 계산 중 `py.detach()` 또는 `py.allow_threads()`를 사용하여 GIL 해제
- 파일 I/O 또는 파싱 중 GIL을 절대 보유하지 마세요

```rust
// 올바름
let ast = py.detach(|| {
    parser::parse_with_options(&yaml_str, resolve_merges)
        .map_err(|e| YamlParseError::new_err(...))?
})?;

// 잘못됨 — 파싱 중 GIL 보유
let ast = parser::parse_with_options(&yaml_str, resolve_merges)?;
```

#### Clippy

`cargo clippy -- -D warnings` 실행 — 모든 경고를 오류로 취급.

### Python

#### 스타일

- [PEP 8](https://peps.python.org/pep-0008/) 준수
- 모든 곳에서 타입 힌트 사용
- docstring은 Google 스타일
- 코드 검사 설정은 `ruff.toml`에 있음（`ruff check` 실행）

```python
def parse(yaml: str, resolve_merges: bool = True, schema: str = "core") -> YamlDocument:
    """YAML 문자열을 YamlDocument로 파싱합니다.

    Args:
        yaml: YAML 콘텐츠를 포함하는 문자열
        resolve_merges: 병합 키를 해석할지 여부 (기본값: True)
        schema: YAML 스키마 구성 ("core", "json", "failsafe", "yaml11")

    Returns:
        파싱된 YAML를 포함하는 YamlDocument

    Raises:
        YamlParseError: YAML가 유효하지 않은 경우
    """
```

#### 테스트

- 코드 전에 테스트 작성 (TDD)
- 필요 시 `uv run --frozen pytest`와 픽스처 사용
- 엣지 케이스 테스트: 빈 입력, 특수 문자, 대용량 문서
- 순환 보존 단언 포함
- Pytest 설정은 `pytest.ini`에 있음（asyncio_mode = auto, 사용자 정의 마커）

### Git

- 커밋 메시지는 명령형으로: "Add feature X" (X가 아님 "Added feature X")
- 커밋당 하나의 논리적 변경
- 커밋 전에 `cargo test`와 `uv run --frozen pytest tests/` 실행

### 문서

- 동작을 변경할 때 문서 업데이트
- 복사하여 붙여넣고 실행할 수 있는 코드 예시 사용
- 예시는 간결하지만 완전하게 유지
