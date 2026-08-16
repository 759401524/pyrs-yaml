---
title: Changelog
description: pyrs-yaml의 모든 중요 변경 사항 — Keep a Changelog 형식, Semantic Versioning 준수
tags:
  - docs
status: new
---

## 변경 이력

이 파일에는 본 프로젝트의 모든 중요 변경 사항이 기록됩니다.

이 형식은 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 기반으로 하며,
이 프로젝트는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 따릅니다.

### [Unreleased]

#### 추가

- **노드 메타데이터 세터/게터** — `Node.comment` / `Node.anchor` / `Node.tag` 읽기 속성과 `set_comment` / `set_anchor` / `set_tag`(및 `remove_*` 계열)를 추가. 별칭 또는 존재하지 않는 경로 편집은 오류가 발생합니다. 인라인 스칼라 값·시퀀스 항목의 독립형 주석은 자체 들여쓰기 행에 출력됩니다(`child:\n  # c\n  val` 및 `- a\n# c\n- b`의 기존 라운드트립 결함 수정).
- **Verbatim 태그** — `set_tag("!<tag:yaml.org,2002:str>")`는 verbatim 태그(빈 핸들)를 생성하며, 소스에서 파싱된 verbatim 태그는 라운드트립 시 보존됩니다: `Tag`의 `Display`는 빈 핸들 태그를 `!<...>`로 감싸 출력하고, `parse_tag`는 `!<...>` 형식을 인식하며, 스트림 이벤트는 `Display`를 통해 태그를 직렬화합니다.
- **스키마 파일 IO 및 목록** — `load_schema(name, path)`는 파일에서 스키마 정의를 읽어 등록하고, `list_schemas()`는 등록된 모든 스키마 이름(내장 `failsafe`/`json`/`core`/`yaml1.1` + 사용자 정의)을 반환합니다.
- **노드 style/format 세터/게터** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping` 읽기 속성과 `set_scalar_style` / `set_flow_style` / `set_chomping` 메서드. ScalarStyle/Chomping이 이제 `Copy`를 derive합니다. 비스칼라 노드는 `None` 반환 / no-op, 별칭 및 존재하지 않는 경로는 오류가 발생합니다.
- **스키마 구조 검증** — 스키마 정의의 `validate` 섹션으로 구조 검사(경로 한정 스칼라 타입, `sequence_of`/`mapping_of` 컨테이너, `required`)를 추가. `validate_against_schema(data, schema_yaml)`는 모든 실패를 나열하며 `YamlValidateError`를 발생시킵니다.
- **`Node.copy()`** — 하위 트리를 문서에서 분리된 독립 Python 값(dict/list/scalar)으로 깊은 복사합니다. `set_value()`로 붙여넣는 데 유용합니다.

#### 변경

- **프리-스레디드 (cp314t) 휠에서 NumPy 재활성화** — cp314t 빌드 인수에서 `--no-default-features` 제거. rust-numpy 0.29는 프리-스레디드 Python을 지원하며, `numpy.ndarray` 직렬화가 프리-스레디드 휠에서 사용 가능합니다(NumPy 설치는 런타임에 자동 감지).

### [v0.14.1] — 2026-08-15

#### 수정

- **백슬래시+제어 문자/비문자를 포함한 단일 인용 스칼라** — 이러한 값은 이중 인용을 사용합니다. 단일 인용은 제어 문자/비문자를 이스케이프할 수 없습니다.
- **비문자 및 BOM 인용** — `needs_quotes` / `needs_double_quoted`가 U+FFFE/U+FFFF/평면 끝 비문자와 U+FEFF(BOM)를 인용 필수로 취급합니다.
- **이중 인용 이스케이프 폭** — U+FFFF 이상의 코드 포인트는 8자리 `\Uxxxxxxxx` 형식으로 출력합니다(4자리 `\u`는 BMP 전용).
- **접힌 plain 스칼라 연속 들여쓰기** — 연속 들여쓰기를 값의 시작 열에서 도출하여 중첩된 시퀀스/매핑 항목의 연속 행이 부모 블록 들여쓰기를 초과하도록 했습니다.
- **멀티바이트 접기 경계** — `wrap_plain_scalar`가 접기 슬라이스를 문자 경계로 내림하여 4바이트 UTF-8이 경계를 가로지를 때 panic을 방지합니다.
- **publish 테스트 요구사항에 `hypothesis`** — `.ci/requirements-test.txt`에 `hypothesis>=6.113.0`을 고정하여 게시 워크플로가 속성 테스트를 실행할 수 있게 했습니다.

#### 추가됨

- **`scripts/fuzz_panics.py`** — dump/parse/edit/멱등성에 걸친 적대적 전략을 사용한 로컬 대규모 Hypothesis fuzz 하네스.

### [v0.14.0] — 2026-08-14

#### 추가됨

- **YAML Schema Language** — 정규식 패턴을 YAML 타입에 매핑하는 사용자 정의
  스키마 정의 가능. `register_schema()`로 등록.
- **인라인 dict 스키마** — `schema` 매개변수에 `dict` 직접 전달 가능.
- **Community Plugins** — `CustomType` 기본 클래스로 사용자 정의 노드 타입 등록.
  `register_type()`으로 등록.
- **내장 플러그인** — `!timestamp`(datetime)와 `!set`이 기본 등록됨.

#### 변경됨

- **스키마 해석이 플러그인 가능하도록** — `SchemaResolver` 트레이트 +
  `Schema` 열거형 + 전역 `SchemaRegistry`. 내장 스키마는 제로 비용 디스패치 유지.
- **`node_to_pyobject`와 `direct_dump`가 `CustomType` 확인** —
  태그 스칼라는 `from_yaml()`로 변환, Python 객체는 `to_yaml()`로 직렬화.

#### 수정

- **따옴표 스칼라는 항상 문자열로 로드** — 암시적 타입 해석은 평문 스칼라에만 적용(YAML 1.2). `safe_load('"true"')`는 문자열 `"true"`를 반환합니다(`True` 아님). 직렬화기는 문서(`to_yaml`) 경로에서도 음수가 올바르게 왕복되도록 유지합니다.
- **홑/겹따옴표 단일 문자 키 왕복** — `'` 또는 `"` 단일 문자 맵 키는 인용 스칼라로 출력되어 파싱 불가 YAML이 되지 않습니다.
- **빈 컬렉션은 `{}`/`[]` 출력** — 빈 매핑/시퀀스 덤프가 재파싱 시 `None`이 되는 빈 문서를 생성하지 않습니다.

#### 변경

- **`get()`은 리터럴 키 전용** — `YamlDocument.get()`은 `.`/`[` 포함 키를 JSONPath로 추정하지 않으며, 항상 최상위 맵 키로 취급합니다(`__getitem__`/`__setitem__`과 일관). 경로 접근은 `find()`/`node()`를 사용하세요.

### [v0.13.0] — 2026-08-10

#### 변경 사항

- **Rust MSRV를 1.96으로 업그레이드하고 edition을 2024로 변경** — 두 crate 모두
  `rust-version = "1.96"` 및 `edition = "2024"`를 선언합니다. CI는
  `build`/`test-freethreaded` 작업을 Rust 1.96으로 고정하여 결정론적인
  wheel 빌드를 보장합니다. 또한 `msrv-check` 작업을 추가하여 MSRV에서
  `cargo check`/`cargo test`를 실행하고 정적 MSRV 드리프트를 방지합니다
  (`rust-lint` 작업은 `stable` 유지). 버전 바트는 PyO3 0.29 자체의 기반
  (rustc 1.83)보다 높게 설정되며, std API 선제 지원(예: `assert_matches!`,
  1.96 안정화)을 목적으로 합니다. `TAG_REGISTRY`(태그 핸들러 관리)가
  `std::sync::LazyLock`로 리팩터링되어 `Mutex<Option<...>>` 간접 계층이
  제거되었습니다.

#### 성능

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — Python→YAML serialization without intermediate `CustomNode` AST.
  Single-pass `direct_dump` replaces the old two-pass `pyobject_to_node` +
  `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on `from_dict`
  (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — when input has no `&` characters, skip `collect_anchors` + anchor
  resolution and use the simpler `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/
  non-boolean first bytes return `Str` immediately, avoiding schema
  resolution overhead for the common case. (#59)
- **granit-parser 마이그레이션** — saphyr-parser를 granit-parser 1.0.1로
  교체하여 네이티브 `Event::Comment` 출력으로 전체 텍스트 `scan_yaml()`
  프리스캔을 제거. parse_small -18%, parse_large -21%,
  roundtrip_large -18%.

#### 수정

- **`float_to_yaml_string` round-trip 수정** — Rust Display가 소수점을
  버릴 때 `.0`을 추가(`42` → `42.0`)하여 float가 int로 바뀌지 않고
  round-trip되도록 함.
- **`count_nodes` 사전 할당 롤백** — 전체 AST 순회 비용이 피한 realloc보다
  커서(serialize_10mb 약 14% 저하) 버퍼 확장은 Vec에 위임.

#### 추가

- **스트림 & frontmatter API에 `max_depth` 추가** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)`
  가 `max_depth`를 허용 (기본값 1000). 스트림 파싱은 이제 코어
  `parse_stream_with_options`를 통해 중첩 깊이 제한을 적용
  (기존에는 스트림 이벤트에 깊이 제한이 없었음).
- **Pydantic 통합** — `dump_pydantic()`은 Pydantic 모델을 YAML 문자열로
  직렬화 (`model_dump(mode='json')` + `safe_dump`); `parse_as()`는
  YAML 문자열을 Pydantic 모델 인스턴스로 파싱. 둘 다 지연 임포트,
  pydantic에 대한 하드 의존성 없음. (#61)

#### 내부

- **Split `py/mod.rs`** — monolithic 1786-line module broken into
  `document.rs` (YamlDocument), `yaml_instance.rs` (YAML class),
  `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` 가드 + `double_quoted_scalar()` 생성자** — `'true'` /
  `'42'` / `'null'` 같은 문자열은 코어 스키마 재파싱 시 오독되지 않도록
  큰따옴표 스칼라로 출력(`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed 벤치마크를 `codspeed-divan-compat`으로 통일** —
  `exclude-allocations`로 할당 노이즈 제거. 크로스 라이브러리 벤치마크를
  `tests/test_benchmark_crosslib.py`로 통합하고 공용 `tests/data/yaml_samples.py`
  픽스처와 스트리밍 커버리지 추가.

### [v0.12.1] — 2026-08-06

#### 추가

- **`set(create_missing=True)`** - missing intermediate mapping keys along
  the edit path are created as nested mappings (e.g. setting `a.b.c` on
  `a: 1` creates `b` and `c`); index segments that miss are still an error,
  and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal
  yielding `Node` objects, avoiding per-node `to_dict()` resolution.
  `walk()` returns all nodes; `scalars()` returns only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate`
  (key_eq, navigate, navigate_mut, normalize_index, mapping_key_index),
  `editing::region` (line helpers, node_is_flow, extend_delete_over_comments,
  nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null
  values, deeply nested, flow collections, mixed types.

#### 변경

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/`
  (pure Rust, no PyO3) and `crates/pyrs-yaml/` (PyO3 bindings). Root
  `Cargo.toml` is now a workspace. Old `src/` directory and `build.rs`
  removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to
  `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy
  duplicate-key detection, `shift_insert` merge prepending, and skipped
  `DocumentEnd` deep-clone for single-document parses cut large-document
  parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event
  text share allocations via `Arc<str>`; AST nodes shrink 8 bytes and
  clones become refcount bumps instead of deep copies.

#### 수정

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip
  previously dropped standalone comments attached to simple-key nodes;
  now preserved (two regression tests).

### [0.11.7] - 2026-08-04

#### 변경

- **stub-build-check replaced with release-guard** - the always-red container
  build (`validate.yml`) that deliberately failed to reproduce the v0.10.0
  `--generate-stubs` failure mode is replaced with three static assertions
  that **pass** when the repo is correct: `grep` guards `publish.yml` against
  `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked,
  and `test -f` checks `py.typed` exists. The job now gives green CI on
  correct state, red only on regression.

#### 추가

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free-
  threaded support status (PyO3/rust-numpy#476) as a dependency for re-enabling
  ndarray serialization on cp314t wheels when the Rust binding matures.

### [0.11.6] - 2026-08-04

#### 변경

- **Free-threaded (cp314t) wheels are now numpy-free** - built with
  `--no-default-features`, so rust-numpy is excluded entirely (smaller
  binary, no runtime probe). `safe_dump` on a `numpy.ndarray` raises
  `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15)
  keep full ndarray serialization.

#### 추가

- **Free-threaded CI validation** - `test-freethreaded` job now builds
  and tests with `--no-default-features`, matching the shipped
  free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

### [0.11.5] - 2026-08-04

#### 변경

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 / yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and `tests/test_strictness_audit.py`.

#### 추가

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current rejection/acceptance behavior (both directions), so future parser changes cannot silently regress strictness or over-reject.

### [0.11.4] - 2026-08-04

#### 수정

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is one tab, fixing tab-encoded suite cases

#### 변경

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2 grammar, matches PyYAML/libyaml)

### [0.11.3] - 2026-08-03

#### 추가

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### 변경

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### 수정

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures before Release

### [0.11.2] - 2026-08-03

#### 추가

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: O(앵커 + 청크) 메모리의 지연 이벤트 반복자

#### 성능

- **파싱 시 스플라이스 자격 계산 안 함** — O(문서) 레이아웃 검사가 첫 편집 시 `YamlDocument.splice_checked`를 통해 지연 실행되어 v0.11.0 회귀 복원: parse_comments -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% 모두 v0.10.0 수준으로 복귀
- **선형 커서 레이아웃 검사** — 사전 계산된 줄 오프셋에 대한 노드별 이진 탐색 대체 (단조 소스 순서 순회)

#### 변경

- `parse_with_options`가 `CustomNode`를 반환 (기존 `(CustomNode, bool)`); 스플라이스 자격은 이제 `YamlDocument` 내부에 있으며 요청 시 계산

### [0.11.0] - 2026-08-02

#### 추가

- **Surgical Serialization** — 모든 AST 노드의 바이트 수준 소스 스팬 추적; 세그먼트 기반 스플라이스 — 편집은 접촉 영역만 재생성, 미접촉 텍스트는 바이트 복사
- 속성 테스트 (proptest, 새 개발 의존성)
- 10MB 편집-플러시 벤치마크 (divan)

#### 변경

- `flush_source`가 세그먼트 스플라이스 사용; 플로우 스타일 영역, 비기본 레이아웃 문서, 병합 키, CRLF/BOM 문서, materialize 후 (단일 버스트 모델) 에서 전체 직렬화로 폴백
- 스플라이스 편집이 `---`/`...`/지시자 마커 라인을 미변경 바이트로 보존 (전체 직렬화는 이전에 이를 제거 — 의도적인 동작 차이)

### [0.10.0] - 2026-08-01

#### 추가

- **제자리 편집** — 서식 메타데이터를 잃지 않고 파싱된 문서 편집:
    - 경로 API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`, `doc.delete(path)`, `doc.rename(path, new_key)`, JSONPath 스타일 경로(`$.a.b[0]`); 루트 슈가 `doc["key"] = value`와 `del doc["key"]`
    - 노드 API: `doc.node()` / `doc.find(path)`는 `Node` 객체를 반환하며 `set_value` / `append` / `insert` / `delete` / `rename`과 트리 탐색(`parent`, `children`, `walk`, `filter`)을 지원
    - 완전한 메타데이터 보존 — 교체된 스칼라는 주석/앵커/태그/따옴표를 유지; 이름이 변경된 키는 위치와 주소를 유지; 삭제 시 매핑 순서 보존
    - 원자적 편집 — 실패한 작업은 문서 (리비전 포함) 를 변경하지 않음
    - 지연 소스 재동기화 — `source()` / `to_yaml()` / `reparse()`는 편집 성공 후에만 재직렬화
    - 오래된 노드 감지 — 문서 편집 후 `Node` 접근은 `YamlDocumentError` 발생 (`RuntimeWarning` 포함)
    - 새 예외: `YamlEditError`, `YamlPathError` (en/zh-CN/ja-JP/ko-KR i18n 지원)
    - 별칭 인식 편집 — 별칭 자신의 경로 설정은 그 자리를 교체; 별칭을 통한 편집은 `YamlEditError` 발생
- **편집 벤치마크** — `benches/yaml_bench.rs`에 divan 벤치마크 6개 추가 (소형~대형 문서의 set/insert/delete)

#### 변경

- `YamlDocument.source()`가 `str`을 반환하고 제자리 편집 후 지연 재직렬화

### [0.9.0] - 2026-08-01

#### 추가

- **Python 3.13, 3.14, 3.15 지원** — PyO3 `abi3-py38` 휠이 Python 3.8-3.15 커버 (GIL 빌드); `abi3t` + `abi3t-py315`는 free-threaded 안정 ABI 제공
- **Free-threaded CPython (GIL 없음) 지원** — `#[pymodule(gil_used = false)]`가 모듈을 free-threaded Python용 스레드 안전으로 선언; `Py_GIL_DISABLED` cfg 플래그로 numpy 게이트 (rust-numpy는 free-threaded 미지원 — `--no-default-features`로 free-threaded 빌드에서 numpy feature 비활성화)
- **CI free-threaded 작업** — 새 `test-freethreaded` 워크플로 작업이 Python 3.14t에서 컴파일과 테스트 검증
- **`pyo3-build-config` 빌드 의존성** — `build.rs`를 통해 `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]` 등 컴파일러 플래그 활성화
- **`numpy` 선택 사항화** — `numpy` feature 뒤에 게이트 (기본 활성화); `Py_GIL_DISABLED` 하에서 자동 제외
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`, `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs` 모두 이 플래그를 받습니다; 중복 매핑 키는 기본적으로 `YamlDuplicateKeyError`를 발생시키며, 허용 시 `last value wins`
- **`SerializeOptions` 확장** — `doc.to_yaml_with_options()`에 `width` (줄 바꿈, 0 = 끄기), `indent_mapping`, `indent_sequence`, `indent_offset` 추가 (기존 `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth`와 함께)
- **태그 핸들러 레지스트리** — `register_tag("!custom")` 데코레이터 및 명령형 형태 + `clear_tag_handlers()`; 등록된 태그를 가진 스칼라 노드는 핸들러를 통해 변환됨
- **우선순위를 가진 태그 핸들러 체이닝** — 여러 핸들러가 `priority` 오름차순으로 실행; `YamlTagSkip`은 핸들러가 다음으로 전달되도록 하고, fallback은 원래 값을 유지
- **Pydantic 통합** — `parse_as(Model, yaml, **yaml_kwargs)`는 YAML을 파싱하여 Pydantic v2 모델로 검증; pydantic이 없을 때 `ImportError` 발생
- **`.pyi` 타입 스텁** — maturin으로 자동 생성되어 커밋되므로 `register_tag`, `parse_as`, `to_yaml_with_options` 및 새 예외가 타입 체커에게 표시됨

#### 변경

- CI Python 매트릭스 확장: ubuntu, windows, macos에서 3.8-3.14
- 안정 ABI: `abi3-py39` → `abi3-py38` (더 넓은 Python 3.8+ 지원), `abi3t` + `abi3t-py315` 추가 (free-threaded 안정 ABI)
- `pyproject.toml` classifiers에 3.13, 3.14, 3.15 항목 추가
- **CI 최적화: 중복 Rust 컴파일 제거** — 단일 `rust-lint` 작업이 `cargo clippy` + `cargo test`를 한 번 실행; 빌드 작업은 OS별 abi3 휠을 하나 생성하고 테스트 작업이 `maturin develop` 대신 설치하여 21개 매트릭스 작업에서 Rust 컴파일을 제거 (~86% 감소); 모든 작업에 `Swatinem/rust-cache` 추가
- **pydantic 테스트 의존성** — `pydantic>=2.10.6`을 `[dependency-groups] test` 및 `.ci/requirements-test.txt`에 추가 (SSOT via `uv sync` in ci.yml)

#### 수정

- **Windows DLL 로딩** — `src/py/tag_registry.rs`에서 `#[cfg(test)]` 블록 제거로 Windows에서 `import pyrs_yaml`broken 문제 해결 (`250b8d0`)
- **Python 3.8 호환성** — `pydantic.py`에 `from __future__ import annotations` 추가 (`63d2495`)
- **CI pydantic 스킵** — `pytest.importorskip("pydantic")`로 pydantic 미설치 시 테스트 통과 (`7be011d`)
- **Windows의 CI glob 확장** — `pip install dist/*.whl`에 `shell: bash` 사용 (PowerShell은 `*` 확장 안 함) (`2f7778d`)
- **문자열이 아닌 태그 핸들러 반환이 `YamlTagError`를 발생시킴** — 비`str` 값을 반환하는 핸들러는 이제 `Tag handler '!x' must return a string` 오류 발생 (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` 인덴트 연결** — `indent_mapping`/`indent_sequence`/`indent_offset`가 직렬화器에 의해 이제 반영됨 (이전에는 dead 필드; 각각 생략 시 `indent_size`/0으로 기본값)
- **`width`가 작은 값에서 멈추지 않음** — `width < continuation indent`일 때 무한 루프 대신 나머지 텍스트를 래핑 없이 출력 (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — 태그 핸들러를 등록 해제하는 새 함수; `register_tag`/`clear_tag_handlers` 보완
- **`duplicate-key` 오류가 i18n 적용** — `YamlDuplicateKeyError` 메시지가 이제 모든 4개 locale을 통해 `format_i18n_error`를 통해 흐름

### [0.8.0] - 2026-07-30

#### 추가

- **`YAML()` 인스턴스 API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` 재사용 가능한 구성; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` 메서드
- **Python `Node` API** — `Node` 클래스: `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`, `children`, `root_type`, `value`로 AST 탐색; JSONPath 스타일 쿼리 언어 (`$.key.sub`, `$.arr[0]`, `$..deep`)
- **`doc.version` 메타데이터** — `YamlDocument.version()`이 YAML spec 버전 반환 (기본값 "1.2")
- **`MergedView`** — `doc.merged()`가 병합 키가 해석된 읽기 전용 dict-like 뷰 반환
- **라이프사이클 경고** — `Node.release()`로 노드 명시적 무효화; 오래된 접근은 `RuntimeWarning` + `YamlDocumentError` 발생

#### 변경

- `parse()` / `safe_load()`가 이제 구문 당용으로 `YAML().parse()` / `.safe_load()`에 위임
- `YamlDocument`가 이제 문서 메타데이터를 위해 `version` 필드 저장

### [0.7.1] - 2026-07-30

#### 추가

- **ryaml 벤치마크 비교** — `tests/test_benchmark.py`에 `ryaml` (Rust YAML 라이브러리) 에 대한 벤치마크 추가 (PyYAML 및 ruamel.yaml와 함께); `benchmark_compare.py` 기능 비교 보고서로 재작성
- **CI 준수 임계값 상향** — YAML Test Suite 준수 게이트가 `test_compliance_report()`에서 70%에서 75%로 증가; 유효 파싱율 게이트는 95% (`tests/test_yaml_suite.py:251`)
- **CI 의존성 통합** — 발행 워크플로와 로컬 개발 전반에 통일된 테스트 의존성 관리를 위해 `.ci/requirements-test.txt` 및 `.ci/requirements-test-lite.txt` 추가
- **벤치마크 현대화** —更快的 C 확장 기반 통계 벤치마킹을 위해 `pytest-benchmark`에서 `pytest-codspeed`로 마이그레이션; 모든 CI 작업은 이제 `-r .ci/requirements-test.txt` 사용
- **Rust 벤치마크 Divan으로 마이그레이션** — `codspeed-criterion-compat`를 `codspeed-divan-compat` v5.0.1로 교체; 16개 벤치마크가 Criterion 그룹에서 `#[divan::bench]` 속성으로 재작성 (`Cargo.toml`, `benches/yaml_bench.rs`)

#### 변경

- CI 벤치마크 작업은 교차 라이브러리 비교를 위해 `ryaml` 설치
- `benchmark_compare.py`는 이제 타이밍을 `pytest-benchmark`에 위임하고 기능 비교/보고 도구로 역할

### [0.7.0] - 2026-07-29

#### 추가

- **직렬화器 `max_depth` 가드** — `serialize_node_internal`이 이제 재귀 깊이 추적하고 제한 초과 시 `YamlMaxDepthError` 발생 (기본값 1000), 파서 보호 일치 (`src/serializer.rs:135-145`)
- **직렬화器 핫패쓰 최적화** — 블록 스타일 직렬화를 대상으로 한 5개 최적화로 ~4.9% 루트립 속도 향상:
    - `write_anchor_tag` 및 `write_inline_comment` None 체크 인라이닝 (~99% 노드에 대한 메서드 호출 제거)
    - `write_indent` hot/cold path 분할 (캐시된 레벨 ≤64에 대한 직접 인덱싱)
    - 짧은 ASCII 영숫자 문자열 (≤8자) 에 대한 `write_plain_scalar` 고속 경로
    - Plain 스칼라에 대한 `write_scalar_for_key` 직접 디스패치 (디스패치 체인 방지)
- **pytest-benchmark 마이그레이션** — Python 벤치마크가 통계적 엄밀성, 구조화된 JSON 출력 및 CI 통합을 위해 원시 `time.perf_counter()`에서 `pytest-benchmark`로 마이그레이션 (`tests/test_benchmark.py` + 업데이트된 `tests/test_performance.py`)

#### 변경

- Python 벤치마크에서 원시 `timeit` 대신 `pytest-benchmark` 사용
- CI 벤치마크 작업은 이제 개별 스크립트 대신 `pytest --benchmark-json` 실행

#### 제거

- `write_inline_comment` 메서드 — 모든 호출 위치에서 인라이닝됨
- 직렬화기에서 `Comment` import — 이제 불필요

### [0.6.0] - 2026-07-27

#### 추가

- **비동기 직렬화** — `asyncio.run_in_executor`를 통한 `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`, `safe_load_async` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema 검증** — `YamlValidateError` 예외 + `YamlDocument.validate(schema)` 메서드 (`str` 또는 `dict` 수락); Python `jsonschema` 모듈에 위임
- **`YamlDocument.to_json()`** — 문서를 JSON 문자열로 직렬화 (Python `json.dumps` 사용)
- **증분 재파싱** — `YamlDocument`가 이제 소스 텍스트 저장 (`doc.source()`); `doc.reparse(resolve_merges=True, schema="core")` 로 제자리 재파싱
- **29개 새 테스트** — `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

#### 변경

- `YamlValidateError` 새 사용자 정의 예외로 등록 (`ValueError` 상속)
- `rust_i18n::i18n!` 매크로 경로가 `"src/i18n/locales"`로 업데이트
- `validate_translations()` 테스트 경로가 새 로케일 디렉토리 일치하도록 업데이트

#### 제거

- 중복 `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` 삭제 (rust-i18n에서 참조되지 않음)
- `locales/*.yml` → `src/i18n/locales/` 이동 (i18n 모듈과 함께 위치)

#### 의존성 변경

- 런타임 의존성: `jsonschema>=4.25.1`
- 개발 의존성: `pytest-asyncio>=0.23` (런타임에서 이동, 더 이상 고정되지 않음)

### [0.5.0] - 2026-07-27

#### 수정

- **`Serializer::write_node`** — `block_mapping`/`block_sequence`의 `values.iter().next().unwrap()`에서 `.unwrap()` 제거하고 안전 인덱스 접근으로 교체하여 edge-case AST에서 잠재적 패닉 제거
- **`YAML_SCHEMA` 상수** — 오타 `yamorg2002`를 `yamlorg2002`로 수정 (YAML 1.2 spec URL 일치)
- **개발 문서** — Python 명령어에 필수 `uv run` 접두사 및 Rust 명령어에 직접 `cargo` 추가하여 `AGENTS.md` 업데이트

### [0.4.0] - 2026-07-27

#### 추가

- **132개 새 gap-filling 테스트** — 이전에 테스트되지 않은 API에 대한 포괄적 커버리지
- **i18n 함수 테스트** — `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`
- **`parse_all_docs` 전용 테스트 스위트** — 단일 문서, 여러 문서, 빈 문서, 주석
- **`parse_file` 성공 사례 테스트** — 기본 파싱, 주석 보존, 파일 없음 오류
- **`to_yaml_with_options` 테스트** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys` 순서 보존
- **`to_dict()` 메서드 테스트** — 스칼라 루트, 중첩, 리스트, bool, null, anchor 해석, 빈 매핑/시퀀스
- **YamlDocument dunder 메서드 테스트** — `__repr__`, `__str__`, `__contains__`, `__len__`, `__iter__`, `__getitem__`, `root_type()`
- **바이트 입력 테스트** — `parse(b"key: value")`, UTF-8 바이트, 잘못된 UTF-8 오류
- **유니코드 & 특수 문자 테스트** — CJK, 이모지, 루트립, CRLF 줄 끝, 중복 키
- **`safe_load`/`safe_loads` 기능 커버리지** — anchor, merge key, block scalar, flow collection, 특수 부동소수, 타입 해석
- **`from_dict` edge cases** — 키의 특수 문자, 중첩 리스트, None 값, 빈 dict/list
- **`from_json` 루트립** — 중첩 구조, 배열, 잘못된 JSON 오류
- **`dump_file` 테스트** — 성공 경로, 잘못된 경로 오류
- **YAML Test Suite 개별 케이스 테스트** — 8진수, 16진수, 과학적 표기법, NaN, 무한대, merge key, 명시적/암시적 키, bool/null 변형, block scalar strip (`|-`), flow collection
- **`resolve_merges` 파라미터 테스트** — 비활성화 시 `<<` 보존, 기본값으로 해석
- **Flow collection 루트립** — 루트 레벨 및 중첩 flow 매핑/시퀀스
- **비스칼라 노드에 anchor** — 매핑 anchor (`&defaults`) 및 시퀀스 anchor (`&items`)
- **시퀀스 인덱싱 테스트** — 양수 인덱스, 범위 벗어남 오류
- **Merge key 통합** — 해석된 및 해석되지 않은 merge key로 루트립
- **태그 보존** — `!!seq` 및 `!!map` 태그 테스트 커버리지
- **주석 보존** — 복잡한 구조의 inline 및 standalone 주석 테스트

#### 변경

- 버전 동기화 수정: `python/pyrs_yaml/__init__.py` `__version__`을 0.2.0에서 0.4.0으로 업데이트하여 Cargo.toml/pyproject.toml과 일치
- `dist/`에서 구버전 0.2.0 wheel 아티팩트 제거

### [0.3.0] - 2026-07-27

#### 추가

- **NumPy ndarray 직렬화** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()`이 모든 차원 (0-D from N-D) 의 `numpy.ndarray` 지원
    - 지원 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - 다차원 배열은 올바른 들여쓰기로 중첩 YAML 리스트로 직렬화
    - 복소수는 `(re+imj)` 문자열 형식으로 직렬화
    - `0-D` 스칼라 배열은 1-D로 재변형되어 단일 항목 리스트로 직렬화
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate로 제로 복제 dtype 디스패치
    - 슬라이스 반복 동안 GIL 해제 (최대 성능)
- **`quoted_scalar()`** — 단일 따옴표 YAML 스타일이 필요한 값용 새 `CustomNode::quoted_scalar()` 생성자
- **따옴표 스칼라의 타입 해석** — `resolve_yaml_type`가 이제 `SingleQuoted`/`DoubleQuoted` 스칼라에 적용되어 따옴표 있는 음수의 올바른 루트립
- **포괄적 NumPy 테스트 스위트** — 모든 dtype, 차원 (0-D from 4-D), 음수, 무한대, NaN, 빈 배열, edge case를 다루는 42개 테스트
- Flow collection (`{}`/`[]`) 루트립 지원 — Mapping/Sequence AST 노드에 `flow_style` 필드
- `parse()`가 `str` 및 `bytes` 입력 모두 수용
- `parse()`가 `resolve_merges` 파라미터로 merge key 확장 옵트아웃 지원
- saphyr 이벤트를 통한 다중 문서 파싱용 `parse_all_docs()`
- `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` 파라미터가 있는 `to_yaml_with_options()`
- 기본값 파라미터를 지원하는 `get()`
- YAML을 파일에 작성하는 `dump_file()`
- `benches/yaml_bench.rs`의 Criterion 벤치마크 (파싱/직렬화/루트립)
- 매트릭스 테스트가 있는 GitHub Actions CI (3 OS x 4 Python 버전)
- 전체 YAML 1.2 spec으로 확장된 anchor 이름 파싱 (도트, 콜론, 해시, 따옴표 anchor)
- `__version__` 속성, `py.typed` PEP 561 마커

#### 수정

- **음수 루트립** — YAML 1.2 블록 시퀀스는 `-`로 시작하는 plain 스칼라를 포함할 수 없음; 이제 직렬화 시 따옴표로 감싸서 정수/부동소수로 올바르게 파싱
- **N-D 배열 지원** — `PyArray1<T>`를 `PyArrayDyn<T>`로 교체하여 1-D뿐만 아니라 모든 차원의 배열 지원
- **올바른 중첩 깊이** — 다차원 배열이 이제 정확히 N 수준의 중첩 생성 (내부 차원은 shape[1..] 처리, 루트 차원은 `plain_sequence`로 래핑)
- `to_dict()` 및 `safe_load()`의 alias 해석 — alias가 이제 참조값 대신 `None`으로 해석
- `safe_loads()`가 더 이상 단순 `split("---")`를 사용하지 않음 — saphyr의 문서 이벤트 사용
- 파싱 중 Mapping/Sequence 태그가 더 이상 폐기되지 않음
- `format_scalar_for_key()`이 Literal/Folded 블록 스칼라 스타일 처리

#### 변경

- ndarray 타입 디스패치를 위해 `numpy` crate (v0.29) 의존성 추가
- PyO3를 0.21에서 0.29로 업그레이드
- 15+ 보일러플레이트 `CustomNode` constructions를 `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` 생성자로 교체
- 직렬화器가 `write_anchor_tag()` 및 `write_inline_comment()` 헬퍼 추출
- 파서가 `detect_flow_style()` 헬퍼 추출
- 데드 코드 제거: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (테스트 전용)
- 6개 중복 테스트 파일 통합, 9개 진단 스크립트를 `scripts/`로 이동
- 키/인덱스/타입 컨텍스트로 오류 메시지 개선

### [0.1.0] - 2026-07-25

#### 추가

- saphyr-parser를 통한 YAML 1.2 준수로 초기 릴리스
- 완전한 메타데이터 (주석, anchor, 태그, chomping, 스칼라 스타일) 와 함께 사용자 정의 AST
- 주석, anchor, 태그 및 서식의 루트립 보존
- PyYAML 호환 API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` 변환 함수
- YAML frontmatter 추출용 `read_markdown`/`read_markdown_str`
- chomping indicator가 있는 블록 스칼라 (`|-`/`|+`/`>-`/`>+)
- 이스케이프 시퀀스 (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 타입 해석 (null, bool, int, float, infinity, NaN)
- Merge key 해석 (`<<: *alias`)
- 복잡한 키 (시퀀스/매핑을 키로)
