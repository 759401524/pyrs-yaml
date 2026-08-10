---
title: チェンジログミラー
description: チェンジログミラーの構造的パリティとワークフローについて説明します。
tags:
  - docs
status: new
---

## チェンジログミラー

チェンジログは特別な構造を持っています: `docs/{en,ja,ko,zh}/changelog.md` はルートの `CHANGELOG.md` をミラーリングしますが、`[Unreleased]` セクションは **ロケールごとに翻訳** され、過去のエントリは英語のままです。

### Structural Parity

ガードスクリプト `scripts/check_changelog_mirrors.py` は、テキストの完全一致ではなく **構造的パリティ**（同じバージョンヘッダー、`[Unreleased]` セクションの存在）をチェックします。これにより、翻訳の差異を許容しながら、欠落したミラーを検出できます。

### Workflow

1. 最初にルートの `CHANGELOG.md`（英語、正規）にエントリを記述します
2. 同じ `[Unreleased]` エントリを `docs/{zh,ja,ko}/changelog.md` に翻訳します（`## [Unreleased]` や `### Added` などのバージョンヘッダーは翻訳します）
3. 確認します:

```bash
uv run python scripts/check_changelog_mirrors.py
```

### Rules

| Rule | Description |
|------|-------------|
| **Root is canonical** | `CHANGELOG.md` がプライマリの英語ソースです |
| **Unreleased is translated** | `[Unreleased]` セクションのみロケールごとに異なります |
| **Historical is English** | 過去のバージョンエントリ（`[v0.x.y]`）はすべてのミラーで英語のままです |
| **Never partial** | コミット前に 4 つすべてのロケールを一緒に更新する必要があります |
| **Headers stay** | バージョンヘッダー（`## [Unreleased]`、`### Added` など）はすべてのロケールに存在する必要があります |
