---

title: Changelog
lang: ko

## 변경 이력

All notable changes to this project will be documented in this file.

The format is based on [Keep a 변경 이력](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased]

### [0.10.0] - 2026-08-01

#### Added

- **제자리 편집** — 서식 메타데이터를 잃지 않고 파싱된 문서 편집:
    - 경로 API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`, `doc.delete(path)`, `doc.rename(path, new_key)`, JSONPath 스타일 경로(`$.a.b[0]`); 루트 슈가 `doc["key"] = value`와 `del doc["key"]`
    - 노드 API: `doc.node()` / `doc.find(path)`는 `Node` 객체를 반환하며 `set_value` / `append` / `insert` / `delete` / `rename`과 트리 탐색(`parent`, `children`, `walk`, `filter`)을 지원
    - 완전한 메타데이터 보존 — 교체된 스칼라는 주석/앵커/태그/따옴표를 유지; 이름이 변경된 키는 위치와 주석을 유지; 삭제 시 매핑 순서 보존
    - 원자적 편집 — 실패한 작업은 문서(리비전 포함)를 변경하지 않음
    - 지연 소스 재동기화 — `source()` / `to_yaml()` / `reparse()`는 편집 성공 후에만 재직렬화
    - 오래된 노드 감지 — 문서 편집 후 `Node` 접근은 `YamlDocumentError` 발생(`RuntimeWarning` 포함)
    - 새 예외: `YamlEditError`, `YamlPathError`(en/zh-CN/ja-JP/ko-KR i18n 지원)
    - 별칭 인식 편집 — 별칭 자신의 경로 설정은 그 자리를 교체; 별칭을 통한 편집은 `YamlEditError` 발생
- **편집 벤치마크** — `benches/yaml_bench.rs`에 divan 벤치마크 6개 추가(소형~대형 문서의 set/insert/delete)
- **Python 3.13, 3.14, 3.15 지원** — PyO3 `abi3-py38` 휠이 Python 3.8-3.15 커버(GIL 빌드); `abi3t` + `abi3t-py315`는 free-threaded 안정 ABI 제공
- **Free-threaded CPython(GIL 없음) 지원** — `#[pymodule(gil_used = false)]`가 모듈을 free-threaded Python용 스레드 안전으로 선언; `Py_GIL_DISABLED` cfg 플래그로 numpy 게이트(rust-numpy는 free-threaded 미지원 — `--no-default-features`로 free-threaded 빌드에서 numpy feature 비활성화)
- **CI free-threaded 작업** — 새 `test-freethreaded` 워크플로 작업이 Python 3.14t에서 컴파일과 테스트 검증
- **`pyo3-build-config` 빌드 의존성** — `build.rs`를 통해 `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]` 등 컴파일러 플래그 활성화
- **`numpy` 선택 사항화** — `numpy` feature 뒤에 게이트(기본 활성화); `Py_GIL_DISABLED` 하에서 자동 제외

#### Changed

- CI Python 매트릭스 확장: ubuntu, windows, macos에서 3.8-3.14
- 안정 ABI: `abi3-py39` → `abi3-py38`(더 넓은 Python 3.8+ 지원), `abi3t` + `abi3t-py315` 추가(free-threaded 안정 ABI)
- `pyproject.toml` classifiers에 3.13, 3.14, 3.15 항목 추가
- `YamlDocument.source()`가 `str`을 반환하고 제자리 편집 후 지연 재직렬화

#### Fixed

- 순환 문서 명확화: 병합 키(`<<`)는 기본적으로 해석되며 `resolve_merges=False`일 때만 그대로 유지

### [0.3.0] - 2025-07-27

#### Added

- **NumPy ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D), negative numbers, infinity, NaN, empty arrays, and edge cases

#### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative numbers are now quoted during serialization and correctly parsed back as integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)

#### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch

#### Added

- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS × 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker
- Full `.pyi` type stubs with type annotations in `#[pyo3(signature)]`
- `experimental-inspect` feature enabled for `pyo3-introspection` compatibility
- i18n support with `set_language()`, `get_language()`, `list_languages()`, `detect_language()`, `negotiate_language()`
- Markdown frontmatter extraction: `read_markdown()`, `read_markdown_str()`
- JSON ↔ YAML conversion: `from_json()`, `from_dict()`

#### Fixed

- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles
- Exception types now properly exported from inline `#[pymodule]` via `#[pymodule_export]`

#### Changed

- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context
- `#[pymodule]` refactored to inline module for `pyo3-introspection` compatibility
- All `#[pyo3(signature)]` attributes now include Python type annotations

### [0.1.0] - 2025-07-25

#### Added

- Initial release with YAML 1.2 compliance via saphyr-parser
- Custom AST with full metadata (comments, anchors, tags, chomping, scalar styles)
- Round-trip preservation of comments, anchors, tags, and formatting
- PyYAML-compatible API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` conversion functions
- `read_markdown`/`read_markdown_str` for YAML frontmatter extraction
- Block scalars (`|`/`>`) with chomping indicators (`|-`/`|+`/`>-`/`>+`)
- Escape sequences (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 type resolution (null, bool, int, float, infinity, NaN)
- Merge key resolution (`<<: *alias`)
- Complex keys (sequence/mapping as key)
