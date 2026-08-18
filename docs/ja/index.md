---
title: pyrs-yaml
description: 高性能な Python YAML ライブラリ。完璧な Round-Trip サポートを備え、Rust と PyO3 で構築されています。
tags:
  - docs
status: new
---

## 高性能な Python YAML ライブラリ、完璧な Round-Trip サポート、Rust と PyO3 で構築されています。

### なぜ pyrs-yaml を選ぶべきか？

ほとんどの Python YAML ライブラリは、パフォーマンスと忠実性のどちらかを犠牲にします。 pyrs-yaml の両方を提供します:

- **PyYAML** (Python) — 遅く、往復解析時に**コメント/アンカー/タグを失う**
- **ruamel.yaml** (Python) — フォーマットを保持するが、pyrs-yaml より **解析は 48–100 倍、シリアライズは 123–371 倍遅い**
- **pyrs-yaml** (Rust) — PyYAML より **解析は 21–43 倍、シリアライズは 55–177 倍高速**、すべてを保持

#### 主要機能

<div class="grid cards" markdown>

- :material-lightning-bolt: **超高速** — PyYAML より解析 21–43 倍、シリアライズ 55–177 倍高速、Rust ゼロコピーバックエンド駆動
- :material-sync: **完璧なラウンドトリップ** — コメント、アンカー、タグ、チョンピング、スカラースタイル、フロー/ブロックフォーマットを保持
- :material-pencil: **インプレース編集** — JSONPath スタイルのパス（`doc.set("$.a.b", v)`）または `Node` ツリー API で編集、フォーマットを失わない
- :material-check-decagram: **YAML 1.2 準拠** — granit-parser 駆動（YAML テストスイート 99.75% 合格率、405/406）
- :material-swap-horizontal: **PyYAML 互換** — `safe_load` / `safe_dump` API で直接置換可能
- :material-language-python: **型ヒント** — PEP 561 準拠、完全な `.pyi` スタブファイル
- :material-package-variant-closed: **ABI3 ホイール** — 単一ホイールで Python 3.8–3.15 に対応
- :material-translate: **国際化エラー** — `set_language("ja")` でバイリンガルエラーレポート
- :material-numeric: **NumPy ndarray** — 任意次元の `numpy.ndarray` をゼロコピー Rust ディスパッチでシリアライズ

</div>

#### クイックスタート

```bash title="インストール"
pip install pyrs-yaml
```

```python title="クイックスタート"
import pyrs_yaml

# Parse YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML compatible API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

#### PyYAML との比較

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.18 ms | 3.8 ms | **21×** |
| Parse (medium) | 0.56 ms | 24.2 ms | **43×** |
| Parse (large) | 1.5 ms | 57.7 ms | **38×** |
| Serialize (small) | 0.04 ms | 2.2 ms | **55×** |
| Serialize (medium) | 0.08 ms | 12.6 ms | **159×** |
| Serialize (large) | 0.17 ms | 30.2 ms | **177×** |

---

[クイックスタート :material-arrow-right:](quick-start.md){ .md-button .md-button--primary }
[API リファレンス :material-code-braces:](api/reference.md){ .md-button }
[GitHub で見る :fontawesome-brands-github:](https://github.com/759401524/pyrs-yaml){ .md-button }
