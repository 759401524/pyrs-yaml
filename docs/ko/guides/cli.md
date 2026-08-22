---
title: 커맨드라인 인터페이스
description: pyrs-yaml CLI를 사용해 터미널에서 YAML 파일을 라운드트립 충실도로 포맷, 쿼리, 편집, 검증, 변환합니다.
tags:
  - docs
status: new
---

## 커맨드라인 인터페이스

pyrs-yaml은 선택적 커맨드라인 도구 `pyrs-yaml`을 제공합니다. 라이브러리의 핵심 기능——라운드트립 포매팅, JSONPath 쿼리, 제자리 편집, 스키마 검증, 형식 변환——을 터미널에서 바로 사용할 수 있습니다.

!!! note "요구 사항"
    CLI에는 선택적 `cli` extra와 **Python >= 3.10**이 필요합니다. 라이브러리 자체는 계속해서 이전 인터프리터를 지원합니다.

### 설치

=== "pip"

    ```bash
    pip install "pyrs-yaml[cli]"
    ```

=== "uv"

    ```bash
    uv add --optional cli pyrs-yaml
    ```

설치 확인:

```bash
pyrs-yaml --version
```

### 명령 개요

| 명령 | 용도 |
|------|------|
| [`fmt`](#cmd-fmt) | 주석·앵커·순서를 보존하며 재포매팅 |
| [`get`](#cmd-get) | JSONPath 표현식으로 값 조회 |
| [`set`](#cmd-edit) | 경로 위치의 값 설정 |
| [`delete`](#cmd-edit) | 경로 위치의 노드 삭제 |
| [`rename`](#cmd-edit) | 매핑 키 이름 변경 |
| [`validate`](#cmd-validate) | 스키마 기준으로 YAML 검증 |
| [`to-json`](#cmd-convert) | YAML → JSON 변환 |
| [`from-json`](#cmd-convert) | JSON → YAML 변환 |

파일 인수가 `-`이거나 생략되면 **stdin**에서 읽고, `-o/--output` 또는 `-i/--inplace`가 없는 한 결과는 **stdout**에 출력됩니다.

### 포매팅(`fmt`) { #cmd-fmt }

`fmt`는 라운드트립 AST를 통해 문서를 다시 직렬화합니다——주석, 앵커, 키 순서, 스타일이 모두 유지됩니다:

```bash
$ echo "a:    1 # keep me" | pyrs-yaml fmt -
a: 1  # keep me
```

주요 옵션:

```bash
pyrs-yaml fmt config.yaml --indent 4        # 4칸 들여쓰기
pyrs-yaml fmt config.yaml --inplace         # 파일 제자리 재작성 (-i)
pyrs-yaml fmt config.yaml -o formatted.yaml # 다른 파일에 출력
```

### 쿼리(`get`) { #cmd-get }

`get`은 [JSONPath](editing.md) 스타일 표현식을 평가하고 일치하는 각 노드를 출력합니다:

```bash
$ pyrs-yaml get deploy.yaml '$.servers[0].host'
db.example.com

$ pyrs-yaml get deploy.yaml '$..name' --format text   # 깊이 탐색
web
db

$ pyrs-yaml get deploy.yaml '$.servers[*]'            # 서브트리는 YAML로 출력 (기본값)
```

`--format/-f`로 출력 형식을 지정합니다: `yaml`(기본값), `json`, `text`(스칼라 값 그대로).

### 편집(`set`, `delete`, `rename`) { #cmd-edit }

편집 명령의 경로는 정확히 하나의 노드를 가리켜야 합니다(와일드카드 불가):

```bash
# VALUE는 YAML로 파싱됩니다——숫자, 불리언, 중첩 구조도 그대로 사용할 수 있습니다
pyrs-yaml set config.yaml "$.retries" 5
pyrs-yaml set config.yaml "$.tags" '[a, b]'
pyrs-yaml set config.yaml "$.token" '12345' --string          # 문자열로 강제
pyrs-yaml set config.yaml "$.a.b.c" new --create-missing      # 부모 자동 생성

pyrs-yaml delete config.yaml "$.legacy_key"
pyrs-yaml rename config.yaml "$.old_name" new_name

pyrs-yaml set config.yaml "$.port" 8080 --inplace             # 파일 제자리 수정
```

편집 시 주변 메타데이터는 보존됩니다——편집한 노드 위나 행 내부의 주석은 그대로 남습니다.

### 검증(`validate`) { #cmd-validate }

`validate`는 스키마 정의(파일 경로) 또는 등록된 스키마 이름을 기준으로 문서를 검사합니다:

```bash
pyrs-yaml validate app.yaml --schema schema.yaml
pyrs-yaml validate app.yaml --schema my_schema        # register_schema()로 등록된 것
```

성공 시 조용히 종료 코드 `0`으로 끝나고, 실패 시 모든 위반 항목을 stderr에 출력하고 종료 코드 `1`로 끝납니다——CI에서 활용하기 좋습니다:

```yaml
# schema.yaml
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
```

스키마 언어 전체는 [사용자 정의 스키마](custom-schema.md)를 참고하세요.

### 변환(`to-json`, `from-json`) { #cmd-convert }

양방향 명령 모두 파이프라인과 자연스럽게 조합됩니다:

```bash
$ pyrs-yaml to-json config.yaml
{
  "b": {
    "c": 2
  }
}

$ echo '{"name": "x"}' | pyrs-yaml from-json -
name: x
```

### 종료 코드

| 코드 | 의미 |
|------|------|
| `0` | 성공 |
| `1` | 런타임 오류——입력을 읽을 수 없음, 파싱 실패, 매치 없음, 검증 실패 |
| `2` | 사용법 오류——알 수 없는 명령 또는 옵션 |

!!! tip "스크립트 활용"
    오류는 stderr로, 데이터는 stdout으로 나오므로 `pyrs-yaml`은 파이프라인과 깔끔하게 조합됩니다: `pyrs-yaml get deploy.yaml '$..host' | sort -u`.
