---
title: 스트리밍 파싱
description: pyrs-yaml의 스트리밍 파싱 — 지연 이벤트 반복, 상수 메모리, StreamIterator, 스트리밍 쓰기
tags:
  - docs
status: new
---

!!! note "스트리밍 파싱"
    `YAML.load_stream()` / `load_stream_file()`은 메모리 사용량이 O(앵커 + 청크)로 입력 크기와 무관합니다. 100MB+ 파일에 적합합니다.

`YAML.load_stream(file_obj)` 및 `YAML.load_stream_file(path)`는 YAML 이벤트를 지연 반복합니다——메모리 사용량은 O(앵커 수 + 64KB 청크)로 입력 크기와 무관합니다. 100MB+ 파일에 적합합니다.

```python title="스트림 로드"
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## 작동 방식

```mermaid title="스트리밍 파싱 아키텍처"
graph LR
    A["YAML 파일 / 문자열"] --> B["지연 이벤트 반복자<br/>O(앵커 + 64KB 청크)"]
    B --> C["이벤트 dict<br/>type, value, style, anchor, tag, line, column"]
    C --> D["소비자<br/>스트리밍 처리"]
```

## parse_stream과의 차이점

| 동작 | load_stream | parse_stream |
| --- | --- | --- |
| 메모리 | O(앵커 + 청크) | O(입력) |
| 주석 | :material-close: 출력하지 않음 | :material-check: 출력함 |
| 앵커 이름 | `anchor_{id}` | 원래 이름 |
| 오류 메시지 | 소스 스니펫 없음 | 소스 스니펫 있음 |
| 빈 입력 | `[stream_start, stream_end]` | `[]` |
| 태그 핸들러 | 적용하지 않음 | 적용함 (YAML.parse) |

## 리소스 관리

조기에 중단할 때는 `close()`를 호출하십시오——이것이 유일하게 보장된 해제 지점입니다(PyPy의 지연 GC는 `Drop` 타이밍을 보장하지 않습니다). `close()`는 멱등적이며, 전달한 파일 객체를 **닫지 않습니다**.

## 스트리밍 쓰기

`YAML().dump_stream(file_obj, iterable, ...)` 및 `YAML().dump_file(path, iterable, ...)`
는 문서를 하나씩 직렬화하며 상수 메모리(O(단일 문서 + 64KB 청크))를 사용합니다.

```python title="스트림 덤프"
from pyrs_yaml import YAML

buf = io.StringIO()
YAML().dump_stream(buf, [{"a": 1}, {"b": 2}])
# buf.getvalue() == "a: 1\n---\nb: 2\n"
```

### 구분자 규칙

- 첫 번째 문서 앞에는 `---` 없음. 이후 각 문서 앞에 `---` 추가.
- `explicit_start=True`는 첫 번째 문서 앞에 `---`를 추가.
- `explicit_end=True`는 마지막 문서 뒤에 `...`를 추가.
- 빈 iterable은 0바이트 출력.

#### 오류 의미론

중간에 실패(이터레이터 예외, 직렬화 오류, 쓰기 실패)하면 이미 작성된 출력이 대상에 남습니다——부분 출력에 대한 롤백은 없습니다.

#### safe_dump와의 차이점

| 측면 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 출력 | 멀티 문서 스트림 | 단일 문서 |
| 메모리 | O(단일 문서 + 64KB) | O(입력) |
| 항목 유형 | `YamlDocument`(주석/앵커 유지) 또는 일반 Python 객체 | 단일 Python 객체 |

#### 키 정렬

`sort_keys=True`를 전달하면 매핑 키를 정렬된 순서로 출력합니다. `safe_dump`의 `sort_keys` 동작과 동일합니다.

## StreamIterator

`StreamIterator` 클래스는 `parse_stream()` 및 `YAML().load_stream()` / `YAML().load_stream_file()`에서 생성됩니다. 반복자 프로토콜을 구현하며 이벤트 dict를 한 번에 하나씩 생성합니다.

```python title="이벤트 반복"
from pyrs_yaml import parse_stream

iterator = parse_stream("key: value\n---\na: 1")
for event in iterator:
    print(event["type"], event["value"])
```

### 반복자 프로토콜

`StreamIterator`는 `__iter__`(`self` 반환)와 `__next__`를 구현합니다:

```python title="반복자 프로토콜"
def __iter__() -> StreamIterator: ...
def __next__() -> dict | None: ...
```

스트림이 고갈되면 `__next__()`는 `None`을 반환합니다(`StopIteration`을 발생시키지 않음).

#### 이벤트 dict 키

| 키 | 타입 | 설명 |
| --- | --- | --- |
| `type` | `str` | 이벤트 타입(아래 참조) |
| `value` | `str` 또는 `None` | 스칼라 값, 별칭 이름, 또는 주석 텍스트 |
| `style` | `str` 또는 `None` | 스칼라 따옴표 스타일: `"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`; 주석의 경우 `"standalone"` 또는 `"inline"` |
| `anchor` | `str` 또는 `None` | 앵커 이름(`&name`) |
| `tag` | `str` 또는 `None` | 태그 문자열(`!!str`, `!custom`) |
| `line` | `int` | 줄 번호(0 기반) |
| `column` | `int` | 열 번호(0 기반) |

#### 이벤트 타입

| `type` | 생성 시점 |
| --- | --- |
| `stream_start` | YAML 스트림의 시작 |
| `stream_end` | 스트림의 끝 |
| `document_start` | 문서의 시작 |
| `document_end` | 문서의 끝 |
| `mapping_start` | 매핑의 시작 |
| `mapping_end` | 매핑의 끝 |
| `sequence_start` | 시퀀스의 시작 |
| `sequence_end` | 시퀀스의 끝 |
| `scalar` | 스칼라 값 |
| `alias` | 별칭 참조(`*name`) |
| `comment` | YAML 주석 |

#### `load_stream`과의 차이점

`parse_stream()`은 주석을 생성하고 원래 앵커 이름을 유지하는 `StreamIterator`를 반환합니다. `YAML().load_stream()` / `YAML().load_stream_file()`은 기본값이 다른 `YamlStream`을 반환합니다(위 비교표 참조).
