---

title: 모듈 참조
lang: ko

## 모듈 참조

`pyrs_yaml` 모듈의 전체 API 참조입니다.

### 코어 함수

#### `parse()`

YAML 문자열 또는 바이트를 파싱하여 `YamlDocument`로 변환합니다.

```python
parse(yaml: str | bytes, resolve_merges: bool = True) -> YamlDocument
```

**매개변수:**

- `yaml` — `str` 또는 `bytes` YAML 콘텐츠
- `resolve_merges` — 파싱 후 병합 키 (`<<: *alias`)를 해석할지 여부 (기본값: `True`)

**반환값:** 파싱된 YAML를 포함하는 `YamlDocument`

**발생:**

- `YamlParseError` — 잘못된 YAML 구문
- `TypeError` — 입력이 `str` 또는 `bytes`가 아님

**예시:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
```

#### `parse_file()`

YAML 파일을 파싱합니다.

```python
parse_file(path: str) -> YamlDocument
```

**매개변수:**

- `path` — YAML 파일 경로

**반환값:** `YamlDocument`

**발생:**

- `IOError` — 파일을 찾을 수 없거나 읽을 수 없음
- `YamlParseError` — 잘못된 YAML

**예시:**

```python
doc = pyrs_yaml.parse_file("config.yaml")
```

#### `parse_all_docs()`

문자열에서 여러 YAML 문서를 파싱합니다.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**반환값:** `YamlDocument` 객체 목록

**예시:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

### PyYAML 호환 함수

#### `safe_load()`

YAML를 파싱하여 네이티브 Python 타입을 반환합니다.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**다음과 동일:** PyYAML의 `yaml.safe_load()`

**예시:**

```python
data = pyrs_yaml.safe_load("key: value")  # {'key': 'value'}
```

#### `safe_loads()`

여러 YAML 문서를 파싱합니다.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**다음과 동일:** PyYAML의 `yaml.safe_loads()`

#### `safe_dump()`

Python 객체를 YAML로 직렬화합니다.

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**다음과 동일:** PyYAML의 `yaml.safe_dump()`

**지원되는 입력 타입:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, 그리고 **`numpy.ndarray`** (모든 차원과 숫자 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

#### `safe_dumps()`

`safe_dump()`의 별칭입니다.

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

### 변환 함수

#### `from_dict()`

Python dict를 YAML 문자열로 변환합니다. dict 값으로 `numpy.ndarray`도 허용됩니다.

```python
from_dict(data: dict[str, Any]) -> str
```

#### `from_json()`

JSON 문자열을 YAML 문자열로 변환합니다.

```python
from_json(json_str: str) -> str
```

#### `dump_file()`

Python 객체를 YAML로 직렬화하여 파일에 씁니다. `dict`, `list` 또는 `numpy.ndarray`를 허용합니다.

```python
dump_file(data: Any, path: str) -> None
```

### 비동기 함수

`asyncio.run_in_executor`를 사용한 비동기 I/O 래퍼. 이벤트 루프 컨텍스트에서 논블로킹.

#### `safe_dumps_async()`

Python 객체를 YAML 문자열로 직렬화 (비동기).

```python
async def safe_dumps_async(data: Any) -> str
```

#### `safe_dump_async()`

Python 객체를 YAML 형식으로 stdout에 출력 (비동기).

```python
async def safe_dump_async(data: Any) -> None
```

#### `safe_loads_async()`

YAML 문자열을 네이티브 Python 객체로 파싱 (비동기).

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

#### `safe_load_async()`

YAML 문자열을 네이티브 Python 객체로 파싱 (비동기).

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**예시:**

```python
import asyncio, pyrs_yaml

async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

### Markdown 프론트메터

#### `read_markdown()`

Markdown 파일에서 YAML 프론트메터를 추출합니다.

```python
read_markdown(path: str) -> tuple[dict[str, Any] | None, str]
```

**반환값:** `(frontmatter_dict, content_string)`. 프론트메터가 없으면 `frontmatter`는 `None`.

#### `read_markdown_str()`

Markdown 문자열에서 YAML 프론트메터를 추출합니다.

```python
read_markdown_str(content: str) -> tuple[dict[str, Any] | None, str]
```

### i18n 함수

#### `set_language()`

오류 메시지의 언어를 설정합니다.

```python
set_language(lang: str) -> None
```

지원: `"en"`, `"zh-CN"`

#### `get_language()`

현재 언어를 가져옵니다.

```python
get_language() -> str
```

#### `list_languages()`

지원되는 모든 언어를 나열합니다.

```python
list_languages() -> list[str]
```

#### `detect_language()`

환경 변수에서 사용자의 선호 언어를 자동 감지합니다.

```python
detect_language() -> str
```

#### `negotiate_language()`

BCP 47 언어 협상.

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

### 예외

- `YamlParseError` — YAML 파싱 오류 (`ValueError` 상속)
- `YamlSerializeError` — YAML 직렬화 오류 (`ValueError` 상속)
- `YamlTypeError` — 타입 변환 오류 (`TypeError` 상속)
- `YamlValidateError` — JSON Schema 검증 오류 (`ValueError` 상속)

### 버전

```python
__version__ = "0.6.0"
```
