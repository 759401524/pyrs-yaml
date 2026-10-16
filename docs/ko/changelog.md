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

#### 추가

- **사용자가 설치하는 wheel은 허용선을 잰 컴파일러로 구축된다** — `publish.yml`은 `maturin-action`에
  `rust-toolchain`을 넘기지 않아서, 모든 릴리스 wheel은 당시 `stable`로 만들어졌다. 반면
  `.ci/ir-baseline.json`은 명령수 게이트가 측정에 쓴 컴파일러를 기록한다. 2026-10-16 실측(같은 트리,
  같은 호스트, `RUSTUP_TOOLCHAIN`만 바꿈)에서 `rustc 1.99.0`은 `rustc 1.97.1` 대비 `serialize_*`
  족에서 계산된 명령수 +6.35%, 22개 시나리오 합계 +0.198%를 냈다. 즉 확정된 허용선은 아무도 설치하지
  않는 빌드를 서술하고 있었다. 이제 일곱 wheel 구축과 `pyq` CLI 산출 작업과 `ci.yml`의 릴리스 wheel
  단계가 기록된 버전을 넘긴다 — 마지막 것은 tag를 밀어서 처음 동작하는 대신 pull request마다 세
  OS에서 그 입력을 검증하게 하려는 것이다. `sdist`는 원시를 담기만 하고 컴파일하지 않으러 고정하지
  않는다. `tests/test_publish_toolchain.py`(다섯 개)는 핀과 기준선이 어느 한쪽이 움직여 어긋나면
  실패한다 — runner 이미지가 `stable`을 옮기는 경우, 기준선 갱신이 핀을 옮기는 경우 모두. 속도
  향상이 아니며 그렇게 팔지도 않는다: 허용선이 산출물을 서술해야 코드 생성 이동이 숫자가 붙은 결정이
  된다. 실측: 새 검사 다섯 개 통과, 한 단계를 `1.99.0`으로 어긋내자 딱 wheel 구축 검사만 빨개진 뒤
  되돌렸다. 두 워크플로 모두 YAML로 다시 해석된다. 미측: 실제 Release 실행 — `publish.yml`은 tag로
  발화하며, 일부러 tag를 밀어 검증하지 않았다.

- **긴 키는 모든 형식의 스트림에서 문자만큼의 열을 요구한다** — 이벤트가 담는 위치는 행 번호와
  *문자*로 센 열이다. JSON과 TOML은 `pyrs_event::LineTable`로 바이트 오프셋을 변환하고, YAML은
  granit 자체 마커가 가진 열을 그대로 보고한다. 두 변환은 서로 다른 크레이트에 살기 때문에, 세
  생산자에게 같은 질문을 던 검사는 하나도 없었다. 이제 6개의 매개변수화 경계 검사가 그것을 한다 —
  `yaml`, `json`, `toml` 각각에 CJK 키와 이모지 키 하나씩 — 기대 열은 손으로 쓰지 않고 Python
  문자열에서 유도하며, 값이 소스의 문자수가 가리키는 자리에 있음을 확인한다. 변이로 귀인시키면
  생산자가 갈라진다: `LineTable`이 바이트 길이를 되게 하면 JSON과 TOML의 4개 검사만 빨개지고
  Rust의 두 층이 동행한다
  (`a_line_table_reports_the_granit_convention_for_both_engine_dialects`,
  `every_character_offset_is_the_position_of_its_own_character`). YAML 어댑터에서 `col()` 대신
  `byte_offset()`을 읽히면 YAML의 2개 검사만 빨개진다. 둘 다 되돌렸다. 검사 전용: 어떤 엔진 경로도
  건드리지 않았으므로 명령수 레그는 판단할 새로운 주장이 없다. 실측:
  `uv run pytest tests/test_streaming.py` 37 통과(신규 6), 확장 최신화 게이트 포함 40.

- **로컬 명령수 읽기가 어떤 컴파일러가 구축했음을 말해 준다** — `AGENTS.md`는 이미지 사이 잔차를 두
  시나리오에서 재측정했다고 적어 왔다. 이제는 22개 전부를 같은 트리와 같은 WSL 호스트에서
  `RUSTUP_TOOLCHAIN`만 바꿔서 쟀다. 기준선이 고정된 `rustc 1.97.1`에서는 허용선을 넘는 것이 없다 —
  최악이 `to_python_medium` +0.1639%, CPython을 링크하지 않는 19개는 ±0.003% 안에 든다. 반면 이
  기계의 기본 `rustc 1.99.0`은 다섯 개를 넘긴다(`serialize_medium` +9.08%, `serialize_block` +8.86%,
  `serialize_small` +8.03%, `serialize_anchors` +7.95%, `to_toml_medium` +3.55%). 게다가
  `serialize_block_scalars`는 반대 방향으로 7.71% 움직인다. 두 방향에 이 크기는 코드 생성 차이이며
  바이너리 요약값도 이를 뒷받침한다. CI가 잘못된 빨강에 걸릴 일은 없다 — `codspeed.yml`이 기준선이
  기록한 툴체인을 설치한다 — 그러나 기여자가 로컬에서 돌리는 측정은 아무것도 말하지 않았다. 그래서
  `ir_gate.py`는 기록된 버전과 현재 버전이 다르면 컴파일러 경고를 출력해 두 버전과 위 두 수치,
  `RUSTUP_TOOLCHAIN=<pin>` 해결책을 지목한다. 그 경고를 `--report-only`에서도 본다(기존에는 경고도
  바이너리 짝맞춤도 없이 최대값만 인쇄했다). `FAIL toolchain changed`도 같은 한 문장을 다시 쓴다.
  환경 경고의 고정값이던 "`serialize_block_scalars` +1.45% 어긋남"은 사라진다 — 컴파일러를 고정하면
  0.00005%만 어긋난다. 표와 근거는 `docs/dev/perf.md`, 검증은 `tests/test_ir_gate_provenance.py` (네
  개, 같은 버전 재구축을 버전 변경으로 보고하지 않는 경우 포함). 실측: 경고는 WSL의 두 경로에서 모두
  인쇄됐다. 미측: 명령수 — 엔진 경로는 움직이지 않는다.

- **이벤트 어휘의 두 일반 작업에 속성 테스트를 붙였습니다** — `pyrs-event`는 바이트 위치를 좌표로
  바꾸고 빌려온 이벤트 본문을 소스에서 옮겨 오는데, 둘 다 골라 쓴 입력으로만 못 박혀 있었습니다.
  이제 두 속성이 임의 문서(1~4바이트 문자 혼합: 모든 위치는 자기 문자의 답을 내고, 넓은 문자 안의
  바이트는 시작점으로 접으며, 위치는 바이트순서를 거스르지 않고, 끝을 지나면 패닉이 아닌 하나의
  답)와 임의 이벤트 필드에 대해 그것들을 말합니다 — 소유변환은 소유 관계만 바꾸고 멱등입니다. 변이
  검사로 경계 클램프를 항등으로 바꾸면 위치 속성이, 행을 0으로 두면 소유속성이 붉어짐을
  확인했습니다. CI는 기존 다리로 20,000례까지 올리고 `cargo nextest run --all` 664 통과(2개 추가). —
  상세: quality-ledger (ed), `QUALITY_MATRIX.md`.

- **이벤트 어휘가 자신의 형태를 직접 측정합니다** — `pyrs-event`는 모든 생성자가 보고하는 타입을
  담고 있는데 테스트는 `LineTable` 두 개뿐이라, 의존하는 열한 갈래 `into_owned` 매치가 생성자가
  우연히 내보낸 변종에만 맡겨져 있었습니다. 이제 crate 안의 단위 테스트 세 개가 모든 변종을 하나씩
  만들어 묻습니다: 복사와 소유화가 모든 필드와 행·열 위치를 그대로 두는가(이 매치가 망가지는 방식은
  조용합니다 — 갈래가 옆으로 이동하거나 필드를 빼고 다시 지어도 컴파일은 통과합니다); 소스에서
  빌려온 스칼라와 같은 텍스트의 소유본이 같은가(스트림/트리 재전 비교가 의미를 갖는 조건입니다);
  주석의 소유화가 공유 `Arc<str>`을 복사 대신 이동하는가(결정 3이 산 할당 주장). 변이 검사는
  귀속까지 확인했습니다 — Comment 갈래에서 `standalone`을 빠뜨리면 필드 테스트만, 이동한 `Arc`를
  복제로 바꾸면 포인터 테스트만 붉어집니다. 테스트 전용 — 읽기 경로를 건드리지 않으니 명령수 검사는
  새 판단이 없습니다. `cargo nextest run --all` 662 통과(3개 추가). — 상세: quality-ledger (eb).

- **`parse_stream`이 JSON, JSONC, JSON5, TOML 문서도 서술합니다** — `docs/dev/event-stream.md` 계획
  B의 마지막 단계는 이미 YAML을 스트림으로 읽던 그 동사에 키워드 전용 `format`을 더하는
  것이었습니다(`parse_stream(text, on_event=None, max_depth=1000, *, format="yaml")`). 하나의 호출
  형태가 세 생성자에 닿고 어휘는 두 번째가 없습니다. dict는 같은 일곱 키를 유지하고, 그 형식이 할
  말이 없는 필드는 `None`으로 남습니다. `anchor`와 `alias`는 YAML 고유의 것으로 남고, TOML의 리터럴
  `'…'`은 기본 문자열 `"…"`과 똑같이 읽힙니다 — 어떤 인용표였는지가 보관되지 않기 때문입니다. TOML이
  실어 오는 태그는 허브의 철자로 도착합니다(`!timestamp`, `!!int`) — YAML의 철자가 아니라. 승인·거부
  판정은 트리를 가는 경로와 같고 같은 스캐너가 답하므로 두 경로를 테스트로 비교했습니다. 기본값은
  그대로여서 기존 호출은 positional 인수를 포함해 이벤트 단위로 전과 같은 스트림을 받습니다.
  `max_depth`는 그 선택지를 갖는 두 읽기 쪽에 적용됩니다(YAML은 `YamlMaxDepthError`, JSON은 파싱
  오류). TOML의 중첩 한도는 중간 표현식 자기 것입니다. stub을 다시 생성했고, 네 언어 API 페이지와
  스트리밍 안내에 새 축을 적었습니다. 실측: `cargo nextest run --all` 659 통과,
  `uv run pytest tests/` 2853 통과에 경계 테스트 15건 추가, JSON5 가지를 strict으로 바꿔 변이
  검사까지 마쳤습니다 — 정확히 JSON5 테스트 두 개만 이름으로 붉어졌습니다. 미측정: 명령수 — 이
  호스트에 valgrind가 없고, 추가된 match는 측정 중인 열로 경로 바깥에 있습니다. — 상세:
  quality-ledger (ea), `docs/dev/event-stream.md`.

- **`pyrs_json::from_events`가 공개됐고, 한 JSON 문서의 두 순회가 게이트가 됐다** — step 2가 테스트
  계층에 남긴 접기는 이제 공유 어휘의 리더이며, 검증하는 리더다. 이 crate의 어떤 producer도 낼 수
  없는 목록(빈 스트림, 열린 것 없는 close, `Alias`, 앵커나 태그가 달린 이벤트, 블록 스칼라)는 추측이
  아니라 진단을 갖춘 거부이고, 여덟 모양이 고정됐다. 불변식가 pull request를 실패시킬 수 있게
  게이트는 두 계층으로 돈다. `fuzz/fuzz_targets/json_events_replay.rs`가 방언마다 탐색하고,
  `crates/pyrs-json/tests/events_replay.rs`는 13개 시드 어료를 세 방언에서 모든 `cargo nextest`마다
  재생한다 — 판정 쌍 39, 재생된 문서 22, 둘 다 조용히 비어돌지 않도록 각각에 하한이 있다. 처음
  90초의 탐색에서 중복 JSON 키가 제 노드가 소유한 주석을 떨어뜨리는 결함을 찾았다 (#450에서 고쳤고
  그 입력은 시드로 남긴다). 그리고 어휘가 답하지 못하는 두 번째 것도 고정했다:
  `Comment::standalone`은 트리에선 "이 주석이 어느 슬롯에 올라타는가"를, 스트림에선 "원문이 그에게
  한 줄을 주었는가"를 답하므로, 대조는 주석 텍스트만 보고 그 플래그은 뺀다. — 상세: quality-ledger
  (dz), `docs/dev/event-stream.md`.

- **TOML에도 트리를 만들지 않는 이벤트 스트림이 있고, 첫 생산자가 비워 둔 칸에 답합니다** —
  `pyrs_toml::parse_stream`은 `from_toml`이 짓는 것과 같은 문서를 설명합니다. `pyrs-json`이 필요로
  했던 두 번째 스캔 걸음은 일부러 택하지 않았습니다. TOML의 표 규칙(어느 헤더가 어떤 경로를 여는지,
  어느 점 표기가 그 층을 닫는지, 표 배열이 어디서 꼬리를 더하는지)은 `register_header` /
  `register_kv`에서 한 번만 서술되는데, 두 번째 걸음은 그것을 두 번 쓰게 되기 때문입니다. 그래서
  이벤트는 AST 투사가 읽는 그 중간 표현에서 나옵니다. 스캐너는 한 번 돌고 판단은 두 읽자에게 함께
  성립합니다(잘못된 입력 26개 × 방언 2개, 여기에 16개 씨앗 덩어리까지 여전히 테스트로 묻습니다).
  공유 어휘는 세 칸에서 두 번째 형식의 답을 얻었습니다. `tag`는 결국 YAML 전용이 아닙니다 —
  date-time은 `!timestamp` / `!date` / `!time`을, 2진 정수는 `!!int`를 싣고 도착해 트리가 가진 것과
  그대로 같습니다. `anchor`는 계속 비고, `style`은 IR이 기억하는 것만 보고하므로 문자열 표기 `'…'`와
  기본 형식 `"…"`는 구별 없이 도착합니다(어떤 따옴표를 쳤는 저장되지 않기 때문입니다). 위치는 두
  가지가 스트림에만 존재합니다. 닫힘 이벤트는 자기가 닫는 구조의 시작 오프셋을 싣는데 TOML은 끝을
  쓰지 않기 때문이고, 헤더의 `[`가 선언하는 이름보다 앞서므로 표의 키와 `MappingStart`은 바이트
  순서가 아니라 구조 순서로 보고됩니다. IR는 키·표·노트마다 `usize` 몇 개를 치르며, 그 대가는
  명령수 관문이 판단합니다 — 상세: quality-ledger (dx), `docs/dev/event-stream.md`.

- **JSON에 트리를 만들지 않는 이벤트 스트림 (`pyrs_json::parse_stream`)** — 공유 어휘가 기다리던 두
  번째 생산자입니다. 2026-10-13 결정이 요구한 형태, 즉 AST 경로가 이미 쓰는 스캐너를 스스로 한 번 더
  훑는 걸음으로 만들었기 때문에 sink를 트리 순회에 꿰는 설계가 내던 +1.5%를 아무도 물지 않습니다.
  `pyrs-json`은 `no_std`인 `pyrs-event` 의존을 하나 얻었고, 각 이벤트는 트리가 줄 수 없는 행과 열을
  싣습니다. 아홉 개의 테스트가 두 번째 형식의 답을 기록합니다: `anchor`와 `tag`는 계속 비고
  `Alias`는 한 번도 나오지 않습니다. `style`은 스칼라가 어떻게 쓰였는지를 보고하므로 JSON5의 `'k'`
  키는 스트림에서 `SingleQuoted`, 허브에서는 plain입니다. 유한하지 않은 토큰은 원문대로 오고, `.inf`
  투영 은 `hub_scalar` 한 곳에만 삽니다. 잘못된 입력 32개와 14개 씨앗 덩어리를 두 걸음이 같은
  `ParseError` 로 거절하고, 통과한 33개는 직접 파서가 만든 트리로 재생됩니다. 어휘가 아직 답하지
  못하는 질문도 감추지 않고 이름을 올렸습니다: `{"a": 1 /* x */, "b": 2}`와
  `{"a": 1, /* x */ "b": 2}`는 같은 이벤트 순서를 내는데 엔진은 노트를 서로 다른 노드에 둡니다.
  따라서 재생이 정하는 것은 구조와 값과 모든 노트의 텍스트이지, 그 자리가 아닙니다 — 상세:
  quality-ledger (dw), `docs/dev/event-stream.md`.

- **의존성 업데이트가 일정에 따라 도착하고, 하나의 경계 안에서 스스로 병합됩니다** —
  `.github/dependabot.yml`은 이제 `cargo`(워크스페이스와 `fuzz/`, 각자의 락파일), `uv`
  (`pyproject.toml` + `uv.lock`), CI actions를 추적하고 매주 `main`을 대상으로 돕니다. 권한은 이미
  사람을 막던 그 관문이 그대로입니다: 해당 head 커밋에서 `Test matrix (all legs)`가 `success`를
  보고하는 것(`skipped`와 `cancelled`는 거절로 읽습니다). 판단은
  `scripts/dependency_merge_gate.py`에 있고 실행은 `.github/workflows/deps-automerge.yml`가 합니다.
  스스로 병합할 수 있는 것은 해석 결과의 이동뿐입니다 — `Cargo.lock`, `uv.lock`, `prek.toml`, action
  pin. 매니페스트, 툴체인, 커밋된 타입 스텁은 사람 몫으로 남습니다. 그것들을 움직이면 봇이 이행할 수
  없는 의무가 따라오기 때문입니다 (maturin 버전 잠금과 함께 바뀌어야 하는 스텁 대조 규칙, 파서
  major의 실측 마이그레이션, 지원되는 인터프리터 하한). 그래서 PyO3 0.30+는 `python-floor` 레이블로
  도착하고, 초록색 점검 하나가 실수로 Python 3.8을 내려놓지는 않습니다. `--admin`은 어디에도 쓰지
  않습니다. CI가 봇에게 주는 락파일 면제 (`scripts/check_changelog_coupling.py`)가 병합 허용 목록과
  같은 묶음을 가리킨다고 검증되어 있습니다. 어긋나면 어떤 업데이트는 초록이 될 수 없거나 예고 없이
  main에 들어갑니다 — (세부 사항: quality-ledger (dv), `CONTRIBUTING.md`)

- **이름을 지킨 모양은 한 라운드에 수렴해야 합니다
  (`crates/pyrs-yaml-core/tests/settles_in_one_round.rs`)** — fuzz 층의 정점 계약을 libFuzzer가 찾고
  `cargo fuzz tmin`으로 줄인 모양에 직접 물었고, 유한 예산의 탐색에 맡기지 않았습니다. 이제 노트
  배치 규칙을 바꾸면 운 좋은 추출이 아니라 이름 지어진 테스트가 빨개집니다. 실제로 재어 물러난 두
  구제는 quality-ledger (ds)에 기록되어 있습니다.

- **표기 집합을 표로 만들어, 모든 쓰는 쪽에 자기 읽는 쪽이 답하는 질문을 묻습니다.** — #432가 고친
  결함은 쓰는 쪽의 맨 token 판정이 읽는 쪽 문법보다 값싼 질문을 묻는 것이었고, 속성 검사가 그것을
  잡은 것은 20 000 차례 뽑기가 하필 `00.`을 만들었기 때문입니다. 이제 표로 움직이는 두 검사가 평문
  스칼라가 실을 수 있는 각 표기에 그 질문을 묻습니다. JSON·JSONC·JSON5에 24개, TOML에 18개
  표기입니다. 그래서 다음 불일치는 무작위 실패가 아니라 표기와 방언을 지목합니다. 실제로 물기도 함을
  재어 확인했습니다. `json5_integer_part`를 "숫자, 그리고 점"으로 되돌리면 검사는
  `json5 wrote "[00.]" for the spelling "00."`에서 실패합니다. — (자세히: quality-ledger (dn))

- **자체 리더가 하나의 키로 해석하는 매핑에서 기록기가 어떻게 할지를 결정하고, 양쪽을 시험으로
  고정했 습니다.** — #336 은 주석이 달린 `!E null` 과 비어 있는 `!E null` 을 나란히 둔 매핑에서
  `to_yaml` 이 어떻게 해야 하는지를 물었습니다. 기록할 때 짝을 접는 변경은 구현되었고 비용도 잰
  상태였습니다. 다섯 개의 `serialize_*` 장에서 +31.481% 에서 +33.655%, `parse_small` 은 움직이지
  않았는데 기준선은 0.50% 만 허용합니다 (`docs/dev/perf.md`). 알리는 일도 모든 매핑의 모든 키를 넘는
  같은 통과 처리를 요구하므로, 둘 다 기록 경로에 놓을 자리가 없고 놓을 필요도 없습니다. 키를
  해석하는 경계는 호출자가 지나는 곳입니다. `doc["~"] = 1` 뒤에 `doc["null"] = 2` 를 하면 `~: 2` 한
  줄만 쓰이고, 다시 읽으면 null 키는 하나입니다 — 먼저 나온 표기가 남고 마지막 값이 이깁니다
  (`tests/test_edit.py::TestKeyIdentityAcrossSpellings`). Rust 에서 `NodeMap` 을 직접 만들면 두 줄이
  그대 로 나가지만, 그것을 중복으로 거부하거나 트리보다 항목이 하나 줄어든 결과로 답하는 쪽은
  리더입니다. 이 형태는 짐작이 아니라 고정입니다
  (`serializer::the_write_path_keeps_both_lines_of_a_pair_its_reader_resolves_as_one`). 동작은
  바뀌지 않 았습니다 — 추가된 것은 `#[cfg(test)]` 와 문서, 그리고 `ROADMAP.md` 의 비게 된 미결
  표입니다.

- **방어 매트릭스는 속성이 몇 개인지가 얼마나 단언하는지를 잰다.** — `property_functions()`는
  `prop_*`를 25개 세었고, 문서는 그 수를 속성 층의 커버리지로 보고해 왔다.
  `conditional_property_assertions()`는 각 본문을 읽어 `if let Some(_) = try_*(..)` 형태의 가드를
  찾아 25개 중 7개를 지목한다: 이 속성들은 자기 보조 함수가 돌려보내지 않은 표본에 대해서는 아무것도
  말하지 않는다. 가드 뒤의 비율은 논증이 아니라 측정이었다 — `arb_custom_node()`에서 트리 4000개를
  뽑아 각각을 다시 해석한 결과와 견주면 **3423개가 걸러지고 해석 실패는 0개**였다. 바로 그것이
  문서의 `? *x` 키를 `null: v`(만든 트리보다 한 항목 짧은 다른 mapping)로 짜 내보낸 쓰기가 층이 도는
  동안에도 녹색이던 이유이고(#410이 쓰기를 고친 쪽, 여기는 왜 아무도 보지 못했나의 쪽), 그 맹점은
  `property-tier:conditional-assert`로 등록되고 탈출 조건은 확인할 수 있는 상태로 적히며
  `QUALITY_MATRIX.md`에 실리고 두 방향으로 고정된다: 가드 목록이 비면 구멍은 사라진다. 문서 검사는
  자기 문장을 측정된 두 계수에 고정하고, 검증용 트리에서 탐지기는 가드가 걸린 함수만 집는다 — 보조
  함수를 *정의한* 파일이 아니라 그쪽을 집는 "`try_roundtrip`이 본문에 나타나는가" 식의 탐사는 그것을
  커버리지로 보고하고 말았을 것이다.

- **Python 실행이 디스크의 소스보다 오래된 바이너리의 말을 거절합니다.** — `python/pyrs_yaml/`에는
  마지막으로 빌드된 확장이 놓여 있으므로, `jj edit`이나 `jj rebase`다음의 `pytest`는 다른 브랜치의
  엔 진을 조용히 검사하기 쉽고, 그 결과는 답으로 보입니다. 이 저장소는 한 세션에서 그 방식으로 세 번
  오인 됐기에 AGENTS.md는 작업 복사를 옮긴 뒤 재빌드를 요구합니다. 그 의무는 지금까지
  문장이었습니다. `tests/test_extension_freshness.py`는 로드된 네이티브 모듈을 그것에 컴파일된 모든
  파일 — 각 crate의 `src` 트리와 Cargo 매니페스트 — 과 비교하고, 이분 탐색을 남기는 대신 넘친
  파일들과 재빌드 명령을 이름 으로 짚습니다. 정직한 동시에 시끄럽지 않게 유지하는 판단은 둘. crate
  루트가 `#[cfg(test)]` 아래 선언 한 모듈은 제외하는데, 그 이름은 손으로 쓴 목록이 아니라 선언에서
  유도합니다(속성 검사를 고쳐서 Python을 돌리는 데 재빌드는 필요 없고, 손으로 쓴 목록은 아무것도
  제외하지 않는 쪽으로 썩습니다). 작업 복사 밖 에 설치된 확장은 이유를 남기고 건너뛰는데, 이 소스가
  그것을 빌드한 것이 아니기 때문입니다. 돌연변이로 확인했습니다: 어떤 crate의 `src` 트리 아래에
  파일을 하나 더 놓으면 이 검사가 빨갛게 실패하면서 제 이름 을 짚고, 그것을 지우면 초록으로
  돌아옵니다.

- **명령어 수 게이트가 이벤트 경로를 재므로, 스칼라 소유 규칙 판결에 숫자가 실렸다.** — 게이트는
  이 경로를 `parse_stream`을 한 번도 지나지 않았다. `parse_*`는 전부 AST를 만들고 각 교량도 같다.
  2026-10-14에 유지보수자가 중립 이벤트는 스칼라 텍스트를 빌려야 한다고(`Cow<'a, str>`, 이스케이프를
  디코딩한 것만 스스로 보유) 판결했고, 이는 할당에 대한 주장이므로 시나리오를 먼저 넣고 코드를
  바꾼다: `stream_medium`은 `MEDIUM_YAML`을 `parse_stream`으로 이천 번 읽고 이벤트를 세며, 기준값은
  275,227,226 측정 명령어다. 같은 텍스트의 `parse_medium`은 321,836,091이고, 세 표본은 명령어
  단위까지 같다(산포 0.000%). 변경 전에 수를 취하는 것은 이득을 문장으로 주장하지 말고 게이트로
  계산하게 하려는 뜻이다. `QUALITY_MATRIX.md` 2절의 시나리오 개 수도 함께 twenty-two가 됐고, 오늘
  넣은 본문 개수 검사기가 먼저 정확히 빨개졌다 — 지금 막으려는 어긋남이 바로 이 변경이 조용히
  만들었을 그것이기 때문이다.

- **파서가 받아들이는 빈 명시적 키가 이제 문서와 테스트로 고정됐다.** — 고정된 파서 아래에서 혼자
  있는 `?`, 뒤에 공백이 붙은 `?`, 뒤에 탭이 붙은 `?` 세 가지 모두 오류가 아니라 문서다. 셋 다 키가
  null 스칼라인 매핑으로 읽히지만, 라이터에는 *빈* 명시적 키용 표식이 없어서 정직하게 쓸 수 있는
  출력은 `~: ~`뿐이다. 원문의 표기는 사라지지만 데이터는 사라지지 않는다 — `? a\n: b`가 같은 이유로
  `a: b\n`으로 줄어 돌아오는 것도 그렇다 — 표식 없이는 왕복할 수 없는 키만 표식을 남긴다(`? ?`는
  중첩 명시 형식 `?\n  ~: ~\n:\n  ~`으로 출력된다). 2026-10-14 채택: 거부하지 않고 받아들인다.
  YAML이 맞는지 정하는 것은 고정된 파서이고, 거부하면 `yaml-test-suite`가 받는 입력을 이
  라이브러리가 막게 된다.
  `serializer::tests::a_degenerate_explicit_key_is_accepted_and_re_emits_the_null_pair`가 이제 여섯
  입력의 정확한 재출력 형태와 한 라운드 뒤에 고정점이 됨을 확인하고, `docs/dev/boundaries.md`가 다른
  명시적 키 계열 옆에 이를 적었다.

- **방어 매트릭스 본문에 적은 개수를, 그 본문이 설명하는 파일과 맞붙이는 검사도가 생겼다.** —
  `QUALITY_MATRIX.md` 2절은 기준선에 "seventeen"개의 시나리오가 있다고 적고 있었지만
  `.ci/ir-baseline.json`은 twenty-one개를 들고 있었다 — 새로고침 잡드가 출력하는 수이고, 이 저장소
  스스로의 테스트 docstring이 말하는 수이기도 하다. 그 파일의 키는 이미 고정되어 있었으니 읽는
  사람은 이 주장이 녹색이라고 읽을 수밖에 없었다. 보장하려는 일과 전혀 무관한 이유로 통과하는
  검사라는 점에서, 이 파일이 기록해 온 다른 결함들과 같은 모양이다.
  `tests/test_ir_baseline_workflow.py`는 이제 커밋된 파일의 시나리오를 세고 2절이 같은 수를 말하기를
  요구한다. 그래서 `benches/ir_gate.rs`에 시나리오를 더하는 사람은 이 문장으로 돌아오거나 이 실행을
  빨갛게 만들거나 둘 중 하나다. 돌연변이로 확인했다: 옛 "seventeen"을 되돌리면 이 테스트만 떨어지고,
  페이지는 그 뒤 바이트 단위로 되돌렸다. 같은 문단에는 오늘 실측으로 부정확함이 드러난 주장이 두 개
  더 남아 있었고, 지우지 않고 바로잡았다 (근거는 PR #402의 댓글). "다른 곳에서 만든 기준선과 최대
  0.10%만 다르다"는 엔진 쪽에서는 성립하지만 바인딩 쪽에서는 적힌 것보다 넓다 —
  `to_python_medium`에서 +0.26%, `to_python_small`에서 +0.19%로, 이 채널 스스로의 실행 내 들쭉날쭉함
  0.076–0.24%와 같은 수준 — 그래서 지금은 채널마다 경계를 따로 이름 붙였다. 그리고 옛 1.45% 어음을
  그 이름으로 짊어졌던 `serialize_block_scalars`는 이제 WSL2에서 12,288,634, 새로고침의 측정값은
  12,288,686으로, 두 호스트 사이 차이는 0.0004%다.

- **명령어 수 게이트가 시나리오의 명령어가 어디로 가는지 말할 수 있다.** — `docs/dev/perf.md`의 모든
  최적화 라운드는 같은 일회성 프로브에서 시작했다. strip 전의 release 벤치를 만들고 callgrind를 두
  번 돌려 `--setup-only`를 빼고 함수별로 순위 매기는 일이었다.
  `python scripts/ir_gate.py --profile serialize_medium`은 그 프로브를 등록한 도구다(`--top N`으로
  표를 넓히고, 이 트리가 만들지 않는 시나리오 이름은 실제로 만드는 목록과 함께 거부한다). 초안
  버전이 밟은 두 버그는 이제 테스트다. callgrind 이벤트 파일을 손으로 읽으면 summary가 4,506,831이라
  적은 실행에 8,854,254를 쌓고 거의 전부 `?`로 돌렸으므로, 이 도구는 `callgrind_annotate` 표를 읽는
  쪽으로 했다. 함수명 정규식으로 갈래를 지르면 `Hasher`가 `IndexMap<…, BuildHasherDefault<…>>`
  메서드의 *타입 인자*에 걸려 TOML의 해시 비용이 12.46%가 된 적이 있으므로, 각 줄은 원 이름 그대로
  출력하고 합치지 않는다 — `tests/test_ir_profile.py`가 그것을 확인한다. 게이트 바이너리는 strip되어
  있어 profile이 `write_plain_scalar` 자리에서 `0x000000000006cc10'2`를 뽑아내므로, 이 플래그가
  스스로 기호를 남긴다. 바뀌는 것은 산출물의 바이트이지 재는 코드가 아니고, 성공·판정 경로에는
  손대지 않는다 — `--profile`은 찍고 0으로 끝난다.

- **`no_std` 잡드가 기반 crate 모두의 호스트 테스트를 돌린다.** — `thumbv7em-none-eabi` 빌드는 다섯
  crate를 전부 했는데 테스트는 `pyrs-ast` 와 `pyrs-schema` 만 해서, `default-features = false`에서
  존재하는 112개 호스트 테스트(`pyrs-json` 58개, `pyrs-toml` 54개)가 스스로 지원한다고 말하는
  경로에서 한 번도 실행되지 않았다. 게다가 `cargo test --no-default-features -p pyrs-toml`은
  컴파일조차 되지 않았다. 중첩 제한 사례가 16 MiB 스택을 받으려 스레드를 띄우는데, 스레드는 std 없이
  존재하지 않는다. 그 사례는 `#[cfg(feature = "std")]`가 되었고, 이유와 `no_std` 쪽에서 무엇을
  잃는지는 게이트를 읽으면 보이는 위치에 적었다. 검증하는 깊이 가드 자체는 `alloc` 전용 로직이라
  `no_std`에서도 같은 코드가 돈다. 베어메탈 빌드는 링크만 증명하고 동작은 테스트가 증명하며, 이제 그
  테스트가 돈다.

- **`pyrs-event`: 문서 이벤트 어휘를 독립 크레이트로 만들었습니다.** — AST 와 나란한, 아키텍처
  기둥이 지정한 두 번째 핵심 추상입니다. `StreamEvent` 와 그 열한 가지 변형을
  `crates/pyrs-yaml-core/src/parser/stream.rs` 에서 옮겨 `#![no_std]` + `alloc` 이고 `pyrs-ast` 만
  의존하는 크레이트에 담았습니다. 의존하는 까닭은 `ScalarStyle` 과 `Tag` 를 공유하기 위해서이고,
  그렇게 하면 이벤트 열과 거기서 짓는 트리가 style 이나 tag 의 뜻에서 어긋나지 않습니다. 엔진은 두
  이름을 계속 re-export 하므로 `pyrs_yaml_core::parser::{StreamEvent, StreamEventType}` 이 그대로
  해석되고 호출 지점은 하나도 바뀌지 않았습니다. 형태는 의도적으로 그대로 두었습니다. 아직 YAML 의
  어휘입니다. 중립적인 어휘는 두 번째 생산자를 놓고 설계해야 하는데 그 생산자는 아직 없기
  때문입니다. `docs/dev/event-stream.md` 가 판결(option B) 과 그것을 실제로 만드는 순서 — JSON
  생산자, TOML 생산자, 그리고 이벤트로 직접 파서가 만든 트리를 복원함을 보이는 재생 성질 테스트 — 를
  기록하며, 이 항목은 그 계획의 첫 단계입니다. 그래서 크레이트 자신의 문서에 어떤 필드가 YAML
  문법인지(JSON 에 anchor 가 있다는 주장이 아님) 를 적었습니다. 동작 변경 없음, 새 의존성 없음, 열
  경로에 추가된 것 없음: 이 크레이트의 thumbv7em `no_std` 빌드가 그 계층 구조를 정직하게 지키는
  관문이고, 이제 다른 기초 크레이트들과 함께 `no_std` 작업에 들어갑니다. — (자세히: quality-ledger
  (cu))

- **CI 가 Python 3.15 에서 검사 묶음을 돌린다.** — 정식 릴리스는 나와 있다(devguide 버전 표는 최초
  릴리스를 2026-10-09, 상태를 `bugfix` 로 적는다). `3.15` classifier 와 `abi3t` + `abi3t-py315` 특성
  지정은 이미 저장소에 있었고 abi3 휠도 3.15 에 설치할 수 있다. 그런데도 어떤 잡도 거기서 묶음을
  돌려본 적이 없으니, 그 버전은 검증된 것이 아니라 선언만 된 상태였다. `test` 는 3.8-3.15 로 (21 개
  줄 → 24 개 줄), `test-freethreaded` 는 단일 인터프리터에서 두 개 줄의 매트릭스로 늘렸다: `3.14t`
  는 이미 배포된 `abi3t` 휠을 설치할 수 없는 쪽이고, `3.15t` 는 그 휠이 적용되는 가장 낮은 버전이다.
  README 두 문서와 `docs/*/installation.md` 도 같은 기준으로 고치고 프리-스레디드 휠이 둘 다 필요한
  이유를 함께 적었다. 3.15 두 줄은 로컬에서 잴 수 없다: 이 기계의 `uv` 는 아직 3.15 정식 빌드를
  내려주지 않는다. 세 경쟁 패키지(`ryaml`, `yaml-edit`, `yaml-rs`)는 3.15 미만으로 고정했다. 3.15
  휠이 없고 고정된 PyO3 가 그 해석자를 거부하므로 이 검사가 시작되기 전에 설치가 죽었다. 이제 3.15
  에서는 건너뛴다. 매트릭스는 fail-fast 를 껐다 — 한 줄의 실패가 같은 run 의 22 개 줄을 취소해 다른
  해석자에 대해 아무것도 보고하지 못했기 때문이다. — (details: quality-ledger (ci))

- **CI 잡이 한 번도 내려받지 않은 코퍼스로 검사 묶음을 돌릴 수 없게 됐다.** —
  `coverage` 는 clone 단계 없이 `pytest tests/` 를 실행해, 세 적합성 검사 묶음
  (`yaml-test-suite`, `toml-test`, `JSONTestSuite`) 이 거기서 건너뛰어졌는데도
  잡은 초록이었다. `tests/test_ci_corpus_gates.py` 는 `tests/*.py` 가 `Reference/`
  아래를 참조한 곳에서 코퍼스를, `pytest tests/` 호출에서 잡을 뽑아, 모든 잡에 모든
  코퍼스 다운로드를 요구한다. 다운로드 단계를 지워도 보고가 변하지 않으면 실패한다.
  세 clone 단계는 coverage 잡에 추가했다. — (details: quality-ledger (cc))

- **JSON 에 적합성 코퍼스 검사가 생겼고, 거절의 “방법”까지 검사한다.** —
  `tests/test_json_test_suite.py` 는 JSONTestSuite 의 판정 가능 사례 283 건을 엄격한
  RFC 8259 로더에 보낸다. 95 건은 파싱, 188 건은 거절이 정답이고, 코퍼스 스스로
  “구현 의존”이라 부른 35 건은 빼되 그 개수까지 단언한다. 처음 실측은 차이가 0, 그래서
  하한은 위로용 값이 아니라 1.0 이다. 둘째 검사는 “어떻게 거절했는가”를 묻는다 - 파서까지
  도착한 입력은 형식 있는 `YamlParseError` 로 답해야 하고, `RecursionError` 나 panic 은
  거절을 위장할 수 없다. 열두 파일은 UTF-8 이 아니라 텍스트를 받는 로더에 닿지 못한다;
  그 개수도 단언하고, 여기서 드러나는 경계(바이트 입구가 없음)는
  `docs/dev/quality-ledger.md` (cb) 에 적어 흐리지 않는다. CI 는 toml-test 와
  yaml-test-suite 옆에서 코퍼스를 clone 하고, 실행측은 빈 코퍼스로 통과하지 않는다.
  — (details: quality-ledger (cb))

- **지역화 문서 검사기가 지금까지 몰랐던 문자 체계를 보고한다.** —
  `check_cjk_localisation.py`는 한자·가나·한글만 봤으므로 일본어 문장 안의 키릴 단어가 모든
  후크를 통과했다. 규칙을 뒤집었다: 라틴·한자·가나·한글은 허용하고 나머지 알파벳 문자는 모두
  보고하므로 그리스·아랍·태·데바나가리 문자는 등록 없이 잡힌다. 라틴은 의도적으로 허용한다 —
  기술 문서에서 식별자와 단위 기호는 피할 수 없고, 이를 무시한 첫 버전은 비교 표에서 `µ`를 예순
  개 넘게 보고했다. 대장이 미리 약속한 대조 실험을 실측했다: `docs/ja/changelog.md`에 `теперь`를
  넣으면 검사기가 빨갛게 되고, 새 분기를 빼면 18개 검사 중 딱 2개만 빨갛게 된다. #346을 닫는다.
  — (자세히: quality-ledger (bx))

- **명령어 수 기준선을 “재생성했다”는 사실만으로 조용히 완화할 수 없게 했다.** —
  `ir_gate.py --update`는 재는 값을 그대로 적었고, 이미 커밋된 값과 비교하는 것은 없었다.
  #342에서 측정한 바로는 같은 빌드가 강제 잡에서 `to_json_medium`을 21,444,770과
  21,444,836으로 읽었고 차이는 0.0003% 이내인데, 한 갱신 artifact는 22,048,805를
  가져와 커밋된 21,232,786 대비 +3.84% — 같은 파일이 선언한 0.005 선의 일곱 배가
  넘는다. 그 artifact는 같은 코드가 아니었다(바이너리 요약이 다르다). 요약은 이제
  기준선에 기록되고 매 실행마다 출력되니, 어떤 바이트에서 왔는지 밝히지 못하면
  비교할 수 없다. 이제 `--update`는 그 선을 넘어 나빠지는 시나리오를
  `--accept-drift <시나리오>`로 지목하지 않으면 거부하고, 승인된 변동은 잡 로그에 출력하며,
  개선은 변동으로 세지 않는다. 허용치는 그대로이고 기준선 값도 하나도 옮기지 않았다.
  — (details: quality-ledger (bv), (by))

- **베이스라인 갱신 잡은 어떤 상향이 의도된 것인지 지시받을 수 있다.** — `ir_gate.py --update`는
  베이스라인 스스로 강제하는 선을 넘어 나빠지는 시나리오를 이름으로 지목하지 않으면 쓰지 않는다.
  그런데 `.github/workflows/ir-baseline.yml`은 맨 명령만 실행했다: 수치를 강제 이미지로 옮기려고
  존재하는 잡이, 어떤 변경이 그 비용을 낼 의도일 때 오히려 옮기지 못했다. 2026-10-12 실측, #360
  head에서 dispatch하면 두 harness를 빌드하고 21개 시나리오를 재고 모든 표본을 출력한 뒤 "`--update`
  would move 3 scenario(s) past the 0.5% line"으로 끝나며 아무것도 쓰지 않았다. 이제 잡은 선택 입력
  `accept_drift`(공백 또는 줄바꿈으로 구분한 시나리오 이름, 하나마다 `--accept-drift` 인자 하나)를
  받고, 비워 두면 종래대로 맨 `--update`가 실행되므로 거부가 기본값으로 남고 로그가 어느 쪽을
  실행했는지 말한다. `tests/test_ir_baseline_workflow.py`에 검사 4개를 추가하고 각각 변이 테스트로
  확인했다: 인자를 무력화하면 `test_the_input_is_actually_passed_to_the_guard`만 빨개지고,
  `to_json_medium`을 반복문에 하드코딩하면 `test_an_empty_input_keeps_the_refusal`만 빨개진다.
  베이스라인 수치도 허용 오차도 움직이지 않았다. 처음 실사용에서 실측: 잡이 출력하는 줄이 플래그를
  시나리오로 세었다(세 이름을 6으로) 이제 이름을 세며 네 번째 검사가 이를 고정한다. — (자세히:
  quality-ledger (cj))

- **품질 대장의 문자 섹션이 순서대로 정리됐고, 관문이 이를 지킨다.** — `### (xy)` 섹션은 각 브랜치가
  고른 위치에 삽입되어 자라나, 두 개가 더 뒤의 문자 뒤에 있었다: `(bh)`가 `(bi)` 뒤에, `(ch)`가
  `(cj)` 뒤에. 빨간 체크는 없었다 - `(details: quality-ledger (ch))`는 여전히 해석되니까 - 다만
  파일이 자기 번호 체계와 맞지 않았을 뿐이다. 둘 다 통째로(98줄과 46줄) 옮기고 파일 행의 중복집합이
  그대로임을 검사해, 재배치가 몰래 단락을 빠뜨리거나 중복시킬 수 없게 했다.
  `tests/test_ledger_section_order.py`는 제목이 엄격히 증가하고 문자 중복이 없기를 요구하며 각각
  변이로 확인했다: 이미 쓴 문자로 고치면 두 검사가 모두 빨개진다 (중복은 동시에 후퇴이기도 하니). 빈
  번호는 보고하지 않는다: `bs`/`bu`/`cd`는 다른 경로로 확정됐고, `ak`-`an`은 이 파일에서 오래된 굵게
  시작하는 형태로 존재할 뿐 제목이 아니다. — (자세히: quality-ledger (ck))

- **CI 인터프리터 행렬을 패키지 선언에서 만들고, 그 안에 숨겨 둔 두 관문을 역할로 바꿨다.** —
  `ci.yml`은 `os x python-version`을 곧장 써서 24칸이었다. 144개 줄을 실측하면 한 줄이 ubuntu 138초,
  macOS 186초, Windows 251초이고 그중 76%는 같은 약 2 500개 단정을 인터프리터마다 다시 돌리는 값이다
  (pull request마다 78.2분). 최근 30회 실행에서 실패가 단일 버전이나 단일 OS로만 격리된 경우는
  없었다. 이제 `scripts/ci_matrix_plan.py`가 `pyproject.toml`의 classifiers에서 행렬을 짠다: 선언된
  모든 버전은 ubuntu에서 묶음을 돌리고, 3.8 / 3.12 / 3.15는 세 OS에서 돌리며, 남은 10칸은 그
  플랫폼에서 실제로 만든 abi3 휠을 설치해 문서 하나를 YAML·JSON·JSONC·JSON5·TOML로
  왕복한다(`scripts/wheel_smoke.py`, 22-68초). 교차적 전체는 매주 일요일 묶음과 함께 돈다. 실측 결과
  pull request마다 78.2 → 약 49분 (-37%), 묶음 줄은 24 → 14, 아무 칸도 버리지 않았다. 깎기가 아니라
  재구성이 된 이유는 `--cov-fail-under=78`과 준수 요약을
  `matrix.os == 'ubuntu-latest' && matrix.python-version == '3.12'`로 골랐다는 점이다 — 그 칸을
  지우면 두 관문이 **초록인 채로** 꺼진다. 이제 그것들은 줄에 얹은 역할이고, 계획 잡이가 줄 값을
  쓰기 전에 계획을 검증한다. 계획을 고의적으로 망가뜨린 대조 검사 넷과, `3.99` classifier를 더하면
  줄이 늘어야 함을 보는 검사 하나로 이를 증명했다. — (자세히: quality-ledger (cl))

- **YAML 주석·앵커·태그가 왕복하며 모든 시드를 검사한다.** —
  `crates/pyrs-yaml-core/tests/note_survival.rs` 는 커밋된 시드 corpus 를 매 `cargo nextest run` 에
  재생하고, 해석기가 기록한 주석이 모두 출력 텍스트에 나타나는지 요구합니다. 왕복 검사 기준은 텍스트
  멱등성이라「안정적이지만 주석이 한 줄 빠진」문서는 그대로 통과합니다. 조용히 잃은 주석 5 건이 끝내
  초록이었던 이유가 이것입니다. — (details: quality-ledger — Note survival, (w), (ac))

- **중첩·별칭 병합 키가 실제로 적용된다.** — `<<: {<<: {x: 1}}` 은 두 겹 `<<` 를 모두 데이터로
  남겼고（`{'<<': {'<<': {'x': 1}}}`）, PyYAML 과 ruamel 이 읽는 `{'x': 1}` 과 달랐습니다.
  `<<: {<<: {x: 1, y: 1}, y: 2}` 는 `x` 를 아예 잃고, 블록 형태 `- <<:` 도 같았습니다. 수집부는 노드
  전체 비교로「이미 있다」라고 판단하지 않습니다. — (details: quality-ledger (ae), (af), (ag), (o),
  (q), (s))

- **명령어 수 게이트가 포맷 간 교량을 양방향으로 센다.** — `from_jsonc`, `from_json5`,
  `to_jsonc_text`, `to_json5_text`, `to_python_small/medium/anchors` 와 JSON/TOML 읽기 경로가
  `.ci/ir-baseline.json` 기준 값을 갖습니다. 같은 hub AST 에서 세 writer 는
  21,232,786 / 22,176,851 / 22,974,851 명령어: 주석 출력에 4.4%, JSON5 표기에 8.2% — Previously
  말할 수 없었던 숫자입니다. — (details: quality-ledger (am), (ap), (aq), (as), (ax))

- **품질 방어 자체가 측정되고, 구멍은 findings 로 드러난다.** — `scripts/quality_matrix.py` 는
  방어를 선언한 파일（`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`,
  `scripts/check_*.py`, Ir bench, `.ci/ir-baseline.json`）에서 `QUALITY_MATRIX.md` 를 다시 유도하고
  옮겨 적지 않습니다. 보고된 각 구멍은 양방향 탈출구를 적어 `.ci/quality-holes.json` 에 등록되고
  주입 트리로 발화하는 시험을 갖습니다. — (details: quality-ledger (ah), (ai), (aj), (ak), (an),
  (ao))

- **제품을 움직이는 변경은 릴리스 노트도 움직인다.** — `scripts/check_changelog_coupling.py` 는 PR
  의 파일 목록에 두 규칙을 적용합니다: `crates/`, `python/pyrs_yaml/`, `fuzz/`, `scripts/`,
  `tests/`, 동봉 manifest 를 건드리면 changelog 도 건드려야 하고, 다섯 미러 중 하나를 바꾸면 다섯
  모두 바꿔야 합니다. 어떤 기능이 노트 없이 배포된 뒤에 추가된 규칙입니다. — (details:
  quality-ledger (al), (ar))

- **시간 하한은 두 단계를 인접 샘플링하고 근거를 printed 다.** — `tests/timing.py` 가 유일한
  샘플러입니다: 후보와 참조를 같은 블록 안에서 나란히 재고, 판정은 블록 최소값에 최소 두 쌍 승리를
  요구하며, 실패 메시지는 모든 쌍을 인용하므로 빨간 한 번이 어느 쪽이 움직였는지 말합니다.
  `macos-latest` 의 `139.4us vs 359.6us (2.58x)` 보고서에서 다섯 블록 중 셋이 자기 하한의 약 3 배에
  줄선 사례가 이 경우입니다. 교차 라이브러리 하한은 5 배와 top-3 을 유지합니다（실측 PyYAML 대비
  38x-280x, ruamel 대비 71x-77x）. — (details: quality-ledger (aw), (ay))

- **문서는 100 표시 열, 포맷터가 자른 제목은 거부.** — `ROADMAP.md` 에 21,850 자,
  `docs/ja/changelog.md` 에 884 자 줄이 있었습니다. `scripts/check_doc_wrapping.py` 는 표시 열을
  재고（전각은 2 열）`--fix` 를 갖습니다. `check_doc_headings.py` 는 줄바꿈 뒷줄이 issue 참조로
  시작될 때 `rumdl fmt` 가 승격시키는 제목을 거부합니다. — (details: quality-ledger (au), (av))

- **각 엔진에 fuzz 대상, 주간 실행과 PR 단계 blocking.** — 네 개의 libFuzzer 대상 （`parse_yaml`,
  `yaml_roundtrip`, 세 방언 × 세 writer 를 교차하고 각 출력을 다시 파싱하는 `parse_json`,
  `parse_toml`）, 시드는 `fuzz/seeds/`. 추적하는 것은 대상과 시드뿐이고 기계 corpus 는 저장소에
  들어가지 않습니다. — (details: quality-ledger (aj))

- **`pyrs-ast`, `pyrs-schema`, `pyrs-json`, `pyrs-toml` 은 `no_std` 가능.** — 네 crate 는 `alloc`
  만으로 빌드됩니다: `indexmap` 과 `thiserror` 의 기본 `std` feature 를 끄고, 선택형 `std` feature
  가 `std::error::Error` 와 `RandomState` 를 되살립니다. 기본은 `std` 유지이므로 기존 사용자의 노드
  맵 타입은 그대로입니다. — (details: boundaries — Known Engine Boundaries)

- **`pyq` 를 모든 릴리스에 사전 빌드 바이너리로 동봉.** — `publish.yml` 의 `pyq` 잡이 여섯 플랫폼의
  네이티브 CLI 를 빌드해 GitHub Release 에 첨부합니다. 독립 바이너리에 Rust 도구가 필요하지
  않습니다. Linux 는 manylinux2014 컨테이너 안에서 `cross` 로 네이티브 컴파일하고 glibc 2.17 을
  하한으로 고정합니다. — (details: perf — Leaderboard & Performance Status)

- **커밋된 타입 stub 을 빌드된 확장으로 비교한다.** — `scripts/check_stub_drift.py` 는 선언된 경로로
  `python/pyrs_yaml/pyrs_yaml.pyi` 를 다시 생성하고 차이가 있으면 실패시킵니다. 이 잡은
  `windows-latest` 에서 도는데, maturin 의 introspection 경로는 Linux 컨테이너에서 확장을 불러올 수
  없기 때문입니다. 이전에는 존재와 추적만 검사해 공개 타입 계약이 조용히 뒤처질 수 있었습니다.
  — (details: quality-ledger (ap))

- **스키마 언어가 컨테이너 형태를 말할 수 있다.** — `type: map` 과 `type: seq` 은 내용을 문제 삼지
  않고 노드의 형태만 주장하고, `map` / `seq` 은 구성 요소 타입으로도 쓰이므로 `mapping_of: seq`
  로 "이 맵핑의 모든 값은 시퀀스"라고 적을 수 있습니다. `[*]` 는 시퀀스 인덱스에만 닿고 맵핑 키에는
  닿지 않아서 이 주장은 표기가 없었습니다. `mapping`, `object`, `sequence`, `array`, `list` 도
  받아들입니다 — 스키마는 도구가 아니라 사람이 치는 글이기 때문입니다. `path` 없는 컨테이너 타입은
  적힌 자리에서 거부됩니다. 지명된 노드가 없으면 아무것도 고르지 못하고 아무것도 검사하지 않습니다.
  — (details: quality-ledger (bm))

- **지침 파일도 신뢰하지 않고 측정한다.** — `scripts/check_hole_claims.py` 는 `AGENTS.md` 와
  `CLAUDE.md` 에 지명된 품질 구멍 id 가 매트릭스가 지금 측정하는 id 임을 강제합니다. 이 검사가 생긴
  이유는 `AGENTS.md` 자체입니다. 한 마일스턴 동안 그 파일은 `mkdocstrings` 이 "아무것도 렌더링하지
  않는다"고 적고 있었습니다 — #321 이 이미 닫은 틈을 현재 시제로 서술한 지침이 다음 작업자를 잘못된
  방향으로 이끌고 있었습니다. 측정은 맞았습니다. 매트릭스는 `.ci/quality-holes.json` 과
  `QUALITY_MATRIX.md` 를 양방향으로 비교하되 파생 문서에서 멈췄습니다. 토큰은 그 kind 를 probe 가
  출력할 수 있을 때만 구멍 id 로 봅니다 — kind 는 옆에 옮겨 적지 않고 `quality_matrix.py` 에서
  읽습니다 — 따라서 본문 속 `line:column` 은 주장이 아닙니다. 끝까지 읽을 수 없는 어휘에서는 2 로
  끝나고 통과한 척하지 않습니다. 날짜가 있는 기록은 id 를 남깁니다. 역사는 닫힌 구멍을 이름 삼을 수
  있지만 지침 파일은 현재 시제이기 때문입니다. — (details: quality-ledger (bn))

- **게시되는 서명을 페이지가 아니라 출하된 대상과 비교한다.** — `scripts/check_doc_signatures.py` 는
  28개 API 페이지에서 서명 형태의 펜스 블록을 모두 읽어, 실행 중인 패키지가 그 파라미터를
  받아들이는지 묻는다 (238개 블록, 279개 이름). 파라미터 이름을 틀려도 HTML 로서는 타당하므로
  사이트는 초록 상태로 게시된다. 생성된 `.pyi` 만 읽는 검사기는 `read_markdown(path=...)` 에서 문서
  탓을 했을 것이다 - 문서가 맞았던 유일한 경우다. 정당한 축약(필수 앞부분만 쓰고 기본값이 있는
  나머지는 생략)은 통과하고, 어떤 선언에도 없는 이름·순서· 빠진 필수 파라미터는 실패한다.
  `tests/test_stub_runtime_parity.py` 는 그 아래층을 고박한다 — 스터이의 40개 모듈 함수 파라미터
  이름은 대상 자신의 이름과 같아야 하고, 이 불변식을 살피지 않은 것이 두 결함의 공통 원인이었다. —
  (details: quality-ledger (bo))

- **표준 `!!` 태그가 로드되는 타입을 결정합니다.** — `!!str`, `!!int`, `!!float`, `!!bool`,
  `!!binary` 는 문서의 암시적 스키마가 아니라 태그 자신의 정의로 읽으며, 두 가지 표기 모두를
  취급합니다: 축약형과, `Node.set_tag` 가 쓰는 완전형 `!<tag:yaml.org,2002:str>`. PyYAML 과 ruamel
  에게 실측한 결과 두 라이브러리가 모든 사례에서 일치합니다: `!!str 1.20` 은 문자열 `"1.20"`
  (이전에는 실수 `1.2` 로, 타입이 바뀐 데다 뒷자리 0 도 사라졌습니다), `!!float 1` 은 `1.0`,
  `!!bool yes` 는 `True`, `!!binary aGk=` 는 `b'hi'`, i64 를 넘는 `!!int` 도 정수입니다. 태그의
  타입에 맞지 않는 텍스트는 문자열을 돌려주지 않고 `YamlTypeError` 를 냅니다. 알 수 없는
  애플리케이션 태그는 관대하게 남깁니다 — 플러그인 체계가 그 이름으로 주소를 찾습니다 — 그리고 이
  선택은 테스트로 적어 두었습니다. — (details: quality-ledger (bq))

- **해석기 하한이 실제로 무너지는 자리에서 검사한다: import 때가 아니라 호출 때.** —
  `tests/test_scripts_import_on_supported_python.py`는 모든 줄에서 `scripts/*.py`를 import 해 왔고,
  자기 docstring에 보이지 않는 절반을 적어 두었다: 하한보다 젊은 API는 *함수가 실행될* 때 터지므로
  3.8에서 깨끗이 import 되는 모듈도 거기서 죽는다. 그 절반이 행렬 한 바퀴를 삼켰다. 이제 `scripts/`,
  `tests/`, 배포되는 `python/` 패키지를 도는 정적 훑기가 목록을 갖는다: `newline` 옵션이 붙은
  `write_text`/`read_text`, `removeprefix`, `zip(strict=)`, `functools.cache`, `int.bit_count`,
  `datetime.UTC`, `StrEnum`, `ExceptionGroup`, `typing.override`. 문자열 리터럴과 주석을 먼저
  지우므로 API를 *이름 짓는* 것은 *호출*하는 것이 아니다. 모든 패턴은 대조 입력 하나로 집합 동일하게
  검증한다 - 죽은 정규식이 "깨끗한 트리"를 사칭하지 못하게. 후보 둘은 실측으로 기각됐다:
  `\.walk\(\)`가 돌려준 7건은 이 프로젝트 자체의 `walk()` 동사였고, `@override\b`가 잡은 것은
  저장고가 표준으로 쓰는 `typing_extensions.override` 어댑터였다. 자기가 단속하는 코드와 이름이
  부딪히는 패턴은 없는 것보다 나쁘다 - 고치는 방법이 면제뿐이고, 면제는 API보다 오래 남는다. —
  (자세히: quality-ledger (cn))

- **유효 문서마다 JSON 작성기를 참고 구현으로 다시 읽는 관문이 늘었다.** — 말뚝은 분석기가
  *수락하는* 문서만 지켰고 작성기가 *무엇을 내미는* 것은 무방문이었다. 그래서
  `to_json`·`to_jsonc`·`to_json5`는 이 저장소 자신의 리더하고만 대조됐다. 이제 유효 문서 95개 전부를
  stdlib `json`으로 다시 읽고(재읽기 190회), `to_json5`는 `pyjson5`로 다시 읽는다. 동등 비교는 타입
  엄격하다. Python에서 `[1] == [1.0]`과 `0 == False`는 모두 `True`라 값만 보는 비교는 #358이 고친
  결함 그대로를 통과시킨다. 증거는 변이 대조 세 개다. 정수를 `{i}.0`으로 쓰면 재읽기 190개 중 38개,
  JSON5 95개 중 4개가 빨개지며 입력과 출력과 실제 텍스트를 이름으로 짚는다. 작성기의 pair 반복을
  뒤집으면 문서 여섯 개가 빨개지고, 문서 순서로 키를 도는 값 비교가 보고한다. 모든 멤버를 두 번 쓰면
  — 여전히 유효한 JSON이고 중복은 접히므로 dict로는 양쪽이 같다 — pair 목록에만 드러난다. 190개 중
  18개. 첫 대조는 시도 당시 초록이었고, 그것 자체가 발견이었다 — `write_plain`이 fidelity 경로로
  JSON 표기 숫자를 그대로 돌려주므로 `from_json`으로 들어온 말뚝은 정규화 분기에 닿지 않으며, 보이게
  하려면 그 경로까지 꺼야 했다. 그 분기가 존재하는 이유인 YAML 표기(`0x1F`가 `31`, `!!str 7`이
  `"7"`)는 Rust 쪽 `exotic_scalars_quote_or_normalize`가 고정한다. 관문이 보지 못하는 것은 각주가
  아니라 관문 자신의 docstring에 적었다. 말뚝에는 주석과 앵커가 없으니 #359의 주석 슬롯은 계속
  `tests/test_json.py`의 몫이다. — (details: quality-ledger (co))

#### 변경

- **테스트 계층의 차이 렌더러를 `similar` 3.2.0으로 올렸습니다.** — 이 crate는
  `pyrs-yaml-core`의 `[dev-dependencies]` 항목 하나이고, 호출 지점은 왕복 단언이 실패할 때 기대값과
  실측값의 차이를 인쇄하는 세 개의 테스트 보조 함수(`pbt.rs`, `serializer.rs`, `splice.rs`)뿐이므로
  출하되는 코드 경로는 이를 링크하지 않습니다. 사용하는 API는 `TextDiff::from_lines` 하나이며,
  이 major를 그대로 넘었습니다: core 테스트 377개(속성 테스트 계층 포함)가 수정 없이 3.2.0을
  통과합니다.

- **CI가 std 없이 쌓은 모든 crate의 std 빌드를 다시 검사합니다.** — `no_std` 잡의 마지막 단계는 기초
  crate 다섯 중 둘만 적어 두었습니다. 나머지는 `std`와 함께 깨져도 그 항목은 초록으로 남았으니, 기능
  분리가 한 방향으로만 증명된 셈입니다. 이제 다섯을 적습니다(이 자리에서 `cargo check --locked`가
  다섯 모두 깨끗이 통과함을 실측). `tests/test_no_std_gate.py`는 workflow 본문에서 두 단계의 crate
  목록을 읽어 멀어지는 것을 거부합니다. 고치기 전 단계 문구를 되돌리면 세 crate를 지목해 빨개짐을
  확인했습니다. — (자세히: quality-ledger (dp))

- **스텁 생성기를 maturin 1.15.0 으로, 의도적이고 실측한 뒤 옮겼습니다.** — 버전을 고정하는 이유는
  `python/pyrs_yaml/pyrs_yaml.pyi` 가 한 정해진 버전에서 만들어지고 `scripts/check_stub_drift.py` 의
  규칙이 그 출력을 설명하기 때문입니다. 따라서 maturin 이동은 그 자체로 변경이어야 하고,
  `tests/test_docs_config_gate.py` 는 몰래 이동하면 빨개지도록 존재합니다. 이 트리에서 실측한 결과
  1.15.0 의 스텁은 1.14.1 과 한 바이트도 차이가 없습니다(608 줄, 그리고 표기 검사기가 이미
 다시 만든 결과물과 일치한다고 보고합니다). 그래서 규칙도 기대 계수도 바뀌지 않았습니다. 움직인 것은
  생성기의 쓰기는 자리뿐이며, `--out` 은 이제 `target/stubs/pyrs_yaml/pyrs_yaml.pyi` 를 받습니다.
  이것이 `GENERATED` 경로입니다. 채택 이유는 상류의 `abi3t` `PyModExport` 진입점 검사 수정이며, 이
  프로젝트의 `abi3t` 와 cp314t 휠이 타는 경로입니다.

- **왕복 속성 층은 차이를 모두 이름 지어 필터 뒤에 숨지 않는다.** — 25개 속성 중 일곱은
  `if let Some(parsed) = try_roundtrip(&node)` 안에서만 주장했으므로 자체 헬퍼가 되돌리기를 거부한
  표본은 결코 말해지지 않았다. 실측으로 그 필터는 뽑은 4000개 트리 중 3423개를 버렸다.
  `try_roundtrip`은 사라졌고, 모든 왕복은 허용 가능한 정규화의 열거 목록 - `NullToken`,
  `PlainRequoted`, `BlockRestyled`, `EmptyContainerFlow`, `NoteRehomed`, `NoteForcesBlockLayout`,
  `InlineContainerFlow` - 으로 분류되며 목록이 이름을 지을 수 없는 차이는 속성을 실패시킨다.
  이것이 #418부터 #422 그리고 #424 뒤에 있던 기록자와 읽기의 결함을 표면으로 끌어냈고, 각각 자기
  테스트로 고정했다. 그리고 `property-tier:conditional-assert`를 닫는다: 프로브는 빈 가드 목록을
  재고, `.ci/quality-holes.json`은 비었으며, `QUALITY_MATRIX.md`의 행은 닫힌 채로 남는다. 두 순서
  속성은 필터로 되돌리지 않고 주장을 고쳤다 — 키는 직렬화된 텍스트가 아니라 형표시된 항목으로
  비교하고, 맵핑 순서는 위치별로 비교한다. 컨테이너가 flow로 읽혀 돌아오는지는 읽기의 정규화이지
  순서의 사실이 아니므로.

- **태어 문법이 PyO3 바인딩에서 떠나고, 그 테스트는 더 이상 libpython을 필요로 하지 않는다.** —
  `!!int`, `!!float`, `!!binary`의 어휘 해석은 `crates/pyrs-yaml/src/py/tags.rs`에 있었기에 그 여섯
  개의 테스트는 libpython을 링크했다. 이들이 부르는 함수는 `&str`만 받아 Rust 값을 반환하는데도,
  Cargo의 lib 의존성은 test 대상에도 적용되고 `pyo3`는 `extension-module` 없이 선언돼 있다. 문법은
  이제 `pyrs_yaml_core::tagged`에 있고, Python이 필요 없는 계층은 통과 시험이 526개에서 532개로
  늘었으나 workspace 합계는 604 그대로다. 바인딩에는 값을 `Py<PyAny>`로 바꾸는 부분만 남는다.

- **`granit-parser` 1.3 → 2.0.** — major 올림을 실측 후 적용했다. 소스 변경 없이 타입 체킹이
  통과한다(지워진 공개 헬퍼 `found_tabs`와 `has_valid_yaml_ws` 모두 `crates/`와
  `python/` 어디에서도 참조되지 않습니다). `cargo nextest run --workspace --exclude pyrs-yaml`은 579
  통과, `uv run pytest tests/ -k "not numpy"`는 2678 통과 / 10 건너뛰기입니다. 핵심은 후자인데,
  컴파일로는 보증할 수 없는 주석·앵커·라이브러리 간 규격 계층까지 통과하기 때문입니다. 게이트가
  고정하는 rustc 1.97.1에서 `scripts/ir_gate.py`는 "모든 시나리오가 허용 오차 이내"로 끝납니다
  (`to_python_medium` +0.26%, `from_json_medium` −0.00%). `cargo tree`가 v2.0.0을 확인했습니다.

- **두 mapping 키는 리더가 그렇다고 보는 곳에서만 같은 키입니다.** — 두 키가 텍스트에서 충돌하는지
  정하는 `keys_collide_in_text` 규칙은 scalar의 원시 값만 비교하고 나머지는 무시했습니다. `!a k:`와
  `!b k:`를 한 키로, `null:`과 `'null':`도 한 키로 불렀지만, 내보낸 텍스트를 다시 읽는 리더는 양쪽
  모두 항목을 둘로 유지합니다(태그는 줄의 일부이고, plain `null`은 null 키로 해석되는 따옴표 표기는
  문자열로 남습니다). 이 규칙은 쓰기의 접힘이 뒤에 쌓을 모델인지라, 지나치면 문서가 가질 수 있는 두
  항목을 접어버립니다. 이제 태그를 비교하고, plain null 표기와 그 따옴표 짝을 다른 키로 봅니다.
  불리(bool)나 수치로 해석되는 plain 표기는 면제되지 않습니다. 리더가 실제로 텍스트를 비교하기
  때문입니다. 규칙과 그것이 본뜨는 쓰기와 읽기를 함께 고정하는 표가 15 개 급에서 18 개로 늘어납니다.

- **스키마 검증이 텍스트보다 태그를 먼저 묻습니다.** — validate 규칙은 스칼라의 *철자*만 봤기 때문에
  `v: !!str 1`을 쓴 문서는 int를 요구하는 규칙을 통과했습니다. 그런데 loader는 그 값을 문자열
  `"1"`로 읽고 JSON과 TOML 쓰기도 문자열로 내보냈습니다. 하나의 AST에 두 의미가 있던 셈입니다(#340).
  이제는 표준 태그(`!!str`, `!!int`, `!!bool`, `!!float`, `!!null`, 그리고 완전 형식
  `!<tag:yaml.org,2002:...>`)라면 태그에 따라 해석합니다. 쓰는 문법은 spoke 쓰기가 이미 쓰는
  것(`standard_tag_kind`와 `TagKind::resolve`)이라, 같은 문법을 두 곳에서 맞추 필요가 없습니다.
  텍스트가 충족하지 못하는 태그는 문자열로 다시 고치지 않고 거부합니다. `!!int hello`는 더 이상
  문자열로 통과하지 않습니다. 규격이 정하지 않은 태그(`!x 1`)는 기존 암시 해석을 그대로 쓰므로
  여전히 숫자입니다. `!!int 0b1010`은 선언한 대로 정수 10으로 통과합니다. 어휘 집합은 loader가 이미
  구현한 YAML 1.1 것입니다.

- **`CustomNode` 는 144 바이트, 208 이 아닙니다.** — 노드의 절반은 `NodeMeta` 였습니다. 208 바이트
  중 128 바이트, `Option` 슬롯 다섯 개였고 그 크기는 모든 읽기 경로의 한복판에 있었습니다. 노드를
  지을 때 쓰고, 노드가 옮겨질 때마다 복사하고, 트리를 버릴 때 필드마다 확인합니다
  (`docs/dev/perf.md` — `memcpy` 가 `from_json_medium` 의 7.88%, 해체가 4.58%). 두 단계는 각자
  무료가 아니므로 한 변경으로 함께 냅니다. 이제 `pyrs-ast` 를 제외한 모든 크레이트가 이 타입 고유의
  접근자로 슬롯에 닿습니다. `comment()`, `set_comment()`, `take_comment()`, 그리고 anchor, tag,
  source range 도 같은 세 가지씩. 앞줄 주석 상자는 `decor_mut()`, 직렬화기가 패턴으로「이 노드에
  comment, anchor, tag 가 없는가」를 물던 자리는 `is_identity_clean()`, 모든 필드 이름을 적던 구조체
  리터럴은 `with_comment()`, `with_tag()`, `scanned()` 로 바꿨습니다. 그리고 anchor 나 tag 가 명시된
  소스만 채우는 두 슬롯은 이제 포인터 하나를 공유합니다. 관문이 고정하는 컴파일러에서 짝을 지어
  측정했습니다 (rustc 1.97.1): `from_json_medium` −4.8%, `serialize_anchors` −4.0%,
  `parse_inline_merge` −3.5%, `parse_medium` −1.5%, `from_toml_medium` +0.7%, 0.50% 허용 범위를
  벗어나는 항목은 없습니다. 어느 필드를 옮길 수 있는지가 이번 발견입니다. source range 까지 상자에
  넣으면 — YAML 어댑터가 *모든* 노드에 적어 두는 값 — `parse_medium` 이 +5.9% 를 지불하고
  `from_json_medium` 의 −7.4% 를 사는 결과가 됩니다. 그래서 노드마다 기록되는 슬롯은 평면에 두고
  `{anchor, tag}` 만 상자에 넣었습니다. `node_layout_is_part_of_the_hot_path_contract` 가 두 구성의
  최종 크기를 고정합니다. 메타데이터는 72 와 64 로 같고, 노드는 기본 `std` 에서 144, 노드 맵 해셔가
  다른 `no_std` 에서 128 입니다.

- **TOML 2진 정수가 자기 표기를 지킨다.** — `b = 0b1010` 이 `b = 10` 으로 돌아왔다. 이 spoke 는
  의도적으로 정규화했다: YAML Core 에 `0b` 어휘가 없으므로 태그 없는 plain `0b1010` 은 허브에서
  *문자열*로 다시 읽히고, 설계 규칙 D1 (YAML Core 도 받아들이는 표기만 지킨다) 은 값을 망가뜨리지
  않는 대신 표기를 버렸다. 브리지가 태그를 읽는 것을 배운 순간 그 근거는 끝났다 (`(bu)`, #340 이
  정한 순서). 이제 투영은 텍스트를 지키며 타입을 밝힌다: `from_toml("b = 0b1010")` 은 허브 스칼라
  `!!int 0b1010` 이고, 로더는 `10` 으로 읽고, `to_toml` 은 `0b1010` 으로 되쓰며, JSON 기록자는 `10`
  으로 투영한다. 한 문서, 한 타입, 그것을 보는 모든 포맷에서 같은 답이고, TOML 텍스트는 한 바퀴에서
  안정된다. 여전히 충실하지 못한 것은 TOML 스스로 적을 수 없는 것뿐이다: `0b1101_0110` 은 구분자를
  잃고, 손으로 만든 *태그 없는* `0b1010` 은 아직도 문자열로 다시 읽힌다 — 타입을 실어 나르는 것은
  태그다. 미결정으로 남은 `!!binary` 매핑은 영향을 받지 않으며 그 issue 에 남아 있다. — (details:
  quality-ledger (cp))

- **커버리지 게이트가 검사 묶음 전체를 중복 실행하지 않는다.** — `coverage` 는 venv 을 따로 만들고
  ubuntu 휠을 내리고 세 코퍼스를 클론한 뒤, `test` 가 이미 덮은 인터프리터(ubuntu/3.12)에서
  `pytest tests/` 를 한 번 더 돌렸다. 이제는 그 줄에 플래그 하나를 더하는 것뿐이고
  (`--cov-fail-under=78` 은 그대로), 세 개의 클론 단계는 공용 composite action
  (`.github/actions/fetch-corpora`) 으로 옮겨 묶음 전체를 돌리는 두 잡이 함께 쓴다. 바로 그 모양이
  클론 게이트가 잡아야 할 구멍을 숨길 수 있다 — 전부 받는 잡이 아무것도 받지 않는 것처럼 보인다 —
  그래서 `tests/test_ci_corpus_gates.py` 는 지역 `uses:` 를 action 자신의 단계까지 따라간다. 새
  검사는 양쪽을 모두 박아 둔다: 해석된 이름은 모든 코퍼스를 덮어야 하고, 해석할 수 없는 경로는 모든
  누락을 보고해야 한다. action 디렉터리 이름을 바꾸면 검사 셋이 빨개지니, 이 추적이 실제로 무게를
  받고 있다는 증거다. 팬인의 `needs:` 에서 `coverage` 를 뺀 것은, 그것을 파일 안의 모든 잡과 견주는
  게이트가 요구하는 대로다. — (details: quality-ledger (ci))

- **`bytes` 값은 뜻하는 태그로 기록됩니다.** — 내장 플러그인은 `!binary` 를 출력했습니다. *지역*
  태그입니다. 표준 이름은 `!!binary` 이므로, 이 라이브러리가 쓴 이진 문서 이름을 빌려 쓴 쪽
  라이브러리에서는 읽을 수 없었고, `{"d": b""}` 는 `!binary` 뒤에 아무것도 없이 기록되어 다시
  읽으면 `None` 이 되어 플러그인 안에서 `TypeError` 를 냈습니다. 이제 빈 페이로드를 포함해
  `!!binary` 를 출력하고, PyYAML 이 우리가 쓴 것을 읽는다는 것도 검증했습니다. 이미 디스크에 있는
  문서를 위해 지역 표기는 로드용으로 계속 등록됩니다 — 지우면 표기 수정이 데이터 손실이 되기
  때문입니다. — (details: quality-ledger (bq))

- **`path` 를 적은 검증 규칙은 그 노드를 주장한다.** — 변경 전에 측정했습니다: `$.config` 에
  `mapping_of: str` 을 두면 `config` 가 스칼라이든 시퀀스든 통과했고, `$.port` 의 `type: int` 는
  `port` 가 맵핑이어도 통과했으며, `sequence_of: int` 는 `[[1, 2]]` 를 통과했습니다. 모든 검사가
  자기가 설명할 수 있는 한 종류 노드에 대한 `if let` 안에 있어서 다른 종류는 그대로 지나쳤습니다 —
  보지 않은 문서에 "타당"하다고 답하는 검증기입니다. 구성 요소도 같은 기준으로 검사됩니다. 중첩된
  시퀀스는 `int` 가 아닙니다. `path` 없는 규칙은 기존의 다른 뜻을 유지합니다. 노드를 지명할 수
  없으니 설명할 수 있는 노드를 형태로 고릅니다 — 경로 없는 `type: str` 은 "모든 스칼라는
  문자열"이라는 뜻이고 문서의 나머지는 문제 삼지 않습니다. 별칭 노드에 대한 규칙은 판정하지
  않습니다. 별칭은 형태가 아니라 이름을 담고 있고 검증기는 앵커 표를 두지 않기 때문입니다. —
  (details: quality-ledger (bm))

- **`[*]` 는 한 요소를 가리키고 부분 트리 전체를 가리지 않는다.** — `$.rows[*]` 는 `$.rows[0].a`
  와도 일치했습니다. 접두사와 접미사를 비교하고 그 사이에 무언가가 있기만을 요구했기 때문입니다.
  규칙이 형태를 무시하던 동안에는 보이지 않다가, 형태를 주장하는 순간 일치한 요소 아래 모든 값이
  거짓 위반이 되었습니다. `$.rows[*].a` 는 여전히 지명한 구성 요소에 닿습니다. — (details:
  quality-ledger (bm))

- **검증 규칙 하나는 검사 항목을 하나만 가진다.** — `type` 과 `mapping_of` 를 나란히 적은 규칙은
  나중에 적힌 검사만 실행하고 앞의 항목은 언급하지 않았습니다. parser 의 모든 팔이 같은 필드에
  대입했기 때문입니다. 이제 한 규칙에 검사가 둘이면 스키마 해석이 실패합니다. `required` 는
  직교하므로 어떤 검사와도 결합할 수 있습니다. — (details: quality-ledger (bm))

- **엄격한 JSON 내보내기는 비유한 부동소수를 다른 표기로 바꾸지 않고 거부합니다.** — RFC 8259 에는
  무한대나 NaN 의 리터럴이 없어서 `to_json` / `to_jsonc` 는 허브의 텍스트에 따옴표를 씌운
  `{"b": ".inf"}` 로 답했고, `load_json` 은 그것을 문자열로 되돌려 주었습니다. 나가는 길에 숫자가
  문자열이 되어 그 텍스트를 값으로 지닌 문서와 구별할 수 없었는데, 이는 #312 이 들어가는 길에 고친
  것과 같은 종류의 손실입니다. 대안은 저울에 달지 않고 측정했습니다: `null`(`JSON.stringify` 의 답)
  은 다른 수이고, 맨 `Infinity` 는 이 라이브러리의 엄격한 해석기가 의도적으로 거부하는 텍스트입니다
  (여기서 CPython 의 `json.loads` 보다 엄격합니다). 이제 내보내기는 안정 사유 키
  `json-cannot-represent-non-finite` 를 실은 `YamlSerializeError` 를 던지고, 그 어구를 가진
  `to_json5` 는 왕복을 지킵니다. 거부가 내부 고장을 가장하지도 않습니다: 새 변형
  `SerializeError::UnsupportedValue` 는「그 형식은 이 값을 담을 수 없다」전용이며, 이전에는
  키·별칭·null· 타임스탬프가 `internal-error` 로 보고되던 곳이 바로 그 곳이었습니다.

- **`pyq validate` 가 `--input` 을 받고 실제로 읽은 형식을 따른다.** — YAML 파서를 하드코드해서
  `pyproject.toml` 이나 `package.json` 은 곧바로 거부됐습니다. 이제
  `--input auto|yaml|json|jsonc|json5|toml` 을 받고 공용 loader 경로를 탑니다.

- **PR 단계 fuzzing 은 blocking 이라 충돌을 merge 할 수 없다.** — 단계별 `continue-on-error` 는
  `main` 이 이 단계가 옳게 짚은 드리프트를 안고 있는 동안만 두는 임시 고정장치였습니다. 현재 트리로
  재검증: 네 대상 모두 커밋된 시드를 깨끗이 재생합니다（5/59/6/5 시드, nightly-2026-08-15,
  cargo-fuzz 0.13.2）. — (details: quality-ledger (aa))

- **문서 도구 사슬을 최신으로, stub 생성기는 의도적으로 유보.** — `zensical` 0.0.56 → 0.0.69,
  `mkdocstrings-python` 1.11.1 → 2.0.9（`mkdocstrings` 1.0.6, `griffelib` 2.2.0, 모두 Python 3.11
  필요）. 한편 `maturin` 은 1.14.1 그대로입니다 — 커밋된 `.pyi` 와 드리프트 검사 조정 규칙이 그
  출력을 전제하기 때문. 설정된 14 개 handler 옵션은 설치된 `PythonOptions` 와 비교했고, 알 수 없는
  옵션은 빌드가 조용히 무시하므로 검사는 시험 쪽에 두었습니다. — (details: quality-ledger (ba))

- **API 참조는 라이브러리 자체 docstring과 형식 stub에서 생성됩니다.** —
  `docs/<locale>/api/{reference,yaml-document,yaml-instance,node,merged-view}.md` 에 `mkdocstrings`
  디렉티브를 넣어, 사이트의 서명은 손으로 옮긴 것이 아니라 코드에서 나옵니다. 네 로케일 모두
  빌드하고 검증했습니다: `YamlDocument` 에 서명 23 개, `Node` 에 41 개, 모듈 페이지에 48 개,
  디렉티브가 없는 쪽은 0 그대로. `exceptions.md` 는 손글씨로 남깁니다 — 생성된 stub 이 클래스를 넷만
  선언하고 예외 형식은 하나도 포함하지 않기 때문입니다. — (details: quality-ledger (be))

#### 수정

- **인라인 테이블 안의 점 표기 키는 TOML이 정의한 대로 중첩된다** — `animal = { type.name = "pug" }`
  는 규격 자체가 주는 예시이고, 뜻은 `[animal.type]` 에 `name = "pug"` 다. 그런데
  `parse_inline_table` 은 점으로 이은 텍스트를 단일 키로 저장했다. 다시 읽으면
  `{"type.name": "pug"}` 가 되고, 더 나쁜 문제로 `animal = { "type.name" = "pug" }` 까지 똑같은
  형태로 읽혀, 서로 다른 데이터를 둔 두 문서가 로드 뒤에 구실을 잃었다. 이제 구성원은 적힌 경로를
  그대로 보관하고 `nest_inline` 이 그 경로가 가리키는 트리로 펼친다. 같은 접두사를 공유하는 구성원은
  중간 테이블 하나를 쓰고(`{ a.b = 1, a.c = 2 }` → `a` 하나에 `b` 와 `c`), 각 층은 자신의 이름을
  정하는 분절의 오프셋을 보고한다. 이는 `register_kv` 가 표 수준의 점 표기 키에 이미 적용하는 규칙과
  같으므로 두 갈래가 이제 일치한다. 실측: toml-test 유효 디코딩 단계가 **248/266 (93.23%) → 257/266
  (96.62%)** 로 움직였고, 정확히 이 원인이 차지한 아홉 사례 (`inline-table/key-dotted-*` 일곱 파일과
  두 버전 트리의 규격 예시 하나씩) 가 해당한다. 인코딩 단계는 **268/268** 을 지키는데 — 써 내는
  `{a = {b = 1}}` 은 원문 표기가 아니지만 CPython 의 참조 디코더 `tomllib` 가 같은 값으로 다시
  읽는다. 그 단계가 약속하는 것이 바로 그것이다. 두 하한은 결함의 나머지 절반이었다: 실측 93.2% 인데
  `0.85` 를 두면 디코딩 쪽 22개, 거절 쪽 57개분의 여지가 남아 아홉 사례 결함 클래스가 초록 백분율
  뒤로 숨었다. 이제 두 하한 모두 이 트리가 실제로 달성한 값에서 사례 하나 아래 (`0.966` 과 `0.954`)
  에 놓이고, 실패한 사례 이름을 비율만이 아니라 메시지에 적는다. 테스트 6 개 추가: Rust 5 개(규격
  예시, 일반 키를 사이 둔 형제 구성원의 병합, 따옴표 키가 여전히 키 하나, 네 분절 경로, 다시 쓰고
  다시 읽기) 와 Python 1 개(두 형태의 `load_toml` 에 허브 왕복). 뮤테이션으로 귀인: 중첩 쓰기를 옛
  이음 키로 되돌리면 그중 넷이 빨개지고, 따옴표 키 대조는 초록을 지킨다 — 바뀌면 안 되는 사례이기
  때문이다. 하한이 고정하는 컴파일러로 명령 수를 세면 `from_toml_medium` 은 커밋된 기준선 대비
  +0.0094%, `to_toml_medium` 은 +0.00017% 로, 같은 소스로 `docs/dev/perf.md` 가 기록한 최악의 호스트
  간 드리프트 +0.1639% 안에 든다. 따라서 어느 방향으로도 속도 주장을 하지 않는다. 거절 쪽 하한은
  CPython 3.14/3.15 다리의 읽은 값(488/511)을 취한다: 범위 밖의 시각 세 건은 그 해석기들에서 이
  파서가 아니라 숙주의 `datetime` 에 의해 거절되기 때문이고, 그 분열은 엔진이 짊어져야 할 결함이지
  줄을 늦추는 허가가 아니다.

- **키 자리에 온 별칭은 앵커가 가리키는 값으로 읽는다** — `a: &b b` 와 `*b : a` 는 PyYAML 에서
  `{'a': 'b', 'b': 'a'}` 가 되고, 스위트의 E76Z 와 26DV 는 바로 그 답을 기대 JSON 으로 적어 둔다.
  그런데 이 엔진은 별칭 키라면 예외 없이 `found unhashable key` 로 거부했다. 대상이 아니라 노드의
  종류로 판단한 결과라 위 두 사례를 잃었다. 이제는 해석된 형태로 판단한다: 맵이나 열을 가리키는
  별칭은 그대로 거부하고(실측하니 PyYAML 도 같은 지점에서 raise 한다), 앵커가 없는 별칭은 문서가 한
  번도 적지 않은 `None` 키가 되는 대신 거부한다. 준수 측정 쪽도 올바른 해석을 실패로 세는 일을
  그만두었다 — `!!binary` 는 `bytes` 로 로드되어야 하는데 `json.dumps` 가 이를 표기할 수 없어서 565N
  은 "JSON comparison error" 로 보고됐다. 보고는 **405/406 (99.75%)** 으로 돌아왔고, 그 숫자는
  `docs/en/features.md` 가 알리는 값이다. 이제 그 값은 인쇄만이 아니라 강제된다:
  `test_compliance_report` 의 `rate >= 0.95` 여지(스위트 사례 21개분이며 402로 미끄러져도 아무도
  몰랐다)를 `rate >= 0.995` 로 올리고, "모든 실패는 의도적 거부인 `ZYU8` 이어야 한다"를 더했다.
  실측: `uv run pyrs-yaml compliance` 는 405/406 이고 `failed_tests` 는 `[ZYU8]`,
  `uv run pytest tests/test_parse.py tests/test_yaml_crosslib.py tests/test_yaml_suite.py` 는 211
  통과(신규 3: 별칭 키 투영과 그 다시 읽기, 그리고 공백 없는 `*b: a` 를 granit 은 거부하고 PyYAML 은
  받아들인다는 파서 경계). 미측: 명령수 — 레그는 이 브랜치를 여전히 심판하지만 맨 스칼라 키 경로는
  손대지 않았으므로 이동을 주장하지 않는다.

- **중복 JSON 키는 이제 자신의 주석을 데려가지 않는다** — 맵은 첫 키 노드와 마지막 값을 유지하므로,
  이름이 반복되면 새로 들어온 키와 대체된 값이 버려졌고, 예전에는 두 노드 중 하나가 소유한 주석까지
  함께 사라졌다: `{"a": 1 // first , "a": 2}` 는 `first` 없이 돌아왔다 — 나중에 중복 키가 짝을
  덮어쓰는 JSONC / JSON5 문서에서 언제나, 조용히. 이제 주석은 살아남은 짝으로 옮겨지고, 단 하나의
  부착 규칙을 임시 노드 위에서 다시 실행해 만들어진다. 규칙을 옮겨 적는 방식이 아니다. 덮어써진 짝
  안의 텍스트 순서는 슬롯 하나에서 복구할 수 없으므로, 구해진 주석은 살아남은 것 곁에 놓인다. 두
  번째 라운드는 같은 텍스트를 쓴다 — 고정점 계층이 요구하는 것도 바로 그것이다. 주석이 없는 키 —
  읽기 게이트가 재는 문서의 모든 키 — 는 분기 하나만 치르고, 중복 키가 없는 문서는 그 외에는
  아무것도 치르지 않는다. 이 결함은 한 JSON 문서의 두 순회를 서로 대조하는 퍼징으로 발견됐다
  (대조하는 리플레이 표적은 이 변경 바로 다음에 도착한다).
  `a_duplicate_key_keeps_the_notes_its_own_nodes_carried` 가 이를 고정하고, 이 이관을 제거하면
  붉어진다.

- **`version()`은 읽는 쪽의 버전이 아니라 문서의 `%YAML` 디렉티브를 보고합니다** — granit은
  `Event::DocumentStart`에서 선언된 버전을 건네지만 AST 쪽이 그 인자를 버렸기 때문에, `1.1`·`1.2`·
  `1.3`을 선언한 문서나 아무것도 선언하지 않은 문서나 모두 `1.2`로 답했습니다. 이제
  `parser::parse_document`와 `parser::parse_all_documents`는 디렉티브를 노드와 함께 돌려주고 문서
  층이 그것을 보고합니다: 흐름은 문서마다 답하고 `reparse()`도 다시 읽습니다. 실측: 선언된 버전은
  각자 그대로 답하고, 미선언 문서 여전히 `1.2`입니다(이 읽는 쪽의 자체 버전 — “미설정”을 뜻하는 값
  때문에 이미 배포된 `-> str`를 `Optional[str]`로 넓히지 않음). 디렉티브는 해석을 바꾸지 않습니다 —
  Core 스키마에서 `%YAML 1.1`도 `yes`를 문자열로 둡니다. 범위는 보고뿐입니다: 기록자는 디렉티브를
  다시 내보내지 않으니, 선언된 문서의 `to_yaml()`은 미선언으로 읽혀 돌아옵니다. 출력에 싣려면 AST에
  문서 수준 슬롯이 필요하고, 이번에는 하지 않기로 결정했습니다 — (세부 사항: quality-ledger (du))

- **CR로만 줄을 끊은 뒤의 노트가 두 번째 라운드에 주인을 바꾸지 않습니다** — 줄 끝 노트가 앞 노드의
  것인지 뒤 노드의 것인지 가르는 줄 판정이 둘 사이에서 `\n`만 훑었기 때문에, `\r`로만 줄을 끊는
  문서는 한 줄로 읽혀 줄 뒤의 노트가 모조리 앞 컨테이너로 거꾸로 묶였습니다.
  `!-\r\r\r- :\r\r\r\r- #-` 는 `\n` 표기와 같은 문서인데, fuzz 층이 이미 한 라운드에 마무리 지은
  그 표기와 달리 이쪽은 두 번의 방출이 필요했습니다: 처음에는 항목을 블록 형태로, 둘째에는 좁힌
  `- key: value` 형태로 적습니다. 이제 `line_break_between` 은 두 가지 개행 모두에 묻습니다 —
  granit이 줄을 나누는 데 쓰는 그 집합이며 주석 스캐너와 스칼라 트리밍과 공백 트리밍이 이미
  쓰던 것입니다. 이 족의 세 모양과 각각의 두 표기를 A/B 로 재면 모든 방출이 바이트 단위로 같고,
  움직인 것은 받아들일 때의 슬롯뿐입니다: 컨테이너의 줄 안 노트에서 앞선 노트로, `\n` 표기가 언제나
  주던 자리입니다. 실측: Rust 633 건, 20 000 사례에서 속성 26 건이 통과하고, 입력은
  `fuzz/seeds/yaml_roundtrip/cr-only-break-note-ownership.seed` 으로 보관되어
  `settles_in_one_round::a_note_after_a_cr_only_break_annotates_the_item_that_follows` 가
  고정합니다 — (세부 사항: quality-ledger (dt))

- **블록 본문 위 키의 노트가 두 번째 라운드에 옮겨 다니지 않습니다** — 기록자는 문서 자신의 노트를
  본문 마지막 줄에 얹고, granit은 노드를 그 노드 자신의 줄 개행까지 잡기 때문에, 다시 읽으면 그
  노트가 값의 것으로 보고되어 `write_pair`의 슬롯 귀속 답이 뒤집혔습니다. 첫 방출은 쌍 줄에 얹히고
  둘째는 키의 노트를 쌍 위로 올립니다. 이제 `write_pair`는 값이 쓸 수 없는 자리에서 비로소 줄에
  묻습니다: 아래에서 시작하는 스칼라나 빈 본문은 줄 끝 슬롯을 비워 둡니다. 실측으로 지목된 두 예외도
  그대로 두었습니다. 아래에 적힌 컨테이너는 `<<:  # note`와 흐름 본문을 빈 merge 키로 바꾸고, 태그만
  있는 줄은 아직 닫히지 않습니다. 실측: 두 크래시 모양 모두 한 라운드에 정점이고, Rust 632 건, 20
  000 사례에서 속성 26 건이 통과하며 fuzz 하네스도 둘 다 녹색 — 그래서 `fuzz/seeds/`에 넣었습니다 —
  (세부 사항: quality-ledger (ds))

- **플로우 안의 앵커 노드가 더 이상 다른 노드 텍스트에서 이름을 가져오지 않습니다.** — 소스에서
  `&name`을 되읽는 복원 과정은 행 전체를 왼쪽으로 훑었습니다. 그래서 쉼표와 따옴표를 담은 평문
  키(`$,":`)가 그 뒤의 모든 따옴표 상태를 어긋나게 했습니다: 다음 멤버의 따옴표 붙은 키 안에 있는
  `#`이 주석으로 읽히고, 그 멤버 자신의 앵커는 주석 텍스트로 건너뛰어지며, 키 *안쪽에* 적힌 `&`가
  이름이 되었습니다. 따라서 두 번째 덤프는 앵커 이름을 고쳐 쓰고 그를 가리키던 별명은 모두 외톨이가
  되었습니다. 이제 검색은 노드 자신의 창 — 빌더가 이 컨테이너에서 가장 마지막으로 소비한 노드의 끝 —
  으로 묶입니다. 실측: 더 좁은 텍스트·문맥 읽기 세 가지는 모두 잔차를 남겼고, 이 판은 20 000 사례
  속성 테스트 층을 통과했습니다. 단위 핀은 텍스트 전용 읽기가 `w`에 답하는 동안 창을 쓴 읽기는 `v`에
  답함을 보여줍니다. 이벤트 스트림 쪽도 같은 경계를 단조로운 하나의 수에서 가져옵니다. 가장 최근의
  비어 있지 않은 스팬이 끝난 지점입니다 — granit은 블록 컨테이너의 시작도 그 `End`도 빈 스팬으로
  보고하므로, 거기에는 실제로 끝난 텍스트가 대응하지 않고, 경계를 그으면 다음 노드 자신의 `&`이 잘려
  나갑니다 (세부 사항: quality-ledger (dr))

- **TOML 여러 줄 배열 안의 노트를 그 항목에 속하게 합니다.** — 배열 훑기는 `#` 줄을 건너뛰고 글을
  버렸습니다. 그래서 `arr = [⏎ # lead⏎ 1⏎]`, `arr = [⏎ 1 # trail⏎]`, 항목의 쉼표와 다음 항목 사이의
  노트가 모두 한 줄로 줄어 사라졌습니다. 항목은 이제 인라인 표 항목과 같은 두 개의 주석을 지니고,
  노트가 있는 배열은 줄을 갈라 쓰며 각 노트는 읽는 쪽이 알리는 자리에 놓입니다. 실제로 재어 두 번째
  출력은 바이트 단위로 같고, 노트 없는 배열은 그대로 한 줄입니다. 마지막 쉼표 뒤의 노트는 돌아갈
  항목이 없어, 여전히 잃음을 고정해 두었습니다. — (자세히: quality-ledger (dq))

- **인라인 표 항목의 키 줄에 있던 노트를 씁니다.** — `to_toml`은 항목의 노트를 두 칸만 읽었습니다.
  키의 자기 줄 목록과 값의 줄 안 노트입니다. 허브는 항목의 노트를 *키*의 줄 안 칸에 넘깁니다.
  거기서는 키 줄을 끝내는 노트가 키의 것이기 때문입니다. 그래서 어느 칸에도 맞지 않아 노트는 사라진
  채 나갔습니다. 첫 항목 앞에 노트 줄이 있는 인라인 표는 한 줄로 줄고 노트는 없어졌습니다. 대칭이
  맞지 않는 쪽이 단서가 됐습니다. 값 줄의 노트는 두 번을 가도 살아남았기 때문입니다. #428은 그
  손실을 적어 두고 닫았다고 하지 않은 채 고정했습니다. 여기서 그것을 닫습니다. TOML에는 키의 줄 안
  노트를 다시 알려 주는 칸이 없으므로 노트는 항목 위의 자기 줄을 차지합니다. TOML 읽는 쪽이 노트를
  알리는 자리입니다. 실제로 재어 보니 두 번째 출력은 바이트 단위로 같고, 손실을 고정하던 두 Python
  검사는 이제 살아남음을 고정합니다. — (자세히: quality-ledger (do))

- **루트의 인라인 표가 섹션 헤더로 다시 쓰이지 않습니다.** — `to_toml` 은 문서 루트의 비어 있지 않은
  매핑을 모두 섹션으로 분류하면서, 자체 리더가 방금 설정한 `flow_style` 비트를 보지 않았습니다.
  `p = { a = 1 } # t` 는 `[p]` 헤더와 본문으로 돌아왔습니다. 이는 표기 선택이 아니라 형태
  변경입니다. TOML 은 표의 일반 키를 하위 표 헤더 앞에 두므로, 같은 층의 키가 앞으로 밀려나고 주석이
  짝 줄에서 헤더 줄로 옮겼습니다. 중첩 및 배열 위치의 인라인 표는 이미 형태를 지키고 있었으니, 그
  비대칭이 신호였습니다. 이제 TOML 한 바퀴가 고정점이며, 앞서 승격을 고정하던 시험(둘은 이번 수정이
  더 이상 만들지 않는 헤더 텍스트를 단언했습니다)은 인라인 형태를 고정합니다. 중괄호 안에서 자기
  줄을 갖는 주석은 여전히 허브에서 사라지지만, 그 손실은 닫았다고 주장하지 않고 시험으로
  고정했습니다. — (details: quality-ledger (dj))

- **JSON5 쓰는 쪽이 자기 읽는 쪽이 거부하는 숫자 표기를 내지 않습니다.** — 맨 token 판정이 "숫자,
  그리고 점"만 물었기에 `00.`으로 적힌 평문 스칼라가 그대로 나갔습니다. JSON5에는 그런 정수부가
  없습니다. 혼자인 `0`이거나 0이 아닌 숫자로 시작하는 숫자열입니다. 그래서 이 라이브러리가 막 쓴
  글을 `from_json5`가 거부했고, `prop_json5_output_reparses`는 20 000 케이스에서 바로 그 모양을
  뽑았습니다. 방언이 담지 못하는 표기는 다른 비 JSON 표기처럼 스키마로 풀기 때문에 값은 남고 버리는
  것은 원문 형태뿐입니다. 적합한 `5.`는 그대로 오갑니다. — (자세히: quality-ledger (dm))

- **컨테이너 키가 자기 노트는 두 키를 가르지 못하는데, 키 정체성 모델은 그것을 모르고 있었습니다.**
  — `KeyIdentity`는 비어 있지 않은 컨테이너와 태그가 붙은 스칼라를 모두 `Other`로 나누어 노드로
  비교했습니다. 그 근거는 노드 지도가 둘을 갈라 놓는다는 것이었지만, 가르는 것은 지도이지 *텍스트*가
  아닙니다. 속성 계층은 아무도 이름 붙이지 못한 짝을 뽑았습니다. `? - null`을 두 번, 한쪽에 노트를
  달아 쓰면 같은 명시 키 블록이 두 개 나가고, 다시 읽으면 `duplicate key: Sequence { .. }`가 되어
  `prop_idempotent`가 실패했습니다. 짝마다 재어 보니, 키 노드에 오른 노트는 아무것도 가르지 못하고
  표기(`[a]` 대 `- a`)와 내용만 가릅니다 — 읽는 쪽이 비교하는 방식 그대로입니다. 모델은 이제 그것을
  등급으로 말합니다 (`Tagged { text, tag, style }`, `Sequence { items, flow, tag }`,
  `Mapping { pairs, flow, tag }`, 동치와 해시는 직접 작성). 읽는 쪽이 무엇을 해석하고 부르는 쪽이
  무엇을 조립할 수 있는지를 두 모델로 두지 않으려고, 만드는 쪽도 같은 식으로 짝을 거부합니다. 고정
  표는 다섯 등급이 늘었고 각 쪽에 실측 결과가 붙었습니다.
  — (자세히: quality-ledger (dl))

- **일반 키 문자 안의 대괄호가 인용 영역을 열지 않습니다.** — #421 과 #425 계열의 세 번째
  형태입니다. 20 000 사례 추출이 `!!t /[": &Ilr |-8  # &m^fake` 를 만들었는데, 키 자기 문자열에 `[`
  가 들어 있었습니다. #425 는 흐름 지시자 뒤의 따옴표를 노드 시작으로도 인정했기에, 그 `"` 가 닫히지
  않는 영역을 열고, 진짜 `#` 는 영역 안으로 판정되고, 주석에 적힌 `&` 가 노드 이름으로 돌아왔습니다.
  앵커 이름이 바뀌고 그것을 가리던 별명은 모두 잃었습니다. 이제 지시자 스스로 토큰을 시작해야 하며,
  그것이 흐름 집합과 대괄호가 들어간 일반 키를 가릅니다. 다만 멤버 사이 쉼표는 앞이 무엇이든 다음
  노드를 시작합니다. `[a, " #", &x ~]` 의 `#` 은 따옴표에 싸여 있고 그 뒤의 앵커는 서 있어야 하기
  때문입니다. — (details: quality-ledger (dk))

- **어떤 노드도 열지 않는 따옴표는 주석을 숨기지 못한다.** — #421은 앵커 이름 읽기가 따옴 안의 `#`를
  무시하도록 가르쳤지만, 공백 뒤의 따옴이면 어떤 영역이든 연다고 믿었다. 그래서 평문 키 스스로의
  아포스트로피(`a '`)가 허상의 문자열을 열었고 그 닫음이 주석 안에 떨어져 진짜 주석 시작은 보이지
  않았으며, 주석에 적힌 `&`가 노드의 앵커 이름이 되었다: 값은 `&*~=9VjQq=8`로 돌아왔고, 그것은 별칭
  표기로 어떤 앵커도 품을 수 없다. 따옴이 있는 노드는 토큰의 시작이므로, 읽기는 이제 따옴 앞에 어떤
  토큰이 있는지를 묻는다 — 경계인지, 노드가 따르는 앵커·태그 속성인지.

- **중첩 컨테이너의 주석은 자기 줄 위에 올리고 본문 아래에 두지 않는다.** — 컨테이너의 인라인 주석을
  위한 기록자의 마지막 예비 자리는 그 컨테이너 들여쓰기에 놓인 빈 주석 줄이었다. 그러나 읽기는
  노드를 *자기* 줄의 개행까지만 드리우므로, 열 0보다 깊게 놓으면 주석은 그 범위 밖에 떨어져 뒤따르는
  줄에 귀속되고 두 번째 인쇄에서 사라졌다. 왕복 속성 실행은 본문이 따르는 `- - |-`까지 줄였고, 블록
  스칼라 없이 같은 소실이 났다 — 그래서 이 규칙은 들여쓰기로 말한다. 열 0은 이전 배치를 유지한다:
  그곳에서 주석 줄은 문서의 마지막 줄이고 읽기는 주석을 컨테이너로 돌려준다.

- **키의 주석은 자기 페어 줄에만 올라가고 중첩 헤더에는 오르지 않는다.** — #419는 값이 블록
  스칼라일 때 키의 주석을 헤더 줄에 싣도록 기록자에게 가르쳤지만, 참조한 위치는 「마지막으로 적힌
  블록 헤더」였고 그것은 문서 어디든 될 수 있었다. 값이 컨테이너면 그 뒤에 자기 페어들이 적히므로
  바깥 키의 주석은 안쪽 페어의 헤더 줄에 떨어졌다: `A: >` + `# a` + `-5`는 헤더를 갈라 두 주석을
  하나로 합치고 본문은 어느 블록도 선언하지 않게 둔다 — 내보낸 텍스트가 아예 해석되지 않는다.
  2만 사례 속성 실행이 찾아냈다(`misplaced bracket`). 이제 헤더 줄과 페어 줄이 같은 줄일 때만 이
  슬롯을 쓴다.

- **앵커는 주석처럼 보이기만 하는 `#`를 지나서도 자기 이름을 지킨다.** — 읽기는 파서가 가리킨
  노드에서 왼쪽으로 거슬러 올라가 앵커의 표시 이름을 되찾지만, 그 줄에서 주석을 여는 `#` 뒤에 있는
  `&`는 모두 건너뛴다. 이 가드가 따옴표 *안쪽*의 `#`까지 주석으로 여긴 근거는 같은 줄에서 앵커가
  인용된 값 뒤에 올 수 없다는 것이었다 — 블록 쌍에서는 참이지만 flow 컬렉션에서는 거짓이다. 그래서
  `{a: &x "x # y", b: &y 2}`는 `b`를 `&x`로 다시 읽었고, 두 노드가 같은 이름을 공유해 모든 `*y`
  별칭이 아무것도 가리키지 않게 됐다. 이제 스캔은 따옴표를 따라간다 — 작은따옴표 안의 `''`, 큰따옴표
  안의 역슬래시, 그리고 스칼라가 시작될 수 있는 자리에서만 따옴표를 센다(granit 이름 문법은 앵커
  이름 자체에 `"`를 허용한다). 이 계열은 왕복 계층에서 나왔다: 뽑은 트리 4000그루에서 앵커 이름 변화
  11건, 축소기가 줄여 닿은 모양이 이 형태다.

- **압축형 `- key: value`가 첫 키의 자체 주석도 찍는다.** — `write_mapping_pair`는 단순 키의 선행
  주석을 페어 위에 쓰지만, 압축 목록 기록자는 그 규칙을 둘째 페어 이후에만 복사했다. 첫 페어는 대시
  줄을 공유해 자기 줄이 없기 때문이고, 그래서 `- null: " "`는 키의 `# 2`가 사라진 채 돌아왔다. 이제
  그 주석은 항목 줄 위에 쓴다. 발견 경로는 속성 계층에 남은 주석 손실을 「그래도 주석을 잃는 가장
  작은 트리」까지 축소한 것이다. 이 구멍을 막은 뒤 4000그루에서 실측한 손실은 한 그루다 — 블록 값의
  독립 주석이며, 결산부에 이름을 남기고 흡수하지 않는다.

- **블록 매핑 키의 줄 끝 주석은 값이 슬롯을 차지해도 사라지지 않는다.** — `key: value  # one`에는
  줄 끝 슬롯이 하나뿐이고, 키와 값이 모두 주장하면 기록자는 값을 택해 키의 주석을 지웠다:
  `? a # kn` + `: b # vv`는 `a: b  # vv`로 적혔다. 이제 그 주석은 flow 경로가 전부터 쓰던 대로
  줄 바꿈을 쓴다. 이 배치는 정하기 전에 실측했다 — 첫 키가 아닌 어떤 키의 바로 위에 쓴 주석 줄은
  **그 키**에 속한다. 즉 `a: 1` + `# K` + `b: 2  # V`는 `K`가 `b`에 그대로 남은 채 다시 읽혀 주석이
  주인을 지킨다. 첫 페어만 키의 주석을 바깥 컨테이너에 넘기고, 어느 쪽이든 텍스트는 남는다.
  압축형 `- a: 1`과 블록 스칼라 본문에도 같은 구멍이 있었고 이번에 막았다. 뽑은 트리 4000개에서
  출력 텍스트에 닿지 않는 주석은 907에서 257로, 그중 키 자리는 707에서 34로 줄었다.

- **블록 스칼라는 헤더가 줄을 옮기거나 주석을 받아도 값을 잃지 않는다.** — 들여쓰기 지정자는
  *부모 노드*에서 열을 센다. PyYAML·ruamel·이 엔진의 읽기도 모두 그렇다(`a: |2`, `- |2`, `? k`
  표기로 실측). 기록자는 헤더 줄에서 셌는데 — 값이 선행 주석에 밀려 자기 줄로 떨어질 때까지는
  같은 열 — 그래서 본문이 헤더의 숫자보다 두 열 깊게 놓이면 어떤 읽기도 그 두 열을 내용으로
  본다: `" "` 값이 `"   "`로 돌아왔다. 둘째 형태도 같은 부류다. 단순 키의 줄 끝 주석은 `:` 앞에
  표기가 없으므로 기록자가 쌍 줄 끝에 싣지만, 블록 값의 끝은 *본문* 마지막 줄이고 거기서 `#`은
  내용이다 — 주석이 달린 키에 `|-`를 붙이면 `abc  # n`이 적혀 트리가 갖지 않은 3바이트가
  늘어난다. 둘 다 속성 계층의 불일치를 노드 위치별로 귀속해 찾았다: 뽑은 트리 4000개 중 434개가
  이 이동을 안고 있었고 두 수정 뒤에는 한 개도 없다. 이제 주석은 헤더 줄에 실리고 읽기가 그것을
  값에 넘긴다.

- **블록 스칼라가 헤더가 뜻하는 줄 끝 개행을 지어내지 않는다.** — `key: |-\n  line1\n  line2\n`은
  `line1\nline2`로 읽힌다. 그 노드에 clip을 요구하면(`set_chomping("clip")`, 혹은 줄 끝 개행이 없는
  값에 `Clip`을 붙인 트리를 쓰기에 넘긴 경우) `|`가 적혔는데, `|`는 줄 끝 개행 하나를 *뜻*하므로
  문서는 트리를 한 바이트 더 길게 되돌려준다. `effective_chomping`은 이제 어떤 헤더도 적을 수 없는
  주장만 고친다 — 줄 끝 개행이 없는 `Clip`과 `Keep`은 `Strip`이 된다. `Strip` 주장은 그대로 둔다 —
  `-`는 적을 수 있고 그것을 고르는 일 자체가 호출자가 의도한 편집이기 때문이며,
  `test_set_chomping_strip`이 반대 방향을 고정한다. 발견 경로는 속성 계층의 불일치 분류표다. 뽑은
  트리 4000개 중 255개가 이 부류였고 형태는 모두 같았다 — 길이 N인 값이 N+1로 읽혔다. 고정된 블록
  검증값은 예외 없이 `\n`으로 끝나서, 바이트 정확 단언이 다섯 세대를 지나도 이 규칙을 닿지 못했다.
  그 다섯 군데는 이제 `|-`/`|-4`를 담는다. 본문이 실제로 지닌 표기다.

- **동봉되는 타입 계약이 패키지가 실제로 건네는 클래스를 쓴다.** —
  `python/pyrs_yaml/pyrs_yaml.pyi`는 확장 모듈에서 생성되는데 `python/pyrs_yaml/__init__.py`는
  불러오기가 끝난 뒤에
  `node`·`find`·`walk`·`scalars`·`merged`·`set`·`set_many`·`insert`·`append`·`delete`·
  `rename`·`sort_keys`를 `YamlDocument`에 붙인다. 그래서 이 열두 구성원은 실행 시간에는 있으나
  검사기가 읽는 산출물에는 하나도 없었다. 설치된 패키지에 mypy 2.4.0을 세워 재면 `document.node`은
  stub이 한 번도 선언하지 않은 구성원을 가리켰다. 첫 도미노는 그 한 층 위에 있었다 — 공개 감싸기
  `parse`와 `parse_file`은 `-> Any`, `parse_all_docs`는 `list[Any]`으로 적혀 있어 binding 스스로
  밝힌 `YamlDocument`를 지워 버렸고, 어느 검사기도 사용자가 먼저 부르는 입구에서 그 클래스에 닿을 수
  없었다. 증세가 나온 자리마다에서 고쳤다: 감싸기에는 정직한 반환 타입, stub에는 선언된 조정 규칙 —
  컴파일된 모듈의 클래스에서 인자와 기본값과 docstring을 그대로 읽어 쓰고
  `EXPECTED_ATTACHED_MEMBERS`로 개수를 센다. 따라서 구성원이 늘거나 줄면 검토 사건이 되고 계약이
  말없이 좁아지지는 않는다. 패키지에 타입이 적히지 않은 구성원은 `Any`로 채워지는 대신 유도를
  멈춘다. 타입 검사 관문도 stub이 불러오는 같은 패키지의 모듈을 임시 패키지에 담게 됐다 — `.pyi`만
  두던 임시 패키지는 모든 wheel이 포함하는 파일을 `import-not-found`로 보고했다. `Node`와
  `MergedView`는 애초에 이 구멍에 들지 않았다 — `py.typed` 패키지의 Python 모듈이라 검사기가 소스를
  읽기 때문이다. 미해결 항목 표는 이 대목에서 구멍을 지나치게 크게 적었다. 이제 검사기와 객체가 같은
  판단을 한다: `document.node("a")`은 검사 때 `Too many arguments`, 실행 때는 언제나처럼
  `TypeError`. 열두 선언을 지우면 `test_the_stub_declares_every_public_member_of_a_class_it_names`가
  열두 개를 이름 찍어 빨개진다.

- **flow 집합 구성원의 주석은 떨구지 않고 쓴다.** — `{A: x  # V}`를 읽은 트리는 `V`를 지켰지만 다시
  내보내면 `{A: x}`였다. 독자는 flow 구성원마다 자리 하나, 그 줄 끝 주석만 넘겨주는데 flow 쪽 쓰기
  계층은 그 자리를 하나도 쓰지 않았다. 곧 주석은 메모리에만 존재했고 텍스트를 읽은 다음 사람은 두 번
  다시 보지 못했다. 뽑은 트리 4020개로 재니 1650개에서 첫 차이가 주석에 떨어졌다. 이제 주석은 자신을
  연 괄호보다 한 칸 깊은 곳에서 이어진다(`- {A: x  # V` 다음 `}`). `#`이 줄을 닫으므로 `- {`나
  `k: {` 아래 0열의 값은 flow 이어쓰기 들여쓰기 오류로 읽히기 때문이고, 여덟 표기는 모두 읽힘과 자기
  고정점을 실측했다. 구성원이 주석을 두 개 가지면 두 번째 자리가 없으니 그 컨테이너는 block으로
  옮겨가 각 주석이 자기 줄을 지킨다(유지보수자 결정, 2026-10-14). 같은 곳이 둘째 질문에도 답했다.
  flow mapping의 key로 mapping이나 sequence를 쓰면 `null`로 돌아왔는데, PR #410이 막은 alias key
  소실의 형제이고, 이제 `?` 표시를 남긴다(`{? {}: v}`). ruamel은 이것을 빈 mapping의 key로 읽는다.
  주석 본문은 독자가 되돌리는 그대로 쓴다(양쪽 공백을 버리고 할 말이 남지 않은 주석은 어디에도
  쓰지 않는다). 그래서 첫 내보내기가 규범형이고 제2 왕복을 기다리지 않는다.

- **mapping 키로 쓰인 alias를 `null`로 떨구지 않고 다시 쓴다.** — `a: &x [1, 2]`에 `? *x` 키를 둔
  문서를 내보내기 쪽은 `null: v`로 다시 짜고 있었다. `write_mapping_pair`의 complex key 판정이
  mapping과 sequence 노드만 봤기 때문에 alias는 plain 키 경로로 떨어져 아무것도 해결하지 못했다 —
  만든 트리가 한 항목 더 짧았 는, 다른 mapping이다. 축약형 `*x: v`는 쓸 수 있는 표기가 아니었다.
  미해결 표가 바로 그 점에서 사실을 잘못 적었다: 이 엔진의 reader는 이를
  거절한다(`simple key expected ':'`). 그래서 내보내기는 `?` 표시를 싣는다 — 블록 스타일에서는 표시
  줄에(`? *x`), flow mapping에서는 중괄호 안에(`{? *x : v}`) — 그리고 lock이 해석하는
  ruamel.yaml이라면 어떤 버전이든 똑같이 읽는다: Python 3.8 잡은 0.18.16, 나머지는 0.19.1이며
  축약형에 예외를 던지는 쪽은 뒤뿐이다. 스캐너에 대한 한 결같은 주장이 3.8 세 잡을 붉게 했고, 두
  버전을 다시 재서 바로잡았다. 왜 어느 층에서도 이 손실을 보지 못했나: AST 전략은 alias를 키로 둔
  mapping을 만들지 않고, round-trip 속성 검사는 맞지 않는 트리를 실패시키지 *않고 걸러내기* 때문에,
  모양이 바뀐 문서는 보고되지 않고 버려졌다. `pbt::tests::prop_alias_key_survives_the_round_trip`은
  그 모양을 만들고 조건 없이 검증한다. 내보내기 판정 을 되돌리면 이 속성과 블록식 단위 테스트만
  떨어진다 — 344개 중 2개, 나머지는 움직이지 않는다 — Python 층은 출력을 바이트 단위로 고정하고,
  ruamel이 그것을 읽는 일도 고정한다.

- **해시 불가 키 거절이 같은 키를 또 거절하는 두 길을 가리키지 않게 됐다.** — 복잡한 키 투사와 함께
  들어온 메시지는 문서를 `to_json()`이나 `to_toml()`으로 투사하라고 독자에게 일렀다. 이 트리에서
  실측하면 복잡한 키는 두 곳에서도 예외가 난다: JSON 작성자는 `json-object-key`, TOML은
  `toml-keys-must-be-scalars`로 답한다. 따라서 그 조언은 출구가 아니라 두 개의 오류로 독자를
  데려갔고, 함수 자신의 주석 — 모양을 이름으로 짚고 멈춘다고 적혀 있다 — 과도 맞지 않았다. 이제
  지문은 통하는 유일한 길, 즉 `parse()`로 읽고 키를 AST에 두는 일을 밝히 적고 다른 투사도 이 키를
  거절한다고 분명히 알린다. `tests/test_parse.py`가 양쪽을 고정하므로 추천을 되돌리면 실패한다. 어
  떤 입력이 예외를 던지는지는 바뀌지 않았고, 잘못된 말은 릴리스에 한 번도 닿지 않았다.

- **복잡한 mapping 키가 노드의 Rust 디버그 텍스트로 Python에 건네지지 않는다.** —
  `safe_load("? ?")`가 돌려주던 dict의 키는 `'Mapping { pairs: {Scalar { value: "~"…'`였고,
  `? [1, 2]`도 같은 모양의 문자열이었다. 변환이 짝을 버리기보다 디버그 표기를 골랐기 때문이다. 둘 다
  옳지 않다. 버리면 문서가 쓴 항목을 잃고, 지어낸 키는 평범한 문자열이라 다시 쓰이므로, 손상이
  고정점처럼 보이며 다음 문서로 건네진다. 2026-10-14에 참조 라이브러리로 실측한 뒤 모양마다 갈랐다.
  순서 있는 키는 tuple이 된다 (`{(1, 2): 'v'}`, ruamel.yaml이 주는 것과 같다. PyYAML은 이를
  거부하므로, 데이터를 지키는 쪽으로 간 의도적 차이로 기록해 두었다). 안쪽 컨테이너는 바깥만 바꾸기
  때문에 Python 자신의 `unhashable type: 'list'`가 거부한다. mapping 키와 alias 키는 이 엔진이
  거부하면서 이름을 메시지에 적는다. 생성자가 해시 불가능한 값을 돌려주는 tag 달린 키를 포함해
  스칼라 키는 기존 텍스트 우회를 그대로 쓴다 — 그 경로는 망가진 적 없고 이번에도 바꾸지 않았다. 열한
  개가 검사하고 돌연변이로 확인했다. 이전 코도로 되돌리면 열한 개가 떨어지고 같은 두 파일의 나머지
  173개는 움직이지 않는다. `ROADMAP.md`는 이번에 발견하고 덮지 않은 나머지를 적었다. alias가 키
  자리에 있으면 여전히 `null: v`로 쓰인다. 이는 직렬화기의 구멍이지 투영의 문제가 아니다.

- **경로 편집은 만난 철자가 아니라 리더가 뜻하는 키를 찾습니다.** — 리더에게 `~`, `NULL`, `Null`
  노드는 같은 매핑 키이고 서로 다른 태그를 단 스칼라 둘은 다른 키입니다. 그런데 편집 계층은 키의
  *철자*만 비교했습니다. 그래서 `~: 2`를 담은 문서에서 `null`을 설정하면 그 항목을 놓치고 둘째 줄을
  덧붙였고, 돌려준 텍스트는 자기 리더가 `duplicate key: null`로 거부하는 것이었습니다(#336). 이제
  경로 조각이 리더가 해석한 결과로 해석되니 그 편집은 한 줄의 `~: 3`이 됩니다. 구분돼야 할 것은 계속
  구분됩니다. 따옴표 붙은 `'null'` 키는 여전히 별도 항목이고, `yes`는 null이 아닙니다.

- **CLI 테스트 장치가 거절로 stdin 이 닫힐 때 패닉하지 않는다.** — `run_with_stdin` 은 자식의 stdin
  에 쓰며 어떤 오류든 패닉했지만, 인자를 거절하는 CLI 는 읽기 전에 끝나므로 그 쓰기는 닫힌 파이프와
  경주했다. 같은 호출이 로컬에서는 통과하고 `msrv-check` 를 붉히며
  `write stdin: Os { code: 32, kind: BrokenPipe }` 를 냈다. 이제 장치는 핸들을 받아들이고 쓰기
  실패를 허용한 뒤 기다리기 전에 닫는다 — 실제로 읽는 자식에 필요한 것도 (열려 있는 파이프가 아니라)
  EOF 다. 뭉갠 것이 아니다: 종료 코드와 stderr 에 대한 단언이 여전히 합불을 정한다.
  `a_refusal_closes_stdin_before_the_harness_finishes_writing` 은 240 KB(파이프 버퍼 이상)를 흘려
  넣어 그 겹침을 결정적으로 만든다. 옛 장치에서는 매번 실패하고
  (`Os { code: 109, kind: BrokenPipe }`), 고친 뒤에는 48 개 CLI 테스트가 통과한다. — (details:
  quality-ledger (cs))

- **압축 JSONC/JSON5를 다시 읽을 때 같은 줄 댓글 묶음이 사라지지 않는다.** — 같은 줄 댓글이 둘 이상
  있는 노드는 마지막 하나만 남았다. 읽는 쪽이 같은 줄 댓글을 하나의 inline 칸에 넣기 때문에 뒤에 온
  것이 앞을 덮었다. 압축 writer는 노드의 주석을 정확히 그 형태로 쓰므로, 이 라이브러리가 스스로 쓴
  텍스트를 다시 읽으면 주석을 잃었고（`{/* note 0 *//* note 1 */`）두 번째 라운드 결과도 달랐다.
  이제 이런 묶음은 원 순서대로 몇 개든 담을 수 있는 leading 목록에 들어간다. 댓글이 하나라면
  이전처럼 inline 칸이 그대로 담당하므로 사람이 쓰는 표기는 변하지 않는다. 대가는 세부 사항 하나이고
  남긴다: 한 줄에 여럿이면 writer는 어느 것이 같은 줄이었는지 구별할 수 없으니 pretty 형식은 각각
  별도 줄로 인쇄한다. 아무것도 버리지 않고 라운드 사이에 이동하지도 않는다. — (details:
  quality-ledger (ch))

- **`to_jsonc`와 `to_json5`가 문서가 지닌 댓글을 더 이상 잃지 않는다.** — JSONC 댓글이 놓일 수 있는
  열한 곳 중 셋은 `to_yaml`은 출력하는데 두 방언 writer가 잃었다: 쌍의 값 노드 독립 댓글, 그 키의
  행끝 댓글, 마지막 항목 뒤 댓글. 매핑 루프는 네 개 칸 중 두 개만 읽었고 출력은 계속 올바른
  JSONC였으니, 손실이 소리 없이 일어난 이유다. pretty 경로는 빠진 두 곳을 쌍 위에 인쇄한다. compact
  경로는 `/* … */`를 쓴다 — flow 컨테이너 안에서 한 줄을 차지한 `//`는 앞서 이끄는 항목의 뒤로 다시
  읽히고, 둘이면 순서까지 뒤바뀐다. writer의 고정점이 허가하지 않는 움직임이다. 본문에 `*/`를 담은
  댓글은 자기 댓글을 미리 닫지 못하도록 compact 표기에서 중화된다. — (details: quality-ledger (cg))

- **64비트를 넘는 Python 정수를 덤프가 더 이상 실수로 바꾸지 않는다.** —
  `safe_dump({"k": 10 ** 30})`이 `k: 1000000000000000000000000000000.0`을 적었다. 프로토콜 변환이
  가능한 타입을 위해 놓은 마무리 경로가 `extract::<f64>()`를 부르는데, 그 변환은 모든 `PyLong`에서
  성공하므로 호출자가 준 수가 가장 가까운 배정밀도의 십진 표기로 바뀌었다. 키도 같은 모양으로
  망가져서 폭이 넓은 키 두 개가 다시 읽을 때 겹칠 수 있다. 두 쓰기 경로 모두 이제 `i64`를 넘는
  `int`에서 `str()`의 자리수를 그대로 내보낸다. 숫자가 아닌 텍스트를 돌려 주는 `int` 하위 타입은
  이전 경로를 지킨다. 읽기 쪽의 `i64` 상한은 그대로라서 `safe_load`는 그 자리수를 문자열로 되돌려
  준다 — 값은 남고 타입은 남지 않는다. — (details: quality-ledger (cf))

- **JSON5 가 자기 문법이 금지하는 앞자리 0 을 더 이상 받아들이지 않는다.** — `[012]`,
  `[-01]`, `[-012]` 이 JSON5 숫자로 해석됐다. 파서에 “JSON5 는 앞자리 0 을 허용한다”는
  명시적 완화가 들어 있었고, 그 주장을 의도된 동작으로 못 박은 검사까지 있었다. 그러나
  JSON5 의 `DecimalIntegerLiteral` 은 `0 | NonZeroDigit DigitalDigits?` 로 RFC 8259 와 같은
  생성식이며, `pyjson5` 는 셋 모두 거절한다. 완화는 삭제, 검사는 반대로 고쳤다. 그동안
  로컬에서만 재던 방언 규칙 세 가지를 단언으로 바꿨다: 정상 JSON 은 두 상위 방언이 모두
  받아야 하고, 참조 구현이 잘못된 문서를 거절하면 우리도 거절해야 하며, JSONC 는 댓글 표지가
  있는 곳에서만 엄격 JSON 에서 벗어날 수 있다. JSON5 의 코퍼스 완화는 30 에서 27 로 줄고
  포함성 위반은 계속 0 이다. — (details: quality-ledger (ce))

- **`to_toml` 이 TOML 로 성립하지 않는 텍스트를 내보냈다.** — 이스케이프 결함 두 건은 새로 넣은
  toml-test 인코더 방향 검증에서 드러났다. DEL(U+007F) 이 basic string 안에 그대로 나가
  `k = "x<DEL>y"` 가 되었고(`tomllib` 도 이 크레이트의 parser 도 모두 거부한다), 개행이 이어질 때
  첫 개행만 이스케이프하고 나머지는 실제 개행으로 한 줄 문자열에 적었다. 이제는 이어진 개행도
  전부 이스케이프하고 DEL 을 두 문자 표기에서 C0 부호와 같이 다룬다. 이 결함을 찾은 검사는 우리
  리더가 아니라 `tomllib` 로 출력을 판정한다. — (details: quality-ledger (ca))

- **빈 태그값이 문서에 없던 공백을 쓰지 않는다.** — `write_anchor_tag` 는 태그 뒤에
  구분 공백을 붙여 태그 token 을 내용보다 먼저 끝내지만, 빈 plain scalar 는 내용을
  가져오지 않으니 `n: !!null` 은 `n: !!null` 로 다시 실렸고, `n: !!null  # c` 는 주석
  앞에 공백을 셋이나 찍었다. 제거는 `write_scalar` 에서 한다 - 빈 값·스타일·태그가 모두
  보이는 자리이고 `pending_tag_insert_at` 을 적는 바로 한 바이트 앞이라, 뒤 주석에
  닫히는 줄은 그대로 `!- ~` 로 찍혀 주석의 주인은 움직이지 않는다. 앞서 시도한 두 길은
  실측으로 기각됐다: 종단에서 pop 하면 대기 기구가 잡은 인덱스에서 panic 하고, 키의
  구분자를 없애면 주석이 주인 줄에서 움직였다. #339 를 닫는다. — (details: quality-ledger
  (bz))

- **표준 태그가 로드할 때만이 아니라 모든 포맷에서 타입을 결정한다.** — #335 으로 로더는 `!!str` /
  `!!int` / `!!float` / `!!bool` 을 존중하게 됐지만, JSON·JSONC·JSON5·TOML 로 가는 브리지는 스칼라의
  *텍스트*만 해석하고 `meta.tag` 를 버렸기에 같은 문서가 두 답을 냈다: `v: !!str 1.20` 은 문자열
  `"1.20"` 으로 읽히는데 `to_json` 은 `1.2` 를 내놓았다 — 타입이 바뀌었고 작성자가 일부러 남긴 뒤의
  0 도 사라졌다. 이제 규칙은 한 곳에 산다: `pyrs-schema` (`standard_tag_kind` / `TagKind::resolve`,
  YAML 1.1 어휘, `!!x` 축약형과 완전형 `!<tag:yaml.org,2002:x` 모두), JSON 기록자·TOML 기록자·CLI 의
  타입 비교가 모두 거기에 묻는다. 텍스트가 태그가 밝힌 타입을 만족하지 않으면 문자열로 바꿔치기하지
  않고 거부한다 (`*-tag-text-mismatch`). 로컬 태그(`!int`), 알 수 없는 `!!` 접미사, 미결정으로 남은
  `!!binary` 매핑은 그대로이며, 그대로임을 테스트로 고정했다. 대가는 측정 이미지에서 재고
  받아들였습니다. 태그를 먼저 묻는 쌍 반복자는 `to_json_medium`에서 +1.12%, `to_jsonc_medium`에서
  +1.03%, `to_json5_medium`에서 +0.82%를 쓰고, `to_toml_medium`은 -0.07% 움직였으며 나머지는 선을
  넘지 않습니다. `.ci/ir-baseline.json`은 이 pull request 안에서 다시 만들고 이 셋을 드리프트 관문에
  지목했습니다. 같은 측정에서 `from_json_medium`이 -0.46%, `from_jsonc_medium`이 -0.42% 조여졌는데,
  그것은 부탁한 적이 없습니다. — (details: quality-ledger (bu))

- **편집이 새 값의 타입을 옛 값의 태그로 덮어썼다.** — `doc.set("$.d", b"hi")` 는
  `d: aGk=` 를 출력했다. 이 대입은 `!!binary` 를 가진 스칼라를 만드는데, 스플리스 편집기의
  메타데이터 상속이 교체되는 노드에서 `comment`, `anchor` *및* `tag` 를 가져가므로, 호출자가
  지정한 타입이 “교체된 스칼라는 코멘트/앵커/태그를 유지한다”고 쓴 함수 스스로에 의해 지워졌다.
  이제 규칙은 필드별로 나뉜다: 주석과 앵커 이름은 문서의 것(같은 문서의 별명은 계속 해석될 수
  있어야 한다), 타입은 값 자신의 것. 태그를 들여오지 않은 값은 저자의 표기를 유지한다 —
  `port: !!int 8080` 에 `9090` 을 설정해도 `!!int` 로 남는다 — 매핑·시퀀스·널도 같은 결함을
  공유했으므로 동일하게 상속한다. — (details: quality-ledger (bt))

- **null 표기 두 개는 하나의 키다. 생성기는 둘 다 기록자에게 넘기고 있었다.** — 20,000 케이스
  속성 계층이 `!E null: A  # 0` 과 `!E null: a` 에 대해 `DuplicateKey("null")` 을 보고했다.
  `Null` 노드는 자체 텍스트를 갖지 않고 기록자는 `null` 단어를 출력하므로, 두 줄은 다시 읽을 때
  같은 키가 된다. `keys_collide_in_text` — 생성된 매핑이 표현 가능하게 남기는 규칙 — 는 스칼라
  텍스트와 빈 컨테이너만 봤고 null 은 보지 않았다. 이제는 독자가 null 으로 해석하는 모든 표기를
  다루며, 두 키를 실제로 갈라 놓는 것(태그, 그리고 태그가 있을 때만 유효한 앵커)과 아무것도
  갈라 놓지 못하는 것(행에는 실리지만 키에는 실리지 않는 주석)을 구별한다. 규칙·기록자·독자는
  양방향으로 고정되었다. 규칙이 다르다고 부른 짝은 엄격 재독에서 두 항목을 유지하고, 같다고 부른
  짝은 거부거나 접혀야 한다. — (details: quality-ledger (br))

- **인라인 dict 스키마가 직렬화기가 몰랐던 부분을 잃지 않는다.** — `safe_load(schema={...})` 와
  `YAML(schema={...})` 는 `_schema_to_yaml` 로 Python dict 를 엔진이 읽는 스키마 텍스트로 바꾼다. 그
  함수는 행별로 텍스트를 조립했고 `extends` 와 `rules` 만 알고 있었으므로, `validate` 절을 담은 dict
  는 그 규칙이 빠진 채 조용히 등록되었다. 규칙 없는 스키마는 모든 문서를 통과시키니 아무것도
  보고하지 않았다. 게다가 유일한 손글씨 인용 규칙（값에 어포스트로피가 있으면 큰따옴표）은 두
  인용부를 모두 담은 패턴에서 안쪽 `"` 를 이스케이프하지 않아
  `invalid trailing content after double-quoted scalar` 를 냈다. 이제 dict 전체를 `from_dict` — 다른
  곳에서 왕복 충실도를 보장하는 그 직렬화기 — 에 넘기므로, 모든 키가 남고 임의의 패턴 텍스트도
  올바르게 인용된다. — (details: quality-ledger (bp))

- **두 공개 함수가 자기 스터이가 받았다고 적은 키워드를 받지 못했다.** —
  `pyrs_yaml.read_markdown(path="note.md")` 는 `TypeError` 를 던졌다. 커밋된 스터이는 그 파라미터를
  `path` 로 선언했는데, `__init__.py` 의 Python 감싸기 함수가 `content` 로 고쳐 부르기 때문이었다.
  게다가 함수는 받은 값의 파일을 열므로, 이름이 이끄는 `read_markdown(content=text)` 쪽이 파일
  시스템에서 실패했다. 이제 감싸기 함수는 `path` 를 쓰고 `read_markdown_str` 은 `content` 를
  유지하며, 두 함수는 그 차이를 적는다. 같은 드리프트가 반대 방향으로도 있었다:
  `validate_against_schema` 는 스터이에 `schema_yaml`, 호출자가 줄곧 쓴 것은 `schema` 였다. 네이티브
  파라미터를 고치고 스터이를 선언된 경로로 다시 생성했다. — (details: quality-ledger (bo))

- **세 언어의 API 페이지가 라이브러리에 없는 파라미터를 적고 있었다.** — zh/ja/ko 참고 페이지는
  `register_schema(name, schema: str | dict)` 와 `register_type(tag, type_handler, priority)` 를
  실었지만, 두 층의 선언은 `register_schema(name, schema_yaml: str)` 와
  `register_type(name, handler)` 이고 `priority=` 는 `TypeError` 를 낸다. 영어 페이지는 맞았으니
  오래된 필사가 아니라 번역 과정에서 생긴 어긋남이다. 페이지는 대상이 받아들이는 것을 적고, 게이트는
  어떤 언어에서의 재발도 거부한다. — (details: quality-ledger (bo))

- **위치를 담은 검증 오류도 일치한 `path` 를 말한다.** — `SchemaValidationError` 는
  `line:column: message` 또는 `path: message` 중 하나만 인쇄해서, 노드에 소스 범위가 붙는 순간
  스키마 저자가 적어 둔 쪽의 단서가 사라졌습니다. `1:9: expected map but got sequence` 는 어떤
  키인지 말하지 않습니다. 이제 두 쪽 모두 인쇄합니다. — (details: quality-ledger (bm))

- **맵핑 키는 값에 있을 때와 같은 의미를 갖는다.** — `1: a` 는 `{"1": "a"}` 로, `a: 1` 은 `{"a": 1}`
  로, `~: 1` 은 `{"~": 1}` 로, `a: ~` 는 `{"a": None}` 로 읽혔습니다. 같은 문서가 `:` 의 어느 쪽에
  스칼라가 있는지로 두 의미가 되어, 정수·boolean·null 을 키로 쓴 설정은 조회로 도달할 수 없었습니다.
  이제 키는 값과 같은 schema 경로로 해석되어 PyYAML, ruamel 과 일치하고, `load_toml` 은 어느 YAML
  schema 에서나 타입이 바뀌는 키를 따옴표로 감쌉니다. — (details: quality-ledger (as), (at))

- **앵커 이름 문법을 해석기와 맞춰 드리프트 계열을 닫았다.** — 출력 쪽이 해석기에 거부되는 앵커 이름
  （`found unknown anchor`）을 낼 수 있어「분석 불가능한 출력은 만들지 않는다」계약을 깨뜨렸습니다.
  `:` 로 끝나는 이름은 한 바퀴마다 한 글자씩 빠지고, 따옴표 붙은 이름은 줄바꿈을 삼켰으며, 맨
  스캐너는 주석 텍스트와 겹치는 `&` 에서 없는 앵커를 만들어냈습니다. 이제는 granit 처럼 앵커 문자의
  최대 연속 열로 읽습니다. — (details: boundaries — Fuzz findings, (x), (y), (z))

- **주석은 marker·태그·컨테이너를 넘어 주인을 지킨다.** — 첫 출력과 재해석 사이에 주석이 주인을
  바꾸거나 사라진 계열입니다：태그만 남는 값（`k: !`）이 바로 아래 줄을 자기 앞 주석으로 삼고,
  컨테이너 자신의 인라인 주석이 그것을 담을 수 없는 줄에 떨어지고, 따옴표 안의 `#` 를 맨 바이트
  스캔이 몰아내고, 압축된 `- key:` 분기가 텍스트만 옮겨 주석 슬롯을 외면하고, 앞 주석이 단일 *슬롯*
  이라 겹친 마지막만 남고 （`# alpha` + `# beta` + `key: 1` → `# beta`）. 앞 주석은 이제 목록이고
  모든 writer 가 따릅니다. — (details: quality-ledger (h), (i), (m), (n), (p), (t), (u), (v), (ad))

- **병합의 동일성과 깊이를 고치고 네이티브 스택 overflow 도 제거.** — `<<: &b` 는 값이 null 인
  리터럴 병합 키인데 아래 진짜 `<<:` 와 접혔고, 접는 길에 버려진 항목의 주석은 옮기지만 앵커는
  옮기지 않았습니다. 주석 달린 비태그 `y` 와 병합된 `y` 는 서로 다른 `IndexMap` 키라 `to_yaml` 이
  같은 수준에 `y:` 를 두 번 찍고, 중첩·자기참조 사슬은 cycle 가드를 pop 한 상태로 꼬리 재귀에 흘러
  깊이에 제한 없이 스택을 넘쳤습니다（58 바이트 libFuzzer 발견）. 주석 달린 `<<` 키 노드는 이 패스에
  보이지 않았습니다. — (details: quality-ledger (o), (q), (r), (s), (y))

- **블록 스칼라는 한 번 출력으로 수렴하고 indicator 도 옳게 판정.** — `>+8\r\r#` 는 값이 개행 하나·
  `Keep`·명시 들여쓰기 8 인 접힌 스칼라로 읽힙니다. 출력부는 indicator 를 남겨 `>+8\n\n` 을 내고,
  그것은 `Clip`·indicator 없음으로 읽히므로 *두 번째 바퀴* 는 `>\n\n` — Clip 은 뒤쪽 개행을 벗겨
  값이 `""` 입니다. 값이 비로 붕괴하는데 매 바퀴는「안정적」으로 보였습니다. 접힌 연속 개행, `4RWC`
  형 명시 들여쓰기, 앞줄이 빈 literal 도 같은 작업으로 다시 분석에 닫혔습니다. — (details:
  quality-ledger (aa), (ab), (r))

- **Unicode 공백, BOM, 태그 접미사가 문서를 망가뜨리지 않는다.** — 방언 fuzzer 가 도달한 공백 집합,
  BOM 위치, 태그 접미사 경계를 해석기 자체 문법에 맞췄고, 각각 커밋된 시드와 결정적 Rust 회귀
  시험으로 고정했습니다. — (details: boundaries — Fuzz findings, Blank set)

- **JSON/JSONC 주석 스캐너가 문자 중간에서 panic 하지 않는다.** — 줄 주석과 닫히지 않은 블록 주석
  스캔이 `pos` 를 한 바이트씩 옮겨, 뒤에서 오는 멀티바이트 문자（U+FEFF, fuzzing 약 25 초에
  도달）안에 정지점이 만들어지고 다음 `&text[pos..]` 가「not a char boundary」로 무너졌습니다.
  이제는 완전한 코드포인트 단위로 가고 잘못된 입력은 깨끗이 거부합니다. — (details: boundaries —
  Fuzz findings)

- **중복 키는 값으로 거부하고, 접는 것은 null 키뿐.** — 비교는 값 기준이므로 `{a: 1, a: 2}` 는 계속
  `YamlDuplicateKeyError`, 빈 키와 null 키（`: a` + `: b`, `~: a` + `~: b`）는 yaml-test-suite
  `2JQS` 요구대로 접습니다. 주석이 달린 키로 이름 접근이 안 되던 문제도 고쳤습니다：
  `CustomNode::hash` 는 정규화된 주석 view 를 접는데 `eq` 는 맨 슬롯을 비교해, `doc["key"]`, `in`,
  병합 전개가 문서에 분명히 있는 키에 「그런 키는 없다」고 답했습니다. — (details: quality-ledger
  (ae))

- **빈 컨테이너는 인라인으로 출력되어 두 번째 바퀴를 요구하지 않는다.** — `safe_dump({"a": {}})` 는
  `"a:\n  {}\n"` 을 만들고 AST 쪽 writer 는 `"a: {}\n"` 을 만듭니다. 순서 항목 아래의 `- \n  {}` 는
  flow 노드로 읽혀 한 번 더 움직입니다. 두 텍스트 모두 같은 데이터로 돌아와 왕복 시험에 보이지
  않았습니다 — fuzz 단계가 주장하는 불변식은 *텍스트* 자체가fixed point 라는 것입니다. — (details:
  quality-ledger (ab))

- **`pyrs-toml` 이 베어메탈에서 다시 빌드된다.** — 겹 주석 작업이 `#![no_std]` crate 에
  `std::mem::take` 를 넣었고 호스트 빌드는 용서해도 `no-std-check` 는 용서하지 않습니다. 여섯 곳을
  `core::mem::take` 로 바꿨습니다. 별도로, 여러 줄 인라인 표의 마지막이 아닌 항목의 주석이 구분 컴마
  뒤에 출력됐는데 — TOML 은 주석 안에 컴마를 둘 수 없어 — 해석기가 그것을 다음 키의 앞 주석으로
  옮기므로 텍스트가 끝내 수렴하지 않았습니다. — (details: boundaries — Known Engine Boundaries)

- **Linux free-threaded wheel 이 배포되고 `pyq` 도 산출물을 빌드한다.** — wheel 행렬은 Windows 와
  macOS 만 free-threaded 를 만들어 GIL 없는 실행계의 Linux 사용자에게 설치물이 없었습니다：GIL 을 켠
  `cp38-abi3` 는 `Py_GIL_DISABLED` 와 ABI 가 맞지 않고 `abi3t` 는 3.15 부터입니다. 교차 아키텍처
  `pyq` 단계는 qemu binfmt 를 등록해 manylinux 이미지 안에서 자체 검증합니다 — 호스트에는 번역기가
  있고 외부 loader 가 없어 aarch64/armv7 을 바로 실행하면 `main` 이전에 죽습니다. — (details:
  quality-ledger (aq))

- **게이트 장치 자체의 여섯 결함을 고쳤다(MSRV leg 포함).** — 세 체커가 Python 3.8 에서 import 불가
  （import 시점에 평가되는 주석, `str.removeprefix`）, 시나리오 탐색이 harness 파일 하나만 지목해 두
  번째 경로가 게이트를 벗어남, Ir 의 crate 집합이 `[dependencies]` 에서 멈춰 절반만 읽은 그림을
  설명, 기준 갱신이 rustc 1.99.0 으로 1.97.1 기록에 맞춰 `serialize_medium` 을 +6.7% 이동, 집합
  검사가 열한 잡 중 셋만 대기, 속성 단계는 `PROPTEST_CASES` 를 아무 workflow 도 설정하지 않아
  proptest 기본 256 cases 로 돌아갔음. — (details: quality-ledger (ah), (an), (ao), (ap), (aq),
  (ar))

- **아무도 보지 않는 자리에 항목을 넣을 수 없다.** — 어떤 커밋은 다섯 미러 모두에 노트를 더하면서도
  `CHANGELOG.md` 에는 서문 위에, en 과 zh 에는 `tags:` 목록 안에, ja 와 ko 에는 front matter 와 첫
  제목 사이에 넣었습니다. 모든 파일에서 본문 밖이었지만 미러 검사는 통과였었는데, '버전 헤더 집합이
  같다' 는 독자가 어디서 보는지를 전혀 말하지 않기 때문입니다. `placement_errors()` 가 위치를
  검사합니다. — (details: quality-ledger (ar))

- **변경 로그 페이지의 설명과 접힌 릴리스 표시를 복원합니다.** — 폭 수정기가 페이지 YAML front
  matter 의 시작 `---` 만 건너뛰어 네 언어 페이지의 `title:`, `description:`, `tags:` 가 한 줄로
  다시 묶였고, 배포된 생성기는 페이지 설명을 사이트 설명으로 바꿔 그대로 공개하며 새 판은 빌드를
  실패시킵니다. 접힌 릴리스는 맨 `<details>` 로 열리는데 Python-Markdown 은 그 내용을 해석하지
  않습니다 — 배포 판 실측으로 제목 11 개와 그대로 새는 `####` 3 개, 속성 적용 때는 제목 69 개와 새는
  것 0 개. `scripts/check_doc_metadata.py` 가 둘 다 검사합니다. — (details: quality-ledger (az))

- **릴리스에 포함된 형식 stub 이 다시 해석됩니다.** — maturin 1.14.1 은 런타임 `__doc__` 을 삼중
  따옴표에 그대로 옮겨 쓰고 이 프로젝트의 Rust doc comment 두 곳에 역슬래시가 있어
  `python/pyrs_yaml/pyrs_yaml.pyi` 가 잘린 unicode escape 로 `ast.parse` 에 실패했습니다. `py.typed`
  와 함께 사용자의 mypy/pyright 가 읽는 타입 계약이 이 파일이고, 어떤 문서 생성기도 여기서 API 에
  도달할 수 없었습니다. drift 경로는 docstring 안 역슬래시를 escape 하고（지점 수를 잠금장치로）파생
  텍스트의 해석을 요구합니다. — (details: quality-ledger (bb))

- **릴리스 노트가 자기 릴리스 전체를 설명하고 첫 줄만은 아니다.** — 접힌 이력의 모든 `<summary>` 가
  해당 릴리스의 첫 항목이었으므로 독자는 펼칠 가치가 있는지 판단할 수 없었습니다（v0.16.0 은 93 개
  중 1 개: Added 64, Fixed 14, Performance 8）. 이제 각 summary 는 릴리스 전체를 요약하고, 접히지
  않았던 0.11.4·0.11.3·0.1.0 도 나머지 와 같은 형태로 접었으며, 다섯 미러에 같이 적용했습니다. —
  (details: quality-ledger (bc))

- **문서만 바꾼 PR은 병합할 수 없었고, 사이트를 빌드하는 PR도 없었습니다.** — 브랜치 보호가 요구하는
  검사는 `Test matrix (all legs)` 하나이고, 그것을 내는 workflow가 `*.md`와 `docs/**`를 트리거에서
  제외했습니다. PR #319은 만들어진 검사 전부 통과였는데도 `Required status check … is expected`로
  거부됐고, 관리자 강제가 켜져 있어 `--admin`도 통하지 않았습니다. 이제 `ci.yml`은 모든 PR에서
  돌아가고 `changes` 작업이 변경셋을 분류해 무거운 legs가 그 답으로 조건화되며, `docs-gates`는 매번
  문서 검사와 네 로케일 `--strict` 빌드를 돌립니다 — 등록해 둔 렌더링 사각지대가 이렇게 닫혔습니다.
  — (details: quality-ledger (bd))

- **모든 wheel에 동봉되는 형식 계약이 예외 클래스를 하나도 선언하지 않았습니다.** — 패키지는 열 개의
  오류 형식을 export 하는데 `python/pyrs_yaml/pyrs_yaml.pyi` 에는 하나도 없습니다 — maturin 1.14.1
  의 introspection 경로가 PyO3 `import_exception!` 클래스를 살피지 않기 때문입니다. 그래서
  `except pyrs_yaml.YamlParseError:` 는 mypy 와 pyright 에게 보이지 않았습니다. drift 경로는 빌드된
  클래스에서 선언 (이름·기반·docstring) 을 읽어 붙이고, `EXPECTED_EXCEPTION_CLASSES` 로 수를 잠그고,
  파일이 해석될 수 있도록 기반 클래스를 subclass 앞에 배치하고, import 가 안 되면 계약을 덜어낸 채로
  통과하는 대신 exit 2 로 멈춥니다. 이로써 `exceptions.md` 도 네 로케일 모두 자동 생성이 되었습니다.
  — (details: quality-ledger (bf))

- **스텁은 해석되지만 형식 검사는 통과하지 못했습니다.** — mypy 가 `python/pyrs_yaml/pyrs_yaml.pyi`
  **안쪽**에서 다섯 개 오류를 냅니다: `u32` 미정의, `Callable` 미정의, `Py<PyAny>` 관련 잘못된 주석
  세 곳. 사용자의 편집기에서는 이 진단이 이 라이브러리에 귀속되고, AST 단계 검사는 모두
  초록이었습니다. maturin 1.14.1 이 Rust 표기를 주석에 옮기고 `Callable` 을 가져오지 않으므로,
  경로가 표기를 다시 쓰고(지점 수 4로 잠금) 주석이 참조하는 이름을 `typing` 가져오기에 합치며,
  `scripts/check_stub_types.py` 가 CI 에서 mypy 와 `ty` 로 산출물을 확인합니다. `ty` 는 우리 보정
  템플릿이 만든 `-> dict` 도 지적했습니다 — 이제 `dict[Any, Any]`. 남은 32개 엄격성 진단은 기록만
  하고 강제하지 않습니다. 이 게이트는 이후 CI 의 모든 테스트 레그를 빨갛게 만들었고, 부족했던 것
  은 바로 호출할 도구입니다. 실행 파일이 없으면 반환 코드가 아니라 예외가 뜨기 때문에, 확인 불가라
  는 답은 이제 exit 2 와 skip 이 되고 절대 통과로 끝나지 않습니다. 게이트 자신의 반대 대조 검증도
  같은 날 무력화되었습니다. `origin/main` 을 단언했는데, 이 수정이 병합된 날 main 의 스텁은 이미
  정상이었기 때문입니다. 이제 대조 검증은 결함을 실은 커밋을 이름으로 지정하고, 폴백으로
  `v0.17.0` 을 삼아 파일에 있어야 할 표기가 존재하는지 확인한 뒤, 두 검사 도구가 설치된
  CI 에서 실행됩니다.
  — (details: quality-ledger (bg))

- **조판 수정 도구가 자신의 검사기가 보고하는 빈틈을 더 이상 쓰지 않습니다.** — 일본어 항목을 추가한
  뒤 `check_doc_wrapping.py --fix` 가 `1 gap(s) [wrap residue]` 로 답하고 수렴하지 않았습니다. 수정
  도구는 두 줄 사이의 이어 붙임을 문자 클래스로 결정하는데, 그 클래스에 U+3001 이 포함되지 않으므로
  `、` 로 끝나는 줄과 다음 가나 사이에 공백이 들어갑니다. 같은 파일의 잔여물 클래스는 해당 코드
  포인트를 포함해 그런 빈틈을 결함으로 보고합니다. 하나의 규칙을 두 가지로 적은 셈입니다. 이제
  `join_lines` 가 두 잔여물 패턴에 직접 묻고, `、` 경계는 붙입니다. — (details: quality-ledger (bi))

- **JSON5 비유한 부동소수가 허브를 오가며 두 방향 모두 형식을 지켰습니다.** — `from_json5` 는 허브
  문서를 돌려주는데, 그 텍스트의 양쪽 끝이 같은 세 단서를 다르게 취급했습니다. YAML Core 는 `.inf` /
  `-.inf` / `.nan` 을 부동소수로, 맨 단어 `Infinity` 는 문자열로 해석하므로 원본 표기를 그대로
  저장하면 숫자가 문자열이 되어 그 투영을 다시 읽는 쪽은 모두 `'Infinity'` 를 얻었습니다
  (`load_json5("{a: Infinity}")` 는 메모리에서 해석하므로 정상이었습니다). 먼저 측정해 보니 보고서는
  절반만 적혀 있었습니다. 따옴표 쪽은 반대 방향으로 틀려 있었습니다. `quoted_or_plain` 이 디코딩된
  JSON 문자열의 따옴표 필요 여부를 `needs_quotes`(core 스키마 질문) 에 물었기 때문에 `"Infinity"` 가
  평문 스칼라로 저장되어 `load_json5('["Infinity"]')` 는 `[inf]` 를 반환했지만 `load_jsonc`,
  `load_json`, `json.loads` 는 모두 `['Infinity']` 를 반환했습니다. 하나의 문서에 두 형식, 어느
  로더가 읽는가에 달려 있었습니다. 이제 발급기는 해석된 *값* 에서 방언의 맨 토큰을 유도하고, 허브는
  YAML 자체의 표기를 저장하며, JSON5 해석기는 파서가 넣어 주는 표기를 받아들입니다. 엄격 JSON 과
  JSONC 는 표기할 수 없는 비유한 부동소수에게 여전히 따옴표를 씌웁니다. 이 결정은 `ROADMAP.md` 에
  열린 채로 두고, 결함 고침 안에서 몰래 정하지는 않습니다. — (details: quality-ledger (bh))

- **ASCII 가 아닌 키를 담은 규칙 경로는 프로세스를 세웠고, `$` 이제 규칙 대상입니다.** —
  `rule_path_to_segments` 커서는 1바이트씩 전진하는데 문자는 통째로 밀어 넣었기 때문에, `$.café` 는
  `é` 안에서 멈추고 다음 슬라이스가 "start byte index 4 is not a char boundary" 로 패닉했습니다.
  올바른 스키마로 도달하는 `PanicException` 입니다 (`$.emoji😀key` 도 같음). 이제 디코딩한 문자의
  폭만큼 전진합니다. 고르고 나서야 뒷반도 보였습니다: 구분자를 요구한 뒤 빈 값을 검사했으므로
  맨 `$` (문서 자체) 은 규칙 대상이 될 수 없고, `$x` 는 해석 불능으로 남습니다. `path: $` 에
  `mapping_of` 를 쓰면 루트 문서의 값이 검사됩니다. 패닉을 찾은 조사는 `ROADMAP.md` 에 쟁점도
  남겼습니다: `mapping_of` / `sequence_of` 는 요소는 보되 컨테이너 종류는 보지 않습니다.
  — (details: quality-ledger (bj))

- **퍼징 계층에 스키마 언어 표적이 없었고, 결함은 바로 거기에 살았습니다(PR #329).** — 여섯 표적
  모두 엔진에게 문서만 먹였고 스키마를 먹인 표적은 하나도 없었습니다. 그래서 `path: $.café` 가
  프로세스를 죽이는 코드가 리뷰의 사각지대에 놓일 수 있었고(ledger (bj)), 손으로 쓴 스키마 테스트는
  모두 ASCII 키를 쓰므로 경로를 탐색하는 코드는 어느 계층에서도 도달할 수 없었습니다. 일곱째 표적은
  입력을 첫 NUL 에서 갈라 두 조각을 `parse_schema_yaml` 와 `validate_node` 에 넘기므로, 한 어휘
  항목이 두 문법과 그 상호작용을 함께 담습니다. 씨앗은 실제로 붕괴된 입력 그 자체이므로, 주간 표본만
  기다리지 않고 pull request 계층(씨앗의 결정적인 `-runs=0` 재생)에도 들어갑니다. 결함을 실제로
  집어내는지는 추정이 아니라 실증되었습니다: 수정을 되돌리면 재생에서 `schema_language.rs` 의 붕괴
  입력 1 건이 나오고, 수정이 있으면 같은 씨앗이 깨끗하게 재생됩니다 (150 초 탐색도 마찬가지).

- **`check_doc_wrapping.py --fix`는 이 규칙이 관리하지 않는 경로를 거부한다.** — Python 모듈에
  겨누면 그 파일을 Markdown 본문으로 취급해 재배열했다: docstring, 그 아래 문장,
  `def`·`if`·`raise`를 같은 줄로 붙여 모듈이 SyntaxError를 낼 때까지. 행렬 작업 중
  `scripts/ci_matrix_plan.py`에서 실제로 일어났고, 아직 커밋 전이라 통째로 다시 쓸 수 있었다.
  훅에서는 일어나지 않는다 — `prek.toml`의 `files:`가 스크립트보다 먼저 걸러서 — 안전은 호출 쪽에
  있었고 명령줄에는 없었다는 뜻이다. 이제 스크립트 안에 `GOVERNED`로 같은 패턴을 두고, `main`은
  관리하지 않는 경로를 exit 2로 거부하며 한 글자도 쓰지 않고 대상 외 인자를 모두 적는다. 검사 넷,
  그중에 대조가 하나: 이 방어막을 껐을 때 복구기가 그 파일을 실제로 고쳐 쓰는지 요구한다. 그것이
  없으면 통과하는 검사는 "이 입력이 우연히 살아남았다"만 보인다. — (자세히: quality-ledger (cm))

- **`newline` 옵션을 쓴 `Path.write_text` 일곱 곳이 Python 3.8과 3.9에서 `TypeError`를 냈다.** — 그
  옵션은 3.10에 들어왔고, 두 가장 오래된 해석기에서 여전히 묶음 전체를 돌리는 네 줄(`3.8` 세 OS,
  ubuntu의 `3.9`)이 바로 그에 답했다 — 검사 대상 동작에 닿기도 전에 fixture 준비에서 죽었다. 셋은
  이번 주에 썼고 넷은 오래전부터 `scripts/check_doc_wrapping.py`, `scripts/check_stub_drift.py`,
  `scripts/check_stub_types.py`, `tests/test_stub_types_gate.py`에 잠들어 있었는데, 하한 해석기에서
  그 경로를 아무도 실행하지 않았을 뿐이다. 스텁 스크립트 둘은 최신 Windows 잡에서 돌고, 그 검사
  하나는 3.8에서 건너뛴다(얕은 clone에 읽을 known-bad 태그가 없다). 이제 모두
  `open(..., newline="\n")`로 쓴다 — 3.5부터 이 옵션을 받고 산출 바이트는 완전히 같다. 표를 읽는
  대신 실제 해석기로 확인했다: CPython 3.8.20과 3.9.25에서 실측해 3.8은 238개 중 235, 3.9는 218개
  전부 통과. 남은 셋은 `pyrs_yaml`을 import 하는 스크립트로, 맨 환경에 휠이 없을 뿐이다. — (자세히:
  quality-ledger (cn))

#### 성능

- **이벤트 스트림이 스칼라 텍스트를 빌리고, 게이트가 이 한 수를 −9.192%로 매겼다.** —
  `StreamEventType::Scalar`은 granit 자신의 `Cow`에서 떠낸 `String`을 소유하고 있었다. 즉 문서의
  스칼라 마다 확보 한 번과 값 바이트의 `memcpy` 한 번이 일었다. 2026-10-14에 유지보수자가
  `Cow<'a, str>`으로 판 결했고, 그 판결에 맞수를 둘 시나리오는 먼저 추가돼 있었다(#405). 그래서 이
  자리의 숫자는 주장이 아니라 계산이다: `stream_medium`은 249,928,334 개의 계산 명령어를 읽었고
  커밋된 값은 275,227,226이며, 세 시료 가 명령어 단위까지 같다(산포 0.000%). 같은 실행에서 나머지는
  최대 +0.202%만 움직였다 — 이 저장소가 이미 재어 둔 호스트 사이 잔차이고, 읽기는 WSL에서, 확정한
  값은 강제 이미지에서 나왔기 때문이다 — 그리고 AST 경로는 이 변경에서 닿을 수 없다. 트리 없는
  읽기는 같은 텍스트에서 노드 모델을 짜는 일보다 이제 약 22% 싸 고, 그 차이는 한때 14.5%였다. 여전히
  떠내는 자리는 숨기지 않고 적었다: 게으른 `StreamIterator`와 증분 `YamlStream`은 빌림을 실을 수
  없는 경계에서 각자 소유로 옮긴다(`#[pyclass]`는 라이프타임이 없고, 청크 읽 기는 이벤트가 가리키는
  버퍼를 돌려쓰기 때문이다). 그래서 새로 돈을 내지 않는 곳은 `on_event` 경로뿐이다.
  `parser::stream::tests::a_plain_scalar_borrows_the_source_and_a_decoded_one_owns_its_text`가 그
  분기를 고 정한다 — 평문 스칼라의 바이트는 소스 범위 안에 있고, 이스케이프가 든 큰따옴표 값은 자기
  텍스트를 소유한 다 — 떠내기를 되돌리면 이름으로 실패하고, `stream_medium`의 바닥도 새 읽기로
  내려온다.

- **명령어 수 바닥이 그것을 만든 엔진까지 내려왔다.** — 최적화 PR 네 개가 새로고침 사이에 합류했는데
  커밋된 기준선은 따라오지 않았다. `Ir baseline refresh` 잡드가 강제
  이미지(`github-runner ubuntu24`, image 20261004.327.1, rustc 1.97.1 — 기준선이 이미 적었던
  툴체인이니 움직이는 것은 컴파일러가 아니라 숫자)에서 21개 시나리오를 다시 쟀고 그 산출물을 그대로
  커밋했다: `to_toml_medium` 53,218,877 → 28,763,688 (−45.95%), `serialize_anchors` −30.25%,
  `serialize_small` −29.83%, `serialize_block` −26.42%, `serialize_medium` −24.81% (32,648,850),
  `serialize_block_scalars` −21.99%, 읽기 쪽은 `from_json5_medium` −11.17%, `from_toml_medium`
  −7.29%, `from_json_medium` −5.82%, `from_jsonc_medium` −4.72%. 0.10% 이상 올라간 것은 없다 — 반대
  방향으로 움직인 일곱 개는 파서와 바인딩 경로이며 모두 이 파일이 스스로 선언한 0.5% 선 안에 있다.
  바닥이 엔진보다 46% 높은 채로 남으면 게이트가 46% 회귀를 조용히 통과시키는데, 그게 영 회귀 약속이
  막으려는 실패다. 같은 두 번의 측정으로 지역 수치가 어디까지 믿을 수 있는지도 잰다. 소스가 완전히
  같은데 WSL은 `serialize_medium`을 32,648,888, 이미지는 32,648,850(+0.0001%)로,
  `to_python_medium`은 386,665,602 대 385,871,983(+0.21%)로 읽었다. 따라서 `ir-baseline.yml`이
  인용하는 +1.45% 이미지 간 차이는 순수 Rust 쪽에서 닫혔고 나머지는 CPython을 링크한 쪽에 남아 있다.
  여기에 적힌 숫자 중 손으로 옮긴 것은 없다.

- **YAML 라이터가 노드마다 같은 비용을 두 가지 형태로 내고 있었다.** — 같은 프로파일의 두 줄이 문서
  하나를 좌우했다: `write_plain_scalar`와 `write_scalar_for_key`가 `serialize_medium`의 9.21%와
  9.29%를 쓰고 그 아래 패턴 탐색에 약 10%, `write_indent`가 8.10%였다. 앞의 것은
  `needs_double_quoted` 한 함수가 "이 토큰을 plain으로 낼 수 있나"를 같은 텍스트에 `contains` 네 번,
  `chars().any` 두 번, `starts_with` 열여섯 번으로 답한 것인데, 이제는 바이트 한 번의 훑기이고, 같은
  훑기가 0x80 이상 바이트를 봤을 때만 문자 쪽 판정(NBSP, BOM, C1 제어, YAML 비문자)을 다시 부른다 -
  UTF-8에서는 그것들이 다른 문자 안에 숨지 않기 때문이다. 그 논거는 `pyrs-schema`의 `needs_quotes`가
  이미 쓰고 있었고 라이터 쪽만 따라오지 못했다. 뒤의 것은 너비로-index하는 `Vec<String>` 캐시였다 -
  줄마다 길이 검사, 깊이가 늘어날 때마다 `resize`, 너비마다 `" ".repeat` 확보, 표 항목을 빌려 복사 -
  가 상수 64칸 공간 줄의 슬라이스로 바뀌었다. 고정한 것 은 속도가 아니라 동치다:
  `needs_double_quoted_agrees_with_the_scan_for_every_code_point`는 여섯 번 훑는 형태를 기준으로
  남겨 둘에 같은 질문을 던지고, U+10000 미만 모든 코드 포인트에 다섯 자리(혼자·맨 앞·맨 뒤·안쪽·공백
  곁)에서 확인하며, 평면을 걸어 별면 표본과 비문자도 본다. 이건 첫 판의 실수를 혼자 잡아냈다.
  끝부분이 `!saw_high_byte || …`여서 순수 ASCII 문자열에 전부 따옴표를 붙였고, 기준 구현이
  `"a b"`에서 다른 어떤 테스트보다 먼저 어긋남을 보여줬다. `b == b':'`를 바이트 판정에서 빼면 동치
  테스트와 `prop_output_always_parses`가 같이 빨개진다. rustc 1.97.1에서 같은 브랜치 세 커밋으로 세
  바이너리를 만들어 짝을 맞춰 재측정(두 번 반복이 명령어 단위까지 일치): 바이트 훑기가
  `serialize_anchors` -28.01%, `serialize_medium` -18.09%, `serialize_block_scalars` -13.05%,
  들여쓰기 표가 `serialize_medium` -8.20%, `serialize_block` -10.82%, `serialize_small` -29.81%,
  합쳐서 `serialize_medium` -24.81%. 대조군 `parse_medium`은 +0.00%(둘 다 호출하지 않는다).

- **TOML 라이터가 내보내는 모든 조각마다 임시 객체를 확보하는 일을 그만뒀다.** — `to_toml_medium`은
  53,218,850개 명령어를 셌는데 같은 문서의 `to_json_medium`은 21,283,040개였다. 출력하는 모든 줄이
  `writeln!`을 지나고, 모든 키와 값이 각각 `String`이 된 뒤 문서 버퍼에 복사되며, 배열과 인라인
  테이블은 `Vec<String>`을 만들어 join하고 다시 포맷하고 다시 복사했다. 같은 브랜치의 세 커밋에서 세
  바이너리를 만들어 단계별로 측정했으니 단계마다 자기 수치를 갖는다. 문서 기록부는 push로
  바뀌(−19.00%)고, `quote_key`/`quote_basic`/`quote_multiline`/`value_str`은 부른 쪽 버퍼에 덧붙이는
  `push_key`/`push_quoted`/`push_multiline`/`write_value`로 바뀌(−33.27%) 합쳐서 −45.95%,
  53,218,850이 약 28,761,000 계산 명령어로 줄었다. 통제군 `from_toml_medium` −0.15%,
  `serialize_medium` +0.01%, `to_json_medium` +0.00%는 이것들을 하나도 호출하지 않는다. 주장의 알은
  바이트 일치이지 단순히 "맞다"가 아니고, 그건 테스트로 말할 수 없다 — 테스트는 출력이 "무엇이어야
  하는지" 볼만 하지 "안 바뀌었음"을 보지 않는다. 그래서 두 트리 모두 저장소의 모든 TOML 문서를
  돌렸다(내용 해시로 중복을 덜어 712개 입력: `toml-test` 유효 사례 777개에 fuzz 코퍼스랑 시드,
  경로는 TOML → 허브 AST → TOML, 두 번째 통과에서 움직이는 텍스트를 잡기 한 바퀴 더). 결과는
  똑같았다: 텍스트를 낸 242개는 SHA-256이 전부 같고, 거부는 같은 오류 클래스, 고정점이 아닌 9개
  형태도 양쪽이 같고, 다른 레코드는 0개다. 이 차이를 뜻있게 만든 확인: "바뀌기 전" 트리의 출력을
  믿기 전에 그 트리에 새 헬퍼가 없음을 grep으로 0건으로 봤다 — 같은 트리끼리의 차이는 "아무것도 안
  바꿨을 때"와 똑같은 이유로 통과한다. `quote_key`는 사라졌고 `quote_basic`과 `quote_multiline`은
  테스트 모듈 안에서 `quoted`와 `multiline`으로 남았다 — 검증은 돌려받는 값을 원하고 배포하는 코드는
  원하지 않는다. `docs/dev/perf.md`에는 귀인과 산식, 그리고 라이터가 여전히 쓰는 부분이 남아 있다.

- **JSON 리더가 "할 일이 없는" 두 호출에 값을 치르지 않도록 했다.** — `from_json_medium`을
  귀인해보니 `Parser::ws`가 시나리오의 6.63%를 차지했다. 그런데 그 픽스처에는 공백이 한 바이트도
  없다. 파싱 한 번에 약 90번 호출, 한 번에 30개 남짓의 명령어, 그리고 답은 거의 언제나 "다음
  바이트는 분리 문자가 아니다"였다. `attach_pending`도 같은 모양이어서, 댓글이 없는 문서에서 3.23%를
  쓰며 모든 키와 모든 값이 지나갔다. 이제 `ws`는 단서 `at_separation` 하나를 인라인으로 검사하는
  가드가 되고, 반복문은 `ws_extended`로 그대로 옮겨갔으며, `attach_pending`은 빈 목록을 보는 즉시
  돌아온다. 가드는 반복문의 건전한 상위집합이며 방향은 효율이 있는 쪽이다: 반복문이 바이트를
  소비한다면 가드는 "예"라고 답한다. 이 방향을
  `the_whitespace_guard_never_declines_what_the_loop_would`가 고정한다 — ASCII 128바이트 전부,
  멀티바이트 분리 문자 다섯(과 분리 문자가 아닌 하나), 댓글 형태 세 가지, 방언 세 가지에 대해 가드와
  반복문에 같은 질문을 던지고 같은 위치와 같은 보류 댓글 수를 요구한다. 변형(뮤테이션) 검사 완료:
  가드가 `/`에 `false`라고 답하게 하면 JSONC의 `"//x\n1"`에서 `left: (0, 0) right: (4, 1)`로
  실패한다. 댓글을 몰래 떨어뜨리는 바로 그 누락이다. rustc 1.97.1에서 같은 브랜치의 세 커밋으로 세
  바이너리를 만들어 짝을 맞춰 측정했으니 가드마다 자기 수치를 갖는다: `from_json_medium` −4.33%
  그리고 −1.56%, `from_jsonc_medium` −3.52% 그리고 −1.25%, `from_json5_medium` −4.49% 그리고 −1.40%,
  대조군 `to_json_medium`은 +0.00%(두 개를 다 호출하지 않는다). `docs/dev/perf.md`에는 첫 줄이 호출
  횟수로 만든 예측을 넘은 이유와, 가드가 회수하지 못한 부분이 기록됐다.

- **두 리더가 한 글자씩 디코딩하는 대신 소스 덩어리를 통째로 복사한다.** — 한쪽따옴표 JSON5 스캐너와
  TOML 한 줄 기본 문자열 스캐너는, 판단이 필요한 다음 바이트까지 건너뛰어 한 번에 복사하는 큰따옴표
  JSON 스캐너와 달리 여전히 코드포인트마다 `chars().next()`와 `push(ch)`를 호출하고 있었다. 게이트가
  쓰는 측정 하네스로 보면 9 글자 TOML 값에 약 540개의 계산된 명령어가 들었다. rustc 1.97.1에서 짝을
  맞춰 재측정했다 (같은 트리, 두 바이너리를 교대로, 두 번 반복은 명령어 단위까지 일치):
  `from_toml_medium` −5.55%, `from_json5_medium` −5.76%. 통제군인 `from_json_medium` +0.00%와
  `to_toml_medium` −0.00%가 움직인 것은 이 둘뿐임을 보여준다. 멈춤 집합은 그대로이며, 그 두 쪽을
  각각 뮤테이션 검사로 고정했다: TOML은 기본 문자열 안의 제어 문자와 줄 구분자를 전처럼 거부하고,
  JSON5는 그 자리에서 제어 문자를 전처럼 *수락*하며, 둘 다 멀티바이트 문자 뒤에 닫히지 않은 입력은
  거부한다. `docs/dev/perf.md`에는 이를 찾아낸 귀인과, 이 변경이 일부러 건드리지 않은 여러 줄 문자열
  쪽 스캐너, 그리고 TOML 리더에 남는 구조적 비용(같은 문서로 169,449,833 대 JSON의 98,994,186 — 허브
  AST 앞에 중간 테이블 모델을 먼저 만들기 때문)을 다음 질문으로 적어 두었다. 끝난 질문으로는 적지
  않았다.

- **명령어 수 기준선을 실제로 측정된 엔진까지 내렸다.** — node-map 해셔가 `main`에 오른 뒤
  `Ir baseline refresh` 워크플로가 적용 이미지(`github-runner ubuntu24`, rustc 1.97.1)에서 21개
  시나리오를 다시 측정했고, 그 산출물이 커밋된 기준선이 됐다: `from_json_medium`
  120,751,446 → 99,000,356 (−18.01%), `from_toml_medium` −16.25%, `parse_anchors` −12.95%,
  `to_python_anchors` −11.17%, 쓰기 시나리오는 +0.03%…−1.80% 범위. 기준선이 실측보다 18% 높게 남으면
  게이트는 18% 성능 후퇴를 묵인하게 되므로, 후퇴 없음 약속은 이제 이 수치를 가리킨다. 수치를 손으로
  옮겨 적지 않았고, 러너에서 `scripts/ir_gate.py --update`가 파일을 썼다.

- **노드 맵의 키 해시를 모든 빌드에서 FxHash로 바꿨다(`no_std` 전용이 아니게 됐다).** — `std`를
  켜면 `NodeMap`은 `IndexMap<K, V, std::hash::RandomState>`였으므로, 맵에 키를 하나 넣을 때마다
  SipHash-1-3 비용을 치렀다. `callgrind`는 `CustomNode::hash`에서 `Hasher::write`로 이어지는 자체
  비용 10,214,000 명령·180,000 호출(`from_json_medium`의 8.98%)을 측정했고, 기능 플래그 분기는
  노드 크기까지 144바이트와 128바이트로 갈라 썼다. 이제 양쪽 모두 crate 고유의 고정 종자
  `FxHasher`를 명시한다 — 내장 빌드가 언제나 쓰던 것이다. 게이트가 고정하는 rustc 1.97.1에서 짝을
  지어 재측정했다: `from_json_medium` −12.75%, `parse_medium` −4.95%, `to_json_medium` +0.11%.
  `no_std` 쪽이 이미 감수했던 절충은 이제 전체 빌드의 것이며 `NodeMap` 문서에 명시했다. HashDoS
  무작위화가 사라지므로 충돌을 노리고 만든 문서는 노드 맵을 선형 탐색으로 떨어뜨린다. 도입하려면
  `RandomState`를 몰래 요구하던 `IndexMap<CustomNode, CustomNode>` 약 40곳을 `NodeMap`으로 고쳐
  적어야 했다. (자세히: docs/dev/perf.md "The node map was hashing its keys with SipHash")

- **스칼라 리졸버가 잘라낼 것이 없는 텍스트를 더 이상 정리하지 않습니다.** — 모든 JSON, TOML, YAML
  스칼라는 `resolve_core_type` 에게 "숫자인가 단어인가" 라고 묻고, 그 답이 먼저 하는 일은 술어
  양방향 순회로 앞뒤 구분 문자를 지우는 것이었습니다. 호출마다 38 개의 계산된 명령을 쓰지만, 앞이나
  뒤가 공백인 문자열은 거의 없습니다. YAML 1.1 스칼라는 `resolve_yaml11_type` 이 core 체인에
  위임하기 전에 한 번 더 정리하므로 그 비용을 두 번 냅니다. YAML 이 허용하는 공백은 모두
  ASCII(`SP TAB CR LF NUL`)이고, UTF-8 은 ASCII 바이트를 여러 바이트 순서에 넣지 않으므로, 첫
  바이트와 마지막 바이트만 보면 지울 것이 있는지 결정됩니다. 컴파일 시간에 `is_yaml_blank` 에서
  *파생한*(같은 규칙을 다시 적은 것이 아닙니다) 분류 표를 두 번 읽어서 순회 전에 답하고, 실제로 어느
  한쪽이 공백일 때만 기존 순회를 실행합니다. 같은 하네스 바이너리로 측정한 한 쌍에서
  `from_json_medium` −0.74%, `from_toml_medium` −0.92%, 그리고 기록 쪽도 같은 crate 를 통해 스칼라를
  해석하므로 `to_json_medium` −1.68%, `serialize_medium` −0.36% 이었고, 대조군인 `parse_medium` 는
  +0.0014% 로 재현되어 앞의 두 차이가 증거가 되었습니다. 가드에 `#[inline]` 을 붙인 결과는 −0.003%
  로 같은 바이너리의 반복 오차 안이었으므로, 측정으로 뒷받침되지 않는 지시로는 남기지 않았습니다. —
  (자세히: quality-ledger (ct))

- **명령어 수 마루를 `main` 의 코드로 옮겼다.** — 커밋된 기준선이 `d7177efc`/`dd211c8d` 이전 숫자
  그대로였으니 게이트의 여력은 결정이 아니라 드리프트로 넓어졌다. `from_json_medium` 은 지금
  측정보다 21.61% 위에, `from_jsonc_medium` 은 19.83%, `from_json5_medium` 은 16.72% 위에 있었고 그
  크기의 후퇴는 소리 없이 통과했다. `Ir baseline refresh` 경로로 강제 이미지에서 다시 생성했다 —
  도구 사슬은 파일에서 읽(`rustc 1.97.1`), 이미지는 `20261004.327.1`, 각 시나리오는 세 번 샘플의
  최댓값, 바이너리 지문도 숫자와 함께 기록. 그래서 손으로 옮긴 값이 없고 0.5% 선을 위로 넘은
  시나리오도 없다. 움직인 셋은 0.0001% 의 프로세스 소음이다. 내보내기 시나리오가 움직이지 않은 것은
  키 해시와 인용 판정을 바꾼 두 변경이 남겼어야 할 모양이다.

- **문자열에 인용표를 붙이려면 세 번 훑던 것을 그만뒀다.** — 모든 브리지가 `needs_quotes`에 "이
  텍스트를 plain 스칼라로 쓰면 잘못 읽힐까"라고 묻고, 그 답은 값을 문자로 읽은 뒤 NBSP와 BOM으로 두
  번 더 훑었다. ASCII 텍스트에는 둘 다 나올 수 없으니, 이제 바이트 한 번 훑기로 ASCII 경우를
  확정하고 `0x7F`보다 큰 바이트가 있는 텍스트만 변화 전의 문자 규칙에 넘긴다. 포괄적 대조
  `needs_quotes_agrees_with_the_char_rule_for_every_code_point`는 두 경로를 문자 규칙과 모든
  1,114,112 코드 지점에서 비교하고, 잘못된 바이트 규칙도 유죄로 만든다. 경계를 하나 옮기면(공백을
  제어 문자로 세면) `first: Some((32, true, false))`로 빨개진다. 게이트 하네스를 같은 호스트에서
  `callgrind`로 잰 결과 `from_json_medium` -6.89%, `from_json5_medium` -1.87%. 이 변경이 들어가지
  않는 `from_toml_medium`(-0.0009%)과 `parse_medium`(+0.0002%)은 두 번의 측정에서 재현됐다. 그래서
  앞의 둘은 주변 소음이 아니라 증거다. — (details: quality-ledger (cr))

- **노드 요약은 동일성이 실제로 읽는 필드만 읽고, 모든 읽기 경로가 값싸졌다.** — `CustomNode::eq`는
  스칼라의 값·스타일·chomping·세 동일성 필드를 비교한다. 명시적 블록 들여쓰기 지정자는 그 안에
  없지만 `Hash`는 그것까지 해시했다. 그래서 지도가 같은 키로 취급하는 두 노드가 서로 다른 버킷으로
  라우팅됐다. 이는 `plain_and_noted`가 다른 필드에 대해 이미 지키는 바로 그 계약이다. 이제 `Hash`는
  `block_indent`를 읽지 않고, 장식이 없는 노드는 메타데이터에 대해 `Option` 구분자 세 개를 쓰는 대신
  아무것도 쓰지 않는다. `callgrind`로 명령 게이트 자체 하네스에서, 같은 호스트와 같은 빌드 플래그로
  값을 매긴 결과 `Hasher::write` 자기 비용은 20.95M → 9.95M 명령, 프로세스 전체로는
  `from_json_medium` -7.80%, `from_json5_medium` -7.71%, `from_toml_medium` -5.66%,
  `parse_medium` -3.38%였다. 키 비교는 늘지 않았다(`__memcmp_avx2` 10,720,143 → 10,720,033). 값싼
  요약이 그 픽스처의 탐색 품질을 갉아먹지 않았다는 뜻이다. 재고한 경로는 `std` 쪽이다. 노드 지도가
  그쪽에서 `RandomState`를 키로 쓰는데, `no_std`는 이미 종자를 고정한 FxHash를 쓴다(`ast.rs:29`).
  재판정은 강제 이미지의 `.ci/ir-baseline.json`이 맡고, 이 시나리오들은 내려갈 것으로 예상한다. —
  (details: quality-ledger (cq))

- **방언 기록기가 잃어버리던 주석을 보존하는 대가는 계산된 명령어로 남는다.** — `to_jsonc`와
  `to_json5`가 쌍마다 지금까지 보지 않던 두 주석 칸을 읽게 되었고(#359), `ir_gate.py`는 넓어진 쌍
  반복을 `to_jsonc_medium` +4.77%, `to_json5_medium` +3.78%, `to_json_medium` +0.94%로 값매겼다.
  `.ci/ir-baseline.json`을 강제 이미지에서 다시 만들고 세 항목을 `--accept-drift`로 지목했다. 같은
  갱신으로 `to_toml_medium`은 -4.90% 조여졌다 — 기준선이 제자리에 있는 동안 쌓인 변동이지 후퇴가
  아니다 — 그리고 두 harness 실행 파일의 SHA-256을 기록했으니 뒤의 차이는 자기가 서술하는 바이트를
  지목한다. 허용치는 그대로다. — (자세히: quality-ledger (cg))

- **태그 출력은 표 기반, escape 은 더 이상 할당하지 않는다.** — 태그 인코딩을 해석기 자신의 문자
  클래스로 옮기면 1 바이트별 소속 판정이 직렬화 hot path 에 들어오므로, 128 항의 컴파일 시점 표가
  됐습니다(영문 숫자 판정도 같은 조회에 병합）. `%XX` escape 은 16 진 표에서 쓰고, escape 문자마다
  `String` 을 만들던 `format!` 을 없앴습니다. — (details: perf — Leaderboard & Performance Status)

- **null 키가 많은 문서 해석이 선형 시간.** — null 키 접기는 null 키마다 맵을 재스캔했습니다. 전부
  null 인 문서에서는 안 보이고（접힌 항목이 slot 0）, 문제 되는 모양에서는 2 차 함수입니다：서로
  다른 키 2k 다음에 null 키 2k 면 입력 4 배에 12.4 배 증가（8k + 8k 에서 99 ms）. 맵은 null 키의
  slot 을 기억하고 스캔은 정확성 fallback 으로만 남깁니다. — (details: quality-ledger (ae))

- **10% 미만 논쟁은 커밋된 명령어 기준으로 결론.** — 두 runner 잡은 `serialize_block_scalars` 를
  1.44% 차이로 재는데 같은 바이너리의 반복은 ±0.0005% 입니다. runner 소음 아래 벽시계로는 아무것도
  결정되지 않으므로, 기준은 만든 환경을 기록하고 `scripts/ir_gate.py` 가 다른 출소를 주석으로
  알립니다. — (details: quality-ledger (am), (as))

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
