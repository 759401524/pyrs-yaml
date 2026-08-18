---
title: PyYAML 互換性
description: pyrs-yaml の PyYAML 互換性について説明します。移行方法、API の違い、注意点をカバーします。
tags:
  - docs
status: new
---

pyrs-yaml は PyYAML の**代替品**を提供し、移行を容易にします。

## シンプルな移行

```python
# 旧コード
import yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)

# 新コード
import pyrs_yaml as yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)
```

## 互換 API

| PyYAML 関数 | pyrs-yaml 対応 | 備考 |
|-------------|---------------|------|
| `yaml.safe_load()` | `pyrs_yaml.safe_load()` | :material-check: 同等 |
| `yaml.safe_loads()` | `pyrs_yaml.safe_loads()` | :material-check: 同等 |
| `yaml.safe_dump()` | `pyrs_yaml.safe_dump()` | :material-check: 同等 |
| `yaml.safe_dumps()` | `pyrs_yaml.safe_dumps()` | :material-check: 同等 |
| `yaml.load()` | `pyrs_yaml.safe_load()` | :material-alert: 安全バリアントを使用 |
| `yaml.dump()` | `pyrs_yaml.safe_dump()` | :material-alert: 安全バリアントを使用 |

## 主な違い

### pyrs-yaml が優れている点

| 機能 | PyYAML | pyrs-yaml |
|------|--------|-----------|
| 往復保存 | :material-close: コメント/アンカーを失う | :material-check: すべて保持 |
| パフォーマンス | ベースライン | **解析で21〜43倍、シリアライズで55〜177倍高速** |
| 型ヒント | 部分的 | :material-check: 完全な `.pyi` スタブ |
| ABI3 wheel | 該当なし | :material-check: すべての Python バージョンに対応 |
| i18n エラー | :material-close: 英語のみ | :material-check: 英語 + 中国語 |

#### 注意すべき点

1. **アンカー/エイリアスの処理**: PyYAML は往復時にアンカーを失う; pyrs-yaml は保持する
2. **コメントの位置**: pyrs-yaml は複雑なネスト構造でコメントの順序を変更する場合がある
3. **フロースタイル**: 両方とも保持するが、出力フォーマットが若干異なる場合がある
4. **エラーメッセージ**: pyrs-yaml はより詳細なコンテキスト付きの i18n エラーメッセージを使用

## 移行チェックリスト

- [ ] `import yaml` を `import pyrs_yaml as yaml` に置換
- [ ] すべての YAML パース/保存ワークフローをテスト
- [ ] 往復出力が期待通りであることを確認
- [ ] アンカー/エイリアスの動作を確認（使用している場合）
- [ ] カスタムエラーメッセージ用のエラーハンドリングを確認

## 移行例

```python
# 旧コード
import yaml


def load_config(path):
    with open(path) as f:
        return yaml.safe_load(f)


def save_config(data, path):
    with open(path, "w") as f:
        yaml.safe_dump(data, f)


# 新コード
import pyrs_yaml


def load_config(path):
    return pyrs_yaml.parse_file(path).to_dict()


def save_config(data, path):
    pyrs_yaml.dump_file(data, path)
```
