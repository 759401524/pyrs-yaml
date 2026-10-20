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
pyq fmt --indent 4 --width 0 cfg.yaml   # 블록 들여쓰기 4, 스칼라 줄바꿈 없음
pyq fmt --sort-keys cfg.yaml     # 모든 mapping 키순 정렬 (문서 전체)
pyq fmt --indent 4 -i cfg.yaml   # 파일을 그 자리에서 다시 쓰기
pyq to-json config.yaml          # YAML -> JSON (키 순서 보존)
pyq to-json --jsonc config.yaml  # AST에 남은 주석을 그대로 재출력
pyq to-json --json5 config.yaml  # JSON5 표기 ('x', .5, +7, Infinity)
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML (값은 모두 문자열)
pyq validate k8s.yaml --schema rules.yaml   # 파싱 검사 + 스키마 언어 검증
pyq frontmatter README.md --body-out body.md # Markdown 프런트매터 분리
```

입력 형식은 확장자로 판정(`.json`, `.jsonc`, `.json5`, `.toml`, `.ini`)하며
`--input yaml|json|jsonc|json5|toml|ini`로 덮어쓸 수 있습니다. YAML은 JSON의 상위
집합이므로 JSON 내용은 YAML 경로에서도 그대로 해석됩니다. JSONC/JSON5 내용은
네이티브 방언 엔진을 거쳐 주석과 JSON5 표기가 AST에 실리므로,
`pyq to-json --input jsonc --jsonc`로 주석 보존 왕복이 됩니다.

## 비교와 병합

```bash
pyq diff base.yaml head.yaml           # 시맨틱 비교: 동일 0, 차이 1
pyq merge base.yaml overrides.yaml     # 딥 병합, 오른쪽 우선 (yq `*+` 형태)
pyq merge a.yaml b.yaml --replace-arrays
```

`diff`는 해석된 값·구조·태그만 비교합니다 — 주석, 인용부호, 레이아웃은
형식 노이즈로 전혀 나타나지 않습니다. 차이 경로는
`- .keep: x` / `+ .c: 4` / `~ .b: 2 -> 3` 형식으로 출력됩니다.
`merge`는 mapping을 재귀 병합하고 sequence는 뒤로 추가하며
(`--replace-arrays`로 통째 교체), 왕복 YAML로 출력합니다.

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

## jq / yq 호환 셸

첫 인수가 pyq 자체 하위 명령이 아닌 argv는 `jq` 1.8 또는 mikefarah `yq` v4로 읽혀 같은 의미의 하위
명령으로 번역됩니다. 두 도구를 위해 작성된 스크립트는 이 바이너리에서 그대로 동작합니다. 방언은
실행 이름이 정하므로, 이 바이너리를 가리키는 `jq`/`yq` 심볼릭 링크가 대체하는 도구처럼 행동합니다.
`--as jq|yq`로 명시할 수도 있습니다.

| 받아들임 | 의미 |
|----------|------|
| `.a.b`, `.list[]`, `$.a`, `[0]` | 경로 → `pyq get` (yq의 `[]`는 pyq의 `[*]`) |
| `.a.b = <리터럴>` | 대입 → `pyq set --create-missing` |
| `del(<경로>)` | 단일 경로 → `pyq delete` |
| `select(<상대 경로 OP 리터럴>)` | → `pyq get --select`, 그리고 끝에 붙은 `\| select` 하나 |
| `jq -r`, `yq -r/--unwrapScalar` | 스칼자를 맨 값으로 (yq에서는 기본 켜짐) |
| `yq -i/--inplace` | 파일을 그 자리에서 갱신 |
| `yq -N/--no-doc` | 스트림을 `---` 구분자 없이 출력 (yq에서는 기본 켜짐) |
| `-o json\|yaml`, `-p json\|yaml\|toml\|ini` | 출력·입력 형식 |
| `-f PROG`, `--from-file F` | 파일에서식을 읽어 다른 식과 똑같이 판정 |

실제 도구와 같은 문서를 돌려줄 수 없는 것은 모두 **이름을 짚어 거부**하고 종료 코드 2에 stdout은 비워
둡니다. 묵묵히 무시하는 일은 없습니다. 거부는 적어도 jq의 `--arg`/`--argjson`, `-s`, `-c`, `-S`,
`--stream`과 jq 모드의 모든 편집 식 (pyq는 YAML을 쓰고 jq는 JSON을 씀), yq의 `-n`, `-e`, `-P`,
`--expression`, `--split-exp`, `--front-matter`, `--xml-*`/`--lua-*`/`--csv-*` 각 계열과
문자열 보간, 끝에 붙은
`select` 하나를 뺀 모든 파이프, 그리고 모든 함수 (`keys`, `length`, `map`, `to_entries`, `env` 등)
입니다.

받아들임 표는 손으로 쓴 기대값이 아닙니다.
`crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs`는 `PATH`에 진짜 `jq`와 `yq`가 있으면 그것들을
실행해 stdout을 바이트 단위로 비교하고, 이 워크스페이스 안의 파일을 참조로 쓰는 것을 거부합니다 —
자기 자신의 사본을 비교 대상으로 삼는 프로브는 아무것도 증명하지 못하기 때문입니다.

## 지원 범위

| 기능 | pyq | pyrs-yaml CLI (Python) |
|------|-----|-------------------------|
| 쿼리 / set / delete / 서식 / 변환 | ✅ | ✅ |
| 동사 후처리 (`select`/`sort`/`unique`/…) | ✅ | — |
| 레이아웃 고정 편집 (splice 엔진) | ✅ | ✅ |
| 경로별 sort-keys | ✅ | ✅ |
| 다중 문서 조회 (`-A`: get/fmt/to-json) | ✅ | ✅ |
| rename / move / append / insert / frontmatter / `validate` (스키마 언어) | ✅ | ✅ |
| 다중 문서 편집 (`-A` 모든 편집 명령, `to-json -A`) | ✅ | ✅ |

양쪽 모두 동일한 코어 plan/splice 엔진으로 레이아웃 고정 편집을 합니다.
스트림에서 `pyq -A`는 한 걸음 더 나아갑니다: 문서마다 독립적인 splice 상태를
가져 건드리지 않은 문서와 모든 `---` 구분자가 바이트 단위로 그대로 유지되고,
레이아웃이 더러운 문서만 개별적으로 폴백합니다(`to-json -A`는 JSON 배열 출력).
Python CLI의 남은 강점은 CustomType 등록 검증으로, Python 계층의 개념입니다.

## 관련 문서

- [명령줄 인터페이스](cli.md) — Python 기반 `pyrs-yaml` 명령
- [TOML, JSON, INI 형식](formats.md) — 라이브러리 측 변환 API
- [제자리 편집](editing.md) — `pyq`가 편집에 쓰는 라운드트립 모델
