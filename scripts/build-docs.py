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
