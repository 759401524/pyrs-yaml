---
title: pyrs-yaml
description: 높은 성능의 Python YAML 라이브러리, 완벽한 순환 지원, Rust 및 PyO3로 구축됨
tags:
  - docs
status: new
---

## 높은 성능의 Python YAML 라이브러리, 완벽한 순환 지원, Rust 및 PyO3로 구축됨

### pyrs-yaml를 선택하는 이유

대부분의 Python YAML 라이브러리는 성능 또는 정확도 중 하나를 희생합니다. pyrs-yaml는 둘을 모두 제공합니다:

- **PyYAML** (Python) — 느리고, 순환 파싱 시 **주석/앵커/태그를 잃음**
- **ruamel.yaml** (Python) — 서식을 유지하지만 pyrs-yaml보다 **파싱 48–100배, 직렬화 123–371배 느림**
- **pyrs-yaml** (Rust) — PyYAML보다 **파싱 21–43배, 직렬화 55–177배 빠르며** 모든 것을 유지

#### 주요 기능

- **YAML 1.2 준수** — granit-parser 기반 (YAML 테스트 스위트 99.75% 통과율, 405/406)
- **완벽한 순환 파싱** — 주석, 앵커, 태그, chomp, 스칼라 스타일 및 흐름/블록 서식 유지
- **제자리 편집** — JSONPath 스타일 경로(`doc.set("$.a.b", v)`) 또는 `Node` 트리 API로 파싱된 문서를 서식 손실 없이 편집
- **PyYAML보다 파싱 21–43배, 직렬화 55–177배 빠름** — Rust 백엔드, 제로 복사 파싱
- **커스텀 AST** — 고급 YAML 조작 및 사용자 정의 서식을 위한 확장 가능한 AST
- **PyYAML 호환** — `safe_load` / `safe_dump` API로 직접 교체 가능
- **타입 힌트** — PEP 561 준수, 완전한 `.pyi` 스텁 파일
- **ABI3** — 단일 휠로 Python 3.8–3.15 지원
- **국제화 오류 메시지** — `set_language("ko")`로 이중 언어 오류 보고
- **NumPy ndarray 지원** — 모든 차원의 `numpy.ndarray`를 제로 복사 Rust 디스패치로 YAML에 직렬화

#### 빠른 시작

```bash
pip install pyrs-yaml
```

```python
import pyrs_yaml

# Parse YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML compatible API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

#### PyYAML와의 성능 비교

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.18 ms | 3.8 ms | **21×** |
| Parse (medium) | 0.56 ms | 24.2 ms | **43×** |
| Parse (large) | 1.5 ms | 57.7 ms | **38×** |
| Serialize (small) | 0.04 ms | 2.2 ms | **55×** |
| Serialize (medium) | 0.08 ms | 12.6 ms | **159×** |
| Serialize (large) | 0.17 ms | 30.2 ms | **177×** |

---

### [시작하기 →](quick-start.md)

### [API 참조 보기 →](api/reference.md)

### [GitHub에서 보기 →](https://github.com/759401524/pyrs-yaml)
