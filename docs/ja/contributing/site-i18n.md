---
title: サイト全体の i18n
description: ドキュメントサイトのサイト全体の i18n の仕組み、ディレクトリ構造、フロントマター、リンク規則を説明します。
tags:
  - docs
status: new
---

## サイト全体の i18n (MkDocs)

pyrs-yaml のドキュメントサイトは、MkDocs Material テーマの組み込み i18n を使用して **サイト全体の国際化** をサポートしています。ユーザーは英語（`en`）、中国語（`zh-CN`）、日本語（`ja-JP`）、韓国語（`ko-KR`）でドキュメントを表示できます。

ランタイムのエラーメッセージ i18n ガイドについては、[guides/i18n.md](../guides/i18n.md) で `set_language()` / `get_language()` を参照してください。

### How It Works

各言語は独自の URL パス（`/zh-CN/`、`/ja-JP/`、`/ko-KR/`）を持ち、`mkdocs.yml` で設定された右上隅の言語切り替え機能を備えた 1 つのナビゲーションを共有します:

```yaml title="mkdocs.yml の i18n 設定"
i18n:
  default_lang: en
  alternate_languages:
    - lang: zh-CN
      url: /zh-CN/
    - lang: ja-JP
      url: /ja-JP/
    - lang: ko-KR
      url: /ko-KR/
```

### Directory Structure

各ロケールは `docs/<lang>/` 以下に存在し、英語の `docs/en/` ツリーをミラーリングします:

```text title="ロケールのディレクトリ構造"
docs/en/  (canonical English)
docs/zh-CN/  (or docs/zh/)
docs/ja/  (or docs/ja-JP)
docs/ko/  (or docs/ko-KR)
```

### Frontmatter

翻訳されたファイルはすべて、`lang` フィールドを持つ YAML フロントマターを **必ず** 含める必要があります:

```yaml title="翻訳ファイルのフロントマター"
---
title: ドキュメントタイトル
lang: ja-JP
---
```

### Link Rules

- 内部リンクに言語プレフィックスを **含めないでください** — 相対パス（`quick-start.md`）を使用してください。
- コード例は言語間で変更されません。
- ライセンスの法的テキストは英語のままです。見出しと説明のみが翻訳されます。

### Verification

```bash title="ドキュメントのビルドとプレビュー"
uv sync
mkdocs build --clean-site
mkdocs serve   # http://localhost:8000/
```

### Troubleshooting

| Issue | Solution |
|-------|----------|
| 言語切り替えが表示されない | `i18n` ブロックが設定され、`alternate_languages.lang` ごとに対応するディレクトリが存在することを確認してください |
| リンク切れ | 内部リンクが相対パス（言語プレフィックスなし）を使用していることを確認してください |
| フロントマターがパースされない | すべてのファイルがマークダウンコンテンツの前に `---` で始まっていることを確認してください |
| 検索が言語ごとにならない | `mkdocs build --clean-site` で再ビルドしてください |
