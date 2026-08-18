---
title: Features
description: pyrs-yaml의 주요 기능 — YAML 1.2 준수, 순환 파싱, 성능, 커스텀 AST, PyYAML 호환성 등
tags:
  - docs
status: new
---

pyrs-yaml는 PyYAML의 **직접 교체**로 설계되었으며, PyYAML이 없는 강력한 기능을 추가합니다.

## YAML 1.2 준수

**granit-parser**로 구동되며, YAML 테스트 스위트에서 **99.75% 통과율 (405/406)**을 달성.

## 완벽한 순환 파싱

PyYAML과 달리, pyrs-yaml는 **모든 서식과 메타데이터를 유지**합니다:

- **주석** — 독립 주석과 인라인 주석
- **앵커** (`&name`)와 **별칭** (`*name`)
- **태그** (`!!str`, `!!int` 등)
- **촙핑 지시자** (`|-`, `|+`, `>-`, `>+`)
- **스칼라 스타일** (일반, 단일 따옴표, 이중 따옴표, 리터럴, 접은 형식)
- **흐름/블록 서식** — `[]`/`{}`와 블록 스타일 유지

## 성능

!!! note "벤치마크 환경"
    벤치마크는 CodSpeed CI(`pytest-codspeed`, WallTime 모드)에서 측정된 결과입니다. 절대 시간은 환경에 따라 다를 수 있으나 상대 속도는 일관됩니다.

Rust 백엔드는 PyYAML보다 파싱 **21–43배**, 직렬화 **55–177배 빠릅니다**:

| Operation | pyrs-yaml | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 1.5 ms | 57.7 ms |
| Serialize (large) | 0.17 ms | 30.2 ms |
| Round-trip | 1.6 ms | 87.9 ms |

## 커스텀 AST

**CustomNode** AST는 YAML 구조를 완전히 제어할 수 있게 합니다:

- 프로그래밍 방식으로 노드 검사 및 수정
- 사용자 정의 메타데이터 (주석, 앵커, 태그) 추가
- 서식을 완전히 제어하면서 YAML를 처음부터 구축
- 고급 사용 사례: 템플릿 엔진, 구성 생성자, 코드 포맷터

## PyYAML 호환성

익숙한 API로 직접 교체 가능：

```python
import pyrs_yaml as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

## 비동기 I/O

`asyncio`를 통한 논블로킹 직렬화 및 파싱:

```python
import asyncio
import pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dump_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

사용 가능한 함수: `safe_dump_async`, `safe_load_async`, `safe_loads_async`.

## JSON Schema 검증

JSON Schema를 기반으로 파싱된 YAML 문서 검증：

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

검증 실패 시 `YamlValidateError`를 발생시킵니다.

## 중복 키

기본적으로 중복 매핑 키는 `YamlDuplicateKeyError`를 발생시킵니다:

```python
pyrs_yaml.parse("key: first\nkey: second")
# pyrs_yaml.YamlDuplicateKeyError: duplicate key: key
```

`allow_duplicate_keys=True`를 전달하면 **마지막 값**이 유지됩니다:

```python
doc = pyrs_yaml.parse("key: first\nkey: second", allow_duplicate_keys=True)
doc.get("key")  # "second"
```

이 스위치는 `parse`, `safe_load`, `safe_loads`, `parse_file`, `parse_all_docs`, `YAML(allow_duplicate_keys=True)`에 적용됩니다. 순환 모드에서 중복 키가 허용된 문서는 마지막 키-값 쌍을 출력하여 직렬화됩니다.

## 직렬화 옵션

`to_yaml_with_options()`는 들여쓰기과 줄 바꿈을 제어합니다:

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=2,  # 기본 들여쓰기 (유형별 옵션 생략 시 사용)
    width=80,  # 줄 바꿈 너비; 0은 줄 바꿈 비활성화
    indent_mapping=4,  # 블록 매핑 레벨별 들여쓰기
    indent_sequence=2,  # 블록 시퀀스 레벨별 들여쓰기
    indent_offset=0,  # 전체 문서의 기본 오프셋
)
```

`indent_mapping` / `indent_sequence` / `indent_offset`를 생략하면 각각 `indent_size` / `indent_size` / `0`이 되므로 `indent_size=4`도 모든 레벨을 4만큼 들여씁니다.

## 커스텀 태그 핸들러

사용자 정의 YAML 태그에 대한 핸들러를 등록하여 스칼라 값을 변환합니다:

=== "데코레이터"

    ```python
    import pyrs_yaml


    @pyrs_yaml.register_tag("!custom")
    def custom_handler(node):
        return f"custom:{node}"

    doc = pyrs_yaml.parse("name: !custom value")
    doc.get("name")  # "custom:value"
    ```

=== "명령형"

    ```python
    import pyrs_yaml


    pyrs_yaml.register_tag("!custom", lambda node: node.upper())

    doc = pyrs_yaml.parse("name: !custom value")
    doc.get("name")  # "custom:value"
    ```

- 같은 태그에 대한 여러 핸들러는 `priority` 오름차순으로 실행됩니다. `YamlTagSkip`을 발생시키면 다음 핸들러에 위임됩니다.
- 핸들러는 문자열을 반환해야 합니다. 그렇지 않으면 `YamlTagError`가 발생합니다.
- `remove_tag("!custom")`와 `clear_tag_handlers()`로 핸들러를 해제합니다.

## Pydantic 통합

Pydantic 모델로 YAML을 직접 파싱하거나 모델을 YAML로 직렬화:

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


# Pydantic 모델로 YAML 파싱
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice

# 모델을 YAML 문자열로 직렬화
yaml_str = pyrs_yaml.dump_pydantic(user)
print(yaml_str)
```

## 점진적 재파싱

다른 옵션으로 저장된 소스 텍스트를 제자리에서 재파싱：

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

## 제자리 편집

파싱된 문서를 **서식 메타데이터를 전혀 잃지 않고** 편집합니다 — 주석, 앵커, 태그, 스칼라 스타일, 흐름/블록 스타일이 모두 유지됩니다:

```python
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # (1)!
doc.insert("$.server.ports", 0, 80)  # (2)!
doc.append("$.server.ports", 443)  # (3)!
doc.rename("$.server", "srv")  # (4)!
del doc["server"]  # 또는: doc.delete("$.server")
```

1. :material-arrow-down: `set`은 경로의 값을 교체하며 인라인 주석을 유지합니다.
2. :material-arrow-down: `insert`는 시퀀스 인덱스 위치에 요소를 삽입합니다.
3. :material-arrow-down: `append`는 시퀀스 끝에 추가합니다.
4. :material-arrow-down: `rename`은 매핑 키를 제자리에서 이름 변경하며 위치와 주석을 유지합니다.

- **경로 API** — JSONPath 스타일 경로(`$.a.b[0]`), 루트 슈가(`doc["k"] = v`, `del doc["k"]`)
- **노드 API** — `doc.node().find(path)`는 `Node` 객체를 반환하며 `set_value` / `insert` / `append` / `delete` / `rename`과 트리 탐색(`parent`, `children`, `walk`, `filter`)을 지원
- **원자성** — 실패한 편집은 문서(리비전 포함)를 변경하지 않습니다
- **메타데이터 보존** — 교체된 스칼라는 주석/앵커/태그/따옴표를 유지; 이름이 변경된 키는 위치와 주석을 유지
- **별칭 인식** — 별칭 자신의 경로 설정은 그 자리를 교체; 별칭*을 통한* 편집은 `YamlEditError` 발생

자세한 내용은 [제자리 편집 가이드](guides/editing.md)를 참조하세요.

## NumPy ndarray 지원

pyrs-yaml는 모든 차원의 `numpy.ndarray` 객체를 직접 YAML로 직렬화할 수 있습니다:

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### 지원되는 dtype

| 타입 | Rust 백엔드 | YAML 출력 |
|------|------------|----------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | 일반 정수（음수 시 따옴표） |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | 일반 정수 |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | 일반 실수（음수 시 따옴표） |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` 문자열 |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

!!! warning "블록 시퀀스의 음수 스칼라"
    YAML 1.2 블록 시퀀스는 `-`로 시작하는 일반 스칼라를 포함할 수 없습니다. 음수 값은 자동으로 따옴표가 추가됩니다.

#### 참고사항

- **제로 복사**: `numpy` Rust crate의 `PyUntypedArray`를 사용하여 타입이 지워진 배열 접근을 수행한 후, 올바른 타입의 `PyArrayDyn<T>`에 디스패치하여 제로 복사 슬라이스 반복을 수행
- **GIL 해제**: 슬라이스 반복은 GIL 외부에서 실행되어 큰 배열에서 최대 성능을 발휘
- **음수**: YAML 1.2 블록 시퀀스에는 `-`로 시작하는 일반 스칼라를 포함할 수 없습니다. 음수 값은 자동으로 따옴표가 추가되며 순환 파싱 시 올바르게 파싱됩니다
- **0차원 배열**: 1차원으로 리셰이프되어 단일 항목 리스트로 직렬화
- **복소수**: YAML에는 네이티브 복소수 타입이 없습니다. `(re+imj)` 문자열로 직렬화됩니다. `safe_load`는 Python `complex`가 아닌 문자열로 반환
- **Markdown frontmatter 추출** — `read_markdown()` 블로그/콘텐츠 도구용
- **JSON ↔ YAML 변환** — `from_json()` / `from_dict()`
- **다중 문서 파싱** — `parse_all_docs()`
- **국제화 오류 메시지** — `set_language("ko")` 이중 언어 오류용
- **타입 힌트** — IDE 지원을 위한 완전한 `.pyi` 스텁

## 지원되는 YAML 구조

| 기능 | 지원 |
|------|------|
| YAML 1.2 사양 | :material-check: 완전 |
| 주석 (독립) | :material-check: 유지 |
| 주석 (인라인) | :material-check: 유지 |
| 앵커 및 별칭 | :material-check: 유지 |
| 태그 (명시적) | :material-check: 유지 |
| 블록 스칼라 (`|`, `>`) | :material-check: 유지 |
| 촙핑 지시자 | :material-check: 유지 |
| 흐름 컬렉션 (`{}`, `[]`) | :material-check: 유지 |
| 병합 키 (`<<`) | :material-check: 해결 |
| 복합 키 | :material-check: 지원 |
| 이스케이프 시퀀스 | :material-check: 지원 |
| 다중 문서 | :material-check: 지원 |
| **비동기 I/O** | **:material-check: `safe_*_async`** |
| **JSON Schema 검증** | **:material-check: `doc.validate()`** |
| **점진적 재파싱** | **:material-check: `doc.reparse()`** |
| **제자리 편집** | **:material-check: `doc.set()` / `insert()` / `append()` / `delete()` / `rename()`** |
| **JSON 내보내기** | **:material-check: `doc.to_json()`** |
| **Metadata editing** | **:material-check:  `Node.set_comment()` / `set_anchor()` / `set_tag()`** |
| **Style/format control** | **:material-check:  `Node.set_scalar_style()` / `set_flow_style()` / `set_chomping()`** |
| **Deep editing** | **:material-check:  `doc.set_many()` / `sort_keys()` / `Node.move()` / `copy()`** |
| **Schema validation** | **:material-check:  `validate_against_schema()`** |
| **Schema file IO** | **:material-check:  `load_schema()` / `list_schemas()`** |
