---

title: 순환 보존
lang: ko

이것은 pyrs-yaml의 **핵심 기능** — Python YAML 라이브러리 중에서 독보적인 특징입니다.

## 순환 보존이란?

순환 보존은: **YAML 파싱 → 수정 → 직렬화 → 출력이 입력과 동일하거나 의미적으로 동등함**을 의미합니다.

```python
original = """
# 서버 설정
server:
  host: 0.0.0.0
  port: 8080  # 메인 포트

# 데이터베이스 앵커
database: &db
  host: localhost
  port: 5432

api:
  <<: *db
  endpoint: /api/v1
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# 모든 포맷과 메타데이터가 보존됨
assert "# 서버 설정" in output
assert "# 메인 포트" in output
assert "&db" in output
# 참고: 병합 키(<<)는 기본적으로 해석(실체화)되어 그대로 출력되지 않습니다.
# <<: *db를 그대로 유지하려면 resolve_merges=False를 사용하세요
```

## 보존되는 것

| 요소 | 보존 여부 | 설명 |
|------|----------|------|
| 독립형 주석 | ✅ | 키와 값 앞 |
| 인라인 주석 | ✅ | 줄 끝 |
| 앵커 (`&name`) | ✅ | 완전한 앵커 구문 |
| 별칭 (`*name`) | ✅ | 별칭 참조 해석됨 |
| 병합 키 (`<<`) | ⚠️ | 기본적으로 해석됨; `resolve_merges=False`로 유지 |
| 태그 (`!!str`, `!!int`) | ✅ | 명시적 태그 보존됨 |
| 스칼라 스타일 | ✅ | Plain, 따옴표, 리터럴, 폴드 |
| 청핑 (`\|-`, `>-`) | ✅ | 블록 스칼라 표시자 |
| 플로우/블록 스타일 | ✅ | `[]`/`{}` vs 블록 보존됨 |
| 컴팩트 시퀀스 항목 | ✅ | `- host: a`가 대시 줄에 유지됨 (메타데이터 없는 매핑 항목만) |
| 키 순서 | ✅ | `IndexMap`이 순서 보장 |

## PyYAML vs pyrs-yaml 순환 보존

```python
original = "# 주석\nkey: value  # 인라인\n"

# PyYAML: 모든 것을 잃음
yaml.safe_dump(yaml.safe_load(original))
# 출력: 'key: value\n'  ❌

# pyrs-yaml: 모든 것을 보존
doc = pyrs_yaml.parse(original)
doc.to_yaml()
# 출력: '# 주석\nkey: value  # 인라인\n'  ✅
```

## 성능

다른 라이브러리와의 순환 성능 비교:

| 라이브러리 | 순환 보존 (대용량) | 주석 | 앵커 | 태그 |
|-----------|------------------|------|------|------|
| **pyrs-yaml** | **0.08 ms** | ✅ | ✅ | ✅ |
| PyYAML | 2.98 ms | ❌ | ❌ | ❌ |
| ruamel.yaml | 6.79 ms | ✅ | ✅ | ✅ |

**pyrs-yaml는 PyYAML보다 37배, ruamel.yaml보다 85배 빠르면서** 모든 것을 보존합니다.
