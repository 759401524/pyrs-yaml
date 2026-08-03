"""Assert root CHANGELOG.md [Unreleased] equals docs/{en,ja,ko,zh} mirrors.

Exit code 0 = in sync; 1 = drift (with per-file diff hints).
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = [
    ROOT / "CHANGELOG.md",
    ROOT / "docs" / "en" / "changelog.md",
    ROOT / "docs" / "ja" / "changelog.md",
    ROOT / "docs" / "ko" / "changelog.md",
    ROOT / "docs" / "zh" / "changelog.md",
]

_SECTION_RE = re.compile(r"^#{2,3} \[Unreleased\](.*?)(?=^#{2,3} \[|\Z)", re.M | re.S)


def extract_unreleased(text: str) -> str:
    m = _SECTION_RE.search(text)
    return m.group(1).strip() if m else ""


def main() -> int:
    sections = {str(p): extract_unreleased(p.read_text(encoding="utf-8")) for p in FILES}
    root_sec = sections[str(FILES[0])]
    errors = []
    for path in FILES[1:]:
        if sections[str(path)] != root_sec:
            errors.append(f"{path}: [Unreleased] differs from root CHANGELOG.md")
    if errors:
        print("changelog drift detected:")
        print("\n".join(errors))
        return 1
    print("OK: all 5 changelog [Unreleased] sections in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main())
