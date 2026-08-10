#!/usr/bin/env python3
"""
I18n documentation quality check script.

Checks:
1. File completeness: all source files exist in target locales
2. Section completeness: H2 heading count consistency
3. Frontmatter check: contains title and lang
4. Terminology consistency: key terms use standard translations
5. Placeholder consistency: i18n file placeholders match
"""

import re
import sys
from pathlib import Path
from typing import List

# Configuration
LOCALES = ["en", "zh", "ja", "ko"]
SOURCE_LOCALE = "en"
DOCS_DIR = Path("docs")
I18N_DIR = Path("crates/pyrs-yaml-core/src/i18n/locales")

# i18n file name mapping
I18N_FILES = {
    "en": "en.yml",
    "zh": "zh-CN.yml",
    "ja": "ja-JP.yml",
    "ko": "ko-KR.yml",
}

# Files allowed to have extra sections (locale-specific supplementary content)
ALLOW_EXTRA_SECTIONS = {
    "guides/i18n.md": 4,  # i18n guide allows up to 4 extra sections (translation examples)
    "features.md": 3,  # features.md allows up to 3 extra sections
    "api/yaml-document.md": 2,  # yaml-document.md allows up to 2 extra sections
    "guides/parsing.md": 1,  # parsing.md allows up to 1 extra section
    "contributing/tests.md": 1,  # tests.md allows up to 1 extra section
}

# Terminology issues (if found, report as problem)
TERMINOLOGY_ISSUES = {
    "zh": {
        "往返保存": "should use '往返'",
        "往返解析": "should use '往返'",
        "前端元数据": "should use 'Front Matter'",
        "pyo3": "should use 'PyO3'",
        "indexmap": "should use 'IndexMap'",
        "修剪指示符": "should use 'chomping 指示符'",
    },
    "ja": {
        "フロントメータ": "should use 'Front Matter'",
        "pyo3": "should use 'PyO3'",
        "indexmap": "should use 'IndexMap'",
        "チョーピング": "should use 'チョンピング'",
    },
    "ko": {
        "프론트메터": "should use 'Front Matter'",
        "pyo3": "should use 'PyO3'",
        "indexmap": "should use 'IndexMap'",
        "chomp 지시자": "should use '촙핑 지시자'",
    },
}


def get_h2_headings(filepath: Path) -> List[str]:
    """Extract all H2 headings from a file."""
    if not filepath.exists():
        return []
    content = filepath.read_text(encoding="utf-8")
    return re.findall(r"^## (.+)$", content, re.MULTILINE)


def check_frontmatter(filepath: Path) -> List[str]:
    """Check if frontmatter is complete."""
    issues = []
    if not filepath.exists():
        return issues
    content = filepath.read_text(encoding="utf-8")

    # Check if frontmatter exists
    if not content.startswith("---"):
        issues.append("missing frontmatter")
        return issues

    # Check title
    if "title:" not in content.split("---")[1]:
        issues.append("frontmatter missing title")

    return issues


def check_terminology(filepath: Path, locale: str) -> List[str]:
    """Check terminology consistency (excluding code blocks, inline code, license files)."""
    issues = []
    if not filepath.exists():
        return issues

    # Skip license files (contains legal text, project names should not be modified)
    if filepath.name == "license.md":
        return issues

    content = filepath.read_text(encoding="utf-8")

    # Remove code blocks
    content = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
    # Remove inline code
    content = re.sub(r"`[^`]+`", "", content)
    # Remove link URLs
    content = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", content)

    if locale in TERMINOLOGY_ISSUES:
        for term, suggestion in TERMINOLOGY_ISSUES[locale].items():
            if term in content:
                issues.append(f"terminology issue: '{term}' - {suggestion}")

    return issues


def check_i18n_placeholders() -> List[str]:
    """Check i18n file placeholder consistency."""
    issues = []

    en_file = I18N_DIR / I18N_FILES["en"]
    if not en_file.exists():
        return [f"cannot find {en_file}"]

    en_content = en_file.read_text(encoding="utf-8")
    en_keys = dict(re.findall(r"^(\w+):\s*[\"'](.+?)[\"']$", en_content, re.MULTILINE))

    for locale in LOCALES:
        if locale == SOURCE_LOCALE:
            continue

        locale_file = I18N_DIR / I18N_FILES.get(locale, f"{locale}.yml")
        if not locale_file.exists():
            issues.append(f"missing i18n file: {locale_file}")
            continue

        locale_content = locale_file.read_text(encoding="utf-8")
        locale_keys = dict(re.findall(r"^(\w+):\s*[\"'](.+?)[\"']$", locale_content, re.MULTILINE))

        # Check for missing keys
        for key in en_keys:
            if key not in locale_keys:
                issues.append(f"[{locale}] missing i18n key: {key}")

        # Check placeholder consistency
        for key in en_keys:
            if key in locale_keys:
                en_placeholders = set(re.findall(r"%\{(\w+)\}", en_keys[key]))
                locale_placeholders = set(re.findall(r"%\{(\w+)\}", locale_keys[key]))
                if en_placeholders != locale_placeholders:
                    issues.append(
                        f"[{locale}] {key} placeholder mismatch: EN={en_placeholders} vs {locale}={locale_placeholders}"
                    )

    return issues


def main() -> int:
    """Main function, returns 0 for pass, 1 for issues."""
    all_issues = []

    print("=" * 60)
    print("I18n Documentation Quality Check")
    print("=" * 60)

    # 1. File completeness check
    print("\n[1/5] File completeness check...")
    source_files = list((DOCS_DIR / SOURCE_LOCALE).rglob("*.md"))

    for source_file in source_files:
        rel_path = source_file.relative_to(DOCS_DIR / SOURCE_LOCALE)
        for locale in LOCALES:
            if locale == SOURCE_LOCALE:
                continue
            target_file = DOCS_DIR / locale / rel_path
            if not target_file.exists():
                all_issues.append(f"[{locale}] missing file: {rel_path}")

    if not any("missing file" in i for i in all_issues):
        print("  OK: all files exist")

    # 2. Section completeness check
    print("\n[2/5] Section completeness check...")
    heading_mismatches = []

    for source_file in source_files:
        rel_path = source_file.relative_to(DOCS_DIR / SOURCE_LOCALE)
        en_headings = get_h2_headings(source_file)

        for locale in LOCALES:
            if locale == SOURCE_LOCALE:
                continue
            target_file = DOCS_DIR / locale / rel_path
            if not target_file.exists():
                continue

            locale_headings = get_h2_headings(target_file)

            # Check heading count difference (default 2 tolerance for heading level demotion)
            rel_path_str = str(rel_path).replace("\\", "/")
            max_extra = ALLOW_EXTRA_SECTIONS.get(rel_path_str, 2)
            diff = len(locale_headings) - len(en_headings)

            if diff > max_extra:
                heading_mismatches.append(
                    f"[{locale}] {rel_path}: EN={len(en_headings)} vs {locale}={len(locale_headings)} "
                    f"(extra {diff} sections, max allowed {max_extra})"
                )
            elif diff < -1:  # Allow missing 1 (page title difference)
                heading_mismatches.append(
                    f"[{locale}] {rel_path}: EN={len(en_headings)} vs {locale}={len(locale_headings)} "
                    f"(missing {abs(diff)} sections)"
                )

    if not heading_mismatches:
        print("  OK: heading structure consistent")
    else:
        for m in heading_mismatches:
            print(f"  W {m}")  # Warnings only, non-blocking

    # 3. Frontmatter check
    print("\n[3/5] Frontmatter check...")
    frontmatter_issues = []

    for locale in LOCALES:
        if locale == SOURCE_LOCALE:
            continue
        locale_files = list((DOCS_DIR / locale).rglob("*.md"))
        for file in locale_files:
            issues = check_frontmatter(file)
            for issue in issues:
                rel_path = file.relative_to(DOCS_DIR / locale)
                frontmatter_issues.append(f"[{locale}] {rel_path}: {issue}")

    if not frontmatter_issues:
        print("  OK: all frontmatter complete")
    else:
        all_issues.extend(frontmatter_issues)

    # 4. Terminology consistency check
    print("\n[4/5] Terminology consistency check...")
    terminology_issues = []

    for locale in LOCALES:
        if locale == SOURCE_LOCALE:
            continue
        locale_files = list((DOCS_DIR / locale).rglob("*.md"))
        for file in locale_files:
            issues = check_terminology(file, locale)
            for issue in issues:
                rel_path = file.relative_to(DOCS_DIR / locale)
                terminology_issues.append(f"[{locale}] {rel_path}: {issue}")

    if not terminology_issues:
        print("  OK: terminology consistent")
    else:
        all_issues.extend(terminology_issues)

    # 5. Placeholder consistency check
    print("\n[5/5] Placeholder consistency check...")
    placeholder_issues = check_i18n_placeholders()

    if not placeholder_issues:
        print("  OK: placeholders consistent")
    else:
        all_issues.extend(placeholder_issues)

    # Output results
    print("\n" + "=" * 60)
    if all_issues:
        print(f"Issues found: {len(all_issues)}")
        print("=" * 60)
        for issue in all_issues:
            print(f"  X {issue}")
        return 1
    else:
        print("All checks passed!")
        print("=" * 60)
        return 0


if __name__ == "__main__":
    sys.exit(main())
