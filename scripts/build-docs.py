"""Build pyyaml-rs documentation for all locales using Zensical.

Each locale is built as a separate Zensical site with the correct theme.language,
so that the HTML lang attribute is accurate (zh, ja, ko, not en).
"""

import re
import subprocess
import sys
import tempfile
from pathlib import Path

import yaml

LOCALES = [
    {"code": "en", "name": "English", "lang": "en"},
    {"code": "zh", "name": "中文", "lang": "zh"},
    {"code": "ja", "name": "日本語", "lang": "ja"},
    {"code": "ko", "name": "한국어", "lang": "ko"},
]

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def strip_locale_prefix(nav_items, locale_code):
    for item in nav_items:
        if isinstance(item, dict):
            for key, value in list(item.items()):
                if isinstance(value, str):
                    expected = f"{locale_code}/"
                    if value.startswith(expected):
                        item[key] = value[len(expected) :]
                elif isinstance(value, list):
                    strip_locale_prefix(value, locale_code)


def build_locale(locale):
    locale_code = locale["code"]
    print(f"  Building {locale['name']} ({locale_code})...")

    config_path = PROJECT_ROOT / "mkdocs.yml"
    raw = config_path.read_text(encoding="utf-8")

    raw = re.sub(r"!!python/name:(\S+)", r"\1", raw)
    config = yaml.safe_load(raw)

    if "plugins" in config:
        config["plugins"] = [p for p in config["plugins"] if not isinstance(p, dict) or "i18n" not in p]

    config["docs_dir"] = f"docs/{locale_code}"
    config["site_dir"] = f"site/{locale_code}"
    config["theme"]["language"] = locale["lang"]

    if "edit_uri" in config:
        config["edit_uri"] = config["edit_uri"].replace("{locale}", "")

    if "markdown_extensions" in config:
        cleaned = []
        for ext in config["markdown_extensions"]:
            if isinstance(ext, dict):
                if "pymdownx.emoji" in ext:
                    emoji_cfg = {
                        k: v for k, v in ext["pymdownx.emoji"].items() if k not in ("emoji_index", "emoji_generator")
                    }
                    cleaned.append({"pymdownx.emoji": emoji_cfg} if emoji_cfg else "pymdownx.emoji")
                else:
                    cleaned.append(ext)
            else:
                cleaned.append(ext)
        config["markdown_extensions"] = cleaned

    if "nav" in config:
        strip_locale_prefix(config["nav"], locale_code)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".yml", delete=False, dir=PROJECT_ROOT) as tmp:
        yaml.dump(config, tmp, allow_unicode=True, sort_keys=False, default_flow_style=False)
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
        print("\nAll locales built successfully!")
    else:
        print("\nSome locales failed to build.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
