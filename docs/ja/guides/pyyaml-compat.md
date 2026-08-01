---

title: PyYAML 互換性
lang: ja

## PyYAML 互換性

pyrs-yaml は PyYAML の**代替品**を提供し、移行を容易にします。

### シンプルな移行

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

### 互換 API

| PyYAML 関数 | pyrs-yaml 対応 | 備考 |
|-------------|---------------|------|
| `yaml.safe_load()` | `pyrs_yaml.safe_load()` | ✅ 同等 |
| `yaml.safe_loads()` | `pyrs_yaml.safe_loads()` | ✅ 同等 |
| `yaml.safe_dump()` | `pyrs_yaml.safe_dump()` | ✅ 同等 |
| `yaml.safe_dumps()` | `pyrs_yaml.safe_dumps()` | ✅ 同等 |
| `yaml.load()` | `pyrs_yaml.safe_load()` | ⚠️ 安全バリアントを使用 |
| `yaml.dump()` | `pyrs_yaml.safe_dump()` | ⚠️ 安全バリアントを使用 |

### 主な違い

#### pyrs-yaml が優れている点

| 機能 | PyYAML | pyrs-yaml |
|------|--------|-----------|
| 往復保存 | ❌ コメント/アンカーを失う | ✅ すべて保持 |
| パフォーマンス | ベースライン | **25〜40 倍高速** |
| 型ヒント | 部分的 | ✅ 完全な `.pyi` スタブ |
| ABI3 wheel | 該当なし | ✅ すべての Python バージョンに対応 |
| i18n エラー | ❌ 英語のみ | ✅ 英語 + 中国語 |

#### 注意すべき点

1. **アンカー/エイリアスの処理**: PyYAML は往復時にアンカーを失う; pyrs-yaml は保持する
2. **コメントの位置**: pyrs-yaml は複雑なネスト構造でコメントの順序を変更する場合がある
3. **フロースタイル**: 両方とも保持するが、出力フォーマットが若干異なる場合がある
4. **エラーメッセージ**: pyrs-yaml はより詳細なコンテキスト付きの i18n エラーメッセージを使用

### 移行チェックリスト

- [ ] `import yaml` を `import pyrs_yaml as yaml` に置換
- [ ] すべての YAML パース/保存ワークフローをテスト
- [ ] 往復出力が期待通りであることを確認
- [ ] アンカー/エイリアスの動作を確認（使用している場合）
- [ ] カスタムエラーメッセージ用のエラーハンドリングを確認

### 移行例

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
