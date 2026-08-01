---

title: Quick Start
lang: ko

## 빠른 시작

이 가이드는 몇 분 만에 pyrs-yaml를 사용하도록 안내합니다.

### 1. 설치

패키지는 아직 PyPI에 게시되지 않았습니다. 소스에서 설치:

```bash
uv run --frozen maturin develop --release
```

### 2. YAML 파싱

```python
import pyrs_yaml

# Parse a YAML string
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# Access values
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

### 3. Python 객체로 변환

```python
# Use safe_load for PyYAML-compatible behavior
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# Returns native Python types (dict, list, str, int, etc.)
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

### 4. YAML로 직렬화

```python
# Convert a Python dict back to YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

### 5. 서식 유지 (순환 파싱)

```python
# The key advantage of pyrs-yaml
original = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080

# Database settings
database: &db
  host: localhost
  port: 5432

# Use the database anchor
api:
  <<: *db
  endpoint: /api/v1
"""

# Parse and re-serialize — comments and anchors preserved
doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output
assert "&db" in output
```

### 6. 제자리 편집

```python
# 주석이나 서식을 잃지 않고 파싱된 문서 편집
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")   # 경로로 교체
doc.append("$.server.ports", 443)     # 시퀀스에 추가

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 8080
#     - 443
```

전체 API는 [제자리 편집 가이드](guides/editing.md)를 참조하세요.

### 7. 파일에서 YAML 읽기

```python
# Parse a YAML file directly
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### 8. 여러 문서

```python
# Parse multiple YAML documents
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

pyrs-yaml는 `numpy.ndarray` 객체를 직접 YAML로 직렬화할 수 있습니다. 이는 과학 데이터, 모델 가중치 또는 다차원 배열을 사람이 읽을 수 있는 형식으로 저장하는 데 유용합니다.

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip preserves values
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### 지원되는 NumPy dtype

| NumPy dtype | YAML output | Notes |
|-------------|-------------|-------|
| `int8/16/32/64` | Plain integer | Quoted if negative |
| `uint8/16/32/64` | Plain integer | — |
| `float32/64` | Plain float | Quoted if negative |
| `complex64/128` | `(re+imj)` string | No native YAML complex type |
| `bool` | `true` / `false` | — |

### 다음 단계

- **[기능](features.md)** — 지원되는 모든 YAML 기능 탐색
- **[파싱 가이드](guides/parsing.md)** — 고급 파싱 옵션
- **[제자리 편집](guides/editing.md)** — 서식을 잃지 않고 문서 편집
- **[API 참조](api/reference.md)** — 완전한 API 문서
