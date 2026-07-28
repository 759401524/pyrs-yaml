---

title: 직렬화
lang: ko

## 직렬화

Python 객체와 `YamlDocument` 인스턴스를 YAML 문자열로 변환합니다.

### 기본 직렬화

#### `YamlDocument.to_yaml()`

```python
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()
print(yaml_str)  # key: value\n
```

#### `YamlDocument.to_yaml_with_options()`

```python
doc = pyrs_yaml.parse("key: value")

# 사용자 정의 들여쓰기와 문서 마커
yaml_str = doc.to_yaml_with_options(
    indent_size=4,           # 들여쓰기 레벨당 4 공백
    explicit_start=True,     # 시작에 "---" 추가
    explicit_end=True,       # 끝에 "..." 추가
    sort_keys=True,          # 키를 알파벳 순으로 정렬
)
```

#### PyYAML 호환 직렬화

```python
# dict를 YAML 문자열로 변환
yaml_str = pyrs_yaml.safe_dump({
    "database": {
        "host": "localhost",
        "port": 5432
    }
})

# safe_dumps (별칭)도 사용 가능
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

### Python 객체를 YAML로 변환

#### `from_dict()`

```python
yaml_str = pyrs_yaml.from_dict({
    "name": "Alice",
    "age": 30,
    "tags": ["admin", "user"]
})
```

#### `from_json()`

```python
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

#### `dump_file()`

```python
# Python 객체를 직접 YAML 파일에 쓰기
pyrs_yaml.dump_file({
    "config": {
        "debug": True,
        "log_level": "info"
    }
}, "output.yaml")
```

### 지원되는 입력 타입

| Python 타입 | YAML 출력 |
|------------|----------|
| `dict` | YAML 매핑 |
| `list` | YAML 시퀀스 |
| `str` | Plain 또는 따옴표 스칼라 |
| `int` | Plain 정수 |
| `float` | Plain 부동소수점 |
| `bool` | `true` / `false` |
| `None` | `null` |

### 순환 보존

```python
# 핵심 장점: 포맷이 보존됨
original = """
# 서버 설정
server:
  host: 0.0.0.0
  port: 8080  # 메인 포트

database: &db
  host: localhost

api:
  <<: *db
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# 주석, 앵커, 병합 키가 보존됨
assert "# 서버 설정" in output
assert "&db" in output
assert "<<: *db" in output
```
