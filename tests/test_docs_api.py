"""Docs/API consistency: every ``pyrs_yaml`` attribute referenced in docs exists.

Guards the "documented surface == real surface" invariant: a doc page that
imports or references a symbol the runtime does not export (the kind of drift
the missing ``YamlStream`` export introduced) fails this test.

Scans fenced code blocks and ``pyrs_yaml.X`` inline-code spans in every
``docs/<locale>/**/*.md``:

- ``import pyrs_yaml[.mod[.sub]]`` chains resolve via importlib
- ``from pyrs_yaml[.mod] import a, b`` resolve as attributes of that module
- ``pyrs_yaml.attr[.attr]`` chains resolve attribute-by-attribute

Chain links that are *instance* members (lowercase-first after a class, e.g.
``pyrs_yaml.YAML().safe_load`` or ``doc.to_yaml()``) are not library-export
claims, so resolution stops at the first link that does not exist as a module
attribute and the remainder is ignored.
"""

from __future__ import annotations

import importlib
import re
from pathlib import Path

import pyrs_yaml

DOCS_ROOT = Path(__file__).resolve().parent.parent / "docs"

# Fenced code blocks: ```py / ```python / ```bash / ```console / ```text ...
FENCE_RE = re.compile(r"^```(\w*)[^\n]*\n(.*?)^```", re.S | re.M)
# `pyrs_yaml.foo.bar` inside single-backtick inline code.
INLINE_RE = re.compile(r"`([^`\n]*pyrs_yaml[^`\n]*)`")
# Full dotted chains starting at pyrs_yaml (not followed by a call paren on a
# lowercase link — those are instance methods, handled by early stop).
CHAIN_RE = re.compile(r"pyrs_yaml(?:\.[A-Za-z_][A-Za-z0-9_]*)+")
FROM_RE = re.compile(r"from\s+(pyrs_yaml(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s+import\s+([^\n#]+)")
IMPORT_RE = re.compile(r"^\s*import\s+(pyrs_yaml(?:\.[A-Za-z_][A-Za-z0-9_]*)*)", re.M)

# Extension filenames like ``pyrs_yaml.abi3.so`` / ``pyrs_yaml.cp314t.pyd`` are
# not attribute paths; drop chains whose next segment is a binary suffix.
BINARY_SUFFIXES = {"abi3", "pyd", "so", "cp38", "cp312", "cp314t", "win_amd64"}


def resolve_chain(chain: str) -> tuple[bool, str]:
    """Resolve `pyrs_yaml.a.b` attribute-by-attribute.

    Returns (ok, broken_link). Resolution stops successfully at the first link
    that is not a module attribute but looks like an instance member (the docs
    are calling a method on an object, not claiming an export).
    """
    parts = chain.split(".")
    obj: object = pyrs_yaml
    for part in parts[1:]:
        if part in BINARY_SUFFIXES:
            return True, ""  # filename tail, not an attribute claim
        if not hasattr(obj, part):
            # Classes are capitalized; a lowercase link after a class is an
            # instance member claim (e.g. YAML().load) — not an export check.
            if part[0].islower() and not isinstance(obj, type):
                return True, ""
            return False, chain + " (missing ." + part + ")"
        obj = getattr(obj, part)
    return True, ""


def collect_claims(text: str) -> list[str]:
    claims: list[str] = []
    in_code = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            in_code = not in_code
            continue
        sources: list[str] = []
        if in_code:
            sources.append(line)
        sources.extend(INLINE_RE.findall(line))
        for src in sources:
            for match in FROM_RE.finditer(src):
                module = importlib.import_module(match.group(1))
                names = [n.strip().split(" as ")[0].strip() for n in match.group(2).split(",") if n.strip()]
                for name in names:
                    if name and name != "*":
                        full = f"{match.group(1)}.{name}"
                        if not hasattr(module, name):
                            claims.append(full)
            for match in IMPORT_RE.finditer(src):
                claims.append(match.group(1))
            claims.extend(CHAIN_RE.findall(src))
    return claims


def test_docs_reference_only_real_api():
    broken: list[str] = []
    checked = 0
    repo_root = DOCS_ROOT.parent
    # Locale guides/API pages plus the root READMEs all claim the shipped
    # surface; design specs under docs/superpowers intentionally document
    # planned APIs and stay excluded.
    paths = sorted(DOCS_ROOT.glob("*/**/*.md")) + sorted(repo_root.glob("README*.md"))
    for path in paths:
        if "superpowers" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        for claim in collect_claims(text):
            checked += 1
            ok, detail = resolve_chain(claim)
            if not ok:
                rel = path.relative_to(DOCS_ROOT.parent)
                broken.append(f"{rel}: {detail}")
    assert checked > 100, f"suspiciously few API claims found ({checked}) — parser drift?"
    assert not broken, "docs reference missing API:\n" + "\n".join(sorted(set(broken)))
