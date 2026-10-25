"""The API pages must state the parameter list the shipped surface declares.

`python/pyrs_yaml/pyrs_yaml.pyi` is the generated truth of the extension layer, while
`python/pyrs_yaml/node.py` and `python/pyrs_yaml/merged_view.py` are the truth of the two public
classes the Python layer owns (`Node`, `MergedView` — the stub does not declare them). The four
locales' `docs/*/api/*.md` pages restate those signatures for readers who never open a stub. When
the two drift, a reader learns a smaller API than the one that ships: measured on 2026-10-20 in
`docs/{zh,ja,ko}/api/reference.md`, where `parse_file`, `parse_all_docs`, `safe_load` and
`safe_loads` were documented with one or two parameters against the stub's five or six, and where
`honor_version_directive` — the `%YAML` version-negotiation gate — appeared in no user page at all.

What makes the comparison sound, in the order each was learned by getting it wrong first:

* **A section, not a line.** Only the signature line a member heading introduces is the public
  surface; matching any `name(...) ->` line turns examples into findings — `yaml.safe_load(text)`
  in a snippet is a call, not a signature.
* **Both sides parsed by `ast`, keyed by class.** Splitting on commas reads the return annotation
  `-> dict[Any, Any] | list[Any]` as parameters, the stub's positional-only `/` hides names from a
  hand-rolled scan, and `append`/`insert`/`rename` exist on both `Node` and `YamlDocument` with
  different parameter lists.
* **Member headings come in two levels.** The English pages write ``#### `to_yaml()` `` under a
  ``### Methods`` group; the locale pages write ``### `to_yaml()` ``. Requiring three hashes made
  the gate bind nothing on the canonical English class pages — measured as 0 of the 26 sections in
  `docs/en/api/yaml-document.md`, and the blindness was invisible because the locale pages kept
  supplying plenty of matches.
* **Class context is the page's own `:::` directive, not its headings.** `docs/en/api/yaml-document.md`
  says `## YamlDocument Class`; `docs/ja` says `## 概要` / `## メソッド`, which names no class. The
  locale-independent authority is the mkdocstrings directive the page already carries
  (`::: pyrs_yaml.YamlDocument`), one per class page, `::: pyrs_yaml` on the module page. Guessing
  the class from any heading that mentions one instead qualified `reference.md`'s module functions
  as `YAML.parse()` and manufactured 18 drifts that were the gate's own artifact.

* **A locale key taken from a string path can collapse every group into one.** `as_posix()`
  on an absolute path makes `split("/")[1]` return `''`, so the four locales land under the same dict
  key and the cross-locale comparison compares a page only with itself — green forever. The locale is
  read from `Path.parts[-3]` instead, and the test that used to trust the key now floors the number of
  names it actually compares in all four locales.

As landed the reader binds 323 member sections, compares 304 of them against a declared source, and
compares 74 of 77 resolvable names across all four locales — the three floors in the tests below are
set well under those readings so a shrinking comparison goes red rather than silent.

A section whose resolved name no source declares is reported by name, never skipped: the allowed
set below is measured, and a new name there means a page documents something the package does not
publish.
"""

from __future__ import annotations

import ast
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent
STUB = REPO / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"
PYTHON_LAYER = [REPO / "python" / "pyrs_yaml" / "node.py", REPO / "python" / "pyrs_yaml" / "merged_view.py"]

MEMBER_HEADING = re.compile(r"^#{3,4}\s+`(\w+)\(\)`")
GROUP_HEADING = re.compile(r"^##\s(?!#)")
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
PAGE_DIRECTIVE = re.compile(r"^:::\s+pyrs_yaml\.([A-Za-z_]\w*)\s*$", re.M)

#: Names a page mentions that the declared sources cannot speak for. Verified 2026-10-20: each is
#: `hasattr(pyrs_yaml, name)` — true — and attached by `python/pyrs_yaml/__init__.py` after import
#: rather than declared as a top-level `def`, so no source file scan can reach its parameters; all of
#: them are listed in `pyrs_yaml.__all__`. `get` is `MergedView`'s inherited `Mapping.get`, which
#: `merged_view.py` does not re-declare.
UNSOURCED = {
    "compliance_report",
    "dump_pydantic",
    "parse_as",
    "safe_dump_async",
    "safe_dumps",
    "safe_load_async",
    "safe_loads_async",
    "get",
}


def _parameters(node: ast.FunctionDef) -> list[str]:
    a = node.args
    return [x.arg for x in [*a.posonlyargs, *a.args, *a.kwonlyargs] if x.arg not in ("self", "cls")]


def _surface(path: pathlib.Path) -> dict[str, list[str]]:
    """`name` and `Class.name` parameter lists as one source declares them."""
    tree = ast.parse(path.read_text(encoding="utf-8"))
    out: dict[str, list[str]] = {}
    for cls in [n for n in tree.body if isinstance(n, ast.ClassDef)]:
        for member in [f for f in cls.body if isinstance(f, ast.FunctionDef)]:
            out.setdefault(f"{cls.name}.{member.name}", _parameters(member))
    for func in [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.col_offset == 0]:
        out.setdefault(func.name, _parameters(func))
    return out


def public_signatures() -> dict[str, list[str]]:
    """The shipped public surface: the extension stub, then the Python layer's own classes."""
    out = _surface(STUB)
    for path in PYTHON_LAYER:
        out.update(_surface(path))
    return out


def documented_signatures() -> list[tuple[str, str, str | None, list[str]]]:
    """Each member section's own signature line, parsed, keyed by the page it lives on.

    The third element is the shipped-surface key it was compared against, or `None` when no source
    declares the name the page documents.
    """
    ref = public_signatures()
    classes = {key.split(".")[0] for key in ref if "." in key}
    out: list[tuple[str, str, str | None, list[str]]] = []
    for page in sorted((REPO / "docs").glob("*/api/*.md")):
        lines = page.read_text(encoding="utf-8").splitlines()
        directives = PAGE_DIRECTIVE.findall("\n".join(lines))
        page_cls = directives[0] if len(set(directives)) == 1 else None
        cls = page_cls
        for i, line in enumerate(lines):
            if GROUP_HEADING.match(line):
                named = [c for c in classes if c in IDENTIFIER.findall(line)]
                cls = named[0] if len(named) == 1 else page_cls
                continue
            member = MEMBER_HEADING.match(line)
            if not member:
                continue
            name = member.group(1)
            for j in range(i + 1, min(i + 12, len(lines))):
                sig = re.match(rf"^{name}\((.*)\)\s*->", lines[j])
                if not sig:
                    continue
                try:
                    fn = ast.parse(f"def {name}({sig.group(1)}) -> None:\n    ...").body[0]
                except SyntaxError:
                    break
                cands = [f"{cls}.{name}"] if cls else []
                cands.append(name)
                key = next((c for c in cands if c in ref), None)
                out.append((page.as_posix(), name, key, _parameters(fn)))
                break
    return out


def _resolved(seen):
    return [(page, key, params) for page, _name, key, params in seen if key is not None]


def test_every_documented_signature_matches_the_stub():
    ref = public_signatures()
    seen = documented_signatures()
    assert len(seen) >= 100, f"the gate bound only {len(seen)} member sections"
    unresolved = sorted({name for page, name, key, _ in seen if key is None})
    assert set(unresolved) <= UNSOURCED, (
        f"sections name {unresolved} which no source declares; if the package really publishes it "
        "at import time, add it to UNSOURCED with its evidence, otherwise the page is wrong"
    )
    compared = [(page, key, params) for page, _name, key, params in seen if key is not None]
    assert len(compared) >= 250, f"only {len(compared)} sections actually compared against a source"
    drift = [(page, key, params, ref[key]) for page, key, params in compared if params != ref[key]]
    assert not drift, "documented parameters differ from the shipped surface:\n" + "\n".join(
        f"  {page}  {key}(): docs={docs} shipped={real}" for page, key, docs, real in drift
    )


def test_the_locale_pages_agree_with_english_on_parameter_lists():
    """A zh/ja/ko reader must not learn a shorter signature than the en reader does."""
    by_key: dict[str, dict[str, list[str]]] = {}
    for page, key, params in _resolved(documented_signatures()):
        by_key.setdefault(key, {})[_locale(pathlib.PurePosixPath(page))] = params
    four = [key for key, pages in by_key.items() if len(pages) == 4]
    assert len(four) >= 40, f"only {len(four)} names were compared across all four locales, not a parity check"
    uneven = {
        key: sorted({tuple(v) for v in pages.values()})
        for key, pages in by_key.items()
        if len(pages) > 1 and len({tuple(v) for v in pages.values()}) > 1
    }
    assert not uneven, f"locales disagree on documented parameters: {uneven}"


#: Names a page mentions that no declared source publishes as a member, with the reason each is
#: tolerated. Measured 2026-10-20: `YamlDocument` is the constructor section ja/ko open (en/zh state
#: construction in prose), `__eq__` is inherited from `object` and no page declares it.
UNPUBLISHED = {
    "YamlDocument": "constructor section, ja/ko only; the class is published, the section shape differs",
    "__eq__": "inherited from object, no page declares it",
}


def _locale(path: pathlib.Path) -> str:
    """The locale a page belongs to, from its position: `docs/<loc>/api/<page>.md`.

    `as_posix().split("/")[1]` looks like it does this and is silently wrong for an absolute path -
    it yields `''` for every page, which collapses the four locales into one dict key and makes any
    cross-locale comparison compare a page only with itself. Caught by emptying the exception set of
    the coverage test and watching it pass anyway.
    """
    return path.parts[-3]


def _documented_names(page):
    """Names a page mentions before its generated block, in any shape: heading, alias line, prose.

    Section *form* is not coverage: `docs/en/api/reference.md` states `safe_dumps()` as an "Alias:"
    line inside `safe_dump()`, and `docs/en/api/yaml-document.md` gives `__setitem__()` and
    `__delitem__()` one combined heading. A gate that counts headings alone reads those as gaps.
    """
    lines = page.read_text(encoding="utf-8").splitlines()
    stop = next((i for i, ln in enumerate(lines) if ln.startswith(":::")), len(lines))
    names = set()
    for line in lines[:stop]:
        if line.startswith("#") or "Alias" in line or "**Equivalent" in line:
            names.update(re.findall(r"`(\w+)(?:\(\))?`", line))
    return names


def test_no_locale_page_is_thinner_than_the_others_on_member_names():
    """Coverage, not just parameter lists: a name one locale documents must be documented in all.

    Measured again 2026-10-25 across the four locales and the seven API pages: no page - English
    included - omits a member any sibling locale names, so the API is equally learnable in every
    language even though the section shapes differ. This is the assertion the signature comparison
    cannot make: two pages can agree on every parameter list they state and still differ in what
    they state.
    """
    by_page: dict[str, dict[str, set[str]]] = {}
    for page in sorted((REPO / "docs").glob("*/api/*.md")):
        by_page.setdefault(page.name, {})[_locale(page)] = _documented_names(page)
    # Measured 2026-10-25: 28 files = 7 page names x 4 locales. Both loops are vacuous when the glob
    # matches nothing, and a page name held by one locale alone is a comparison of a page with itself:
    # `union - docs` is empty in either case, so the run reports green without having read a page.
    assert len(by_page) >= 7, (
        f"docs/*/api/*.md matched {len(by_page)} page names, so no locale was compared with another"
    )
    for name, per in sorted(by_page.items()):
        assert len(per) >= 2, f"docs/*/api/{name} is published only in {sorted(per)}: nothing to compare"
        union = set().union(*per.values())
        for loc, docs in sorted(per.items()):
            missing = sorted(union - docs)
            assert not [m for m in missing if m not in UNPUBLISHED], (
                f"docs/{loc}/api/{name} omits {missing} that its siblings document"
            )


def test_no_public_callable_is_absent_from_every_locale():
    """A name the stub publishes must be findable in each locale's API pages.

    Measured 2026-10-20: `YamlDocument.flush_source()` was the only public callable named nowhere
    in `docs/en/**` outside the changelogs, and the same hole was identical in all four locales -
    coverage, not translation, was missing.
    """
    tree = ast.parse(STUB.read_text(encoding="utf-8"))
    public = set()
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            public.add(node.name)
        if isinstance(node, ast.ClassDef) and not node.name.startswith("_"):
            public.update(f.name for f in node.body if isinstance(f, ast.FunctionDef) and not f.name.startswith("_"))
    for loc in ("en", "zh", "ja", "ko"):
        pages = [page for page in (REPO / "docs" / loc).rglob("*.md") if "changelog" not in page.name]
        assert len(pages) >= 20, f"docs/{loc} contributed only {len(pages)} pages"
        tokens = set()
        for page in pages:
            tokens.update(re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", page.read_text(encoding="utf-8")))
        absent = sorted(public - tokens)
        assert not absent, f"docs/{loc} never names these public callables: {absent}"
