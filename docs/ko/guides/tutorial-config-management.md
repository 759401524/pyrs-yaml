---
title: pyrs-yaml로 구성 관리
description: 모든 메타데이터를 보존하면서 YAML 구성 파일을 파싱·편집·검증·재직렬화하는 방법을 보여주는 엔드투엔드 튜토리얼.
tags:
  - docs
  - tutorial
status: new
---

## pyrs-yaml로 구성 관리

이 튜토리얼에서는 실제 시나리오를 다룹니다: 마이크로서비스 애플리케이션의 YAML 구성 파일 관리. 주석, 앵커, 태그, 서식을 모두 보존하면서 YAML 파일을 파싱하고, 검사하고, 편집하고, 검증하고, 다시 쓰는 방법을 배웁니다.

## 설정

```bash title="PyPI에서 설치"
pip install pyrs-yaml
```

## 1. 구성 파일

주석, 앵커, 병합 키, 블록/플로우 혼합 서식이 포함된 YAML 구성 파일로 시작합니다:

```yaml title="config.yaml"
# Application configuration (v2.0)
app:
  name: my-service
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  staging:
    <<: *default-db
    host: staging.example.com
    debug: true

  production:
    <<: *default-db
    host: prod.example.com
    port: 5432
    debug: false

# Feature flags
features:
  - name: login
    enabled: true
  - name: export
    enabled: true
  - name: reporting
    enabled: false

# Custom scalar example
threshold: 0x1F  # hex value (should be parsed as int)
```

## 2. 파일 파싱

```python title="파일 파싱"
import pyrs_yaml

doc = pyrs_yaml.parse_file("config.yaml")
print(f"Parsed: {doc.get('app.name')} v{doc.get('app.version')}")
# Parsed: my-service v2.0
```

**핵심**: 모든 주석, 앵커, 태그, 서식이 메모리에 보존됩니다. 문서는 `YamlDocument` 객체이지 원시 Python dict가 아닙니다.

## 3. 값 검사

경로 API(JSONPath 스타일) 또는 Node API(트리 기반)를 사용합니다:

```python title="값 검사"
# Path API — simple and direct
db_host = doc.get("database.host")
print(f"Database host: {db_host}")

# Node API — access metadata and formatting
db_node = doc.node().find("$.database")
print(f"Database is flow style: {db_node.flow_style}")  # False (block)
print(f"Database anchor: {db_node.anchor}")  # "default-db"
```

## 4. 메타데이터를 보존한 값 편집

값을 편집하면 해당 주석, 앵커, 태그, 따옴표 스타일이 보존됩니다. 편집은 AST에서 직접 수행되며 문자열 조작이 아닙니다:

```python title="제자리 값 편집"
# Change the production port
doc.set("$.environments.production.port", 5444)

# Change the app name while keeping its comment
doc.set("$.app.name", "my-service-v2")

# Add a comment to document a change
prod_node = doc.node().find("$.environments.production")
prod_node.set_comment("overridden for v2 rollout")
```

## 5. 메타데이터 조작

pyrs-yaml은 값 편집을 넘어 YAML 메타데이터 자체를 읽고 쓸 수 있습니다:

```python
# Read existing metadata
debug_node = doc.node().find("$.environments.staging.debug")
print(f"Debug comment: {debug_node.comment}")  # None

# Add a tag to document a custom type
import_node = doc.node().find("$.threshold")
import_node.set_tag("!!int")
print(f"Threshold tag: {import_node.tag}")  # "!!int"

# Add an anchor for later reference
prod_db = doc.node().find("$.environments.production")
prod_db.set_anchor("prod-db")
```

## 6. 서식 제어

스칼라 따옴표, 블록/플로우 레이아웃, 촙핑 지시자를 전환합니다:

```python
# Switch the threshold to single-quoted for clarity
doc.node().find("$.threshold").set_scalar_style("single_quoted")

# Switch the staging environment to compact flow style
staging = doc.node().find("$.environments.staging")
staging.set_flow_style(True)
```

## 7. 와일드카드로 일괄 편집

`set_many`를 사용하여 일치하는 모든 경로에 변경을 적용합니다 — 토글류 작업에 유용합니다:

```python title="와일드카드 일괄 편집"
# Disable ALL debug flags across every environment
doc.set_many(
    {
        "$.environments[*].debug": False,
    }
)

# Disable all features at once
doc.set_many(
    {
        "$.features[*].enabled": False,
    }
)
```

## 8. 키 정렬

가독성을 위해 최상위 키와 환경 키를 정렬합니다:

```python title="키 정렬"
doc.sort_keys()  # sort the root mapping
doc.sort_keys("$.environments")  # sort the environments
```

## 9. 스키마 검증

구조 규칙이 있는 스키마를 정의하고 구성을 검증합니다:

```python title="스키마 검증"
schema = """\
name: app-config
extends: core
validate:
  - path: $.app.name
    type: str
    required: true
  - path: $.environments.*.debug
    type: bool
  - path: $.threshold
    type: int
"""

# Validate — raises YamlValidateError on failure
pyrs_yaml.validate_against_schema(doc.to_yaml(), schema)
print("Configuration is valid!")
```

## 10. 하위 트리 딥 카피

하위 트리를 (문서에서 분리된) 독립 Python 값으로 복사합니다:

```python title="하위 트리 딥 카피"
# Copy the staging configuration for reuse
staging_config = doc.node().find("$.environments.staging").copy()
print(staging_config)  # {'host': 'staging.example.com', 'debug': False, ...}
```

## 11. 하위 트리 이동

같은 문서 안에서 하위 트리를 이동합니다:

```python title="하위 트리 이동"
# Move the reporting feature to a new section
doc.node().find("$.features[2]").move("$.deprecated-features")
```

## 12. 파일에 다시 쓰기

마지막으로 편집된 문서를 YAML로 직렬화합니다:

```python title="파일에 다시 쓰기"
output = doc.to_yaml()
with open("config-updated.yaml", "w", encoding="utf-8") as f:
    f.write(output)
```

출력은 **모든 것**을 보존합니다 — 주석, 앵커, 병합 키, 서식, 그리고 우리가 만든 모든 편집:

```yaml title="config-updated.yaml"
# Application configuration (v2.0)
app:
  name: my-service-v2
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  # overridden for v2 rollout
  production: &prod-db
    <<: *default-db
    host: prod.example.com
    port: 5444
    debug: false

  staging:
    <<: *default-db
    debug: false
    host: staging.example.com
```

## 요약

이 튜토리얼에서 다음을 수행했습니다:

- :material-file-code: 메타데이터를 완전히 보존하며 YAML 파일을 **파싱**
- :material-magnify: 경로 API와 Node API로 값을 **검사**
- :material-pencil: 값, 주석, 앵커, 태그, 서식을 **편집**
- :material-format-list-bulleted: `set_many`로 와일드카드 **일괄 편집**
- :material-sort: 가독성을 위해 키를 **정렬**
- :material-check-decagram: 스키마에 대해 **검증**
- :material-content-copy: 하위 트리를 **복사** 및 **이동**
- :material-sync: 모든 것을 보존한 채 YAML로 **직렬화**

### 다음 단계

- :material-rocket-launch: [빠른 시작](../quick-start.md) — 몇 분 안에 시작
- :material-pencil: [제자리 편집 가이드](../guides/editing.md) — 편집 API 전체 레퍼런스
- :material-check-decagram: [사용자 지정 스키마 가이드](../guides/custom-schema.md) — 나만의 스키마 정의
- :material-book-open-page-variant: [API 레퍼런스](../api/reference.md) — 전체 API 문서
