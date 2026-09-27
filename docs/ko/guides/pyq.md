---
title: pyq - Rust 네이티브 CLI
description: pyrs-yaml-core 기반의 jq/yq 스타일 Rust CLI. 런타임 Python 없이 YAML·JSON·TOML·INI를 처리합니다.
tags:
  - docs
status: new
---

`pyq`는 Rust 네이티브 명령줄 도구입니다. `pyrs-yaml-core`를 jq/yq 스타일
인터페이스에 직접 연결하며 런타임에 Python이 필요 없습니다. Python 기반
[pyrs-yaml CLI](cli.md)를 보완하며, 두 도구는 같은 코어와 종료 코드·
에러 메시지 규격을 공유합니다.

## 설치

```bash
cargo install --path crates/pyrs-yaml-cli   # 소스 트리에서 설치
# 또는 리포지토리에서 빌드:
cargo build -p pyrs-yaml-cli --release      # -> target/release/pyq
```

## 쿼리 (jq 스타일)

```bash
# JSONPath-lite: 점 키, [n], [-n](파이썬식 음수 인덱스), ['key'], [*]
# 선행 점은 생략 가능, `.`만 쓰면 문서 전체입니다
$ pyq get '.servers[-1].host' inventory.yaml
web-3

# 와일드카드는 모든 매칭으로 확장되어 jq처럼 스트리밍됩니다:
# 매칭마다 YAML 문서 하나(--json이면 줄당 JSON 값 하나)
$ pyq get '.servers[*].port' --json inventory.yaml
8080
8081

# 파일이 `-`이거나 생략되면 stdin, --raw는 맨 스칼라 출력
$ cat services.yaml | pyq get --raw .db.pool.size
20

# JSON 출력 (키 순서 보존)
$ pyq get '.servers' --json services.yaml
[ { "host": "web-1", "port": 8080 }, ... ]
```

## 필터 동사 (jq 스타일 후처리)

표현식이 아닌 구조화 플래그—명령줄 순서와 무관하게 매칭 스트림에
고정 파이프라인 `select -> sort -> unique -> slice`, 끝에 `join`을 적용합니다.

```bash
pyq get '.servers[*]' --select 'port >= 1000' services.yaml
pyq get '.servers[*]' --sort-by host --desc services.yaml
pyq get '.tags[*]' --unique --skip 2 --take 5 blob.yaml
pyq get '.hosts[*]' --join ',' --raw inventory.yaml   # 맨 줄 텍스트
```

| 플래그 | jq 대응 | 참고 |
|--------|---------|------|
| `--select 'PATH OP LITERAL'` | `select(.PATH OP LITERAL)` | OP는 `== != > >= < <=`. 리터럴은 YAML. 경로 누락·타입 불일치는 false(jq 전순서 없음) |
| `--sort-by PATH` / `--desc` | `sort_by(.PATH)` | 안정 정렬. 키 누락은 마지막 |
| `--unique` | `unique` | 정렬 후 중복 제거, jq와 동일 |
| `--first` / `--last` | `.[0]` / `.[-1]` | 상호 배타 |
| `--skip N` / `--take N` | `.[N:][…]` | 스트림 슬라이스 |
| `--join SEP` | `join(SEP)` | 전체 스칼라 스트림만 |

## 편집 (yq 스타일)

```bash
# 값은 YAML 표현식(JSON도 가능, YAML의 상위 집합)
pyq set '.db.pool.size' 50 services.yaml          # 편집된 문서 출력
pyq set -i '.db.pool.size' 50 services.yaml       # 파일을 제자리에서 재작성
pyq set --create-missing '.a.b.c' 1 empty.yaml    # 중간 매핑 자동 생성
pyq delete '.legacy_field' -i config.yaml
pyq sort-keys '$' -i config.yaml                  # 매핑 한 계층 키순 정렬
```

모든 편집은 공유 splice 엔진을 통과합니다: 레이아웃이 조건을 만족하면
건드리지 않은 행(주석, 빈 줄, 불규칙 공백)은 바이트 단위로 그대로 유지됩니다.

편집된 출력은 `fmt`와 같은 라운드트립 직렬화기를 거칩니다: 주석·앵커·
키 순서가 보존되고, 삽입된 값은 자신의 표기 스타일을 유지합니다
(`[1, two]`는 플로우 그대로, `"true"`는 따옴표 문자열 그대로).

## 변환

```bash
pyq fmt k8s.yaml                 # 주석 보존 정규화
pyq fmt --explicit-start cfg.yaml
pyq to-json config.yaml          # YAML -> JSON (키 순서 보존)
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML (값은 모두 문자열)
pyq validate k8s.yaml --schema rules.yaml   # 파싱 검사 + 스키마 언어 검증
pyq frontmatter README.md --body-out body.md # Markdown 프런트매터 분리
```

입력 형식은 확장자로 판정(`.json`, `.toml`, `.ini`)하며
`--input yaml|json|toml|ini`로 덮어쓸 수 있습니다. YAML은 JSON의 상위
집합이므로 JSON 내용은 YAML 경로에서도 그대로 해석됩니다.

## 종료 코드

- `0` 성공;
- `1`: 경로 미발견, 해석 실패, TOML로 표현할 수 없는 구조(null 값,
  테이블이 아닌 루트)에서는 stderr에 `pyq: <메시지>` — Python API와
  동일한 안정적 메시지.

## 셸 자동 완성

```bash
pyq completion bash > /etc/bash_completion.d/pyq   # bash
pyq completion zsh  > "${functions[@]:0:1}/_pyq"   # zsh
pyq completion fish | source                        # fish
pyq completion powershell > pyq.ps1                 # PowerShell
```

## 지원 범위

| 기능 | pyq | pyrs-yaml CLI (Python) |
|------|-----|-------------------------|
| 쿼리 / set / delete / 서식 / 변환 | ✅ | ✅ |
| 동사 후처리 (`select`/`sort`/`unique`/…) | ✅ | — |
| 레이아웃 고정 편집 (splice 엔진) | ✅ | ✅ |
| 경로별 sort-keys | ✅ | ✅ |
| 다중 문서 조회 (`-A`: get/fmt/to-json) | ✅ | ✅ |
| rename / move / append / insert / frontmatter / `validate` (스키마 언어) | ✅ | ✅ |
| 다중 문서 편집 (`-A`와 set/delete) | 계획 중 | ✅ |

양쪽 모두 동일한 코어 splice 엔진으로 레이아웃 고정 편집을 합니다
(건드리지 않은 행, standalone 주석 포함, 드리프트 없음). 전체 기능은
Python CLI가 담당하고(`-A` 다중 문서, `validate`, rename/move/frontmatter),
`pyq`는 단일 문서의 빠르고 의존성 없는 스크립팅을 목표로 합니다.

## 관련 문서

- [명령줄 인터페이스](cli.md) — Python 기반 `pyrs-yaml` 명령
- [TOML, JSON, INI 형식](formats.md) — 라이브러리 측 변환 API
- [제자리 편집](editing.md) — `pyq`가 편집에 쓰는 라운드트립 모델
