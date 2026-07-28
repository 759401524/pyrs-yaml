---

title: Features
lang: ko

## 기능

pyyaml-rs는 PyYAML의 **직접 교체**로 설계되었으며, PyYAML이 없는 강력한 기능을 추가합니다.

### YAML 1.2 준수

**saphyr-parser**로 구동되며, YAML 테스트 스위트에서 **98.1% 통과율**을 달성.

### 완벽한 순환 파싱

PyYAML과 달리, pyyaml-rs는 **모든 서식과 메타데이터를 유지**합니다:

- **주석** — 독립 주석과 인라인 주석
- **앵커** (`&name`)와 **별칭** (`*name`)
- **태그** (`!!str`, `!!int` 등)
- **chomp 지시자** (`|-`, `|+`, `>-`, `>+`)
- **스칼라 스타일** (일반, 단일 따옴표, 이중 따옴표, 리터럴, 접은 형식)
- **흐름/블록 서식** — `[]`/`{}`와 블록 스타일 유지

### 성능

Rust 백엔드는 PyYAML보다 **25–40배 빠릅니다**:

| Operation | pyyaml-rs | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.08 ms | 2.96 ms |
| Round-trip | 0.08 ms | 2.98 ms |

### 커스텀 AST

**CustomNode** AST는 YAML 구조를 완전히 제어할 수 있게 합니다:

- 프로그래밍 방식으로 노드 검사 및 수정
- 사용자 정의 메타데이터 (주석, 앵커, 태그) 추가
- 서식을 완전히 제어하면서 YAML를 처음부터 구축
- 고급 사용 사례: 템플릿 엔진, 구성 생성자, 코드 포맷터

### PyYAML 호환성

익숙한 API로 직접 교체 가능：

```python
import pyyaml_rs as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

### 비동기 I/O

`asyncio`를 통한 논블로킹 직렬화 및 파싱:

```python
import asyncio
import pyyaml_rs

async def main():
    yaml = await pyyaml_rs.safe_dump_async({"a": 1})
    data = await pyyaml_rs.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

사용 가능한 함수: `safe_dump_async`, `safe_load_async`, `safe_loads_async`.

### JSON Schema 검증

JSON Schema를 기반으로 파싱된 YAML 문서 검증：

```python
doc = pyyaml_rs.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

검증 실패 시 `YamlValidateError`를 발생시킵니다.

### 점진적 재파싱

다른 옵션으로 저장된 소스 텍스트를 제자리에서 재파싱：

```python
doc = pyyaml_rs.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### NumPy ndarray 지원

pyyaml-rs는 모든 차원의 `numpy.ndarray` 객체를 직접 YAML로 직렬화할 수 있습니다:

```python
import numpy as np
import pyyaml_rs

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyyaml_rs.safe_dump(arr)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyyaml_rs.safe_dump(matrix)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip
loaded = pyyaml_rs.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

#### 지원되는 dtype

| 타입 | Rust 백엔드 | YAML 출력 |
|------|------------|----------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | 일반 정수（음수 시 따옴표） |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | 일반 정수 |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | 일반 실수（음수 시 따옴표） |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` 문자열 |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

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

### 지원되는 YAML 구조

| 기능 | 지원 |
|------|------|
| YAML 1.2 사양 | ✅ 완전 |
| 주석 (독립) | ✅ 유지 |
| 주석 (인라인) | ✅ 유지 |
| 앵커 및 별칭 | ✅ 유지 |
| 태그 (명시적) | ✅ 유지 |
| 블록 스칼라 (`|`, `>`) | ✅ 유지 |
| chomp 지시자 | ✅ 유지 |
| 흐름 컬렉션 (`{}`, `[]`) | ✅ 유지 |
| 병합 키 (`<<`) | ✅ 해결 |
| 복합 키 | ✅ 지원 |
| 이스케이프 시퀀스 | ✅ 지원 |
| 다중 문서 | ✅ 지원 |
| **비동기 I/O** | **✅ `safe_*_async`** |
| **JSON Schema 검증** | **✅ `doc.validate()`** |
| **점진적 재파싱** | **✅ `doc.reparse()`** |
| **JSON 내보내기** | **✅ `doc.to_json()`** |
