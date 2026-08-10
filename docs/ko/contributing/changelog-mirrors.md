---
title: 체인지로그 미러
description: 체인지로그 미러 관리 — 구조적 일관성(Structural Parity), 워크플로, 규칙
tags:
  - docs
status: new
---

## 체인지로그 미러

체인지로그는 특별한 구조를 가지고 있습니다: `docs/{en,ja,ko,zh}/changelog.md`는 루트 `CHANGELOG.md`를 미러링하지만, `[Unreleased]` 섹션은 **로케일별로 번역**되며 이전 항목은 영어로 유지됩니다.

### Structural Parity

가드 스크립트 `scripts/check_changelog_mirrors.py`는 문자 그대로의 텍스트 동등성보다는 **구조적 일관성(Structural Parity)**(동일한 버전 헤더, `[Unreleased]` 섹션 존재)을 확인합니다. 이를 통해 번역 차이는 허용하면서 누락된 미러를 포착할 수 있습니다.

### Workflow

1. 먼저 루트 `CHANGELOG.md`(영어, 정식)에 항목을 작성합니다
2. 동일한 `[Unreleased]` 항목을 `docs/{zh,ja,ko}/changelog.md`로 번역합니다(버전 헤더 `## [Unreleased]`와 `### Added` 등은 번역 상태로 유지)
3. 검증:

```bash
uv run python scripts/check_changelog_mirrors.py
```

### Rules

| 규칙 | 설명 |
|------|-------------|
| **루트가 정식** | `CHANGELOG.md`가 기본 영어 소스입니다 |
| **Unreleased는 번역** | `[Unreleased]` 섹션만 로케일별로 다릅니다 |
| **이전 버전은 영어** | 모든 과거 버전 항목(`[v0.x.y]`)은 모든 미러에서 영어로 유지됩니다 |
| **부분 업데이트 금지** | 커밋 전에 4개 로케일 모두 함께 업데이트해야 합니다 |
| **헤더 유지** | 버전 헤더(`## [Unreleased]`, `### Added` 등)는 모든 로케일에 존재해야 합니다 |
