# 스트리밍 파싱

`YAML.load_stream(file_obj)` 및 `YAML.load_stream_file(path)`는 YAML 이벤트를 지연 반복합니다——메모리 사용량은 O(앵커 수 + 64KB 청크)로 입력 크기와 무관합니다. 100MB+ 파일에 적합합니다.

```python
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## parse_stream과의 차이점

| 동작 | load_stream | parse_stream |
| --- | --- | --- |
| 메모리 | O(앵커 + 청크) | O(입력) |
| 주석 | 출력하지 않음 | 출력함 |
| 앵커 이름 | `anchor_{id}` | 원래 이름 |
| 오류 메시지 | 소스 스니펫 없음 | 소스 스니펫 있음 |
| 빈 입력 | `[stream_start, stream_end]` | `[]` |
| 태그 핸들러 | 적용하지 않음 | 적용함 (YAML.parse) |

## 리소스 관리

조기에 중단할 때는 `close()`를 호출하십시오——이것이 유일하게 보장된 해제 지점입니다(PyPy의 지연 GC는 `Drop` 타이밍을 보장하지 않습니다). `close()`는 멱등적이며, 전달한 파일 객체를 **닫지 않습니다**.

## 스트리밍 쓰기

`YAML().dump_stream(file_obj, iterable, ...)` 및 `YAML().dump_file(path, iterable, ...)`
는 문서를 하나씩 직렬화하며 상수 메모리(O(단일 문서 + 64KB 청크))를 사용합니다.

```python
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

### 오류 의미론

중간에 실패(이터레이터 예외, 직렬화 오류, 쓰기 실패)하면 이미 작성된 출력이 대상에 남습니다——부분 출력에 대한 롤백은 없습니다.

### safe_dump와의 차이점

| 측면 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 출력 | 멀티 문서 스트림 | 단일 문서 |
| 메모리 | O(단일 문서 + 64KB) | O(입력) |
| 항목 유형 | `YamlDocument`(주석/앵커 유지) 또는 일반 Python 객체 | 단일 Python 객체 |

### 키 정렬

`sort_keys=True`를 전달하면 매핑 키를 정렬된 순서로 출력합니다. `safe_dump`의 `sort_keys` 동작과 동일합니다.
