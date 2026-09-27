---
title: TOML, JSON, INI 형식
description: pyrs-yaml로 TOML·JSON·INI를 주고받되 편집 가능한 표현은 YAML 하나로 유지하는 가이드.
tags:
  - docs
status: new
---

TOML·JSON·INI 읽기/쓰기 — 허브는 YAML.

## 허브-스포크 모델

pyrs-yaml은 여러 설정 형식을 지원하지만, **편집 가능한 표현은 YAML뿐**입니다.
라운드트립 편집(주석·앵커·제자리 splice)은 YAML 전용이고, 교환 형식은
입출력 변환을 담당합니다:

```text
load_toml / load_ini / JSON 텍스트 ──▶ 값과 YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML 텍스트 ◀──────────────────┘
```

## JSON: 이미 네이티브

YAML 1.2는 JSON의 상위 집합이므로 모든 JSON 문서는 YAML을 받는 곳이라면
어디서나 그대로 로드됩니다. 별도 API가 필요 없습니다:

```python title="JSON 입력"
import pyrs_yaml

data = pyrs_yaml.safe_load('{"a": [1, 2], "b": true}')
# {'a': [1, 2], 'b': True}
```

전용 `json` 스키마([사용자 정의 스키마](custom-schema.md) 참조)는 스칼라
해석을 JSON 호환 규칙으로 제한하고, `from_json`과 CLI `to-json`이 명시적
변환을 제공합니다.

## TOML

JSON 변환 가계와 같은 형태의 함수 셋:

```python title="TOML 입출력"
import pyrs_yaml

# TOML 텍스트 -> Python 값(중간 문서 없는 고속 경로)
config = pyrs_yaml.load_toml('s = "true"\nn = 42\n')
# {'s': 'true', 'n': 42}   <- TOML 문자열은 재해석되지 않음: "true"는 문자열 유지

# TOML 텍스트 -> YAML 텍스트(이후 일반 편집 가능)
yaml_text = pyrs_yaml.from_toml('title = "app"\nport = 8080\n')
# 'title: "app"\nport: 8080\n'

# YAML 텍스트 -> TOML 텍스트
toml_text = pyrs_yaml.to_toml("name: app\ncount: 3\nnested:\n  a: 1\n  b: two\n")
# 'name = "app"\ncount = 3\nnested = { a = 1, b = "two" }\n'
```

!!! note "날짜/시각"

    TOML datetime은 내장 `!timestamp` 플러그인을 거쳐 진짜
    `datetime.datetime` 객체가 됩니다:

    ```python
    pyrs_yaml.load_toml("when = 2026-01-02T03:04:05Z\n")
    # {'when': datetime.datetime(2026, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)}
    ```

### `to_toml`이 거부하는 것

TOML로 표현할 수 없는 YAML 구조는 데이터를 몰래 잃는 대신 안정적인
메시지의 `ValueError`를 던집니다:

- null 값(TOML에는 null이 없음)
- 테이블이 아닌 문서(루트가 스칼라나 배열)
- 앵커 / 별칭, 그리고 `!timestamp` 이외의 태그
- 비스칼라 맵 키

## INI

의도적으로 읽기 전용 — INI에는 공식 문법이 없으므로 pyrs-yaml은 표준
라이브러리 파서로 받아들이고 쓰기는 각 도구의 판단에 맡깁니다:

```python title="INI 입력"
import pyrs_yaml

config = pyrs_yaml.load_ini("[server]\nHost = 127.0.0.1\nPort = 8080\n")
# {'server': {'Host': '127.0.0.1', 'Port': '8080'}}
```

동작 메모:

- 키 대소문자는 보존됩니다(`host`가 아니라 `Host`);
- 섹션, `;`/`#` 주석, 다중 줄 값은 엄격 모드의
  `configparser.RawConfigParser` 의미론을 따릅니다 — 중복 키나 섹션
  헤더 누락은 `ValueError`를 발생시킵니다;
- 모든 값은 문자열입니다. 타입이 필요하면 직접 변환하거나 YAML을
  거치세요:

```python title="INI에서 YAML로"
import pyrs_yaml

yaml_text = pyrs_yaml.safe_dump(pyrs_yaml.load_ini("[s]\nport = 8080\n"))
# s:\n  port: '8080'\n
```

## 관련 문서

- [사용자 정의 스키마](custom-schema.md) — `json` 스칼라 해석 스키마
- [명령줄 인터페이스](cli.md) — `to-json` / `from-json` 하위 명령
- [라운드트립 보존](round-trip.md) — YAML이 편집 허브인 이유
