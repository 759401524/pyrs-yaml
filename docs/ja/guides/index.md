---
title: ユーザーガイド
description: pyrs-yaml を使用した YAML のパース、シリアライズ、編集、統合のガイド。
tags:
  - docs
status: new
---

## ユーザーガイド

ユーザーガイドは 2 つのセクションに分かれています：

### コア

コア YAML 操作 — パース、シリアライズ、ラウンドトリップ、インプレース編集、ストリーム解析。

- [YAML のパース](parsing.md) — 文字列、ファイル、複数ドキュメントのパース
- [シリアライズ](serialization.md) — YAML ドキュメントと Python オブジェクト間の変換
- [PyYAML 互換](pyyaml-compat.md) — 直接置換可能な API
- [ラウンドトリップ](round-trip.md) — コメント、アンカー、タグ、フォーマットを保持
- [インプレース編集](editing.md) — JSONPath パスでフォーマットを失わずに編集
- [ストリーム解析](streaming.md) — 一定メモリのインクリメンタルパース

### 統合

高度な機能 — カスタムスキーマ、プラグイン開発、コミュニティプラグイン、設定管理、Markdown フロントマター、i18n エラーメッセージ、NumPy ndarray サポート。

- [カスタムスキーマ](custom-schema.md) — 型解決用のカスタム YAML スキーマを定義
- [プラグイン開発](plugin-development.md) — カスタムタグハンドラとノードタイプを構築
- [コミュニティプラグイン](community-plugins.md) — datetime、UUID、decimal などの組み込みプラグイン
- [設定管理](tutorial-config-management.md) — エンドツーエンドのチュートリアル
- [Markdown フロントマター](frontmatter.md) — Markdown ファイルから YAML フロントマターを抽出
- [i18n エラーメッセージ](i18n.md) — エラーメッセージのローカライズ
- [NumPy ndarray](numpy.md) — numpy 配列を YAML にシリアライズ
- [Pydantic 統合](pydantic.md) — YAML を Pydantic モデルにパースし、BaseSettings を読み込みます
