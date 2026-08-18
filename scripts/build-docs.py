"""Build pyrs-yaml documentation for all locales using Zensical.

Each locale is built as a separate Zensical site with the correct theme.language,
so that the HTML lang attribute is accurate (zh, ja, ko, not en).
"""

import subprocess
import sys
import tempfile
from pathlib import Path

import tomli_w

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # Python < 3.11

LOCALES = [
    {"code": "en", "name": "English", "lang": "en"},
    {"code": "zh", "name": "中文", "lang": "zh"},
    {"code": "ja", "name": "日本語", "lang": "ja"},
    {"code": "ko", "name": "한국어", "lang": "ko"},
]

LOCALE_NAV = {
    "en": [
        {"Home": "index.md"},
        {
            "Getting Started": [
                {"Installation": "installation.md"},
                {"Quick Start": "quick-start.md"},
                {"Features": "features.md"},
            ]
        },
        {
            "User Guide": [
                {
                    "Core": [
                        {"Parsing YAML": "guides/parsing.md"},
                        {"Serialization": "guides/serialization.md"},
                        {"PyYAML Compatibility": "guides/pyyaml-compat.md"},
                        {"Round-Trip Preservation": "guides/round-trip.md"},
                        {"In-Place Editing": "guides/editing.md"},
                        {"Streaming Parse": "guides/streaming.md"},
                    ]
                },
                {
                    "Integrations": [
                        {"Custom Schemas": "guides/custom-schema.md"},
                        {"Plugin Development": "guides/plugin-development.md"},
                        {"Community Plugins": "guides/community-plugins.md"},
                        {"Configuration Management": "guides/tutorial-config-management.md"},
                        {"Markdown Frontmatter": "guides/frontmatter.md"},
                        {"i18n Error Messages": "guides/i18n.md"},
                        {"NumPy ndarray": "guides/numpy.md"},
                    ]
                },
            ]
        },
        {
            "API Reference": [
                {"Module Reference": "api/reference.md"},
                {"YamlDocument Class": "api/yaml-document.md"},
                {"YAML Instance": "api/yaml-instance.md"},
                {"Node Class": "api/node.md"},
                {"MergedView Class": "api/merged-view.md"},
                {"Exceptions": "api/exceptions.md"},
            ]
        },
        {
            "Performance": [
                {"Benchmarks": "performance/benchmarks.md"},
                {"Comparison": "performance/comparison.md"},
            ]
        },
        {
            "Contributing": [
                {"Development Setup": "contributing/setup.md"},
                {"Architecture": "contributing/architecture.md"},
                {"Coding Standards": "contributing/standards.md"},
                {"Running Tests": "contributing/tests.md"},
                {"Changelog Mirrors": "contributing/changelog-mirrors.md"},
                {"Site-wide i18n": "contributing/site-i18n.md"},
            ]
        },
        {"Changelog": "changelog.md"},
        {"License": "license.md"},
    ],
    "zh": [
        {"首页": "index.md"},
        {
            "入门指南": [
                {"安装": "installation.md"},
                {"快速开始": "quick-start.md"},
                {"功能特性": "features.md"},
            ]
        },
        {
            "用户指南": [
                {
                    "核心": [
                        {"解析 YAML": "guides/parsing.md"},
                        {"序列化": "guides/serialization.md"},
                        {"PyYAML 兼容": "guides/pyyaml-compat.md"},
                        {"往返保留": "guides/round-trip.md"},
                        {"就地编辑": "guides/editing.md"},
                        {"流式解析": "guides/streaming.md"},
                    ]
                },
                {
                    "集成": [
                        {"自定义 Schema": "guides/custom-schema.md"},
                        {"插件开发": "guides/plugin-development.md"},
                        {"社区插件": "guides/community-plugins.md"},
                        {"配置管理": "guides/tutorial-config-management.md"},
                        {"Markdown 头信息": "guides/frontmatter.md"},
                        {"i18n 错误消息": "guides/i18n.md"},
                        {"NumPy ndarray": "guides/numpy.md"},
                    ]
                },
            ]
        },
        {
            "API 参考": [
                {"模块参考": "api/reference.md"},
                {"YamlDocument 类": "api/yaml-document.md"},
                {"YAML 实例": "api/yaml-instance.md"},
                {"Node 类": "api/node.md"},
                {"MergedView 类": "api/merged-view.md"},
                {"异常": "api/exceptions.md"},
            ]
        },
        {
            "性能": [
                {"基准测试": "performance/benchmarks.md"},
                {"对比": "performance/comparison.md"},
            ]
        },
        {
            "贡献": [
                {"开发环境搭建": "contributing/setup.md"},
                {"架构": "contributing/architecture.md"},
                {"编码标准": "contributing/standards.md"},
                {"运行测试": "contributing/tests.md"},
                {"更新日志镜像": "contributing/changelog-mirrors.md"},
                {"站点国际化": "contributing/site-i18n.md"},
            ]
        },
        {"更新日志": "changelog.md"},
        {"许可证": "license.md"},
    ],
    "ja": [
        {"ホーム": "index.md"},
        {
            "はじめに": [
                {"インストール": "installation.md"},
                {"クイックスタート": "quick-start.md"},
                {"機能": "features.md"},
            ]
        },
        {
            "ユーザーガイド": [
                {
                    "コア": [
                        {"YAML のパース": "guides/parsing.md"},
                        {"シリアライズ": "guides/serialization.md"},
                        {"PyYAML 互換": "guides/pyyaml-compat.md"},
                        {"ラウンドトリップ": "guides/round-trip.md"},
                        {"インプレース編集": "guides/editing.md"},
                        {"ストリーム解析": "guides/streaming.md"},
                    ]
                },
                {
                    "統合": [
                        {"カスタムスキーマ": "guides/custom-schema.md"},
                        {"プラグイン開発": "guides/plugin-development.md"},
                        {"コミュニティプラグイン": "guides/community-plugins.md"},
                        {"設定管理": "guides/tutorial-config-management.md"},
                        {"Markdown フロントマター": "guides/frontmatter.md"},
                        {"i18n エラーメッセージ": "guides/i18n.md"},
                        {"NumPy ndarray": "guides/numpy.md"},
                    ]
                },
            ]
        },
        {
            "API リファレンス": [
                {"モジュールリファレンス": "api/reference.md"},
                {"YamlDocument クラス": "api/yaml-document.md"},
                {"YAML インスタンス": "api/yaml-instance.md"},
                {"Node クラス": "api/node.md"},
                {"MergedView クラス": "api/merged-view.md"},
                {"例外": "api/exceptions.md"},
            ]
        },
        {
            "パフォーマンス": [
                {"ベンチマーク": "performance/benchmarks.md"},
                {"比較": "performance/comparison.md"},
            ]
        },
        {
            "コントリビュート": [
                {"開発環境のセットアップ": "contributing/setup.md"},
                {"アーキテクチャ": "contributing/architecture.md"},
                {"コーディング規約": "contributing/standards.md"},
                {"テストの実行": "contributing/tests.md"},
                {"チェンジログミラー": "contributing/changelog-mirrors.md"},
                {"サイトの国際化": "contributing/site-i18n.md"},
            ]
        },
        {"チェンジログ": "changelog.md"},
        {"ライセンス": "license.md"},
    ],
    "ko": [
        {"홈": "index.md"},
        {
            "시작하기": [
                {"설치": "installation.md"},
                {"빠른 시작": "quick-start.md"},
                {"기능": "features.md"},
            ]
        },
        {
            "사용자 가이드": [
                {
                    "핵심": [
                        {"YAML 파싱": "guides/parsing.md"},
                        {"직렬화": "guides/serialization.md"},
                        {"PyYAML 호환": "guides/pyyaml-compat.md"},
                        {"라운드트립": "guides/round-trip.md"},
                        {"제자리 편집": "guides/editing.md"},
                        {"스트리밍 파싱": "guides/streaming.md"},
                    ]
                },
                {
                    "통합": [
                        {"사용자 정의 스키마": "guides/custom-schema.md"},
                        {"플러그인 개발": "guides/plugin-development.md"},
                        {"커뮤니티 플러그인": "guides/community-plugins.md"},
                        {"설정 관리": "guides/tutorial-config-management.md"},
                        {"Markdown 프론트매터": "guides/frontmatter.md"},
                        {"i18n 오류 메시지": "guides/i18n.md"},
                        {"NumPy ndarray": "guides/numpy.md"},
                    ]
                },
            ]
        },
        {
            "API 참조": [
                {"모듈 참조": "api/reference.md"},
                {"YamlDocument 클래스": "api/yaml-document.md"},
                {"YAML 인스턴스": "api/yaml-instance.md"},
                {"Node 클래스": "api/node.md"},
                {"MergedView 클래스": "api/merged-view.md"},
                {"예외": "api/exceptions.md"},
            ]
        },
        {
            "성능": [
                {"벤치마크": "performance/benchmarks.md"},
                {"비교": "performance/comparison.md"},
            ]
        },
        {
            "기여": [
                {"개발 환경 설정": "contributing/setup.md"},
                {"아키텍처": "contributing/architecture.md"},
                {"코딩 표준": "contributing/standards.md"},
                {"테스트 실행": "contributing/tests.md"},
                {"변경 로그 미러": "contributing/changelog-mirrors.md"},
                {"사이트 국제화": "contributing/site-i18n.md"},
            ]
        },
        {"변경 로그": "changelog.md"},
        {"라이선스": "license.md"},
    ],
}

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def build_locale(locale):
    locale_code = locale["code"]
    print(f"  Building {locale['name']} ({locale_code})...")

    config_path = PROJECT_ROOT / "zensical.toml"
    with config_path.open("rb") as f:
        config = tomllib.load(f)

    project = config["project"]

    project["docs_dir"] = f"docs/{locale_code}"
    project["site_dir"] = f"site/{locale_code}"
    project["theme"]["language"] = locale["lang"]
    project["nav"] = LOCALE_NAV[locale_code]
    project["edit_uri"] = f"edit/main/docs/{locale_code}/"

    with tempfile.NamedTemporaryFile(mode="wb", suffix=".toml", delete=False, dir=PROJECT_ROOT) as tmp:
        tomli_w.dump(config, tmp)
        tmp_path = tmp.name

    try:
        result = subprocess.run(
            [sys.executable, "-m", "zensical", "build", "--config-file", tmp_path, "--strict"],
            cwd=PROJECT_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"    ERROR: {result.stderr.strip() or result.stdout.strip()}", file=sys.stderr)
            return False
        for line in result.stdout.splitlines():
            if "warning" in line.lower() or "warn" in line.lower():
                print(f"    {line}")
        return True
    finally:
        Path(tmp_path).unlink()


def main():
    success = True
    for locale in LOCALES:
        if not build_locale(locale):
            success = False
    if success:
        _create_root_redirect()
        print("\nAll locales built successfully!")
    else:
        print("\nSome locales failed to build.", file=sys.stderr)
        sys.exit(1)


def _create_root_redirect():
    site_dir = PROJECT_ROOT / "site"
    site_dir.mkdir(parents=True, exist_ok=True)
    html = """<!DOCTYPE html>
<meta charset="utf-8">
<title>pyrs-yaml</title>
<meta http-equiv="refresh" content="0; URL=/pyrs-yaml/en/">
<link rel="canonical" href="/pyrs-yaml/en/">
"""
    (site_dir / "index.html").write_text(html, encoding="utf-8")
    print("  Created root redirect: / -> /en/")


if __name__ == "__main__":
    main()
