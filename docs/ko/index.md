---
title: pyrs-yaml
description: 높은 성능의 Python YAML 라이브러리, 완벽한 순환 지원, Rust 및 PyO3로 구축됨
tags:
  - docs
status: new
---

<div class="hero" markdown>

## pyrs-yaml

**높은 성능의 Python YAML 라이브러리, 완벽한 순환 지원, Rust 및 PyO3로 구축됨**

[:material-rocket-launch: 시작하기 :material-arrow-right:](quick-start.md){ .md-button .md-button--primary }
[:material-code-braces: API 참조](api/reference.md){ .md-button }
[:fontawesome-brands-github: GitHub에서 보기](https://github.com/759401524/pyrs-yaml){ .md-button }

<div class="badges" markdown>

- :material-check-decagram: YAML 1.2 준수
- :material-package-variant-closed: ABI3 휠
- :material-numeric: Python 3.8–3.15
- :material-language-python: 타입 힌트
- :material-lightning-bolt: 프리-스레디드 지원

</div>

</div>

### pyrs-yaml을 선택하는 이유

대부분의 Python YAML 라이브러리는 성능 또는 정확도 중 하나를 희생합니다. pyrs-yaml은 둘을 모두 제공합니다:

- **PyYAML** (Python) — 느리고, 순환 파싱 시 **주석/앵커/태그를 잃음**
- **ruamel.yaml** (Python) — 서식을 유지하지만 pyrs-yaml보다 **파싱 48–100배, 직렬화 123–371배 느림**
- **pyrs-yaml** (Rust) — PyYAML보다 **파싱 21–43배, 직렬화 55–177배 빠르며** 모든 것을 유지

### 주요 기능

<div class="grid cards" markdown>

- :material-lightning-bolt: **초고속** — PyYAML보다 파싱 21–43배, 직렬화 55–177배 빠름, Rust 제로 복사 백엔드 구동
- :material-sync: **완벽한 순환 파싱** — 주석, 앵커, 태그, chomp, 스칼라 스타일 및 흐름/블록 서식 유지
- :material-pencil: **제자리 편집** — JSONPath 스타일 경로(`doc.set("$.a.b", v)`) 또는 `Node` 트리 API로 서식 손실 없이 편집
- :material-check-decagram: **YAML 1.2 준수** — granit-parser 기반 (YAML 테스트 스위트 99.75% 통과율, 405/406)
- :material-swap-horizontal: **PyYAML 호환** — `safe_load` / `safe_dump` API로 직접 교체 가능
- :material-language-python: **타입 힌트** — PEP 561 준수, 완전한 `.pyi` 스텁 파일
- :material-package-variant-closed: **ABI3 휠** — 단일 휠로 Python 3.8–3.15 지원
- :material-translate: **국제화 오류** — `set_language("ko-KR")`로 이중 언어 오류 보고
- :material-numeric: **NumPy ndarray** — 모든 차원의 `numpy.ndarray`를 제로 복사 Rust 디스패치로 직렬화

</div>

### 빠른 시작

```bash title="설치"
pip install pyrs-yaml
```

```python title="빠른 시작"
import pyrs_yaml

# YAML 파싱
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML 호환 API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 라운드트립으로 주석 유지
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

### PyYAML과의 성능 비교

| 작업 | pyrs-yaml | PyYAML | 속도 향상 |
|-----------|-----------|--------|---------|
| Parse (small) | 0.18 ms | 3.8 ms | **21×** |
| Parse (medium) | 0.56 ms | 24.2 ms | **43×** |
| Parse (large) | 1.5 ms | 57.7 ms | **38×** |
| Serialize (small) | 0.04 ms | 2.2 ms | **55×** |
| Serialize (medium) | 0.08 ms | 12.6 ms | **159×** |
| Serialize (large) | 0.17 ms | 30.2 ms | **177×** |
