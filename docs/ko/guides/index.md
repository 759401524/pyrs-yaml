---
title: 사용자 가이드
description: pyrs-yaml로 YAML 파싱, 직렬화, 편집 및 통합 가이드.
tags:
  - docs
status: new
---

## 사용자 가이드

사용자 가이드는 두 섹션으로 구성됩니다:

### 핵심

핵심 YAML 작업 — 파싱, 직렬화, 라운드트립, 제자리 편집, 스트리밍 파싱.

- [YAML 파싱](parsing.md) — 문자열, 파일, 여러 문서 파싱
- [직렬화](serialization.md) — YAML 문서와 Python 객체 간 변환
- [PyYAML 호환](pyyaml-compat.md) — 직접 교체 가능한 API
- [라운드트립](round-trip.md) — 주석, 앵커, 태그, 서식 유지
- [제자리 편집](editing.md) — JSONPath 경로로 서식 손실 없이 문서 편집
- [스트리밍 파싱](streaming.md) — 상수 메모리 증분 파싱

### 통합

고급 기능 — 사용자 정의 스키마, 플러그인 개발, 커뮤니티 플러그인, 설정 관리, Markdown 프론트매터, i18n 오류 메시지, NumPy ndarray 지원.

- [사용자 정의 스키마](custom-schema.md) — 타입 해석용 사용자 정의 YAML 스키마 정의
- [플러그인 개발](plugin-development.md) — 사용자 정의 태그 핸들러와 노드 타입 구축
- [커뮤니티 플러그인](community-plugins.md) — datetime, UUID, decimal 등 내장 플러그인
- [설정 관리](tutorial-config-management.md) — 종단간 실습
- [Markdown 프론트매터](frontmatter.md) — Markdown 파일에서 YAML 프론트매터 추출
- [i18n 오류 메시지](i18n.md) — 오류 메시지 지역화
- [NumPy ndarray](numpy.md) — numpy 배열을 YAML로 직렬화
- [Pydantic 통합](pydantic.md) — YAML을 Pydantic 모델로 파싱하고 BaseSettings를 로드합니다
