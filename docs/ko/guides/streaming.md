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
