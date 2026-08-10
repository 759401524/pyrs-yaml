---

title: Quick Start
lang: ko

# 빠른 시작

이 가이드를 통해 몇 분 안에 pyrs-yaml을 시작할 수 있습니다.

## 1. 설치

패키지는 아직 PyPI에 게시되지 않았습니다. 소스에서 설치:

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 2. YAML 파싱

```python
import pyrs_yaml

# YAML 문자열 파싱
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# 값 접근
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

## 3. Python 객체로 변환

```python
# PyYAML 호환 동작을 위해 safe_load 사용
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# 기본 Python 타입(dict, list, str, int 등)을 반환합니다
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

## 4. YAML로 직렬화

```python
# Python 딕셔너리를 YAML로 변환
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

## 5. 서식 보존 (Round-Trip)

```python
# pyrs-yaml의 핵심 장점
original = """
# 서버 설정
server:
  host: 0.0.0.0
  port: 8080

# 데이터베이스 설정
database: &db
  host: localhost
  port: 5432

# 데이터베이스 앵커 사용
api:
  <<: *db
  endpoint: /api/v1
"""

# 파싱 및 직렬화 — 주석과 앵커 보존
doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# 출력은 입력과 일치합니다 (또는 의미상 동일함)
assert "# 서버 설정" in output
assert "&db" in output
```

## 6. 제자리 편집

```python
# 주석이나 서식을 잃지 않고 파싱된 문서 편집
doc = pyrs_yaml.parse("""
server:
  host: localhost  # 바인딩 주소
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # 경로로 교체
doc.append("$.server.ports", 443)  # 시퀀스에 추가

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # 바인딩 주소
#   ports:
#     - 8080
#     - 443
```

전체 API는 [제자리 편집 가이드](guides/editing.md)를 참조하세요.

## 7. 파일에서 YAML 읽기

```python
# YAML 파일 직접 파싱
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

## 8. 여러 문서

```python
# 여러 YAML 문서 파싱
yaml_text = """
---

name: config1
value: 1
---
name: config2
value: 2
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # config1
```

## 9. NumPy ndarray 지원

pyrs-yaml은 `numpy.ndarray` 객체를 직접 YAML로 직렬화할 수 있습니다. 이는 과학 데이터, 모델 가중치 또는 다차원 배열을 사람이 읽을 수 있는 형식으로 저장하는 데 유용합니다.

```python
import numpy as np
import pyrs_yaml

# 1차원 배열
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2차원 행렬
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip으로 값 보존
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### 지원되는 NumPy dtype

| NumPy dtype | YAML 출력 | 참고 |
|-------------|-------------|-------|
| `int8/16/32/64` | 일반 정수 | 음수일 때 따옴표 |
| `uint8/16/32/64` | 일반 정수 | — |
| `float32/64` | 일반 부동소수 | 음수일 때 따옴표 |
| `complex64/128` | `(re+imj)` 문자열 | YAML에는 복잡한 타입 없음 |
| `bool` | `true` / `false` | — |

## 다음 단계

- **[기능](features.md)** — 지원되는 모든 YAML 기능 탐색
- **[파싱 가이드](guides/parsing.md)** — 고급 파싱 옵션
- **[제자리 편집](guides/editing.md)** — 서식을 잃지 않고 문서 편집
- **[API 참조](api/reference.md)** — 완전한 API 문서
