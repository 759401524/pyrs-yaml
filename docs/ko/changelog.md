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

- **CI 주간 정기 퍼징** — `.github/workflows/fuzz.yml`가 네 libFuzzer 타깃을
  매주 토요일(수동 실행 및 `fuzz/` 변경 시 자동) 실행하고, 큐레이션한
  `fuzz/seeds/`(과거 크래시 입력 + 수작업 형태 시드)로 매 세션 임시 코퍼스를
  시딩하며, 실패 시 크래시 산출물을 업로드해 '크래시→회귀 테스트→시드→수정'
  파이프라인에 연결합니다. 기계 생성 코퍼스는 계속 git에 넣지 않습니다.
- **엔진용 `cargo-fuzz` 퍼징 기반(`fuzz/`)** — 커버리지 유도형 libFuzzer
  타깃 4종: `parse_yaml`(단일 + 스트림), `yaml_roundtrip`(파싱 → 직렬화 →
  재파싱과 직렬화 멱등), `parse_json`(3 방언 × 3 writer 전 조합 재파싱),
  `parse_toml`(1.0/1.1 및 writer 재파싱). 타깃만 저장소에 추적하고 코퍼스와
  크래치 산출물은 세션별 로컬 생성·gitignore 유지(크래치 발견은 회귀 테스트로
  고정, 코퍼스 파일로는 커밋하지 않음). 첫 실행 1분 만에 가치를 증명 — 아래 주석
  스캐너 수정 참고.
- **`pyrs-ast` / `pyrs-schema`가 `no_std` 지원** — 모든 포맷 엔진이 그 위에 세워지는 두
  기초 크레이트가 이제 `alloc` 만으로 빌드됩니다: `indexmap`과 `thiserror`의 기본 `std`
  feature를 비활성화하고, 새로 추가된 옵트인 `std` feature로 `std::error::Error` 구현과
  `RandomState` 해시를 다시 활성화합니다. `std`는 계속 기본 활성화이므로 기존 모든
  컨슈머는 전에와 동일한 `IndexMap<K, V, RandomState>` 노드 맵 타입을 그대로 씁니다;
  `no_std` 사용자는 `default-features = false`로 옵트아웃하여 고정 시드 해시를 얻습니다.
  Proptest 노드 전략은 새 `test-strategy` feature 뒤로 이동해 일반 빌드에는 속성 테스트
  비용이 없습니다. CI 잡 `no-std-check`이 맨메탈 타깃으로 크로스 컴파일하여 이 성질을
  검증합니다.
- **`pyrs-json` / `pyrs-toml`도 `no_std` 지원** — 두 네이티브 포맷 엔진이 `alloc` 만으로
  빌드됩니다: `std::sync::Arc`은 `alloc`으로, `String`/`Vec`/`format!` prelude는
  `#[macro_use] extern crate alloc`으로 명시 도입, 파싱 시 key/value 스토어는 `pyrs-ast`의
  `NodeMap` 해시 별칭을 재사용, `canonical_float`의 정수값 판정은 core 전용으로
  구현(`f64::trunc`은 std 고유의 메서드). `no-std-check` 잡이 이제 네 크레이트 전체를
  크로스 컴파일합니다.
- **`pyq` 프리빌드 바이너리 릴리스 동봉** — `publish.yml`의 새 `pyq` 잡이 6개 플랫폼용
  네이티브 CLI를 빌드해 아카이브를 GitHub Release에 첨부합니다. 단독 바이너리를 얻는 데
  Rust 툴체인이 필요하지 않습니다.

#### 변경

- **`pyq validate`가 `--input`을 받고 실제 포맷을 존중** — 기존엔 YAML 파서를 하드코딩해
  바꿀 방법도 없었으므로 `pyproject.toml`, `package.json` 등 비-YAML 설정 파일은 무조건
  거부됐습니다. 이제 공유 로더를 거치며
  `--input auto|yaml|json|jsonc|json5|toml`을 받습니다.
- **`pyq` 프리빌드 바이너리, manylinux2014 컨테이너에서 빌드** — Linux 타깃은
  공식 CentOS 7 이미지 안에서 `cross`로 네이티브 컴파일되어, 수제 크로스
  툴체인 없이 glibc 2.17 하한(CentOS 7 / Ubuntu 16.04 / 18.04 / Debian 8 / 9
  커버)을 고정합니다. 이전 zigbuild 안은 x86_64를 한 단계 더 낮게(2.16, 실측
  `getauxval` 마루) 고정했지만, `publish.yml`의 첫 실환경 실행(이 워크플로는
  일반 PR에서는 절대 돌지 않음)에서 zig 스택 전체가 미비(`cargo zigbuild: no
  such command`, armv7 레그의 호스트 기본 `-fuse-ld=lld` 오링크, runner에서
  실행 불가한 바이너리 스모크 테스트)였음이 드러남. 컨테이너 방식은 세 실패
  모드를 커뮤니티 표준 도구 하나로 통합하며, 하한은 runner 자체 glibc보다 두
  메이저 낮습니다.

#### 수정

- **주석이나 앵커 안의 BOM이 문서를 깨지 않음** — U+FEFF은 스트림 선두 BOM으로만
  *허용*되며 문서 안에는 올 수 없다. granit은 이를 디코딩된 주석 텍스트에 담아
  반환하고, 우리 `anchor_name_before` 텍스트 주사도 그것을 앵커 이름에 흡입했다.
  두 위치 모두 그대로(`# note`, `&name`) 출력되고 이스케이프 문법이 전혀 없어서,
  BOM을 다시 내보내면 우리 파서가 그 출력을 거부했다("a BOM must not appear inside a
  document", libFuzzer `yaml_roundtrip` crash-2d14c6f6, 55바이트). 이제 주석과
  앵커 텍스트를 수집 시점에 문서 허용 문자로 걸러내 AST가 유일한 안전 형태가 되고
  모든 출력 지점은 구조상 올바르게 유지된다—빈 주석(#248)이나 고립된 노트
  (#256/#258)에서 쓴 "재읽을 수 있는 형태를 기록한다"와 같은 규칙이다. 주석은 읽을 수
  있는 텍스트를 유지하고(`# a<FEFF>b` -> `# ab`), 남는 내용이 없면 해석 불가능한
  형태로 출력하지 않고 버린다.
- **태그 접미어를 쓸 때 다시 인코딩해 디코딩된 태그도 재파싱됨** — granit은 리더에게
  *디코딩된* 접미어를 주므로 소스의 `!y5%7c`는 `y5|`로 도착한다. writer는 그 디코딩된
  텍스트를 그대로 내보냈지만 `|`는 태그에서 허용되지 않는 문자라 출력이 아예 재파싱되지
  않았다("while scanning a tag, did not find expected whitespace or line break", libFuzzer
  `yaml_roundtrip` crash-b91536ce, 7바이트 `!y5%7c `). 이제 태그 출력은 태그 URI 문자
  집합 밖의 문자를 퍼센트 인코딩한다(`%` 자체도 포함하므로 리터럴 퍼센트가 새 이스케이프의
  시작이 되지 않음). 매 라운드 동일한 표기로 복원되며 이스케이프가 필요 없는 태그는
  그대로 출력된다.
- **블록 항목의 대시 줄에 있는 주석이 실제로 주석 다는 항목에 연결됨** — 후행 주석
  (`Placement::Right`)은 더 뒤의 줄에 있어도 항상 가장 최근 생성된 노드에 붙었다.
  `- :\u{feff}:\n- #e`에서 둘째 항목 대시 줄의 주석이 *첫째* 항목의 값에 붙어 writer가
  그 주석을 첫째 항목 블록 안으로 밀어냈고, 재읽기는 둘째 항목에 연결해 소유권이 매
  라운드 뒤바뀌었다(libFuzzer `yaml_roundtrip` crash-aee06aca). `attach_inline_comment`는
  이제 주석 줄과 역추적 후보를 비교한다. 같은 줄이면 기존처럼 인라인으로, 블록 스칼라
  헤더 줄의 주석도 그대로 연결되고(granit은 노드를 *내용* 기준으로 span하므로 헤더보다
  뒤의 줄이 됨), 더 뒤의 줄에 있는 주석은 다음 노드의 선행 주석으로 앞으로 넘긴다. 앵커,
  블록 스칼라, 빈 컨테이너 슬롯은 그대로다.
- **블록 컨테이너의 후행 주석이 왕복에서 보존됨** — writer는 인라인 주석
  (`meta.comment`, `standalone = false`)을 *블록* 맵이나 시퀀스와 같은 줄에 둘 수
  없다(마지막 항목 뒤에 줄이 남지 않으므로). 그래서 주석을 별도 후행 줄로 옮겨
  기록했다. 재읽을 때 granit은 이를 뒤에 노드가 없는 standalone 주석으로 보고하고
  리시버는 pending 슬롯에 남긴 채 버렸기 때문에 두 번째 직렬화에서 주석이
  사라졌다(`&"\n-\r... #-o` -> `&" \n- ~\n# -o\n` -> `&" \n- ~\n`, libFuzzer
  `yaml_roundtrip` crash-96fa252c). 이제 `DocumentEnd`에서 남아 있는 주석은 완성된
  문서의 루트로(writer가 읽어간 바로 그 슬롯으로) flush되므로 왕복이 안정적이고
  주석도 남는다.
- **블록 헤더 인식을 스칼라의 바이트 span에 고정** — 헤더 재추출(#250)은 파서 줄 번호로
  내용 줄에서 위쪽으로 훑어 첫 `|`/`>`를 취했다. 그래서 키 안의 `|`(따옴표 `"k:yam  |1": |`
  또는 평범 `k:yam  |1: |2`)나 내용 줄의 기호를 헤더로 오인해 잘못된 들여쓰기 지시자를
  해석하고 `|`↔`|1`가 매 라운드 표류했다(crash-cad17b2b, crash-bdf3f15f 확장). 이제 인식은
  스칼라 자신의 소스 바이트 span에 고정되어 그 내용 바로 위 한 물리 줄만 읽고, 꼬리말이
  블록 헤더 문법(들여쓰기 숫자 최대 하나와 chomping 부호 최대 하나, 그 뒤는 공백이나 `#`
  주석으로 줄 끝까지)을 만족하는 첫 기호를 택한다. 모든 형태에서 구성상 정확하고, granit의
  `\r` 줄 이동에 면역(`\n`만 줄바꿈으로 봄)이며, 블록 헤더 핫 경로에서 위쪽으로 줄마다
  다시 훑던 2차 재스캔을 제거한다. 정상 헤더는 그대로.
- **문서 지시자로 시작하는 평범 스칼라도 인용**(#249 보강) — `... `/`--- `로 시작하는
  값(`... k` 등)을 줄 시작에 날것으로 두면 문서 지시자+잘못된 후속 내용으로 재해석
  실패한다(crash-08f05e25). 이제 이 접두 형태도 인용한다.
- **너비 접기가 긴 평범 스칼라의 공백을 망가뜨리지 않음** — `width`를 넘는 평범 스칼라는
  공백에서 접히지만 접은 줄바꿈은 재해석 시 단일 공백이 된다. 2개 이상 연속 공백(또는 탭)
  옆에서 접으면 줄 끝 공백이 남아 재읽을 때 공백 수가 달라져 값이 매 라운드 표류했다
  (libFuzzer `yaml_roundtrip` crash-9ee754bf). 이제 `write_plain_scalar`는 다중 공백
  연속이나 탭을 포함하는 값은 접지 않고 긴 무손실 한 줄로 출력한다. 단일 공백만이면
  그대로 접혀 안정적이고 값은 항상 정확하다.
- **블록 스칼라 들여쓰기 지시자를 내용 행 위에서만 탐지** — `detect_block_header`는
  첫 내용 행에서 위로 스캔하되 그 행에서 시작했기에 `|`/`>`를 포함 내용 행이
  헤더로 해석될 수 있었다. granit은 `\n`만 줄바꿈으로 세므로 소스의 `\r`이
  `key: |2`와 `|` 포함 내용 행을 한 논리 줄에 두고, 재출력 시 `\n` 분리로
  스캔이 다른 줄에 닿아 `|2`와 `|`가 매 라운드 뒤집혔다(libFuzzer
  `yaml_roundtrip` crash-bdf3f15f). 이제 헤더는 블록 내용보다 엄격히 얕은
  줄에서만 찾아 내용이 오판되지 않으며, 하중을 지는 지시자(내용이 선언보다 깊음)는
  보존되고 값은 그대로다.
- **문서 지시자와 같은 평범 스칼라는 이제 인용** — granit은 앞 공백이 있는 `...`를
  문자열 `"..."`로 읽지만 작성자는 이를 날것으로 냈다. 줄 시작의 `...`는 문서 끝
  표지이므로 값이 null(`...` -> `null`)로 재해석돼 매 라운드 표류했다(libFuzzer
  `yaml_roundtrip` crash-41acfbbe). `needs_double_quoted`는 이제 정확히 `...` 또는
  `---`인 값을 인용 대상으로 한다(`---`는 선행 `-`로 이미 포착). 다른 평범 스칼라는 영향 없음.
- **내용 없는 주석은 저장도 출력도 하지 않음** — granit은 맨 `#`/`# `를 빈
  `Event::Comment`로 보고하지만 재읽을 때는 되읽지 않으므로, 작성자가 낸 `#` 줄이
  다음 해석에서 떨어져 여분의 `  # `가 매 라운드 표류했다(libFuzzer
  `yaml_roundtrip` crash-0de6be17). AST·스트림 리시버 모두 trim 후 빈 주석을
  건너뛰어 할 말 없는 주석은 기록도 출력도 하지 않는다. 비어 있지 않은 주석은 그대로.
- **빈 블록 스칼라는 여분의 들여쓰기 지시자를 갖지 않음** — 빈 `|`/`>` 본문은
  들여쓰기를 잴 대상이 없어 granit은 재해석 시 명시적 지시자를 버린다. 그러나
  `detect_block_header`가 `|2`의 `2`를 AST에 읽고 작성자가 이를 다시 내보내
  `|2`가 매회 `|`로 표류했다(libFuzzer `yaml_roundtrip` crash-d4ea8a23). 이제 두
  블록 작성자는 값이 빈 경우 들여쓰기 지시자를 생략해 빈 형태를 멱등으로 만든다.
  논빈 블록 스칼라의 지시자는 그대로 유지.
- **맵 키가 출력 시 앵커·태그·빈 키 인용을 보존** — `write_scalar_for_key`는 키의 스칼라
  토큰을 내면서 키 노드의 앵커와 태그(값 스칼라는 내보내는 속성)를 떨구고 빈 평범 키를
  날것으로 두어, `&f& !&&f&&&  `(빈 문자열의 앵커와 태그) 같은 키가 `:`로 출력돼 null
  `~` 스칼라로 재해석됐다—앵커와 태그가 사라지고 왕복이 `: ~` → `~: ~`로 표류했다
  (libFuzzer `yaml_roundtrip` crash-62bcff6f). 이제 키는 값과 같이 앵커/태그를 싣고,
  빈 키는 인용(`""`)해 null이 아닌 빈 문자열로 재해석된다. 복잡한 키(`? `)는 원래
  옳았다—속성을 내보내는 노드 작성자를 거친다.
- **앵커 이름을 노드별로 granit의 앵커 위치에서 복원, 전문 사전 스캔 폐지** — granit은
  앵커의 `&name` 텍스트를 주지 않으므로, 파서는 raw source에서 손으로 쓴 따옴표/이스케이프/
  주석 상태 기계(`extract_anchors`)를 돌려 이름을 복원하고, 카운터(`anchor_name_idx`)로
  N번째 스캔 이름을 N번째 앵커 이벤트에 짝지었다. 그 위치 기반 짝지기는 상태 기계가 단 하나의
  바이트 류를 오분류하는 순간—맨 아포스트로피(`bas'e`), 묻힌 `&`(`sbb&e`), 한쪽따옴표
  역슬래시—매번 어긋났고, 각각이 separate fix이자 separate libFuzzer `yaml_roundtrip`
  crash가 되었을 뿐만 아니라 이후 모든 앵커까지 잘못 표기했다. 사전 스캔을 제거: granit
  이벤트는 노드가 앵커를 가짐(`anchor_id != 0`)을 표시하고 정확한 source span을 주므로,
  이름은 이제 그 span 위치에서 granit 스캐너와 같은 최대 `is_anchor_char` 묶음으로 국소
  재읽기(`anchor_name_before`), granit의 권위 id를 키로 쓴다. 복원이 위치 격리되니 읽을
  수 없는 바이트 류는 그 노드에만 영향, 다른 앵커 이름은 절대 어긋나지 않는다—표류 가족
  전체를 형태별 때움질이 아닌 구조로 폐한다. 또한 매 파싱마다 전문 스캔 한 번을 줄인다.
  앵커 이름 안 BOM의 emit 표현가능성 격차는 별개의 근본 원인으로 별도 추적.
- **역슬래시가 앵커 스캔에서 한쪽 따옴표 스칼라의 닫는 큰따옴표를
  이스케이프하지 않음** — `extract_anchors`는 따옴표 안에서도 이스케이프 상태
  기계를 돌렸다. YAML 한쪽따옴표 스칼라엔 이스케이프 처리가 없어(`''`만),
  키 닫는 `'` 앞의 `\`(역슬래시로 끝나는 `'a\'` 같은 키)가 `'`를 이스케이프하는
  걸로 읽혀 따옴표가 닫히지 않고 뒤의 모든 `&앵커`가 숨겨졌다—값의 앵커가
  재해석에서 사라져 왕복이 표류했다(libFuzzer `yaml_roundtrip` crash-12f01ee0).
  이제 역슬래시 이스케이프는 두쪽따옴표 안에서만. 앵커 없음/한쪽/두쪽따옴표
  문서는 granit 읽기 그대로 스캔된다.
- **평범 스칼안에 묻힌 `&`를 더 이상 앵커로 읽지 않는다** — `extract_anchors`는
  인용 밖의 모든 `&`를 줍고, 평범 스칼안에 묻힌 것(맨 키 `sbb&e`의 `&`)도 담았다.
  granit은 노드 시작 위치에서만 앵커를 여므로, 그 환상 `&e` 이름이 순서 있는
  `anchor_names`에 쌓여 `register_anchor`의 인덱스 기반 id→name 짝을 어긋나게 하고,
  후진 진짜 앵커가 잘못 표기됐다(`&b`가 `&e:`로 재출력)—왕복이 표류했다(libFuzzer
  `yaml_roundtrip` crash-83cc68c6). 이제 앵커 추출은 `&`를 인용 상태 기계와 같은
  노드 경계 테스트(행두 또는 `\t:,[]{}-` 뒤)로 게이팅해 `sbb&e`는 평범 키로 남는다.
  앵커 없음/정상 앵커 문서는 전과 동일하게 스캔된다.
- **중복 키는 전체 노드가 아닌 값으로 거부** — AST의 `IndexMap`은 전체
  `CustomNode`를 키로 쓰므로, 같은 텍스트이지만 후미 주석/스타일/앵커가 다른 두
  스칼라 키(`key # a` vs `key # b`)는 구분된 채 남았다: 해석 땐 중복이 안 뜨지만,
  직렬화기가 키 장식을 떨어同一한 `key:` 줄을 두 번 내 우리 parser가 재해석에서
  거부했다(libFuzzer `yaml_roundtrip` crash-3b0a7d1d—출력 문서가 재해석 불가).
  중복 탐지 이제 스칼라 키를 `to_yaml`이 내보내는 것과 같은 「값」으로 식별해,
  그런 입력은 첫 해석에서 거부된다. `<<` 병합 키는 계속 면제: YAML은 맵에서
  이를 반복하는 것을 허용한다.
- **빈 블록 용기를 맵 값으로 인라인 직렬화** — 빈 `Mapping`/`Sequence`에는 블록
  형태가 없는데, 블록 형태의 빈 값을 `key:`로 내고 `{}`/`[]`를 다음 들여쓰기에
  두었다. 이를 다시 읽으면 *플로ウ* 수집이 되어 `flow_style`이 반전하고 다음
  라운드에 인라인됐다—`key:\n  {}` 와 `key: {}`가 매 라운드 표류했다(libFuzzer
  `yaml_roundtrip` crash-d0e84310). 이제 빈 용기는 항상 인라인(`key: {}`)이고,
  앵커/태그를 단 값(`key: &a {}`)도 포함한다. 그것들엔 행두 선출력을 건너뛰어
  헤더가 중복되지 않는다.
- **평범한 키 속 맨 어포스트로피가 이후 모든 앵커를 삼켰다** — `extract_anchors`는
  인용된 `&`를 건너뛰기 위해 인용 상태 기계을 돌리지만, 평범 스칼안에 묻힌
  `'`/`"`(맨 키 `bas'e`나 `a'`의 `'`)에서도 토글했다. 그 환상 인용은 문서 끝까지 열린
  채 남아, 프리스캔은 앵커 이름을 하나도 반환하지 않고 `register_anchor`가 모든 노드에
  `None`를 건네 — 앵커가 출력에서 조용히 사라져 왕복이 표류했다(libFuzzer
  `yaml_roundtrip` crash-68da2420). 이제 인용 「열기」는 토큰 경계(행두 또는 `\t:,[]{}-`
  뒤)로 게이팅해 granit과 일치한다. 평범 스칼안의 인용은 리터럴 내용이고, 진짜 인용
  스칼라는 여전히 `&`를 숨긴다.
- **리터럴 블록 스칼라는 첫 줄이 공백일 때 들여쓰기를 강제** — AST는 `|`/`|N` 본문을
  들여쓰기 제거해 저장하고 원문의 명시적 지시자를 버린다. 그래서 첫 내용 줄은 공백으로
  시작하지만 이후 줄은 더 얕은 값(` 1|l\n:t\n`)을 지시자 없이 다시 출력하면, granit이
  더 깊은 첫 줄을 블록 들여쓰기로 삼고 얕은 줄을 디들렌트로 읽어 — 출력이 재파싱되지
  않는다(libFuzzer `yaml_roundtrip` crash-e432d4b8). 리터럴 작성자는 접힘 작성자를
  본떠 바로 그 케이스에서 들여쓰기 지시자를 강제해 자동 탐지를 건너뛰고 선두 공백을
  내용으로 보존한다. 첫 줄이 공백이 아닌 문서는 전과 같이 바이트 단위로 동일하게
  직렬화된다.
- **접기 작성자가 more-indented 줄 「뒤」의 break를 보존하도록 수정** — granit 접기
  규칙은 `leading_blank` 플래트를 추적한다. more-indented 줄(공백 또는 탭으로 시작하는
  연속 줄)은 자기 선행 break를 보존할 뿐 아니라 이 플래트를 세워 그 다음 줄의 break도
  접지 않는다. 출력 쪽 묶음 규칙은 앞부분만 알고 억제 판단을 「이전 줄」 기준으로 내렸기
  때문에, more-indented 줄 뒤에 평범한 줄이 이어지면 묶음에 개행을 하나 과잉 채워 왕복마다
  빈 줄이 하나씩 늘었다(libFuzzer `yaml_roundtrip` crash-b7a2285e). 이제 규칙을 「방금 쓴
  줄」 기준으로 바껴, 어느 쪽 이웃이 more-indented든 r개 묶음은 정확히 r개 물리 개행을
  출력한다. 바이트 멍등성뿐 아니라 완전한 값 충실성(재파싱이 스칼라 값을 보존)으로 고정.
- **접힌 스칼라가 자기 개행을 다시 읽도록 수정** — granit 접힌 읽기는 선행이든
  텍스트 줄 사이든 빈 줄 k개를 정확히 k개 개행으로 읽는데, 줄 분할 작성자는
  묶음마다 빈 줄을 하나 덜 찍어 접힌 값은 왕복마다 개행이 하나씩 줄었다
  (libFuzzer `yaml_roundtrip` crash-490c4beb: 4 → 3 → 2 → …; crash-6288e5be도
  선행 빈 줄이 같은 방식으로 표류). 이제 접기 인식 작성자로 r개 묶음은 빈 줄
  r개를 차지하고(선행은 헤더 개행 포함), 모든 연장이 구성적으로 폐쇄(내부·선행
  ·more-indented 연속 줄 모두 1~5 길이 검증; more-indented 줄은 자기 break를
  보존하므로 빈 줄 하나 덜).
- **블록 스칼라 출력을 재파싱에 대해 닫히도록 수정** — granit 판독 형태에
  직렬화기가 맞지 않았던 두 가지: 말미 빈 줄을 담은 `Clip` 블록 스칼라 값은
  `Keep` 지시자로만 왕복된다(Clip 판독은 말미 빈 줄을 제거하므로—어느 위치에서나
  같은 값으로 재독되는 유일한 헤더 형식—libFuzzer `yaml_roundtrip`
  crash-c18cb1fd), 출력 시 승격. 블록 스칼라의 인라인 댓글은 별도 줄이 아닌
  헤더 줄(`y: |  # c`)에 실린다—종종 블록 내용에 흡수됐다(crash-cfb3fa83).
  두 규칙 모두 출력 쪽 정규화만 수행: 기존에 안정적이던 문서는 바이트 단위로
  동일 출력을 유지.
- **앵커 이름 문법을 granit과 정렬 — 표류 계열 전체를 근본 해결** —
  `extract_anchors`/`scan_anchor_name`에 granit 스캐너에는 없는 두 개의
  자체 분기가 늘었다: 인용 앵커 형식(`&"a b"` 공백 포함)과 값 표시 규칙(공백/EOL
  앞의 `:`로 이름을 종료). granit은 이름을 `is_anchor_char`의 최대 연속으로
  읽는다(`:`/`#`/`"`/`&`는 평범한 이름 문자; 공백/개행/플로우 표시자에서만 종료 —
  granit 자신의 issue14 테스트). 두 문법의 불일치는 매번 id↔이름 대응을 어긋나게
  해 왕복을 깨뜨렸고, 아래 네 항목(#215/#218/#227/#228)은 모두 이 하나의 원인
  증상이다. 이제 스캐너는 granit과 완전히 일치하며(최대 연속 + 앵커 토큰을 원자적으로
  건너뛰어 이름 속 `"`/`#`이 더 이상 인용/주석 상태를 어지럽힘 없음),
  `write_anchor_tag`는 `&name`을 그대로 출력한다 — 폐합이 구성상 성립해 형태별
  임시 대응이 흡수되고, 인용 앵커(애초 왕복 불가)은 제거.
- **인용된 앵커 이름이 줄바꿈을 삼켰던 문제** — `scan_anchor_name`의 인용 분기가
  버퍼 뒤쪽의 임의의 `"`를 닫는 인용부호로 취급해, `&"X-<CR>:&"X-`에서 개문자를
  넘어 이름을 `X-\r:&`로 읽었다. 직렬화기가 그것을 그대로 출력했고 재파싱은 왕복마다
  한 겹씩 더 감쌌다(커지는 libFuzzer `yaml_roundtrip` 비멱등, 11바이트). granit은
  CR/LF에서 앵커 토큰을 끝내므로, 줄 종결을 넘은 닫는 인용부호는 더 이상 인용
  앵커로 인정되지 않아 이름이 한 줄로 유지되고 재출력도 안정된다.
- **중첩된 자기참조 병합 앵커가 네이티브 스택을 넘쳤던 문제** — `&b`로
  앵커된 매핑의 본문이 `*b`를(직접 또는 두 번째 `&b`를 통해) 재사용하면,
  경로 순환 가드가 이미 pop된 상태에서 `resolve_mapping_merges`의 후미
  재귀로 흘러들어, 각 탐색이 앵커의 새 복제본을 재전개하여 하강이 끝없이
  커졌다(libFuzzer `parse_yaml`, 58바이트 `bas: &b … <<: *b …`). 이제 후미
  탐색은 매핑 *고유* 자식에만 재귀한다(병합된 복제본은 전개 루프에서 가드
  아래 이미 해석됨). 또한 `MAX_MERGE_DEPTH` 한도가 남은 폭주를 우아하게
  정지시켜, 파서의 컨테이너 깊이·직렬화기의 `max_depth` 가드와 정렬한다.
- **`:`로 끝나는 앵커 이름이 불안정한 형태로 출력됐던 문제** — `write_anchor_tag`가
  모든 앵커를 맨 `&name` 토큰으로 썼다. 파싱된 앵커 이름이 `:`로 끝나면(종결되지
  않은 인용 앵커 `&"X-::…:`를 통해), 끝의 `:`가 출력된 공백과 합쳐져 값 표시자가
  되고 재스캔 시 사라져, 각 직렬화 왕복마다 한 글자씩 잃었다(42바이트 libFuzzer
  `yaml_roundtrip` 발견, `fmt(fmt(x)) != fmt(x)`). 안전하지 않은 이름(끝에 `:`,
  포함된 공백이나 흐름 표시자)은 이제 인용된 `&"name"` 앵커로 출력되며, 원시
  스캐너가 닫는 인용부호까지 읽어 왕복 간 정확한 바이트를 보존한다.
- **원시 앵커 스캐너가 텍스트로 보존할 수 없는 앵커를 만들어냈음** —
  `extract_anchors`가 공백/줄바꿈 뒤의 `:`를 앵커 이름에 포함하고(`&&&&:` →
  `&&&:`) 주석 텍스트에서 앵커를 수집하며, 채택된 이름 안의 중복 `&`를
  다시 스캔(`&&&&`가 유령 앵커 `&&&`, `&&`, `&`를 생성)해 이후 id→이름
  대응이 모두 어긋났습니다. 회귀 테스트로 원본 크래시를 재현 가능하게 했습니다.
- **큰따옴표 스칼라가 두 번 디코딩됨** — granit이 이미 이스케이프 해제된
  값을 전달하는데도 두 리시버가 `unescape_double_quoted`를 다시
  적용했습니다. `a: "\\n"`(글자 그대로 `\` `n` 두 문자)이 개행으로
  조용히 축소되고 직렬화/재파싱마다 역슬래시가 하나씩 사라졌습니다
  (libFuzzer `yaml_roundtrip`: `!-# \\f"<TAB>0:!`). 이제 두 호출 지점은
  그대로 통과하며 stream/AST 단위 테스트로 단일 디코딩 계약을 고정합니다.
- **닫지 않은 따옴표 앵커 이름이 행 나머지를 삼켰음** — `&"X-<CR>:`에서
  `extract_anchors`의 quoted 스캔이 닫는 따옴표를 끝내 못 만나 행 끝까지 모아

    raw CR과 콜론이 앵커 이름에 들어갔고, 직렬화가 `&X-\r:`를 그대로 내보낸 뒤
    granit은 공백에서 앵커 이름을 끊어 재파싱 결과가 `X-`가 됨 —
    `fuzz/yaml_roundtrip`이 6바이트 입력으로 직렬화 멱등
    (`fmt(fmt(x)) == fmt(x)`)을 깼음. 닫지 않은 `"`는 이제 granit 비따옴표
    앵커 토큰이 멈추는 바로 그 문자에서 멈추고, 진짜 `&"quoted anchor"`
    (공백 포함 이름)는 그대로입니다.
- **JSON 주석 스캐너가 멀티바이트 문자 중간에서 panic** — `ws()`의 행 주석과
  미종결 블록 주석 스캔이 `pos`를 바이트 단위로 진행해 후행 멀티바이트 문자
  (U+FEFF 등) 내부에 pos가 남고, 다음 `&text[pos..]` 슬라이스가 "not a char
  boundary"로 panic했다(`fuzz/parse_json`이 약 25초 만에 발견:
  `\r\r{aMNaN/*0\u{feff}`). 이제 행 주석은 코드포인트 단위로 진행하고 미종결
  블록 주석은 `/`로 되감겨 모든 실패 경로가 다시 타입화된 오류입니다.
- **Linux 프리스레드(`cp314t`) wheel을 Release에 동봉** — wheel 빌드 매트릭스가
  Windows와 macOS만 프리스레드 산출물을 만들어, Linux의 GIL 없는 인터프리터
  사용자는 설치 수단이 없었습니다: GIL 있는 `cp38-abi3` wheel은
  `Py_GIL_DISABLED` 빌드와 ABI 비호환이고 `abi3t` wheel은 CPython 3.15부터
  적용되기 때문입니다. 이제 `linux` 잡이 x86_64에서 이미지 자체의
  프리스레드 인터프리터로 manylinux cp314t wheel을 빌드하고 `3.14t` venv에서
  스모크 테스트를 통과한 뒤 Release에 첨부합니다(aarch64는 제외: 비-abi3
  wheel 빌드는 대상 인터프리터를 실제로 실행해야 하는데, qemu-user 환경에서
  그 실행이 실패하기 때문).
- **`pyq` 릴리스 잡이 Linux 산출물을 실제로 빌드** — 크로스 아키텍처 레그는
  `cross` 에뮬레이션 컨테이너용 qemu binfmt 핸들러를 등록하고, 스모크 테스트는
  manylinux 이미지 *내부*에서 갓 빌드한 바이너리를 실행합니다. 호스트에는 qemu
  번역기가 있어도 이기종의 `/lib/ld-linux-*.so` 로더가 없어 aarch64/armv7 바이너리를
  호스트에서 직접 exec하면 `main` 이전에서 죽습니다. 이제 모든 레그가 업로드 전
  빌드·자기검증됩니다.
- **블록 스칼라가 명시적 들여쓰기 지시자를 보존** — `key: |2`로 작성된 본문은 직렬화할 때 `2`가
  조용히 사라져, 본문 첫 줄이 뒤 줄보다 깊게 들여쓰인 경우(딱 `4RWC.yaml` 모양: 첫 줄 6,
  후속 줄 4) 출력의 재파싱 결과가 입력과 달랐습니다. 지시자가 없으면 리더가 본문 첫 줄에서
  들여쓰기를 자동 감지하므로, 지시자 유실은 외관이 아닌 의미 변경입니다. 이제 지시자가 AST에
  실리고 사양 순서(`c-b-block-header`: chomping 먼저, 들여쓰기 나중 — 따라서 strip은 `|-2`)로
  재출력되며, 라이터는 본문 기준을 부모 노드 열이 아니라 헤더를 실은 줄의 열에서 잡습니다.

### [v0.17.0] — 2026-10-01

#### 추가

- **pyq CLI 패리티 플래그** — `pyq fmt`에 `--indent N`(블록 들여쓰기, 기본 2),
  `--width N`(스칼라 소프트 줄바꿈 열폭, 0이면 비활성), `--sort-keys`(직렬화
  단계 문서 전체 키 정렬), `-i/--inplace`(파일 그 자리 다시 쓰기) 추가,
  `pyrs-yaml-core::SerializeOptions`의 모든 옵션을 노출하고 Python CLI의
  `fmt --indent`와 맞춤. `pyq to-json`에 상호 배타적인 `--jsonc` / `--json5`
  방언 출력을 추가하여 `pyrs-json`의 주석 보존·JSON5 표기 직렬화기
  (`to_jsonc_text*`, `to_json5_text*`)를 연결.
- **pyq JSONC/JSON5 입력 방언** — `Format` 열거형에 `--input jsonc|json5`
  추가(자동 감지는 `.jsonc` / `.json5` 확장자도 인식). `pyrs-json`
  네이티브 방언 파서를 거쳐 주석과 JSON5 표기가 AST에 실리며,
  `to-json --jsonc`과 결합하면 한 명령으로 주석 보존 JSONC→JSONC 왕복이
  됩니다. `--all-docs`는 단일 문서 방언에서 안정적 오류 메시지로 거부.
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
- **`README.md` / `README.zh-CN.md`에 네이티브 `pyq` CLI 문서화** — 두
  README의 Python CLI 절 옆에 `pyq` 절을 추가했습니다. 체크아웃에서의
  설치 방법, 실제 실행 가능한 예시 3개, 전체 명령 목록을 담고 자세한 내용은
  pyq 가이드로 연결합니다.

#### 수정

- **`pyrs-json` 모듈 문서** — 옛 설명은 "주석은 읽을 때 버려지고 다시
  출력되지 않는다"였으나, #122 이후 주석은 AST 주석 슬롯에 실려
  JSONC/JSON5 직렬화기가 복원합니다.

### [v0.16.0] — 2026-10-01

#### 추가

- **JSONC block-comment 핫스팟 벤치** — objective §테스트 커버리지 5 가 “block-comment” 热点 样本로 指定。以前 inline `//` 만 计量. 新 fixture 가 `test_load_jsonc_block_comments` 驱动: 50 pair + header/footer, 各项 独立 `/* item N */` 以及 末尾 `value /* trailing */` 持有, 块走查 回归을 CodSpeed 로 可视化.
- **YAML 의 PyYAML + ruamel.yaml 跨库 对拍** — objective §테스트 커버리지 3 이 两者 指名 oracle. 以前 `test_benchmark_crosslib.py` 벤치 + 特性 support printout 만。`tests/test_yaml_crosslib.py`: 20 正規 文档 × 5 对拍 面 + 2 文档化 分歧(duplicate-key 严格性, YAML 1.1 傳統 bool schema-scope) = 122 테스트. optional dep skipif 자동 降格.
- **`load_toml` tomlkit 跨库 对拍** — objective §테스트 커버리지 3 이 tomlkit 을 oracle 로 指名. 以前은 벤치만. `tests/test_toml_crosslib.py` 24 件: 11 正規 구조物에서 pyrs / tomlkit / tomllib 三者 一致, `>i64` 拒否를 仕樣 準據(TOML v1.0 §Integers: 64bit signed)로 固定, `-2^63` 境界(PR #174 修正) 確認. optional dep, skipif 자동 降格.
- **orjson 을 strict-JSON oracle 로** — objective §테스트 커버리지 3 「orjson 과 자리별 비교」는 그동안 벤치만。16 정규 문서 값 일치, 12 비정규 형식(주석, trailing comma, single quote, bare `NaN`/`Infinity`/`-Infinity`, hex, leading 0, `+.5`, `5.`) 양측 거부 단언. stdlib `json.loads` 는 `allow_nan=True` 하에서 bare literal 을 받아들이지만 orjson 은 거부 — RFC 8259 oracle 로서 stdlib 보다 강함. optional dep, `skipif` 자동 강하.
- **CLI ↔ Binding 대칭 게이트(`tests/test_cli_binding_parity.py`)** — Pillar 1 의 “CLI 와 Python Binding 양쪽이 동일한 기능” 선언을 문서관에서 실행 가능한 계약으로 승격. CLI 등록 명령을 18 개 고정 목록과 비교(cyclopts 의 `--help`/`-h`/`--version` 의사 명령 제외), 각 `to-X` / `from-X` 동사에 `YamlDocument.to_X` / `from_X` / `load_X` 대응 필수. `load_*` 패밀리(json/jsonc/json5/toml) 네 형제 대칭 단언, editing/validate/compliance 동사는 live Python API 에 매핑. 어느 한쪽 드리프트는 CI 실패로surface.
- **`load_json` 속성 테스트 + CodSpeed 벤치** — Hypothesis(`test_load_json_matches_stdlib_json`, `test_load_json_matches_load_jsonc_on_strict_domain`)가 생성된 모든 정규 문서에 대해 STRICT loader 와 `json.loads` 의 일치, 및 두 loader 의 strict 영역 문자 그대로 동의를 고정. 고속 경로 확대나 AST 경로 드리프트는 속성 실패로surface. 3 개 CodSpeed wall-time 벤치(`test_load_json_large` / `_floats` / `_escapes`) 가 `load_jsonc` 샘플을 미러링하여 STRICT binding 레이어 자체를 회귀 추적에 포함.
- **`load_json` (엄격) — `load_*` 패밀리 대칭 완성** — binding에는 이미 `load_jsonc` / `load_json5` / `load_toml` 이 있었으나 엄격한 RFC 8259 대응 함수가 빠져 있었습니다. `pyrs_yaml.load_json(s)` 는 정규 입력에서 `json.loads` 와 문자 그대로 일치하며 JSONC/JSON5 확장(`//`, `/* */`, 후행 쉼표, single quote, bare `Infinity`/`NaN`, `0x…`)을 타입이 지정된 `YamlParseError` 로 거부합니다. 고속 경로는 `load_jsonc` 와 `json_fast::try_load` 를 공유(비정규 바이트는 모두 bail → 문법 확대 리스크 0); 거부 대상은 STRICT `from_json` AST 경로로 라우팅. 아래 CLI ↔ Binding 대칭 항목의 마지막 공백을 메워 Pillar 1 이 완성됩니다. `pyrs_yaml.__init__` 에서 재내보내기 및 `__all__` 등재; `.pyi` 는 `maturin generate-stubs` 로 재생성.
- **방언 writer 고정점 속성** — `fmt_pbt.rs` 헤더가 약속한 writer 고정점(writer 출력을 재파싱→재직렬화하면 문자 그대로 동일)이 구현돼 있지 않았습니다. 4개의 proptest가 이제 JSON/JSONC/JSON5/TOML에서 이를 지키며(유일한 입력 필터 `json_object_domain`는 서로 다른 키가 같은 JSON 이름을 만드는 수작업 AST를 제외 — RFC 8259 object 영역 밖), 이 게이트는 즉시 3개의 실제 주석 충실도 결함을 발견했습니다(아래 수정 참조).
- **핫스팟 벤치 코퍼스** — 7개 CodSpeed wall-time 벤치가 역사적으로 취약한 직렬화 경로를 대상으로 합니다: YAML 블록 스칼라 문서(6종 헤더 표기 `|`/`|-`/`|+`/`>`/`>-`/`>+`) 및 주석 밀도 문서, TOML multiline 문자열/진법 정수/underscores/지수/날짜시간, JSON5 특수 수치 형식(16진, `+.1`, `5.`, `Infinity`, `NaN`, single quotes, trailing commas). 픽스처는 `tests/data/yaml_samples.py`, 벤치는 `tests/test_benchmark_api.py`. 이 코퍼스 구축 과정에서 중첩 블록 스칼라 들여쓰기 버그가 발견되었습니다.
- **텍스트 수준 재파싱 게이트(`prop_output_always_parses`)** — Rust proptest 스위트가 생성된 모든 AST의 직렬화 출력이 파서에서 재파싱 가능함을 주장합니다. AST 대 AST 왕복 프로퍼티는 재파싱 불가 형상(`try_roundtrip`이 `None`)을 조용히 건너뛰었음. 새 게이트가 첫 실행에서 6개 실제 버그를 발견했으며, 각각 targeted Rust 단위 테스트와 Python 회귀 클래스(`TestNestedBlockScalarIndent`)로 고정.
- **toml-test 적합성 하네스** — `tests/test_toml_test_suite.py` 가 공식 [toml-test](https://github.com/toml-lang/toml-test) 를 `test_yaml_suite.py` 가 YAML 스위트를 실행하는 것과 동일한 방식으로 실행합니다: 추적되지 않은 로컬 아티팩트, 부재 시 `skipif`, 실측 하한 게이트, 그리고 디코딩 비교를 위한 타입 태그 어댑터.
- **TOML 시간 타입 정상 디코딩** — 날짜 전용과 시간 전용은 각각 서로 다른 `!date`/`!time` 태그를 가져(날짜시간은 `!timestamp` 유지) `date`/`time.fromisoformat` 를 사용합니다. toml-test 가 맨 시간(`07:32:00`), 초 생략 시간(`13:37`), 소문자 구분자 날짜시간(`1987-07-05t17:45:00z`)이 유효한 TOML 에서 `ValueError` 를 던지는 것을 발견. `!time` 은 생략된 초를 채우고 `!timestamp` 는 소문자 `t`/`z` 를 정규화합니다.
- **TOML 제어 문자 엄격성** — 기본·자구·다중줄 문자열 내부에서 raw C0 제어 코드(NUL, FF, DLE, US 등)와 DEL(U+007F) 을 거절합니다(탭 및 다중줄 줄바꿈만 허용). toml-test 의 `invalid/control` 이 13 건의 오인식 문서를 발견. 코멘트 본문·bare CR 검사는 후속 항목.
- **TOML 숫자 리터럴 엄격성** — 선행 0 을 가진 10 진수(`01`, `-01`), 진수 접두 정수에 부호(`+0x1F`, `-0b101` — `signed-int` 는 10 진수 전용), 후행/이중 언더스코어(`1_`, `1__0`) 를 거절합니다. toml-test 의 `invalid/integer`+`invalid/float` 이 23 건의 오인식을 발견(총 71->48). 기존 “진수 정수에 부호 허용” 은 사양 위반이었습니다.
- **TOML 인라인 테이블 키 충돌 엄격성** — 인라인 테이블은 정의된 경로와 동일·확장·피복 관계인 점 키(`{ a = 1, a.b = 2 }`, `{ a.b = 1, a.b.c = 2 }`)를 거절합니다. 형제 경로(`{ a.b = 1, a.c = 2 }`)는 허용. toml-test `invalid/inline-table` 의 duplicate-key/overwrite 가 발견(오인식 총계 48->39).
- **TOML 비 ASCII 문자열 충돌 수정** — 기본 및 다중줄 기본 문자열 파서가 바이트 단위로 전진하며 멀티바이트 입력(U+00A0 등)에서 문자 중간을 슬라이스해 panic 하던 것을 문자 단위로 소비하도록 수정. toml-test 로 발견, #153 의 한줄/JSON 판을 보완.
- **형식 퍼징 + 견고성 수정** — 새 `proptest` 속성 테스트가 TOML/JSON/JSONC/JSON5 파서와 라이터를 퍼징(no-panic + 재파싱 가능성). 발견·수정: TOML 및 JSON 문자열 파서의 중각 문자 슬라이스 panic, 및 JSONC/JSON5 인라인 `//` 주석이 후속 `,`/`}`를 삼키는 버그.
- **YAML merge/별칭 속성 퍼징** — 정상 형태의 앵커/별칭/병합-키 문서(단일 별칭, 별칭
  시퀀스, 인라인 맵이 들어간 시퀀스, 인라인-맵 병합, #166 이 거절하는 스칼라/널 병합
  소스)를 새로 생성하며 자기 참조 앵커와 반복 별칭 참조도 포함합니다. 기존에는 모든
  속성 테스트가 `arb_custom_node()` 를 썼는데 이는 `meta.anchor` 만 내고 `Alias` 노드는
  절대 만들지 않아 별칭 해석과 병합 전개 경로(#163/#166 의 바로 그 구조 등급)가
  프로세스 내 퍼징 대상이 아니었습니다. `prop_merge_alias_never_panics` 은 파싱 + 병합
  해석이 panic 이나 네이티브 스택 오버플로 없이 수행되고 파싱된 트리는 재직렬화·재파싱에서
  안정적임을 단언합니다.
- **CLI 형식 대등성** — CLI 에 `to-toml`/`from-toml`, `to-jsonc`/`from-jsonc`, `to-json5`/`from-json5` 추가(기존 `to-json`/`from-json` 본뜬). 바인딩이 처리하는 모든 형식을 명령줄에서 사용 가능.
- **JSON 문자열 이스케이프 고속 경로(성능)** — `load_jsonc` 가 단순한 2바이트 이스케이프 8종을 인라인 디코딩하여 문서 전체를 AST 경로로 피하지 않습니다. 이스케이프가 있는 JSON 이 고속 경로에 탑니다(AST 경로 대비 약 15 배 빠름). 값은 `json.loads` 와 일치; `\u`·잘못된 이스케이프는 계속 AST 경로 경유.
- **JSON 부동소수점 고속 경로(성능)** — `load_jsonc` 가 정규 부동소수(소수/지수)를 문서 전체를 AST 경로로 피하지 않고 곧바로 Python 객체로 해석합니다. 값은 `json.loads` 와 완전히 일치(올바르게 반올림된 parse). 새 bench 가 이 분기를 회귀 추적합니다.
- **JSON 문자열 직렬화 고속화(성능)** — 이스케이프가 필요 없는 문자열은 문자 단위 UTF-8 재인코딩 대신 한 번의 `push_str`로 일괄 복사합니다. 문자열이 많은 `to_json` 은 약 35% 빨라짐(41→27 ns/건), 출력은 바이트 동일.
- **`YamlDocument.to_toml()`** — 문서가 `to_json`/`to_jsonc`/`to_json5`와 동일하게 AST에서 TOML 을 바로 출력합니다. `to_toml(doc.to_yaml())` 의 직렬화→재파싱 왕복이 불필요해지고 출력은 바이트 동일. 라이터 자체는 `tomli_w` 보다 약 4.3배 빠름.
- **`to_json` 네이티브 직렬화기(성능)** — `YamlDocument.to_json`이 `to_dict()` + `json.dumps` 이중 변환을 멈추고 네이티브 엔진을 사용. ASCII 는 바이트 단위 동일, 약 10배 빨라짐(1200건 ~1450µs→~120µs, `json.dumps`를 앞섬). 비ASCII는 `\uXXXX` 대신 원시 UTF-8(`to_jsonc`/`to_json5`와 일치)로 출력되며 유효한 JSON 유지.
- **JSON 키 직접 출력(성능)** — 라이터가 매핑 키를 출력 버퍼에 바로 기록(키별 `String` 할당 폐지). 컴팩트 `to_json` 이 약 2배 더 빨라짐(~120µs→~60µs), 바이트 동일, 직렬화가 현장에서 #2(orjson만 상위).
- **JSON 로드 고속 경로** — `load_jsonc`가 정준 strict JSON을 `CustomNode` AST 없이 직접 Python 객체로 변환(실측 약 5-6배 빨라 stdlib `json.loads`를 앞섬). 비정준 입력(실수/이스케이프/주석/범위 초과 정수/후행 쉼표)은 일반 경로로 폴백하여 값과 오류가 불변.
- **TOML 다중 줄 문자열 무결성** — TOML 다중 줄 문자열을 `ScalarStyle::Literal` YAML 블록으로 투영(텍스트 허브 왕복 유지)하여 `to_toml`이 이스케이프 한 줄로 붕괴시키지 않고 `"""` 블록으로 재출력한다. 값은 바이트 단위로 왕복하고 출력은 멱등이며 한 줄 문자열은 한 줄로 유지된다. 기존 `Literal` 재사용으로 AST 구조 변경 없음.
- **TOML 문서 수준 주석 무결성** — `to_toml`이 루트 매핑의 선행 주석을 출력하게 되어, 문서 맨 앞의 독립 `# 주석`이 TOML→허브→TOML 왕복에서 유지된다(JSON 라이터의 `emit_root_leading`에 해당). 네이티브 TOML 파싱과 주석 없는 문서는 영향 없다.
- **JSON5 Unicode 식별자 키** — 큰따옴표 없는 객체 키가 ASCII 제한을 넘어 완전한 Unicode `ID_Start` / `ID_Continue` 집합을 받아들인다. `from_json5`/`load_json5`가 `{ é: 1, 名: 2, हिन्दी: 3 }`를 파싱한다. rustc 자체 어휘 분석이 쓰는 `unicode-ident` 테이블로 스크립트별 정확히 적합(식별자 중간 결합 문자 포함). JSON5 모드에서만 켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 따옴표를 요구한다. `\uXXXX`의 고아 UTF-16 서러게이트는 계속 거부(Rust `String`이 무손실로 표현 불가). 의존성 1개 추가(`unicode-ident`).
- **JSON5 Unicode 구조 공백** — `from_json5`/`load_json5`가 RFC 8259의 4개(탭/스페이스/줄바꿈/CR) 외에 JSON5가 추가한 공백을 토큰 구분자로 받아들인다: 수직 탭, 폼 픽, NBSP(U+00A0), 모든 Unicode `Zs` 구분자, LS/PS 줄 종결자(U+2028/U+2029), ZWNBSP(U+FEFF). `std`의 `char::is_whitespace`(JSON5가 공백으로 보지 않는 NEL U+0085 제외)에 U+FEFF를 더해서 구현, 신규 의존성 없음. JSON5 모드에서만 켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 전부 거부한다.
- **JSON5 줄 연속과 `\'` 이스케이프** — 큰따옴표 JSON5 문자열이 JSON 집합 밖의 두 이스케이프를 받아들인다: 줄 종결자 바로 앞의 역슬래시(줄 연속으로 둘 다 제거)와 이스케이프된 작은따옴표(`\'`→`'`). JSON5 모드에서만 켜지므로 엄격 `from_json`/`from_jsonc`는 종래대로 둘 다 거부한다. #125의 작은따옴표 처리를 반영해 JSON5 문자열 무결성을 완성한다.
- **JSON5 문자열 이스케이프 `\v`와 `\0`** — `from_json5`가 수직 탭(`\v`)과 NUL(`\0`)을 큰따옴표/작은따옴표 문자열 모두에서 받아들인다. 엄격 JSON / JSONC는 종래대로 거부한다. #120(수치)·#124(수치 시맨틱)과 함께 JSON5 문법을 완성한다.
- **load_json5의 JSON5 수치 시맨틱** — `load_json5`가 JSON5 전용 수치 형식(`0x1F`→31, `+7`→7, `5.`→5.0, `Infinity`/`NaN`)을 새로운 `Schema::Json5`로 실수 값으로 해석한다. 엄격 JSON/JSONC 로더는 불변이며 `to_json5_text`는 원본 표기 그대로 출력한다.
- **JSON5/JSONC를 공개 API에서 사용 가능하도록** — `pyrs_yaml.from_json5` / `load_json5`, 그리고 `YamlDocument.to_jsonc()` / `to_json5()`(네이티브 엔진 경유로 주석과 JSON5 스타일 보존). 함께 도달성 결함도 수정: `from_jsonc` / `load_jsonc`가 `pyrs_yaml` 패키지에 재수출되지 않아 `AttributeError`가 났는데 이제 `__all__`에 포함된다. `to_jsonc`/`to_json5`는 `emit_root_leading`으로 문서 수준 standalone 주석을 보존한다. `test_benchmark_api.py`에 JSON 계열 벤치마크를 추가했다.
- **JSON5 라이터(`to_json5_text` / `to_json5_text_pretty`)** — 계약 B 2단계. AST를 JSON5로 직렬화해 파서 보존한 한따옴표 문자열과 `0x…`/`.5`/`+7`/`Infinity`/`NaN` 숫자 형식, 그리고 `//` 주석을 복원한다. 키는 항상 인용부호를 붙인다(무손실). 내부에서 `Mode`(Json/Jsonc/Json5)를 공유하며, 엄격·JSONC 출력은 변하지 않는다.
- **파서에 JSON5 수치 형식 추가** — `from_json5`(새 `allow_json5_numbers` 축)가 십육진(`0xDECAF`), 전/후 소수점(`.5`, `5.`), 부호 `+`(`+7`), 선행 0(`07`), 그리고 `Infinity` / `NaN` / `-Infinity`를 받아들인다. 각 형식은 원문을 보존해 훗날 JSON5 라이터가 그대로 재현할 수 있다. STRICT / JSONC는 이 축을 OFF로 두어 종래대로 거부한다. `from_jsonc`의 낡은 "주석은 버려진다" doc도 정정했다.
- **TOML 인라인 테이블 내부 주석 보전** — PR #119가 인라인 테이블 내
  `# ...` 주석을 잡아 (멤버 위 독립 행 -> leading, 값 같은 행 후 -> trailing)
  IR 로 전달해 워드트립이 도는 혀 주석이 손실되지 않습니다. 장식이 없는
  인라인 테이블은 깁깐한 한 줄 형식을 유지하며, 배열 내 노안된 장식
  테이블은 멀리 줄로 승급됩니다. 동시에 #114 재재 버그
  (`skip_all_blank`가 standalone 주석 자신의 냄배를 단련 줄로 오인)을 수정했습니다.
- **YAML 리시버가 standalone 주석을 `decor.leading_comment` 에
  기록** — PR #117b 로 마지막 엔진(granit-parser 리시버)이 #114 /
  #115 에서 도입한 새 슬롯으로 이전했습니다. scalar / mapping /
  sequence 의 standalone note 가 `NodeMeta::decor.leading_comment`
  에 실리고 오래된 `comment(standalone = true)` 에는 쓰지
  않습니다. #117 의 규정화로 수작업 fixture 는 여전히 동등하고
  `CustomNode::remove_comment` 가 **두 슬롯**을 원자적으로
  비우므로 Python 의 `Node.remove_comment()` 는 YAML 문서에서도
  변화 없이 동작합니다.
- **슬롯 간 standalone 주석 규정화 + Python
  `Node.leading_comment`** — `NodeMeta::eq` / `Hash`가 standalone 주석을 새로운
  `leading_comment` 슬롯(TOML / JSON 엔진)이나 오랜
  `comment(standalone = true)` 슬롯(YAML 리시버 + 수곤픽스테 기본)에
  있는지 여부와 관련하지 않고 동일 개념으로 처리합니다. setter / remover는
  두 슬롯에 원자적으로 작용하며 YAML 서리라이저는 규정화된 뷰를 읽어
  `to_yaml(toml_ast)`의 leading note가 사라지지 않습니다. Python의
  `Node.leading_comment` getter / setter / remover가 `Node.comment`를 미러링하며
  TOML / JSONC 소스의 standalone 주석을 처음으로 Python 호출자에게 노출합니다.
- **TOML 1.1.0 문법** — `from_toml`이 TOML v1.1.0（2025-12-18 공개）에 맞춰
  파싱합니다. 다음 두 가지 추가: **(A1)** 인라인 테이블의 줄바꾸기 및 후행 슜표
  허용, **(A2)** 기본 문자열에서 `\xHH` 바이트 특별문자（0x00..=0xFF）, **(A3)** `\e` =
  U+001B, **(A4)** time / date-time 항목에서 초 생략（`t = 14:15` /
  `dt = 2010-02-03 14:15`）. 엄격 1.0.0 사용자를 위해 `TomlDialect::V1_0` 과
  `from_toml_v1_0` 입력 경로를 별도 두멍니다. 1.0.0 문서는 두 방언 모두
  동일한 결과를 냅니다. 동시에 space 구분 date-time 감지 인덱스의
  off-by-one 버그（`T` 가 없는 date-time이 1.0 모드에서도 인식되지 않았는）를
  함꺼 수정했습니다.
- **JSON dual-slot 주석 보전** — JSONC 파서가 독립 행 `// ...` 주석을
  #114 가 추가한 `leading_comment` 슬롯에 기록하고, 같은 행의
  `// trailing` 은 `comment` 에 남깁니다. 개체 멤버와 배열 요소가 같은
  노드에서 두 주석을 모두보존할 수 있게 되며, 이는 #112 의 단일
  슬롯 모델로는 표현할 수 없는 형태입니다. `to_jsonc_text_pretty` 는
  `leading_comment` 를 우선 읽고 hand-built fixture 를 위해 `comment`
  (`standalone = true`) fallback 을 보존합니다. 엄격 JSON
  (`to_json_text`) 의 동작은 변하지 않습니다 (여전히 `//` 미출력).
- **TOML 빈 행 + dual-slot 주석 보전** — `NodeMeta`에
  `leading_comment: Option<Comment>` 와 `blank_before: bool` 이
  추가되었습니다 (둘 다 구조적 `Hash` / `PartialEq` 에서 제외).
  섹션 헤더나 AOT 요소가 `]` 뒤의 행미 주석과 그 위의 독립 행 주석을
  동시에 보존할 수 있어 서로 밀어내지 않습니다. `to_toml` 은 소스의 빈
  행 구역을 재현하며 (`a = 1\n\nb = 2` 가 바이트 안정적으로 왕복),
  문서 첫 페어 앞에는 빈 행을 쓰지 않습니다. 수작업 빌드 및 YAML 출처
  노드는 writer 의 fallback 읽기로 계속 동일하게 렌더링됩니다.
- **JSON5 방언** — `pyrs_yaml_core::json::from_json5(text)`과
  `from_json_with_options(text, JsonParseOptions)`가 JSON5의 4개 축
  (후행 쉼표, 한따옴표 문자열, 따옴표 없는 식별자 키, 행/블록
  주석)을 모두 허용합니다. 각 축은 개별 토글 가능; `STRICT`,
  `JSONC`, `JSON5` 상수를 기본값으로 제공합니다.
- **JSONC/JSON5 바인딩과 CLI** — `pyrs_yaml.from_jsonc(str)`은 YAML
  텍스트를 반환하고, `pyrs_yaml.load_jsonc(str)`는 Python dict / list를
  바로 반환합니다. `pyq from-json`에 `--jsonc` 및 `--json5` 플래그를
  추가해 `tsconfig.json` / `settings.json` 이 verb 파이프라인으로 바로
  이어집니다.
- **JSONC 주석 보존** — `from_jsonc`이 수집한 `// 행` 및
  `/* 블록 */` 주석을 AST의 `NodeMeta::comment`에 부착합니다
  (독립 부분은 key 노드, 행미 부분은 value 노드). PR #109에서
  도입한 TOML 모델과 동일합니다. 쌍을 이루는
  `to_jsonc_text(node)` / `to_jsonc_text_pretty(node, indent)`
  가 원 위치로 재출력하며, 블록 주석은 AST에 본문만 저장되므로
  출력 시 `//`로 정규화됩니다. 엄밀 writer `to_json_text` /
  `to_json_text_pretty`는 바이트 단위로 불변이므로 소비측은保전
  여부는 선택할 수 있습니다.
- **pyq 다중 문서 편집** — `-A/--all-docs`가 모든 편집 명령(set/delete/rename/
  move/append/insert/sort-keys)과 `to-json -A`(JSON 배열, Python 대응)를 커버.
  각 문서는 스트림 내 자기 구간에 대해 splice(`MultiDocEditor` + `DirtyUnit::
  shifted`): 건드리지 않은 문서와 모든 `---` 구분자는 바이트 단위로 유지,
  경로 미매칭 문서는 스킵(Python try/skip 의미 일치, 전원 미매칭은 오류),
  레이아웃 오염 문서는 단독 폴백으로 이웃에 영향 없음.
- **JSONC 파싱** — `pyrs_yaml_core::json::from_jsonc(text)`와
  `from_json_with_options(text, JsonParseOptions)`가 공백이 허용되는 아무
  위치에서나 `// 행` 과 `/* 블록 */` 주석을 허용합니다
  (TypeScript `tsconfig.json`, VS Code `settings.json` 방언). 주석은
  제거되며 보존되지 않습니다. 후행 쉼표와 JSON5 고유 구문은
  계속 거부되므로 허용 언어는 RFC 8259의 엄격한 상위 집합으로
  남습니다. `from_json`의 기본 동작(엄격 모드)은 바뀌지 않습니다.
- **TOML 주석 보존** — 파서가 페어나 section 헤더 위에 별도 행으로
  나타나는 `# ...` 주석과 행미 주석 (`key = value # ...` /
  `[name] # ...`) 을 모두 캡처하여 공유 AST의 `NodeMeta::comment`에
  부착합니다 (독립 부분은 key 노드, 행미 부분은 value 노드).
  `to_toml(from_toml(src))`가 원 위치로 재출력하므로 `pyq edit`과
  `YamlDocument.set()`은 TOML 왕복 중 주석을 더 이상 제거하지 않습니다.
  빈 행 구분은 설계 문서에 따라 writer 기본 레이아웃 그대로.
- **TOML 숫자 원문 표기 보존** — `to_toml(from_toml(src))`가 16진
  (`0xDEADBEEF`)·8진 (`0o755`) 정수의 원문 표기와 지수형 부동소수점
  (`1e10`、`-3.14e-2`)을 그대로 유지합니다. 밑줄 구분자、명시적 `+` 부호、
  음수 radix 형식 (`-0x1F`)、2진수 (`0b101`)는 YAML Core가 다시 읽을 수 없어
  10진수로 정규화됩니다. 이로써 공유 AST와 YAML 파이프라인의 상호운용성을
  유지합니다。주석 보존과 JSONC 지원은 설계 문서에 따라 후속 PR에서 도입 예정。
- **pyq 기능 보완** — CLI가 Python CLI 기능면에 대응: `rename`/`move`/`append`/`insert`
  splice 편집, `validate`(파싱 검사 또는 `--schema rules.yaml` 스키마 언어 규칙 검증),
  `frontmatter`(`--body-out` 본문 분리), `get`/`fmt`/`to-json`의 `-A/--all-docs` 다중
  문서 지원. 배선 중 코어 엔진 버그 발견: `move_path`가 이동 대상 INSERT 단위만
  반환해 splice 텍스트에 이동 원본 사본이 잔류(폴백 시 눈에 안 탐). 이제 두 단위를
  반환하고 bindings는 배치 splice 경로로 적용.
- **`pyq` 필터 동사** — 매칭 스트림의 jq 스타일 구조화 후처리:
  `--select 'PATH OP LITERAL'`, `--sort-by PATH` / `--desc`, `--unique`,
  `--first` / `--last`, `--skip N` / `--take N`, `--join SEP`.
  `get`과 `from-*`에서 고정 파이프라인 `select -> sort -> unique -> slice`
  → `join` 적용. 의도적으로 플래그 설계(표현식 없음): 술어는 미세 구문
  1회 파싱(약 40줄), 타입 불일치는 false(jq 전순서와의 알려진 차이), 즉시 시작 유지.
- **`pyq completion`** — bash·zsh·fish·PowerShell 셸 자동 완성 스크립트 출력
  (`pyq completion bash > ...`). `clap_complete` 구현(승인된 CLI 크레이트
  의존성 추가. `pyrs-yaml-cli` 바이너리 내에 완결되며 Python 배포에 영향 없음).
- **`pyq sort-keys`** — 임의 경로(`$`는 루트)의 매핑 키를 정렬. `set`/`delete`와
  동일한 코어 plan/splice 엔진을 거쳐 그 자리 다시 쓰기 또는 표준 출력 모두
  지원하며, Python CLI의 `sort-keys`와의 대응 격차를 해소.
- **커맨드라인 인터페이스** — 새로운 `pyrs-yaml` 명령(`pip install "pyrs-yaml[cli]"`로
  옵트인, Python 3.10+ 필요)을 통해 라이브러리의 핵심 기능을 터미널에서 사용할 수
  있습니다: `fmt`(주석/앵커/순서를 보존하는 라운드트립 재포매팅), `get`(JSONPath 쿼리,
  `--format yaml|json|text` 지원), `set` / `delete` / `rename`(경로 기반 편집,
  `--inplace`, `--string`, `--create-missing` 지원), `validate`(CI 친화적 종료 코드), 그리고 `to-json` / `from-json` 변환. 모든 명령은
  `-`로 stdin을 읽고 기본적으로 stdout에 출력합니다. 구현은 순수 Python
  (`python/pyrs_yaml/cli/`)이며 [Cyclopts](https://github.com/BrianPugh/cyclopts)를
  선택적 extra로 사용하므로, 기본 설치는 추가 의존성 없음과 Python 3.8 지원을 유지합니다.
- **CLI 확장** — `sort-keys`(경로 위치의 매핑 키 정렬), `move`(서브트리를 기존 대상으로
  이동), `frontmatter`(Markdown 프론트매터를 YAML로 추출, 본문 분리 지원),
  `compliance`(YAML Test Suite 리포트, `--json` 지원) 명령이 추가되었습니다.
  `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/`validate`/`to-json`에
  `-A/--all-docs` 다중 문서 모드가 제공되고, `validate`는 상호 배타적인
  `--schema <이름>`과 `--schema-file <경로>`로 분리되었습니다. 문서화되지 않았던
  `python -m pyrs_yaml.compliance` 진입점은 서브커맨드로 대체되어 제거되었습니다.
- **`YamlStream` import 가능** — API 문서와 타입 스텁대로
  `from pyrs_yaml import YamlStream`이 동작합니다. 그동안 이 클래스는
  `YAML().load_stream*()`의 반환값으로만 얻을 수 있고 네이티브 모듈에서 내보내지
  않았습니다.
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
  (`python/pyrs_yaml/settings.py`)는 `pydantic_settings.YamlConfigSettingsSource`의
  드롭인 대체품으로, PyYAML 대신 pyrs-yaml(YAML 1.2 코어 스키마)로 파싱합니다.
  지연 내보내기되므로 `import pyrs_yaml`에 pydantic-settings가 필요 없습니다.
  `pip install "pyrs-yaml[settings]"`로 설치합니다(Python 3.10+).
  `dump_pydantic`과 `parse_as`도 동일한 모듈 수준 `__getattr__` 지연 내보내기 패턴으로 변경되었습니다.
- **`pyq` — Rust 네이티브 CLI 크레이트** — `crates/pyrs-yaml-cli`
  (워크스페이스 멤버, clap 기반)는 `pyrs-yaml-core`를 jq/yq 스타일
  CLI에 직접 연결하여 런타임 Python이 필요 없습니다: `fmt`(주석 보존
  라운드트립), `get <path>`(JSONPath-lite, `--json`/`--raw` 지원),
  `set <path> <value>` 및 `delete <path>`(yq 스타일 편집, `--create-missing`
  과 `-i/--inplace` 파일 쓰기 지원; 출력은 라운드트립 직렬화기 경유로
  주석과 값 스타일 보존), `to-json`(순서 보존), `to-toml`, 도입 명령 `from-json` / `from-toml` /
  `from-ini`. 입력 형식은 확장자로 판정(`--input`으로 덮어씀), stdin은
  `-`, 실패 시 core의 안정적인 오류 텍스트로 비정상 종료.
- **TOML 및 INI 교환 형식** — YAML을 유일한 편집 가능 표현으로 하는 허브-스포크
  멀티 형식 지원: `from_toml`/`to_toml`로 TOML 텍스트 ⇄ YAML 텍스트 변환
  (Rust `toml_edit`), `load_toml`은 TOML을 Python 값으로 직접 읽기
  (datetime은 내장 `!timestamp` 플러그인 경유, TOML 문자열은 재해석되지 않음),
  `load_ini`는 표준 라이브러리 configparser로 INI 읽기(엄격 모드·읽기 전용).
  TOML 출력은 표현 불가 구조를 안정적인 오류로 거부하며, 라운드트립 편집은
  YAML 전용으로 유지됩니다.

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

- **`\u` / `\x` 이스케이프 뒤 多字节 字符에 parser panic** — 고정폭 이스케이프 리더가 `&self.text[pos..pos+width]`를 字节 오프셋으로 슬라이스. JSON `\u` / TOML `\xHH`/`\uXXXX`/`\UXXXX` 뒤에 多字节 字符가 오면 슬라이스가 字符 中间에 落하 여 프로세스 abort (#153 同族). 字节 슬라이스 + UTF-8 校验로 修改, 不正 이스케이프는 干净히 에러. 方言 fuzz 로 發見, 두 parser 에 決定性 Rust 回归 테스트로 固定.
- **비(非)머지 값의 리터럴 `<<` 키가 조용히 폐기됨** — `load(safe_dump({"<<": None}))` 이 키를 잃고 `{}` 반환. 머지 리졸버가 Null/Scalar 값에도 모든 `<<` 를 머지로 소비. YAML 에선 `<<` 값이 mapping 별자/inline mapping/그 시퀀스일 때만 머지. Null/스칼라 `<<` 는 일반 키로 왕복 보존. Alias/mapping/sequence 경로(#166 자기참조 앵커 가드 포함) 무변경, yaml-test-suite 405/406 유지. 왕복 속성 fuzz 가 비결정 포착(#163/#165/#166 급 결함), 결정적 Rust 회귀 테스트로 고정.
- **TOML 深네스팅이 네이티브 스택을 넘어 프로세스를 abort** — TOML 파서엔 중첩 예산이 없어(JSON 은 `DEFAULT_MAX_DEPTH`, YAML 은 `parse` `max_depth`), `parse_value` → `parse_array`/`parse_inline_table` 이 무제한 재귀. 深배열/인라인 테이블은 인터프리터를 즉시 충돌시킴(검증: exit `0xC00000FD` STACK_OVERFLOW) — #166 YAML merge 오버플로의 TOML 판. 파서가 `depth` 추적해 1000 초과시 `ParseError::MaxDepthExceeded` 반환, JSON 과 대칭. in-process Python 경계 테스트 + subprocess crash canary + 大스택 Rust 테스트로 보호.
- **방언 writer/parser가 문서 수준 주석을 유실·오배치** — 고정점 속성이 잡아낸 3개 결함: (a) JSONC/JSON5 값 앞 파일 선행 `// note`가 inline으로 오분류(오프셋 0 앞 개행 없음)되고 writer가 주석 달는 root가 아닌 첫 object 멤버에 선점돼, 빈 `{}`/root 스칼라에서는 완전히 소실; (b) JSON 계열 및 TOML writer는 주석 본문를 그대로 출력하나 parser는 trim 저장—미trim 주석은 pass마다 후미 공백이 요동—writer도 출력 시 trim해 첫 표기부터 안정; (c) 주석만 있는 TOML 문서(`# note` 뒤 키 없음)는 재파싱 시 주석이 유실돼 빈 root가 `""`화—미소비 standalone 주석을 빈 root 테이블에 부착. 5개 포맷의 leading 주석이 모두 문자 안정 고정점에 도달(3개 Rust 테스트로 고정).
- **중첩 블록 스칼라 본문이 부모 들여쓰기를 유지** — 중첩 키 아래 literal/folded 스칼라의 본문 줄이 `b: |` 헤더 줄의 다음 단계가 아니라 0열부터 고정 1단계로 출력되어, 모든 중첩 형상의 왕복 텍스트가 재파싱 불가하거나 오류 값이 됨. `block_base`(부모 줄 열 위치) 매개변수를 모든 출력 지점에 관철; 7종 중첩 형식의 왕복이 완전 일치. TOML 핫스팟 벤치가 발견.
- **직렬화기가 재파싱 가능한 YAML만 출력** — 텍스트 수준 게이트가 발견한 5개 결함: (a) 개행 분기에서 anchor/tag pre-emit이 child(스칼라/null/flow 용기)의 자체 헤더와 중복 → block 용기로 제한; (b) flow 용기 내 블록 스칼라(`[|`, `{k: >}`) 및 키 위치는 double-quoted로 강등; (c) own line을 시작하는 flow 용기의 행두 들여쓰기 결락 및 complex key(`?`) 값 표지 `:`의 0열 출력 → 부모 인덴트 따르도록 수정; (d) standalone 주석/tag가 있는 complex key의 모호한 텍스트(주석 `?` 상단으로, 본문 항상 1단계 깊은 별도 줄); (e) flow 용기 내 선두/후미 공백 또는 `,[]{}` 포함 plain 스칼라는 인용(미인용시 토큰 끊어짐). 추가: tag 부속 빈 block 용기는 헤더를 `{}`/`[]`와 같은 줄에, compact dash 항목은 standalone 주석 부속 값을 인라인하지 않음. 9개 targeted Rust 테스트와 Python 회귀로 각 계통 고정.
- **TOML 이 합법적인 최소 i64 정수를 거부** — `from_toml`/`load_toml` 이
  `-9223372036854775808`(`i64::MIN`)에서 실패: 부호 경로가 절대값을 먼저 해석해
  부호 반전 전에 오버플로했습니다. 이제 부호를 자릿수와 함께 해석하고
  (`i64::from_str` 는 음의 방향으로 누적), 부호 있는 실수는 지수 표기를 유지하며
  기존 negate 경로는 제거되었습니다. 새 Python 측 Hypothesis 방언 퍼징
  (`tests/test_property_dialects.py`, stdlib `json`/`tomllib`/`pyjson5` 오라클로
  타입 엄격 비교) 에서 발견; JSON5 베어 `Infinity`/`NaN` 리터럴과 i64 초과 숫자
  문자열이라는 AST 모호 표기 2종도 고정했습니다. Rust 회귀:
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`.
- **잘못 들여쓰기된 플로우 시퀀스 지속 행이 다시 거부됨** — YAML 파서를
  granit-parser 1.3으로 업그레이드하면(*변경* 참조) 지속 행의 들여쓰기가 포함
  블록 키보다 깊지 않은 다중 행 플로우 콜렉션(yaml-test-suite `9C9N`: `flow: [a,`
  뒤 열 0의 `b,`)을 조용히 *수락*하기 시작해 엄격성이 `405/406 → 404/406`로
  후퇴했다 — suite의 ≥95% 임계값 게이트에는 보이지 않아 CI를 통과했다. AST
  receiver의 파싱 후 in-tree 가드가 포함 블록 들여쓰기를 추적해 들여쓰기가 부족한
  플로우 지속 행을 거부하여 `405/406`을 회복한다. 가드는 파서가 이미 계산한
  span만 사용하므로 올바르게 들여쓰기된 다중 행 플로우는 영향받지 않는다. `9C9N`은
  이제 케이스별 하드 게이트(입력 리터럴, `skipif` 없음)로 `tests/test_yaml_suite.py`
  에 고정되었고, Rust 단위 테스트
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
- **NumPy 직렬화가 GIL 없이 Python 메모리를 읽지 않음** — ndarray writer가
  `unsafe { as_slice() }`로 배열 데이터 버퍼를 빌려 그 슬라이스를 `py.detach`
  **내부**(즉 GIL 해제 후)에 순회했다. 출처와 무관하게 `&[T]`는 `Send`이므로
  borrow checker가 잡을 수 없지만, 이 메모리는 Python 소유이므로 다른 스레드가
  동시에 resize하거나 쓸 수 있다:健全하지 않은 데이터 경쟁 / UB이며 동시성
  환경에서만 드러난다. 이제는 **GIL을 잡은 채** 버퍼를 Rust 소유 메모리로
  스냅샷(`slice.to_vec()`)하고, 스칼라→노드 변환만 스레드 밖에서 수행한다.
  이는 bindings 계층의 **유일한** `unsafe` 버퍼 빌림이며, 나머지 모든
  `py.detach` 사이트는 Rust 소유 상태(AST, 소스 텍스트, `BufWriter<File>`)만
  건드림을 확인했다. 회귀 커버리지는 `tests/test_numpy.py::TestNumpyConcurrency`.
  [@bourumir-wyngs](https://github.com/bourumir-wyngs)의 #165 보고.
- **반복된 별칭 참조가 `None`으로 해석되지 않음** — `to_dict()`가 **전역** visited
  앵커 집합으로 별칭을 확장하면서 이를 한 번도 지우지 않아, 각 앵커의 **첫
  참조만** 값을 만들고 이후의 참조는 조용히 `None`으로 타락했다:

    ```yaml
    a: &x 1
    b: *x      # 1
    c: *x      # 이전에는 None, 현재는 1
    ```

    영향 범위는 "두 번째 참조"보다 넓었다. 같은 컨테이너 안의 형제 참조끼리도
    서로 오염했다(`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}`에서 `b`는 값이었고
    `c`는 `None`). 이제 이 guard는 현재 재귀 경로로 한정되어, 한 번의 확장
    동안에만 push되고 이후 pop된다. 따라서 반복 참조와 형제 참조는 각각 완전히
    구성된 값을 얻고, 진짜 순환은 여전히 종료한다. `<<` 머지 해석과 AST 자체는
    영향을 받지 않음을 확인했다. `tests/test_direct_load.py`에 PyYAML과의
    일치 여부를 고정하는 6개 케이스를 추가했으며, 잘못된 출력을 기대값으로
    고정하던 테스트 2개를 다시 작성했다.
    [@bourumir-wyngs](https://github.com/bourumir-wyngs)의 #163 보고.
- **첫 키 값이 중첩 컨테이너일 때 문서 헤더 주석 소실 수정** — 파서가가 진행 중인
  모든 컨테이너에 단일 주석 슬롯을 공유했고, 중첩 컨테이너 start가 아직 자리 잡지
  못한 standalone 헤더를 지웠다(parse 단계에서 폐기, `to_dict`엔 보이지 않고 `dump`에서
  치명적). 이제 컨테이너별 슬롯 스택으로 관리.
- **splice 편집 시 선행 주석 중복 수정** — 재생성된 리전 텍스트가 pair/item 자신의
  standalone 주석을 담을 때 교체 범위가 기존 주석 행을 덮지 않아 두 개가 공존했으나,
  plan이 주석 행까지 범위를 확장(`pyq set`/`delete`와 bindings splice 경로 공통 수정).
- **`!timestamp`가 지원되는 모든 Python에서 끝의 `Z` 허용** —
  `datetime.fromisoformat`은 UTC-`Z` 접미사를 3.11부터만 인식하므로
  플러그인에서 `...Z`를 `+00:00`으로 정규화, 3.8~3.10의 YAML
  `!timestamp` 스칼라 및 `load_toml` / `from_toml` 경로의 TOML datetime에서
  `Invalid isoformat string` 오류를 해소.

#### 성능

- **이벤트 스트림→Python 객체 직접 구성** — `safe_load`, `safe_loads`,
  `YAML().safe_load*`는 완전한 AST를 구성한 뒤 `convert.rs`에서 다시
  순회하는 대신 granit 이벤트 스트림을 한 번 순회해 Python 객체를
  구성한다. 스키마 해석·원문 매핑 키·중복 키 오류 의미론은 완전히
  동일하다. 앵커/tag/merge/멀티 문서 입력은 제로 비용 사전 제외로
  AST 경로에 폴백. WSL 실측: 스칼라 위주 `safe_load` −21~25%,
  패밀리 전체 −13~18%, 폴백 형태는 불변.
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

### [v0.15.0] — 2026-08-19

#### 추가

- **노드 메타데이터 세터/게터** — `Node.comment` / `Node.anchor` / `Node.tag` 읽기 속성과 `set_comment` / `set_anchor` / `set_tag`(및 `remove_*` 계열)를 추가. 별칭 또는 존재하지 않는 경로 편집은 오류가 발생합니다. 인라인 스칼라 값·시퀀스 항목의 독립형 주석은 자체 들여쓰기 행에 출력됩니다(`child:\n  # c\n  val` 및 `- a\n# c\n- b`의 기존 라운드트립 결함 수정).
- **Verbatim 태그** — `set_tag("!<tag:yaml.org,2002:str>")`는 verbatim 태그(빈 핸들)를 생성하며, 소스에서 파싱된 verbatim 태그는 라운드트립 시 보존됩니다: `Tag`의 `Display`는 빈 핸들 태그를 `!<...>`로 감싸 출력하고, `parse_tag`는 `!<...>` 형식을 인식하며, 스트림 이벤트는 `Display`를 통해 태그를 직렬화합니다.
- **스키마 파일 IO 및 목록** — `load_schema(name, path)`는 파일에서 스키마 정의를 읽어 등록하고, `list_schemas()`는 등록된 모든 스키마 이름(내장 `failsafe`/`json`/`core`/`yaml1.1` + 사용자 정의)을 반환합니다.
- **노드 style/format 세터/게터** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping` 읽기 속성과 `set_scalar_style` / `set_flow_style` / `set_chomping` 메서드. ScalarStyle/Chomping이 이제 `Copy`를 derive합니다. 비스칼라 노드는 `None` 반환 / no-op, 별칭 및 존재하지 않는 경로는 오류가 발생합니다.
- **스키마 구조 검증** — 스키마 정의의 `validate` 섹션으로 구조 검사(경로 한정 스칼라 타입, `sequence_of`/`mapping_of` 컨테이너, `required`)를 추가. `validate_against_schema(data, schema_yaml)`는 모든 실패를 나열하며 `YamlValidateError`를 발생시킵니다.
- **`Node.copy()`** — 하위 트리를 문서에서 분리된 독립 Python 값(dict/list/scalar)으로 깊은 복사합니다. `set_value()`로 붙여넣는 데 유용합니다.
- **고급 편집 API** — `doc.set_many({path: value})`로 여러 경로(와일드카드 `[*]` 및 딥 스캔 `..` 지원)를 단일 스플라이스 버스트로 설정. `doc.sort_keys()`로 매핑 키를 제자리에서 정렬. `Node.move(new_path)`로 하위 트리 이동. `Node.path` / `Node.find_first()` / `Node.value_eq()`로 경로 접근·첫 와일드카드 검색·값 비교 추가.
- **0.14+ 기능 프로퍼티 테스트** — `validate_node` / 스키마 파싱 / style round-trip Rust proptest, `set_many` 와일드카드 / metadata 편집 / `sort_keys` Python hypothesis 테스트 추가. `hypothesis`를 `test` 그룹으로 이동해 CI에서 프로퍼티 테스트 실행.
- **시리얼라이저 수정** — 빈 흐름 컨테이너(`key: {}` / `key: []`)의 독립형 주석이 무효 YAML을 생성하던 문제 수정(인라인으로 강등).

#### 변경

- **프리-스레디드 (cp314t) 휠에서 NumPy 재활성화** — cp314t 빌드 인수에서 `--no-default-features` 제거. rust-numpy 0.29는 프리-스레디드 Python을 지원하며, `numpy.ndarray` 직렬화가 프리-스레디드 휠에서 사용 가능합니다(NumPy 설치는 런타임에 자동 감지).

#### 문서

- **모든 언어(en/zh/ja/ko) 문서의 오래된 참조 수정** — `saphyr-parser` → `granit-parser`, YAML 준수율 98.1% → 99.75%(스위트 405/406 케이스), ABI3 지원 3.9–3.13 → 3.8–3.15(py3.9+ → py3.8+), 벤치마크 표를 현재 CodSpeed CI 수치(파싱 21–43배, 직렬화 55–177배 PyYAML 대비 빠름)로 업데이트. Rust 측 벤치마크 섹션을 Criterion에서 divan으로 마이그레이션(`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`).

### [v0.14.1] — 2026-08-15

#### 수정

- **백슬래시+제어 문자/비문자를 포함한 단일 인용 스칼라** — 이러한 값은 이중 인용을 사용합니다. 단일 인용은 제어 문자/비문자를 이스케이프할 수 없습니다.
- **비문자 및 BOM 인용** — `needs_quotes` / `needs_double_quoted`가 U+FFFE/U+FFFF/평면 끝 비문자와 U+FEFF(BOM)를 인용 필수로 취급합니다.
- **이중 인용 이스케이프 폭** — U+FFFF 이상의 코드 포인트는 8자리 `\Uxxxxxxxx` 형식으로 출력합니다(4자리 `\u`는 BMP 전용).
- **접힌 plain 스칼라 연속 들여쓰기** — 연속 들여쓰기를 값의 시작 열에서 도출하여 중첩된 시퀀스/매핑 항목의 연속 행이 부모 블록 들여쓰기를 초과하도록 했습니다.
- **멀티바이트 접기 경계** — `wrap_plain_scalar`가 접기 슬라이스를 문자 경계로 내림하여 4바이트 UTF-8이 경계를 가로지를 때 panic을 방지합니다.
- **publish 테스트 요구사항에 `hypothesis`** — `.ci/requirements-test.txt`에 `hypothesis>=6.113.0`을 고정하여 게시 워크플로가 속성 테스트를 실행할 수 있게 했습니다.

#### 추가됨

- **`scripts/fuzz_panics.py`** — dump/parse/edit/멱등성에 걸친 적대적 전략을 사용한 로컬 대규모 Hypothesis fuzz 하네스.

### [v0.14.0] — 2026-08-14

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

- **따옴표 스칼라는 항상 문자열로 로드** — 암시적 타입 해석은 평문 스칼라에만 적용(YAML 1.2). `safe_load('"true"')`는 문자열 `"true"`를 반환합니다(`True` 아님). 직렬화기는 문서(`to_yaml`) 경로에서도 음수가 올바르게 왕복되도록 유지합니다.
- **홑/겹따옴표 단일 문자 키 왕복** — `'` 또는 `"` 단일 문자 맵 키는 인용 스칼라로 출력되어 파싱 불가 YAML이 되지 않습니다.
- **빈 컬렉션은 `{}`/`[]` 출력** — 빈 매핑/시퀀스 덤프가 재파싱 시 `None`이 되는 빈 문서를 생성하지 않습니다.

#### 변경

- **`get()`은 리터럴 키 전용** — `YamlDocument.get()`은 `.`/`[` 포함 키를 JSONPath로 추정하지 않으며, 항상 최상위 맵 키로 취급합니다(`__getitem__`/`__setitem__`과 일관). 경로 접근은 `find()`/`node()`를 사용하세요.

### [v0.13.0] — 2026-08-10

#### 변경 사항

- **Rust MSRV를 1.96으로 업그레이드하고 edition을 2024로 변경** — 두 crate 모두
  `rust-version = "1.96"` 및 `edition = "2024"`를 선언합니다. CI는
  `build`/`test-freethreaded` 작업을 Rust 1.96으로 고정하여 결정론적인
  wheel 빌드를 보장합니다. 또한 `msrv-check` 작업을 추가하여 MSRV에서
  `cargo check`/`cargo test`를 실행하고 정적 MSRV 드리프트를 방지합니다
  (`rust-lint` 작업은 `stable` 유지). 버전 바트는 PyO3 0.29 자체의 기반
  (rustc 1.83)보다 높게 설정되며, std API 선제 지원(예: `assert_matches!`,
  1.96 안정화)을 목적으로 합니다. `TAG_REGISTRY`(태그 핸들러 관리)가
  `std::sync::LazyLock`로 리팩터링되어 `Mutex<Option<...>>` 간접 계층이
  제거되었습니다.

#### 성능

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — Python→YAML serialization without intermediate `CustomNode` AST.
  Single-pass `direct_dump` replaces the old two-pass `pyobject_to_node` +
  `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on `from_dict`
  (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — when input has no `&` characters, skip `collect_anchors` + anchor
  resolution and use the simpler `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/
  non-boolean first bytes return `Str` immediately, avoiding schema
  resolution overhead for the common case. (#59)
- **granit-parser 마이그레이션** — saphyr-parser를 granit-parser 1.0.1로
  교체하여 네이티브 `Event::Comment` 출력으로 전체 텍스트 `scan_yaml()`
  프리스캔을 제거. parse_small -18%, parse_large -21%,
  roundtrip_large -18%.

#### 수정

- **`float_to_yaml_string` round-trip 수정** — Rust Display가 소수점을
  버릴 때 `.0`을 추가(`42` → `42.0`)하여 float가 int로 바뀌지 않고
  round-trip되도록 함.
- **`count_nodes` 사전 할당 롤백** — 전체 AST 순회 비용이 피한 realloc보다
  커서(serialize_10mb 약 14% 저하) 버퍼 확장은 Vec에 위임.

#### 추가

- **스트림 & frontmatter API에 `max_depth` 추가** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)`
  가 `max_depth`를 허용 (기본값 1000). 스트림 파싱은 이제 코어
  `parse_stream_with_options`를 통해 중첩 깊이 제한을 적용
  (기존에는 스트림 이벤트에 깊이 제한이 없었음).
- **Pydantic 통합** — `dump_pydantic()`은 Pydantic 모델을 YAML 문자열로
  직렬화 (`model_dump(mode='json')` + `safe_dump`); `parse_as()`는
  YAML 문자열을 Pydantic 모델 인스턴스로 파싱. 둘 다 지연 임포트,
  pydantic에 대한 하드 의존성 없음. (#61)

#### 내부

- **Split `py/mod.rs`** — monolithic 1786-line module broken into
  `document.rs` (YamlDocument), `yaml_instance.rs` (YAML class),
  `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` 가드 + `double_quoted_scalar()` 생성자** — `'true'` /
  `'42'` / `'null'` 같은 문자열은 코어 스키마 재파싱 시 오독되지 않도록
  큰따옴표 스칼라로 출력(`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed 벤치마크를 `codspeed-divan-compat`으로 통일** —
  `exclude-allocations`로 할당 노이즈 제거. 크로스 라이브러리 벤치마크를
  `tests/test_benchmark_crosslib.py`로 통합하고 공용 `tests/data/yaml_samples.py`
  픽스처와 스트리밍 커버리지 추가.

### [v0.12.1] — 2026-08-06

#### 추가

- **`set(create_missing=True)`** - missing intermediate mapping keys along
  the edit path are created as nested mappings (e.g. setting `a.b.c` on
  `a: 1` creates `b` and `c`); index segments that miss are still an error,
  and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal
  yielding `Node` objects, avoiding per-node `to_dict()` resolution.
  `walk()` returns all nodes; `scalars()` returns only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate`
  (key_eq, navigate, navigate_mut, normalize_index, mapping_key_index),
  `editing::region` (line helpers, node_is_flow, extend_delete_over_comments,
  nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null
  values, deeply nested, flow collections, mixed types.

#### 변경

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/`
  (pure Rust, no PyO3) and `crates/pyrs-yaml/` (PyO3 bindings). Root
  `Cargo.toml` is now a workspace. Old `src/` directory and `build.rs`
  removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to
  `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy
  duplicate-key detection, `shift_insert` merge prepending, and skipped
  `DocumentEnd` deep-clone for single-document parses cut large-document
  parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event
  text share allocations via `Arc<str>`; AST nodes shrink 8 bytes and
  clones become refcount bumps instead of deep copies.

#### 수정

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip
  previously dropped standalone comments attached to simple-key nodes;
  now preserved (two regression tests).

### [0.11.7] - 2026-08-04

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

### [0.11.6] - 2026-08-04

#### 변경

- **Free-threaded (cp314t) wheels are now numpy-free** - built with
  `--no-default-features`, so rust-numpy is excluded entirely (smaller
  binary, no runtime probe). `safe_dump` on a `numpy.ndarray` raises
  `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15)
  keep full ndarray serialization.

#### 추가

- **Free-threaded CI validation** - `test-freethreaded` job now builds
  and tests with `--no-default-features`, matching the shipped
  free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

### [0.11.5] - 2026-08-04

#### 변경

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 / yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and `tests/test_strictness_audit.py`.

#### 추가

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current rejection/acceptance behavior (both directions), so future parser changes cannot silently regress strictness or over-reject.

### [0.11.4] - 2026-08-04

#### 수정

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is one tab, fixing tab-encoded suite cases

#### 변경

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2 grammar, matches PyYAML/libyaml)

### [0.11.3] - 2026-08-03

#### 추가

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### 변경

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### 수정

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures before Release

### [0.11.2] - 2026-08-03

#### 추가

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: O(앵커 + 청크) 메모리의 지연 이벤트 반복자

#### 성능

- **파싱 시 스플라이스 자격 계산 안 함** — O(문서) 레이아웃 검사가 첫 편집 시 `YamlDocument.splice_checked`를 통해 지연 실행되어 v0.11.0 회귀 복원: parse_comments -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% 모두 v0.10.0 수준으로 복귀
- **선형 커서 레이아웃 검사** — 사전 계산된 줄 오프셋에 대한 노드별 이진 탐색 대체 (단조 소스 순서 순회)

#### 변경

- `parse_with_options`가 `CustomNode`를 반환 (기존 `(CustomNode, bool)`); 스플라이스 자격은 이제 `YamlDocument` 내부에 있으며 요청 시 계산

### [0.11.0] - 2026-08-02

#### 추가

- **Surgical Serialization** — 모든 AST 노드의 바이트 수준 소스 스팬 추적; 세그먼트 기반 스플라이스 — 편집은 접촉 영역만 재생성, 미접촉 텍스트는 바이트 복사
- 속성 테스트 (proptest, 새 개발 의존성)
- 10MB 편집-플러시 벤치마크 (divan)

#### 변경

- `flush_source`가 세그먼트 스플라이스 사용; 플로우 스타일 영역, 비기본 레이아웃 문서, 병합 키, CRLF/BOM 문서, materialize 후 (단일 버스트 모델) 에서 전체 직렬화로 폴백
- 스플라이스 편집이 `---`/`...`/지시자 마커 라인을 미변경 바이트로 보존 (전체 직렬화는 이전에 이를 제거 — 의도적인 동작 차이)

### [0.10.0] - 2026-08-01

#### 추가

- **제자리 편집** — 서식 메타데이터를 잃지 않고 파싱된 문서 편집:
    - 경로 API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`, `doc.delete(path)`, `doc.rename(path, new_key)`, JSONPath 스타일 경로(`$.a.b[0]`); 루트 슈가 `doc["key"] = value`와 `del doc["key"]`
    - 노드 API: `doc.node()` / `doc.find(path)`는 `Node` 객체를 반환하며 `set_value` / `append` / `insert` / `delete` / `rename`과 트리 탐색(`parent`, `children`, `walk`, `filter`)을 지원
    - 완전한 메타데이터 보존 — 교체된 스칼라는 주석/앵커/태그/따옴표를 유지; 이름이 변경된 키는 위치와 주소를 유지; 삭제 시 매핑 순서 보존
    - 원자적 편집 — 실패한 작업은 문서 (리비전 포함) 를 변경하지 않음
    - 지연 소스 재동기화 — `source()` / `to_yaml()` / `reparse()`는 편집 성공 후에만 재직렬화
    - 오래된 노드 감지 — 문서 편집 후 `Node` 접근은 `YamlDocumentError` 발생 (`RuntimeWarning` 포함)
    - 새 예외: `YamlEditError`, `YamlPathError` (en/zh-CN/ja-JP/ko-KR i18n 지원)
    - 별칭 인식 편집 — 별칭 자신의 경로 설정은 그 자리를 교체; 별칭을 통한 편집은 `YamlEditError` 발생
- **편집 벤치마크** — `benches/yaml_bench.rs`에 divan 벤치마크 6개 추가 (소형~대형 문서의 set/insert/delete)

#### 변경

- `YamlDocument.source()`가 `str`을 반환하고 제자리 편집 후 지연 재직렬화

### [0.9.0] - 2026-08-01

#### 추가

- **Python 3.13, 3.14, 3.15 지원** — PyO3 `abi3-py38` 휠이 Python 3.8-3.15 커버 (GIL 빌드); `abi3t` + `abi3t-py315`는 free-threaded 안정 ABI 제공
- **Free-threaded CPython (GIL 없음) 지원** — `#[pymodule(gil_used = false)]`가 모듈을 free-threaded Python용 스레드 안전으로 선언; `Py_GIL_DISABLED` cfg 플래그로 numpy 게이트 (rust-numpy는 free-threaded 미지원 — `--no-default-features`로 free-threaded 빌드에서 numpy feature 비활성화)
- **CI free-threaded 작업** — 새 `test-freethreaded` 워크플로 작업이 Python 3.14t에서 컴파일과 테스트 검증
- **`pyo3-build-config` 빌드 의존성** — `build.rs`를 통해 `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]` 등 컴파일러 플래그 활성화
- **`numpy` 선택 사항화** — `numpy` feature 뒤에 게이트 (기본 활성화); `Py_GIL_DISABLED` 하에서 자동 제외
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`, `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs` 모두 이 플래그를 받습니다; 중복 매핑 키는 기본적으로 `YamlDuplicateKeyError`를 발생시키며, 허용 시 `last value wins`
- **`SerializeOptions` 확장** — `doc.to_yaml_with_options()`에 `width` (줄 바꿈, 0 = 끄기), `indent_mapping`, `indent_sequence`, `indent_offset` 추가 (기존 `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth`와 함께)
- **태그 핸들러 레지스트리** — `register_tag("!custom")` 데코레이터 및 명령형 형태 + `clear_tag_handlers()`; 등록된 태그를 가진 스칼라 노드는 핸들러를 통해 변환됨
- **우선순위를 가진 태그 핸들러 체이닝** — 여러 핸들러가 `priority` 오름차순으로 실행; `YamlTagSkip`은 핸들러가 다음으로 전달되도록 하고, fallback은 원래 값을 유지
- **Pydantic 통합** — `parse_as(Model, yaml, **yaml_kwargs)`는 YAML을 파싱하여 Pydantic v2 모델로 검증; pydantic이 없을 때 `ImportError` 발생
- **`.pyi` 타입 스텁** — maturin으로 자동 생성되어 커밋되므로 `register_tag`, `parse_as`, `to_yaml_with_options` 및 새 예외가 타입 체커에게 표시됨

#### 변경

- CI Python 매트릭스 확장: ubuntu, windows, macos에서 3.8-3.14
- 안정 ABI: `abi3-py39` → `abi3-py38` (더 넓은 Python 3.8+ 지원), `abi3t` + `abi3t-py315` 추가 (free-threaded 안정 ABI)
- `pyproject.toml` classifiers에 3.13, 3.14, 3.15 항목 추가
- **CI 최적화: 중복 Rust 컴파일 제거** — 단일 `rust-lint` 작업이 `cargo clippy` + `cargo test`를 한 번 실행; 빌드 작업은 OS별 abi3 휠을 하나 생성하고 테스트 작업이 `maturin develop` 대신 설치하여 21개 매트릭스 작업에서 Rust 컴파일을 제거 (~86% 감소); 모든 작업에 `Swatinem/rust-cache` 추가
- **pydantic 테스트 의존성** — `pydantic>=2.10.6`을 `[dependency-groups] test` 및 `.ci/requirements-test.txt`에 추가 (SSOT via `uv sync` in ci.yml)

#### 수정

- **Windows DLL 로딩** — `src/py/tag_registry.rs`에서 `#[cfg(test)]` 블록 제거로 Windows에서 `import pyrs_yaml`broken 문제 해결 (`250b8d0`)
- **Python 3.8 호환성** — `pydantic.py`에 `from __future__ import annotations` 추가 (`63d2495`)
- **CI pydantic 스킵** — `pytest.importorskip("pydantic")`로 pydantic 미설치 시 테스트 통과 (`7be011d`)
- **Windows의 CI glob 확장** — `pip install dist/*.whl`에 `shell: bash` 사용 (PowerShell은 `*` 확장 안 함) (`2f7778d`)
- **문자열이 아닌 태그 핸들러 반환이 `YamlTagError`를 발생시킴** — 비`str` 값을 반환하는 핸들러는 이제 `Tag handler '!x' must return a string` 오류 발생 (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` 인덴트 연결** — `indent_mapping`/`indent_sequence`/`indent_offset`가 직렬화器에 의해 이제 반영됨 (이전에는 dead 필드; 각각 생략 시 `indent_size`/0으로 기본값)
- **`width`가 작은 값에서 멈추지 않음** — `width < continuation indent`일 때 무한 루프 대신 나머지 텍스트를 래핑 없이 출력 (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — 태그 핸들러를 등록 해제하는 새 함수; `register_tag`/`clear_tag_handlers` 보완
- **`duplicate-key` 오류가 i18n 적용** — `YamlDuplicateKeyError` 메시지가 이제 모든 4개 locale을 통해 `format_i18n_error`를 통해 흐름

### [0.8.0] - 2026-07-30

#### 추가

- **`YAML()` 인스턴스 API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` 재사용 가능한 구성; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` 메서드
- **Python `Node` API** — `Node` 클래스: `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`, `children`, `root_type`, `value`로 AST 탐색; JSONPath 스타일 쿼리 언어 (`$.key.sub`, `$.arr[0]`, `$..deep`)
- **`doc.version` 메타데이터** — `YamlDocument.version()`이 YAML spec 버전 반환 (기본값 "1.2")
- **`MergedView`** — `doc.merged()`가 병합 키가 해석된 읽기 전용 dict-like 뷰 반환
- **라이프사이클 경고** — `Node.release()`로 노드 명시적 무효화; 오래된 접근은 `RuntimeWarning` + `YamlDocumentError` 발생

#### 변경

- `parse()` / `safe_load()`가 이제 구문 당용으로 `YAML().parse()` / `.safe_load()`에 위임
- `YamlDocument`가 이제 문서 메타데이터를 위해 `version` 필드 저장

### [0.7.1] - 2026-07-30

#### 추가

- **ryaml 벤치마크 비교** — `tests/test_benchmark.py`에 `ryaml` (Rust YAML 라이브러리) 에 대한 벤치마크 추가 (PyYAML 및 ruamel.yaml와 함께); `benchmark_compare.py` 기능 비교 보고서로 재작성
- **CI 준수 임계값 상향** — YAML Test Suite 준수 게이트가 `test_compliance_report()`에서 70%에서 75%로 증가; 유효 파싱율 게이트는 95% (`tests/test_yaml_suite.py:251`)
- **CI 의존성 통합** — 발행 워크플로와 로컬 개발 전반에 통일된 테스트 의존성 관리를 위해 `.ci/requirements-test.txt` 및 `.ci/requirements-test-lite.txt` 추가
- **벤치마크 현대화** —更快的 C 확장 기반 통계 벤치마킹을 위해 `pytest-benchmark`에서 `pytest-codspeed`로 마이그레이션; 모든 CI 작업은 이제 `-r .ci/requirements-test.txt` 사용
- **Rust 벤치마크 Divan으로 마이그레이션** — `codspeed-criterion-compat`를 `codspeed-divan-compat` v5.0.1로 교체; 16개 벤치마크가 Criterion 그룹에서 `#[divan::bench]` 속성으로 재작성 (`Cargo.toml`, `benches/yaml_bench.rs`)

#### 변경

- CI 벤치마크 작업은 교차 라이브러리 비교를 위해 `ryaml` 설치
- `benchmark_compare.py`는 이제 타이밍을 `pytest-benchmark`에 위임하고 기능 비교/보고 도구로 역할

### [0.7.0] - 2026-07-29

#### 추가

- **직렬화器 `max_depth` 가드** — `serialize_node_internal`이 이제 재귀 깊이 추적하고 제한 초과 시 `YamlMaxDepthError` 발생 (기본값 1000), 파서 보호 일치 (`src/serializer.rs:135-145`)
- **직렬화器 핫패쓰 최적화** — 블록 스타일 직렬화를 대상으로 한 5개 최적화로 ~4.9% 루트립 속도 향상:
    - `write_anchor_tag` 및 `write_inline_comment` None 체크 인라이닝 (~99% 노드에 대한 메서드 호출 제거)
    - `write_indent` hot/cold path 분할 (캐시된 레벨 ≤64에 대한 직접 인덱싱)
    - 짧은 ASCII 영숫자 문자열 (≤8자) 에 대한 `write_plain_scalar` 고속 경로
    - Plain 스칼라에 대한 `write_scalar_for_key` 직접 디스패치 (디스패치 체인 방지)
- **pytest-benchmark 마이그레이션** — Python 벤치마크가 통계적 엄밀성, 구조화된 JSON 출력 및 CI 통합을 위해 원시 `time.perf_counter()`에서 `pytest-benchmark`로 마이그레이션 (`tests/test_benchmark.py` + 업데이트된 `tests/test_performance.py`)

#### 변경

- Python 벤치마크에서 원시 `timeit` 대신 `pytest-benchmark` 사용
- CI 벤치마크 작업은 이제 개별 스크립트 대신 `pytest --benchmark-json` 실행

#### 제거

- `write_inline_comment` 메서드 — 모든 호출 위치에서 인라이닝됨
- 직렬화기에서 `Comment` import — 이제 불필요

### [0.6.0] - 2026-07-27

#### 추가

- **비동기 직렬화** — `asyncio.run_in_executor`를 통한 `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`, `safe_load_async` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema 검증** — `YamlValidateError` 예외 + `YamlDocument.validate(schema)` 메서드 (`str` 또는 `dict` 수락); Python `jsonschema` 모듈에 위임
- **`YamlDocument.to_json()`** — 문서를 JSON 문자열로 직렬화 (Python `json.dumps` 사용)
- **증분 재파싱** — `YamlDocument`가 이제 소스 텍스트 저장 (`doc.source()`); `doc.reparse(resolve_merges=True, schema="core")` 로 제자리 재파싱
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

### [0.5.0] - 2026-07-27

#### 수정

- **`Serializer::write_node`** — `block_mapping`/`block_sequence`의 `values.iter().next().unwrap()`에서 `.unwrap()` 제거하고 안전 인덱스 접근으로 교체하여 edge-case AST에서 잠재적 패닉 제거
- **`YAML_SCHEMA` 상수** — 오타 `yamorg2002`를 `yamlorg2002`로 수정 (YAML 1.2 spec URL 일치)
- **개발 문서** — Python 명령어에 필수 `uv run` 접두사 및 Rust 명령어에 직접 `cargo` 추가하여 `AGENTS.md` 업데이트

### [0.4.0] - 2026-07-27

#### 추가

- **132개 새 gap-filling 테스트** — 이전에 테스트되지 않은 API에 대한 포괄적 커버리지
- **i18n 함수 테스트** — `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`
- **`parse_all_docs` 전용 테스트 스위트** — 단일 문서, 여러 문서, 빈 문서, 주석
- **`parse_file` 성공 사례 테스트** — 기본 파싱, 주석 보존, 파일 없음 오류
- **`to_yaml_with_options` 테스트** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys` 순서 보존
- **`to_dict()` 메서드 테스트** — 스칼라 루트, 중첩, 리스트, bool, null, anchor 해석, 빈 매핑/시퀀스
- **YamlDocument dunder 메서드 테스트** — `__repr__`, `__str__`, `__contains__`, `__len__`, `__iter__`, `__getitem__`, `root_type()`
- **바이트 입력 테스트** — `parse(b"key: value")`, UTF-8 바이트, 잘못된 UTF-8 오류
- **유니코드 & 특수 문자 테스트** — CJK, 이모지, 루트립, CRLF 줄 끝, 중복 키
- **`safe_load`/`safe_loads` 기능 커버리지** — anchor, merge key, block scalar, flow collection, 특수 부동소수, 타입 해석
- **`from_dict` edge cases** — 키의 특수 문자, 중첩 리스트, None 값, 빈 dict/list
- **`from_json` 루트립** — 중첩 구조, 배열, 잘못된 JSON 오류
- **`dump_file` 테스트** — 성공 경로, 잘못된 경로 오류
- **YAML Test Suite 개별 케이스 테스트** — 8진수, 16진수, 과학적 표기법, NaN, 무한대, merge key, 명시적/암시적 키, bool/null 변형, block scalar strip (`|-`), flow collection
- **`resolve_merges` 파라미터 테스트** — 비활성화 시 `<<` 보존, 기본값으로 해석
- **Flow collection 루트립** — 루트 레벨 및 중첩 flow 매핑/시퀀스
- **비스칼라 노드에 anchor** — 매핑 anchor (`&defaults`) 및 시퀀스 anchor (`&items`)
- **시퀀스 인덱싱 테스트** — 양수 인덱스, 범위 벗어남 오류
- **Merge key 통합** — 해석된 및 해석되지 않은 merge key로 루트립
- **태그 보존** — `!!seq` 및 `!!map` 태그 테스트 커버리지
- **주석 보존** — 복잡한 구조의 inline 및 standalone 주석 테스트

#### 변경

- 버전 동기화 수정: `python/pyrs_yaml/__init__.py` `__version__`을 0.2.0에서 0.4.0으로 업데이트하여 Cargo.toml/pyproject.toml과 일치
- `dist/`에서 구버전 0.2.0 wheel 아티팩트 제거

### [0.3.0] - 2026-07-27

#### 추가

- **NumPy ndarray 직렬화** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()`이 모든 차원 (0-D from N-D) 의 `numpy.ndarray` 지원
    - 지원 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - 다차원 배열은 올바른 들여쓰기로 중첩 YAML 리스트로 직렬화
    - 복소수는 `(re+imj)` 문자열 형식으로 직렬화
    - `0-D` 스칼라 배열은 1-D로 재변형되어 단일 항목 리스트로 직렬화
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate로 제로 복제 dtype 디스패치
    - 슬라이스 반복 동안 GIL 해제 (최대 성능)
- **`quoted_scalar()`** — 단일 따옴표 YAML 스타일이 필요한 값용 새 `CustomNode::quoted_scalar()` 생성자
- **따옴표 스칼라의 타입 해석** — `resolve_yaml_type`가 이제 `SingleQuoted`/`DoubleQuoted` 스칼라에 적용되어 따옴표 있는 음수의 올바른 루트립
- **포괄적 NumPy 테스트 스위트** — 모든 dtype, 차원 (0-D from 4-D), 음수, 무한대, NaN, 빈 배열, edge case를 다루는 42개 테스트
- Flow collection (`{}`/`[]`) 루트립 지원 — Mapping/Sequence AST 노드에 `flow_style` 필드
- `parse()`가 `str` 및 `bytes` 입력 모두 수용
- `parse()`가 `resolve_merges` 파라미터로 merge key 확장 옵트아웃 지원
- saphyr 이벤트를 통한 다중 문서 파싱용 `parse_all_docs()`
- `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` 파라미터가 있는 `to_yaml_with_options()`
- 기본값 파라미터를 지원하는 `get()`
- YAML을 파일에 작성하는 `dump_file()`
- `benches/yaml_bench.rs`의 Criterion 벤치마크 (파싱/직렬화/루트립)
- 매트릭스 테스트가 있는 GitHub Actions CI (3 OS x 4 Python 버전)
- 전체 YAML 1.2 spec으로 확장된 anchor 이름 파싱 (도트, 콜론, 해시, 따옴표 anchor)
- `__version__` 속성, `py.typed` PEP 561 마커

#### 수정

- **음수 루트립** — YAML 1.2 블록 시퀀스는 `-`로 시작하는 plain 스칼라를 포함할 수 없음; 이제 직렬화 시 따옴표로 감싸서 정수/부동소수로 올바르게 파싱
- **N-D 배열 지원** — `PyArray1<T>`를 `PyArrayDyn<T>`로 교체하여 1-D뿐만 아니라 모든 차원의 배열 지원
- **올바른 중첩 깊이** — 다차원 배열이 이제 정확히 N 수준의 중첩 생성 (내부 차원은 shape[1..] 처리, 루트 차원은 `plain_sequence`로 래핑)
- `to_dict()` 및 `safe_load()`의 alias 해석 — alias가 이제 참조값 대신 `None`으로 해석
- `safe_loads()`가 더 이상 단순 `split("---")`를 사용하지 않음 — saphyr의 문서 이벤트 사용
- 파싱 중 Mapping/Sequence 태그가 더 이상 폐기되지 않음
- `format_scalar_for_key()`이 Literal/Folded 블록 스칼라 스타일 처리

#### 변경

- ndarray 타입 디스패치를 위해 `numpy` crate (v0.29) 의존성 추가
- PyO3를 0.21에서 0.29로 업그레이드
- 15+ 보일러플레이트 `CustomNode` constructions를 `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` 생성자로 교체
- 직렬화器가 `write_anchor_tag()` 및 `write_inline_comment()` 헬퍼 추출
- 파서가 `detect_flow_style()` 헬퍼 추출
- 데드 코드 제거: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (테스트 전용)
- 6개 중복 테스트 파일 통합, 9개 진단 스크립트를 `scripts/`로 이동
- 키/인덱스/타입 컨텍스트로 오류 메시지 개선

### [0.1.0] - 2026-07-25

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
