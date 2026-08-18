---
title: 사이트 전체 i18n
description: 사이트 전체 i18n — 작동 원리, 디렉토리 구조, 프론트메타데이터, 링크 규칙, 검증, 문제 해결
tags:
  - docs
status: new
---

## 사이트 전체 i18n (MkDocs)

pyrs-yaml 문서 사이트는 MkDocs Material 테마의 내장 i18n을 사용하여 **사이트 전체 국제화**를 지원합니다. 사용자는 영어(`en`), 중국어(`zh-CN`), 일본어(`ja-JP`), 한국어(`ko-KR`)로 문서를 볼 수 있습니다.

런타임 에러 메시지 i18n 가이드는 [guides/i18n.md](../guides/i18n.md)에서 `set_language()` / `get_language()`를 참조하세요.

### How It Works

각 언어는 고유한 URL 경로(`/zh-CN/`, `/ja-JP/`, `/ko-KR/`)를 가지며, `zensical.toml`에 구성된 언어 전환기를 오른쪽 상단에 공유합니다:

```toml title="zensical.toml i18n 설정"
[project.extra]
alternate = [
    {name = "English", link = "/pyrs-yaml/en/", lang = "en"},
    {name = "中文", link = "/pyrs-yaml/zh/", lang = "zh"},
    {name = "日本語", link = "/pyrs-yaml/ja/", lang = "ja"},
    {name = "한국어", link = "/pyrs-yaml/ko/", lang = "ko"},
]
```

### Directory Structure

각 로케일은 `docs/<lang>/` 아래에 있으며, 영어 `docs/en/` 트리를 미러링합니다:

```text title="로케일 디렉터리 구조"
docs/en/  (canonical English)
docs/zh-CN/  (or docs/zh/)
docs/ja/  (or docs/ja-JP)
docs/ko/  (or docs/ko-KR)
```

### Frontmatter

모든 번역 파일은 `lang` 필드가 포함된 YAML 프론트매터를 **반드시** 가져야 합니다:

```yaml title="번역 파일 프론트매터"
---
title: 文档标题
lang: zh-CN
---
```

### Link Rules

- 내부 링크에 **언어 접두사를 포함하지 마세요** — 상대 경로를 사용하세요(`quick-start.md`).
- 코드 예제는 언어 간에 변경되지 않습니다.
- 라이선스 법률 텍스트는 영어로 유지되며, 제목/설명만 번역됩니다.

### Verification

```bash title="문서 빌드 및 미리보기"
# 4개 언어 모두 빌드
uv run --group docs python scripts/build-docs.py

# 개발용 단일 언어 미리보기
uv run --group docs python -m zensical serve --config-file zensical.toml --dirty
```

### Troubleshooting

| 문제 | 해결 방법 |
|-------|----------|
| 언어 전환기가 표시되지 않음 | `i18n` 블록이 구성되어 있고 모든 `alternate_languages.lang`에 일치하는 디렉토리가 있는지 확인하세요 |
| 깨진 링크 | 내부 링크가 상대 경로(언어 접두사 없음)를 사용하는지 확인하세요 |
| 프론트매터가 파싱되지 않음 | 모든 파일이 마크다운 콘텐츠 전에 `---`로 시작하는지 확인하세요 |
| 언어별 검색이 작동하지 않음 | `uv run --group docs python scripts/build-docs.py`로 다시 빌드하세요 |
