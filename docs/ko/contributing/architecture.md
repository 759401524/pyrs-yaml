---
title: 아키텍처
description: pyrs-yaml의 모듈화된 아키텍처 — 워크스페이스 구조, 모듈 설명, 데이터 흐름, 성능 특성
tags:
  - docs
status: new
---

pyrs-yaml는 성능과 정확성을 위해 설계된 모듈화된 아키텍처를 사용합니다.

## 개요

```text
┌─────────────────────────────────────────────────────────┐
│                     Python 레이어                        │
│  ┌─────────────────────────────────────────────────────┐│
│  │               pyrs_yaml 모듈                        ││
│  │  parse() | safe_load() | dump_file() | ...          ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │ PyO3 바인딩                     │
├────────────────────────▼─────────────────────────────────┤
│                    Rust 레이어                           │
│  ┌─────────────────────────────────────────────────────┐│
│  │  lib.rs — PyO3 모듈 (inline pymodule)              ││
│  │  • YamlDocument 클래스                              ││
│  │  • 예외 타입 (YamlParseError 등)                    ││
│  │  • 함수 래퍼                                        ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│      ┌─────────────────┼─────────────────┐              │
│      ▼                 ▼                 ▼              │
│  ┌─────────┐    ┌────────────┐    ┌────────────┐        │
│  │ ast.rs  │    │ parser/    │    │serializer  │        │
│  │ Custom  │◄──►│ saphyr     │    │ to_yaml()  │        │
│  │ Node    │    │ 통합       │    │ to_yaml_*  │        │
│  └─────────┘    └────────────┘    └────────────┘        │
│      ▲                 ▲                                    │
│      └─────────────────┴────────────────────┘              │
│                      CustomNode                           │
└─────────────────────────────────────────────────────────┘
```

## 워크스페이스 구조

코드베이스는 `crates/` 아래 두 개의 크레이트로 나뉩니다:

```text
crates/
├── pyrs-yaml-core/ # 순수 Rust, PyO3 의존성 없음
│ └── src/
│   ├── lib.rs # 모든 코어 모듈 재내보내기
│   ├── ast.rs # CustomNode AST
│   ├── editing/ # 편집 프리미티브 (navigate, region, dirty, metadata)
│   ├── i18n.rs # 국제화
│   ├── parser/ # YAML 파서 (saphyr 기반)
│   ├── serializer.rs # YAML 직렬화기
│   └── splice.rs # 스플라이스 기반 텍스트 조립
└── pyrs-yaml/ # PyO3 바인딩 레이어
    └── src/
        ├── lib.rs # 코어 재내보내기 + #[pymodule] 정의
        ├── py/ # PyO3 바인딩
        │   ├── mod.rs # YamlDocument pyclass
        │   ├── convert.rs # CustomNode ↔ Python 타입 변환
        │   └── editing/ # Python용 편집 래퍼
        └── fidelity.rs # 속성 기반 테스트
```

## 모듈 아키텍처

### 1. `crates/pyrs-yaml-core/src/ast.rs` — 사용자 정의 AST

**CustomNode** 열거형은 pyrs-yaml의 핵심입니다:

- **Scalar** — 스타일 (plain, 따옴표, 리터럴, 폴드), 주석, 앵커, 태그, 청핑 포함
- **Mapping** — 키 순서 유지를 위한 `IndexMap`, flow_style 플래그
- **Sequence** — 순서가 있는 리스트, flow_style 플래그
- **Null** — 주석, 앵커, 태그 포함
- **Alias** — 별칭 참조 (이름만)

**사용자 정의 AST를 사용하는 이유:**

- 표준 YAML 파서는 메타데이터 (주석, 포맷)를 폐기
- 사용자 정의 AST는 순환 보존에 필요한 모든 것을 유지
- 향후 기능 (사용자 정의 노드 타입, 메타데이터)을 위해 확장 가능

#### 2. `crates/pyrs-yaml-core/src/parser/` — YAML 파서

**saphyr-parser** (YAML 1.2 호환) 위에 구축:

- **`mod.rs`** — `AstReceiver` 상태 머신, 이벤트 기반 파싱, 플로우 스타일 감지
- **`stream.rs`** — 스트리밍 이벤트 파서 (라인별 YAML 이벤트)
- **`yaml/comment.rs`** — 원시 텍스트에서 주석 및 앵커 추출
- **`yaml/merge.rs`** — 병합 키 (`<<`) 해석
- **`yaml/scalar.rs`** — 스칼라 스타일 감지, 언스케이핑, 청핑
- **`yaml/schema.rs`** — YAML 스키마 해석 (core, JSON, failsafe, YAML 1.1)
- **`yaml/types.rs`** — YAML 1.2 타입 해석 (null, bool, int, float)

**핵심 설계 결정:**

- 이벤트 기반 API (토큰 기반 아님) — 구조화된 출력에 더 적합
- 2 패스 파싱: 먼저 주석/앵커를 추출하고, 이벤트를 파싱
- 병합 키 해석은 파싱 후 발생 (구성 가능)

#### 3. `crates/pyrs-yaml-core/src/serializer.rs` — YAML 직렬화기

AST에서 YAML를 재구성하는 사용자 정의 직렬화기:

- **`to_yaml()`** — 기본 옵션으로 직렬화
- **`to_yaml_with_options()`** — 사용자 정의 들여쓰기, 마커, 정렬
- **`write_anchor_tag()`** — 앵커/태그 출력 헬퍼
- **`write_inline_comment()`** — 인라인 주석 출력 헬퍼

**핵심 설계 결정:**

- 서드파티 이미터 없음 — 출력 포맷을 완전히 제어
- 중첩된 구조를 위한 들여쓰기 레벨 상태 관리
- 블록 스칼라를 위한 청핑 표시자 처리

#### 4. `crates/pyrs-yaml/src/py/` — PyO3 바인딩

Python-facing 레이어로 Rust 기능을 Python에 노출합니다:

- **`mod.rs`** — `YamlDocument` pyclass, `#[pymodule]` 진입점
- **`convert.rs`** — Python ↔ CustomNode 변환 및 오류 포맷팅
- **`python_types.rs`** — Python → CustomNode 타입 변환
- **`ndarray.rs`** — NumPy ndarray 직렬화 (선택 사항, `numpy` 기능)
- **`stream_events.rs`** — Python용 스트림 이벤트 타입
- **`streaming.rs`** — 스트리밍 파싱 (일정 메모리)
- **`writing.rs`** — 스트리밍 쓰기 (일정 메모리)
- **`tag_registry.rs`** — Python 태그 핸들러 등록
- **`editing/`** — Python용 편집 래퍼 (`segment_py.rs` + 코어 재내보내기)

PyO3 바인딩에서 사용하는 순수 Rust 편집 프리미티브:

- **`navigate.rs`** — AST 경로 탐색 (`navigate`, `navigate_mut`, `key_eq`, `mapping_key_index`, `normalize_index`, `parse_path_segments`)
- **`region.rs`** — 편집 영역 계산 (`path_nodes`, `region_unit`, `precompute`, 라인 헬퍼, `extend_delete_over_comments`)
- **`dirty.rs`** — 편집 연산 타입 (`DirtyKind`, `DirtyUnit`)
- **`metadata.rs`** — 메타데이터 보존 (`with_metadata_from`)

**내보내진 Python 함수 (총 18개):**
`parse`, `safe_load`, `safe_loads`, `safe_dump`, `safe_dumps`, `parse_file`, `dump_file`, `parse_all_docs`, `parse_stream`, `read_markdown`, `from_dict`, `from_json`, `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`, `YamlDocument`

#### 5. `crates/pyrs-yaml-core/src/lib.rs` — 모듈 진입점

- 모든 모듈 재내보내기
- 오류 타입: `YamlParseError`, `YamlSerializeError`, `YamlTypeError`
- `create_exception!` 매크로를 위한 사용자 정의 Python 예외
- `rust-i18n` 초기화

#### 6. `crates/pyrs-yaml-core/src/i18n.rs` — 국제화

- `i18n.rs` — 설정 및 언어 협상
- `i18n/` — 로케일 번들 (en, zh-CN, ja-JP, ko-KR)
- 이중 언어 오류 메시지 (포맷 문자열 포함)

#### 7. `crates/pyrs-yaml-core/src/integration/` — 통합 헬퍼

- `yaml_suite.rs` — 검증을 위한 YAML Test Suite 러너
- 벤치마크 및 규정 준수 검사를 위한 테스트 헬퍼

## 데이터 흐름

### 파싱 흐름

```text
YAML 문자열
    │
    ▼
┌─────────────────────────────────────┐
│ 1. 원시 텍스트에서 주석 추출        │
│ 2. 원시 텍스트에서 앵커 추출        │
│ 3. saphyr-parser → YAML 이벤트      │
│ 4. AstReceiver가 CustomNode 구축    │
│ 5. 스키마 타입 해석                  │
│ 6. 병합 키 해석 (활성화된 경우)      │
└─────────────────────────────────────┘
    │
    ▼
CustomNode (AST)
```

#### 직렬화 흐름

```text
CustomNode (AST)
    │
    ▼
┌─────────────────────────────────────┐
│ 1. 노드 타입 결정                   │
│ 2. 시작 부분 기록 (앵커, 태그)      │
│ 3. 내용 기록 (key: value)            │
│ 4. 인라인 주석 기록                  │
│ 5. 중첩된 노드 재귀 처리            │
└─────────────────────────────────────┘
    │
    ▼
YAML 문자열
```

## 성능 특성

| 작업 | 복잡도 | 설명 |
|------|--------|------|
| 파싱 | O(n) | YAML 이벤트의 단일 패스 |
| 직렬화 | O(n) | AST의 단일 패스 |
| 순환 보존 | O(n) | 파싱 + 직렬화 |
| 병합 해석 | O(n × m) | n = 문서 수, m = 문서당 병합 수 |
| 주석 추출 | O(n) | 원시 텍스트의 단일 패스 |

## 의존성

| 크레이트 | 용도 |
|---------|------|
| **PyO3** | Python 바인딩 (`experimental-inspect`, `abi3-py38`, `abi3t` 포함) |
| **saphyr-parser** | YAML 1.2 호환 파싱 |
| **IndexMap** | 키 유지를 위한 순서가 있는 해시 맵 |
| **serde_json** | JSON ↔ YAML 변환 |
| **numpy** | NumPy ndarray 지원 (선택 사항, 기본 활성화) |
| **rust-i18n** | 국제화 오류 메시지 |
