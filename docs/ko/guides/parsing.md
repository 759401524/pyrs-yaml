---

title: YAML 파싱
lang: ko


이 가이드는 pyrs-yaml로 YAML를 파싱하는 모든 방법을 설명합니다.

## 기본 파싱

#### YAML 문자열 파싱

```python
import pyrs_yaml

doc = pyrs_yaml.parse("key: value")
print(doc.get("key"))  # value
```

#### 옵션 파싱

```python
# 병합 키 해석 비활성화 (<<: *alias를 그대로 유지)
doc = pyrs_yaml.parse(yaml_text, resolve_merges=False)
```

#### YAML 파일 파싱

```python
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

#### 여러 문서 파싱

```python
# --- 구분자로 분리된 YAML
yaml_text = """
---

name: first
---
name: second
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

## PyYAML 호환 파싱

```python
# 네이티브 Python 타입을 반환 (dict, list, str, int 등)
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 여러 문서
docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

## 지원되는 입력 타입

- `str` — 표준 YAML 문자열
- `bytes` — 유효한 UTF-8 인코딩 바이트
- `str` BOM 포함 — 올바르게 처리됨

```python
# str과 bytes 모두 허용
doc1 = pyrs_yaml.parse("key: value")
doc2 = pyrs_yaml.parse(b"key: value")
```

## 오류 처리

```python
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"파싱 오류: {e}")
```

## 지원되는 데이터 타입

pyrs-yaml는 모든 YAML 1.2 스칼라 타입을 올바르게 파싱합니다.

| 타입 | 예시 | Python 타입 |
|------|------|------------|
| 문자열 | `hello` | `str` |
| 정수 | `42`, `0x1A`, `0o77` | `int` |
| 부동소수점 | `3.14`, `1.23e-4` | `float` |
| 부울 | `true`, `false` | `bool` |
| 널 | `null`, `~` | `None` |
| 무한대 | `.inf`, `-.inf` | `float` |
| NaN | `.nan` | `float` |
