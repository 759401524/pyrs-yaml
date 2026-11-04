---
title: Changelog
description: pyrs-yaml의 모든 중요 변경 사항 — Keep a Changelog 형식, Semantic Versioning 준수
tags:
  - docs
status: new
---

## 변경 이력

이 파일에는 본 프로젝트의 모든 중요 변경 사항이 기록됩니다.

이 형식은 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 기반으로 하며,
이 프로젝트는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 따릅니다.

### [Unreleased]

#### 수정

- **별칭을 통한 편집에 대한 문장 셋이 거짓이었고, 메시지가 출력하는 이유 어휘가 이제 목록으로
  실립니다** — 페이지들은 별칭을 통한 쓰기가 `cannot-edit-alias`를 발생시키고 살아 있는 정의를 뜬
  상태로 만드는 쓰기에 `set()`이 `anchor-is-referenced`로 답한다고 적었습니다. 배포된 빌드에서
  측정하면: `base: &x 1\nuse: *x`에서의 `set("$.use", 2)`는 성공해서 `base: &x 1\nuse: 2`를 남기고,
  `delete("$.use")`는 `base: &x 1`를 남기며, `rename("$.use", "used")`는 `used: *x`를 남기고, 같은
  짝에서의 `set("$.base", 2)`는 `base: &x 2\nuse: *x`를 남깁니다 — 그 토큰에 닿는 `set` 경로는
  없습니다. 그 토큰은 삭제, 이름 변경, 이동, 메타데이터 경로에 살아 있습니다. 별칭 *부모*는 여전히
  거부되고(`set("$.prod.timeout", 99)` → `edit failed: cannot-edit-alias`), 별칭 노드에 대한 열두
  메타데이터 동사도 마찬가집니다. `api/exceptions.md`에는 편집 이유 토큰 표가 새로 생겨, 닿을 수
  있는 열다섯 토큰을 각각 답하는 호출과 완전한 측정 메시지와 함께 이름 지었습니다 —
  `edit failed: cannot-descend-into-scalar:b`, `duplicate-key-target`,
  `unknown on_duplicate "both"; expected one, all or error` — 그리고 생성 지점이 없는 선언
  변형(`wildcard-or-deep-scan`, `invalid-index`, `invalid-path`)이 무엇인지도 적었습니다.
  `validate()`의 위임은 사용자가 만나는 자리에 이제 적혔습니다: 이 프로젝트가 배포하는 무엇도
  `jsonschema`를 설치하지 않습니다(기본 의존성은 `typing-extensions`, extras는
  `cli`/`pydantic`/`settings`). 그래서 기본 설치에서는 맨 `ModuleNotFoundError`로 답하고, 텍스트로
  넘긴 스키마는 `json.loads`가 먼저 읽으므로 YAML로 철자한 스키마는 `json.decoder.JSONDecodeError`를
  발생시킵니다. 둘 다 .ci/known-residuals.json에
  `api:validate-evaluator-is-not-installed-by-anything-we-ship`과
  `api:validate-schema-text-is-json-only`로 등록됐고, 여기서 어느 쪽도 고치지 않습니다. 실제 시간
  표도 이 릴리스의 빌드로 2026-11-04에 다시 측정해, 모든 셀과 본문의 범위, 출처 날짜가 같은 스냅샷을
  가리킵니다. 일본어 두 페이지에서는 영어판에 없고 현재 메시지 형식과도 모순되는 오류 메시지 예시
  블록을 지우고, `dict`만 적혀 있던 `validate()` 서명을 `str | dict`로 고쳤습니다. 영어와 그
  zh/ja/ko 쌍입니다.

### [v0.19.0] — 2026-11-04

<details markdown="1">
<summary>호출 지점에서 이름 지은 방침 · 중복 키 · 주석 위치 · 강제 이미지로 끄는 하한</summary>

#### 추가

- **`duplicate_keys=`는 읽는 쪽의 중복 키 방침을 이름으로 받고, `as_is`는 네 대답을 남긴다** —
  플래그 하나가 넷을 대답하는 읽는 쪽에 두 동작을 물었다: `a: 1` 옆의 `a: 2`는
  `YamlDuplicateKeyError`를 올리고, `: 1` 옆의 `: 2`는 뒤 짝으로 접고, `0set: 1` 옆의
  `!base 0set: 2`는 두 짝을 다 남기고 `to_dict()`만 거부하며, `'a': 1` 옆의 `a: 2`는 올린다.
  `as_is`(기본)는 바로 *그* 동일성별 대답들이다; `error`는 기본이 넘어가는 빈 키와 병합 키까지
  거부하고, `last_wins`는 `allow_duplicate_keys=True`가 늘 뜻하던 것이며, `first_wins`는 먼저 나온
  짝을 남기고 접혀 나간 짝의 주석과 앵커 정의를 남은 쪽으로 옮긴다. 두 철자를 함께 넘기면
  거부하고(`pass one`), 알아듣지 못하는 값은 의도한 네 가지를 이름으로 말하며, 두 YAML 읽기가 같은
  답을 준다. 모든 읽기 진입점과 열두 CLI 명령이 이를 `--duplicate-keys`로 받는다. 이 트리에서의
  갱신은 아홉 하한을 올린다 - `parse_medium` 322,863,958 → 325,633,529 (+0.86%),
  `to_python_medium`와 `parse_block` +0.73%, `parse_anchors` +0.68%, `parse_small` +0.67%,
  `parse_inline_merge` +0.65%, `to_python_small` +0.64%, `to_python_anchors` +0.59%, `parse_notes`
  +0.53% - 나머지 마흔여섯은 ±0.07% 안에 두고; 퍼즈 대상은 기본으로 해석하므로 이름 붙은 세 대체는
  아직 퍼즈 층에 들어가지 않았다.
- **호출 지점이 모호함을 이름 지운다: `note_placement=`과 `on_duplicate=`** — 짝의 줄 끝 주석 칸이
  `&a` / `!tag` 헤더 아래 이미 쓰였을 때, 쓰기는 그 주석을 헤더의 틈에 두었는데 읽는 쪽은 그 위로
  정리한다 - libFuzzer `edit_yaml` crash-547a9705. `to_yaml_with_options(note_placement=...)`:
  `note_first`(기본)는 읽기가 두는 자리에 쓰므로 첫 발출이 그대로 고정점이고, `header_first`는
  예전의 자리를 유지하되 그 자리에서는 수렴하지 않는다고 문서에 적으며, 알아듣지 못하는 값은 그
  자체가 이름 붙은 거부다. 두 키가 같은 이름 하나를 철자할 수도 있고, 그때 구분은 읽는 쪽이 보존하는
  태그나 앵커로만 가능하며, 하나를 지우면 다시 읽는 순간 나머지 하나가 그 이름에 답한다 -
  crash-46d35d71, `a deleted key is still in the document`로 보고되었다.
  `delete(path, on_duplicate=...)`: `one`(기본값, 모든 기존 호출자가 봐 온 답), `all`(그 키가 답할
  수 있는 짝을 모두 지운다), `error`(둘 이상 남으면 `duplicate-key-target`으로 거부하되 트리는
  건드리지 않는다). 엔진 이음새 `delete_path_with_policy`는 제거마다 변경 단위를 하나 돌려주고 앵커
  방어막을 다시 돌리므로, 별칭이 아직 가리키는 정의는 뜬 채로 발출되는 대신 거부된다; `delete_path`,
  `del doc[key]`, `Node.delete()`는 `one`을 유지하고, 같은 실행의 `b7e16929`(별칭보다 먼저 발출된
  앵커)는 그것이 본래 안고 있는 앵커 순서 문제로 열려 남는다.
- **퍼즈 코퍼스에 정리가 붙었다: 오래된 산출물 보관, 샘플러 발견 등록, 대상별 시드 계수** — 시드도
  기록도 없는 산출물 세 개 - `yaml_roundtrip` crash-3528db38 (2026-10-07), crash-d461a3f2
  (2026-10-16), `parse_json` crash-331b36ec (2026-10-17) - 가 깨끗하게 재생되어 `fuzz/seeds/`에
  `former-crash-*.seed`로 들어간다; 어떤 수정도 주장하지 않고, 증명된 것은 보관이다. 2026-10-31 예약
  실행이 남긴 `toml_roundtrip` 및 `edit_yaml` 산출물 여섯 개 - crash-969291e0, crash-b750553e,
  crash-fe4a2b80, crash-445ddfff, crash-450cac9b, crash-dce5f5ed - 는 이번 릴리스의 TOML 주석 수정이
  해결하는 한 형태를 철자한다. 그 형태는 컨테이너 자신인 배열 요소 위의 `#` 줄이고, 여섯 개 모두
  깨끗하게 재생되어 보관된다. 주간 샘플러에 마침내 소비자가 생겼다: 2026-10-03 부터 2026-10-31
  까지의 다섯 예약 실행은 아무도 읽지 않은 크래시 입력을 안고 실패했다. 이제
  `scripts/fuzz_file_issues.py`가 대상마다 issue 하나를 열어 run 링크, 시그니처 개수, 입력 이름을
  싣고, 제목 완전 일치로 중복을 판정해 덧붙이며 `issues: write`를 얻고, `pull_request`에서는
  건너뛰고, 남은 바이트가 하나도 없으면 발견 없음 보고 대신 exit 2를 낸다. `fuzz_seed_inventory()`는
  `fuzz.yml`이 복사하는 그대로 각 `fuzz/seeds/<target>/`을 세어 `fuzz_seed_counts`를 공개하고,
  `fuzz-corpus-unseeded` - PR 층은 `-runs=0`으로 확정된 시드만 재생하고 탐색은 하지 않는다 - 와
  `fuzz-corpus-orphan`을 거부한다.
- **주석 오라클이 형식을 넘어 넓어지고, 이 실행의 주석 모양이 못박혔다** — TOML 주석 스택을 지워도
  고정점 재생은 초록으로 남는다 - 이제 보존 주장은 코퍼스를 상대로 관문이 걸린다: TOML 주석 생존
  테스트가 주석을 지닌 `toml_roundtrip`/`parse_toml` 시드에 과금을 걸고(2026-11-02 실측),
  `note_survival.rs`의 YAML 순회는 `edit_yaml`의 문서 절반을 받아들인다. `corpus_notes.rs`는 그
  규칙을 확정된 두 코퍼스에 걸쳐 주석을 받을 수 있는 두 JSON 방언까지 데려온다: 트리가 지닌 모든
  주석은 발출된 텍스트에 나타나야 하고, `to_jsonc_text`나 `to_json5_text`에서 루트 주석 호출을
  지우면 새 `shape-root-line-note.seed`에서 빨개진다. `prop_yaml_writer_carries_every_note`는 생성된
  트리에도 같은 것을 묻는다 - 수집 단계에서 사라진 주석은 AST에 닿지 않았고, `Alias`는 생성기 밖에
  있다. 이 실행의 마지막 발견들도 닫혔다: crash-1bbd4d7f는 깨뜨릴 이름만 빠져 있었다 - 이미 첫
  라운드에 올려 쓰고, 그 다섯 모양이 못박혀 있다; 태그만 있는 줄(`bg: !`)은 지난 시작 위치로 이전 줄
  안쪽에 키 주석을 썼고, 그 텍스트는 자기 읽기가 거부한다(`simple key expected ':'`). 그래서 이제 두
  키 분기 모두 시작 위치를 다시 잡고, crash-2c76568e는 바이트 단위로 못박혀 보관된다.
  `rustc 1.97.1`로 값을 매기면 YAML 라이터 시나리오 어느 것도 0.5 % 선을 넘지
  않는다(`serialize_anchors` −0.24 %, `serialize_medium` −0.22 %, `serialize_note_flow` −0.06 %),
  그리고 이 호스트가 자기 하한보다 1.65–2.99 % 높게 읽는 두 TOML 읽어들기 시나리오는 부모 트리에서도
  같은 값을 읽는다.
- **Python 생성 계층이 잎으로는 결코 이름을 붙일 수 없던 계열을 그린다** — `arbitrary_json`은 dump
  속성의 잎을 `None`, `bool`, 범위 안의 `int`, 유한한 `float`, `str`으로 묶었다: 25,000번의 추출에
  다른 형은 하나도 없으므로, 그 계층의 주장은「JSON이라는 부분 집합에서 패닉이 없다」였다.
  `hard_leaf`는 범위 없는 정수, 비유한 실수, 튜플, `bytes`, `date`, `time`, `datetime`, `Decimal`
  그리고 YAML 표기가 없는 네 형을 더한다. `test_dump_never_raises`는 텍스트 또는
  `pyrs_yaml.YamlTypeError`만 받고 그 나머지는 거부하며, 계열 표는 결과를 못박는다: 범위 밖 정수는
  `str`으로 돌아오고, 튜플은 `list`로, `bytes`는 `bytes`로, 시각형과 `Decimal`은 자기 형으로, `set`,
  `frozenset`, `complex`, 미등록 클래스는 그 형화된 거절로. 텍스트에 닿은 행은 각각 다시 써도 같은
  바이트 열이 된다, `safe_dump(-(2**64))`만 빼면: 맨 숫자는 문자열로 읽히고 다시 쓰면 따옴표가
  붙는데, 같은 판단을 `2**63`에 적용하면 자기 텍스트를 되풀이한다 - 한 따옴표 방어막 안의 부호
  비대칭이고, `NON_SETTLING`에 기록하고 결함으로 열어 둔다. 도달 방어막은 400번의 추출에서 도착하지
  않은 계열을 이름으로 지목한다(드문 넓은 정수와 비유한 갈래에는 전용 추출이 필요했다). 그래서 이
  계층은 소리 없이 다시 좁아질 수 없다; `_register_builtins()` 방어막은 대답을 import 순서로부터
  안전하게 두고, 완전 일치 전략은 JSON 모양 그대로 남으며, 엔진 코드는 한 줄도 움직이지 않는다.
- **JSON Schema 표면에 속성 계층이 붙고, 그 계약 둘이 등록됐다** —
  `tests/test_property_json_schema.py`는 메타 유효한 draft2020-12 스키마를 YAML 타입 키를 지닌
  문서와 조합해 판정과 메시지를 `jsonschema`와 맞비교하며, `check_schema` 증거는 반복 호출이 실제로
  검증기 캐시를 지남을 보인다. 키 철자를 뽑은 일이 접힘을 드러냈다: `to_dict()`는 `1: one`과
  `true: two`를 한 항목으로 접는데 `to_yaml()`과 `to_json()`은 둘 다 발출하므로, `validate()`는
  접히는 문을 읽어서 JSON 투사가 받아들이는 두 키 문서를 `minProperties 2`가 거절한다 - PyYAML의
  로더도 같은 지점에서 접는다(2026-10-31 실측). 따라서 그것은 대항 라이브러리의 공통 형태다. 두 YAML
  읽어들기는 인접한 짝에서 반대 답을 낸다(`parse()`과 `safe_load()`은 `1:` 옆의 `'1':`와 옆의
  `true:`에서 서로 갈라진다), 판결이 필요한 질문으로 `.ci/known-residuals.json`에 등록했다. 위임의
  대가: 이 프로젝트가 배포하는 무엇도 평가기를 설치하지 않으므로, 기본 설치에서는
  `Document.validate()`가 맨 `ModuleNotFoundError`로 답하고, `str` 문은 평가기에게 묻기 전에
  `json.loads`를 돌린다 - YAML 표기 스키마는 `json.decoder.JSONDecodeError`를 올리며, 그 클래스는
  문서에 적힌 열 개의 `Yaml*`에도 CLI의 `except YamlValidateError` 절에도 속하지 않는다 — 실측 결과
  `pyrs-yaml validate doc.yaml --schema-file schema.yaml`은 바로 그 traceback을 출력한다. 둘 다
  여기서는 고치지 않고 남고, PR #517은 두 질문을 없앨 평가기를 설계한다.
- **형식 간 주석 진입, 공개 호출 표면, JSON 트리 동등성** — 134개 테스트 모듈을 AST로 훑은 일이
  `json -> jsonc`와 한 방향으로만 열리던 나머지 네 형식 짝이 한 번도 시험되지 않았음을 발견했다.
  `tests/test_cross_format_note_entry.py`가 바로 그 표다 - 주석 문법을 가진 네 진입과 네 내보내기,
  네 주석 칸 - 각 칸은 주석 토큰이 텍스트에 도달하거나 내보내기가 대상에 대해 참인 이유로 거부함을
  단언하고, 그 토큰은 내보내기 자신의 문법으로 사라져야 하므로 데이터가 된 주석으로는 행을 채울 수
  없다. 다리는 리터럴이 해석된 채로 건넌다(`0xFF`는 TOML에서 `255`), 유일하게 움직이는 칸은
  `docs/dev/boundaries.md`가 이미 적어 둔 위치 이동으로 못박히고, 엄격한 `to_json`은 표 밖에 있다 -
  JSON에는 쓸 주석이 없으므로. `tests/test_api_call_surface.py`는 배포된 스텁의 모든 공개 호출
  가능이 호출 지점을 갖도록 요구하고, 그런 관문이 몰래 초록이 되는 네 경로를 거부한다; 그 점검이
  `get_plugin`과 `YamlDocument.flush_source`가 한 번도 불리지 않음을 발견했고 이제 실측했다:
  `get_plugin("!date")`는 `("!date", "<class 'datetime.date'>")`이고 알려지지 않은 태그와 빈 태그는
  `None`이며, 편집하지 않은 문서에서 `flush_source()`는 읽을 때의 텍스트를 그대로 둔다.
  `prop_json_roundtrip_preserves_the_tree`는 세 JSON 방언에 걸쳐 값·모양·순서를 비교한다: `null`
  자리에 `0`을 쓰면 텍스트 수준 속성 일곱은 초록으로 남고 이것만 빨개진다.
- **하한과 하한 인용이 주석 경로와 Python 객체 쓰기로 넓어졌다** — TOML의 주석 경로에 값이
  매겨졌다(주석 일곱 개: 각각 1,945개 명령); 읽히지 않던 인라인 표 주석 칸에 읽는 손이
  생겼다. #673의 `safe_dump_*` 하한을 실어 나른 파견은 `accept_drift`를 비운 채 53개 중 47개를
  움직였다: 위쪽 가장 큰 폭은 +0.385 %(`from_json_long_strings`)이고, 0.5 % 선을 아래로 넘은 것은
  `from_json5_medium` -1.132 %, `from_jsonc_medium` -1.130 %, `parse_inline_merge` -0.551 % 셋뿐이며
  게이트는 2 %를 넘어야 경고한다 - 하네스 자기 코드 크기다. 바인딩의 쓰기 쪽은
  `safe_dump_small`/`safe_dump_medium`이 값을 매긴다(확보는 루프 밖): `needs_quotes`가
  `safe_dump_medium`의 5.45 %, `DirectWriter::write_key`가 3.70 %이고 `type_registry`는 아예
  없다 - #672의 빈 handler 빠른 길이다. 인용된 하한도 확인된다: `QUALITY_MATRIX.md`의 인용된 하한
  열하나 중 아홉이 모든 관문이 녹색인 가운데 낡아 있었고, 이제 `scripts/check_ir_floor_quotes.py`가
  지난 숫자, 없는 시나리오, 값을 지운 뒤 홀로 남은 표시, 아무것도 선언하지 않는 페이지를
  거부한다(exit 2); 키가 지정된 표시는 자기 이미지를 이름
  지는다(`load_toml_medium @ 20261023.334.3`) - 그 파일은 144,902,515와 142,467,241을 담고 있고
  1.71 % 차이이며, 평평한 열은 이미지마다 최대 1.68 % 움직이고, 다시 맞춘 읽기 아홉 개는 0.001 %
  안에 있다. 갱신은 '어디'에 답한다: `--profile`은 #401 이후 자기 명령 수로 순위를 매기지만, 손으로
  낸 실행은 확정된 하한과 짝을 이룰 수 없다 - `documents_short_strings`의 -21.23 %가 어떻게 원인
  규명 없이 남았는지가 그것이다 - 그래서 `profile`/`profile_top` 입력이 그것을 강제 이미지 위에서
  돌린다.
- **저장소 자신의 주장도 이제 관문이 있거나 못박혀 있다** — CI의 넓은 clippy는 `required-features`
  때문에 Ir 하네스 두 개를 조용히 넘겼다; 이제 관문이 있는 단계와 `prek` 훅이 그 둘을 덮는다. 아홉
  개의 `python_version` 상한 중 다섯 개가 wheel이 이미 배포되는 인터프리터를 잘라냈다 - PR #665는
  3.13에서 4,433개 어서션을, 3.14에서 4,197개를 돌렸다 -; 올린 쪽: `pyjson5`는 `< '3.15'`,
  `numpy`/`ujson`/`python-rapidjson`/`yaml-rs`는 `< '3.16'`, `orjson`/`rtoml`은 3.14t wheel이 없어
  3.14 아래에 남는다; 훅이 붙은 `scripts/check_counterparty_caps.py`는 실측된 가용성에 비추어
  양방향의 낡음을 거부한다. 공유 객체가 쓰는 모양은 못박혔다: 객체 경로는 두 사본을 발출한다(다시
  읽으면 같고, 하나의 객체는 아니다) - 왕복은 `&x`/`*x`를 보존하는 자리에서; 순환은
  `YamlMaxDepthError`를 올린다. pytest 계층의 폭은 `scripts/ci_matrix_plan.py`에서 유도된다(12개 칸,
  ubuntu가 여덟 인터프리터를 모두 담당), 문장 자체는 프로브가 다시 쓴다. `rumdl fmt`는 단락이나 code
  span을 해시 연속으로 여는 자리를 제목으로 승격하고, 다시 쓴 뒤에는 잡을 수 없으므로,
  `scripts/check_doc_headings.py --predamage`는 포맷터보다 앞의 자리에서 그 형태를 거부한다. 대장의
  `(xy)` 마디 순서도 훅이다(종료 1은 순서가 틀린 마디, 종료 2는 문자 마디가 없는 파일). 그리고 네
  테스트(JSON/TOML 읽기와 쓰기)는 스칼라 문자열 순회 - aarch64 wheel이 지나는 전 경로 - 와 SSE2
  루프를, 모든 바이트 값을 열세 오프셋에 놓은 80바이트 본문에서 맞비교한다.
- **행 구분 JSON이 Python 계층에 닿았다: `load_json_lines`, 샤드당 수신기 하나, 선택적 `mmap`** —
  엔진은 #599 이후 NDJSON을 기술할 수 있었다(`pyrs_json::parse_documents`)만, Python 계층에는 그것을
  읽는 창구가 없었다. `pyrs_yaml.load_json_lines(text)`는 `__next__`마다 한 줄을 해결하는
  `JsonLines` 반복자를 돌려준다 - 표준형 JSON은 AST를 거치지 않는 경로로, 그 외에는
  `document_from_line`으로 - 그래서 동시에 살아 있는 문서는 하나다. 빈 줄, 주석만 있는 줄, 다음 줄을
  필요로 하는 값은 그 줄 번호를 밝혀 올리고, 다시 물으면 건너뛰지 않고 같은 기록을 보고한다. 실측값
  (10,000 기록): 전체 스트림 3.861 ms, 첫 기록 0.343 ms, 줄별 `load_json` 8.440 ms. Rust의
  `parse_documents*`는 모든 이벤트와 오류의 위치를 입력 전체에 대해 보고하고, 방언 하나(JSONC 또는
  JSON5)가 모든 줄에 적용된다; 선택적 `parallel` 피처는 샤드마다 자기 sink와 스레드를 주고 전역
  `offset`으로 병합한다: 200,000줄은 sink 하나로 112.3 ms, 24개 샤드로는 18.9 ms(5.96배) - 공유
  sink는 병렬화되지 않는다(142.1 ms), 136바이트 이벤트 때문에 버퍼링 비용은 입력의 대략 19배 -.
  `parse_file`은 선택적 `mmap` 경로를 얻어, 비어 있지 않은 일반 파일을 `read_to_string` 복사본 대신
  메모리 매핑에서 직접 파싱해 파일 전체 할당 하나를 없앤다; 기본은 꺼짐(`default = ["numpy"]` 불변),
  빈 파일·디렉터리·비 UTF-8 입력은 기존 오류를 그대로 낸다.
- **스트림은 소비자가 말하면 멈추고 자기 소스 바이트를 이름 지며, 덤퍼가 tuple을 받는다** —
  `parse_stream(..., on_event=...)`는 첫 콜백 이전에 모든 이벤트를 모았으므로, 첫 이벤트에서
  `False`를 돌려도 스캔 전체 비용을 치렀다. `pyrs_event::EventSink`는 이제 `ControlFlow`를 반환하고,
  JSON/TOML 스트림은 만들어 내는 즉시 전달한다 - 콜백은 `Python::attach` 아래, 프로듀서는
  `py.detach` 아래. `False`가 멈추는 것은 형식 자체의 사실이다: JSON/JSON5는 다음 이벤트의 바이트에
  도달하기 전에 멈추므로 중단점 뒤의 오류에는 결코 도달하지 못하고, TOML은 이미 세운 IR에서 발출을
  멈추므로 거부된 문서는 여전히 먼저 올라오며, YAML의 어댑터는 중단할 수 없는 파싱 이후에 전달만
  멈춘다 - 증분 표면은 계속 `load_stream`이다. 콜백 예외는 뒤따르는 파싱 오류보다 우선하고,
  `False`는 `None`을 반환한다. 모든 `StreamEvent`는 `offset`/`end`를 싣는다 - 부분 트리가 아니라
  토큰의 범위이고, 프로듀서가 폭이 아니라 위치만 기록하는 자리에서는 `end == offset` - 이벤트
  딕셔너리에도 두 키가 들어간다. 덤퍼는 로더가 돌려주는 tuple을 받는다: `? [a, b]`는 `("a", "b")`로
  투영되는데 두 쓰기는 모두 tuple을 `Unsupported Python type`으로 거부했으므로,
  `safe_dump(safe_load(text))`는 받아들인 입력에서 예외를 냈다. 이제 tuple은 두 경로에서 시퀀스이고,
  노드 직렬화기가 소유한 명시 키 형태의 시퀀스 키다; 투영되는 것은 키의 바깥 시퀀스뿐이므로
  `? [[1], [2]]`는 여전히 Python 고유의 `unhashable type: 'list'`를 던진다.

#### 수정

- **어떤 통과가 유일한 정의를 접어 버린 별칭은 이제 그 사용이 닿을 수 있는 자리에서 정의를 얻는다**
  — 중복 키 접힘이 `x`를 정의한 짝을 떨어뜨리는 동안 살아남은 `&x`는 별칭 뒤에 떨어지는 키에 얹혀
  나오므로, `*x`가 모든 `&x` 위에 나오면 이 라이브러리 자신의 읽기가 `found unknown anchor`로 그
  발출을 거부했다 (`edit_yaml` crash-b7e16929). 그 수리는 어딘가에 정의가 살아남았는지만 물었다;
  이제 그 이름이 이 사용보다 앞에 정의됐는지 묻고, 닿을 수 없는 사용들 안으로 노드를 쓴다
  (`inline_dropped_uses`). 병합 통과는 앵커를 실은 `<<` 항목을 삭제하는데 자기 수리가 없어서,
  `<<: &b` / `  x: *b`를 3번째 글자에서 거부되는 맨 `x: *b`로 바꿔 놓았다; `resolve_merge_keys`는
  이제 그 항목의 키와 값에서 그 앵커를 읽고, 통과가 돈 뒤에 살아남은 것들을 인라인한다.
  `delete_path`는 값 노드에만 그 앵커를 물었으므로 키에 실린 `&x`는 끝내 보이지 않아 `k8: *x`가
  받아들여졌다; 이제 키 노드도 읽는데, 바인딩은 여전히 같은 편집을 `anchor-is-referenced`로 거부한다
  (crash-6a9b6538, crash-718abefa). 앵커 그래프는 문서 135개를 돈다 - 2026-10-20의 읽기는 102.
- **별칭에서 다시 쓴 키는 떨어지는 자리에서 접히고, 따옴표로 끝나는 앵커 이름은 그 이름을 지킨다** —
  `repair_orphaned_anchors`는 떨어진 짝의 노드를 남은 모든 `*name`에 인라인하고 키 자리도
  포함하는데, 재구성이 그걸 맨 `pairs.insert`로 다시 끼워 넣었으므로, 자기 null 키를 둔 맵이 키 두
  개를 갖게 되고 둘 다 `~:`로 적혀, 다시 읽는 순간 하나가 접혀 사라졌다 (`yaml_roundtrip`
  crash-49f44d18; crash-fce042d0은 다시 쓴 짝이 그 맨 짝 옆에 떨어질 때의 같은 접힘이다). 재구성은
  이제 끼워 넣는 모든 키를 이미 놓인 키들과 접는다 - 마지막 짝이 얻고, 떨어진 짝의 주석은 살아남은
  쪽으로 돌아간다 - 그리고 그 짝의 앵커 달린 노드들을 다음 인라인 라운드에 대기시킨다;
  crash-663b71ab과 crash-cf1a63bf는 깨끗하게 재생된다. `is_valid_anchor_char`가 granit의 규칙을
  그대로 옮기므로 앵커 이름은 `"`로 끝날 수 있는데, `opens_quoted_node`는 앵커나 태그 토큰에
  들러붙은 따옴표가 인용 노드를 열게 했으므로, 발출을 다시 읽을 때 자기 줄 끝 주석의 `#`을 인용 안의
  내용으로, 그 안의 `&`을 앵커로 봐서 두 라운드 사이에 노드 이름을 `!`로 바꿔 버렸다
  (crash-abc541ff, crash-c06e8a8f); 이제 속성이 그 속성이 도입하는 인용 노드와 갈리려면 빈칸이 하나
  있어야 한다.
- **첫 짝의 주석은 마커 위로 올라가고, 첫 항목의 주석은 자기 줄표 아래에 남는다** — `?` 마커 body
  안에서 라이터는 컨테이너가 앵커나 태그 헤더를 쓸 때만 첫 짝의 키 주석을 마커 위로 올렸는데, 짝
  라이터는 그 줄의 줄 끝 칸을 값의 인라인 주석에게 넘기므로, 읽기가 키 맵에서 보고하는 주석이 두
  번째 라운드에 올려졌다 (`yaml_roundtrip` crash-430f4f0e). 그 올림은 이제 값이 그 칸을 차지하면
  헤더와 무관하게 그 주석을 가져간다 - body 안에서는 그 줄이 다시 읽기를 버티지 못하기 때문이다.
  `lift_first_item_notes`는 블록 시퀀스 첫 항목의 선행 스택을 바깥 헤더 위로 옮겼는데, 제 줄에 따로
  철자된 항목에는 그 자리가 한 단계 너무 높았다 - 읽기가 그 칸을 시퀀스 자신의 주석으로 보고하기
  때문이다 (`edit_yaml` crash-439f5d2c); 그 올림은 이제 `write_sequence_item`의 비압축 갈래에서는
  스택을 body에 두고, 압축 철자에만 올린다.
- **마커 올림이 주석을 헤더 하나만큼 너무 올리고, 이름 바꾸기가 키가 실은 fidelity를 떨어뜨렸다**
  — #726의 `edit_yaml` 입력 셋은 결함 두 개 뒤에 여전히 실패했다. `rename_path`는 키를 새 텍스트로
  다시 지어 단순 키가 실던 `Fidelity`를 버렸으므로, `!base: &x 0se: *x`에서 `&x 0se`의 이름을 바꾸면
  정의되지 않은 앵커를 가리키는 별칭인 `k7: *x`가 발출됐다; 이제 키는 자기 앵커와 태그를 지킨다.
  `spine_has_notes`는 선행 스택만 물었는데 `hoist_marker_note`는 헤더 아래에서 첫 키의 인라인
  주석까지 가져가므로, 주석은 라운드마다 헤더 하나씩 위에 자리잡았다; 그 방어막은 이제 그 순회를
  그대로 옮긴다. 시퀀스 쪽 짝 함수는 첫 항목의 스택만 읽었으므로 한 단계 아래 주석은 헤더 아래에
  남아 두 번째 통과에 자리잡았다; 이제 그도 같은 방식으로 스파인을 돈다 (`crash-094d4d6a`,
  `crash-8492f5ee` - 2026-10-17과 2026-10-24 실행에서 나온 것으로 #691과 #689, 그 둘의 바깥이다 -
  그리고 `crash-701f76bf`). 그 순회는 독립 스택만 가져갔으므로, 헤더가 달린 컨테이너 키는 자기
  주석을 `?` body 안에 썼고 다시 읽기가 그것을 마커 위로 올렸다 (`crash-6cfbaa57`); 두 순회는 이제
  같은 질문 하나 (`first_entry_inline_note`)를 하고 헤더에 관문을 둔다. 라이터 시나리오 여섯은 0.5 %
  선 안에서 지불한다 - `serialize_block` +0.18 %, `serialize_note_flow` +0.17 %, `serialize_medium`
  +0.12 %, `serialize_small` +0.10 %, `serialize_anchors` +0.06 %, `serialize_block_scalars`
  +0.03 % - 그 방어막이 없으면 +0.00 %인 자리에서.
- **주석은 더 이상 값을 두 줄로 쪼개지 않는다: 닫기가 밀어 낸 뒤 그 짝은 자기 키 줄의 닻을 다시
  박는다** — 맵 항목은 자기 줄의 시작을 기억하는데, 대기 중인 태그 전용 줄을 닫으면 기억한 오프셋
  앞에 그 값의 `~`를 옮겨 놓지 않고 끼워 넣으므로, 다음 키의 인라인 주석이 방금 그 닫기로 늘어난 줄
  한가운데 떨어졌다: `: !` / `<<: #b` / `<<: #b` / `3 #~`의 `to_yaml`은 `~: ! # b`를 쓰고 곧이어 맨
  `~`를 열의 맨 앞에 놓았고, 이 엔진 자신의 읽기는 그것을 거부한다 (`simple key expected ':'`).
  `note_line_above`는 이제「이 오프셋은 줄을 가리킨다」를 불변식으로 다루고 낡은 캡처를 다음 줄
  시작으로 밀어간다. 같은 낡은 캡처는 키 줄 자체도 옮겼는데, 값이 자기 주석을 지닐 때 키 주석은 짝
  줄을 나눌 수 없기 때문이다 (`yaml_roundtrip` crash-76e05a78, 104바이트에서 25로 최소화), 그리고 그
  짝은 이제 자기 선행 주석 다음에 키 줄 시작을 다시 잡는다. crash-2c76568e는 예약 실행 발견 층에서
  왔고, 그 보고서는 다섯 주간 빨간 채로 이 주장을 버려 왔다.
- **빈 컨테이너 위에 남은 주석은 항목을 자기 줄표 줄에 남기고, 줄표와 콜론은 블록으로 가기로 결정한
  자식 때문에 더 이상 열리지 않는다** — 읽기는 컨테이너 첫 짝 위에 있는 주석을 컨테이너 자신의 선행
  주석으로 정리하므로, 마지막 짝을 지우면 아무것도 없는 위의 주석이 남고, `write_mapping_pair`는
  이미 그 답으로 `{}`를 인라인하고 주석을 짝 줄 끝으로 강등한다; 그런데 `is_compact_item`은 독립
  주석을 실은 값에는 압축된 `- key: value` 줄을 거부했으므로, 첫 라운드는 줄표만 썼고 두 번째
  라운드는 그 줄표 줄에 붙였다 (`edit_yaml` crash-ad42b9cc). 압축 판단은 이제 라이터의 규칙을
  따른다. 한 단계 위에서는, 멤버가 중괄호가 담을 수 있는 것보다 많은 주석을 실으면 flow로 표시된
  컨테이너가 강등되는데, 판단 일곱 곳이 `flow_style` 깃발만 믿고 각자 자기 표시를 쓴 반면 강등된
  자식은 부모의 열에서 블록 body를 열었다; 일곱 곳 모두 이제 그 강등 자신의 질문을 묻는다. 추가된
  테스트 때문에 라이터 일가가 +0.18 %에서 +0.72 %까지 움직였고 (`serialize_small`이 가장 컸다) 어느
  읽기 시나리오도 0.19 %를 넘지 않았으므로, 두 하한은 선을 넓히는 대신 다시 잰다.
- **노드 메타데이터만 구분할 수 있는 키는 거부된다** — 읽는 쪽은 태그가 달린 키를 스타일과 앵커까지
  포함한 노드 통째로 비교했는데, 그 두 가지를 고르는 쪽은 라이터다: 태그가 달린 키의 평문과 인용
  철자는 둘 다 `!a "…"`로, 앵커가 달린 `Null`은 `!a null`로 발출되므로, 자기 발출이 되돌아와
  `duplicate key`가 됐다 (crash-4bf6ee37). 이제 둘은 태그와 값으로 추적한다 - YAML이 스칼라 동등을
  정의하는 바로 그것이고, 병합도 같은 것을 묻는다; 키마다 표를 두는 읽기는 0.50% 선을 넘겼다. 키
  자리는 더 나빴다: crash-b88f1b4f의 맵 키 둘은 태그가 달린 빈 스칼라 하나 위에 얹혀 있고 라이터는
  둘 다 `!? ""`로 인용한다; 이제 읽기는 컨테이너 키가 무엇으로 철자되는지 라이터에게 묻고, 둘이 같게
  읽히면 그 원문을 거부한다; 같은 계열의 crash-469125e3에는 배워야 할 철자가 하나 더 있었다. 가장
  바깥 `?` 위로 올려진 독립 주석도 키의 차이는 아니다 (#645보다 앞선 것): `keys_collide_in_text`는
  독립 칸을 모두 버리고, 인라인 주석은 구분한다. 투영은 텍스트로 키를 삼으므로 `c: 0` 옆의
  `!A c: "!"`가 `c = 0`과 `c = "!"`를 함께 발출했는데, 이 프로젝트의 TOML 읽기가 그것을 거부하고
  `json.loads`는 항목 하나로 접었다; 이제 두 쓰기 경로 모두 `YamlSerializeError`를 올리고
  (`toml-keys-collide-without-metadata`, `json-object-keys-collide-without-metadata`),
  `to_dict`/`safe_load`는 그 텍스트를 이름으로 밝히는 `YamlTypeError`를 올리며, `to_yaml()`은
  그대로다.
- **반복된 JSON 키는 자기가 대신하는 값 안의 주석까지 지키고, 반복 사이에 낀 주석 달린 키는 뒤에서
  둘째가 아니라 마지막 값을 남긴다** — 이름이 겹치면 첫 위치와 마지막 값을 남기므로 앞선 값은 통째로
  떨어지는데, 그 구제는 떨어진 키와 값 노드 위의 주석만 살아남은 쪽에 넘겼다 - `notes_of`는 넘겨받은
  노드만 읽으므로, 떨어진 컨테이너 안의 주석은 그와 함께 사라졌고 이벤트 스트림은 여전히 그것을
  보고했다 (`json_events_replay` crash-e294755f: 주석 여덟 개를 보고하고 다섯 개만 남겼다).
  `subtree_notes`는 이제 노드와 그 아래 모든 것을 소스 순서로 모은다; Ir 관문은
  `from_json_medium`에서 +0.09%, `from_jsonc_medium`에서 +0.07%를 읽고, 고정물 둘 다 이런 중복을
  담지 않으므로 이 이동은 배치에서 온 것이다. 마감 접힘은 원인이 다르다: `"a"` 키 여러 중 하나에
  같은 줄 주석이 붙으면 그 키는 나머지 키와 같지 않게 되고, 맵은 두 항목을 다 쥐며, 뒤따르는 맨
  `"a"`가 첫 항목의 값을 대신하고, 접힘은 주석 달린 항목의 오래된 값을 그 위로 되돌려 놓았다 -
  `{"a": 1, /* n */ "a": 2, "a": 3}`이 `{"a": 2}`로 해석됐다. 접힘은 그 이름의 마지막 항목에서
  끝나므로, `file_duplicate`은 이제 가장 최신 값을 그 자리에 놓고, 트리 순회와 `from_events`가 그
  함수를 나눈다.
- **JSON 접힘은 요소의 두 번째 주석 덩어리를 트리가 놓는 자리에 정리하고, 루트의 줄 끝 주석은 이제
  쓰인다** — 요소의 주석은 그 위의 것과 그 뒤의 것 둘 다 그 노드의 인라인 칸 하나에 떨어진다:
  `array`는 `value()` 앞에 대기 목록을 가져간 뒤 뒤따르는 것을 flush하므로, 두 번째 `set_comment`가
  첫 번째를 덮어써서 트리는 주석 하나를 담았고 `parse_stream`은 둘 다 보고했다 (`json_events_replay`
  crash-0449c0ae, 그리고 중첩 배열에서 같은 칸에 닿은 crash-0466d7f6).
  `crates/pyrs-json/src/parser.rs`의 `apply`는 이제 두 번째 같은 줄 주석을 트리 순회의
  `attach_pending`가 하는 대로 정리하면서 앞선 주석을 그 `standalone` 플래그와 함께 선행 목록으로
  옮긴다; 접힘 경로는 자기 입력에서 같은 덮어쓰기를 만났고, `former-crash-e72e24ef.seed`로 보관됐다.
  루트의 줄 끝 주석에는 라이터가 아예 없었다: JSONC와 JSON5 라이터가 발출하는 주석은 모두 멤버
  루프에서 나오고 루트는 어떤 멤버의 자식도 아니므로, `{"a": 1} // after`는 `{"a": 1}`로 되돌려
  써져서 YAML 허브를 지나며 똑같이 사라졌다; 이제 네 진입점 각자가 최상위 값 뒤에 그것을 발출하고,
  `prop_jsonc_writer_carries_every_note`와 그 JSON5 짝이 여기에서 형식 간 층에 오라클을 준다. 센
  시나리오 어느 것도 `from_events`에 닿지 않는다.
- **JSON5: 숫자처럼 보이기만 하는 문자열은 다시 인용되고, 키 거부는 그 문법이 허락하는 철자를
  이름으로 밝힌다** — 맨 토큰 판정은 앞에 부호 하나를 벗긴 뒤 숫자 검사에 넣고 그 본문이 엄격 JSON
  숫자인지만 물었으므로, `+-33337333333373333333733333`이 받아들여질 수 있는 `-333…` 위에 `+`를 얹은
  형태로 통과했고 `to_json5_text`는 자기 읽기가 거부하는 텍스트를 발출했다
  (`expected integer digits in number`); 부호 없는 본문 위에 앞에 붙은 `+`만이 JSON5 철자이므로,
  그런 문자열은 다시 인용되고 `+7`과 `+1.5`는 맨 토큰을 지킨다 (libFuzzer `json_roundtrip`
  crash-0a929a32). 키 자리에서 그 메시지는 `expected a quoted object key`로 읽혔는데, JSON5는 맨
  identifier도 받아들이므로 그 오류가 그 방언이 한 번도 요구하지 않은 모양을 요구한 것이다; 문구는
  이제 그 방언의 자기 깃발에서 온다 - 따옴표 없는 키가 허용되는 곳에서는
  `expected an object key (a quoted string or a bare identifier)`, 허용되지 않는 곳에서는 옛 문장 -
  세 방언 모두에 `Parser` 하나가 서므로, 공유 helper가 트리 경로와 스트림 경로를 나란히 덮고 엄격
  JSON과 JSONC는 자기 문장을 지킨다.
- **TOML 헤더 주석: 아무도 읽지 않은 키 줄 칸, 그리고 대안 취급되던 두 스택** —
  `to_toml("sec: # note\n  x: 1\n")`는 `[sec]\nx = 1\n`을 돌려줬다: 그 주석은 버려졌는데, section
  통과가 맵 자신의 인라인 칸만 읽었고 허브는 YAML 문서의 키 줄 주석을 키 노드에 얹어 넘기기
  때문이다 - #146이 라이터에게 한 단계 아래를 읽도록 가르친 바로 그 칸이다. 그래서 여기서 앞선 설명
  둘을 거둔다: `pyrs_toml` 자신의 스캐너는 `[sec] # note` 주석을 실제로 잡고, granit은 그 경로에
  없다; 열어 남는 것은 배치 규칙 `api:set-comment-container-owner`가 기록한 것뿐이다. 헤더 줄의 칸은
  `key = value`를 쓰는 줄도 읽지 않았으므로, `x: 1` 위의 `b: # note`는 그 주석 없이 `b = {x = 1}`로
  발출됐다; `emit_pair`는 이제 그 칸을 물러나 읽는다. section 키와 그 키가 이끄는 맵은 선행 목록을
  둘 갖고 있고, 통과는 section의 것을 먼저 물은 뒤 키의 것으로 물러났으므로, `# T`를 실은 section
  위의 `# K`는 `# T\n[a]\nx = 1\n`만 발출했다; 이제 둘 다 쓴다. 모든 짝이 지불한다: 강제 이미지에서
  `to_toml_medium` +1.58 %, `to_toml_long_strings` +1.25 %, 나머지 37개 시나리오 중 0.32 %를 넘는
  것은 없다; 두 루프의 비용은 +0.46 %이고 같은 바이트에서 `Iterator::chain`을 타면 +0.83 %이며,
  주석을 실은 두 행은 움직이지 않았다 (−0.03 %, −0.02 %).
- **아무도 읽지 않은 TOML 멤버 주석 칸 다섯, 그리고 두 번 쓴 요소 주석** — `to_toml`은 필요한 줄이
  없으면 주석을 버렸다: 자기 주석을 실은 빈 표나 배열에서, 주석이 혼자 서는 스칼라에서, 한 줄이 한
  `or_else` 칸으로 두 주석에 답하려 한 두 곳에서. 새 `prop_toml_writer_carries_every_note`는 20,000
  케이스 안에서 그 네 모양을 모두 이름 지었고, 네 모양은 이제 키 위의 줄을 쓴다. 가격은 그
  방어막이지 주석이 아니다: 고정한 컴파일러에서는 `#`을 하나도 담지 않은 문서에서조차
  `to_toml_medium` +3.29 %, `to_toml_long_strings` +3.43 %, `to_toml_notes` +1.24 %이고, 강제
  이미지에서는 같은 세 이름이 +1.256 %, +2.081 %, +6.872 %다; TOML 읽기 하한 둘은
  +0.0014 %와 -0.0007 %를 움직였는데, 그 칸은 읽기 경로가 한 번도 물어보지 않는 자리다. 한 단계
  위에서는 허브가 멤버 위 줄을 끝내는 주석을 컨테이너 자신의 선행 목록에 놓는데 두 압축 갈래가
  그것을 무시했다 - PR #146의 다섯째 칸 문제가 한 단계 위에 놓인 형태다; 두 갈래 모두 이제 그 목록을
  `container_leading_notes` 하나로 묻고, PR #114가 헤더에 하듯 강등한다 - 어떤 읽들이 거부하는
  철자다 (2026-10-29). 다섯째 칸은 두 번 쓰였다: 읽기는 배열 요소 위의 `#`을 그 요소의 줄 끝에
  정리하는데, inline 표 요소에는 그 자리가 곧 `write_notated_table`의 선행 주석 칸이기도 하므로,
  배열이 그것을 위로 올리고 표가 중괄호 안에서 다시 썼다 (crash-3e95a4cf, crash-6802dabe,
  crash-9c0b2acb, crash-1ec1ea89, crash-20cc6b69).
- **점 경로도 다른 모든 하강과 같은 상한에 묶인다** — `parse_inline_table`은 멤버 키 경로를
  `nest_inline`에서 펼쳐 놓았으므로 `t = { a.b.… = 1 }`이 구간마다 표를 하나씩 중첩했는데,
  `parse_value`라는 통로도 `register_kv`의 검사도 다 밖에 있었다: crash-2a5ea394가 바로 그 모양인
  구간 1059개 입력이고, `from_toml`은 그것을 받아들였지만 1.1 읽기는 `to_toml`이 다시 철자한 결과를
  거부했다 (`max depth exceeded (max=1000)`). 예산은 이제 그 위에 펼쳐지는 모든 경로에 펼쳐진 경로의
  길이까지 더한다 (`depth + inline_path_shift + key.len()`). `DEFAULT_MAX_DEPTH`는 `parse_value`만
  묶었고, 점 키와 표 헤더는 자기 표를 방어막이 한 번도 세지 않은 반복형 포인터 하강으로 짓는다;
  그래서 긴 경로는 더 깊은 트리를 짓고 두 갈래로 실패했다: 다시 철자한 발출이 거부되거나, 네이티브
  스택이 넘쳤다 (`0xC00000FD`) - crash-7a3aac52, 2026-10-24 실행에서, 2 MiB 스레드에는 중단되고 512
  MiB에서만 깨끗했다. 이제 두 하강이 예산 하나를 나눈다: 구간 1000개까지는 왕복하고 자리잡고, 1001개
  이상은 형식화된 `max depth exceeded`다; 2026-11-01 실행의 crash-2503e079, crash-6c15f693과
  crash-837bd790이 바로 그 모양이다. 방어막 없이는 읽기가 `load_toml_medium` +0.006 %와
  `load_toml_long_strings` +0.046 %만 움직일 뿐이고, 하한 위에 커 보이는 +1.65 % / +2.98 %는 부모
  트리에서도 그대로 재현되므로 WSL 호스트의 기준선 흔들림이다.
- **등록된 handler는 자기가 주장하는 스칼라의 주인이고, 배열과 숫자 프로토콜보다 먼저 물어본다**
  — #671이 열어 둔 모양들은 이제 규칙 하나로 합쳐진다: `__bool__`만 실은 클래스를 `safe_dump`한 것은
  `m:\n  !booly B\n`을 발출했지만 AST 경로는 `m: !booly B\n`을 발출했다 - 직접 라이터가 등록부에
  `write_other`로 닿는데 그 함수가 자기 들여쓰기를 발출하기 때문이다; 같은 클래스는 시퀀스
  항목에서는 줄표 줄을 잃고 키에서는 `YamlTypeError`를 올렸다. `numpy.ndarray`용 handler를 두면 배열
  경로가 먼저 와, AST 경로가 `!nd ND`로 답하는 자리에서 배열 자체로 답했다. 두 라이터는 이제 모든
  자리에서 배열, 프로토콜, 자기 철자 규칙보다 먼저 handler를 묻는다. 등록부 검사는 float 모양만을
  위해 써서, `python_type`이 `__index__`를 실은 `register_type("!money", ...)`는 돌아나갔다:
  `safe_dump({"m": Money(500)})`은 프로토콜이 되돌려주는 정수인 `m: 500`을 발출했다 - `Decimal`은
  지켜졌고 정수 모양 값형식은 한 번도 지켜지지 않았는데 - 이제 등록부를 먼저 묻고, 다만 `int`나
  `int` 하위 클래스 - `bool`이 그렇다 - 는 읽기가 해석할 정수 철자를 그대로 둔다. `str` handler는
  여전히 텍스트에 손대지 않고 `dict`나 `list`는 넘긴 쪽의 컨테이너로 쓰인다; 배치 질문이 handler를
  돌려 답할 수 없으므로 `type_registry::claims`가 있다.
- **NumPy 배열을 덤프하는 일은 Python 3.15에서 더 이상 프로세스를 끝내지 않는다** — 배열 경로는
  `rust-numpy`로 객체를 캐스팅하고 그 메모리를 읽었는데, `cp315-abi3t` wheel은 그것을 건전하게 할 수
  없다: 출시된 0.18.3에서 3.15 GIL 아래 `safe_dump(numpy.arange(8))`은
  `Fatal Python error: Segmentation fault`로 죽었고 (종료 139), 같은 바이트는 3.15t에서
  `YamlTypeError`로 답했으며, 3.12, 3.14와 3.14t는 올바르게 덤프했다. 결정적이고 단일 스레드였으며
  `numpy.zeros((0,))`도 죽었으므로, 과오는 배열 내부를 처음 읽는 자리에 있었다.
  `crates/pyrs-yaml/src/py/ndarray.rs`는 이제 배열에게 자기 `dtype`, `shape`, `tobytes()`를 요구해
  그 버퍼를 Rust에서 해독한다: 어떤 산출물에서도 numpy C API에 닿지 않고, `numpy` Cargo feature는
  의존성이 아니라 그 경로를 관문한다. 그것과 함께 결함 둘이 쓰러졌다: 옛 경로는 메모리 순서로 읽고
  논리 모양으로 중첩하므로, 전치된 뷰가 전치되지 않은 격자를 썼고; 두 라이터 모두 배열 경로보다
  Python의 숫자 프로토콜을 먼저 물었는데, 그 프로토콜은 원소 하나짜리 배열에서 차원을 떨군 채로
  답한다 - 그것이 오류가 된 것은 numpy 2.3부터 - 그래서 어떤 설치에서는 `1.5`, 다른 설치에서는
  `- 1.5`가 쓰였다. 차원을 실은 배열은 이제 그 프로토콜에서 거부되고, 0-D 배열은 스칼라 철자를
  지킨다.
- **주석은 더 이상 자기 줄을 끊을 수 없고, 블록 스칼라는 자기가 담고 줄바꿈을 철자할 수 있다** —
  `arb_comment`는 주석 텍스트를 `[ -~]+`에서 뽑았으므로, 한 줄 주석 담이 빚진 평탄화에 닿는 속성
  테스트가 하나도 없었다; `\n`과 `\r`까지 뽑게 하자 속성 네 개가 붉어졌다 - YAML과 TOML 라이터가
  줄바꿈 뒤의 텍스트를 문서에 넣었기 때문이다 - TOML은 자기 출력을 거부하고
  (`bare carriage return (must be part of a CRLF pair)`), 배열 항목의 주석 `"<\n&"`는 맨 `&`으로
  돌아왔다. 두 라이터는 이제 `push_note_text` 한 곳에서 줄바꿈을 빈칸으로 평탄화한다; 할당 없는
  `Cow` 모양은 여전히 `serialize_note_flow`에서 +8.80%를 요구했고, 그래서 출시된 철자는 자기가
  움직인 하한으로 값을 매긴다: 62,066,033 → 64,411,980 (+3.78%). 블록 쪽: 생성기는 줄바꿈 없는 인쇄
  가능 ASCII만 뽑았는데 - `Literal`/`Folded`에는 바로 그 줄바꿈이 그 스타일의 전부다 -
  `[ -~\n\t]+`로 뽑게 하자 `needs_double_quoted`가 붉어졌다; 가장자리 검사가 빈칸과 탭은 막아도
  줄바꿈은 안 막았고 비문자열 짧은 경로가 먼저 돌므로, null로 해석되는 `"\n"`가 맨으로 발출돼서
  `~`로 돌아왔다. 마지막 부류는 층 자신의 것이었다: `|-`는 설계상 뒤따르는 줄바꿈을 버리므로, 그
  층은 이제 읽기가 하듯 주장을 해석한 뒤에 비교한다; 단순 키까지 해석하면 `"'\n"`가 옆의 평문 `'`
  위로 접혔다.
- **한 줄 문서의 스트리밍은 다시 선형 시간이 된다** — `LineTable::line_col`은 이벤트마다 그 줄의
  앞에서 문자를 세었으므로, 소스에 줄바꿈이 없는 생성자는 이벤트마다 O(n)을 지불했고 스트림은 제곱
  시간이 됐다: 1.5 MB minified JSON 배열은 자기 이벤트 224,006개를 보고하는 데 4.9 s를 썼고, 입력이
  두 배로 늘 때마다 네 배 오래 걸렸다. 이제 그 표는 마지막으로 답한 위치에 커서를 두고 한 이벤트와
  다음 이벤트 사이 바이트만 걸으며, 커서보다 앞의 위치는 여전히 자기 줄의 시작에서 센다; 같은 문서가
  19 ms에 스트리밍되고, 기록마다 줄바꿈을 둔 짝 문서는 18 ms다. JSON과 TOML이 그 표를 나누어 쓰고,
  YAML은 자기 위치를 따로 계산해 이 비용을 한 번도 낸 적 없다.
- **관문 여섯은「지켜짐」을 답할 수 있었고 단언 열은 아무것도 재지 않은 측정에서 초록을 보고할 수
  있었다** — `(fz)` 등급은 한 번에 쓸었다: `check_changelog_coupling.py`는 빈 경로 목록에서 종료
  0으로 빠졌고, `check_doc_headings.py`는 실제로 연 파일이 아니라 넘겨받은 경로를 셌으며,
  `check_line_endings.py`는 `git ls-files`의 종료 코드를 한 번도 확인하지 않았고,
  `check_doc_signatures.py`와 `check_i18n.py`는 빈 glob 위에서 OK 줄을 찍었으며,
  `ir_gate.py --only a,b` - `--only`는 추가하므로 그건 아무것도 매칭되지 않는 이름 하나다 - 는
  시나리오 하나도 대조하지 않고 하한이 지켜졌다고 보고했다. 여섯 모두 이제 종료 2로 거부하고, 각각의
  거부를 테스트가 강제하며, 자성 테스트라고는 하나도 없던 `check_i18n.py`는
  `tests/test_i18n_gate.py`를 얻었다. 같은 모양이 스위트에도 앉아 있었다: `assert`가 `for` 본문
  안에만 있는 테스트는 빈 iterable에서 통과하고 `all(x for x in [])`는 `True`라는 점이다 - AST
  순사가 후보 41개를 이름 지었고, 그중 iterable가 테스트 밖에서 들어오는 열 개는 이제 실측된 숫자를
  적힌 하한을 갖는다; 나머지 32개는 리터럴 iterable, 다른 테스트가 하한을 둔 상수, 빈 경우가 곧
  단언인 루프다 - 순사가 일회용 스크립트이고 빈 루프 테스트를 막을 것이 아직 없으므로 `(ge)`로
  기록했다.
- **측정 대신 건너뜀을 보고한 방어막, 그리고 재보지 않은 대항 라이브러리를 이름 올린 순위표 행** —
  `load_stream_file`의 메모리 상한은 `pytest.importorskip`으로 `psutil`을 요구했는데, 그 패키지를
  이름으로 적힌 요구 목록이 없으므로, 모든 레그는 커버리지처럼 읽히지만 커버리지가 전혀 아닌
  건너뜀을 보고했다. 그것은 표준 라이브러리 `tracemalloc`의 크기 불변식으로 대체됐다: 같은 경로를 돈
  2,000줄 문서와 200,000줄 문서는 같은 추적 발자취를 남겨야 한다 - 두 크기 모두에서 3.12, 3.14t는
  각각 추적 발자취를 3,168바이트와 3,200으로 남긴다 - 반면 버퍼링하며 도는 순회는
  199,854,440바이트로 커지므로, 누적하는 읽기는 그 증가량의 백분 일에 놓인 상한을 통과하지 못한다.
  같은 맹점이 순위표에 앉아 있었다: `tests/test_benchmark_crosslib.py`는 고정물을 받아들일 수 없는
  대항 라이브러리를 건너뛰고, `yaml_edit` 갈래는 "newer yaml_edit releases"를 탓했는데, 고정한
  0.1.3을 대조하면 세 고정물이 모두 `Document::from_str: Input contains stream-level comments`를
  올렸으므로, 그 행은 설치된 패키지 옆에 건너뜀 셋이었다. 범위를 좁힌 `uv lock --upgrade-package`로
  얻은 0.2.0, 세 중 둘을 해석하므로, 그 행은 그것들을 재고,
  `test_yaml_edit_row_measures_something`이 이제 거기에 하한을 둔다: 설치됐다는 것은 고정물 적어도
  하나는 받아들인다는 뜻이어야 한다.
- **훅 집합은 붉은데 판결은 초록이었고, 결함 주장 관문은 너무 적게 읽었다** —
  `Test matrix (all legs)`는 `ci.yml`의 일감들에서 계산되고 `toJSON(needs)`는 다른 workflow의 일감을
  실을 수 없으므로, `hygiene.yml`에 사는 `prek hook set on the full tree`는 그 관문이 읽은 이름이 한
  번도 없었다: PR #632의 첫 head에서 그 check-run은 `failure`를 보고했다 - 출시 노트 없는 changeset
  때문에 `check_changelog_coupling.py`가 거부한 것 - 반면 같은 sha의 필수로 지정된 이름은
  `success`를 보고했다. 그 일감은 이제 `ci.yml`에서 돌고, `quality_matrix.py`는 판결이 기다리는 바로
  그 일감에서 건너뛰기 목록을 읽는다. `scripts/check_hole_claims.py`는 `AGENTS.md`와 `CLAUDE.md`만
  읽었고, 그래서 낡은 주장 하나가 여덟 곳에서 #613과 #614를 넘어 관문 모두 초록인 채로 살아남았다;
  범위는 이제 현재형이다 - 지시 파일 둘에 각 거울의 `[Unreleased]` 블록을 더하고,
  `check_changelog_mirrors.py`의 새 `unreleased_lines`가 잘라내며, 어느 거울에서든 그런 줄이 안
  나오면 그 함수는 종료 2로 거부한다. 그 관문은 이어서 파일을 읽기 전에 `3.15t` 레그에서 실패했는데,
  쓰기 지점보다 95줄 위에 정의된 `_VERSION_HEADING_RE`의 `NameError` 때문이었다:
  `scripts/__pycache__`를 비운 뒤 차갑게 돌리면 20 중 20이 붉고 따뜻하게 돌리면 초록이므로, 촉발은
  그 모듈의 첫 컴파일이다 - 호출 전에 모듈을 지역 변수에 묶는 것이 치료다.
- **편집 하한은 출시된 호출자가 돌지 않는 스캔에 값을 매겼고, 갱신 일감은 첫 이름 뒤의 이름들을
  떨어뜨렸다** — `edit_*` 세 시나리오는 편집 계층에 `line_offsets: None`을 넘겼고, 그 계층은
  `None`에 대한 답으로 문서의 줄 표를 다시 짓는다 - `scan_yaml(text).line_offsets`, `has_hash`,
  `has_amp`, `is_ascii`까지 함께 결정하는 전체 통과다: 기준선의 `rustc 1.97.1`에서 그 스캔은 세 루프
  각각에서 센 명령어 10,104,000개를 썼고, 바인딩은 그 표를 문서별로 기억한다
  (`Splice::line_offsets`). 이제 하네스는 표를 파싱 옆에서 만들고 세 동사 모두에 `Some(&offsets)`를
  넘기며 `--setup-only`가 그것을 빼게 하고, 강제 이미지의 갱신 run 81은 −24.45% (`edit_set_nested`
  48,288,771 → 36,483,718), −27.48% (`edit_insert_sequence` 42,462,454 → 30,793,506), −27.86%
  (`edit_delete_key` 42,392,088 → 30,583,030)를 읽었고, 스캔 몫 20.9%를 넘는 초과분은 `Vec`의 할당과
  해제 왕복이었다. 그 하한을 옮기는 일감은 이름들을 떨어뜨렸다: `profile`과 `accept_drift`는 입력을
  `read -ra <<< "$VAR"`로 쪼개는데 `read`는 한 줄만 먹으므로, 갱신 run 40221999337은 두 줄로 적힌
  `profile` 입력을 받고 "attributing 1 scenario(s)"를 남겼다; 두 단계 모두 이제 구분자를 납작게 한
  뒤 쪼갠다.
- **퍼즈 보고서는 더 이상 발견을 버리지 않고, 그 개수와 등록기도 큐에 닿는다** — 아카이브는
  `artifacts/`와 `collected/` 두 루트를 담고 있다; run 41541555317의 다운로드는 그 루트들을
  `--crashes`에 그대로 풀었으므로, 경로 첫 구간을 대상으로 읽는 일이 `artifacts`와 `collected`라는
  대상 둘을 만들어 냈고, `yaml_roundtrip` 아카이브 하나에 있는 입력 셋은 issue #739와 #740, 두
  issue로 등록됐으며, 요약은 각 입력을 두 번 셌다. 이제 대상은 그 어느 루트 뒤의 구간이고, 요약은
  `collected/`만 읽는다. 보고 단계는 각 대상을 `grep -c .`로 셌는데 `-eo pipefail` 아래에서였고 단계
  자신의 `set -uo pipefail`는 그것을 끄지 않으므로, 영인 수는 종료 1로 단계를 끝냈다: 크래시 파일이
  없는 아카이브는 표에 헤더만 남기고, 아래 등록 단계는 한 번도 돌지 않았고, main push에서 나온
  발견은 어떤 소비자에게도 닿지 않았다 - 이제 개수는 영도 보고할 값으로 다룬다. 세 번째 틈은
  서명이었다: `grep -m1 -hoE "panicked at [^:]+"`는 첫 colon에서 멈추므로, panic은
  `panicked at fuzz_targets/yaml_roundtrip.rs` 형태로만 보고서에 닿고 줄, 열, 메시지는 남지 않았고,
  예약 실행 층은 다섯 토요일 연속 `completed/failure`로 돌아왔다 (2026-10-03, -10, -17, -24, -31);
  이제 그 서명은 위치와 메시지 첫 줄을 함께 남긴다.
- **게시된 문서: 거짓 `Node` 주장, 포맷터가 지운 바이트, 잘못 놓인 섹션들** — `Node` 페이지들은
  `is_valid()`가 수정된 문서도 거부한다고 적었지만 그 함수는 문서 참조만 시험한다: 편집 때문에
  낡아진 노드는 자기 `repr`이 `Node(invalid)`로 뒤집힌 뒤에도 `True`로 답하고, `value`를 읽으면
  `YamlDocumentError`가 난다. 같은 repr은 원인 세 개를 한데 둔다: 낡은 노드, 없는 경로, dict 투영이
  거부하는 경로 - 읽기는 `to_dict()`를 지나기 때문이다: `0set: 1` 옆의 `!base 0set: 2`에서는 모든
  `Node` 읽기가 `YamlTypeError`를 올리고, 없는 경로는 `KeyError`를 올린다; 쓰기는 AST를 돈다. 네
  로케일이 모두 그것을 싣고 있었고 이제 다 바로잡았다. `rumdl fmt`의 MD038은「code span 요소 안의
  빈칸」을 그 바이트를 지워서 고치므로, backtick, 빈칸, `? > #I` 순으로 쓰인 span은 그 빈칸을 거울
  다섯 곳과 장부에서 잃었고 훅은 `Passed`를 찍었다: 그 전제는 CommonMark이 렌더에서 벗겨 내는 양쪽
  짝인 경우에만 성립하고, 앞에 붙은 빈칸 하나만은 YAML marker body를 들이게 하는 들여쓰기다.
  `MD038`은 `.rumdl.toml`의 비활성 목록에 들어갔고, `rumdl check`이 페이지 178개에서 발견 0건을 읽은
  시점이 2026-11-01. 로케일 changelog 페이지들은 섹션을 그 버전 제목과 같은 단계에 두므로, 사이트는
  각 섹션을 그 버전의 형제로 렌더링했다; `check_changelog_mirrors.py`가 이제 그것을 확인한다.
- **공개된 벤치마크 표는 네 형식과 Rust 기반 읽어들기를 덮는다** — 측정면은 이미 `json`, `orjson`,
  `tomllib`, `tomlkit`, `pyjson5`와 겨루고 있었지만, `.ci/perf-snapshot.json`은 PyYAML과
  ruamel.yaml만 기록했다. `scripts/bench_snapshot.py`는 이제 형식 축을 잰다 - YAML은 parse,
  serialize, 왕복을, JSON, TOML, JSON5는 parse를 - 그리고 열을 세는 대신 `<!-- bench: ... -->`
  표시로 각 표를 자기 측정에 묶고, `tests/test_perf_docs.py`가 양쪽으로 묻는다: 잰 항목이 공개되지
  않아도 실패하고, 재는 것을 하나도 가리키지 않는 표시도 실패한다. 새로 들어온 두 행은 패배이고
  패배로 인쇄된다 - 1.9 KB 문서에서 `orjson`이 1.5x 빠르게 읽고, JSON5 설정에서 `pyjson5`가 2.4x
  빠르다 - 반면 TOML은 `tomllib`보다 5.5x에서 5.9x 빠르다; JSON, TOML, JSON5 쓰기는 계속 미공개로
  남는데, 그것들은 Python 객체가 아니라 엔진의 AST에서 나오기 때문이다. Rust 기반 읽어들기는 기록
  어디에서도 재지 않았으므로, `yaml-rs`와 `ryaml`이 YAML 축에, `rtoml`이 TOML 축에 합류하고; 다시 쟌
  스냅샷 (2026-10-25)은 `yaml-rs`가 pyrs-yaml보다 parse에서 2.4–2.6x, 왕복에서 1.6–1.9x 빠르고
  `rtoml`이 `load_toml`보다 1.9x 빠르게 읽으며, pyrs-yaml은 `yaml-rs`보다 serialize에서 2.2–4.1x
  빠르다고 적는다.

#### 변경

- **스트림을 단일 문서 경로에 물으면 거부하고, 두 항목 사이 빈 줄은 못박힌 경계가 되고, 옮겨진 TOML
  주석은 맞는 원인을 이름 지는다** — `parse`, `parse_file`, `safe_load`, `to_toml`과 `YAML()`의 대응
  메서드는 문서 하나만 읽는데, 문서가 둘인 입력은 순회가 마지막에 닿은 쪽을 돌려줬다:
  `a: 1\n---\nb: 2\n`는 `b: 2`로, `a: 1\n---\n`는 `null`로 돌아왔다. 이제는 모두
  `multi-document-input`으로 시작하는 `YamlParseError`를 올리고 두 번째 문서의 행과 열을 알려주며
  `parse_all_docs`, `safe_loads`, `parse_stream`으로 안내한다. PyYAML과 ruamel.yaml도 같은 열 가지
  입력에서 올리지만, 앞의 `---`, 뒤의 `...`, `---` 혼자, 빈 입력은 여전히 받아들인다: 방어막이
  과하게 터지지 말아야 할 대조 표본이다. CLI의 `-A/--all-docs`도 같은 말이고 동작은 그대로이며,
  `(hg)`로 기록했다. 여덟 위치와 편집 경로에서 빈 줄은 어디에서도 남지 않고 값·키 순서·앵커·주석은
  남는다; `tests/test_roundtrip.py`는 오늘 그 고정점 그대로를 못박으므로, 그 격차를 메우면 못박은
  주장이 붉어진다. 옮겨진 TOML `[sec] # note`는 YAML 파서의 주석 수집 모델로 돌렸지만,
  `preserves_section_header_trailing_and_leading`은 고정점을 보여 준다: 이동은 이 프로젝트 자신의
  규칙에서 온다 - 컨테이너의 제자리 주석을 그 본문 마지막 줄에 놓는 규칙이다.
- **주석은 모든 다리의 양쪽에 각자의 하한을 가졌다, 읽는 쪽도 포함** — 고정물을 센 일은 PR #641의
  쓰는 쪽 하한 귀인을 뒤집었다: 그때 serialize 시나리오가 흘리는 YAML은 `#`이 하나도 없고 다섯 쓰기
  입력은 `{`, `[`, `]`, `}`가 하나도 없었다. 그래서 주석 보조 함수 다섯은 측정된 질문 어느 것에도
  답한 적이 없고, 방언 라이터에도 `#`이 없는 AST가 주어졌다. 이제 세 문서가 자기 절반이 답하는
  모양을 싣는다: `serialize_note_flow`는 다섯 답을 하나씩 그 문서로 순회하고, 이미지는 그 순회에
  12.61%를 귀인한다; `to_jsonc_notes`과 `to_json5_notes`는 PR #643이 블록 절반을 한 통과로 가져가기
  전에 `<str>::replace`를 양쪽에서 가장 큰 자기 항으로 놓으며 각각 16.08%와 16.01%였다; 그 읽기
  쌍둥이는 주석 일곱 개 위에서 `ws_extended`를 19.86%와 19.79%로 첫 자리에 놓는다. `#`이 없는 고정물
  앞에서 바이트 스캔과 주석 이관은 가장 싼 답변 위로 돌았으므로, `parse_notes`와 `stream_notes`는
  쓰는 쪽 행이 쓰는 바로 그 문서를 읽는다: 주석 자리를 공백으로 지운 같은 434바이트에서, 열한 개
  주석은 트리를 만드는 경로에서 +16.30 %, 이벤트 경로에서 +19.37 %를 더 요구하고, 그중 71 %와 74 %는
  고정된 파서 안쪽에 있다. `ir_fixtures.rs`는 고정물에서 주석 하나가 사라지면 이름을 짚어 빨개지고,
  그 갱신은 `from_json5_medium` +0.55 %에서 자기 자신을 거부했으므로 그 이름을 `accept_drift`에
  적었고 `tolerance_hint`는 0.5 % 그대로다.
- **메타데이터 이관에 두 하한이 붙고, 인터프리터를 세는 하한에는 runner 이미지 키가 생기고, TOML
  crate는 자기가 세우는 트리를 적는다** — 치환은 매번 `merged_meta`에 무엇을 옮길지 묻고
  `delete_path`는 `extend_delete_over_comments`에 범위 확대를 묻지만, 이 분기들에 하한은 닿지
  않았다: `edit_*` 세 시나리오는 `MEDIUM_YAML`을 달리므로 거기에 `#`, `&`, `!`이 없고, 모든 분기는
  빈손이었다. 26자 더 작은 문서에서 `edit_set_metadata` 42,838,349를 `edit_set_nested` 36,483,730
  곁에 놓는 수는 이관 자체이지 더 큰 트리의 복사가 아니고, 그 고정물에 장식을 추가하면
  `ir_fixtures.rs`가 붉어진다. 그 실행은 전혀 들어가지 않는 두 하한을 움직였다 —
  `documents_short_strings` −10.46%, `documents_long_strings` −7.00% — 전부 glibc의 복사 행 하나에
  놓인다. `.ci/ir-baseline.json`에 `image_floors`가 추가되고, `scripts/ir_gate.py`는 실행을 자기
  이미지의 행으로 판단한다 — 인터프리터까지 세는 시나리오에만 키를 둔다. 하나의 트리에서 두 하네스
  요약이 같은 조건에서 `load_toml_medium`은 142,467,241과 144,904,012를 읽었고, 그 1.71 % 산포는
  하한 하나로 담을 수 없기 때문이다. 행이 없는 이미지는 판단 대신 거부한다(exit 2).
  `crates/pyrs-toml/src/lib.rs`는「중간 값 트리는 없다」로 시작했지만, `(fw)`는 그 표 삽입을
  `load_toml_medium`의 8.10 %와 `from_toml_medium`의 9.68 %로 값을 매겼다: 다시 쓰기는 일정에도
  없으니, 그 문장은 이제 자기가 내보내는 빌드를 적는다.
- **이벤트 스트림과 줄 경로에 긴 값 모양의 하한이 생기고, 긴 본문은 줄 경로에서 더 비싸다** —
  `stream_long_strings`는 쓰기 쪽 하한들이 쓰는 그 여덟 개의 256문자 레코드에 같은 `Work::Stream`을
  돌리므로, 이 수치들은 문자열 모양만 다르다: 276,633,813 대 `stream_medium`의 255,359,778,
  **+8.33%** — 이 관문이 이벤트 경로에서 긴 본문에 대해 처음 내놓은 읽기다. `Work::Documents`는
  `pyrs_json::parse_documents`로 한 줄에 하나인 여덟 레코드를 이벤트로 읽고, 값의 폭만 다르다: 9문자
  대 256문자. 95,815,186 대 328,951,144, **+243.3%**이고, 같은 짝을 이벤트 경로에서 재면
  **+8.16%**다. 다시 측정하자 이미 공개된 비율도 움직였다 — 이벤트 짝은 처음 8.33%로 적혔고 이제
  8.16%로 읽히며, `QUALITY_MATRIX.md` §2는 그 수를 싣고 `tests/test_ir_baseline_workflow.py`는
  기억이 아니라 파일과 대조한다. 서른한 개 하한은 이벤트 실행에서, 서른세 개는 줄 실행에서 왔다.
- **긴 문자열 모양이 두 채널에서 값을 매기고, 그 표면은 다섯 형식 전체를 덮는다** — 어떤 벤치마크도
  긴 값을 읽지 않았으므로(`MEDIUM_JSON`의 가장 긴 값은 21문자) 벡터화된 스캔은 측정되지 않은 채였다.
  `long_string_json` / `_toml` / `_yaml`은 256문자의 오직 문자로만 된 값으로 같은 목록을 생성하고,
  모든 다리의 읽기·쓰기 양끝에서 읽힌다: 랭킹 채널에 벤치 넷(읽기 2.599 µs와 3.499 µs, 쓰기 950 ns와
  1.174 µs), 강제 채널에 하한 넷. 소스 바이트당으로 긴 문서는 `from_json_medium`의 5분의 1,
  `from_toml_medium`의 8분의 1을 요구한다. `scripts/quality_matrix.py`는 한쪽에만 번호가 매겨진
  다리를 `ir-bridge-unidirectional`로 보고하므로, 방언 다리는 양끝으로 돈다 —
  `from_jsonc_long_strings`, `from_json5_long_strings`, `to_jsonc_long_strings`,
  `to_json5_long_strings`, 각각 산포 0.000% — 앞선 네 하한도 다시 쟘: `from_json_long_strings`
  112,664,928 → 112,987,111, `from_toml_long_strings` 109,254,466 → 109,535,451,
  `to_json_long_strings` 58,550,997 → 58,550,735, `to_toml_long_strings` 72,438,982 → 72,438,675. 그
  `long_string_jsonc`(2,557바이트, 두 주석 형태 모두)와 `long_string_json5`(2,550바이트, 홑따옴표 —
  홑따옴표용 벡터 인스턴스가 본문을 스캔하는 방언)는 엄격 JSON 목록에 못박힌다.
- **하네스를 키우면 그 변경이 전혀 들어가지 않는 하한도 움직이므로, 이름은 예측이 아니라 파견 자신의
  거부에서 가져온다** — 15개, 이어서 13개, 이어서 18개 하한이 0.005%를 지나 건넜고, 0.50% 선을 넘은
  이름은 세 번의 확장마다 달랐다: `from_json_medium` 78,671,707 → 79,283,878 → 79,904,977는 앞두 번
  각각 한 단계에 +0.78%로 건넜고, 세 번째 확장은 그것을 0.41% 되돌려 놓는 대신 이 변경이 전혀
  들어가지 않는 읽기 네 개가 건넜다(`from_json5_medium` +1.05%, `from_jsonc_medium` +0.72%,
  `parse_inline_merge` +0.67%, `from_json5_long_strings` +0.50%); run 74는 그 네 개를 짚어 거부했고,
  run 75가 그 이름들을 적어 기록했다. 뒤이은 실행은 하나씩 이름을 필요로 했다: `parse_inline_merge`
  +1.03%(`stream_medium` +0.485%는 선 안), 이어서 `from_json5_long_strings`
  +0.82%(`from_json5_medium` +0.479%와 `from_json_medium` −0.905%는 선 안); 26에서 30으로 늘린
  확장은 아무도 넘기지 않았고, 가장 큰 움직임은 `from_toml_medium` +0.363%였다. `stream_medium`은
  변수가 하나만 다른 조건에서 +0.01%와 +0.63%를 읽었고, `accept_drift stream_medium` 하나로만
  통과했다. `tolerance_hint`는 0.005 그대로고, 마흔일곱 개 하한은 그 실행들에서 왔고 run 79가 다시
  쟀으며, 고정된 트리에서 바인딩 채널은 이미지의 하한 대비 최악 +0.15%이고 스캐너 계열 세 시나리오는
  최대 -21.23%까지 벌어진다. 그래서 §2의 호스트 간 상한은 그 시점의 읽기로 표시했다(장부 (gk)).
- **Rust 벤치마크는 항목마다 자기 프로세스로 돕니다, 그리고 두 잠금 파일은 선언한 범위가 허락하는
  최신으로 움직였다** — `codspeed-divan-compat` 5.0 → 6.0은 Divan 벤치마크를 프로세스별로 분리해
  돌리므로, 한 항목이 남긴 할당기 상태가 다음 항목을 물들이지 않는다. 상류 보고로는 1%를 넘어
  흔들리는 벤치마크가 약 70% 줄고, `CODSPEED_PROCESS_ISOLATION=false`로 이전 동작에 돌아간다. 합병을
  결정하는 것은 움직이지 않지만, 이 bump를 넘는 CodSpeed 경향선은 한 번의 측정이 아니다. `similar`
  3.2.0 → 3.2.1은 여덟 크레이트가 선언한 MSRV 1.96 하한이 남기는 유일한 Rust 단계이고, Python 패키지
  아홉 개가 움직였다: `zensical` 0.0.71 → 0.0.72, `mkdocstrings` 1.0.7 → 1.0.8,
  `mkdocstrings-python` 2.0.9 → 2.0.10, `python-dateutil` 2.9.0.post0 → 3.0.0, `deepmerge` 3.0.1 →
  4.0.0, 여기에 griffelib, markdown, 패치 두 단계. `maturin`은 1.15.0에, `pyo3`는 0.29 계열에 그대로
  남는다. 최신 `pytest`, `hypothesis`, `pydantic`, `numpy`는 3.8 하한보다 새로운 인터프리터를
  요구하기 때문이다. 실측: Rust 테스트 806건과 Python 테스트 4,306건, `-D warnings` 아래 clippy, 훅
  전체, 네 로케일 strict 문서 빌드.
- **`no_std` 호스트 테스트를 Ubuntu뿐 아니라 Windows에서도 돌리고, 단위 테스트 레그는 이름이 말하는
  설정을 실제로 돈다** — 그 잡은 wheel이 세 시스템에 배포되는데도 러너 하나에 묶여 있었다; 호스트와
  무관한 베어메탈 빌드는 하나에 남기고, 단위 테스트는 이제 넓힌다. 그 단계의 주석은 크레이트 목록을
  넓힌 일을「std 없이 한 번도 돌지 않은 호스트 테스트 112개의 값」으로 적었지만,
  `cargo test -v --no-run`은 여섯 크레이트를 한 호출에 담는 명령이 `pyrs-toml`, `pyrs_json`,
  `pyrs_ast`에 `--cfg feature="std"`를 싣는 것을 보여 주고, `-p pyrs-toml` 혼자에는 그것이 하나도
  없다: cargo는 한 호출의 패키지 전체에서 feature를 통일하는데, `pyrs-yaml-core`가 그 형제들을
  가리키는 dev-dependency 간선에 `default-features = false`를 적지 않았기 때문이다. 출시된 것은
  아무것도 영향을 받지 않았지만, 그 주장은 std 빌드를 재던 걸음 위에 서 있었다. 매니페스트 간선 세
  개가 그것을 막고, `tests/test_no_std_gate.py`는 `-p` 목록을 믿는 대신 매니페스트를 읽는다:
  `default = ["std"]`를 선언한 형제로 가는 기반 간선을 `default-features = false` 없이 거부하고,
  수정 전 매니페스트를 대조군으로 삼아 정확히 그 세 간선을 보고하며, 러너 하나로 후퇴한 잡도
  거부한다. std 없는 명령은 Windows에서 23개 표적에 724개 테스트를 돌려 실패 0개이고, 기본 모양은
  여전히 815개를 돈다.
- **자동 올림 봇은 하한의 컴파일러를 제안하지 못하고 설정도 잘못 철자하지 않으며, 문서 둘이 실측한
  것을 적는다** — 주간 `github-actions` 그룹은 action 두 개를 올리려고 #620을 열어 `ci.yml` 안의
  `dtolnay/rust-toolchain@1.96`을 `@1.120`으로 다시 썼다 — 그 ref는 `msrv-check`이 컴파일러로
  설치하는 값이다. 다섯 테스트가 빨개졌지만, `ignore` 없이 `patterns: ["*"]`를 둔 설정이 매주
  재제안을 보장하고 있었다; 이제 그 이름은 `.github/dependabot.yml`에서 제외되고, 그 제외가 지워지면
  `test_the_bump_bot_is_told_never_to_offer_the_floor_selector`가 빨개진다. 세 개의 `groups` 항목이
  `version-update:semver-patch`/`-minor`/`-major`를 실었다 — `ignore`/`allow` 어휘인데,
  `groups.*.update-types`는 `major`/`minor`/`patch`만 받기 때문이다 — 그래서 Dependabot은 아홉 날
  뒤에 자기 파일을 거부했고, 그 check-run은 어떤 합병 관문도 읽지 않는다; 다섯 값은 짧은 형태를 쓰고
  `tests/test_dependabot_config.py`는 두 어휘를 못박는다. `QUALITY_MATRIX.md`는
  `Seven blind spots were found by measuring`라고 여덟 항목 목록 위에 썼고, 이제 한 테스트가 그
  개수를 다시 구하며, 훅 집합 레그가 표에 들어온 뒤로는 아홉으로 읽힌다. `docs/dev/perf.md`의 세
  문장은 `AGENTS.md`가 바인딩 채널을 흔들린다고 불렀다고 돌렸지만(`grep -c noisy AGENTS.md`는 0),
  넓게 읽힌 그 항은 CPython이 프로세스마다 뽑는 해시 시드였고, 지금은 고정됐다.

#### 성능

- **쓰기 출력은 중간 할당을 더 이상 지불하지 않는다: 버퍼 용량 하나, 한 통과 주석 평탄화** — JSON,
  JSONC, JSON5, TOML, YAML 라이터는 각각 `String::new()`에서 시작했고, 512바이트에 이르는 일곱 번의
  재할당이 설정 파일 규모의 쓰기 대부분을 차지했다. 게이트가 고정한 `rustc 1.97.1`로 센 명령어 수:
  `to_json_medium` 22,884,635 → 17,718,056 (-22.6%), `to_toml_notes` 16,468,905 → 12,071,715
  (-26.7%), `serialize_block` 14,518,712 → 10,899,301 (-24.9%), `serialize_note_flow` 64,411,669 →
  59,227,943 (-8.1%) - 라이터 시나리오 열여섯 개가 모두 내려갔고 가장 작은 폭은
  `to_toml_long_strings`의 -5.2% - 움직이지 않은 트리는 `to_json_medium`의 하한을 +0.002%로 읽는다.
  `edit_set_metadata`은 -7.3%, 나머지 편집 시나리오는 -1.3%와 변화 없음 사이에, `safe_dump_*` /
  `to_python_*`는 0.28% 안에 있다. 블록 주석의 평탄화는 두 번 확보했지만 이제는 한 통과로 쓴다;
  그리고 강제 이미지에서 `to_jsonc_notes`과 `to_json5_notes`은 PR #642가 커밋한 하한보다 38.27%와
  38.11% 아래에 놓이고 나머지 45개 시나리오는 0.16% 안에 들어간다(`to_json_medium` -0.0004%). 그
  통과에 여섯 곳의 줄 모양 자리를 넘긴 일은 그곳에서 2.5포인트를 더 벌었으나, 주석에 닿지 않는 쓰기
  여섯 개를 `ir_gate.py`의 여유 밖으로 밀었다(`to_jsonc_medium` +1.63%, `to_json5_medium` +1.59%,
  `to_jsonc_long_strings` +1.44%, `to_json5_long_strings` +1.42%, `to_json_medium` +1.02%,
  `to_json_long_strings` +0.96%) - 주석에 닿지 않는 자리에서 2.5포인트는 1%를 되갚지 못하므로, 그
  절반은 다시 나왔다. 출력 바이트는 하나도 바뀌지 않는다.
- **읽어들기의 문자열 본문과 TOML 행, 쓰기의 이스케이프 탐색은 SSE2로 열여섯 바이트씩 돈다** —
  JSON과 TOML 읽어들기는 문자열 본문을 바이트마다 스캔했고, TOML은 호출마다 소스 전체를 한 바퀴 돌아
  맨 캐리지 리턴을 거부했다; 쓰기의 이스케이프 판단은 긴 값 하한이 들어온 뒤(#600) 뒤를 이었다.
  SSE2는 x86_64 기본 명령어 집합에 있다 - 새로운 의존성도 `std`도 없다 - 바이트 루프는 벡터의
  끝부분으로, 다른 아키텍처에서는 전체 경로로 남겨 두었다. 개발 호스트에서: 5000바이트 문자열의 2 MB
  JSON이 2.47 → 1.97 ms, 같은 것을 6 MB CJK로 한 것이 6.17 → 5.27, JSON5 홑따옴표가 1.53 → 1.11, 2
  MB TOML 기본이 2.36 → 0.99, 리터럴이 2.41 → 0.96, 여러 줄이 3.37 → 1.01, 100바이트 값의 0.45 MB가
  2.98 → 2.54; 강제 이미지에서: `from_toml_medium` -4.02% (125,580,997 → 120,534,055),
  `from_jsonc_medium` -3.69%, `from_json_medium` -3.55%, `from_json5_medium` -1.09%. 이스케이프
  판단은 `to_json_long_strings`의 75.12%와 `to_toml_long_strings`의 69.01%로 측정됐고, 갱신된 하한
  네 개는 58,550,735 → 22,038,711 (-62.36%), 72,438,675 → 28,254,711 (-60.99%), 60,056,647 →
  23,544,666 (-60.80%), 60,480,707 → 23,968,721 (-60.37%)로 읽힌다. 짧은 값은 다른 모양이다 - 그
  순회는 그런 시나리오의 25.95-26.61%에 불과하고 - 하나만 값을 지불했다: `to_json_medium` +0.52%;
  `to_jsonc_medium` +0.48%과 `to_json5_medium` +0.47%는 선 안에 남고, `to_toml_medium`은 0.84%
  내려갔으며, `parse_inline_merge`(YAML 시나리오, 개발 호스트에서 +0.55%)는 이미지에서 +0.00%였다.
- **JSON 한 줄을 여덟 바이트씩 인덱스한다** — `pyrs_event::LineTable`은 줄 시작을 찾을 때도 이벤트
  사이에서 커서를 옮길 때도 UTF-8 문자를 한 걸음씩 세었고, 하한을 강제하는 이미지에서 그 순회는
  `documents_long_strings`의 51.79%와 `documents_short_strings`의 21.09%를
  차지했다(`docs/dev/quality-ledger.md` (gl)). 이제 `u64`가 여덟 바이트를 한 번에 답한다:
  `zero_lanes`가 끊김을 찾고, 후보 lane은 그 lane이 주장하는 바이트로 다시 확인한다 - 0인 lane에서의
  borrow는 바로 뒤의 `0x01`까지 끊김처럼 보이게 하므로, 새 속성 테스트가 그것을 출시 전에 잡아냈다.
  같은 호스트에서: `documents_long_strings` 308,642,512 → 126,840,512 (-58.84%), `line_col` 82.6%
  감소, `LineTable::new` 66.0% 감소, `documents_short_strings` 75,506,559 → 64,952,559 (-13.98%).
  위치는 하나도 바뀌지 않았고, 하한은 코드와 함께 움직였다. 강제 이미지에서:
  `documents_long_strings` 328,990,934 → 147,188,912 (-55.26%), `documents_short_strings` 95,854,968
  → 85,300,932 (-11.01%), 여기에 지역 프로파일링이 이름으로 밝히지 않은 시나리오 둘 - 같은 인덱스가
  JSON 스트림 경로 전체를 담당하기 때문 - `stream_json_medium` -14.90%와 `stream_json_stop` -27.17%.
  나머지 38개는 0.0051% 안에서 움직였다.
- **`load_toml`은 남길 수 없는 트리를 만들지 않고, `load_json5`는 두 형제가 이미 쓰던 AST를 거치지
  않는 경로를 따른다** — TOML 텍스트는 탐색, 표 트리, `CustomNode`, Python 순으로 진행했으므로
  평범한 dict가 서식 보존 노드 모델의 값을 치렀다. 이제 `pyrs_toml::read_toml`은 IR을 호출 측
  `TomlSink`로 읽고, 그 스칼라는 공유된 `scalar_projection`에서 오며, 관문은 태그 등록부다: 어떤
  handler든 걸린 동안 읽는 문서는 AST 경로를 탄다. 같은 실행의 `rtoml`을 자로 삼으면 공개된 행은
  1.92배와 1.91배 뒤처짐에서 1.47배와 1.46배가 되었고, 대략 1.31배 빠르다 - 두 스냅샷 사이에서
  호스트가 약 20% 흔들렸으니 절대 값은 증거가 아니다 - 관문이 고르게 하는 경로와 한 프로세스에서
  짝지으면 1.15-1.34배와 1.31-1.35배다. `json_fast::try_load`는 `load_json`과 `load_jsonc`에만
  연결돼 있었으므로, `load_json5`는 `CustomNode` 왕복을 통째로 지불했다: 같은 123 B에서
  `load_json`은 0.473 µs, 이쪽은 2.063 µs였다. JSON5는 엄격 JSON의 상위 집합이므로, 빠른 경로가
  받아들이는 것은 같은 값을 지닌 올바른 문서이고 방언이 더하는 것은 모두 AST 경로로 빠진다; 값은
  그대로이며, `json.loads`와 `pyjson5`를 맞대는 712개 테스트가 이를 판정한다. 같은 읽어들기의 자기
  방언 경로와 견주면 300개 항목에서 6.25배, 1,200개에서 9.34배 이기고 둘 다 5/5 짝이다 - 그 비를
  이제 `tests/test_json_leaderboard.py`가 관문으로 삼는다. 두 경로 어느 쪽도 명령어 수로 값을 매기지
  않기 때문이다.
- **로더 경로와 세 편집 동사, 스트림 중단에 명령어 수 하한이 생겼다** — 어떤 Ir 하네스도 그 경로에
  들어가지 않았다: `from_toml_*`는 트리에서 멈추고, `from_json_*`는 빠른 읽기가 건너뛰는 노드
  모델에서 멈추며, 바인딩 채널에는 `bench_to_python`과 `bench_load_toml`만 있었다. 두 시나리오가
  특성으로 막은 이음새를 통해 `load_toml`을 몰고, 그것은 `from_toml_medium`과
  `from_toml_long_strings`의 1.20배와 1.29배로 읽힌다 - 센 숫자 어느 것도 말한 적 없는 Python 쪽
  절반이다; `load_json_medium`과 `load_json_long_strings`는 `from_json_medium`과
  `from_json_long_strings`의 0.74배와 0.86배로 JSON 빠른 경로에 하한을 둔다. 이 경로는 트리에 얹는
  대신 그것을 대체한다; `load_jsonc`과 `load_json5`는 같은 행을 공유한다. 편집 시나리오 셋이
  합류한다. 동사당 하나씩 `MEDIUM_YAML`을 달리고, 복사와 파싱을 setup 안에 두니 남는 것은 동사다;
  `move_path`의 두 절반에도 값이 매겨졌다. `stream_json_medium`과 `stream_json_stop` 짝은 중단을
  전체 걷기의 25.09 %에 놓는다 - `Break`는 전달만이 아니라 스캔을 끝낸다; TOML에는 그런 행이 없다.
  그 `Break`는 발출을 세우기 때문이다. 기준선은 33개 시나리오에서 35개로 늘었고 이 행들이 형제를
  움직였다: `to_python_*` 세 행에서 +0.212%, -0.357%, -0.111%, `to_python_anchors` +0.1343%,
  `load_toml_long_strings` -0.2175%, diff에 엔진 파일이 하나도 없는데 JSON 읽기 여섯이
  +0.73%…+1.91%, `load_json_long_strings` +0.76 %는 runner에서 온 것이지 code에서 온 것이 아니다 -
  이름을 `accept_drift`에 적고, `tolerance_hint`는 0.5% 그대로. `(fs)`, `(ft)`, `(gf)`, `(gi)`,
  `(gh)`로 기록했다.
- **관문이 가장 넓게 읽은 항은 인터프리터였고, 이제 그것이 고정됐다** — 세 번의 갱신 실행은 바인딩
  하네스를 하나의 바이너리 요약(`7adbeac719edacd6`)으로 기록했고 아홉 표본이
  `load_json_long_strings`를 1.121% 벌렸지만, 같은 바이트의 엔진 시나리오는 0.00001% 안에서
  일치한다. CPython을 링크하는 쪽은 하나뿐이고, 그 항은 프로세스마다 뽑는 해시 시드였다:
  `PYTHONHASHSEED=0`에서 그 시나리오는 세 번 모두 95,136,157을 읽고, 일곱 바인딩 시나리오가 모두
  수렴한다(0.2811%, 0.2550%, 0.1249%, 0.0787%가 0.0000-0.0004%로). `scripts/ir_gate.py`는 그 시드를
  glibc 상한 옆에 고정한다 - 1.44% 호스트 간 차이를 지운 것과 같은 처방이다 - 그리고 이 head에서의
  갱신은 42개 시나리오가 서로 0.0010% 이내라고 보고한다. 하한이 움직인 곳은 시드가 일하던
  자리뿐이다: `load_json_long_strings` -0.526%, 다른 다섯 로더가 -0.166%에서 -0.055%,
  `to_python_medium` +0.051%, 엔진 쪽 35개 시나리오는 0.001% 이하, 위로 선을 넘긴 것은 없다. 선은
  0.5%에 그대로 남는다. 계측기가 더 이상 그 여백을 필요로 하지 않기 때문이고, 키가 사라지면
  `tests/test_ir_gate_provenance.py`가 이름을 짚는다.
- **빈 블록 본문은 선언된 chomping을 그대로 쓰므로, 그 점만 다른 두 컨테이너 키가 두 키로 남는다** —
  맵 동등성은 `chomping`을 읽으므로, 읽어들기는 `? hF&a?k !tag o: |+1` 옆의 `? hF&a?k !tag o: |`를
  모두 유지했지만, 쓰기는 모든 빈 본문에 `Clip`을 강제해 둘 다 `|`로 적었고, 자기 읽기가 그 발출을
  `DuplicateKey`로 거부했다 (libFuzzer `yaml_roundtrip` crash-469125e3, 최소화 후 48바이트).
  `effective_chomping`은 이제 빈 값에 대해 선언된 chomping을 돌려주고, 두 블록 라이터는 본문 줄을
  열지 않고 헤더 줄을 호출자의 종결 개행에 맡긴다 - 두 절반 모두 하중을 지고, 변형이 둘은 별개의
  주장임을 보여 준다: `Clip`으로 되돌리면 읽기 자신의 "duplicate key"로
  `container_keys_differing_only_in_chomping_survive`가 빨개지고, 헤더 개행을 되돌리면 그것은
  초록으로 남은 채 `empty_block_header_reaches_a_fixed_point`가 `">+\n"`이 기대된 자리에
  `">+\n\n"`로 빨개진다. 옛 규칙이 정규화했던 빈 `Keep` 시드(`>+8\r#`, `ancho: |+`; crash-89d81d99,
  crash-b5dcc38f)는 첫 발출에서 고정점이 되고, 그 입력은
  `fuzz/seeds/yaml_roundtrip/former-crash-469125e3.seed`로 보관된다.

</details>

### [v0.18.3] — 2026-10-22

<details markdown="1">
<summary>트리를 읽는 관문 · crate 행렬 · 주석 귀속 · 넓은 wheel 행렬</summary>

#### 추가

- **라이브 문서의 landed 주장이 코드를 지목합니다** — `docs/dev/event-stream.md`는 docs-only 커밋
  `0a074148`만 담긴 트렁크에 "decision 6 landed"라고 적고 있었고, 설계 문서의 현재 시제 서술을
  트리와 견주는 관문이 없었습니다. `scripts/check_landed_claims.py`는 이제 landed나 shipped라고 쓴
  제목 바로 아래 줄에 그 문장을 참으로 만드는 파일(`::식별자` 선택)을 적도록 요구합니다.
  `landed-claims` prek 훅으로, 그리고 `tests/test_landed_claims_gate.py`를 통해 실행됩니다.

- **취소된 CI 레그는 자동으로 재실행됩니다** — `.github/workflows/ci-rerun-cancelled.yml`는 pull
  request에서 CI 워크플로가 끝나면 발동하고, `scripts/ci_rerun_gate.py`를 통해, 한 레그가
  `cancelled`인데 나머지는 초록이고 `failure`는 없을 때에만, 시도 상한을 넘지 않는 범위에서 그
  실행을 재실행합니다. 상한은 무한 재실행을 막습니다. 전에는 `cancel-in-progress` 치환이나 러너
  회수로 취소된 레그가 이번 변경과 무관한 이유로 필수 검사 `Test matrix (all legs)` 판정을 빨갛게
  만들어, 사람이 Re-run을 누를 때까지 병합할 수 없었습니다. 판정의 허용 집합은 그대로 두었습니다 —
  `cancelled`는 여전히 병합을 거부 — 따라서 이는 예외가 아니라 재시도 하나를 더하는 것입니다.
  `tests/test_ci_rerun_gate.py`는 판정을 양방향으로, 배선을 계약으로 고정합니다.

- **YAML 엔진이 표준 라이브러리가 없는 타깃에서도 빌드됩니다** — `pyrs-yaml-core`이 이제
  `#![no_std]`입니다. 파서, 왕복 직렬화기, 주석을 보존하는 편집 계층, 스플라이스가 `core` + `alloc`
  만으로 동작하며, 그 아래의 `granit-parser`는 원래 std 없이 빌드된 crate입니다. `std` 기능은
  기본으로 켜져 있으므로 Python 바인딩, CLI, 각 벤치마크는 기존의 API를 그대로 유지합니다. `std`가
  더하는 것은 엔진 자체가 아니라 운영체제를 요구하는 세 가지 표면뿐입니다. 번역된 오류 메시지
  카탈로그 (rust-i18n은 프로세스 전역 locale을 소유합니다), 런타임 스키마 레지스트리(전역 `RwLock`),
  그리고 regex 기반 사용자 스키마 언어입니다. `no_std` 호출자는 `pyrs_schema::types::Schema`의 변형
  값을 직접 만들어 스키마를 고릅니다. `ci.yml`의 `no-std-check` 세 단계는 이제 이 crate를
  `thumbv7em-none-eabi`용으로 빌드하고 그 단위 테스트를 호스트에서 실행하며,
  `tests/test_no_std_gate.py`는 `#![no_std]`를 선언한 crate 루트에서 foundation 목록을 읽습니다.
  no_std에 들어왔는데 게이트에는 들어오지 않은 crate는 조용한 빈틈이 아니라 빨간 판정이 됩니다.
  `no_std` 빌드는 이미 본 스칼라 키를 `HashSet`이 아니라 crate의 순서 있는 노드 맵에 기록합니다.
  `alloc`에는 해시 함수가 없기 때문입니다. `parse_medium`의 할당 상한은 74회로, std 빌드의 67회보다
  매핑마다 한 번씩 많고, `tests/alloc_gate.rs`는 기능별로 상한을 기록합니다.

- **no_std 이야는 실제로 선언된 crate와 맞춰보게 되었습니다** — foundation 목록은 `#![no_std]`를
  적는 crate 루트에서 읽어 나오고, `tests/test_no_std_gate.py`는 이를 두 가지와 추가로 비교합니다.
  `ci.yml`의 `no-std-check` 세 단계와, 현재 문서가 직접 이름 붙이는 crate입니다. `AGENTS.md`에는 늘
  없던 `no_std` foundation 이야가 들어갔고, `docs/dev/boundaries.md`의 그것을 가리키던 문단은 YAML
  엔진이 이 모임에 들어온 뒤에도 다섯 crate를 이름 붙이고 있었습니다. 그러고 말할 수 있었던 게이트는
  주석에 지나지 않았습니다. 날짜가 붙은 기록은 예외입니다. `CHANGELOG.md`와 그 네 개의 미러, 그리고
  `docs/dev/quality-ledger.md`는 역사이며, 역사는 다섯 crate로 이루어진 foundation을 이름 붙여도
  됩니다. 적힌 시점의 사실이었기 때문입니다.

- **crate 매트릭스는 서술이 아니라 측정으로 유지됩니다** —
  `scripts/check_rust_python_decoupling.py`가 manifest에서 매트릭스를 직접 도출해 네 가지
  규칙을 강제합니다. cdylib crate는 정확히 하나(확장 모듈), 다른 어떤 crate의 의존성 폐쇄에도
  Python 런타임 crate가 없고, 확장 crate는 Rust 테스트 계층을 갖지 않으면서
  `[lib] test = false`를 선언하며, 모든 `required-features` 항목은 선언되어 있고 기본값이 아닌
  기능을 가리킵니다. 이 게이트는 엄격한 TOML 리더를 직접 가지고 있어서 `cargo`도 `tomllib`도
  필요하지 않으며 패키지와 같은 Python 3.8 하한에서 동작합니다. cargo가 없는 Python 테스트
  레그에서도 실행되는 이유입니다. `tests/test_rust_python_decoupling_gate.py`가 반증 제어입니다.
  모든 규칙을 합성 트리에서 깨뜨려 판정이 뒤집히도록 요구하고, 리더는 이 저장소의 모든 manifest를
  `tomllib`과 대조하며, 리더가 읽지 못하는 manifest는 통과가 아니라 종료 코드 2를 내야 합니다.
  `crates/pyrs-yaml/Cargo.toml`이 `test = false`를 선언하므로 cargo는 확장 모듈의 테스트 타깃을
  더 이상 만들지 않습니다. `prek.toml`의 훅이 manifest나 crate를 건드리는 변경마다 이 게이트를
  실행합니다.

#### 변경

- **이벤트 스트림의 주석은 어느 노드의 것인지 말해 준다** — `pyrs-event` 의
  `StreamEventType::Comment` 는 플래그 하나(`standalone`)만 들고 있었고, 그 플래그는 두
  질문을 한 번에 답하고 있었다: 그 주석이 혼자 한 줄을 차지했는가(소스의 사실), 그리고 그
  주석이 어느 노드에 매달려 있는가(트리의 사실). 트리가 두었던 자리에 주석을 되돌려 놓고
  싶은 소비자는 두 번째 질문을 할 수가 없었으므로, 플래그는 이제 둘이다. `owns_line` 는
  소스 자신의 대답을 그대로 지니고, 새 `NoteOwner` 열거형이 트리의 대답을 옮긴다:
  `FollowingLeading`, `FollowingInline`, `PrecedingLeading`, `PrecedingInline`. 이름은
  소비자의 구성 순서를 따르므로 `following_*` 은 소비자가 다음에 만드는 노드이고
  `preceding_*` 은 마지막에 만든 노드이다. `StreamEventType::Comment` 를 직접 만드는 Rust
  호출자는 같은 변경에서 함께 갱신되었고, 이 crate들은 공개되지 않으므로 이미 배포된 API는
  움직이지 않는다. `parse_stream` 을 쓰는 Python 호출자는 기존의 `style` 키(`standalone` /
  `inline`)가 그대로 두고 같은 네 배치를 가리키는 `owner` 키를 얻는다. 세 생산자 모두 주석을
  매달아 둔 노드에서 새 질문에 답하므로, 스트림과 트리는 서로 다를 수 없다: YAML 생산자는 노드
  뒤에 따라오는 주석을 그 노드의 인라인 슬롯에, 다음 노드 앞에 오는 주석을 그 노드의 선행 목록에
  매달고, TOML 생산자는 표의 이름 뒤에 그 표의 주석을 보고한다. `[a]` 위의 주석들은 `a` 가 가진
  매핑을 앞세우지 `a` 라는 이름을 앞세우지 않기 때문이다. JSON 생산자는 트리의 자체 규칙이
  주는 쪽과 슬롯으로 주석 묶음을 보고한다. 두 가지 눈에 띄는 변화가 따른다: 쉼표 뒤에 적힌
  TOML 주석은 이제 `style: "inline"` 으로 보고된다. 예전에는 `standalone` 이었는데, 그 주석은
  자기 줄을 차지하지 않았고 `owner` 가 그 자리를 말해 주기 때문이다. 그리고 TOML 표의 주석은
  이제 키 이벤트 뒤에 도착한다. JSON 생산자는 또한 주석의 오프셋을 위해 소스를 다시 읽지
  않는다 - 오프셋이 기록과 함께 이동한다 - 이제 스트림 순회는 텍스트를 두 번째로 훑지
  않는다. 이 분리는 트리와 스트림에게 같은 질문을 던지는 계층들이 증명한다:
  `crates/pyrs-yaml-core/tests/event_notes.rs` 는 열네 문서를 두 번 훑으며 트리가 매달아 둔
  모든 주석이 같은 슬롯에 같은 방향과 같은 자체 줄 사실로 보고되는지 단언하고, JSON과
  TOML의 재생 스위트는 두 필드 중 하나를 건너뛰지 않고 주석 전체를 비교하며, JSONC 문서
  하나는 자기 이벤트에서 다시 접혀 엄격한 기록기를 거친 왕복으로 같은 텍스트로 돌아온다.

- **`rules:` 스키마 언어를 외부에서 판정** — `tests/test_parse_schema_spec_table.py`는 문법이
  받아들이는 각 `type:` 이름에 대해 이 저장소가 소유하지 않은 무언가에게 묻습니다. 규칙이 해석한
  값이 그 이름이 가리키는 JSON 스키마 타입의 인스턴스가 정말 맞는가. 이 귀속은 `jsonschema`가
  판정하고, Python의 `bool`이 `int`의 하위 클래스임에도 `boolean`과 `integer`를 서로 배타로
  다루므로, 타입을 그른 규칙은 합의가 아니라 실패(빨간 실행)가 됩니다. 각 행은 같은 값이 만족해서는
  안 되는 미끼 타입을 곁들여, 확인이 도장 놀음이 되지 않게 합니다. `Schema rule language` 행의
  `external authority` 칸은 `no`에서 `yes`로 바뀌고, `authority-tier:schema-rule-language` 구멍은
  등록에서 빠지며, `.ci/quality-holes.json`은 다시 비어 있습니다. `scripts/quality_matrix.py`는
  오라클 집합에 `jsonschema`를 더해, 인정이 파일 접미사가 아니라 테스트가 실제로 호출하는 리더에서
  나오도록 했습니다.

- **분리 게이트가 bench 레그를 지킴** — `scripts/check_rust_python_decoupling.py`에 규칙 R5를
  추가했습니다. 확장 crate는 `required-features` 없는 `[[bench]]` 대상을 둘 수 없습니다. 그런
  대상은 모든 workspace-급 bench 명령에 선택되어, CI bench 레그가 막 제거한 `pyo3-build-config`
  인터프리터 탐색을 다시 빌드하기 때문입니다. 게이트에 네 규칙이 있었으나, 이 다섯째 규칙이 bench
  레그 범위 지정으로 얻은 것을 고정해, `crates/pyrs-yaml`에 무조건 bench가 앞으로 생기면 결합의
  조용한 재출현이 아니라 실패 실행이 됩니다. `tests/test_rust_python_decoupling_gate.py`가 합성
  트리에서 R5를 깨고 판정 반전을 요구합니다(반증 제어 이제 13개). 그리고 깨끗한 워크스페이스는 계속
  통과합니다.

- **CI 의 Rust 벤치마크 레그도 인터프리터를 요구하지 않음** — CodSpeed 의 `Rust benchmarks` 는
  Python 런타임을 빌드에 끌어들였던 마지막 레그였고, 원인은 테스트 레그와 동일하게 의존 간선이
  아니라 workspace 의 *멤버십* 이었습니다. `cargo codspeed build` 는 모든 멤버를 선택하므로
  `pyrs-yaml` cdylib 와 인터프리터를 탐색하는 `pyo3-build-config` 빌드 스크립트를 컴파일했습니다.
  `yaml_bench`(`pyrs-yaml-core` 안)와 `cli_bench`(`pyrs-yaml-cli` 안) 모두 바인딩 crate 에 의존하지
  않습니다 — `cargo tree -i pyrs-yaml` 은 workspace 멤버를 하나도 지목하지 않습니다. 이 레그는 이제
  테스트 레그가 이미 쓰던 제외를 따라 `cargo codspeed build --workspace --exclude pyrs-yaml` 을
  실행하고, 레그 안에 인터프리터를 탐색할 것이 남아 있지 않으므로 `setup-python` 단계도 없앴습니다.
  이 트리에서 실측: 범위 지정 후에도 `yaml_bench` 와 `cli_bench` 가 선택되고 `pyo3`/`numpy` 컴파일
  수는 0 입니다. `ir_gate` 대상은 영향을 받지 않습니다 — `ir-gate` 로 게이트되고, CodSpeed
  벤치마크가 아니며, `scripts/ir_gate.py` 가 CPython 을 시작하려고 명시적으로 빌드할 때만
  구성됩니다.

- **Rust 벤치마크는 측정 대상 crate 안에 있습니다** — divan 벤치마크 세트가
  `crates/pyrs-yaml/benches/yaml_bench.rs`에서 `crates/pyrs-yaml-core/benches/yaml_bench.rs`로
  옮겨졌습니다. 마흔일곱 개의 벤치마크가 측정하는 코드는 이전과 똑같습니다. 바인딩 crate는
  엔진의 API를 그대로 다시 내보냈을 뿐이므로 모든 import가 이름만 바뀌었고 그 밖에 달라진 것은
  없습니다. 벤치의 package closure는 바인딩 crate에서 물려받은 여섯 개의 Python 런타임
  crate(`pyo3`, `pyo3-ffi`, `pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config`, `numpy`)를
  잃고 하나도 갖지 않습니다. 벤치 바이너리는 이미 Python 런타임이 없었습니다. Linux의
  `DT_NEEDED` 목록에 `libpython`이 없고 Windows의 import 테이블에 `python3.dll`이 없습니다.
  다만 어떤 심볼도 참조하지 않던 의존성을 링커가 잘라냈기 때문이었습니다. 이제는 의존성 자체가
  없어졌으므로 이 독립성은 구조적인 것이며 게이트가 단언할 수 있습니다. 확장 모듈은
  `libpython3.12.so.1.0`에 대한 자체 링크를 그대로 유지합니다. Python 확장이 있을 자리가
  그곳입니다. 측정 대상 코드는 바뀌지 않았으므로 눈에 보이는 차이는 CodSpeed ID의 package
  세그먼트뿐이며 벤치마크 항목은 `pyrs-yaml-core` 아래에서 다시 기준선이 잡힙니다.

- **이벤트 스트림과 merge 해결기는 앵커를 crate의 노드 맵에 둡니다** — `#![no_std]` crate는
  `std::collections::HashMap`을 이름으로 쓸 수 없기 때문에 `event_to_stream_event`와
  `resolve_anchor_name`은 이제 `&mut NodeMap<usize, String>`을 받고, merge 해결기의 앵커 테이블도
  `NodeMap<String, _>`입니다. AST가 이미 쓰던 `FxHasher` 위의 `IndexMap` 그대로이므로 앵커 순서는
  삽입 순서가 되고 키 조회도 실측으로 더 싸집니다. 명령 수 게이트의 YAML 시나리오에서 이 트리는
  `parse_anchors` -2.36%, `parse_inline_merge` -2.32%, `parse_medium` -0.15%, `parse_small` -0.21%
  만큼 움직였습니다. 확정된 하한은 건드리지 않았으므로 게이트가 이 두 시나리오에서 받아들이는
  여유는 빠른 방향으로 낡았고, 집행 이미지에서의 갱신을 기다립니다.

- **Ir 게이트가 cargo의 요약 대신 컴파일러 자체의 진단을 출력합니다** — harness 빌드가 실패하면
  이전에는 `error: could not compile pyrs-yaml (lib) due to 1 previous error`만 표시되고 그
  이상은 없었습니다. 게이트는 `--message-format=json`으로 빌드하므로 rustc의 진단이
  `compiler-message` 레코드로 stdout으로 옮겨가지만, 출력하던 것은 `res.stderr`뿐이었습니다.
  감춰져 있던 것은 게이트가 가장 잘 설명해야 할 실패, 즉 링크 오류
  `rust-lld: unable to find library -lpython3.12`였고, 손으로 cargo를 다시 실행해야만 보였습니다.
  이제 `build_diagnostics()`가 빌드 스트림의 rendered 진단을 모두 이어 붙이고, stdout에 하나도
  없을 때만 stderr 끝부분으로 폴백합니다(툴체인 부재나 잘못된 manifest이며 컴파일 오류가
  아닙니다). `tests/test_ir_gate_diagnostics.py`는 두 스트림과 폴백을 고정하며, stderr만
  반환하도록 되돌린 변이에서 3개 중 2개가 실패했습니다.

- **표준 JSON Schema 표면이 공식 코퍼스로 판정되며 독립된 행을 갖습니다** — `Document.validate()`는
  평가를 Python의 `jsonschema`에 맡기고 AST→인스턴스 변환, schema 수용, `YamlValidateError` 계약만
  이 저장소에 남깁니다. 그래서 매트릭스의 기존 `schema validation` 행은 *다른* 표면, 즉
  `validate_against_schema()`가 평가하는 이 저장소가 만든 규칙 언어를 측정하고 있었습니다. 새
  `JSON Schema validation` 행은 `tests/test_document_validate_suite.py`가 공식
  JSON-Schema-Test-Suite(커밋 `0b4a7fa`, 46개 파일, 1302개 케이스)를 재생하여 `yes`로 기록됩니다.
  `jsonschema` 4.26.0에서 측정: 1261개 케이스가 코퍼스의 결론에 도달하며, `refRemote.json`은 계약
  경계로 파일 전체가 제외되고(리졸버를 노출하지 않으며, 제외를 살아 있게 유지하려고 대표 케이스
  하나만 시도), 10개의 `dynamicRef`/`vocabulary`/`\p{...}` 케이스가 위임 엔진의 경계로 명명됩니다.
  같은 코퍼스를 `jsonschema.validate()`로 직접 재생해도 1302개 케이스 전부 같은 결론이므로 변환에
  귀속할 제외는 없습니다. 제외 집합은 설치된 버전에 따라 결정됩니다. Python 3.8 레그는 4.23.0을
  해석하며 `unevaluatedProperties` 케이스 하나를 더 제외해야 하고(4.25.1에서 수정됨), 양방향으로
  검증합니다. 명명된 케이스가 통과하기 시작해도, 명명되지 않은 케이스가 실패하기 시작해도
  실패합니다. 발명된 언어는 이 코퍼스의 신용을 빌리지 않고 등록된 구멍
  `authority-tier:schema-validation`을 유지하며, 그 종료 조건은 이제 `Document.validate()`로
  대체하면 구멍이 닫힌다고 명시합니다.

- **`rules:` 규칙 언어는 다시 제 행을 갖고, 사각지대도 다시 이름을 갖게 됩니다** — 매트릭스의
  schema 문단은 2026-10-21에 검증기를 은퇴시킨 뒤에도 규칙 언어의 "Rust 진입점은 삭제되었다"고
  적고 있었습니다. 그러나 *언어* 자체는 남아 있습니다. `parse_schema_yaml()`, `RuleResolver`와
  `YamlTypeKind`는 지금도 `pyrs-yaml-core`가 공개하고, `register_schema()`, `load_schema()`,
  `list_schemas()`, `YAML(schema=...)`는 지금도 `rules:` 블록을 그 경유로 읽습니다. 표에는 그
  행이 아예 없었고, 페이지는 표면이 네 줄이라고 주장하면서 실제로는 다섯 개를 인도했으며, 등록부는
  `holes: []`로 적혀 있었습니다. 바깥의 독자가 반박할 수 없는 문법을 오직 저자의 테스트만
  재고 있었습니다. 이번 행은 손으로 쓴 것이 아니라 파생된 것입니다. `scripts/quality_matrix.py`는
  각 행의 단위 계층과 속성 계층을 그 표면을 선언한 파일로 한정합니다. 그래서 이 언어는
  `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs`의 열한 개 `#[test]`와 이 언어를
  지목하는 유일한 속성 `prop_schema_parse_no_panic`만을 기록하고, YAML 엔진 크레이트 전체의
  개수는 아닙니다. 왕복 열과 parity 열은 `n/a`입니다. 이 언어는 스칼라 타입 해석만 하며
  직렬화기를 갖고 있지 않고, 이 문법에 비교할 두 번째 구현도 존재하지 않기 때문입니다. authority
  열은 `no`이며, 그것이 정직한 답입니다. 이 문법은 이 저장소가 만들어 낸 것이므로 단위·속성·fuzz
  세 계층은 모두 엔진이 스스로 믿는 바를 물을 뿐입니다. 측정값은 사각지대
  `authority-tier:schema-rule-language`를 파생하고, 등록부는 검사 가능한 종료 조건과 함께 그것을
  받으며, `QUALITY_MATRIX.md`는 사각지대 표에서 그것을 이름 붙입니다. 그래서 등록부는 다시
  내용을 갖게 되고, 새로 추가된 두 번째 반증 제어는 은퇴한 검증기의 기호가 정말 없고 살아남은
  언어가 정말 존재하는 양쪽을 모두 주장합니다. 페이지가 "삭제됨"으로 흘러가는 일은 이제 없습니다.

- **스칼라 경로에 대한 `sort_keys`는 조용한 성공이 아니라 문서화된 no-op이 되었습니다** —
  `sort_keys("$.a")`는 `None`을 반환하고 아무것도 바꾸지 않았지만 `sort_keys("$.missing")`은
  `YamlEditError`를 발생시켰고, 패키지가 제공하는 문서는 이 사실을 언급하지 않았습니다. 레지스트리
  항목은 이 불일치를 기록했고 종료 조건은 "존재하지 않는 경로와 같은 방식으로 응답하거나, 배포되는
  스텁에 no-op임을 명시"였습니다. 이제 스텁이 이를 명시합니다. 스칼라로 해석되는 경로에는 정렬할
  키가 없으므로 호출은 `None`을 반환하고, 해석되지 않는 경로는 여전히 거부합니다. 이 형태를 고정하던
  테스트는 계약에 맞게 이름이 바뀌어 양쪽을 검증하며, `api:sort-keys-scalar-path-noop`은
  `.ci/known-residuals.json`에서 제거되었고 매트릭스의 Known residuals 표에는
  `api:set-comment-container-owner`만 남습니다.

- **리더가 제한되지 않아도 패리티 열은 테스트 단위를 읽습니다** — 기존 `crosslib_parity`는 제한된
  리더가 없다고 판단한 지점에서 테스트를 하나도 읽지 않고 `yes`를 반환했습니다. 그래서 모든 테스트
  레그에 설치되는 리더만 가진 패밀리는 import 목록만으로 점수를 받았고, 모든 비교에
  `@pytest.mark.skip`을 단 `test_*_crosslib.py`와 테스트가 하나도 없는 파일도 `yes`로
  표시되었습니다. 이제 단위 규칙은 모든 모듈에 적용됩니다. YAML이 `yes`를 유지하는 이유는 가드가
  발동할 수 없기 때문입니다. 모든 비교에 `skipif(pyyaml is None)` 또는
  `skipif(not _ruamel_available())`가 붙고, 두 리더 모두 마커 없이 선언되며, 헬퍼는 그
  본문(`return _ruamel is not None`)으로 해석됩니다. 반면 `skipif(sys.version_info < ...)`는 여전히
  점수를 거부합니다. YAML의 비교를 `safe_load`, `safe_dump`, `parse`에서 읽으려면 방언 정규식이
  접미사 없는 기본 동사를 알아야 했습니다. 제어는 "살아 있는 비교 없음"의 네 가지
  형태(`@pytest.mark.skip`, 테스트 없음, 모듈 수준 `pytestmark`, 인터프리터 게이트)를 `partial`로,
  죽은 가용성 가드를 `yes`로 고정합니다. 측정된 트리는 그대로이며 YAML, JSON, TOML 패리티는 여전히
  `yes`이고, 파생된 구멍 목록도 유일한 `authority-tier:schema-validation` 그대로입니다.

- **코퍼스 스위트는 그것이 지목한 코퍼스로 점수를 받습니다** — 외부 권위 열의 리더 없음
  분기는 리더를 import하지 않는 `tests/test_*_suite.py` 또는 `*_spec_table.py`에 모두 `yes`를
  주었습니다. 그래서 올바른 접미사와 자리표시자 테스트만 있는 파일도 `yes`로 표시되었고, 문서의
  "코퍼스 파일은 재생하는 스위트로 센다"는 서술과 어긋났습니다. 이제 분기는 모듈이 문자열
  상수로 코퍼스를 지목할 것을 요구합니다 — 세 스위트가 로더에 넘기는 `Reference/` 루트입니다 —
  주석 속 언급은 로드가 아닙니다. 제어 항목이 양쪽을 고정합니다. 접미사만 있는 파일과 주석에서만
  코퍼스를 언급한 파일은 `no`, 실제 코퍼스 스위트는 여전히 `yes`이며, 측정된 트리는 그대로입니다 —
  `test_yaml_suite.py`가 이 분기에 도달하는 유일한 파일이고, 일곱 개 권위 파일은 모두 `yes`,
  네 개 형식 행은 `authority=yes`를 유지하며, 파생된 구멍 목록은 단일
  `authority-tier:schema-validation`입니다.

- **외부 권위 열은 import만으로는 점수를 주지 않습니다** — `tests/test_*_suite.py` 또는
  `*_spec_table.py` 파일은 마커가 제한하지 않는 리더를 import하기만 하면, 파일 안의 어떤 코드가
  실제로 그 리더를 사용하는지와 무관하게 `yes`로 집계되었습니다. 셀이 물었던 것은 테스트가 아니라
  의존성이었기에, 살아 있는 것처럼 보이는 import 옆에서 비교가 삭제되어도 열은 초록으로 남았습니다.
  이제 파일이 리더 점수를 얻는 방법은 그 리더에 "손을 뻗는" 것뿐입니다. 테스트 안의 호출, 또는
  테스트가 실행되기 전에 모듈 수준 표에 연결되는 호출 가능 객체 — 라이터 스위트가 쓰는 형태입니다.
  `json is None`은 비교가 아니라 기능 탐침이며, 사용되지 않는 import는 둘 다 아닙니다. 경계가 있는
  분기는 더 엄격한 규칙을 유지합니다. 그곳의 건너뛰기는 특정 레그에서 리더를 제거하는 바로 그
  건너뛰기일 수 있기 때문입니다. 코퍼스 스위트는 여전히 리더가 필요하지 않습니다. 코퍼스에는 부재할
  수 있는 리더가 없기 때문입니다. `tests/test_yaml_suite.py`는 맨 import로 `yes`를 읽던 유일한
  파일이었습니다. `import yaml`은 그 게이트가 쓰는 `HAS_PYYAML` 플래그를 도출하기 위해서만
  존재했고, PyYAML은 `pyrs_yaml.compliance` 안에서 코퍼스를 읽습니다. 따라서 플래그는 이제 그
  모듈에서 오며, 파일은 그것이 실행하는 스위트로 점수를 받습니다. 대조 사례는 네 가지 형태를 모두
  고정합니다. import만 있는 경우와 `json is None` 탐침은 `no`, 모듈 수준 리더 표와 코퍼스 스위트는
  `yes`로 읽습니다. 측정된 트리는 그대로입니다. 일곱 개 권위 파일 모두 `yes`, 네 개 포맷 행은
  여전히 `authority=yes`, 파생된 구멍 목록은 여전히 단일
  `authority-tier:schema-validation`입니다.

- **마지막 대조 구멍이 닫힙니다: JSON5와 JSONC는 모든 레그에서 참조 구현을 가집니다** — 등록된
  항목이 지목한 두 방언의 유일한 외부 리더는
  `pyjson5`였고(`python_version >= '3.9' and python_version < '3.14'`, armv7l/s390x/ppc64le 제외),
  이 패키지가 약속하는 3.8 하한과 3.14+에서는 두 방언 모두 "아무것도 없는" 대상과 비교되면서 레그는
  초록으로 남았습니다. 순수 Python `json5` 패키지(`>= 0.17.3`, `requires-python >= 3.8`,
  `py3-none-any` wheel)를 test 의존성 그룹에 추가하고, 이 방언을 판정하는 두 파일이 무조건
  임포트합니다. 이 방언을 판정하는 비교에는 건너뛰기 marker가 없으며, `pyjson5`는 wheel이 설치되는
  레그에서 두 번째 의견으로 남습니다. 두 참조 구현은 두 가지 구문에서 의견이 갈리므로, 테스트는
  차이를 숨기지 않고 이 엔진의 해석을 고정합니다. 서로게이트 페어는 여기서 한 문자로 결합되지만 순수
  Python 참조는 두 UTF-16 코드 단위를 유지하며, 고립된 서로게이트는 여기서 거부되지만 그 참조는
  받아들입니다. `tests/test_json5_spec_table.py`는 더 이상 모듈 전체를 `pyjson5`로 게이트하지
  않으므로 명세 표의 행은 하한 레그에서도 건너뛰지 않습니다. 측정 결과: `cross_library.restricted`는
  비어 있고, JSON 행의 parity 셀은 `yes`이며, `.ci/quality-holes.json`에는 schema 행만 남았고,
  external-authority 열은 이제 파일 이름이 아니라 파일별 판정을 냅니다. 유계 리더 하나에만
  의존하도록 퇴행한 명세 표는 파일 이름을 가진 `authority-tier` 종류의 구멍으로 나타납니다.

- **TOML 삼자 일치는 3.11 이상에서만이 아니라 모든 레그에서 실행됩니다** — 크로스 라이브러리 모듈은
  3.8부터 3.10까지 pyrs/tomlkit/tomllib의 11개 대조를 건너뛰었습니다. `tomllib`은 3.11에서 들어오고
  그 아래 레그에는 참조 리더가 설치되어 있지 않았기 때문입니다. 백포트 `tomli>=2.0`을
  `python_version < '3.11'`로 test 그룹에 선언하고 `tomllib` 이름으로 import하도록 했으므로 모든
  레그가 24개 대조를 전부 실행합니다. 행은 `tomli`와 `tomllib`를 각자의 경계와 함께 따로 보여 주며,
  `tomlkit`은 어느 레그에서나 방언을 판정하므로 판정은 `yes`로 유지됩니다. 리더 탐색도 배포판 이름의
  끝에서 멈춥니다. `_dependency_marker("tomli")`는 이전에 docs 그룹의 `tomli-w` 요구 사항에 답해,
  어떤 테스트도 import하지 않는 패키지의 marker를 보고했습니다. 새 통제 테스트는 `tomli`를 자신의
  marker에, `tomli-w`를 marker 없음에 고정합니다. 공식 코퍼스의 인코더 절반도 같은 `tomllib` 판정
  으로 같은 레그에서 통째로 건너뛰어졌지만, 이제 해당 레그에서도 실행됩니다. 소수점 이하 초를 한
  자리 로 쓰는 `spec-1.1.0` 세 건은 테스트 자체 변환기에서 소수를 여섯 자리로 맞춥니다. CPython <
  3.11의 `fromisoformat`은 세 자리 또는 여섯 자리만 받기 때문입니다.

- **`partial` 대조 칸은 각주가 아니라 등록된 결함입니다** — 대조 열은 이미 각 `test_*_crosslib.py`의
  oracle import와 `pyproject.toml`의 marker에서 판정을 도출합니다. `orjson`과 `pyjson5`가 모두
  `python_version >= '3.9' and python_version < '3.14'`이고 armv7l/s390x/ppc64le를 제외하므로 JSON
  계열은 `partial`로 읽힙니다. 페이지는 산문으로 설명했지만 대장은 침묵했기에 "JSON 계열에는 크로스
  라이브러리 계층이 있다"는 문장이 이 패키지가 약속하는 3.8 레그에 대해 인용될 수 있었습니다. 그
  레그에서는 모듈이 아무것도 import하지 않고 건너뛰며 43개 대조를 모두 돌린 레그와 같은 초록을
  보고합니다. `parity_gaps()`는 같은 도출에서 `parity-tier:json-jsonc-json5`를 발행하고,
  `.ci/quality-holes.json`은 그것을 없애는 출구(`restricted`가 비어 있음, 같은 측정 방식)와 함께
  기록합니다. 결함 id 슬러그는 레이블의 영문자와 숫자로만 만드므로 `JSON / JSONC / JSON5`가 아무도
  읽지 못할 id가 될 수 없습니다.

- **JSON 리더를 설치할 수 없는 레그에서도 RFC 8259를 판정합니다** — JSON 크로스 라이브러리 모듈은
  게이트를 파일 전체에 걸었습니다. 그래서 이 패키지가 약속하는 3.8 바닥과 3.14+에서는 43개 대조가
  모두 건너뛰어졌고, 서드파티 리더가 필요 없는 비교까지 사라졌습니다. 이제 그중 하나가 표준
  라이브러리 `json` 입니다 — `parse_constant`이 표준 라이브러리가 기본으로 통과시키는 세 개의
  리터럴을 거부하므로, 선언으로 경계가 묶이지 않고 모든 레그에 있는 RFC 8259 기준이 됩니다. 제외한
  레그를 흉내 낸 실측(`orjson`과 `pyjson5`를 모두 import할 수 없는 상태)에서는 29개 대조가 돌고
  44개가 건너뛰며, 전에는 0 / 43이었습니다. 두 리더가 모두 설치된 레그에서는 73개가 실행됩니다. 이런
  사실을 말하려면 칸을 방언별로 만들어야 했고, `crosslib_parity()`는 테스트가 스스로 경계 없는
  리더를 호출하고 그 테스트를 건너뛸 수 없을 때만 그 방언에 실적을 줍니다. 따라서 `@needs_pyjson5`,
  모듈 `pytestmark`, 산문 속 언급, 테스트 뒤에 정의한 helper는 실적으로 잡히지 않습니다. 결함은
  그대로 등록되고 대상은 계열 전체가 아니라 `json5`와 `jsonc`입니다.

- **해석기가 그은 경계도 이제 행이 보고하는 경계입니다** — parity 열은 '어떤 리더가 레그에서 빠질 수
  있는가'를 `pyproject.toml` 에만 물었고, 리더도 모듈 최상위 import 에서만 찾았습니다. `tomllib` 은
  두 쪽 모두에서 사라집니다 — 선언한 파일이 없고, `tests/test_toml_crosslib.py` 에서는 두 번째
  확인을 요구하는 하나의 테스트 안에서 `pytest.importorskip` 이 묶을 뿐입니다. 레그 자신의 건너뜀
  집계가 셀이 말하지 못한 것을 말했습니다 — CI 바닥 레그는 그 모듈의 비교 24 개 중 13 개를 실행하고
  11 개를 건너뛰었고, 3.9 와 3.10 도 같은 11 개를 건너뜁니다 — 즉 행은 하나의 리더만 세었지만 파일은
  둘에게 묻고 있었습니다. `STDLIB_SINCE` 는 `requires-python` 에서 읽은 마루를 기준으로 표준
  라이브러리 리더에 경계를 긋고, `_oracle_locals()` 는 이제 구문에서 `importorskip` 묶음을 취합니다.
  TOML 판정은 `yes` 로, 그리고 정직하게 남습니다 — `tomlkit` 은 제약 없이 선언되어 모든 레그에
  들어가므로, 3.8 부터 3.10 까지 잃는 것은 세 쪽 일치이지 아무도 판정하지 않는 방언이 아닙니다. 그
  모듈은 이제 `tomlkit` 을 그대로 import 합니다 — 어떤 marker 도 묶지 않는 방어는 통과하는 동안 죽은
  코드이고, 통과하지 못하면 조용한 건너뛰기가 되기 때문입니다. 또한 `tomllib` 이 제약 있음으로
  계산되는 순간 그 파일 전체의 `pytestmark` 은 점수를 거부하고 실존하지 않는 구멍을 만들어 냈을
  것입니다. 변형으로 확인했습니다 — `tomllib` 의 도착 버전을 마루와 같게 두면 새 대조 두 개가 빨갛게
  됩니다.

- **계층 표의 수는 그것을 검증하는 실측이 직접 씁니다** — 이 행은 이미 비교 테스트로 프로브에 묶여
  있어 오래된 숫자는 거짓이 아니라 빨간 실행이었습니다. 빠진 것은 알맞은 숫자를 *만드는*
  방법이었습니다. 그 격차로 PR #549의 14개 테스트 레그가 모두 빨개졌습니다. 새 테스트 파일이 Python
  계수를 움직였고, 누가 다시 쳐야 하는지 알아야 했으며, 같은 행을 옮기는 두 open PR은 나중에
  도착하는 쪽의 재 재생성을 강요합니다. `python scripts/quality_matrix.py --write-rows`는 같은
  실측으로 문장을 다시 쓰고 숫자 주위의 산문을 보존하며, 더 이상 해석하지 못하는 표에서는 성공을
  보고하지 않습니다. `--check-rows`는 드리프트를 출력합니다. 이 브랜치에서 시연: `47349338`(#553의
  테스트 포함)으로의 재생성에서 Python 계수가 1,822 → 1,824로 움직였고 명령 하나로 행을 썼습니다.

- **중간 버전 인터프리터가 과금 러너에서 스위트를 다시 실행하지 않습니다** — 플래너는 최하위·최상위
  *그리고* 커버리지 담당 버전을 모든 OS에서 스위트로 돌려, `test (macos-latest, 3.12)`가
  `test (ubuntu-latest, 3.12)`가 이미 임계값과 함께 검증한 내용을 173~285초 동안 다시 확인했습니다.
  macOS는 10배로 과금됩니다. PR #550의 초록 실측(47개 작업, 96.8월클록 분)에서 macOS는 22.3분이지만
  과금 환산 약 300분 중 223분이었습니다. 비싼 OS의 두 칸은 역할이 ubuntu로 정의된 유일한 스위트
  레그였습니다. `scripts/ci_matrix_plan.py`는 이제 `expensive_suite_versions`로 선언 범위에서만
  값비싼 러너의 의무 버전(처음과 마지막)을 찾고, 강등된 칸은 사라지지 않고 wheel smoke로 남으며,
  `check()`는 게이트 역할을 비싼 OS로 옮기는 계획을 거부합니다. 스위트 레그는 14개에서 12개로 줄고,
  주간 스케줄은 여전히 24칸을 스위트로 돌므로 강등이 은폐가 아니게 됩니다.

- **테스트 매트릭스가 설치하지 않는 wheel을 기다리지 않습니다** — `build`는 한 작업에서 두 종류의
  결과물을 컴파일했습니다. 각 레그가 설치하는 abi3 wheel과 컴플라이언스 리포트가 읽는
  `--find-interpreter` 릴리스 레시피입니다. 44개 작업으로 구성된 두 번의 PR 실행에서 실측한 결과, 첫
  실행의 abi3 wheel은 +134초(Windows), +135초(macOS), +151초(ubuntu)에 업로드됐고, 두 번째 실행은
  같은 세 OS에서 +172·+192·+167초였습니다. 그러나 뒤이은 릴리스 빌드가 OS당 45~346초를 더 써서
  `build`는 +485초와 +477초에 끝났고, `test`와 `wheel-smoke`의 24개 레그는 모두 자신이 읽는 유일한
  결과물이 이미 존재한 뒤에도 291~471초를 더 기다린 뒤에 시작했습니다. 릴리스 레시피는 이제
  `release-wheels`라는 독립 작업이고, `tests/test_ci_artifact_wiring.py`가 각 작업 자신의
  `upload-artifact`와 `download-artifact` 입력을 읽어 두 생성자를 분리합니다. 한 작업이 두 계열을
  모두 올리거나, 설치하지 않는 결과물의 생성자를 레그가 기다리면 이름이 보고됩니다. 변경 전
  `ci.yml`로 이 검사를 돌리면 보고되는 것이 바로 그 결합 작업이므로, 거부되는 형태는 실제로 배포된
  형태입니다. 월클록 총시간은 856초와 846초에서 약 500초로 내려갈 것으로 보이며(팬인 작업은 여전히
  가장 느린 릴리스 빌드를 기다립니다), 이 PR 자신의 실행이 실측입니다. runner 분은 100.2와 93.1으로
  그대로이고, 줄어든 것은 일 앞의 대기줄뿐입니다.

- **`pyq`가 `jq`와 `yq` 명령줄을 이어받는다** — 첫 인수가 pyq 하위 명령이 아닌 argv는
  `jq` 1.8 또는 mikefarah `yq` v4로 읽혀(실행 이름이 정하고 `--as jq|yq`로 강제할 수도 있음)
  같은 의미의 하위 명령으로 번역된다. 받아들인 형은 손으로 쓴 기대값이 아니라 실제 도구로
  판정한다. `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs`는 `PATH`에 `jq-1.8.2`와
  `yq v4.54.1`이 있으면 그들을 실행하고, 이 워크스페이스 안의 파일을 참조로 쓰는 것을
  거부한다(처음 작성한 탐색 스크립트는 `PATH`에서 우리 자신의 `yq.exe`를 주어 아무것도
  증명하지 못한 9건의 완전 일치을 보고했다). 그리고 24가지 호출에서 stdout을 바이트 단위로
  비교한다. `.a.b`, `.list[] | select(.x > 1)`, `del(.a.n)`, `.a.n = 9`, `-o json`, `-r`,
  `-N` 등이 포함된다. 같은 문서를 돌려줄 수 없는 것은 이름을 짚어 거부하고 종료 코드 2로
  stdout은 비운다: 마지막 `select` 하나를 뺀 모든 파이프, jq의 모든 함수(`keys`, `length`,
  `map`, `to_entries`), jq 모드의 편집 식(pyq는 YAML, jq는 JSON을 씀), jq의
  `--arg`/`-s`/`-c`/`-S`/`--stream`, yq의
  `-n`/`-e`/`-P`/`--split-exp`/`--front-matter`/`--xml-*`/`--lua-*`. `pyq get`에
  `--no-doc`이 추가됐는데, 탐색이 찾아낸 유일한 동작 차이이며, yq v4는 여러 항목의 스트림을
  `---` 구분자 없이 찍으므로 방언 기본값으로 삼았다. 전체 약속은 `pyq --help`에 적혀 있다.

- **사이트의 모든 성능 수치는 이제 하나의 커밋된 실측에서 나옵니다** — 네 로케일의 성능 페이지와
  기능 페이지의 월클록 표는 손으로 옮겨 적었습니다. 2026-08-17 수치에는 재현 가능한 스크립트도
  기록도 없었고, 같은 호출을 다시 측정하니 모든 절대 판독값이 22~108배 빨라졌습니다.
  `scripts/bench_snapshot.py`는 호출 배치를 계측하고(9개 배치, 각 0.1초 이상, 가장 빠른 배치 보고),
  인터프리터·라이브러리 버전·픽스처 크기와 함께 `.ci/perf-snapshot.json`에 기록합니다. `--docs`가
  12개 표를 모두 다시 쓰고, `tests/test_perf_docs.py`가 모든 셀을 스냅샷과 대조하므로 표가 어긋날 수
  없습니다. 산문은 따로 어긋났습니다: 인덱스, 왕복 가이드, PyYAML 호환, 벤치마크, 비교, 기능
  페이지는 표가 철회한 뒤에도 21–43×, 55–177×, 48–100×, 123–371×, 37×, 85× 를 계속 인용했고,
  그래서 게이트는 속도 배수를 산문으로 제시하는 24개 페이지 모두의 범위를 도출하며 철회된 날짜와
  범위를 이름으로 거부합니다. 현재 값은 PyYAML 대비 파싱 29–34×, 직렬화 223–368×, ruamel.yaml 대비
  68–81×, 541–876×, 왕복 가이드는 45–59× 와 102–125× 입니다. yaml-rs 트레이드오프 문단은 같은
  타이머로 측정한 2~4배 직렬화와 약 2.5배 느린 `YamlDocument` 파싱으로 바뀌었습니다. 2~4배는
  스냅샷이 아니라 교차 라이브러리 벤치마크에서 나오므로 도출하지 않고 이름으로 유지합니다.

- **Rust 벤치마크가 TOML, JSON, 1.1 리졸버를 측정합니다** — CodSpeed의 Rust 레그는 모든 사례가
  YAML 입력에 YAML 또는 Python 출력인 파일 하나만 실행했기에, 이 트리에서 가장 크게 성장한 두
  엔진과 플레인 스칼라의 의미를 결정하는 프로파일에는 월클록 계열이 없었습니다.
  `crates/pyrs-yaml/benches/yaml_bench.rs`에 6개 벤치마크를 추가합니다:
  `from_toml_medium`/`to_toml_medium`, `from_json_medium`/`to_json_medium`,
  `resolve_yaml11_type_many`, `needs_quotes_over_scalars`. 네 개의 포맷 벤치마크는 명령 수
  게이트가 측정하는 `bench_inputs` 문서를 읽고 Ir 시나리오와 같은 이름을 가져, 월클록 변화와
  명령 수 변화를 한 쌍으로 읽을 수 있습니다. 바닥은 움직이지 않습니다: CodSpeed는 순위를 맡고,
  제로 회귀 판정은 `.ci/ir-baseline.json`에 남습니다.

- **MSRV 작업이 테스트한다고 말하는 MSRV를 설치합니다** — `msrv-check`는
  `dtolnay/rust-toolchain@stable`을 설치해 현재 stable에서 워크스페이스를 컴파일하고 그것을 최소
  버전이라고 불렀습니다. 매니페스트의 `rust-version = "1.96"`은 하한을 기록할 뿐 강제하지 않으므로,
  1.97이 필요한 의존성은 이 작업을 통과하고 사용자의 설치에서 실패합니다. 이제 이 단계는 8개
  크레이트가 모두 선언한 `1.96`을 고정하며, `tests/test_msrv_gate.py`가 양방향을 검사합니다——
  워크플로 고정이 매니페스트에서 벗어나거나 매니페스트가 고정에서 벗어나면 실패합니다. 고정을
  바꾸기 전에 하한 자체를 측정했습니다: `cargo +1.96.0 check --workspace --locked`와
  `cargo +1.96.0 test --workspace --locked`가 이 트리에서 모두 통과합니다.

- **매트릭스가 도출할 수 없는 잔여 항목을 기록하는 두 번째 등록부** — `.ci/quality-holes.json`은
  `scripts/quality_matrix.py`가 도출할 수 있는 구멍만 기술할 수 있어, 빈 목록이 "잔여 없음"으로
  읽혔지만 이해하고 의도적으로 남겨 둔 동작은 존재합니다. 표 헤더 줄에 머물 수 없는 TOML 표 헤더
  주석, `set_comment`가 컨테이너의 마지막 자식 노드에 두는 주석, 존재하지 않는 경로는 예외를
  던지지만 스칼라 경로는 `None`을 반환하는 `sort_keys`입니다. 새 `.ci/known-residuals.json`은 각
  항목에 재현 테스트의 노드 id, 담당 계층, 검사 가능한 종료 조건을 등록하고, 고정된 테스트는
  `@pytest.mark.known_residual`을 가지며, `tests/test_known_residuals.py`는 양방향으로
  실패합니다——살아 있는 재현이 없는 등록 항목, 또는 등록 항목이 없는 마커 달린 재현입니다. 하나를
  고치면 특징화 테스트가 빨간색이 되어 같은 변경에서 등록 항목을 제거하도록 강제합니다.

- **aarch64 휠은 네이티브 arm64 러너에서 빌드** — `publish.yml`의 `linux` 및 `musllinux` aarch64
  행을 qemu 기반 `ubuntu-22.04`에서 `ubuntu-24.04-arm`으로 옮기고, glibc 행도 에뮬레이션 대신
  `uv`로 그 자리에서 테스트 스위트를 실행합니다. s390x만 여전히 에뮬레이션으로 스위트를 실행하는
  휠 대상이며, `pyq`의 aarch64 크로스 빌드, armv7, ppc64le는 변경하지 않았습니다.

- **`main`의 병합 이후 작업이 과거에 출력만 하던 주장을 실제로 증명** — trunk가 코드 변경에 대해
  실행하는 유일한 작업은 푸시된 tip의 "동일한 트리가 pull request로서 전체 CI 매트릭스를 이미
  통과했다"를 `echo`로 주장할 뿐이었고, 그것을 측정하는 것도 실패하게 하는 것도 아무것도 없었습니다.
  `scripts/check_main_provenance.py`는 이제 그 문장의 세 가지 질문을 API에 던집니다 — 이 커밋을 만든
  pull request는 무엇인가, 푸시된 트리가 그 pull request의 커밋 트리 중 하나인가, 그것을 담은
  커밋에서 `Test matrix (all legs)`가 `success`를 보고했는가 — 그리고 성립하지 않으면 거부합니다.
  측정한 모든 tip(`07f988fa`, `60f670d0`, `225cf373`)에서 내용에 관한 주장은 성립했으므로, 결함은
  방어 체계의 형태에 있었습니다. 그런데 이 문지기 자체가 trunk에서 처음 실행될 때 잘못된 이유로
  빨개졌고, 그 수정도 이번 변경에 함께 들어갑니다 — 하나의 커밋은 그 검사 이름의 기록을 여러 개
  가질 수 있고, API는 완료 순서로 나열하지 않으며, 이 workflow의 판단 작업은 `pull_request`에서만
  실행되므로 `workflow_dispatch`는 진짜 판단 옆에 `skipped`를 남깁니다. 마지막 행을 답으로 읽은
  것은 그 부재를 거절으로 다루는 것이었습니다. `newest_verdict`은 아무것도 판단하지 않은 기록을
  버리고 나머지를 작업 종료 시각으로 정렬하므로, `fddcc76e`는 통과하고 일곱 조상은 실제로
  존재하는 판단 때문에 계속 거부됩니다.

- **형식별 방어 표를 파생 측정으로 전환, 두 칸을 실측으로 정정** — `scripts/quality_matrix.py`가
  방어를 선언하는 파일(fuzz 대상, `proptest!` 블록, 각 `test_*_crosslib.py`, `pyproject.toml`
  의존 마커)에서 `QUALITY_MATRIX.md`의 형식 표를 계산하고 `tests/test_quality_matrix.py`가 각
  칸을 고정합니다. 대조 결과 두 칸의 오차가 측정됐습니다: `schema validation`은 과소
  보고였습니다(`validate_schema.rs`는 validate 전에 parse 하므로 `fuzz parse`는 `yes`), 그리고
  property 계층은 25가 아니라 27입니다(기존 계산은 `prop_` 접두어를 요구해 산문 이름으로 선언된
  `pyrs-event`의 속성 두 개가 빠져 있었습니다). parity 칸의 `partial`은 이제 인상이 아니라
  oracle에 선언된 인터프리터·아키텍처 제약을 뜻합니다.

- **JSON5 `\xHH` escapes를 해석합니다 — 나머지 JSON5 escapes는 이미 있었습니다** —
  `load_json5`와 `from_json5`는 `\v`, `\0`, 줄 연속은 받아들였지만, JSON5 escape 집합에
  속하는 `\x` + 16진수 두 자리는 거부했습니다. 동봉된 stub은 이미 `load_json5`에 대해
  "Accepts the full JSON5 grammar"라고 이미 공언한 상태였습니다. 참조 구현과 대조한 실측에서
  pyjson5는 `{a: '\x41'}`에 `{'a': 'A'}`를 주는데 pyrs는 "invalid escape sequence"를
  반환했습니다. 두 자리는 1바이트씩 누적하는 방식으로 바꿨습니다 — `from_str_radix`는 앞의
  `+`를 통과시켜 `\x+1`이 유효한 escape가 되어버립니다 — 엄격한 JSON과 JSONC는 계속
  거부합니다. Rust 테스트는 두 종류의 인용부호, `\x0A`와 `\x00`, 잘리거나 16진수가 아닌
  뒷부분, 멀티바이트 문자를 넘는 잘못된 escape를 고정하고, pyjson5 대조 테스트는 수락과
  더 좁은 방언 두 개의 거부를 고정합니다.

- **JSON5 escape 키와 모든 줄 끝 표시, 그것을 고정하는 표** — 같은 stub이 약속해 놓고 빠져 있던
  문법 두 개 — 따옴표 없는 키 안의 역슬래시 + u escape(JSON5가 상속한 ES5의 `IdentifierName`은
  식별자 문자가 올 자리면 어디든 허용합니다), 그리고 문자열 안에서 역슬래시 뒤에 오는 CRLF·LS·PS
  입니다. 연속 입력의 빈틈은 같은 방언의 두 스캐너가 서로 다르다는 사실도 드러냈습니다 — CRLF
  모양은 큰따옴표에서는 통과되고 작은따옴표에서는 실패했습니다. escape 문자는 이제 그것을 나타내는
  코드 포인트로 바로 채택하고 `ID_Start`/`ID_Continue`로 다시 검사하지 않습니다 — 그래서 escape한
  숫자나 점은 키가 될 수 있지만 그 문자 자체는 키가 될 수 없습니다. 두 인용부호가 하나의 줄 끝
  규칙을 함께 쓰게 되었습니다. 이 두 지점을 찾아낸 대조는 게이트로도 남습니다 —
  `tests/test_json5_spec_table.py`는 수락 41·거부 11의 문법을 `pyjson5`와 비교합니다. reader가 한
  번도 허용하지 않은 표기는 속성 계층과 fuzz 계층에 보이지 않기 때문인데, 두 오라클은 이미 존재하는
  경로만 비교할 수 있습니다.

- **플로우 매핑은 블록 스타일 키를 적을 수 없고, 생성기도 더 이상 만들지 않는다** — 2만 케이스
  property 단계가 `{? [null]: A, ? [null]: B}` 에서 무작위로 빨개졌다. 같은 한 요소 순서가 두 키가
  되어 하나는 블록, 하나는 플로우로 적힌 형태다. 블록 부모에서는 이 둘이 실제로 두 개의 키이고,
  `? - null` 과 `? [null]` 은 다시 읽어도 두 항목으로 남는다(실측). 그러나 `{...}` 안에서는 블록
  컨테이너에 합법적인 형태가 전혀 없어서 작성기가 둘 다 플로우 토큰을 내므로, 읽는 쪽이 중복 키를
  거부하는 것이 맞다. 그 트리는 문서가 아니라 생성기의 것이었다: `arb_mapping` 은 문맥을 보지 않는
  규칙으로 키를 걸러냈다. `keys_collide_in_flow` 은 비교 전에 두 키를 플로우 부모가 허용하는 유일한
  표기로 접는다. 접는 대상은 사본이므로 모델은 `KeyIdentity` 하나 그대로다. 일치 검증은 같은 표를
  플로우 부모에서도 다시 물으면서, 생성기가 책임져야 하는 방향만 지킨다: 규칙이 통과시킨 키 쌍은
  읽는 쪽에서도 남아야 한다.

- **방어 매트릭스가 외부 권위를 측정합니다** — `scripts/quality_matrix.py`가 여섯 번째 열
  `external authority`를 파생합니다. 그 방언에 `tests/test_*_suite.py`나
  `tests/test_*_spec_table.py`가 있으면 `yes`입니다. 지금까지 형식별 표는 unit·property·fuzz·
  parity 네 겹만 세어서, 이 저장소에서 가장 강한 `test_yaml_suite.py`, `test_json_test_suite.py`,
  `test_toml_test_suite.py`, `test_json5_spec_table.py` 문지기들이 방어를 나열한 페이지에서
  완전히 빠져 있었습니다. 세고 있던 parity는 구현 둘을 비교할 뿐입니다. `schema validation`은
  `no`입니다 — 외부 권위가 그 규칙을 묻지 않으니까요. `tests/test_quality_matrix.py`는 이 열을
  칸마다 단언하고 줄마다 파일 하나를 지목하므로, 파생에서 접미사가 빠지면 조용히 열이 사라지는
  대신 테스트가 빨개집니다.

- **Unicode 공백으로만 된 JSON 문자열이 값을 지켰다** — 키가 U+2028 하나인 문서에 `load_json5`는
  `{None: 1}`으로 답했다. `load_json`과 `load_jsonc`는 문자열을 돌려주고 `pyjson5`도 같은 편이다.
  허브는 디코딩한 JSON 문자열을 YAML 스칼라로 두며 `needs_quotes`가 말하기 전에는 평문으로 쓴다.
  그런데 JSON 해석기는 평문 스칼라를 Unicode 규칙으로 잘라 낸다 — `resolve_json_type`은
  `str::trim`을 의도적으로 둔다. JSON5의 구조적 공백이 실제로 그만큼 넓기 때문이다 — 그래서 ASCII가
  아닌 공백으로 시작하거나 끝나는 텍스트는 그 글자를 잃고, 그것으로만 이루어진 텍스트는 null이 됐다.
  따옴표 규칙은 ASCII 공백과 NBSP는 알고 있었지만, 이제는 Unicode `White_Space` 25개 전체의 양쪽
  끝을 안다. 두 표가 서로 어긋나던 지점이 여기서 사라진다. 이 판단은 바이트 훑기가 비 ASCII 바이트가
  있는 텍스트에서 반드시 도달하는 문자 쪽에 놓였으니, 순수 ASCII 텍스트는 아무것도 더 치르지 않는다.
  `tests/test_json_whitespace_scalar.py`는 이 집합을 키와 값 양쪽 위치에서 세 방언으로 돌며
  `pyjson5`와 `json`에게 "이것은 null이 아니라 문자열"임을 확인시킨다.

- **JSON5 문자열은 원시 제어 바이트를 내용으로 둔다** — 큰따옴표 스캐너는 0x20 미만 바이트에서
  멈추고 거부했지만, 그것은 엄격한 JSON 을 위한 RFC 8259 §7 의 규칙이지 JSON5 의 규칙이 아니다. 이
  방언은 따옴표 안에서 줄 종결자만 금지한다. 그래서 실제 탭을 담은 문서를 `load_json5` 가 거부하는
  동안 `pyjson5` 는 읽어 냈고, 이 파일의 작은따옴표 문자열은 같은 바이트를 예전부터 그대로 옮겨
  적었다 — 한 방언이 두 답을 냈다, 어떤 따옴표로 열었느냐에 갈린 답으로. 이제 멈춤 집합은 방언을
  따른다: JSON5 는 CR 과 LF 를 제외한 모든 C0 바이트를 받고, 엄격한 JSON 과 JSONC 는 32 개 전부를
  계속 거부하며, JSON5 씁니다는 읽은 것을 이스케이프하니 왕복도 한 줄이다.
  `tests/test_json5_spec_table.py` 는 30 개를 두 따옴표 모두로 참조 구현에 묻고, 두 엄격 로더는 같은
  바이트로 RFC 에 묶인다.

- **JSON 내보내는 쪽에 이 저장소가 소유하지 않은 읽는 도구를 주었다** — 읽는 쪽은 우리 해석기로만
  검증해 왔고, 고정점은 같은 잘못을 두 번 저지른 구현도 기꺼이 만족시킨다.
  `tests/test_json_writer_suite.py` 는 이제 1500 그루의 트리를 만들어 `to_json`, `to_jsonc`,
  `to_json5` 로 내보낸 뒤, 그 문서를 Python 의 `json` 과 `pyjson5` — 인코더를 본 적 없는 남의
  해석기 — 에게 넘기고, 읽은 값이 넣은 트리와 같기를 요구한다. 못 박기 전 실측은 세 경로 모두
  거부 0, 값 바뀜 0. 그리고 실제로 문다: `write_json_string` 이 원시 탭을 내뱉게 하면 두 엄격한
  다리가 붉어지고 JSON5 다리는 초록으로 남는다.

- **1.1 이 숫자로 적는 모양의 YAML 문자열이 읽으면 숫자가 됐다** —
  `safe_dump({"v": "1_000"})` 은 `v: 1_000` 을 뱉었고 PyYAML 과 ruamel 은 둘 다 정수 1000 을
  돌려준다. `8_0`, `0b101`, `+0x1f`, 옛 불리언 낱말 `yes`/`on`/`off`/`y`, 날짜만 있는 시각
  `1990-06-23` (읽으면 `datetime.date`) 도 같다. 경로마다 26 개 모양이 새고 키 자리와 값 자리
  모두, Python 경로와 JSON→YAML 경로 모두 그랬다. 따옴표를 칠지 가드 는 `resolve_core_type`
  만 물었다. 이 엔진 자신의 1.2 core 읽는 도구에서는 그 모양이 전부 문자열이다. YAML 1.1 타입
  저장소를 따르는 읽는 도구는 타입을 붙이고, 이 엔진 자신의 `schema="1.1"` 프로파일도 `y` 와
  `yes` 에게 붙인다 — 내보내는 쪽과 자기 프로파일 하나가 같은 토큰에서 서로 다른 답을 냈다.
  가드는 이제 1.1 표를 둘째 질문으로 물어서 네 family 를 본다: core 가 적지 않은 불리언 낱말,
  숫자 속에 `_`, core 가 못 보는 진수 접두 (`0b`, 그리고 부호 붙은 `0x`/`0o`), `YYYY-MM-DD`.
  따옴표는 안전한 쪽으로 틀리는 것이다 — 인용한 스칼라는 해석되지 않는다 — 그래서 각각은 해석기
  대신 표가 받는 모양을 쓴다: `a_b`, `0x`, `2026-10` 은 평문으로 남고, `12:00` 은 여기서 필요
  없다. 콜론이 한 아래 층에서 평문 출력을 거절하기 때문이다. 바뀐 출력은 이 모양들뿐이다:
  `1_000` 은 이제 `"1_000"` 으로 쓴다.

- **YAML 내보내는 쪽에 이 저장소가 소유하지 않은 읽는 도구를 주었다** —
  `tests/test_yaml_writer_suite.py` 는 `safe_dump`, `from_json`, 편집 계층으로 문서를 내보낸 뒤
  그것을 PyYAML 과 ruamel.yaml 에게 넘기고, 읽은 값이 넣은 값이기를 요구한다. 경로마다 만든 트리
  400 그루, 옛 낱말은 키 자리와 값 자리 양쪽에서 묻는다. 고친 뒤 실측은 세 경로 모두 거부 0, 값
  바뀜 0 이고, `QUALITY_MATRIX.md` 는 공식 스위트 읽는 관문 곁에 이 칸을 YAML 행에 적는다. 편집
  계층은 데이터 경로보다 좁은 규칙을 지키고, 이 파일은 양쪽을 모두 못 박는다: 경로 조각은 읽는
  도구가 해석하는 키를 *가리킨다* (issue #336). 그래서 `$.~` 와 `$.42` 는 저마다의 평문 접힘을
  지키고, 두 프로파일 사이에서 타입을 잃는 이름은 — `rename("$.v", "yes")` 는 `true` 로 읽히는
  키를 썼다 — 인용한다. 날짜와 시각은 바깥에 둔다: `!date` 는 SafeLoader 에게 생성자가 없는
  지역 태그라서 남의 읽는 도구는 값을 바꾸는 대신 그 문서 자체를 거절한다. 태그 설계의 경계다.

- **TOML 내보내는 쪽에 이 저장소가 소유하지 않은 해석기를 주었다** — 쓰는 쪽 세 칸 가운데 마지막
  칸이다. TOML 의 바깥 층(`test_toml_test_suite.py`, `test_toml_crosslib.py`) 은 둘 다 읽는 쪽만
  물었기 때문에 `to_toml` 은 우리 해석기로만 검증돼 있었다. `tests/test_toml_writer_suite.py` 는
  `to_toml` 과 `YamlDocument.to_toml()` 로 만든 트리 400 그루를 내보내고 `tomlkit` 에게,
  Python 3.11+ 에서는 `tomllib` 에게도 넘긴다. 실측: 1200 그루 가운데 1074 그루가 나가서 읽혔고
  거부 0, 값 바뀜 0 이었다. 나머지 126 그루는 하나의 유형화된 거부 `toml-cannot-represent-null`
  에서 멈췄다 — 그 형식에 적을 말이 없는 값은 표기를 지어내는 대신 거부된다. 경계 두 개는 못
  박았다: `datetime.time` 은 맨 시각(1.1 의 표기) 으로, i64 를 넘는 정수는 인용표를 입고(허브는
  문자열로 부른다) 나간다. 동작은 바꾸지 않았다.

- **완전 형식 `!<tag:yaml.org,2002:X>`도 `!!X`로 등록된 플러그인에 닿습니다** — YAML 1.2 §6.8은 두
  표기를 같은 태그로 봅니다. 직접 나열한 다섯 태그(`!!str`, `!!int`, `!!float`, `!!bool`,
  `!!binary`)는 둘 다 봤지만 나머지는 렌더링된 문자열로찾지 않으므로 완전 형식은 스키마 해석으로
  떨어졌습니다. 2026-10-18 trunk 실측: `!<tag:yaml.org,2002:widget> 7`은 `!!widget` 등록을 무시하고
  정수 `7`을 읽었습니다. 이제 문서의 표기를 먼저 찾고, 어긋났을 때만 접두 판단을 한 번 더 하므로,
  YAML 밖의 이름공간 URI는 지금까지처럼 태그 없는 값으로 읽히고 오류를 내지 않습니다.

- **대조 열이 다른 이름으로 선언된 오라클을 알아춥니다** — `scripts/quality_matrix.py`는
  `pyproject.toml`이 각 라이브러리 오라클에 붙인 경계를 읽지만 import 이름을 글자 그대로 찾았습니다.
  `ruamel.yaml`은 그 이름으로 import되고 `ruamel-yaml`로 선언되므로 조회는 `undeclared`를 답했고,
  모든 레그에 설치되는 오라클을 프로젝트가 선언하지 않은 것처럼 보고했습니다. 이제 조회는 PEP
  503처럼 `-`, `_`, `.`과 대소문자를 정규화하며, `tests/test_quality_matrix.py`는 `undeclared`를
  거부합니다. 탐지기가 오라클을 놓치면 잘못 보고되는 대신 실패합니다.

- **`extends`를 생략한 스키마가 문서에 적힌 기본값대로 해석됩니다** — `extends`는 선택이고
  `docs/ko/guides/custom-schema.md`는 기본값을 `core`로 적지만, 파서 코어를 고른 건 키가 빈 값으로
  존재할 때뿐이었습니다. 키를 빠뜨린 스키마는 기반 해석기가 없어 모든 스칼라가 자기 텍스트로
  처리되므로 판정이 소리 없이 두 방향으로 뒤집혔습니다. `type: int`는 `port: 80`을 거부하고 (오류
  문장에 `expected int but got Str("80")`) `type: str`은 이를 받아들였습니다. 옛 기본값을 고정하던
  Rust 행은 두 행이 되었고, 둘째 행은 남아 있는 경계를 기록합니다. 이 엔진이 찾지 못하는 이름은
  여전히 대체 해석기 없이 떨어지며, 잘못 쓴 이름을 거부할지는 공개된 오류 범위의 다른 판단입니다.

- **앵커를 고아로 만드는 편집은 쓰이지 않고 거부됩니다** — `base: &x 1`에 `use: *x`가 함께 있는
  문서에서 그 앵커 이름을 바꾸거나, 메타데이터 없이 그 마디를 대체하거나, 정의를 다른 곳으로 옮기면
  모두 성공을 반환한 뒤, 남아 있는 참조가 아무도 정의하지 않는 앵커를 가리키는 문서를 내보냈습니다.
  이 라이브러리의 판독기가 `found unknown anchor`로 거부합니다. 규칙은 이미 엔진에 있었습니다 —
  메타데이터를 보존하는 값 대체에서 `with_metadata_from`이 문서 쪽 앵커 이름을 지킵니다 — 세 문이
  그것을 우회했으니, 검사가 이름이 실제로 떨어지는 자리, 즉 `set_path`(`set_many_path`와
  `move_path`가 함께 쓰므로), `delete_path`, 앵커 동사에 놓였습니다. 검사는 실제로 참조가 끊기는
  경우보다 넓게 걸립니다: 별명은 같은 이름의 앵커 중 *가장 앞의* 것과 묶이므로, 뒤의 `&x`가 살려둘
  이름 바꾸기도, 앵커와 그것을 가리키는 별명을 함께 담는 부분 트리의 삭제도 거부됩니다. 과도한
  거부는 한 번 더 호출하면 끝나지만, 부족한 거부는 아무도 읽지 못하는 파일을 씁니다. 이름 바꾸기가
  별명을 데려가는 일은 동사의 의미를 정하는 문제이며 그대로 과제에 남겼습니다.

- **편집 계층 전용 퍼즈 대상이 생겼습니다** — 워크스페이스는 이제 아홉 개이고 앞의 여덟 개는
  *문서*만 엔진에 넘겼으며 *변경*을 몰아치는 대상이 없었습니다. 왕복 보증의 본론인 계층입니다.
  `fuzz_targets/edit_yaml.rs`는 `document NUL script`를 읽고 판독기가 실제로 도달한 경로를 고른 뒤,
  허용된 각 편집에 대해 재파싱과 값을 지정한 자리에 남음을 요구합니다. 삭제에 관한 검증은 종류를
  나눠 말합니다 — 키는 동일성이지만 배열 첨자는 다음 항목이 올라오는 자리이기 때문입니다 — 위 앵커
  결함은 첫 종자 재생에서, 첫 탐색 몇 분 뒤에 다시 검출되었고, 세 형태는 단언 대신 기록만 했습니다:
  스칼라에 건 정렬 요구는 “ok”를 읽고 아무것도 바꾸지 않고, 주석이 지정한 마디에 남으란 보장이
  없으며, 태그와 주석을 함께 담은 flow 항목은 두 바퀴 만에 안정됩니다. 마지막은 아직 열린 결함이며,
  다음 개수가 입력을 그대로 물려받게 종자로 저장했습니다.

- **`date`와 `datetime`이 모든 리더가 아는 태그로 쓰여 나갑니다** — `safe_dump()`는 이전에
  `!date`와 `!timestamp`를 썼고, 로컬 태그는 이 라이브러리 밖에 생성자가 없습니다. 2026-10-18
  trunk에서 실측한 결과 `yaml.safe_load`와 `ruamel.yaml` 모두 `ConstructorError`를
  반환했습니다. 값의 타입이 바뀐 것이 아니라 아예 회수되지 않았습니다. 이제 두 객체는 YAML이
  정의한 유일한 타임스탬프 태그 `!!timestamp`로 나가며, 두 리더는 날짜만 적힌 텍스트를 `date`로,
  시각이 포함된 것을 `datetime`으로 해석합니다. 이 라이브러리도 같은 기준으로 나눕니다. `!date`와
  `!timestamp`는 계속 등록되어 이전 버전이 쓴 문서가 같은 객체로 읽히고, 로컬 태그로 들어온 문서는
  그대로 다시 쓰입니다 — 표준 태그는 Python 객체에서 새로 만든 값에만 붙습니다. `!time`은 로컬
  확장으로 남습니다. YAML은 날짜 없는 시각의 태그를 정의하지 않습니다. TOML에서 읽은 값은 여전히
  허브 태그 `!date`/`!timestamp`를 쓰므로, 그 부분은 아래 항목에서 Rust 쪽도 같은
  바꾸기를 합니다.

- **TOML의 날짜와 날짜·시각도 같은 표준 태그로 허브에 들어갑니다** — `crates/pyrs-toml/src/lib.rs`의
  투영은 날짜만 있는 값에 `!date`, 날짜와 시각이 함께 있는 값에 `!timestamp`을 붙였기 때문에, TOML을
  읽어 YAML로 쓰는 것처럼 지나가기만 한 문서는 위 항목이 Python 경계에서 그만둔 로컬 표기로
  나갔습니다. 이제 둘 다 `!!timestamp`를 쓰고, 읽는 쪽은 텍스트로 갈라하므로 TOML 날짜는 자정
  `datetime`이 아니라 `date`로 남습니다. `parse_stream`의 `tag`에서 확인할 수 있습니다. 시각만 있는
  값은 `!time`을, `!date`는 그대로 읽힙니다.

- **값이 태그만 있고 본문이 없는 flow 요소는 주석을 첫 라운드에 고정한다** — `{a: !t }`는 값을
  아무것도 없는 형태로 써내므로 태그 뒤에 쓰인 주석이 매달릴 본문이 없다. 리더는 그 주석을 바깥쪽
  쌍에 넘기고 태그를 그 뒤로 옮기며, 문서는 두 번째 출력에서야 안정된다. 왕복에서 사라지는 것이
  없었기에 다른 어느 계층도 이 결함을 보지 못했다. 기록기는 주석이 뒤따를 때 그 빈 본문을 `''`로
  쓴다 — 주석 앞의 빈 값을 `~`로 쓰는 것과 같은 판단이다 (libFuzzer `edit_yaml` crash-b23eed05).

- **블록 순서 요소의 주석은 대시 줄에 남는다** — 편집 계층이 요소 맵에 심은 주석은 `-` 줄에 자리가
  없으므로 기록기는 맵 전체를 별도의 줄로 밀어냈다. 독자는 그 주석을 블록을 끝내는 쌍에 넘기고 다음
  출력이 줄을 다시 줄였으므로, 텍스트는 두 라운드에 걸쳐 안정됐다. 줄인 형태는 요소 자신의 주석을
  싣는다（`- a: 1  # n`）. 그 줄이 YAML 의 유일한 줄 끝 슬롯을 이미 썼다면 주석은 대시 위로 간다
  （`# n`, 이어 `- a: 1  # m`）— 두 표기 모두 고정점이다（libFuzzer `edit_yaml` crash-61474da6）.

- **빈 plain 스칼라는 실제로 해석되는 null 표기로 쓴다** — 텍스트도 속성도 없는 스칼라가 아무것도
  쓰지 않으면서 시작한다. 빈 줄에서 독자는 같은 노드를 돌려주지 않는다: `k: `와 `-` 둘 다 null로
  다시 읽히고 그 표기는 `~`이므로 첫 출력은 두 번째 라운드에 가서야 안정됐다. 계기는 `- >`의
  스타일을 plain으로 바꾼 편집이다（libFuzzer `edit_yaml` crash-63749496）. 기록기는 이제 본문이 빈
  모든 자리 — 중첩 값, 루트 값, 순서 요소, flow 쌍 값, flow 요소 — 에 null 표기를 쓴다. 태그가 달린
  빈 값은 그대로 둔다: 태그가 해석을 억제하므로 안정되는 `''` 표기（#514）를 유지한다. 앵커는 같은
  경우가 아니다: 노드에 이름을 붙일 뿐 해석을 억제하지 않으므로 `- &a3 `는 이제 `- &a3 ~`로
  쓴다(crash-07b6a10e).

- **TOML 기록기의 방언 약속에 속성 검증 층을 더했습니다** — 1.1이 주는 눈에 보이는 자유는 인라인 표
  안의 줄바꿈, 곧 주석을 둘 수 있다는 것인데, 주석이 없는 트리가 1.0으로도 읽히는 텍스트를
  받았는지를 묻는 검사가 없었습니다.
  `prop_toml_undecorated_output_loads_as_1_0`(`crates/pyrs-yaml-core/src/fmt_pbt.rs`)은 어디에도
  note가 없는 생성 트리가 `from_toml_v1_0`으로 다시 읽혀야 한다고 요구합니다.
  `an_undecorated_inline_table_writes_compact_and_loads_as_1_0`과
  `the_1_0_reader_refuses_the_1_1_inline_table_freedom`(`crates/pyrs-toml/src/writer.rs`)은 해당
  갈래와 그 판단 근거를 형태로 고정합니다. 실측: 여러 줄 갈래를 강제로 타면 처음에는 이름 붙은
  검증만 붉어지고 20 000개 생성 사례는 초록으로 남았습니다. 그래서 인라인 표를 반드시 포함하는
  입력을 만드는 전략을 더했고, 같은 강제로는 이제 0.01초 안에 속성 검증도 붉어집니다. 세 번째 속성이
  이 층에 없던 동일성 주장을 그대로 밝힙니다. `prop_toml_roundtrip_preserves_the_tree`는 한 번 왕복
  전후의 트리를 값과 모양, 순서까지 견줍니다. 값을 빼는 돌연변이 두 가지가 둘 다 붉게 만들었고 (같은
  두 번의 실행에서 텍스트 고정점도 붉어졌으니, 이는 더 강한 검증자가 아니라 주장을 그대로 셈긴
  것입니다). 동작은 그대로이고, 수는 Rust 검증 720개와 속성 29개로 움직입니다.

- **편집 티어가 출력도 심사한다** — 허브 문서는 바로 `doc.to_toml()`, `doc.to_json5()`, CLI의
  TOML/JSON 출력이 쓰는 것이므로「편집한 뒤 내보내기」는 사용자가 실제로 지나는 길입니다. 그런데
  이전의 모든 티어는 이 기록기들에게 *판독기가* 만든 트리만 건넸습니다. `toml_roundtrip`과
  `json_roundtrip`은 스플라이스 계층이 다시 쓴 트리를 본 적이 없습니다. `edit_yaml.rs`는 이제 안정된
  트리를 모든 방언(들린 형태도 포함)에 각각 넘기고, 그 방언 자신의 판독기가 텍스트를 받아들이는지,
  두 번째 출력이 같은 바이트인지 요구합니다. 어떤 방언이 트리를 거부하는 것은 통과입니다. TOML에는
  최상위 스칼라가 없고 엄격한 JSON은 비유한 부동소수를 거부합니다. 편집 동사는 설계상 YAML
  전용이며(`crates/pyrs-toml/src/lib.rs`에 그렇게 적혀 있다), 따라서 한 티어가 일곱 기록기에
  질문하는 형태입니다. 같은 두 질문이 `json_roundtrip`에도 들어갔습니다. 세 들린 JSON 표기가
  그것까지 포함되며, 이 티어는 아무도 생성을 골라 작성하지 않은 텍스트에게는 그 물을 아직 물어 보지
  않았습니다.

- **용기를 치환하는 `set`이 그 안에서 참조되던 앵커의 정의를 잃지 않는다** — `set_path`의 가드는
  치환되는 노드*자신*의 앵커만 물었는데, 그 노드엔 앵커가 없고 자식에 있었다.
  `preserve_metadata: true`는 정확히 그 자신의 속성만 옮겨 심기에, 맵을 덮어쓰면 `x`의 유일한 정의를
  지우고 `use: *x`는 아무도 정의하지 않는 이름을 가리켜 이 라이브러리 자신의 판독기가
  거부했다(libFuzzer `edit_yaml` crash-fbd364b3). `delete_path`가 이미 `anchors_in`으로 더 넓은
  부분트리의 답을 갖고 있었으니, 이 문에서도 같은 질문을 같은 `anchor-is-referenced` 사유로 거절하고
  트리는 그대로 둔다.

- **태그만 달린 순서 요소에 쓴 주석은 그 줄에 남는다** — `- !fuzz2`는 값을 빈으로 표기하므로 태그
  뒤의 주석은 얹을 본문이 없다. 판독기는 이를 요소의 선행 주석으로 접수하고 두 번째 출력에서는 대시
  위 줄로 옮긴다. 아무것도 잃지 않았는데 텍스트가 두 번째 라운드에 움직였다(libFuzzer `edit_yaml`
  crash-3dd60cad). 기록기는 이제 행 내 주석 앞에 대기 중인 태그 줄을 닫는다 — 주석 줄 앞에 이미 해
  왔던 것과 같은 일이다. 요소는 `- !fuzz2 ~  # fuzz 47`로 쓰인다.

- **부호 있는 영점은 TOML이 다시 읽는 영점으로 TOML에 전달됩니다** — 정수 그대로 두기는 plain 스칼라
  자신의 표기를 지키고, `-0`은 올바른 TOML 텍스트입니다. 그러나 TOML에 음의 영점은 없어서
  `ist = -0`은 `0`으로 다시 읽히고 두 번째 출력이 움직였습니다. 고정점은 읽는 쪽의 표기로 쓰는
  것이므로 부호 있는 영점은 `0`으로 쓰고, 읽기가 실제로 보존하는 진법 표기(`0xDEAD`, `0b1`)는 그대로
  통과합니다(libFuzzer `edit_yaml` crash-917e103b).

- **비워진 블록 순서가 대시 줄을 지킵니다** — 블록 순서의 마지막 요소를 지우면 flow 깃발이 남지만,
  빈 용기에는 블록 형식이 없어 언제나 키의 줄에서 `[]`로 적힙니다. 즉 컴팩트 규칙이 출력이 갖지 않는
  컴팩트성을 거부했던 것입니다. 첫 출력은 `-\n  t: []`이었고 읽기는 `- t: []`로 답해 두 번째
  라운드에 텍스트가 움직였습니다 (libFuzzer `edit_yaml` crash-a58caa38).

- **긴 정수 모양의 JSON 문자열은 허브를 지나도 따옴표를 지킵니다** — `i64`에 들어가지 않는 숫자
  연속에 대해 Core 해석기는 `Str`이라 답하지만, JSON은 정수 리터럴에 상한을 두지 않습니다. 그래서
  `"9999999999999999999"`는 plain 스칼라로 투사되었고 `to_json`은 그것을 수자로 되돌려 썼습니다.
  `load_json`이 문자열로 유지하던 값이 허브를 한 번 지나 정수가 된 것입니다. 이제 투사는 문자열을
  plain으로 적기 전에 JSON 자신의 수자 문법에 먼저 묻습니다. 적기가 그 텍스트에 물을 바로 그
  질문입니다 (libFuzzer `json_roundtrip` crash-09341b75).

- **`schema="1.1"` 프로필이 1.1 이 정의한 숫자 계열을 해석합니다** — `1_000` 은 정수 1000, `0b101`
  은 5, `+0x1f` 는 31, `+.inf` 는 무한, `017` 은 15 입니다. 모두 PyYAML 기본 로더가 같은 텍스트에
  주는 답과 같습니다. 이 프로필은 Core 에 레거시 불린만 더하는 것이었고 기록 조항은 #504 부터 “1.1
  읽는 쪽이 형을 정한다”는 이유로 따옴표를 씌우고 있었으니, 이 엔진에 없는 읽는 쪽을 위해 씌운
  셈입니다. 타임스탬프와 육십진법은 텍스트로 남고(태그 없는 텍스트는 텍스트라는 판정, 콜론은 이미 한
  단계 아래에서 따옴표를 강제합니다), `0o17` 은 PyYAML 에서는 텍스트인데 여기서 형이 붙는 이유는 이
  프로필이 Core 에 더하는 방식이고 `0o` 는 Core 접두사이기 때문입니다.

- **1.1 읽는 쪽만 형을 정하는 문자열을 그대로 내보내지 않습니다** — `0x_1f` 는 PyYAML 에서 31, Core
  에서 텍스트인데 “1.1 읽는 쪽이 형을 정하는가”에 답하는 조항은 밑줄을 10 진 어절에서만 살펴보았기
  때문에, Python 문자열 `"0x_1f"` 는 그대로 나가서 읽으면 숫자가 되었습니다. 이 구멍은 이 프로필이
  형을 정하는 모든 텍스트에 대해 `needs_quotes` 의 합의를 요구하는 새 불변식 테스트가 찾아냈습니다.

- **TOML 작성자가 절 제목의 키 이름을 더 이상 따로 만들지 않는다** — `to_toml` 은 모든 최상위 표의
  제목 문자를 별도의 `String` 에 담았다가 최상위 쌍을 다 쓴 뒤에 적었습니다. 절 목록이 키 마디를
  이미 지니고 있으므로 제목 갈래는 쌍의 줄과 똑같이 `push_scalar_key` 로 바로 덧붙일 수 있습니다.
  이것이 작성자가 만들던 유일한 키였습니다: `to_toml_medium` 은 바닥값 28,939,722 에 대해 계측 명령
  26,741,057, 즉 -7.60% 입니다. 제목마다 확보와 복사가 한 번씩 줄어듭니다.

- **TOML에서 쌍 뒤에 놓을 수 없는 최상위 표는 제자리를 지키려고 인라인이 된다** — TOML은 최상위
  table 헤더 뒤에 최상위 key/value를 인정하지 않으므로, `to_toml`은 모든 절을 모든 쌍 뒤에
  적었습니다. 그래서 `0`이 표를 받고 그 뒤에 `a`가 오는 트리는 `a`, `0`으로 되돌아왔습니다. 생성기가
  20 000 사례 속성 단계에서 이를 찾았습니다(`prop_toml_roundtrip_preserves_the_tree`). 이제 그런
  표는 제자리에 인라인으로 쓰이고, 분석된 문서가 가질 수 있는 순서는 그대로 헤더 형태를 유지합니다.

- **시퀀스 자신의 주석이 항목의 대시 행을 빼앗지 않는다** — 편집 계층이 *컨테이너*에 붙인
  주석은 그 컨테이너의 블록을 끝낸 행 위에 쓰였습니다. 컨테이너가 시퀀스일 때 그것을 끝내는 행은
  항목 자신의 대시 행이고, granit 은 대시 행 아래에 쓰인 주석을 그 항목의 선행 주석으로 보고하므로
  두 번째 출력에서 주석이 시퀀스에서 항목으로 옮겨갔습니다(`edit_yaml` crash-08b30707: `?ss:` 뒤에
  태그가 붙은 빈 항목 둘). 시퀀스의 주석은 이제 시퀀스의 첫 행 위에 놓입니다. 독자가 시퀀스 자신의
  주석을 보관하는 자리이며 `lift_first_item_notes` 가 이미 의존하는 자리와 같은 곳입니다. 매핑의
  주석은 여전히 그 블록을 끝낸 쌍 위에 놓입니다. 독자가 그 자리에서 쌍의 주석을 읽어들이기
  때문입니다.

- **#522가 깨뜨린 벤치마크가 이제 일반 테스트 스위트에서 계약을 검증합니다** — `Python benchmarks`
  레그는 `806811c5`에서 `assert '[server]' in ...` 때문에 빨간색이 되었지만 필수
  `Test matrix (all legs)`는 성공을 보고했습니다. #522는 뒤따르는 루트 쌍 앞에 있는 루트
  테이블을 인라인으로 출력하므로 픽스처의 `[server]` 헤더는 더 이상 출력이 아니며, 기존 단언은
  문서의 속성이 아니라 #522 이전의 표기를 설명했습니다. 단언은 이제 일반 스위트
  (`tests/test_toml.py::TestToToml::test_benchmark_fixture_roundtrips_by_values_and_order`)에
  있습니다. 벤치마크 모듈은 `--codspeed` 없이는 선택 해제되기 때문입니다. 출력을 다시 읽으면
  잃거나 바뀐 키를 잡고, 루트 키 목록 비교는 순서를 잡습니다. 출력된 루트 줄 두 개를 맞바꾸면
  첫 비교는 같고 두 번째가 뒤집히며, 벤치마크 본문은 측정 대상 호출만 남습니다.

- **명령 수 게이트가 이제 병합 판정의 일부입니다** — `Instruction-count baseline` 작업은
  `.github/workflows/codspeed.yml`에 있었지만 `Test matrix (all legs)`는 `ci.yml`의 작업만
  모으므로 그 결과는 보고에 불과했습니다. #522가 빨갛게 만든 `Python benchmarks` 레그는 필수
  검사가 초록인 채로 트렁크에 도달했고, Ir 게이트도 같은 길을 걸을 수 있었습니다. 작업은 그대로
  `ci.yml`로 옮겨져 다른 무거운 레그와 같은 `changes` 분류로 실행됩니다. `matrix-verdict`가
  이제 이 작업을 기다리므로 성능 회귀는 트렁크에 도착하기 전에 병합을 거부합니다. 두 CodSpeed
  작업은 `codspeed.yml`에 조언용으로 남습니다. 10% 미만의 월타임 변동은 순위용이며 병합 결정에
  쓰기에는 부적합합니다.

- **Ir 게이트가 트리에 뒤처진 바닥값을 거부합니다** — 게이트는 베이스라인의 한쪽만 읽었습니다.
  허용치를 넘어 증가한 시나리오는 실패하지만, 현재 트리보다 훨씬 높은 바닥값은 조용히 받아들여져
  엔진이 빨라질 때마다 허용되는 여유가 넓어졌습니다. 2026-10-14 리프레시는 그 누적을 측정했습니다:
  라이터 시나리오 6개가 바닥값보다 22–46% 낮았고 리더 4개가 4.7–11.2% 낮았습니다.
  이제 `scripts/ir_gate.py`가 양쪽을 읽습니다. 바닥값보다 2% 넘게 낮은 시나리오는 `FLOOR_STALE`로
  경고하고, 5%를 넘으면 실패합니다. 실패 메시지는 바닥값을 로컬 호스트로 옮기는 `--update`가 아니라
  리프레시 dispatch를 가리킵니다. 규칙 적용 후 첫 강제 이미지 리프레시(run 39242949404, 산출물
  `ir-baseline-39`)는 `accept_drift` 없이 `to_toml_medium`을 −7.12%(옛 게이트는 받아들이고 새
  게이트는 거부하는 이동) 움직였고, `from_toml_medium`을 −3.00% 움직였습니다. 가장 큰 상승은
  `serialize_block`의 +0.32%였으며 모든 시나리오의 반복 측정 산포는 0.000%였습니다.
  `.ci/ir-baseline.json`은 그 산출물 그대로이며 편집하지 않았습니다.

- **중첩 시퀀스에서 끝나는 매핑의 주석이 한 번의 라운드에서 정착합니다** — 블록이 중첩
  시퀀스 내부에서 끝나는 매핑의 인라인 주석이 마지막 항목의 대시 줄 위에 기록되어, 리더가
  이를 해당 항목의 선행 주석으로 보고했습니다: `ss:\n  - !a\n  # fuzz 35\n  - !a\n` 는
  `ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n` 로 다시 읽혀 첫 출력이 고정점이 아니었습니다.
  컨테이너 자체의 인라인 주석을 배치하는 `pair_line_start` 필드는 블록 값의 기록이 자신의
  시작 위치를 덮어써서 시퀀스 항목의 줄을 가리킨 채였습니다. 이제 각 페어는 반환 전에 자신의
  줄을 복원하므로 주석이 매핑의 블록을 끝낸 페어 위에 놓입니다. 주석의 소유자는 여전히
  이동하지만(컨테이너에 대한 `set_comment` 판정은 그대로입니다), 텍스트는 한 라운드에서
  정착하며 `a_mapping_note_over_a_nested_sequence_settles_in_one_round`가 세 가지 형태로
  고정합니다.

- **TOML 정수는 작성된 철자를 유지합니다** — `1_000`, `+5`, `-1_000` 및 각 기수 형식은
  허브를 거치며 소스 텍스트를 잃었습니다. 파서가 기록 전에 토큰을 정리했기 때문에
  `to_toml(from_toml("a = 1_000"))` 은 `a = 1000` 을 반환했습니다. 구분자와 명시적 부호는 모두
  합법적인 TOML(ABNF의 `underscored` 와 `signed-int`)이므로 손실은 문법이 아니라 기록 단계에
  있었습니다. 이제 파서는 토큰 텍스트를 그대로 보존하고, YAML Core가 다른 값을 해석할 때만
  프로젝션이 태그를 붙입니다(`0x1f` 와 `+5` 는 태그 없음, `1_000`, `0b1101_0110`, `-1_000` 은
  `!!int`). 라이터의 리터럴 판정도 자신이 출력하는 것과 같은 문법을 받아들입니다. 태그가 붙은
  정수는 선행 `-` 때문에 YAML 이미터가 필요로 한 인용부호보다 우선하며, 태그 없는 인용부호
  `"42"` 는 문자열로 남습니다. 값은 변하지 않고 `tomllib` 과 일치하며,
  `TestIntegerSpellingFidelity`, `decimal_spellings_reach_the_hub_word_for_word`,
  `a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise` 로 고정합니다.

- **선언된 `%YAML` 버전은 요청 시 존중되며 출력에도 명시됩니다** — 문서 헤더는 그동안
  `version()`으로 보고될 뿐 리더가 무시했기 때문에 `%YAML 1.1`도 Core로 해석되었고 지시어는
  출력에서 사라졌습니다. `parse`, `parse_file`, `parse_all_docs`, `safe_load`, `safe_loads`,
  `YAML(...)`에 `honor_version_directive=True`를 전달하면 `1.1` 선언이 YAML 1.1 프로파일을
  선택합니다(`flag: yes`는 `True`, `012`는 `10`). `1.2`, `1.3` 및 선언이 없는 문서는 여전히
  Core로 해석되며, 명시적 `schema`가 선언보다 우선합니다. 플래그가 켜지면 채택된 선언이 `---`
  앞에 `%YAML <version>`으로 다시 출력되고(편집 후와 `reparse()` 후에도), Core 리더가 텍스트를
  조용히 다른 타입으로 읽지 못하게 합니다. 스트림에서 뒤따르는 문서의 선언 앞에는 문법이 요구하는
  `...` 종료 표식이 붙습니다. 기본 동작은 그대로입니다(플래그 꺼짐, Core 해석, 출력에 지시어 없음).
  `schema`의 기본값은 `None`이 되어 "지정하지 않음"과 "명시적 Core"를 구분합니다.
  `TestYamlVersionNegotiation`으로 고정합니다.

- **벤치마크 페이지가 실제 사용하는 측정 방식을 표기합니다** — `codspeed.yml`은 추가된 때부터
  `mode: simulation`으로 실행되었지만 `AGENTS.md`와 네 개 로케일의 벤치마크/기능 페이지는
  "WallTime 모드"라고 적혀 있었습니다. 이제 페이지는 시뮬레이션이 명령 수를 센다고 밝히고,
  밀리초 표는 2026-08-17에 한 대의 머신에서 기록한 실제 시간 측정값이며 CodSpeed 출력이 아니라고
  명시합니다. 중간 크기 픽스처는 약 500 B가 아니라 687 B입니다. Rust 표는
  `pyrs_yaml_core::bench_inputs`에 대한 `cargo codspeed` 시뮬레이션 레그임을 밝히고, "small"은
  19 B로 Python 표의 95 B와 다르므로 같은 이름의 행은 비교할 수 없습니다. 기능 매트릭스에서
  PyYAML의 앵커/별칭, 명시적 태그, 병합 키를 체크로 바꾸고(6.0.3에서 실측), 타입 스텁 행은 각
  패키지의 실제 내용을 따릅니다: PyYAML 없음, ryaml과 yaml_edit 있음. ABI3 행은 실제 하한인
  py3.8+를 적습니다.

- **`safe_dumps_async`를 `pyrs_yaml.__all__`에 공개** — 이 별명은
  `python/pyrs_yaml/__init__.py`에서 `safe_dumps`와 함께 대입되고 3개 로케일 API 페이지도 이미
  멤버로 적어 두었지만, 선언된 공개 면에는 async 형제 3개만 올라 있었다. 즉
  `from pyrs_yaml import safe_dumps_async`는 동작하는데 공개 면은 그 이름이 없다고 말한 셈이다.
  동작은 변하지 않는다(예전부터 import 가능). `tests/test_api_doc_signatures.py`는 이제 4개 로케일의
  `reference.md`에서 같은 35개 이름을 이 건에 대한 예외 없이 비교한다.

- **직접 로드의 적합성 판정을 엔진 crate로 옮겼다** — 어떤 문서가 값 전용 빠른 경로의 안쪽에
  들어가는지는 문서 그 자체와 문법이 정하는 것이지, 값이 마지막으로 어떤 Python 객체가 되느냐가
  정하는 것이 아니다. 그래서 그 판정과 판정을 고정하는 두 개의 테스트는 이제 `pyrs-yaml-core`
  (`crates/pyrs-yaml-core/src/parser/yaml/direct.rs`)에 놓였다. 바인딩 crate에는 값을 만들어 내는
  일만 남는데, 이 부분이 Python 객체가 있어야만 동작하는 유일한 부분이고, 바인딩 crate의 Rust 테스트
  단계는 이제 비어 있다. 이 단계의 출발 전제였던 "Rust 테스트 단계가 Python 런타임을 요구한다"는
  주장은 바이너리 수준에서 사실이 아니다. `cargo test --workspace --no-run`이 만들어 낸 848개의
  산출물 가운데 `python*.dll`을 이름으로 든 것은 하나뿐이고, 그것이 바로 확장 모듈 그 자체다. 여전히
  묶여 있는 것은 빌드 그래프다. `pyo3`와 `numpy`, 그리고 그 빌드용 crate들이 바인딩 crate의 closure
  안에 아직 있으며, 바인딩 crate의 `ir_gate` 벤치 하네스는 명령어 수를 세기 위해 CPython을 띄운다.
  동작은 달라지지 않았다. `safe_load`과 그 형제함수는 예전과 똑같은 문서들에 똑같은 답을 준다.

- **엔진 CI 레그가 Python 런타임 더 이상 빌드하지 않습니다** — `cargo test --workspace`와 20 000
  케이스로 올린 property 계층은 이제 `--exclude pyrs-yaml`을 넘기고, `PYO3_PYTHON`도 함께
  사라졌습니다. 이 트리에서 측정하면 앞의 명령은 153개 패키지에 걸친 250개 빌드 단위로 전개되고,
  그 집합은 `pyo3`, `pyo3-ffi`, `pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config`,
  `numpy`를 이름으로 듣습니다. 같은 명령에 제외를 붙이면 139개 패키지의 220개가 되고 위 항목은
  하나도 남지 않습니다. 잃는 것은 없고, 그것이 바로 제외를 허가하는 근거입니다. `cargo nextest
  run --all`도 제외를 붙인 같은 실행도 748개 테스트를 돌리고, 확장 crate에는 잃을 doctest가
  없습니다(`cargo test -p pyrs-yaml --doc`은 0개). 두 주장을 함께 묶는 규칙이
  `tests/test_ci_rust_leg_exclusion.py`입니다. 워크스페이스 전체를 테스트하는 레그가 decoupling
  게이트가 지칭하는 확장 crate의 제외를 멈추거나, 제외하면서 `PYO3_PYTHON`을 남기거나, 다른
  멤버를 제외하거나, MSRV 레그가 제외를 시작하면 빨갛게 실패합니다. MSRV 레그는 출하하는 crate를
  포함해 모든 crate에 대해 선언된 하한을 증명하는 자리이기 때문입니다. 공개된 동작은 변하지
  않습니다. 같은 748개 Rust 테스트와 같은 Python 스위트가 그대로 실행됩니다.

#### 제거

- **자체 제작 규칙 검증기를 은퇴시키고 `Document.validate()`를 유일한 스키마 표면으로 삼습니다** ——
  `validate_against_schema()`와 `validate_against_registered_schema()`가 제거되었고, 스키마 정의의
  `validate:` 섹션은 더 이상 읽히지 않습니다. 두 이름은 이 저장소가 만든 규칙 언어를 평가했지만,
  `Document.validate()`는 이미 Python의 `jsonschema`에 위임하고 있었습니다. 검증기가 둘이라는 것은
  오류 계약이 둘, "이 문서는 유효한가"에 대한 답이 둘이라는 뜻입니다. 호출자는
  `Document.validate(schema)`(`YamlValidateError`가 참조 엔진의 메시지를 담습니다) 또는
  `jsonschema.validate()`를 직접 사용합니다. CLI의 `validate`는 `--schema`와 `--schema-file`을
  하나의 매개변수(JSON Schema 파일)로 묶고, 둘 다 주지 않으면 구문 검사만 수행합니다. 스칼라 타입
  해석은 영향을 받지 않습니다: `register_schema()`, `load_schema()`, `list_schemas()`,
  `YAML(schema=...)`, `rules:` 목록은 그대로 동작합니다. 은퇴한 언어만을 위해 존재하던 Rust 타입
  (`TypeSpec`, `ValidateKind`, `ValidateRule`, `SchemaValidationError`)도 삭제되고, 매트릭스의
  `schema validation` 행도 함께 사라집니다. 표는 이제 네 행을 도출하며
  `.ci/quality-holes.json`은 `holes: []`로 측정됩니다. 그 행을 담당하던 fuzz 타깃은
  `validate_schema`에서 `parse_schema`로 이름이 바뀌었습니다. 퍼징 대상이 JSON Schema 평가가 아니라
  살아남은 `rules:` 문법이기 때문입니다. 네 로케일의 가이드, 빠른 시작, 튜토리얼, 기능 표, API
  페이지는 `Document.validate()`를 문서화하며, 배포되는 스텁에는 삭제된 두 함수 이름이 나타나지
  않습니다.

#### 수정

- **가장 최근 두 릴리스도 이전처럼 접힙니다** - `## [v0.18.1]`과 `## [v0.18.3]`는 다섯 릴리스 노트
  모두에서 `<details markdown="1">` 접기를 빼먹었습니다. v0.12.x 이후 모든 릴리스가 그 접기를
  가집니다. `scripts/check_doc_metadata.py`는 접기의 속성만 보고 존재는 묻지 않았습니다. 이제 두
  절을 접었고, 접기나 `<summary>`가 없는 릴리스 절은 바로 빨개집니다.

- **wheel 레그가 스위트가 들여오는 것을 설치합니다** - `.ci/requirements-test.txt`는 릴리스
  워크플로가 설치하는 유일한 요구 파일인데 `tomlkit`과 `json5`를 모두 적지 않았습니다. 네 테스트
  모듈은 둘 다 모듈 범위에서 들여옵니다. 이 릴리스의 첫 Publish에서는 wheel이 하나도 올라가기 전에
  다섯 wheel 레그가 수집 단계에서 빨개지고 `Release` 잡이 건너뛰었으니 PyPI에 도달한 것은 없습니다.
  PR 매트릭스는 개발 그룹을 설치하고 거기서는 둘 다 선언되어 있어 이 실패를 볼 수 없었습니다. 이제
  둘을 선언하고, `scripts/check_wheel_requirements.py`(prek 훅이며
  `tests/test_wheel_requirements_gate.py`가 지킴)가 `tests/`의 모듈 범위 들여오기를 그 파일과
  대조합니다.

- **CLI 명령 면 목록 게이트가 실제로 실행됩니다** - `tests/test_cli_binding_parity.py`는
  `getattr(pyrs_yaml.cli, "app")`를 먼저 읽어 App을 해결했지만 `python/pyrs_yaml/cli/__init__.py`는
  해당 속성을 바인딩하지 않습니다. `main()`만 정의합니다. 앞서 `from pyrs_yaml.cli.app import app`을
  실행한 테스트(`tests/test_cli.py`는 모듈 스코프에서 그렇게 합니다)가 이 이름을 하위 모듈에
  바인딩하므로 fixture는 모듈을 반환하고 `_commands`가 없어 가드는「cyclopts 내부 레이아웃 변경」을
  이유로 건너뛰었습니다. 실측 결과 해당 파일 단독 실행은 22건 통과하고,
  `tests/test_cli.py::test_entry_point_without_cyclopts`를 앞에 두면 목록 단언이 SKIPPED가 됩니다.
  이 저장소 자체 runner의 `test (ubuntu-latest, 3.10)` 로그에서도 같은 한 줄이 건너뛰기입니다.
  즉「등록된 명령 집합이 문서 목록과 같다」는 단언이 전체 세션에서 단 한 번도 실행되지 않았습니다.
  이제 fixture는 명령 테이블을 지닌 package 속성만 받아들입니다. `_commands`가 없으면 드리프트와
  구별 불가능한 건너뛰기가 아니라 설치된 cyclopts 버전을 지목하는 실패입니다. 순서 회귀가 조용해지지
  않고 빨갛게 드러나도록, 해석 결과가 모듈에 닿지 않음을 검증하는 제어 테스트 하나를 추가했습니다.
  변경 전 목록 자체도 대조했습니다: 문서 18개 이름 대 등록 18개, `missing=[] extra=[]`. 따라서
  이번은 가드의 정직함을 되돌리는 변경이며, 면의 드리프트를 발견한 것이 아닙니다.

- **pyq의 모든 fixture 디렉터리는 이제 프로세스 id와 함께 카운터를 가집니다** — 방언 대조의 비교
  디렉터리는 공유 경로가 `rust-lint`를 붉게 만들었을 때 이 보호를 얻었지만, 같은 crate의 나머지
  쓰기는 여전히 프로세스 id만으로 이름을 지었습니다. `crates/pyrs-yaml-cli/tests/cli.rs`의 여섯 곳,
  `crates/pyrs-yaml-cli/tests/compare.rs`의 `pair()` 헬퍼와 그 유일한 수동 사본, 그리고
  `src/compat.rs`의 compat 필터 테스트입니다. 오늘의 테스트 중 같은 접두사나 tag를 고르는 쌍은
  없으므로 이것은 두 번째로 재현된 충돌이 아니라 같은 불변 조건을 모든 곳으로 넓힌 것입니다.
  카운터는 "접두사가 항상 유일하다"는 전제에 대한 의존을 없애고, `compare.rs`는 `pair()`의 두 번
  쓰기를 손으로 복사하지 않습니다.

- **방언 대조 비교들이 fixture 디렉터리를 공유했습니다** —
  `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs`는 임시 디렉터리에 프로세스 id로 이름을
  붙였기에, Rust 하네스가 하나의 테스트 바이너리를 같은 프로세스 안에서 스레드로 돌릴 때 모든 비교가
  같은 위치의 `doc.yaml`과 `doc.json`을 서로 읽는 중에 다시 씁니다. `fs::write`는 먼저 잘라낸 뒤
  쓰고, 그 대가는 PR #556의 `rust-lint`에서 실측됐습니다:
  `jq [".a", "doc.json"] answered differently`, 우리 쪽 `(0, "")` 대비 참조 쪽 `(0, "null\n")` — 두
  쪽이 같은 파일을 서로 다른 순간에 읽은 것입니다. `jq`와 `yq`를 설치하는 것은 그 잡뿐이므로,
  여기서의 충돌은 `rust-lint`가 빨개지는 것으로만 드러나고 건너뛰기로는 드러나지 않습니다. 이제
  호출마다 자기 디렉터리(프로세스 id와 카운터)를 쓰고,
  `each_comparison_gets_a_fixture_directory_no_other_comparison_reads`가 공유 경로를 거부합니다.
  프로세스 id만으로 되돌리면 그 테스트가 실패함을 확인해 제어항으로 삼았습니다.

- **`no_std` 게이트가 자기 자신 말고는 비교하지 못했다** - `tests/test_no_std_gate.py`는
  `ci.yml`의 크레이트 목록을 테스트에 옮겨 적어 넣은 집합과 견줬기 때문에, `#![no_std]`를
  늘려도 맨메탈 빌드가 빠진 채로, 속성을 지워도 빌드가 남은 채로 일이 초록으로 지나갔다.
  이제 집합은 각 크레이트의 루트 파일에서 읽는다. 대조군은 둘이다: 합성한 소스에 읽기
  함수를 돌려 속성과 그것을 인용한 문장을 나누는 것, 그리고 빈 집합을 거부하는 것.
  빈 것과 빈 것의 일치는 통과로 보이기 때문이다.

- **권한 열이 빈 것은 문장이었고 측정이 아니었다** - `QUALITY_MATRIX.md`는 스키마 층에 바깥 읽기가
  없다고 적으면서도 `scripts/quality_matrix.py`는 빈 권한 열을 구멍으로 내놓지 못했다. 그래서
  `.ci/quality-holes.json`이 `holes: []`로 남아 있으면 그것이 곧 약점 없음으로 읽혔다. 이제
  점검기는 `authority-tier:schema-validation`을 내놓고, 대장부는 그것과 그것을 닫는 말뭉치를 함께
  적는다. 대조 검사는 손으로 만든 행으로 두 방향을 본다: 비면 표식이 나오고, 채우면 나오지 않는다.

- **`pyq`가 순서열 슬라이스를 키로 읽었다** - `[0:2]`는 `jq`와 mikefarah `yq`가
  범위를 쓰는 방식인데, 대괄호 파싱이 키 분기로 빠져서 `.list[0:2]`는 `0:2`라는
  키를 찾고 실패했고, 호환 셸은 `null`로 답했습니다. 이제 문법도 찾지 못하는 `:`를
  거부하고 셸은 구성 요소를 이름으로 거부합니다. 따옴표가 있는 대괄호
  (`["k:v"]`)는 여전히 키입니다. `--as jq|yq`에서는 하나도 고르지 못한 `select`도
  더 이상 오류가 아닙니다. 실제 두 도구는 출력 없이 0으로 끝나므로 셸도 같아지고,
  네이티브 `pyq get`은 계속 실패로 보고합니다.

- **`pyq get --json`이 일치 1건과 여러 건에 서로 다른 형식을 내놓았다** — 1건은
  들여쓰기 출력기를 거치는데 여러 건은 값마다 한 줄로 압축됐으니, `.servers`와
  `.servers[*]`가 `--json`의 뜻에서 맞지 않았다. 이제 모든 일치가 같은 형식을 쓰고,
  그것은 `jq`가 각 출력값에 쓰는 형태와 같다. 호환 셸은 그 합의의 나머지 절반을
  얻었다. `--as jq|yq`에서는 문서에 없는 경로가 `null`을 출력하고 0으로 끝나 실제
  두 도구와 같은 답을 낸다. 네이티브 `pyq get`은 미스를 보고하고 1로 끝난다.

- **CLI만 고친 수정이 바꿀 수 없는 확장 재빌드를 요구했다** - 선명도 게이트가
  「어떤 파일이 불러온 모듈에 컴파일되는가」를 정할 때 `crates/` 아래를 모두
  돌았기 때문에 단독 `pyq` 크레이트까지 배포 대상으로 계산됐고, 그 파일만
  건드릴 때마다
  `maturin develop`과 빨간 실행을 치렀습니다. 이제 crate 집합은 바인딩 manifests의
  `path` 의존을 추적적으로 풀어구하고 dev 의존은 뺍니다. 두 방향이 모두 대조군입니다:
  실제로 컴파일되는 소스는 그대로 이름을 밝히고, CLI 소식은 밝히지 않습니다.

- **병합은 같은 매핑에 두 번째 null 키를 주지 않는다** — `~`, `null`, 빈 키, 맨 `Null`
  노드는 모두 null로 해석되고 읽기 쪽은 의도적으로 null 키를 하나의 항목으로 접는다 (그래서
  `: a` + `: b`가 통과한다). 그런데 병합 쪽 판정은 다른 질문을 했다: 태그 없는 스칼라는 텍스트로,
  나머지는 노드 전체로 비교했다. 그래서 `<<` 원본이 자기 null 키를 이미 하나 가진 매핑에 집어
  넣었고, 두 병합 원본이 같은 키를 부를 수도 있었다 — 출력은 같은 층에 `~:`를 두 번 쓰고, 다시
  읽으면 그 짝은 값과 실려 있던 앵커까지 잃는다. `same_emitted_key`는 읽기 쪽 자기 서술자를
  묻도록 바뀌었고 병합 집합도 같은 동일성으로 접기 때문에, "먼저 그 키를 준 원본이 이긴다"가
  null의 모든 표기에 성립하고, 원래부터 같았던 키에만 성립하던 이전과 차이가 난다. 20,000
  예제 property tier가 찾아냈다 (CI 시드 `c9ef4fd3...`, 하나의 트리에서
  `prop_mapping_order_preserved`와 `prop_tag_preserved`가 이 하나의 이유로 실패).
  `a_merge_never_repeats_a_null_key_the_mapping_owns`와
  `crates/pyrs-yaml-core/proptest-regressions/pbt.txt`에서 재생하는 시드로 고정했다.

- **API 페이지가 스텁이 선언한 매개변수 목록을 적습니다** — `docs/{zh,ja,ko}/api/reference.md`에서
  `parse_file`, `parse_all_docs`, `safe_load`, `safe_loads`를 한두 개의 매개변수로 적어 두었지만
  공개 스텁은 다섯·여섯 개를 선언합니다. `%YAML` 버전 협상 깃표 `honor_version_directive`는 사용자
  문서에 전혀 나타나지 않았고, `reparse()`는 확장 기능이 쓰지 않는 `schema` 기본값을 적고
  있었습니다. 이제 네 언어가 스텁과 일치하고, 각 매개변수는 담당 페이지에 보강했으며,
  `tests/test_api_doc_signatures.py`가 양쪽을 `ast`로 비교하므로 짧아진 서명은 독자의 의외가 아니라
  빨간 실행이 됩니다. 본문 페이지에서 단 한 번도 불리지 않은 유일한 공개 멤버
  `YamlDocument.flush_source()`도 네 언어에 보강했고, 게이트는 스텁이 선언한 모든 공개 이름이 각
  언어 페이지에 나타남도 확인합니다.

- **중복 키 접기가 떼 내는 닻 정의는 버려지지 않고 펴 넣습니다** — `: abase: &x 1`에 `use: *x`를
  잇고 빈 키를 한 줄 더 얹으면 첫 줄과 겹치므로, 접기는 `&x`를 *값 아래*에 둔 항목을 떨어 냅니다.
  복원 단계는 떨어진 항목 자신의 닻만 받았기 때문에 이름이 정의를 잃고, 출력은 `use: *x` 였습니다 —
  이 라이브러리 자기 읽기 단계가 거부하는(found unknown anchor) 텍스트이며, 엔진은 결코 만들지
  않기로 약속했습니다. 이제 접기는 트리를 떠나는 정의를 모두 등록하니 남은 참조가 닻과 함께 펴
  들어가고, 출력은 다시 읽혀 한 번에 고정점이 됩니다(libFuzzer `edit_yaml` crash-68617eaf).
  `a_definition_folded_away_from_inside_the_dropped_value_is_still_emitted`, Python 쪽
  `test_a_folded_away_anchor_definition_still_reaches_the_output`, 새로 만든 닻 그림 말뚝문 검증,
  다시 실행하는 시드

- **머리표를 가진 항목의 주석은 사슬 가장 바깥 머리표 위에 놓습니다** — 블록 컨테이너가 자신의
  `&a` / `!tag` 줄 위에 쓸 수 없는 주석을 *가장 가까운* 머리표 위에 올려 왔습니다. 읽는 쪽이 첫
  항목의 선행 주석을 출력마다 한 단계씩 위로 올리기 때문에 주석이 계속 올라갑니다:
  `- &a3\n  - &a0 !fuzz2\n    - {"-(.0bb:": v6}  # fuzz 47` 에서 가장 안쪽 항목에 주석을 붙이면
  첫 출력은 `&a0` 아래에, 두 번째 출력은 `&a3` 아래에 씁니다. 이제 쓰는 쪽이 값이 시작하는 틈의 가장
  바깥 머리표를 추적해 그 머리표 위에 주석을 쓰므로 첫 출력이 고정점이 됩니다(libFuzzer `edit_yaml`
  crash-2d978c7b). `a_headed_items_note_settles_above_the_outer_sequences_header`와 재실행한 시드
  `fuzz/seeds/edit_yaml/former-crash-2d978c7b.seed`가 이를 고정합니다. 머리표 위치는 직렬화기
  상태가 아니라 항목 작성자에게 넘기는 인자로 바꿨습니다. 상태 방식이 무료가 아니었기 때문입니다:
  명령어 게이트의 Rust 하네스로 확정된 바닥선에 대해 실측한 결과, 상태 방식은 `serialize_small`에서
  +0.92%, `serialize_block`에서 +0.75%를 쓰는 반면 인자 방식은 +0.0006%와 +0.19%입니다.

- **앵커가 붙은 항목의 주석은 쪼갤 수 없는 헤더 위에서 안정됩니다** — 시퀀스 항목인 블록 매핑이
  태그만 있는 행(`- a: !o`)으로 블록을 끝낼 때, `set_comment_path`는 그 매핑 자신의 인라인 주석을
  `&a` 헤더와 본문 사이에 썼습니다. 읽기는 그 자리의 주석을 매핑의 *선행* 주석으로 취급하므로 다음
  출력에서 주석이 헤더 위로 올라갔습니다. 첫 출력은 `&a0` / `# note` / `a: !o`, 두 번째는
  `# note` / `&a0` / `a: !o`여서 편집이 안정되기까지 두 라운드가 필요했습니다. 쓰기는 이제 그 주석을
  헤더 행 위에 놓습니다. 읽기가 주석을 돌려주는 위치이며 첫 출력이 고정점이 됩니다(libFuzzer
  `edit_yaml` crash-b22d0653). `an_anchored_items_note_settles_above_its_header`와 재실행한 시드
  `fuzz/seeds/edit_yaml/former-crash-b22d0653.seed`가 이를 고정합니다.

- **TOML이 거부하는 YAML 부동소수점 표기는 TOML 문법으로 기록됩니다** — Core YAML은 `083e6`을
  부동소수점으로 해석합니다. 지수 형식은 정수부의 선행 0을 허용하지만 TOML의 `float-int-part`는
  `dec-int`여서 그런 표기가 없습니다. 쓰기의 통과 판정은 `e` 포함 여부만 보았기에 값이 `083e6`인
  문서를 내보내면 `lis6 = 083e6`을 써서 TOML 읽기가 자기 쓰기의 텍스트를 거부했습니다(libFuzzer
  `edit_yaml` crash-71feb9d9). 판정은 이제 부동소수점 문법 전체를 검증한 뒤 통과시키며, TOML이
  읽을 수 없는 표기는 값에서 렌더링합니다.
  `a_yaml_float_spelling_toml_refuses_is_written_in_toml_grammar`와 재실행한 시드
  `fuzz/seeds/edit_yaml/former-crash-71feb9d9.seed`가 이를 고정합니다.

- **컴팩트 항목의 주석은 해당 시퀀스 헤더 위에 안정됩니다** — 줄 끝이 이미 자신의 주석으로 채워진
  컴팩트 형식 `- key: value` 항목은 그 주석을 대시 위 줄에 씁니다. 시퀀스가 `&a` / `!tag` 헤더도
  쓴다면 그 줄은 헤더와 본문 사이에 놓이고, 읽기는 그 자리의 주석을 시퀀스의 *선행* 주석으로
  취급하므로 다음 출력에서 헤더 위로 올라갔습니다. 첫 출력은 `&a0` / `# note` / `- k: v  # note`,
  두 번째는 `# note` / `&a0` / `- k: v  # note`였습니다. 첫 항목은 이제 헤더의 줄 시작을 기록하고
  그 주석을 헤더 위에 씁니다. 읽기가 주석을 돌려주는 위치입니다(libFuzzer `edit_yaml`
  crash-2fc59020). `a_compact_items_note_settles_above_the_sequences_header`와 재실행한 시드
  `fuzz/seeds/edit_yaml/former-crash-2fc59020.seed`가 이를 고정합니다.

</details>

### [v0.18.1] — 2026-10-17

<details markdown="1">
<summary>프리 스레드 3.15t wheel · Linux arm64 · merge 키 주석 보존</summary>

#### 추가

- **Python 3.15t 및 Linux arm64용 프리-스레디드 휠** — 릴리스 매트릭스가 Windows, macOS, glibc
  Linux(x86_64 및 arm64)에서 `cp315-abi3.abi3t` 휠을 빌드하므로 3.15t 이상은 소스에서 컴파일하지
  않고 미리 빌드된 산출물을 설치합니다. Linux arm64의 `cp314t`와 `abi3t` 휠은 네이티브 arm64
  러너에서 빌드되어, qemu에서 실패하던 대상 인터프리터 exec가 네이티브 호출이 됩니다. 각 휠은 대상
  인터프리터에서 설치되고 전체 스위트를 실행합니다.

#### 수정

- **별칭을 값으로 갖는 병합 키가 인라인 노트를 유지** — `<<: *d  # note`는 읽기 단계에서
  노트를 잃었습니다. 별칭에는 메타데이터 슬롯이 없는데도 리더는 첨부에 성공했다고 보고했습니다.
  이제 노트는 페어의 키가 보유하며(블록 라이터가 출력하는 슬롯), 문서는 그대로 왕복합니다.
  병합을 해석하면 노트는 살아남은 항목의 매핑으로 옮겨집니다.

- **태그만 있는 줄 바로 아래의 키 인라인 주석이 소유자를 유지한다** — 쌍의 값이 자체 주석을 가지면
  키의 주석은 쌍 줄 위에 쓰인다. 바로 위가 태그만 있는 값(`k: !:`)이면 판독기는 주석을 그 값에
  넘겨 버려 문서가 두 번째 출력에서야 안정되었다. 작성기는 쌍이 시작될 때 캡처한 오프셋으로 그
  줄을 값 자체의 `~`로 먼저 닫는다(libFuzzer `yaml_roundtrip` crash-08cf4fe4).

</details>

### [v0.18.0] — 2026-10-17

<details markdown="1">
<summary>5개 형식 이벤트 스트림 · 태그 의미론 · 왕복 충실도 · 더 빠른 읽기/쓰기</summary>

#### 추가

- **5개 형식 이벤트 스트림** — `parse_stream`에 키워드 전용
  `format="yaml"|"json"|"jsonc"|"json5"|"toml"`이 추가되었다. 이벤트 딕셔너리는 동일한 7개 키를
  유지하고, 형식이 답할 수 없는 필드는 `None`이며, `anchor`/`alias`는 YAML 전용으로 남는다.
  기본 동작은 이벤트 단위로 변하지 않는다. `pyrs_json::parse_stream`과 검증하는
  `pyrs_json::from_events`가 공개되었고 TOML에도 트리 없는 프로듀서가 생겼다.
- **스키마 언어가 컨테이너를 지정할 수 있다** — `type: map` / `type: seq`는 내용을 제약하지 않고
  노드의 모양만 검사하며, `mapping_of: seq`는 매핑의 모든 값이 시퀀스임을 뜻한다. `mapping`,
  `object`, `sequence`, `array`, `list` 표기도 허용된다.
- **`no_std` 기반 크레이트** — `pyrs-ast`, `pyrs-schema`, `pyrs-json`, `pyrs-toml`이 `alloc`만으로
  빌드된다. 새 옵트인 `std` 피처가 `std::error::Error`와 `RandomState` 해셔를 복원하며 기본값은
  켜짐이다.
- **미리 빌드된 `pyq`와 Linux free-threaded wheel** — 릴리스가 6개 플랫폼용 `pyq`를 빌드해
  (Linux는 manylinux2014, glibc 2.17) GitHub Release에 첨부한다. wheel 매트릭스는 Linux
  free-threaded wheel도 출시한다.
- **CI에 Python 3.15 추가** — 매트릭스는 3.8부터 3.15까지, free-threaded 인터프리터를 포함한다.

#### 변경

- **표준 `!!` 태그가 로드 타입을 결정한다** — `!!str 1.20`은 문자열 `"1.20"`(이전에는 부동소수
  `1.2`가 되어 끝의 0이 사라졌다), `!!float 1`은 `1.0`, `!!bool yes`는 `True`,
  `!!binary aGk=`는 `b'hi'`, i64를 넘는 `!!int`는 정수로 남는다. 태그를 만족할 수 없는 텍스트는
  `YamlTypeError`를 발생시키고, 알 수 없는 애플리케이션 태그는 계속 관대하게 처리한다.
- **맨 `!`는 스칼라에 스키마 해석을 주지 않는다** — `! 12`는 YAML 1.2 §6.8에 따라 문자열
  `"12"`로 로드된다. PyYAML과 ruamel은 암묵적으로 해석해 `12`를 반환하지만 이번 릴리스는
  명세를 따른다. `!`에 등록된 플러그인이 태그를 덮어쓰지도 않는다. 0.18.0의 명시적 동작
  변경이다.
- **`version()`이 문서의 `%YAML` 지시자를 보고한다** — `%YAML 1.1`은 `1.1`, `%YAML 1.3`은
  `1.3`, 선언이 없는 문서는 여전히 `1.2`다. 라이터는 지시자를 다시 출력하지 않는다.
- **`bytes` 값은 `!!binary`로 쓰인다** — 내장 플러그인은 로컬 태그 `!binary`를 출력해 PyYAML이
  읽지 못했고 빈 페이로드에서 깨졌다. 로컬 표기는 로드 시 계속 허용된다.
- **TOML 이진 정수가 표기를 유지한다** — `b = 0b1010`은 `0b1010`으로 왕복한다(이전에는 `10`으로
  바뀌었다). `!!int 0b1010`으로 전달되어 모든 형식이 같은 타입을 읽는다. `0b1101_0110`은 여전히
  구분자를 잃고, 직접 만든 태그 없는 `0b1010`은 문자열로 읽힌다.
- **엄격한 JSON은 RFC 8259가 표기할 수 없는 값을 거부한다** — JSON 라이터는 비유한 부동소수를
  다시 쓰지 않고 오류를 내며, JSON5 리더는 문법이 금지하는 선행 0을 거부하고, JSON5 라이터는
  자체 리더가 거부하는 숫자를 더 이상 출력하지 않는다.
- **`granit-parser` 1.3 → 2.0** — 메이저 업그레이드에 소스 변경이 필요 없었다. 주석, 앵커,
  준수 계층이 모두 통과하고 `cargo tree`가 의존 그래프에 2.0이 있음을 확인한다.
- **`CustomNode`는 208바이트에서 144바이트로** (`no_std`에서는 128) — 메타데이터가 접근자 뒤로
  옮겨지고 드물게 쓰는 anchor/tag 슬롯이 하나의 포인터를 공유한다. `from_json_medium` −4.8%,
  `serialize_anchors` −4.0%이며 두 크기는 핫 패스 계약에 고정되었다.

#### 수정

- **주석, 앵커, 태그 소유권** — 주석은 마커, 태그, 중첩 컨테이너, 압축 `- key: value` 형식을
  지나며 자기 노드의 줄에 머문다. 첫 항목의 주석은 머리말 위로 올라가고, 플로우 안의 앵커된
  멤버는 이름을 유지한다. 앵커는 주석처럼 보이는 `#`, 짝이 맞지 않는 따옴표, 대괄호를 넘어
  살아남는다. 별칭은 바로 앞의 앵커에 묶이며(키 위치에서도), 중첩되거나 별칭을 거친 병합 키는
  버려지지 않고 적용된다.
- **블록 스칼라는 한 번의 출력으로 안정된다** — 빈 블록 스칼라는 머리말이 쓴 지시자를 유지하고,
  끝 개행을 만들거나 잃지 않으며, 지시자를 올바르게 감지한다.
- **TOML 왕복** — 인라인 테이블 안의 점 키가 중첩되고, 루트 수준 인라인 테이블이 섹션 머리말로
  다시 쓰이지 않으며, 테이블 머리말은 점으로 이은 텍스트가 아니라 세그먼트로 식별되고, 여러 줄
  배열 안의 주석은 해당 요소에 속하며, 인라인 테이블 멤버의 주석이 쓰이고, 여러 줄 문자열은
  따옴표, 이스케이프된 개행, 끝 구분자를 유지한다.
- **라이터 출력은 자체 리더에게 거부되지 않는다** — `to_toml`은 여러 형태에서 잘못된 TOML을
  출력했다(이제 게이트됨). JSON5 출력은 JSON5 리더로 다시 읽히고, JSONC/JSON5 라이터가 버리던
  주석이 살아남는다. 실측 비용은 `to_jsonc_medium` +4.77%, `to_json5_medium` +3.78%,
  `to_json_medium` +0.94%다.
- **중복 키와 null 키** — 중복 JSON 키가 주석을 함께 잃지 않고, 키에 주석이 붙어 있어도 중복
  이름은 이름으로 접히며, 중복 키는 해석된 값으로 거부되고, `null:`과 `'null':`은 서로 다른
  키다. null 키가 많은 문서는 선형 시간에 파싱된다(서로 다른 키 2k개 + null 키 2k개는 4배
  입력에서 12.4배로 늘었다).
- **매핑 키는 같은 텍스트의 값과 같은 뜻이다** — 키 정체성은 AST 노드 종류가 아니라 리더의
  해석을 따른다. 숫자로 해석되는 맨 표기도 예외가 아니다. 고정 모델은 15개 키 클래스에서
  18개로 늘었다.
- **스키마 검증은 태그를 먼저 읽는다** — `!!int 0b1010`은 정수 10으로 검증되고, `!!int hello`는
  타입을 바꾸지 않고 거부되며, 경로를 지정한 규칙은 잘못된 종류를 지나치지 않고 그 경로가 선택한
  노드를 검증한다. `$`가 규칙 대상이 되었고, 비 ASCII 규칙 경로가 더 이상 프로세스를 중단시키지
  않는다.
- **Python 계약이 온전히 출시된다** — 커밋된 stub은 파싱되고 mypy와 ty 검사를 통과하며, 10개
  확장 예외 클래스를 선언하고, 40개 모듈 함수의 매개변수 이름이 실행 객체와 일치한다. 28개 API
  페이지의 게시된 시그니처가 패키지에 대해 검사된다. 인라인 dict 스키마가 허용되고 모든 키가
  남는다. 생성된 레퍼런스는 이전에 빠졌던 4개 공개 함수(`to_toml`,
  `validate_against_registered_schema`, `validate_against_schema`, `validate_custom_types`)를
  렌더링한다.
- **경계 입력** — CR만 있는 개행에서 두 번째 라운드에 주석이 움직이지 않고, Unicode 공백, BOM,
  태그 접미사가 문서를 손상시키지 않으며, CJK와 이모지 키의 스트림 위치는 바이트가 아닌 문자로
  세고, 64비트를 넘는 Python 정수가 부동소수로 dumper를 떠나지 않는다.
- **`pyq`** — `validate`가 `--input`을 받아 실제로 읽은 형식을 따르고, 거부가 stdin을 닫아도
  테스트 하네스가 panic하지 않는다.
- **방어 계층은 끝에서 끝까지 측정된다** — 품질 매트릭스의 수치는 트리에서 세고, JSON/TOML fuzz
  타깃은 자체 라이터 출력이 다시 로드됨을 단정하며, PR 단계 fuzz는 차단형이고, 할당 상한과 명령
  수 플로어가 핫 패스를 덮는다.
- **릴리스의 `pyq` 자산은 매트릭스 타깃을 rustup에 전달한다** — `publish.yml`은
  `targets: ${ matrix.platform.std }`라고 썼다. 단일 중괄호는 GitHub 표현식이 아니므로
  rustup은 `${`, `matrix.platform.std`, `}`를 리터럴 타깃 이름 세 개로 받았고 모든 `pyq`
  레그가 툴체인 설정에서 멈췄다. 이제 단계는 `${{ matrix.platform.std }}`를 전달하며,
  형식이 되돌아가면 `tests/test_publish_toolchain.py`가 실패한다.
- **릴리스의 wheel 테스트는 스위트가 쓰는 도구를 갖춘다** — wheel 작업은 빌드 후 전체
  스위트를 실행하지만, 환경에는 `git`(Alpine 컨테이너)도 `tomli-w`(모든 레그)도 없어
  9개의 저장소 게이트와 스크립트 가져오기 테스트가 wheel 빌드 후 실패했다.
  `.ci/requirements-test.txt`는 `tomli-w`를 제공하고, 컨테이너 단계는 `git`을 설치한다.
  스위트 본문은 이제 `.ci/pytest-wheel.sh`에 있으며 모든 컨테이너 레그가 이를 호출한다.
  이전에는 `docker run ... sh -c '...'`에 포함되어 있었고, 주석의 아포스트로피(`container's`)가
  인용을 끝내 Alpine은 `apk add`만 실행했으며 나머지는 러너의 toolcache Python이 실행했다.
  스크립트는 venv 바이너리를 경로로 직접 호출한다. Alpine의 `/bin/sh`에는 `source`가 없다.
  이 입력 중 하나라도 사라지면 `tests/test_publish_toolchain.py`가 실패한다.

#### 성능

- **리더는 바이트당 일을 줄였다** — TOML 리더는 스칼라마다 한 번만 할당하고(`from_toml_medium`
  148 → 97회, 16,362 → 15,612바이트), JSON 리더의 이스케이프 없는 문자열은 소스를 빌린다
  (74 → 48회, 9,675 → 9,444바이트). JSON과 TOML 스캐너는 문자 단위로 디코딩하지 않고 런을
  복사하며, 스칼라 리졸버는 자를 것이 없는 텍스트를 자르지 않는다.
- **라이터 할당이 줄었다** — TOML 라이터는 임시 객체를 포맷하지 않고 버퍼에 직접 쓰며(같은
  문서에서 53.2M 명령, `to_json_medium`은 21.3M), YAML 라이터는 한 번의 바이트 스캔으로 인용
  여부를 정하고, 태그 출력은 테이블 기반이 되었으며 이스케이프는 할당하지 않는다.
- **이벤트 스트림이 스칼라 텍스트를 빌린다** — `StreamEventType::Scalar`가 granit의 `Cow`에서
  값을 복사하지 않는다. 이벤트 시나리오에서 −9.192%를 측정했다.
- **명령 수 플로어, 고정된 `rustc 1.97.1`에서 측정** — 커밋된 베이스라인 대비
  `from_toml_medium` −9.80%, `from_json_medium` −13.37%, `from_jsonc_medium` −11.28%,
  `from_json5_medium` −9.61%. 릴리스 wheel은 플로어를 측정한 것과 같은 컴파일러로 빌드된다.

</details>

### [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq 동등 플래그 · JSONC/JSON5 입력 · diff/merge · Release 자동화</summary>

#### 추가

- **pyq CLI 패리티 플래그** — `pyq fmt`에 `--indent N`(블록 들여쓰기, 기본 2), `--width N`(스칼라
  소프트 줄바꿈 열폭, 0이면 비활성), `--sort-keys`(직렬화 단계 문서 전체 키 정렬),
  `-i/--inplace`(파일 그 자리 다시 쓰기) 추가, `pyrs-yaml-core::SerializeOptions`의 모든 옵션을
  노출하고 Python CLI의 `fmt --indent`와 맞춤. `pyq to-json`에 상호 배타적인 `--jsonc` / `--json5`
  방언 출력을 추가하여 `pyrs-json`의 주석 보존·JSON5 표기 직렬화기 (`to_jsonc_text*`,
  `to_json5_text*`)를 연결.
- **pyq JSONC/JSON5 입력 방언** — `Format` 열거형에 `--input jsonc|json5` 추가(자동 감지는 `.jsonc`
  / `.json5` 확장자도 인식). `pyrs-json` 네이티브 방언 파서를 거쳐 주석과 JSON5 표기가 AST에 실리며,
  `to-json --jsonc`과 결합하면 한 명령으로 주석 보존 JSONC→JSONC 왕복이 됩니다. `--all-docs`는 단일
  문서 방언에서 안정적 오류 메시지로 거부.
- **`pyq diff` / `pyq merge`** — 네이티브 CLI에 시맨틱 문서 비교와 오른쪽 우선
  딥 병합(yq `*+` 형태) 추가. `diff`는 두 AST를 순회하며 해석된 값·구조·태그만
  비교(주석/인용/레이아웃은 나타나지 않음)하고 `-`/`+`/`~` 경로 행을 출력하며,
  동일 0·차이 1로 종료. `merge`는 mapping을 재귀 겹쳐 병합하고 sequence는
  뒤로 추가(`--replace-arrays`로 통째 교체), 왕복 YAML로 출력. 두 명령 모두
  `--input`/확장자 판정으로 지원 방언을 읽을 수 있습니다.

#### 변경

- **GitHub Release를 `publish.yml`이 자동 생성** — 지금까지는 publish 마다
  수동으로 `gh release create`를 실행해 왔는데, 잊기 쉬운 단계가 하나 늘고
  공개된 버전과 tag가 어긋날 여지도 생겼습니다. 이제 `release` job이 같은
  `refs/tags/` 조건으로 `uv publish` 성공 후 `gh release create`를 실행하고,
  릴리스 노트를 자동 생성해 빌드된 wheel을 첨부합니다. 노트와 산출물 모두
  PyPI에 공개된 것과 동일한 tag에서 나옵니다. `workflow_dispatch` 실행의
  동작은 그대로입니다(PyPI 공개도 Release 생성도 하지 않음).
- **`README.md` / `README.zh-CN.md`에 네이티브 `pyq` CLI 문서화** — 두 README의 Python CLI 절 옆에
  `pyq` 절을 추가했습니다. 체크아웃에서의 설치 방법, 실제 실행 가능한 예시 3개, 전체 명령 목록을
  담고 자세한 내용은 pyq 가이드로 연결합니다.

#### 수정

- **`pyrs-json` 모듈 문서** — 옛 설명은 "주석은 읽을 때 버려지고 다시 출력되지 않는다"였으나, #122
  이후 주석은 AST 주석 슬롯에 실려 JSONC/JSON5 직렬화기가 복원합니다.

</details>

### [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>라이브러리 간 패리티 · `load_json` · TOML 1.1과 toml-test · JSON5/JSONC · pyq ·
성능</summary>

#### 추가

- **JSONC block-comment 핫스팟 벤치** — objective §테스트 커버리지 5 가 “block-comment”을 핫스팟
  표본으로 지정. 이전에는 inline `//` 만 측정했다. 새 fixture 가 `test_load_jsonc_block_comments` 로
  구동: 50 pair + header/footer, 각 항목이 독립적인 `/* item N */`를 갖고 말미는
  `value /* trailing */`를 보유, 블록 순회 회귀를 CodSpeed 로 시각화.
- **YAML 의 PyYAML + ruamel.yaml 크로스 라이브러리 대조** — objective §테스트 커버리지 3 이 둘을
  oracle 로 지정. 이전에는 `test_benchmark_crosslib.py` 벤치 + 특성 support printout 만 있었다.
  `tests/test_yaml_crosslib.py`: 정규 문서 20 × 대조 면 5 + 문서화된 분기 2(duplicate-key 엄격성,
  YAML 1.1 전통 bool schema-scope) = 122 테스트. optional dep 은 skipif 로 자동 강등.
- **`load_toml` tomlkit 크로스 라이브러리 대조** — objective §테스트 커버리지 3 이 tomlkit 을 oracle
  로 지정. 이전에는 벤치만 있었다. `tests/test_toml_crosslib.py` 24 건: 정규 구조물 11 에서 pyrs /
  tomlkit / tomllib 셋이 일치, `>i64` 거부는 사양 준거(TOML v1.0 §Integers: 64bit signed)로 고정,
  `-2^63` 경계(PR #174 수정) 확인. optional dep, skipif 자동 강등.
- **orjson 을 strict-JSON oracle 로** — objective §테스트 커버리지 3 “orjson 과 자리별 비교”는
  그동안 벤치만 있었다. 16 정규 문서 값 일치, 12 비정규 형식(주석, trailing comma, single quote,
  bare `NaN`/`Infinity`/`-Infinity`, hex, leading 0, `+.5`, `5.`) 양측 거부 단언. stdlib
  `json.loads` 는 `allow_nan=True` 하에서 bare literal 을 받아들이지만 orjson 은 거부 — RFC 8259
  oracle 로서 stdlib 보다 강함. optional dep, `skipif` 자동 강하.
- **CLI ↔ Binding 대칭 게이트(`tests/test_cli_binding_parity.py`)** — Pillar 1 의 “CLI 와 Python
  Binding 양쪽이 동일한 기능” 선언을 문서관에서 실행 가능한 계약으로 승격. CLI 등록 명령을 18 개
  고정 목록과 비교(cyclopts 의 `--help`/`-h`/`--version` 의사 명령 제외), 각 `to-X` / `from-X`
  동사에 `YamlDocument.to_X` / `from_X` / `load_X` 대응 필수. `load_*` 패밀리(json/jsonc/json5/toml)
  네 형제 대칭 단언, editing/validate/compliance 동사는 live Python API 에 매핑. 어느 한쪽
  드리프트는 CI 실패로surface.
- **`load_json` 속성 테스트 + CodSpeed 벤치** — Hypothesis(`test_load_json_matches_stdlib_json`,
  `test_load_json_matches_load_jsonc_on_strict_domain`)가 생성된 모든 정규 문서에 대해 STRICT loader
  와 `json.loads` 의 일치, 및 두 loader 의 strict 영역 문자 그대로 동의를 고정. 고속 경로 확대나 AST
  경로 드리프트는 속성 실패로surface. 3 개 CodSpeed wall-time 벤치(`test_load_json_large` /
  `_floats` / `_escapes`) 가 `load_jsonc` 샘플을 미러링하여 STRICT binding 레이어 자체를 회귀 추적에
  포함.
- **`load_json` (엄격) — `load_*` 패밀리 대칭 완성** — binding에는 이미 `load_jsonc` / `load_json5`
  / `load_toml` 이 있었으나 엄격한 RFC 8259 대응 함수가 빠져 있었습니다. `pyrs_yaml.load_json(s)` 는
  정규 입력에서 `json.loads` 와 문자 그대로 일치하며 JSONC/JSON5 확장(`//`, `/* */`, 후행 쉼표,
  single quote, bare `Infinity`/`NaN`, `0x…`)을 타입이 지정된 `YamlParseError` 로 거부합니다. 고속
  경로는 `load_jsonc` 와 `json_fast::try_load` 를 공유(비정규 바이트는 모두 bail → 문법 확대 리스크
  0); 거부 대상은 STRICT `from_json` AST 경로로 라우팅. 아래 CLI ↔ Binding 대칭 항목의 마지막 공백을
  메워 Pillar 1 이 완성됩니다. `pyrs_yaml.__init__` 에서 재내보내기 및 `__all__` 등재; `.pyi` 는
  `maturin generate-stubs` 로 재생성.
- **방언 writer 고정점 속성** — `fmt_pbt.rs` 헤더가 약속한 writer 고정점(writer 출력을
  재파싱→재직렬화하면 문자 그대로 동일)이 구현돼 있지 않았습니다. 4개의 proptest가 이제
  JSON/JSONC/JSON5/TOML에서 이를 지키며(유일한 입력 필터 `json_object_domain`는 서로 다른 키가 같은
  JSON 이름을 만드는 수작업 AST를 제외 — RFC 8259 object 영역 밖), 이 게이트는 즉시 3개의 실제 주석
  충실도 결함을 발견했습니다(아래 수정 참조).
- **핫스팟 벤치 코퍼스** — 7개 CodSpeed wall-time 벤치가 역사적으로 취약한 직렬화 경로를 대상으로
  합니다: YAML 블록 스칼라 문서(6종 헤더 표기 `|`/`|-`/`|+`/`>`/`>-`/`>+`) 및 주석 밀도 문서, TOML
  multiline 문자열/진법 정수/underscores/지수/날짜시간, JSON5 특수 수치 형식(16진, `+.1`, `5.`,
  `Infinity`, `NaN`, single quotes, trailing commas). 픽스처는 `tests/data/yaml_samples.py`, 벤치는
  `tests/test_benchmark_api.py`. 이 코퍼스 구축 과정에서 중첩 블록 스칼라 들여쓰기 버그가
  발견되었습니다.
- **텍스트 수준 재파싱 게이트(`prop_output_always_parses`)** — Rust proptest 스위트가 생성된 모든
  AST의 직렬화 출력이 파서에서 재파싱 가능함을 주장합니다. AST 대 AST 왕복 프로퍼티는 재파싱 불가
  형상(`try_roundtrip`이 `None`)을 조용히 건너뛰었음. 새 게이트가 첫 실행에서 6개 실제 버그를
  발견했으며, 각각 targeted Rust 단위 테스트와 Python 회귀 클래스(`TestNestedBlockScalarIndent`)로
  고정.
- **toml-test 적합성 하네스** — `tests/test_toml_test_suite.py` 가 공식
  [toml-test](https://github.com/toml-lang/toml-test) 를 `test_yaml_suite.py` 가 YAML 스위트를
  실행하는 것과 동일한 방식으로 실행합니다: 추적되지 않은 로컬 아티팩트, 부재 시 `skipif`, 실측 하한
  게이트, 그리고 디코딩 비교를 위한 타입 태그 어댑터.
- **TOML 시간 타입 정상 디코딩** — 날짜 전용과 시간 전용은 각각 서로 다른 `!date`/`!time` 태그를
  가져(날짜시간은 `!timestamp` 유지) `date`/`time.fromisoformat` 를 사용합니다. toml-test 가 맨
  시간(`07:32:00`), 초 생략 시간(`13:37`), 소문자 구분자 날짜시간(`1987-07-05t17:45:00z`)이 유효한
  TOML 에서 `ValueError` 를 던지는 것을 발견. `!time` 은 생략된 초를 채우고 `!timestamp` 는 소문자
  `t`/`z` 를 정규화합니다.
- **TOML 제어 문자 엄격성** — 기본·자구·다중줄 문자열 내부에서 raw C0 제어 코드(NUL, FF, DLE, US
  등)와 DEL(U+007F) 을 거절합니다(탭 및 다중줄 줄바꿈만 허용). toml-test 의 `invalid/control` 이 13
  건의 오인식 문서를 발견. 코멘트 본문·bare CR 검사는 후속 항목.
- **TOML 숫자 리터럴 엄격성** — 선행 0 을 가진 10 진수(`01`, `-01`), 진수 접두 정수에 부호(`+0x1F`,
  `-0b101` — `signed-int` 는 10 진수 전용), 후행/이중 언더스코어(`1_`, `1__0`) 를 거절합니다.
  toml-test 의 `invalid/integer`+`invalid/float` 이 23 건의 오인식을 발견(총 71->48). 기존 “진수
  정수에 부호 허용” 은 사양 위반이었습니다.
- **TOML 인라인 테이블 키 충돌 엄격성** — 인라인 테이블은 정의된 경로와 동일·확장·피복 관계인 점
  키(`{ a = 1, a.b = 2 }`, `{ a.b = 1, a.b.c = 2 }`)를 거절합니다. 형제
  경로(`{ a.b = 1, a.c = 2 }`)는 허용. toml-test `invalid/inline-table` 의 duplicate-key/overwrite
  가 발견(오인식 총계 48->39).
- **TOML 비 ASCII 문자열 충돌 수정** — 기본 및 다중줄 기본 문자열 파서가 바이트 단위로 전진하며
  멀티바이트 입력(U+00A0 등)에서 문자 중간을 슬라이스해 panic 하던 것을 문자 단위로 소비하도록 수정.
  toml-test 로 발견, #153 의 한줄/JSON 판을 보완.
- **형식 퍼징 + 견고성 수정** — 새 `proptest` 속성 테스트가 TOML/JSON/JSONC/JSON5 파서와 라이터를
  퍼징(no-panic + 재파싱 가능성). 발견·수정: TOML 및 JSON 문자열 파서의 중각 문자 슬라이스 panic, 및
  JSONC/JSON5 인라인 `//` 주석이 후속 `,`/`}`를 삼키는 버그.
- **YAML merge/별칭 속성 퍼징** — 정상 형태의 앵커/별칭/병합-키 문서(단일 별칭, 별칭
  시퀀스, 인라인 맵이 들어간 시퀀스, 인라인-맵 병합, #166 이 거절하는 스칼라/널 병합
  소스)를 새로 생성하며 자기 참조 앵커와 반복 별칭 참조도 포함합니다. 기존에는 모든
  속성 테스트가 `arb_custom_node()` 를 썼는데 이는 `meta.anchor` 만 내고 `Alias` 노드는
  절대 만들지 않아 별칭 해석과 병합 전개 경로(#163/#166 의 바로 그 구조 등급)가
  프로세스 내 퍼징 대상이 아니었습니다. `prop_merge_alias_never_panics` 은 파싱 + 병합
  해석이 panic 이나 네이티브 스택 오버플로 없이 수행되고 파싱된 트리는 재직렬화·재파싱에서
  안정적임을 단언합니다.
- **CLI 형식 대등성** — CLI 에 `to-toml`/`from-toml`, `to-jsonc`/`from-jsonc`,
  `to-json5`/`from-json5` 추가(기존 `to-json`/`from-json` 본뜬). 바인딩이 처리하는 모든 형식을
  명령줄에서 사용 가능.
- **JSON 문자열 이스케이프 고속 경로(성능)** — `load_jsonc` 가 단순한 2바이트 이스케이프 8종을
  인라인 디코딩하여 문서 전체를 AST 경로로 피하지 않습니다. 이스케이프가 있는 JSON 이 고속 경로에
  탑니다(AST 경로 대비 약 15 배 빠름). 값은 `json.loads` 와 일치; `\u`·잘못된 이스케이프는 계속 AST
  경로 경유.
- **JSON 부동소수점 고속 경로(성능)** — `load_jsonc` 가 정규 부동소수(소수/지수)를 문서 전체를 AST
  경로로 피하지 않고 곧바로 Python 객체로 해석합니다. 값은 `json.loads` 와 완전히 일치(올바르게
  반올림된 parse). 새 bench 가 이 분기를 회귀 추적합니다.
- **JSON 문자열 직렬화 고속화(성능)** — 이스케이프가 필요 없는 문자열은 문자 단위 UTF-8 재인코딩
  대신 한 번의 `push_str`로 일괄 복사합니다. 문자열이 많은 `to_json` 은 약 35% 빨라짐(41→27 ns/건),
  출력은 바이트 동일.
- **`YamlDocument.to_toml()`** — 문서가 `to_json`/`to_jsonc`/`to_json5`와 동일하게 AST에서 TOML 을
  바로 출력합니다. `to_toml(doc.to_yaml())` 의 직렬화→재파싱 왕복이 불필요해지고 출력은 바이트 동일.
  라이터 자체는 `tomli_w` 보다 약 4.3배 빠름.
- **`to_json` 네이티브 직렬화기(성능)** — `YamlDocument.to_json`이 `to_dict()` + `json.dumps` 이중
  변환을 멈추고 네이티브 엔진을 사용. ASCII 는 바이트 단위 동일, 약 10배 빨라짐(1200건
  ~1450µs→~120µs, `json.dumps`를 앞섬). 비ASCII는 `\uXXXX` 대신 원시 UTF-8(`to_jsonc`/`to_json5`와
  일치)로 출력되며 유효한 JSON 유지.
- **JSON 키 직접 출력(성능)** — 라이터가 매핑 키를 출력 버퍼에 바로 기록(키별 `String` 할당 폐지).
  컴팩트 `to_json` 이 약 2배 더 빨라짐(~120µs→~60µs), 바이트 동일, 직렬화가 현장에서 #2(orjson만
  상위).
- **JSON 로드 고속 경로** — `load_jsonc`가 정준 strict JSON을 `CustomNode` AST 없이 직접 Python
  객체로 변환(실측 약 5-6배 빨라 stdlib `json.loads`를 앞섬). 비정준 입력(실수/이스케이프/주석/범위
  초과 정수/후행 쉼표)은 일반 경로로 폴백하여 값과 오류가 불변.
- **TOML 다중 줄 문자열 무결성** — TOML 다중 줄 문자열을 `ScalarStyle::Literal` YAML 블록으로
  투영(텍스트 허브 왕복 유지)하여 `to_toml`이 이스케이프 한 줄로 붕괴시키지 않고 `"""` 블록으로
  재출력한다. 값은 바이트 단위로 왕복하고 출력은 멱등이며 한 줄 문자열은 한 줄로 유지된다. 기존
  `Literal` 재사용으로 AST 구조 변경 없음.
- **TOML 문서 수준 주석 무결성** — `to_toml`이 루트 매핑의 선행 주석을 출력하게 되어, 문서 맨 앞의
  독립 `# 주석`이 TOML→허브→TOML 왕복에서 유지된다(JSON 라이터의 `emit_root_leading`에 해당).
  네이티브 TOML 파싱과 주석 없는 문서는 영향 없다.
- **JSON5 Unicode 식별자 키** — 큰따옴표 없는 객체 키가 ASCII 제한을 넘어 완전한 Unicode `ID_Start`
  / `ID_Continue` 집합을 받아들인다. `from_json5`/`load_json5`가 `{ é: 1, 名: 2, हिन्दी: 3 }`를
  파싱한다. rustc 자체 어휘 분석이 쓰는 `unicode-ident` 테이블로 스크립트별 정확히 적합(식별자 중간
  결합 문자 포함). JSON5 모드에서만 켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 따옴표를
  요구한다. `\uXXXX`의 고아 UTF-16 서러게이트는 계속 거부(Rust `String`이 무손실로 표현 불가).
  의존성 1개 추가(`unicode-ident`).
- **JSON5 Unicode 구조 공백** — `from_json5`/`load_json5`가 RFC 8259의 4개(탭/스페이스/줄바꿈/CR)
  외에 JSON5가 추가한 공백을 토큰 구분자로 받아들인다: 수직 탭, 폼 픽, NBSP(U+00A0), 모든 Unicode
  `Zs` 구분자, LS/PS 줄 종결자(U+2028/U+2029), ZWNBSP(U+FEFF). `std`의 `char::is_whitespace`(JSON5가
  공백으로 보지 않는 NEL U+0085 제외)에 U+FEFF를 더해서 구현, 신규 의존성 없음. JSON5 모드에서만
  켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 전부 거부한다.
- **JSON5 줄 연속과 `\'` 이스케이프** — 큰따옴표 JSON5 문자열이 JSON 집합 밖의 두 이스케이프를
  받아들인다: 줄 종결자 바로 앞의 역슬래시(줄 연속으로 둘 다 제거)와 이스케이프된
  작은따옴표(`\'`→`'`). JSON5 모드에서만 켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 둘 다
  거부한다. #125의 작은따옴표 처리를 반영해 JSON5 문자열 무결성을 완성한다.
- **JSON5 문자열 이스케이프 `\v`와 `\0`** — `from_json5`가 수직 탭(`\v`)과 NUL(`\0`)을
  큰따옴표/작은따옴표 문자열 모두에서 받아들인다. 엄격 JSON / JSONC는 종래대로
  거부한다. #120(수치)·#124(수치 시맨틱)과 함께 JSON5 문법을 완성한다.
- **load_json5의 JSON5 수치 시맨틱** — `load_json5`가 JSON5 전용 수치 형식(`0x1F`→31, `+7`→7,
  `5.`→5.0, `Infinity`/`NaN`)을 새로운 `Schema::Json5`로 실수 값으로 해석한다. 엄격 JSON/JSONC
  로더는 불변이며 `to_json5_text`는 원본 표기 그대로 출력한다.
- **JSON5/JSONC를 공개 API에서 사용 가능하도록** — `pyrs_yaml.from_json5` / `load_json5`, 그리고
  `YamlDocument.to_jsonc()` / `to_json5()`(네이티브 엔진 경유로 주석과 JSON5 스타일 보존). 함께
  도달성 결함도 수정: `from_jsonc` / `load_jsonc`가 `pyrs_yaml` 패키지에 재수출되지 않아
  `AttributeError`가 났는데 이제 `__all__`에 포함된다. `to_jsonc`/`to_json5`는
  `emit_root_leading`으로 문서 수준 standalone 주석을 보존한다. `test_benchmark_api.py`에 JSON 계열
  벤치마크를 추가했다.
- **JSON5 라이터(`to_json5_text` / `to_json5_text_pretty`)** — 계약 B 2단계. AST를 JSON5로 직렬화해
  파서 보존한 한따옴표 문자열과 `0x…`/`.5`/`+7`/`Infinity`/`NaN` 숫자 형식, 그리고 `//` 주석을
  복원한다. 키는 항상 인용부호를 붙인다(무손실). 내부에서 `Mode`(Json/Jsonc/Json5)를 공유하며,
  엄격·JSONC 출력은 변하지 않는다.
- **파서에 JSON5 수치 형식 추가** — `from_json5`(새 `allow_json5_numbers` 축)가 십육진(`0xDECAF`),
  전/후 소수점(`.5`, `5.`), 부호 `+`(`+7`), 선행 0(`07`), 그리고 `Infinity` / `NaN` / `-Infinity`를
  받아들인다. 각 형식은 원문을 보존해 훗날 JSON5 라이터가 그대로 재현할 수 있다. STRICT / JSONC는 이
  축을 OFF로 두어 종래대로 거부한다. `from_jsonc`의 낡은 "주석은 버려진다" doc도 정정했다.
- **TOML 인라인 테이블 내부 주석 보전** — PR #119가 인라인 테이블 내 `# ...` 주석을 잡아 (멤버 위
  독립 행 -> leading, 값 같은 행 후 -> trailing) IR 로 전달해 워드트립이 도는 혀 주석이 손실되지
  않습니다. 장식이 없는 인라인 테이블은 깁깐한 한 줄 형식을 유지하며, 배열 내 노안된 장식 테이블은
  멀리 줄로 승급됩니다. 동시에 #114 재재 버그 (`skip_all_blank`가 standalone 주석 자신의 냄배를 단련
  줄로 오인)을 수정했습니다.
- **YAML 리시버가 standalone 주석을 `decor.leading_comment` 에 기록** — PR #117b 로 마지막
  엔진(granit-parser 리시버)이 #114 / #115 에서 도입한 새 슬롯으로 이전했습니다. scalar / mapping /
  sequence 의 standalone note 가 `NodeMeta::decor.leading_comment` 에 실리고 오래된
  `comment(standalone = true)` 에는 쓰지 않습니다. #117 의 규정화로 수작업 fixture 는 여전히
  동등하고 `CustomNode::remove_comment` 가 **두 슬롯**을 원자적으로 비우므로 Python 의
  `Node.remove_comment()` 는 YAML 문서에서도 변화 없이 동작합니다.
- **슬롯 간 standalone 주석 규정화 + Python `Node.leading_comment`** — `NodeMeta::eq` / `Hash`가
  standalone 주석을 새로운 `leading_comment` 슬롯(TOML / JSON 엔진)이나 오랜
  `comment(standalone = true)` 슬롯(YAML 리시버 + 수곤픽스테 기본)에 있는지 여부와 관련하지 않고
  동일 개념으로 처리합니다. setter / remover는 두 슬롯에 원자적으로 작용하며 YAML 서리라이저는
  규정화된 뷰를 읽어 `to_yaml(toml_ast)`의 leading note가 사라지지 않습니다. Python의
  `Node.leading_comment` getter / setter / remover가 `Node.comment`를 미러링하며 TOML / JSONC 소스의
  standalone 주석을 처음으로 Python 호출자에게 노출합니다.
- **TOML 1.1.0 문법** — `from_toml`이 TOML v1.1.0（2025-12-18 공개）에 맞춰 파싱합니다. 다음 두 가지
  추가: **(A1)** 인라인 테이블의 줄바꾸기 및 후행 슜표 허용, **(A2)** 기본 문자열에서 `\xHH` 바이트
  특별문자（0x00..=0xFF）, **(A3)** `\e` = U+001B, **(A4)** time / date-time 항목에서 초
  생략（`t = 14:15` / `dt = 2010-02-03 14:15`）. 엄격 1.0.0 사용자를 위해 `TomlDialect::V1_0` 과
  `from_toml_v1_0` 입력 경로를 별도 두멍니다. 1.0.0 문서는 두 방언 모두 동일한 결과를 냅니다. 동시에
  space 구분 date-time 감지 인덱스의 off-by-one 버그（`T` 가 없는 date-time이 1.0 모드에서도
  인식되지 않았는）를 함꺼 수정했습니다.
- **JSON dual-slot 주석 보전** — JSONC 파서가 독립 행 `// ...` 주석을 #114 가 추가한
  `leading_comment` 슬롯에 기록하고, 같은 행의 `// trailing` 은 `comment` 에 남깁니다. 개체 멤버와
  배열 요소가 같은 노드에서 두 주석을 모두보존할 수 있게 되며, 이는 #112 의 단일 슬롯 모델로는
  표현할 수 없는 형태입니다. `to_jsonc_text_pretty` 는 `leading_comment` 를 우선 읽고 hand-built
  fixture 를 위해 `comment` (`standalone = true`) fallback 을 보존합니다. 엄격 JSON (`to_json_text`)
  의 동작은 변하지 않습니다 (여전히 `//` 미출력).
- **TOML 빈 행 + dual-slot 주석 보전** — `NodeMeta`에 `leading_comment: Option<Comment>` 와
  `blank_before: bool` 이 추가되었습니다 (둘 다 구조적 `Hash` / `PartialEq` 에서 제외). 섹션 헤더나
  AOT 요소가 `]` 뒤의 행미 주석과 그 위의 독립 행 주석을 동시에 보존할 수 있어 서로 밀어내지
  않습니다. `to_toml` 은 소스의 빈 행 구역을 재현하며 (`a = 1\n\nb = 2` 가 바이트 안정적으로 왕복),
  문서 첫 페어 앞에는 빈 행을 쓰지 않습니다. 수작업 빌드 및 YAML 출처 노드는 writer 의 fallback
  읽기로 계속 동일하게 렌더링됩니다.
- **JSON5 방언** — `pyrs_yaml_core::json::from_json5(text)`과
  `from_json_with_options(text, JsonParseOptions)`가 JSON5의 4개 축 (후행 쉼표, 한따옴표 문자열,
  따옴표 없는 식별자 키, 행/블록 주석)을 모두 허용합니다. 각 축은 개별 토글 가능; `STRICT`, `JSONC`,
  `JSON5` 상수를 기본값으로 제공합니다.
- **JSONC/JSON5 바인딩과 CLI** — `pyrs_yaml.from_jsonc(str)`은 YAML 텍스트를 반환하고,
  `pyrs_yaml.load_jsonc(str)`는 Python dict / list를 바로 반환합니다. `pyq from-json`에 `--jsonc` 및
  `--json5` 플래그를 추가해 `tsconfig.json` / `settings.json` 이 verb 파이프라인으로 바로
  이어집니다.
- **JSONC 주석 보존** — `from_jsonc`이 수집한 `// 행` 및 `/* 블록 */` 주석을 AST의
  `NodeMeta::comment`에 부착합니다 (독립 부분은 key 노드, 행미 부분은 value 노드). PR #109에서
  도입한 TOML 모델과 동일합니다. 쌍을 이루는 `to_jsonc_text(node)` /
  `to_jsonc_text_pretty(node, indent)` 가 원 위치로 재출력하며, 블록 주석은 AST에 본문만 저장되므로
  출력 시 `//`로 정규화됩니다. 엄밀 writer `to_json_text` / `to_json_text_pretty`는 바이트 단위로
  불변이므로 소비 측은 보존 여부는 선택할 수 있습니다.
- **pyq 다중 문서 편집** — `-A/--all-docs`가 모든 편집 명령(set/delete/rename/
  move/append/insert/sort-keys)과 `to-json -A`(JSON 배열, Python 대응)를 커버.
  각 문서는 스트림 내 자기 구간에 대해 splice(`MultiDocEditor` + `DirtyUnit::
  shifted`): 건드리지 않은 문서와 모든 `---` 구분자는 바이트 단위로 유지,
  경로 미매칭 문서는 스킵(Python try/skip 의미 일치, 전원 미매칭은 오류),
  레이아웃 오염 문서는 단독 폴백으로 이웃에 영향 없음.
- **JSONC 파싱** — `pyrs_yaml_core::json::from_jsonc(text)`와
  `from_json_with_options(text, JsonParseOptions)`가 공백이 허용되는 아무 위치에서나 `// 행` 과
  `/* 블록 */` 주석을 허용합니다 (TypeScript `tsconfig.json`, VS Code `settings.json` 방언). 주석은
  제거되며 보존되지 않습니다. 후행 쉼표와 JSON5 고유 구문은 계속 거부되므로 허용 언어는 RFC 8259의
  엄격한 상위 집합으로 남습니다. `from_json`의 기본 동작(엄격 모드)은 바뀌지 않습니다.
- **TOML 주석 보존** — 파서가 페어나 section 헤더 위에 별도 행으로 나타나는 `# ...` 주석과 행미 주석
  (`key = value # ...` / `[name] # ...`) 을 모두 캡처하여 공유 AST의 `NodeMeta::comment`에
  부착합니다 (독립 부분은 key 노드, 행미 부분은 value 노드). `to_toml(from_toml(src))`가 원 위치로
  재출력하므로 `pyq edit`과 `YamlDocument.set()`은 TOML 왕복 중 주석을 더 이상 제거하지 않습니다. 빈
  행 구분은 설계 문서에 따라 writer 기본 레이아웃 그대로.
- **TOML 숫자 원문 표기 보존** — `to_toml(from_toml(src))`가 16진 (`0xDEADBEEF`)·8진 (`0o755`)
  정수의 원문 표기와 지수형 부동소수점 (`1e10`、`-3.14e-2`)을 그대로 유지합니다. 밑줄 구분자、명시적
  `+` 부호、 음수 radix 형식 (`-0x1F`)、2진수 (`0b101`)는 YAML Core가 다시 읽을 수 없어 10진수로
  정규화됩니다. 이로써 공유 AST와 YAML 파이프라인의 상호운용성을 유지합니다。주석 보존과 JSONC
  지원은 설계 문서에 따라 후속 PR에서 도입 예정。
- **pyq 기능 보완** — CLI가 Python CLI 기능면에 대응: `rename`/`move`/`append`/`insert`
  splice 편집, `validate`(파싱 검사 또는 `--schema rules.yaml` 스키마 언어 규칙 검증),
  `frontmatter`(`--body-out` 본문 분리), `get`/`fmt`/`to-json`의 `-A/--all-docs` 다중
  문서 지원. 배선 중 코어 엔진 버그 발견: `move_path`가 이동 대상 INSERT 단위만
  반환해 splice 텍스트에 이동 원본 사본이 잔류(폴백 시 눈에 안 탐). 이제 두 단위를
  반환하고 bindings는 배치 splice 경로로 적용.
- **`pyq` 필터 동사** — 매칭 스트림의 jq 스타일 구조화 후처리: `--select 'PATH OP LITERAL'`,
  `--sort-by PATH` / `--desc`, `--unique`, `--first` / `--last`, `--skip N` / `--take N`,
  `--join SEP`. `get`과 `from-*`에서 고정 파이프라인 `select -> sort -> unique -> slice` → `join`
  적용. 의도적으로 플래그 설계(표현식 없음): 술어는 미세 구문 1회 파싱(약 40줄), 타입 불일치는
  false(jq 전순서와의 알려진 차이), 즉시 시작 유지.
- **`pyq completion`** — bash·zsh·fish·PowerShell 셸 자동 완성 스크립트 출력
  (`pyq completion bash > ...`). `clap_complete` 구현(승인된 CLI 크레이트
  의존성 추가. `pyrs-yaml-cli` 바이너리 내에 완결되며 Python 배포에 영향 없음).
- **`pyq sort-keys`** — 임의 경로(`$`는 루트)의 매핑 키를 정렬. `set`/`delete`와
  동일한 코어 plan/splice 엔진을 거쳐 그 자리 다시 쓰기 또는 표준 출력 모두
  지원하며, Python CLI의 `sort-keys`와의 대응 격차를 해소.
- **커맨드라인 인터페이스** — 새로운 `pyrs-yaml` 명령(`pip install "pyrs-yaml[cli]"`로 옵트인,
  Python 3.10+ 필요)을 통해 라이브러리의 핵심 기능을 터미널에서 사용할 수 있습니다:
  `fmt`(주석/앵커/순서를 보존하는 라운드트립 재포매팅), `get`(JSONPath 쿼리,
  `--format yaml|json|text` 지원), `set` / `delete` / `rename`(경로 기반 편집, `--inplace`,
  `--string`, `--create-missing` 지원), `validate`(CI 친화적 종료 코드), 그리고 `to-json` /
  `from-json` 변환. 모든 명령은 `-`로 stdin을 읽고 기본적으로 stdout에 출력합니다. 구현은 순수
  Python (`python/pyrs_yaml/cli/`)이며 [Cyclopts](https://github.com/BrianPugh/cyclopts)를 선택적
  extra로 사용하므로, 기본 설치는 추가 의존성 없음과 Python 3.8 지원을 유지합니다.
- **CLI 확장** — `sort-keys`(경로 위치의 매핑 키 정렬), `move`(서브트리를 기존 대상으로
  이동), `frontmatter`(Markdown 프론트매터를 YAML로 추출, 본문 분리 지원),
  `compliance`(YAML Test Suite 리포트, `--json` 지원) 명령이 추가되었습니다.
  `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/`validate`/`to-json`에
  `-A/--all-docs` 다중 문서 모드가 제공되고, `validate`는 상호 배타적인
  `--schema <이름>`과 `--schema-file <경로>`로 분리되었습니다. 문서화되지 않았던
  `python -m pyrs_yaml.compliance` 진입점은 서브커맨드로 대체되어 제거되었습니다.
- **`YamlStream` import 가능** — API 문서와 타입 스텁대로 `from pyrs_yaml import YamlStream`이
  동작합니다. 그동안 이 클래스는 `YAML().load_stream*()`의 반환값으로만 얻을 수 있고 네이티브
  모듈에서 내보내지 않았습니다.
- **CLI `move --all-docs`** — `move`가 `-A/--all-docs`를 지원합니다. 양쪽 경로가 해석되는
  모든 문서에 서브트리 이동을 적용합니다(`set`/`delete`/`rename`과 동일한 의미론). 이제
  다중 문서 플래그가 모든 편집 명령을 커버합니다.
- **문서↔API 일관성 가드** — `tests/test_docs_api.py`가 모든 언어 문서 페이지의
  `pyrs_yaml.…` 속성 체인, `import pyrs_yaml…`, `from pyrs_yaml … import …` 참조를
  훑어 실행 시간에 존재하지 않는 심볼을 참조하면 실패합니다(약 965개 선언 검사).
- **선택적 서드파티 유형 플러그인** — `!duration`(`pendulum.Duration`),
  `!arrow`(`arrow.Arrow`), `!ulid`(`ulid.ULID`)는 해당 라이브러리가 설치되어 있을 때
  자동으로 등록됩니다(`python/pyrs_yaml/plugins/_builtin.py`의 `_register_third_party`).
  각 플러그인은 고유한 태그를 사용하므로 기존 `!timestamp` / `!date` / `!uuid` 핸들러에
  영향을 주지 않습니다. 표준 라이브러리 `timedelta`가 `!duration`에 매칭되지 않습니다.
- **pydantic-settings YAML 소스** — `PyrsYamlConfigSettingsSource`
  (`python/pyrs_yaml/settings.py`)는 `pydantic_settings.YamlConfigSettingsSource`의 드롭인
  대체품으로, PyYAML 대신 pyrs-yaml(YAML 1.2 코어 스키마)로 파싱합니다. 지연 내보내기되므로
  `import pyrs_yaml`에 pydantic-settings가 필요 없습니다. `pip install "pyrs-yaml[settings]"`로
  설치합니다(Python 3.10+). `dump_pydantic`과 `parse_as`도 동일한 모듈 수준 `__getattr__` 지연
  내보내기 패턴으로 변경되었습니다.
- **`pyq` — Rust 네이티브 CLI 크레이트** — `crates/pyrs-yaml-cli` (워크스페이스 멤버, clap 기반)는
  `pyrs-yaml-core`를 jq/yq 스타일 CLI에 직접 연결하여 런타임 Python이 필요 없습니다: `fmt`(주석 보존
  라운드트립), `get <path>`(JSONPath-lite, `--json`/`--raw` 지원), `set <path> <value>` 및
  `delete <path>`(yq 스타일 편집, `--create-missing` 과 `-i/--inplace` 파일 쓰기 지원; 출력은
  라운드트립 직렬화기 경유로 주석과 값 스타일 보존), `to-json`(순서 보존), `to-toml`, 도입 명령
  `from-json` / `from-toml` / `from-ini`. 입력 형식은 확장자로 판정(`--input`으로 덮어씀), stdin은
  `-`, 실패 시 core의 안정적인 오류 텍스트로 비정상 종료.
- **TOML 및 INI 교환 형식** — YAML을 유일한 편집 가능 표현으로 하는 허브-스포크 멀티 형식 지원:
  `from_toml`/`to_toml`로 TOML 텍스트 ⇄ YAML 텍스트 변환 (Rust `toml_edit`), `load_toml`은 TOML을
  Python 값으로 직접 읽기 (datetime은 내장 `!timestamp` 플러그인 경유, TOML 문자열은 재해석되지
  않음), `load_ini`는 표준 라이브러리 configparser로 INI 읽기(엄격 모드·읽기 전용). TOML 출력은 표현
  불가 구조를 안정적인 오류로 거부하며, 라운드트립 편집은 YAML 전용으로 유지됩니다.

#### 변경

- **granit-parser 1.1 → 1.3** — YAML 이벤트 파서을 1.1.0에서 1.3.0으로
  업그레이드했습니다. 1.x 라인 내 시맨틱 버전 호환 마이너 업그레이드입니다:
  1.2.0은 특이한 문서의 제한을 위한 선택적 `Options` 필드를 추가했고, 1.2.1은
  몇 가지 파싱 결과를 YAML 사양에 맞게 강화했으며, 1.3.0은 기본 구현이 있는 두 개의
  `Input` 메서드(`fetch_block_scalar_line`와 `take_quoted_scalar_ascii_chunk`)를
  추가하여 스캐너가 블록 및 따옴표 스칼라 바이트를 더 빠르게 건널 수 있게 했습니다.
  이 프로젝트는 `Parser::new_from_str`을 통해 파서를 소비하고 `EventReceiver` /
  `SpannedEventReceiver`만 구현하며 `Input`은 구현하지 않으므로 소스 변경이 필요
  없습니다—새 trait 메서드는 기본 구현으로 해결됩니다. 전체 스위트 통과:
  `cargo nextest run --all`(359), `pytest`(1436 + 43 numpy), 순수 Rust
  `--no-default-features` 빌드, 그리고 YAML 테스트 스위트 준수 게이트는 변경 없음.
- **네이티브 JSON 및 TOML 코어** — `serde_json` 및 `toml_edit` 의존성을 완전히
  제거했습니다. `pyrs-yaml-core`는 RFC 8259 JSON 엔진(바이트 수준 스캐너, 숫자는
  소스 표기를 그대로 보존하여 `from_json → to_json`이 바이트 안정적이고 큰
  정수/부동소수점 정밀도 손실이 없습니다. 라인/컬럼 오류에 타입을 부가하며,
  후행 쉼표·선행 0·고립 서러게이트· 이스케이프되지 않은 제어 문자·다중 루트 문서를
  엄격히 거부)와 TOML 1.0 전문어법을 완전히 다루는 엔진(bare/quoted/dotted 키,
  basic/literal/멀티라인 문자열, `_` 구분자를 허용하는 10/16/8/2진 정수,
  `inf`/`nan`/지수 포함 float, offset/local 날짜·시간·날짜시간)를 내장합니다.
  모든 거부는 granit-parser와 동일한 스타일로 0-indexed `line`/`col`을 담고
  `ParseError::Syntax`로 보고됩니다. 공개 API는 그대로이며 왕복 테스트와
  `tests/test_toml.py`가 새 엔진에서 모두 통과합니다.
- **내부 중복 코드 정리** — 벤치마크 fixture를 공유 블록 조립으로 변경, PyO3 경로 편집
  메서드를 기존 `apply_metadata_edit` 헬퍼로 위임, 반복된 파일 읽기/에러 매핑과 행 오프셋
  보일러플레이트를 공유 함수로 통합했습니다. 공개 동작 변경은 없습니다. jscpd로 측정한
  중복 코드 비율이 5.25%에서 3.45%로 감소했습니다.
- **`YamlDocument.validate()`가 컴파일된 validator를 캐시** — 스키마(JSON 텍스트 또는
  dict)에 대한 첫 검증 성공 시 `jsonschema` validator를 캐시해 이후 호출에서는 스키마
  파싱, 메타스키마 검사, validator 구축을 생략합니다. dict 스키마는 객체 식별자로
  키핑하며 딥복사 스냅샷 가드로 감지합니다: 제자리 변경은 다음 사용 시 `==`로 감지되어
  투명하게 재컴파일됩니다. 캐시 경로는 `exceptions.best_match(validator.iter_errors(instance))`
  를 raise하므로 `jsonschema.validate()`와 동일한 의미론. WSL 실측: `document_validate` −98%.
- **파서/직렬화 커널의 구조적 중복 제거** — mapping과 sequence 렌더링이 단일
  `write_container_node` 골격을 공유(출력 바이트 단위 동일, `serialize_*` 중앙값
  −5~11%); 단일/다중 문서 파싱 진입점이 동일한 `load_ast` 오류 계약을 공유;
  schema 해석 체인은 `bool_word`/`numeric_tail`을 공유하고 YAML 1.1은 core의
  null/bool 단어를 스칼라마다 다시 검사하지 않음; 앵커 등록(`register_anchor`)과
  독립/인라인 주석 분류(`is_standalone_placement`)를 AST·stream receiver 간 단일화.
  저장소 중복율 3.38% → 2.60%.

#### 수정

- **`\u` / `\x` 이스케이프 뒤 멀티바이트 문자에서 parser panic** — 고정폭 이스케이프 리더가
  `&self.text[pos..pos+width]`를 바이트 오프셋으로 슬라이스. JSON `\u` / TOML
  `\xHH`/`\uXXXX`/`\UXXXX` 뒤에 멀티바이트 문자가 오면 슬라이스가 문자 중간에 떨어져 프로세스 abort
  (#153 동족). 바이트 슬라이스 + UTF-8 검증으로 수정, 부정한 이스케이프는 깨끗하게 에러. 방언 fuzz
  로 발견, 두 parser 에 결정적 Rust 회귀 테스트로 고정.
- **비머지 값의 리터럴 `<<` 키가 조용히 폐기됨** — `load(safe_dump({"<<": None}))` 이 키를 잃고 `{}`
  반환. 머지 리졸버가 Null/Scalar 값에도 모든 `<<` 를 머지로 소비. YAML 에선 `<<` 값이 mapping
  별자/inline mapping/그 시퀀스일 때만 머지. Null/스칼라 `<<` 는 일반 키로 왕복 보존.
  Alias/mapping/sequence 경로(#166 자기참조 앵커 가드 포함) 무변경, yaml-test-suite 405/406 유지.
  왕복 속성 fuzz 가 비결정 포착(#163/#165/#166 급 결함), 결정적 Rust 회귀 테스트로 고정.
- **TOML 심층 네스팅이 네이티브 스택을 넘어 프로세스를 abort** — TOML 파서엔 중첩 예산이 없어(JSON
  은 `DEFAULT_MAX_DEPTH`, YAML 은 `parse` `max_depth`), `parse_value` →
  `parse_array`/`parse_inline_table` 이 무제한 재귀. 깊은 배열/인라인 테이블은 인터프리터를 즉시
  충돌시킴(검증: exit `0xC00000FD` STACK_OVERFLOW) — #166 YAML merge 오버플로의 TOML 판. 파서가
  `depth` 추적해 1000 초과시 `ParseError::MaxDepthExceeded` 반환, JSON 과 대칭. in-process Python
  경계 테스트 + subprocess crash canary + 대형 스택 Rust 테스트로 보호.
- **방언 writer/parser가 문서 수준 주석을 유실·오배치** — 고정점 속성이 잡아낸 3개 결함: (a)
  JSONC/JSON5 값 앞 파일 선행 `// note`가 inline으로 오분류(오프셋 0 앞 개행 없음)되고 writer가 주석
  달는 root가 아닌 첫 object 멤버에 선점돼, 빈 `{}`/root 스칼라에서는 완전히 소실; (b) JSON 계열 및
  TOML writer는 주석 본문를 그대로 출력하나 parser는 trim 저장—미trim 주석은 pass마다 후미 공백이
  요동—writer도 출력 시 trim해 첫 표기부터 안정; (c) 주석만 있는 TOML 문서(`# note` 뒤 키 없음)는
  재파싱 시 주석이 유실돼 빈 root가 `""`화—미소비 standalone 주석을 빈 root 테이블에 부착. 5개
  포맷의 leading 주석이 모두 문자 안정 고정점에 도달(3개 Rust 테스트로 고정).
- **중첩 블록 스칼라 본문이 부모 들여쓰기를 유지** — 중첩 키 아래 literal/folded 스칼라의 본문 줄이
  `b: |` 헤더 줄의 다음 단계가 아니라 0열부터 고정 1단계로 출력되어, 모든 중첩 형상의 왕복 텍스트가
  재파싱 불가하거나 오류 값이 됨. `block_base`(부모 줄 열 위치) 매개변수를 모든 출력 지점에 관철;
  7종 중첩 형식의 왕복이 완전 일치. TOML 핫스팟 벤치가 발견.
- **직렬화기가 재파싱 가능한 YAML만 출력** — 텍스트 수준 게이트가 발견한 5개 결함: (a) 개행 분기에서
  anchor/tag pre-emit이 child(스칼라/null/flow 용기)의 자체 헤더와 중복 → block 용기로 제한; (b)
  flow 용기 내 블록 스칼라(`[|`, `{k: >}`) 및 키 위치는 double-quoted로 강등; (c) own line을
  시작하는 flow 용기의 행두 들여쓰기 결락 및 complex key(`?`) 값 표지 `:`의 0열 출력 → 부모 인덴트
  따르도록 수정; (d) standalone 주석/tag가 있는 complex key의 모호한 텍스트(주석 `?` 상단으로, 본문
  항상 1단계 깊은 별도 줄); (e) flow 용기 내 선두/후미 공백 또는 `,[]{}` 포함 plain 스칼라는
  인용(미인용시 토큰 끊어짐). 추가: tag 부속 빈 block 용기는 헤더를 `{}`/`[]`와 같은 줄에, compact
  dash 항목은 standalone 주석 부속 값을 인라인하지 않음. 9개 targeted Rust 테스트와 Python 회귀로 각
  계통 고정.
- **TOML 이 합법적인 최소 i64 정수를 거부** — `from_toml`/`load_toml` 이
  `-9223372036854775808`(`i64::MIN`)에서 실패: 부호 경로가 절대값을 먼저 해석해 부호 반전 전에
  오버플로했습니다. 이제 부호를 자릿수와 함께 해석하고 (`i64::from_str` 는 음의 방향으로 누적), 부호
  있는 실수는 지수 표기를 유지하며 기존 negate 경로는 제거되었습니다. 새 Python 측 Hypothesis 방언
  퍼징 (`tests/test_property_dialects.py`, stdlib `json`/`tomllib`/`pyjson5` 오라클로 타입 엄격
  비교) 에서 발견; JSON5 베어 `Infinity`/`NaN` 리터럴과 i64 초과 숫자 문자열이라는 AST 모호 표기
  2종도 고정했습니다. Rust 회귀:
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`.
- **잘못 들여쓰기된 플로우 시퀀스 지속 행이 다시 거부됨** — YAML 파서를 granit-parser 1.3으로
  업그레이드하면(*변경* 참조) 지속 행의 들여쓰기가 포함 블록 키보다 깊지 않은 다중 행 플로우
  콜렉션(yaml-test-suite `9C9N`: `flow: [a,` 뒤 열 0의 `b,`)을 조용히 *수락*하기 시작해 엄격성이
  `405/406 → 404/406`로 후퇴했다 — suite의 ≥95% 임계값 게이트에는 보이지 않아 CI를 통과했다. AST
  receiver의 파싱 후 in-tree 가드가 포함 블록 들여쓰기를 추적해 들여쓰기가 부족한 플로우 지속 행을
  거부하여 `405/406`을 회복한다. 가드는 파서가 이미 계산한 span만 사용하므로 올바르게 들여쓰기된
  다중 행 플로우는 영향받지 않는다. `9C9N`은 이제 케이스별 하드 게이트(입력 리터럴, `skipif` 없음)로
  `tests/test_yaml_suite.py` 에 고정되었고, Rust 단위 테스트
  (`parser::tests::flow_continuation_under_indented_is_rejected`)도 추가되었다.
- **자기 참조 병합 키가 더 이상 네이티브 스택을 오버플로하지 않음** — 펼침이
  자기 앵커(`a: &a` 안에 `b: {<<: *a}`)로 돌아가는 `<<`는 `resolve_merge_keys`에서
  무한히 펼쳐 네이티브 스택을 고갈시키고 인터프리터 프로세스 전체를 종료시켰다
  (Windows 종료 코드 `0xC00000FD`, 세그먼테이션 폴트). 순환 가드는 병합 쌍을
  *수집*할 때만 적용되고 펼침을 *순회*할 때는 적용되지 않아 재진입이 전혀 잡히지
  않았다. 이제 앵커 가드는 별개 펼침과 동일하게 경로 스코프: 앵커 이름은 그 펼침이
  순회되는 동안 재귀 경로에 남아, 이미 그 경로의 조상으로 되돌아오는 병합은 재귀하지
  않고 빈 펼침으로 종료한다. 비순환 AST는 PyYAML의 순환 dict를 담을 수 없으므로
  자기 병합은 이제 충돌 대신 `{}`로 수렴한다. 같은 변경에서 관련 병합 의미 결함
  4가지도 수정됨: null/스칼라/시퀀스 병합 소원이 리터럴 `<<` 키로 남지 않고, 병합
  값으로 직접 쓰이는 인라인 맵(`<<: {x: 1}`)이 이제 병합되며, 병합 시퀀스의 비-별개
  요소(`<<: [*a, {y: 2}]`)가 인라인 맵을 유지한다. Rust 6개 및 Python 9개 회귀
  테스트로 커버(`merge::tests`, `tests/test_gaps.py::TestSelfReferentialMerge166`).
  [@bourumir-wyngs](https://github.com/bourumir-wyngs)의 #166 보고.
- **NumPy 직렬화가 GIL 없이 Python 메모리를 읽지 않음** — ndarray writer가 `unsafe { as_slice() }`로
  배열 데이터 버퍼를 빌려 그 슬라이스를 `py.detach` **내부**(즉 GIL 해제 후)에 순회했다. 출처와
  무관하게 `&[T]`는 `Send`이므로 borrow checker가 잡을 수 없지만, 이 메모리는 Python 소유이므로 다른
  스레드가 동시에 resize하거나 쓸 수 있다: 건전하지 않은 데이터 경쟁 / UB이며 동시성 환경에서만
  드러난다. 이제는 **GIL을 잡은 채** 버퍼를 Rust 소유 메모리로 스냅샷(`slice.to_vec()`)하고,
  스칼라→노드 변환만 스레드 밖에서 수행한다. 이는 bindings 계층의 **유일한** `unsafe` 버퍼 빌림이며,
  나머지 모든 `py.detach` 사이트는 Rust 소유 상태(AST, 소스 텍스트, `BufWriter<File>`)만 건드림을
  확인했다. 회귀 커버리지는 `tests/test_numpy.py::TestNumpyConcurrency`.
  [@bourumir-wyngs](https://github.com/bourumir-wyngs)의 #165 보고.
- **반복된 별칭 참조가 `None`으로 해석되지 않음** — `to_dict()`가 **전역** visited
  앵커 집합으로 별칭을 확장하면서 이를 한 번도 지우지 않아, 각 앵커의 **첫
  참조만** 값을 만들고 이후의 참조는 조용히 `None`으로 타락했다:
    ```yaml a: &x 1 b: *x      # 1 c: *x      # 이전에는 None, 현재는 1```
    영향 범위는 "두 번째 참조"보다 넓었다. 같은 컨테이너 안의 형제 참조끼리도 서로
    오염했다(`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}`에서 `b`는 값이었고 `c`는 `None`). 이제 이
    guard는 현재 재귀 경로로 한정되어, 한 번의 확장 동안에만 push되고 이후 pop된다. 따라서 반복
    참조와 형제 참조는 각각 완전히 구성된 값을 얻고, 진짜 순환은 여전히 종료한다. `<<` 머지 해석과
    AST 자체는 영향을 받지 않음을 확인했다. `tests/test_direct_load.py`에 PyYAML과의 일치 여부를
    고정하는 6개 케이스를 추가했으며, 잘못된 출력을 기대값으로 고정하던 테스트 2개를 다시 작성했다.
    [@bourumir-wyngs](https://github.com/bourumir-wyngs)의 #163 보고.
- **첫 키 값이 중첩 컨테이너일 때 문서 헤더 주석 소실 수정** — 파서가가 진행 중인
  모든 컨테이너에 단일 주석 슬롯을 공유했고, 중첩 컨테이너 start가 아직 자리 잡지
  못한 standalone 헤더를 지웠다(parse 단계에서 폐기, `to_dict`엔 보이지 않고 `dump`에서
  치명적). 이제 컨테이너별 슬롯 스택으로 관리.
- **splice 편집 시 선행 주석 중복 수정** — 재생성된 리전 텍스트가 pair/item 자신의
  standalone 주석을 담을 때 교체 범위가 기존 주석 행을 덮지 않아 두 개가 공존했으나,
  plan이 주석 행까지 범위를 확장(`pyq set`/`delete`와 bindings splice 경로 공통 수정).
- **`!timestamp`가 지원되는 모든 Python에서 끝의 `Z` 허용** — `datetime.fromisoformat`은 UTC-`Z`
  접미사를 3.11부터만 인식하므로 플러그인에서 `...Z`를 `+00:00`으로 정규화, 3.8~3.10의 YAML
  `!timestamp` 스칼라 및 `load_toml` / `from_toml` 경로의 TOML datetime에서
  `Invalid isoformat string` 오류를 해소.

#### 성능

- **이벤트 스트림→Python 객체 직접 구성** — `safe_load`, `safe_loads`, `YAML().safe_load*`는 완전한
  AST를 구성한 뒤 `convert.rs`에서 다시 순회하는 대신 granit 이벤트 스트림을 한 번 순회해 Python
  객체를 구성한다. 스키마 해석·원문 매핑 키·중복 키 오류 의미론은 완전히 동일하다.
  앵커/tag/merge/멀티 문서 입력은 제로 비용 사전 제외로 AST 경로에 폴백. WSL 실측: 스칼라 위주
  `safe_load` −21~25%, 패밀리 전체 −13~18%, 폴백 형태는 불변.
- **앵커 추출 바이트 게이트** — `extract_anchors`는 `&` 바이트 포함 여부를 한 번만
  검사해 앵커가 없는 문서는 문자 단위 인용 상태 머신을 완전히 건너뜁니다. Rust 쪽
  `parse_*` 벤치 중간값 11–18% 개선, 스캔 자체는 1.5µs → 38ns.
- **스트림 이벤트 딕셔너리 키 인터닝** — `parse_stream`/`load_stream`이 이벤트마다
  넣는 고정 키를 `pyo3::intern!` 상주 객체로 재사용해 키별 Python 문자열 할당을
  없앴습니다. WSL 실측: `parse_stream` −34%, `parse_stream_multidoc` −39%,
  `load_stream` −22%.
- **분해 마이크로벤치** — `granit_events_*` 벤치로 granit 순수 이벤트 파이프라인
  비용과 AST 구축을 분리(벤치 전용).
- **다중 문서 파싱의 문서별 딥복사 제거** — `on_document_end`가 완성된 문서를
  딥복사 대신 컬렉션으로 소유권을 이동합니다(다음 문서가 result를 재구성하므로
  복사는 순수 오버헤드). WSL 실측: `parse_all_docs` −9.7%, `safe_loads`(다중 문서)
  −9.5%, `YAML().safe_loads` −6.7%.
- **스트림 쓰기는 문서 간 단일 버퍼를 재사용** — 새 `direct_dump_into`는 각 문서를
  재사용 `String`에 쓰고, `dump_iterable`은 텍스트가 개행 하나로 이미 끝나면
  `normalize_doc` 재복사를 건너뜁니다. WSL 실측: `dump_stream_multi_doc` −27.2%,
  `dump_stream` −4.4%.
- **AST 빌더 스칼라 빠른 경로** — `unescape_double_quoted`는 백슬래시가 없으면 즉시
  반환하고, `detect_chomping`은 블록 스칼라마다 문서 전체 행을 collect 대신 지연
  가져오기를 합니다. WSL 실측: `to_dict` 계열 −4~9%, 스칼라 타입 로드 −3~4%, 회귀 없음.

#### 문서

- **numpy 가이드의 0차원 스칼라 절 모든 로케일 수정** — 기존 문서는 "단일 항목
  리스트로 리셰이프"(`assert data == [42]`)라고 했지만 실제 동작(`tests/test_numpy.py`로
  고정)은 맨 스칼라로 직렬화(`assert data == 42`)합니다. 4개 로케일 텍스트를 정정했고
  en 문서에 0-D `bool` → `1.0` rust-numpy 특성 경고 admonition을 추가했습니다.

</details>

### [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node 메타데이터/스타일 API · 스키마 파일 입출력 · 심층 편집 · cp314t numpy 재개</summary>

#### 추가

- **노드 메타데이터 세터/게터** — `Node.comment` / `Node.anchor` / `Node.tag` 읽기 속성과
  `set_comment` / `set_anchor` / `set_tag`(및 `remove_*` 계열)를 추가. 별칭 또는 존재하지 않는 경로
  편집은 오류가 발생합니다. 인라인 스칼라 값·시퀀스 항목의 독립형 주석은 자체 들여쓰기 행에
  출력됩니다(`child:\n  # c\n  val` 및 `- a\n# c\n- b`의 기존 라운드트립 결함 수정).
- **Verbatim 태그** — `set_tag("!<tag:yaml.org,2002:str>")`는 verbatim 태그(빈 핸들)를 생성하며,
  소스에서 파싱된 verbatim 태그는 라운드트립 시 보존됩니다: `Tag`의 `Display`는 빈 핸들 태그를
  `!<...>`로 감싸 출력하고, `parse_tag`는 `!<...>` 형식을 인식하며, 스트림 이벤트는 `Display`를 통해
  태그를 직렬화합니다.
- **스키마 파일 IO 및 목록** — `load_schema(name, path)`는 파일에서 스키마 정의를 읽어 등록하고,
  `list_schemas()`는 등록된 모든 스키마 이름(내장 `failsafe`/`json`/`core`/`yaml1.1` + 사용자
  정의)을 반환합니다.
- **노드 style/format 세터/게터** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping` 읽기
  속성과 `set_scalar_style` / `set_flow_style` / `set_chomping` 메서드. ScalarStyle/Chomping이 이제
  `Copy`를 derive합니다. 비스칼라 노드는 `None` 반환 / no-op, 별칭 및 존재하지 않는 경로는 오류가
  발생합니다.
- **스키마 구조 검증** — 스키마 정의의 `validate` 섹션으로 구조 검사(경로 한정 스칼라 타입,
  `sequence_of`/`mapping_of` 컨테이너, `required`)를 추가.
  `validate_against_schema(data, schema_yaml)`는 모든 실패를 나열하며 `YamlValidateError`를
  발생시킵니다.
- **`Node.copy()`** — 하위 트리를 문서에서 분리된 독립 Python 값(dict/list/scalar)으로 깊은
  복사합니다. `set_value()`로 붙여넣는 데 유용합니다.
- **고급 편집 API** — `doc.set_many({path: value})`로 여러 경로(와일드카드 `[*]` 및 딥 스캔 `..`
  지원)를 단일 스플라이스 버스트로 설정. `doc.sort_keys()`로 매핑 키를 제자리에서 정렬.
  `Node.move(new_path)`로 하위 트리 이동. `Node.path` / `Node.find_first()` / `Node.value_eq()`로
  경로 접근·첫 와일드카드 검색·값 비교 추가.
- **0.14+ 기능 프로퍼티 테스트** — `validate_node` / 스키마 파싱 / style round-trip Rust proptest,
  `set_many` 와일드카드 / metadata 편집 / `sort_keys` Python hypothesis 테스트 추가. `hypothesis`를
  `test` 그룹으로 이동해 CI에서 프로퍼티 테스트 실행.
- **시리얼라이저 수정** — 빈 흐름 컨테이너(`key: {}` / `key: []`)의 독립형 주석이 무효 YAML을
  생성하던 문제 수정(인라인으로 강등).

#### 변경

- **프리-스레디드 (cp314t) 휠에서 NumPy 재활성화** — cp314t 빌드 인수에서 `--no-default-features`
  제거. rust-numpy 0.29는 프리-스레디드 Python을 지원하며, `numpy.ndarray` 직렬화가 프리-스레디드
  휠에서 사용 가능합니다(NumPy 설치는 런타임에 자동 감지).

#### 문서

- **모든 언어(en/zh/ja/ko) 문서의 오래된 참조 수정** — `saphyr-parser` → `granit-parser`, YAML
  준수율 98.1% → 99.75%(스위트 405/406 케이스), ABI3 지원 3.9–3.13 → 3.8–3.15(py3.9+ → py3.8+),
  벤치마크 표를 현재 CodSpeed CI 수치(파싱 21–43배, 직렬화 55–177배 PyYAML 대비 빠름)로 업데이트.
  Rust 측 벤치마크 섹션을 Criterion에서 divan으로 마이그레이션(`benches/yaml_bench.rs` →
  `crates/pyrs-yaml/benches/yaml_bench.rs`).

</details>

### [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>따옴표와 이스케이프 경계: 홑따옴표 역슬래시, BOM, 비문자, 멀티바이트 줄바꿈</summary>

#### 수정

- **백슬래시+제어 문자/비문자를 포함한 단일 인용 스칼라** — 이러한 값은 이중 인용을 사용합니다. 단일
  인용은 제어 문자/비문자를 이스케이프할 수 없습니다.
- **비문자 및 BOM 인용** — `needs_quotes` / `needs_double_quoted`가 U+FFFE/U+FFFF/평면 끝 비문자와
  U+FEFF(BOM)를 인용 필수로 취급합니다.
- **이중 인용 이스케이프 폭** — U+FFFF 이상의 코드 포인트는 8자리 `\Uxxxxxxxx` 형식으로
  출력합니다(4자리 `\u`는 BMP 전용).
- **접힌 plain 스칼라 연속 들여쓰기** — 연속 들여쓰기를 값의 시작 열에서 도출하여 중첩된 시퀀스/매핑
  항목의 연속 행이 부모 블록 들여쓰기를 초과하도록 했습니다.
- **멀티바이트 접기 경계** — `wrap_plain_scalar`가 접기 슬라이스를 문자 경계로 내림하여 4바이트
  UTF-8이 경계를 가로지를 때 panic을 방지합니다.
- **publish 테스트 요구사항에 `hypothesis`** — `.ci/requirements-test.txt`에 `hypothesis>=6.113.0`을
  고정하여 게시 워크플로가 속성 테스트를 실행할 수 있게 했습니다.

#### 추가됨

- **`scripts/fuzz_panics.py`** — dump/parse/edit/멱등성에 걸친 적대적 전략을 사용한 로컬 대규모
  Hypothesis fuzz 하네스.

</details>

### [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML 스키마 언어와 교체 가능한 해석 · 따옴표 스칼라 · 빈 컬렉션</summary>

#### 추가됨

- **YAML Schema Language** — 정규식 패턴을 YAML 타입에 매핑하는 사용자 정의
  스키마 정의 가능. `register_schema()`로 등록.
- **인라인 dict 스키마** — `schema` 매개변수에 `dict` 직접 전달 가능.
- **Community Plugins** — `CustomType` 기본 클래스로 사용자 정의 노드 타입 등록.
  `register_type()`으로 등록.
- **내장 플러그인** — `!timestamp`(datetime)와 `!set`이 기본 등록됨.

#### 변경됨

- **스키마 해석이 플러그인 가능하도록** — `SchemaResolver` 트레이트 +
  `Schema` 열거형 + 전역 `SchemaRegistry`. 내장 스키마는 제로 비용 디스패치 유지.
- **`node_to_pyobject`와 `direct_dump`가 `CustomType` 확인** —
  태그 스칼라는 `from_yaml()`로 변환, Python 객체는 `to_yaml()`로 직렬화.

#### 수정

- **따옴표 스칼라는 항상 문자열로 로드** — 암시적 타입 해석은 평문 스칼라에만 적용(YAML 1.2).
  `safe_load('"true"')`는 문자열 `"true"`를 반환합니다(`True` 아님). 직렬화기는 문서(`to_yaml`)
  경로에서도 음수가 올바르게 왕복되도록 유지합니다.
- **홑/겹따옴표 단일 문자 키 왕복** — `'` 또는 `"` 단일 문자 맵 키는 인용 스칼라로 출력되어 파싱
  불가 YAML이 되지 않습니다.
- **빈 컬렉션은 `{}`/`[]` 출력** — 빈 매핑/시퀀스 덤프가 재파싱 시 `None`이 되는 빈 문서를 생성하지
  않습니다.

#### 변경

- **`get()`은 리터럴 키 전용** — `YamlDocument.get()`은 `.`/`[` 포함 키를 JSONPath로 추정하지
  않으며, 항상 최상위 맵 키로 취급합니다(`__getitem__`/`__setitem__`과 일관). 경로 접근은
  `find()`/`node()`를 사용하세요.

</details>

### [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96과 edition 2024 · 직접 기록기와 해석 빨래길 · granit 이전</summary>

#### 변경 사항

- **Rust MSRV를 1.96으로 업그레이드하고 edition을 2024로 변경** — 두 crate 모두
  `rust-version = "1.96"` 및 `edition = "2024"`를 선언합니다. CI는 `build`/`test-freethreaded`
  작업을 Rust 1.96으로 고정하여 결정론적인 wheel 빌드를 보장합니다. 또한 `msrv-check` 작업을
  추가하여 MSRV에서 `cargo check`/`cargo test`를 실행하고 정적 MSRV 드리프트를 방지합니다
  (`rust-lint` 작업은 `stable` 유지). 버전 바트는 PyO3 0.29 자체의 기반 (rustc 1.83)보다 높게
  설정되며, std API 선제 지원(예: `assert_matches!`, 1.96 안정화)을 목적으로 합니다.
  `TAG_REGISTRY`(태그 핸들러 관리)가 `std::sync::LazyLock`로 리팩터링되어 `Mutex<Option<...>>` 간접
  계층이 제거되었습니다.

#### 성능

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer** — Python→YAML
  serialization without intermediate `CustomNode` AST. Single-pass `direct_dump` replaces the old
  two-pass `pyobject_to_node` + `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on
  `from_dict` (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking** — when input has no `&`
  characters, skip `collect_anchors` + anchor resolution and use the simpler
  `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/ non-boolean first bytes
  return `Str` immediately, avoiding schema resolution overhead for the common case. (#59)
- **granit-parser 마이그레이션** — saphyr-parser를 granit-parser 1.0.1로 교체하여 네이티브
  `Event::Comment` 출력으로 전체 텍스트 `scan_yaml()` 프리스캔을 제거. parse_small -18%,
  parse_large -21%, roundtrip_large -18%.

#### 수정

- **`float_to_yaml_string` round-trip 수정** — Rust Display가 소수점을 버릴 때 `.0`을 추가(`42` →
  `42.0`)하여 float가 int로 바뀌지 않고 round-trip되도록 함.
- **`count_nodes` 사전 할당 롤백** — 전체 AST 순회 비용이 피한 realloc보다
  커서(serialize_10mb 약 14% 저하) 버퍼 확장은 Vec에 위임.

#### 추가

- **스트림 & frontmatter API에 `max_depth` 추가** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)` 가
  `max_depth`를 허용 (기본값 1000). 스트림 파싱은 이제 코어 `parse_stream_with_options`를 통해 중첩
  깊이 제한을 적용 (기존에는 스트림 이벤트에 깊이 제한이 없었음).
- **Pydantic 통합** — `dump_pydantic()`은 Pydantic 모델을 YAML 문자열로 직렬화
  (`model_dump(mode='json')` + `safe_dump`); `parse_as()`는 YAML 문자열을 Pydantic 모델 인스턴스로
  파싱. 둘 다 지연 임포트, pydantic에 대한 하드 의존성 없음. (#61)

#### 내부

- **Split `py/mod.rs`** — monolithic 1786-line module broken into `document.rs` (YamlDocument),
  `yaml_instance.rs` (YAML class), `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` 가드 + `double_quoted_scalar()` 생성자** — `'true'` /
  `'42'` / `'null'` 같은 문자열은 코어 스키마 재파싱 시 오독되지 않도록
  큰따옴표 스칼라로 출력(`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed 벤치마크를 `codspeed-divan-compat`으로 통일** — `exclude-allocations`로 할당 노이즈
  제거. 크로스 라이브러리 벤치마크를 `tests/test_benchmark_crosslib.py`로 통합하고 공용
  `tests/data/yaml_samples.py` 픽스처와 스트리밍 커버리지 추가.

</details>

### [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>`set(create_missing)` · `walk`/`scalars` · 모노레포 · 해석 핫 경로</summary>

#### 추가

- **`set(create_missing=True)`** - missing intermediate mapping keys along
  the edit path are created as nested mappings (e.g. setting `a.b.c` on
  `a: 1` creates `b` and `c`); index segments that miss are still an error,
  and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal yielding `Node`
  objects, avoiding per-node `to_dict()` resolution. `walk()` returns all nodes; `scalars()` returns
  only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate` (key_eq, navigate,
  navigate_mut, normalize_index, mapping_key_index), `editing::region` (line helpers, node_is_flow,
  extend_delete_over_comments, nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null
  values, deeply nested, flow collections, mixed types.

#### 변경

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/` (pure Rust, no PyO3) and
  `crates/pyrs-yaml/` (PyO3 bindings). Root `Cargo.toml` is now a workspace. Old `src/` directory
  and `build.rs` removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to
  `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy duplicate-key detection,
  `shift_insert` merge prepending, and skipped `DocumentEnd` deep-clone for single-document parses
  cut large-document parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event text share allocations via
  `Arc<str>`; AST nodes shrink 8 bytes and clones become refcount bumps instead of deep copies.

#### 수정

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip previously dropped standalone
  comments attached to simple-key nodes; now preserved (two regression tests).

</details>

### [0.11.7] - 2026-08-04

<details markdown="1">
<summary>항상 실패하던 검사 대신 release-guard 정적 단언 · numpy 추적</summary>

#### 변경

- **stub-build-check replaced with release-guard** - the always-red container
  build (`validate.yml`) that deliberately failed to reproduce the v0.10.0
  `--generate-stubs` failure mode is replaced with three static assertions
  that **pass** when the repo is correct: `grep` guards `publish.yml` against
  `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked,
  and `test -f` checks `py.typed` exists. The job now gives green CI on
  correct state, red only on regression.

#### 추가

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free-
  threaded support status (PyO3/rust-numpy#476) as a dependency for re-enabling
  ndarray serialization on cp314t wheels when the Rust binding matures.

</details>

### [0.11.6] - 2026-08-04

<details markdown="1">
<summary>cp314t wheel에서 numpy 제거 · 자유 스레드 CI · 설치 문서</summary>

#### 변경

- **Free-threaded (cp314t) wheels are now numpy-free** - built with `--no-default-features`, so
  rust-numpy is excluded entirely (smaller binary, no runtime probe). `safe_dump` on a
  `numpy.ndarray` raises `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15) keep
  full ndarray serialization.

#### 추가

- **Free-threaded CI validation** - `test-freethreaded` job now builds and tests with
  `--no-default-features`, matching the shipped free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

</details>

### [0.11.5] - 2026-08-04

<details markdown="1">
<summary>구문 분석 견고성 3/4/5: 감사 결과 없음으로 종결</summary>

#### 변경

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus
  (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no
  fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 /
  yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key
  strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and
  `tests/test_strictness_audit.py`.

#### 추가

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current
  rejection/acceptance behavior (both directions), so future parser changes cannot silently regress
  strictness or over-reject.

</details>

### [0.11.4] - 2026-08-04

<details markdown="1">
<summary>빈 키 중복 허용(2JQS) · 올바른 거절도 집계 · tab 복원</summary>

#### 수정

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches
  yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate
  despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is
  one tab, fixing tab-encoded suite cases

#### 변경

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2
  grammar, matches PyYAML/libyaml)

</details>

### [0.11.3] - 2026-08-03

<details markdown="1">
<summary>스트리밍 기록 · 줄 오프셋 캐시 · 게시 사전 검증 · 준수 보고서</summary>

#### 추가

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with
  document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### 변경

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API
  unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### 수정

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures
  before Release

</details>

### [0.11.2] - 2026-08-03

<details markdown="1">
<summary>해석은 결합 자격 계산을 생략 · 선형 커서 레이아웃 검사</summary>

#### 추가

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: O(앵커 + 청크) 메모리의 지연 이벤트
  반복자

#### 성능

- **파싱 시 스플라이스 자격 계산 안 함** — O(문서) 레이아웃 검사가 첫 편집 시
  `YamlDocument.splice_checked`를 통해 지연 실행되어 v0.11.0 회귀 복원: parse_comments -59%,
  parse_anchors -42%, parse/roundtrip/edit -10~35% 모두 v0.10.0 수준으로 복귀
- **선형 커서 레이아웃 검사** — 사전 계산된 줄 오프셋에 대한 노드별 이진 탐색 대체 (단조 소스 순서
  순회)

#### 변경

- `parse_with_options`가 `CustomNode`를 반환 (기존 `(CustomNode, bool)`); 스플라이스 자격은 이제
  `YamlDocument` 내부에 있으며 요청 시 계산

</details>

### [0.11.0] - 2026-08-02

<details markdown="1">
<summary>외과식 직렬화: 편집 반영 때 문서 전체를 다시 놓지 않음</summary>

#### 추가

- **Surgical Serialization** — 모든 AST 노드의 바이트 수준 소스 스팬 추적; 세그먼트 기반 스플라이스
  — 편집은 접촉 영역만 재생성, 미접촉 텍스트는 바이트 복사
- 속성 테스트 (proptest, 새 개발 의존성)
- 10MB 편집-플러시 벤치마크 (divan)

#### 변경

- `flush_source`가 세그먼트 스플라이스 사용; 플로우 스타일 영역, 비기본 레이아웃 문서, 병합 키,
  CRLF/BOM 문서, materialize 후 (단일 버스트 모델) 에서 전체 직렬화로 폴백
- 스플라이스 편집이 `---`/`...`/지시자 마커 라인을 미변경 바이트로 보존 (전체 직렬화는 이전에 이를
  제거 — 의도적인 동작 차이)

</details>

### [0.10.0] - 2026-08-01

<details markdown="1">
<summary>제자리 편집 API와 편집 벤치마크</summary>

#### 추가

- **제자리 편집** — 서식 메타데이터를 잃지 않고 파싱된 문서 편집:
    - 경로 API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`,
      `doc.delete(path)`, `doc.rename(path, new_key)`, JSONPath 스타일 경로(`$.a.b[0]`); 루트 슈가
      `doc["key"] = value`와 `del doc["key"]`
    - 노드 API: `doc.node()` / `doc.find(path)`는 `Node` 객체를 반환하며 `set_value` / `append` /
      `insert` / `delete` / `rename`과 트리 탐색(`parent`, `children`, `walk`, `filter`)을 지원
    - 완전한 메타데이터 보존 — 교체된 스칼라는 주석/앵커/태그/따옴표를 유지; 이름이 변경된 키는
      위치와 주소를 유지; 삭제 시 매핑 순서 보존
    - 원자적 편집 — 실패한 작업은 문서 (리비전 포함) 를 변경하지 않음
    - 지연 소스 재동기화 — `source()` / `to_yaml()` / `reparse()`는 편집 성공 후에만 재직렬화
    - 오래된 노드 감지 — 문서 편집 후 `Node` 접근은 `YamlDocumentError` 발생 (`RuntimeWarning` 포함)
    - 새 예외: `YamlEditError`, `YamlPathError` (en/zh-CN/ja-JP/ko-KR i18n 지원)
    - 별칭 인식 편집 — 별칭 자신의 경로 설정은 그 자리를 교체; 별칭을 통한 편집은 `YamlEditError`
      발생
- **편집 벤치마크** — `benches/yaml_bench.rs`에 divan 벤치마크 6개 추가 (소형~대형 문서의
  set/insert/delete)

#### 변경

- `YamlDocument.source()`가 `str`을 반환하고 제자리 편집 후 지연 재직렬화

</details>

### [0.9.0] - 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15와 GIL 해제 · 태그 처리기 · pydantic · `.pyi`</summary>

#### 추가

- **Python 3.13, 3.14, 3.15 지원** — PyO3 `abi3-py38` 휠이 Python 3.8-3.15 커버 (GIL 빌드);
  `abi3t` + `abi3t-py315`는 free-threaded 안정 ABI 제공
- **Free-threaded CPython (GIL 없음) 지원** — `#[pymodule(gil_used = false)]`가 모듈을 free-threaded
  Python용 스레드 안전으로 선언; `Py_GIL_DISABLED` cfg 플래그로 numpy 게이트 (rust-numpy는
  free-threaded 미지원 — `--no-default-features`로 free-threaded 빌드에서 numpy feature 비활성화)
- **CI free-threaded 작업** — 새 `test-freethreaded` 워크플로 작업이 Python 3.14t에서 컴파일과
  테스트 검증
- **`pyo3-build-config` 빌드 의존성** — `build.rs`를 통해 `#[cfg(Py_GIL_DISABLED)]`,
  `#[cfg(Py_3_15)]` 등 컴파일러 플래그 활성화
- **`numpy` 선택 사항화** — `numpy` feature 뒤에 게이트 (기본 활성화); `Py_GIL_DISABLED` 하에서 자동
  제외
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`,
  `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs`
  모두 이 플래그를 받습니다; 중복 매핑 키는 기본적으로 `YamlDuplicateKeyError`를 발생시키며, 허용 시
  `last value wins`
- **`SerializeOptions` 확장** — `doc.to_yaml_with_options()`에 `width` (줄 바꿈, 0 = 끄기),
  `indent_mapping`, `indent_sequence`, `indent_offset` 추가 (기존
  `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth`와 함께)
- **태그 핸들러 레지스트리** — `register_tag("!custom")` 데코레이터 및 명령형 형태 +
  `clear_tag_handlers()`; 등록된 태그를 가진 스칼라 노드는 핸들러를 통해 변환됨
- **우선순위를 가진 태그 핸들러 체이닝** — 여러 핸들러가 `priority` 오름차순으로 실행;
  `YamlTagSkip`은 핸들러가 다음으로 전달되도록 하고, fallback은 원래 값을 유지
- **Pydantic 통합** — `parse_as(Model, yaml, **yaml_kwargs)`는 YAML을 파싱하여 Pydantic v2 모델로
  검증; pydantic이 없을 때 `ImportError` 발생
- **`.pyi` 타입 스텁** — maturin으로 자동 생성되어 커밋되므로 `register_tag`, `parse_as`,
  `to_yaml_with_options` 및 새 예외가 타입 체커에게 표시됨

#### 변경

- CI Python 매트릭스 확장: ubuntu, windows, macos에서 3.8-3.14
- 안정 ABI: `abi3-py39` → `abi3-py38` (더 넓은 Python 3.8+ 지원), `abi3t` + `abi3t-py315` 추가
  (free-threaded 안정 ABI)
- `pyproject.toml` classifiers에 3.13, 3.14, 3.15 항목 추가
- **CI 최적화: 중복 Rust 컴파일 제거** — 단일 `rust-lint` 작업이 `cargo clippy` + `cargo test`를 한
  번 실행; 빌드 작업은 OS별 abi3 휠을 하나 생성하고 테스트 작업이 `maturin develop` 대신 설치하여
  21개 매트릭스 작업에서 Rust 컴파일을 제거 (~86% 감소); 모든 작업에 `Swatinem/rust-cache` 추가
- **pydantic 테스트 의존성** — `pydantic>=2.10.6`을 `[dependency-groups] test` 및
  `.ci/requirements-test.txt`에 추가 (SSOT via `uv sync` in ci.yml)

#### 수정

- **Windows DLL 로딩** — `src/py/tag_registry.rs`에서 `#[cfg(test)]` 블록 제거로 Windows에서
  `import pyrs_yaml`broken 문제 해결 (`250b8d0`)
- **Python 3.8 호환성** — `pydantic.py`에 `from __future__ import annotations` 추가 (`63d2495`)
- **CI pydantic 스킵** — `pytest.importorskip("pydantic")`로 pydantic 미설치 시 테스트 통과
  (`7be011d`)
- **Windows의 CI glob 확장** — `pip install dist/*.whl`에 `shell: bash` 사용 (PowerShell은 `*` 확장
  안 함) (`2f7778d`)
- **문자열이 아닌 태그 핸들러 반환이 `YamlTagError`를 발생시킴** — 비`str` 값을 반환하는 핸들러는
  이제 `Tag handler '!x' must return a string` 오류 발생 (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` 인덴트 연결** — `indent_mapping`/`indent_sequence`/`indent_offset`가
  직렬화기에 의해 이제 반영됨 (이전에는 dead 필드; 각각 생략 시 `indent_size`/0으로 기본값)
- **`width`가 작은 값에서 멈추지 않음** — `width < continuation indent`일 때 무한 루프 대신 나머지
  텍스트를 래핑 없이 출력 (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — 태그 핸들러를 등록 해제하는 새 함수; `register_tag`/`clear_tag_handlers`
  보완
- **`duplicate-key` 오류가 i18n 적용** — `YamlDuplicateKeyError` 메시지가 이제 모든 4개 locale을
  통해 `format_i18n_error`를 통해 흐름

</details>

### [0.8.0] - 2026-07-30

<details markdown="1">
<summary>`YAML()` 인스턴스 API · Python Node API · `MergedView` · 생명주기 경고</summary>

#### 추가

- **`YAML()` 인스턴스 API** —
  `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` 재사용 가능한 구성;
  `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` 메서드
- **Python `Node` API** — `Node` 클래스: `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`,
  `children`, `root_type`, `value`로 AST 탐색; JSONPath 스타일 쿼리 언어 (`$.key.sub`, `$.arr[0]`,
  `$..deep`)
- **`doc.version` 메타데이터** — `YamlDocument.version()`이 YAML spec 버전 반환 (기본값 "1.2")
- **`MergedView`** — `doc.merged()`가 병합 키가 해석된 읽기 전용 dict-like 뷰 반환
- **라이프사이클 경고** — `Node.release()`로 노드 명시적 무효화; 오래된 접근은 `RuntimeWarning` +
  `YamlDocumentError` 발생

#### 변경

- `parse()` / `safe_load()`가 이제 구문 당용으로 `YAML().parse()` / `.safe_load()`에 위임
- `YamlDocument`가 이제 문서 메타데이터를 위해 `version` 필드 저장

</details>

### [0.7.1] - 2026-07-30

<details markdown="1">
<summary>ryaml 벤치 비교 · 준수 기준 상향 · Divan 이전</summary>

#### 추가

- **ryaml 벤치마크 비교** — `tests/test_benchmark.py`에 `ryaml` (Rust YAML 라이브러리) 에 대한
  벤치마크 추가 (PyYAML 및 ruamel.yaml와 함께); `benchmark_compare.py` 기능 비교 보고서로 재작성
- **CI 준수 임계값 상향** — YAML Test Suite 준수 게이트가 `test_compliance_report()`에서 70%에서
  75%로 증가; 유효 파싱율 게이트는 95% (`tests/test_yaml_suite.py:251`)
- **CI 의존성 통합** — 발행 워크플로와 로컬 개발 전반에 통일된 테스트 의존성 관리를 위해
  `.ci/requirements-test.txt` 및 `.ci/requirements-test-lite.txt` 추가
- **벤치마크 현대화** — 더 빠른 C 확장 기반 통계 벤치마킹을 위해 `pytest-benchmark`에서
  `pytest-codspeed`로 마이그레이션; 모든 CI 작업은 이제 `-r .ci/requirements-test.txt` 사용
- **Rust 벤치마크 Divan으로 마이그레이션** — `codspeed-criterion-compat`를 `codspeed-divan-compat`
  v5.0.1로 교체; 16개 벤치마크가 Criterion 그룹에서 `#[divan::bench]` 속성으로 재작성 (`Cargo.toml`,
  `benches/yaml_bench.rs`)

#### 변경

- CI 벤치마크 작업은 교차 라이브러리 비교를 위해 `ryaml` 설치
- `benchmark_compare.py`는 이제 타이밍을 `pytest-benchmark`에 위임하고 기능 비교/보고 도구로 역할

</details>

### [0.7.0] - 2026-07-29

<details markdown="1">
<summary>직렬화기의 `max_depth` 방어 · 핫 경로 최적화 · pytest-benchmark</summary>

#### 추가

- **직렬화기 `max_depth` 가드** — `serialize_node_internal`이 이제 재귀 깊이 추적하고 제한 초과 시
  `YamlMaxDepthError` 발생 (기본값 1000), 파서 보호 일치 (`src/serializer.rs:135-145`)
- **직렬화기 핫패쓰 최적화** — 블록 스타일 직렬화를 대상으로 한 5개 최적화로 ~4.9% 루트립 속도 향상:
    - `write_anchor_tag` 및 `write_inline_comment` None 체크 인라이닝 (~99% 노드에 대한 메서드 호출
      제거)
    - `write_indent` hot/cold path 분할 (캐시된 레벨 ≤64에 대한 직접 인덱싱)
    - 짧은 ASCII 영숫자 문자열 (≤8자) 에 대한 `write_plain_scalar` 고속 경로
    - Plain 스칼라에 대한 `write_scalar_for_key` 직접 디스패치 (디스패치 체인 방지)
- **pytest-benchmark 마이그레이션** — Python 벤치마크가 통계적 엄밀성, 구조화된 JSON 출력 및 CI
  통합을 위해 원시 `time.perf_counter()`에서 `pytest-benchmark`로 마이그레이션
  (`tests/test_benchmark.py` + 업데이트된 `tests/test_performance.py`)

#### 변경

- Python 벤치마크에서 원시 `timeit` 대신 `pytest-benchmark` 사용
- CI 벤치마크 작업은 이제 개별 스크립트 대신 `pytest --benchmark-json` 실행

#### 제거

- `write_inline_comment` 메서드 — 모든 호출 위치에서 인라이닝됨
- 직렬화기에서 `Comment` import — 이제 불필요

</details>

### [0.6.0] - 2026-07-27

<details markdown="1">
<summary>비동기 직렬화 · JSON Schema 검증 · `to_json()` · 점진적 재해석</summary>

#### 추가

- **비동기 직렬화** — `asyncio.run_in_executor`를 통한 `safe_dumps_async`, `safe_dump_async`,
  `safe_loads_async`, `safe_load_async` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema 검증** — `YamlValidateError` 예외 + `YamlDocument.validate(schema)` 메서드 (`str`
  또는 `dict` 수락); Python `jsonschema` 모듈에 위임
- **`YamlDocument.to_json()`** — 문서를 JSON 문자열로 직렬화 (Python `json.dumps` 사용)
- **증분 재파싱** — `YamlDocument`가 이제 소스 텍스트 저장 (`doc.source()`);
  `doc.reparse(resolve_merges=True, schema="core")` 로 제자리 재파싱
- **29개 새 테스트** — `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

#### 변경

- `YamlValidateError` 새 사용자 정의 예외로 등록 (`ValueError` 상속)
- `rust_i18n::i18n!` 매크로 경로가 `"src/i18n/locales"`로 업데이트
- `validate_translations()` 테스트 경로가 새 로케일 디렉토리 일치하도록 업데이트

#### 제거

- 중복 `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` 삭제 (rust-i18n에서 참조되지 않음)
- `locales/*.yml` → `src/i18n/locales/` 이동 (i18n 모듈과 함께 위치)

#### 의존성 변경

- 런타임 의존성: `jsonschema>=4.25.1`
- 개발 의존성: `pytest-asyncio>=0.23` (런타임에서 이동, 더 이상 고정되지 않음)

</details>

### [0.5.0] - 2026-07-27

<details markdown="1">
<summary>`Serializer::write_node` 고침 · `YAML_SCHEMA` 상수 · 개발 문서</summary>

#### 수정

- **`Serializer::write_node`** — `block_mapping`/`block_sequence`의
  `values.iter().next().unwrap()`에서 `.unwrap()` 제거하고 안전 인덱스 접근으로 교체하여 edge-case
  AST에서 잠재적 패닉 제거
- **`YAML_SCHEMA` 상수** — 오타 `yamorg2002`를 `yamlorg2002`로 수정 (YAML 1.2 spec URL 일치)
- **개발 문서** — Python 명령어에 필수 `uv run` 접두사 및 Rust 명령어에 직접 `cargo` 추가하여
  `AGENTS.md` 업데이트

</details>

### [0.4.0] - 2026-07-27

<details markdown="1">
<summary>132개 빈칸 채움 테스트: 다중 문서 · 바이트 입력 · 케이스별 검사</summary>

#### 추가

- **132개 새 gap-filling 테스트** — 이전에 테스트되지 않은 API에 대한 포괄적 커버리지
- **i18n 함수 테스트** — `set_language`, `get_language`, `list_languages`, `detect_language`,
  `negotiate_language`
- **`parse_all_docs` 전용 테스트 스위트** — 단일 문서, 여러 문서, 빈 문서, 주석
- **`parse_file` 성공 사례 테스트** — 기본 파싱, 주석 보존, 파일 없음 오류
- **`to_yaml_with_options` 테스트** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys`
  순서 보존
- **`to_dict()` 메서드 테스트** — 스칼라 루트, 중첩, 리스트, bool, null, anchor 해석, 빈 매핑/시퀀스
- **YamlDocument dunder 메서드 테스트** — `__repr__`, `__str__`, `__contains__`, `__len__`,
  `__iter__`, `__getitem__`, `root_type()`
- **바이트 입력 테스트** — `parse(b"key: value")`, UTF-8 바이트, 잘못된 UTF-8 오류
- **유니코드 & 특수 문자 테스트** — CJK, 이모지, 루트립, CRLF 줄 끝, 중복 키
- **`safe_load`/`safe_loads` 기능 커버리지** — anchor, merge key, block scalar, flow collection,
  특수 부동소수, 타입 해석
- **`from_dict` edge cases** — 키의 특수 문자, 중첩 리스트, None 값, 빈 dict/list
- **`from_json` 루트립** — 중첩 구조, 배열, 잘못된 JSON 오류
- **`dump_file` 테스트** — 성공 경로, 잘못된 경로 오류
- **YAML Test Suite 개별 케이스 테스트** — 8진수, 16진수, 과학적 표기법, NaN, 무한대, merge key,
  명시적/암시적 키, bool/null 변형, block scalar strip (`|-`), flow collection
- **`resolve_merges` 파라미터 테스트** — 비활성화 시 `<<` 보존, 기본값으로 해석
- **Flow collection 루트립** — 루트 레벨 및 중첩 flow 매핑/시퀀스
- **비스칼라 노드에 anchor** — 매핑 anchor (`&defaults`) 및 시퀀스 anchor (`&items`)
- **시퀀스 인덱싱 테스트** — 양수 인덱스, 범위 벗어남 오류
- **Merge key 통합** — 해석된 및 해석되지 않은 merge key로 루트립
- **태그 보존** — `!!seq` 및 `!!map` 태그 테스트 커버리지
- **주석 보존** — 복잡한 구조의 inline 및 standalone 주석 테스트

#### 변경

- 버전 동기화 수정: `python/pyrs_yaml/__init__.py` `__version__`을 0.2.0에서 0.4.0으로 업데이트하여
  Cargo.toml/pyproject.toml과 일치
- `dist/`에서 구버전 0.2.0 wheel 아티팩트 제거

</details>

### [0.3.0] - 2026-07-27

<details markdown="1">
<summary>NumPy ndarray 직렬화(N-D) · 따옴표 스칼라 형 · 음수 왕복</summary>

#### 추가

- **NumPy ndarray 직렬화** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()`이 모든
  차원 (0-D from N-D) 의 `numpy.ndarray` 지원
    - 지원 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - 다차원 배열은 올바른 들여쓰기로 중첩 YAML 리스트로 직렬화
    - 복소수는 `(re+imj)` 문자열 형식으로 직렬화
    - `0-D` 스칼라 배열은 1-D로 재변형되어 단일 항목 리스트로 직렬화
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate로 제로 복제 dtype 디스패치
    - 슬라이스 반복 동안 GIL 해제 (최대 성능)
- **`quoted_scalar()`** — 단일 따옴표 YAML 스타일이 필요한 값용 새 `CustomNode::quoted_scalar()`
  생성자
- **따옴표 스칼라의 타입 해석** — `resolve_yaml_type`가 이제 `SingleQuoted`/`DoubleQuoted` 스칼라에
  적용되어 따옴표 있는 음수의 올바른 루트립
- **포괄적 NumPy 테스트 스위트** — 모든 dtype, 차원 (0-D from 4-D), 음수, 무한대, NaN, 빈 배열, edge
  case를 다루는 42개 테스트
- Flow collection (`{}`/`[]`) 루트립 지원 — Mapping/Sequence AST 노드에 `flow_style` 필드
- `parse()`가 `str` 및 `bytes` 입력 모두 수용
- `parse()`가 `resolve_merges` 파라미터로 merge key 확장 옵트아웃 지원
- saphyr 이벤트를 통한 다중 문서 파싱용 `parse_all_docs()`
- `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` 파라미터가 있는
  `to_yaml_with_options()`
- 기본값 파라미터를 지원하는 `get()`
- YAML을 파일에 작성하는 `dump_file()`
- `benches/yaml_bench.rs`의 Criterion 벤치마크 (파싱/직렬화/루트립)
- 매트릭스 테스트가 있는 GitHub Actions CI (3 OS x 4 Python 버전)
- 전체 YAML 1.2 spec으로 확장된 anchor 이름 파싱 (도트, 콜론, 해시, 따옴표 anchor)
- `__version__` 속성, `py.typed` PEP 561 마커

#### 수정

- **음수 루트립** — YAML 1.2 블록 시퀀스는 `-`로 시작하는 plain 스칼라를 포함할 수 없음; 이제 직렬화
  시 따옴표로 감싸서 정수/부동소수로 올바르게 파싱
- **N-D 배열 지원** — `PyArray1<T>`를 `PyArrayDyn<T>`로 교체하여 1-D뿐만 아니라 모든 차원의 배열
  지원
- **올바른 중첩 깊이** — 다차원 배열이 이제 정확히 N 수준의 중첩 생성 (내부 차원은 shape[1..] 처리,
  루트 차원은 `plain_sequence`로 래핑)
- `to_dict()` 및 `safe_load()`의 alias 해석 — alias가 이제 참조값 대신 `None`으로 해석
- `safe_loads()`가 더 이상 단순 `split("---")`를 사용하지 않음 — saphyr의 문서 이벤트 사용
- 파싱 중 Mapping/Sequence 태그가 더 이상 폐기되지 않음
- `format_scalar_for_key()`이 Literal/Folded 블록 스칼라 스타일 처리

#### 변경

- ndarray 타입 디스패치를 위해 `numpy` crate (v0.29) 의존성 추가
- PyO3를 0.21에서 0.29로 업그레이드
- 15+ 보일러플레이트 `CustomNode` constructions를
  `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` 생성자로 교체
- 직렬화기가 `write_anchor_tag()` 및 `write_inline_comment()` 헬퍼 추출
- 파서가 `detect_flow_style()` 헬퍼 추출
- 데드 코드 제거: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`,
  `format_yaml_type` (테스트 전용)
- 6개 중복 테스트 파일 통합, 9개 진단 스크립트를 `scripts/`로 이동
- 키/인덱스/타입 컨텍스트로 오류 메시지 개선

</details>

### [0.1.0] - 2026-07-25

<details markdown="1">
<summary>최초 릴리스: YAML 1.2 완전 메타데이터 AST · 왕복 보존 · PyYAML 호환 API</summary>

#### 추가

- saphyr-parser를 통한 YAML 1.2 준수로 초기 릴리스
- 완전한 메타데이터 (주석, anchor, 태그, chomping, 스칼라 스타일) 와 함께 사용자 정의 AST
- 주석, anchor, 태그 및 서식의 루트립 보존
- PyYAML 호환 API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` 변환 함수
- YAML frontmatter 추출용 `read_markdown`/`read_markdown_str`
- chomping indicator가 있는 블록 스칼라 (`|-`/`|+`/`>-`/`>+)
- 이스케이프 시퀀스 (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 타입 해석 (null, bool, int, float, infinity, NaN)
- Merge key 해석 (`<<: *alias`)
- 복잡한 키 (시퀀스/매핑을 키로)

</details>
