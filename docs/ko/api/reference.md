---
title: 모듈 참조
description: pyrs_yaml 모듈의 전체 API 참조 — 코어 함수, PyYAML 호환 함수, 변환 함수, 예외 등
tags:
  - docs
status: new
---

`pyrs_yaml` 모듈의 전체 API 참조입니다.

!!! tip "버전 호환성"
    pyrs-yaml은 ABI3 휠로 빌드되어 Python 3.8부터 3.15까지 단일 휠로 지원합니다.

## :material-code-braces: 코어 함수

### `parse()`

YAML 문자열 또는 바이트를 파싱하여 `YamlDocument`로 변환합니다.

```python
parse(yaml: str | bytes, resolve_merges: bool = True, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**매개변수:**

- `yaml` — `str` 또는 `bytes` YAML 콘텐츠
- `resolve_merges` — 파싱 후 병합 키 (`<<: *alias`)를 해석할지 여부 (기본값: `True`)
- `schema` — 스키마 이름 (`"core"`, `"json"`, `"failsafe"`, `"yaml1.1"` 또는 등록된 사용자 정의 이름), 또는 인라인 스키마 dict ([YAML 스키마 언어](#yaml-schema-language) 참조)
- `max_depth` — 최대 중첩 깊이 (기본값: `1000`)
- `allow_duplicate_keys` — 중복 매핑 키를 허용할지 여부 (기본값: `False`)

**반환값:** 파싱된 YAML를 포함하는 `YamlDocument`

**발생:**

- `YamlParseError` — 잘못된 YAML 구문
- `YamlTypeError` — 지정된 스키마를 찾을 수 없음
- `TypeError` — 입력이 `str` 또는 `bytes`가 아님

**예시:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, schema="json")
doc = pyrs_yaml.parse(
    "addr: 0xFF", schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]}
)
```

### `parse_file()`

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

### `parse_all_docs()`

문자열에서 여러 YAML 문서를 파싱합니다.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**반환값:** `YamlDocument` 객체 목록

**예시:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

## :material-swap-horizontal: PyYAML 호환 함수

### `safe_load()`

YAML를 파싱하여 네이티브 Python 타입을 반환합니다.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**다음과 동일:** PyYAML의 `yaml.safe_load()`

**예시:**

```python
data = pyrs_yaml.safe_load("key: value")  # {'key': 'value'}
```

### `safe_loads()`

여러 YAML 문서를 파싱합니다.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**다음과 동일:** PyYAML의 `yaml.safe_loads()`

### `safe_dump()`

Python 객체를 YAML로 직렬화합니다.

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray, sort_keys: bool = False) -> str
```

**다음과 동일:** PyYAML의 `yaml.safe_dump()`

**지원되는 입력 타입:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, 그리고 **`numpy.ndarray`** (모든 차원과 숫자 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

### `safe_dumps()`

`safe_dump()`의 별칭입니다.

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

## :material-json: 변환 함수

### `from_dict()`

Python dict를 YAML 문자열로 변환합니다. dict 값으로 `numpy.ndarray`도 허용됩니다.

```python
from_dict(data: dict[str, Any], sort_keys: bool = False) -> str
```

### `from_json()`

JSON 문자열을 YAML 문자열로 변환합니다.

```python
from_json(json_str: str) -> str
```

### `dump_file()`

Python 객체를 YAML로 직렬화하여 파일에 씁니다. `dict`, `list` 또는 `numpy.ndarray`를 허용합니다.

```python
dump_file(data: Any, path: str, sort_keys: bool = False) -> None
```

## :material-pillar: Pydantic 통합

### `dump_pydantic()`

Pydantic 모델을 YAML 문자열로 직렬화합니다.

```python
dump_pydantic(model: BaseModel) -> str
```

`model_dump(mode='json')`를 사용하여 문자열 타입을 유지한 다음(예: `"10001"` 우편번호는 문자열로 유지), `safe_dump`에 위임합니다.

**발생:**

- `ImportError` — pydantic이 설치되지 않음
- `TypeError` — `model`이 Pydantic `BaseModel` 인스턴스가 아님

**예시:**

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
```

### `parse_as()`

YAML 문자열을 파싱하고 Pydantic 모델에 대해 검증합니다.

```python
parse_as(model: type[BaseModel], src: str, **yaml_kwargs: Any) -> BaseModel
```

**매개변수:**

- `model` — Pydantic `BaseModel` 하위 클래스
- `src` — 파싱할 YAML 문자열
- `**yaml_kwargs` — `YAML()` 생성자로 전달되는 키워드 인자

**발생:**

- `ImportError` — pydantic이 설치되지 않음
- `TypeError` — `model`이 Pydantic `BaseModel` 하위 클래스가 아님
- `pydantic.ValidationError` — 파싱된 데이터가 모델 검증에 실패

**예시:**

```python
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
```

### `PyrsYamlConfigSettingsSource`

pydantic-settings YAML 소스를 pyrs-yaml로 구현한 클래스. `pydantic_settings.YamlConfigSettingsSource`의 드롭인 대체품으로, PyYAML 대신 pyrs-yaml을 파서로 사용합니다.

```python
PyrsYamlConfigSettingsSource(
    settings_cls: type[BaseSettings],
    yaml_file: ConfigFileSourceType | None = DEFAULT_PATH,
    yaml_file_encoding: str | None = None,
    yaml_config_section: str | None = None,
    deep_merge: bool = False,
)
```

`SettingsConfigDict(yaml_file=...)`(또는 직접 전달)로 선언된 YAML 파일에서 설정을 로드한 뒤 pyrs-yaml 파서(YAML 1.2 코어 스키마)로 해석합니다. 환경 변수·dotenv 덮어쓰기, `yaml_config_section`(점 표기 경로 지원), 여러 파일의 `deep_merge`, `yaml_file_encoding` 등 다른 pydantic-settings 기능은 `YamlConfigSettingsSource`와 동일하게 동작합니다.

**예외:**

- `ImportError` — pydantic-settings가 설치되지 않은 경우(`pyrs-yaml[settings]` 설치)

**예제:**

```python
from pydantic_settings import BaseSettings, SettingsConfigDict
import pyrs_yaml


class Settings(BaseSettings):
    app_name: str

    model_config = SettingsConfigDict(yaml_file="config.yaml")

    @classmethod
    def settings_customise_sources(
        cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
    ):
        return (
            init_settings,
            env_settings,
            dotenv_settings,
            file_secret_settings,
            pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls),
        )
```

> **참고:** 이 클래스는 지연 내보내기됩니다 — `import pyrs_yaml`에 pydantic-settings가 필요 없습니다. pydantic-settings가 설치되지 않은 상태에서 `pyrs_yaml.PyrsYamlConfigSettingsSource`에 접근하면 설치 방법을 안내하는 `ImportError`가 발생합니다.

## :material-file-document: YAML 스키마 언어 {#yaml-schema-language}

사용자 정의 스키마를 정의하여 일반 스칼라가 Python 타입으로 해석되는 방식을 제어합니다.

### `register_schema()`

사용자 정의 스키마를 등록합니다.

```python
register_schema(name: str, schema: str | dict) -> None
```

**매개변수:**

- `name` — 스키마 이름
- `schema` — YAML 문자열 또는 dict（`extends`, `rules`, `validate` 키 포함）

**예제:**

```python
import pyrs_yaml

# YAML 문자열에서 사용자 정의 스키마 등록
pyrs_yaml.register_schema(
    "hex",
    """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""",
)

# 사용자 정의 스키마 사용
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

### 인라인 스키마 dict

등록하지 않고 dict를 직접 전달:

```python
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
```

- **`extends`** — 선택적 기본 스키마（`core`, `json`, `failsafe`, `yaml1.1`）
- **`rules`** — 순서가 있는 `{pattern, type}` 목록; 첫 번째 일치 항목이 적용
- **`validate`** — 선택적 구조 검증 규칙: 경로 한정 타입(`$.port: int`), 컨테이너 검사(`sequence_of`, `mapping_of`), `required` 존재 확인; `validate_against_schema(data, schema_yaml)`로 문서 검증
- **지원 타입** : `null`, `bool`, `int`, `float`, `str`
- 내장 Core 스키마는 계속 제로 비용 `match` 디스패치 사용（영향 없음）
- **파일 I/O** — `load_schema(name, path)`로 YAML 파일에서 스키마 로드; `list_schemas()`로 등록된 모든 스키마 반환

## :material-link-variant: 통합 TagHandler API

`TagHandler` 기반 클래스는 태그 레지스트리(`register_tag`)와 커뮤니티 플러그인(`register_type`) API를 단일 등록 지점으로 통합합니다. 핸들러는 스칼라, 컨테이너, 라이프사이클 메서드를 임의 조합으로 구현할 수 있으며, 정의한 메서드만 호출됩니다.

### `TagHandler`

통합 태그 핸들러의 기반 클래스. 서브클래싱하여 필요한 메서드를 재정의합니다.

```python
class TagHandler:
    python_type = None  # isinstance 기반 직렬화 디스패치용

    # 스칼라: 파싱 시 문자열 변환 (register_tag 대체)
    def resolve(self, value: str, tag: str) -> str: ...

    # 스칼라: 양방향 유형 변환 (CustomType 대체)
    def from_yaml(self, value: Any, tag: str) -> Any: ...
    def to_yaml(self, obj: Any, tag: str) -> str: ...

    # 컨테이너: mapping/sequence 태그 변환
    def from_yaml_mapping(self, mapping: dict, tag: str) -> Any: ...
    def from_yaml_sequence(self, sequence: list, tag: str) -> Any: ...
    def to_yaml_mapping(self, obj: Any, tag: str) -> dict: ...
    def to_yaml_sequence(self, obj: Any, tag: str) -> list: ...

    # 라이프사이클 훅
    def on_parse_start(self, yaml: str, ctx: ParseContext) -> str: ...
    def on_parse_end(self, doc: YamlDocument, ctx: ParseContext) -> None: ...
    def on_serialize_start(self, doc: YamlDocument, ctx: SerializeContext) -> None: ...
    def on_serialize_end(self, result: str, ctx: SerializeContext) -> str: ...
```

!!! note "컨테이너 태그와 `safe_dump`"
    컨테이너 태그(`from_yaml_mapping`/`from_yaml_sequence`/
    `to_yaml_mapping`/`to_yaml_sequence`)는 AST 라운드트립 경로
    (`parse` → `to_yaml_with_options`/`serialize`)에서만 작동합니다.
    `safe_dump`/`from_dict`/`dump_file` 고속 경로는 AST 컨텍스트가 없어
    어떤 dict/list가 원래 태그가 있었는지 판단할 수 없으므로 항상 일반
    컨테이너를 출력합니다.

### `register_tag_handler()`

`TagHandler` 인스턴스를 등록. 클래스도 허용(자동 인스턴스화). 데코레이터 형식도 지원.

```python
register_tag_handler(name: str, handler: TagHandler | None = None, priority: int = 0) -> TagHandler
```

### `list_tag_handlers()`

```python
list_tag_handlers() -> list[tuple[str, int]]
```

등록된 모든 태그 핸들러를 `(tag_name, priority)` 튜플 목록으로 반환합니다.

### `get_tag_handler()`

```python
get_tag_handler(name: str) -> TagHandler | None
```

태그의 핸들러 인스턴스를 가져옵니다. 미등록 시 `None`.

### `remove_tag_handler()`

```python
remove_tag_handler(name: str) -> None
```

Rust 및 Python 레지스트리에서 태그의 모든 핸들러를 제거합니다.

### `serialize()`

라이프사이클 훅 지원으로 `YamlDocument`를 직렬화합니다.

```python
serialize(document: YamlDocument, indent: int = 2, sort_keys: bool = False,
          explicit_start: bool = False, explicit_end: bool = False,
          max_depth: int = 1000, width: int = 80) -> str
```

`to_yaml_with_options()` 전에 `on_serialize_start`, 후에 `on_serialize_end`를 호출합니다.

### 컨텍스트 객체

- `ParseContext` — 파싱 라이프사이클 훅에 전달(`schema`, `max_depth`, `resolve_merges`, `source`)
- `SerializeContext` — 직렬화 라이프사이클 훅에 전달(`indent`, `sort_keys`)
- `PluginInfo` — 플러그인 메타데이터 데이터 클래스(`name`, `version`, `description`, `conflicts`)

### `get_plugin_info()` / `list_plugin_info()`

```python
get_plugin_info(tag: str) -> PluginInfo | None
list_plugin_info() -> list[PluginInfo]
```

엔트리 포인트 발견으로 등록된 플러그인 메타데이터를 조회합니다.

## :material-check-decagram: 컴플라이언스

### `compliance_report()`

YAML 테스트 스위트 컴플라이언스 보고서를 계산합니다.

```python
compliance_report() -> dict
```

YAML 테스트 스위트 통과율과 테스트별 결과를 반환합니다.

## :material-wave: 스트리밍 이벤트

### `parse_stream()`

YAML을 점진적으로 파싱하여 원시 이벤트 dict를 생성합니다.

```python
parse_stream(yaml: str) -> StreamIterator
```

각 단계마다 이벤트 dict를 생성하는 `StreamIterator`를 반환합니다. `YAML().load_stream()`(Python 값으로 해석)과 달리 원시 토큰 스트림을 노출합니다.

### `YamlStream` { #yamlstream }

`YamlStream` 클래스는 `YAML().load_stream()` 및 `YAML().load_stream_file()`이 반환하는 지연 이벤트 반복자입니다. 전체 문서를 메모리에 로드하지 않고 파싱된 이벤트 dict를 한 번에 하나씩 생성합니다.

```python
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    print(event)
```

전체 API 세부 정보는 [`YamlStream`](yaml-instance.md)을 참조하세요.

## :material-clock-fast: 비동기 함수

`asyncio.run_in_executor`를 사용한 비동기 I/O 래퍼. 이벤트 루프 컨텍스트에서 논블로킹.

### `safe_dumps_async()`

Python 객체를 YAML 문자열로 직렬화 (비동기).

```python
async def safe_dumps_async(data: Any) -> str
```

### `safe_dump_async()`

Python 객체를 YAML 형식으로 stdout에 출력 (비동기).

```python
async def safe_dump_async(data: Any) -> None
```

### `safe_loads_async()`

YAML 문자열을 네이티브 Python 객체로 파싱 (비동기).

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

### `safe_load_async()`

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

## :material-page-layout-body: Markdown Front Matter {#markdown-frontmatter}

### `read_markdown()`

Markdown 파일에서 YAML Front Matter를 추출합니다.

```python
read_markdown(path: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**반환값:** `(frontmatter_dict, content_string)`. Front Matter가 없으면 `frontmatter`는 `None`.

### `read_markdown_str()`

Markdown 문자열에서 YAML Front Matter를 추출합니다.

```python
read_markdown_str(content: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

## :material-translate: i18n 함수 {#i18n-functions}

### `set_language()`

오류 메시지의 언어를 설정합니다.

```python
set_language(lang: str) -> None
```

지원: `"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

### `get_language()`

현재 언어를 가져옵니다.

```python
get_language() -> str
```

### `list_languages()`

지원되는 모든 언어를 나열합니다.

```python
list_languages() -> list[str]
```

### `detect_language()`

환경 변수에서 사용자의 선호 언어를 자동 감지합니다.

```python
detect_language() -> str
```

### `negotiate_language()`

BCP 47 언어 협상.

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

## :material-bug: 예외

- `YamlParseError` — YAML 파싱 오류 (`ValueError` 상속)
- `YamlSerializeError` — YAML 직렬화 오류 (`ValueError` 상속)
- `YamlTypeError` — 타입 변환 오류 (`TypeError` 상속)
- `YamlValidateError` — JSON Schema 검증 오류 (`ValueError` 상속)
- `YamlEditError` — 제자리 편집 오류 (`ValueError` 상속)
- `YamlPathError` — YAML 경로 오류 (`ValueError` 상속)
- `YamlDocumentError` — 오래된 `Node` 접근 오류 (`Exception` 상속)

자세한 내용은 [예외](exceptions.md) 페이지를 참조하세요.

## :material-information: 버전

```python
__version__ = "0.14.0"
```
