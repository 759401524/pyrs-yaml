"""Assert root CHANGELOG.md mirrors are structurally in sync across locales.

Instead of comparing text verbatim (which breaks translation), this script
checks structural parity: every locale must declare the same set of version
headers (## [X.Y.Z]) and must contain a [Unreleased] section. This catches
common mistakes — adding an entry to root but forgetting a mirror — without
requiring translated content to match the English text byte-for-byte.

Exit code 0 = structurally in sync; 1 = drift detected.
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

_VERSION_RE = re.compile(r"^#{2,3} \[(\d+\.\d+\.\d+)\](?: — [^\]]+)?\s*$", re.M)
_UNRELEASED_RE = re.compile(r"^#{2,3} \[Unreleased\]", re.M)


def _versions(text: str) -> set[str]:
    return set(_VERSION_RE.findall(text))


def _has_unreleased(text: str) -> bool:
    return bool(_UNRELEASED_RE.search(text))


def main() -> int:
    errors: list[str] = []
    root_text = FILES[0].read_text(encoding="utf-8")
    root_versions = _versions(root_text)

    for path in FILES[1:]:
        text = path.read_text(encoding="utf-8")
        versions = _versions(text)
        unreleased = _has_unreleased(text)

        missing = root_versions - versions
        if missing:
            errors.append(f"{path.name}: missing versions {sorted(missing)}")
        extra = versions - root_versions
        if extra:
            errors.append(f"{path.name}: has extra versions {sorted(extra)}")
        if not unreleased:
            errors.append(f"{path.name}: missing [Unreleased] section")

    # Also verify root has all versions the mirrors do (catches root missing
    # entries after mirrors are updated first, which sometimes happens).
    mirror_texts = {p.name: p.read_text(encoding="utf-8") for p in FILES[1:]}
    mirror_versions = set()
    for t in mirror_texts.values():
        mirror_versions |= _versions(t)
    root_missing = mirror_versions - root_versions
    if root_missing:
        errors.append(f"root CHANGELOG.md: missing versions {sorted(root_missing)}")

    if errors:
        print("changelog structural drift detected:")
        print("\n".join(errors))
        return 1
    print("OK: all 5 changelogs structurally in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main())
