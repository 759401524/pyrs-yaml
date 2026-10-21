"""Measure the quality-defence matrix from the files that declare it.

The four questions a "we have tests" answer cannot answer: which tiers exist per
format, which of them can actually stop a merge, what each tier is blind to, and
whether a hole that was closed stays closed. Every number here is parsed out of the
declaring files (`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`,
`scripts/check_*.py`, the Ir bench, `.ci/ir-baseline.json`), never transcribed — a
transcribed matrix rots the moment a workflow is edited, and the document describing
the defence is the one artefact nothing checks.

`tests/test_quality_matrix.py` compares the derived holes against the committed
registry in `.ci/quality-holes.json`, so an undeclared blind spot fails CI and a hole
that has quietly been closed also fails CI until it is deleted from the registry.

Usage:
    python scripts/quality_matrix.py            # human-readable report
    python scripts/quality_matrix.py --json     # machine-readable dump
    python scripts/quality_matrix.py --holes    # derived hole ids, one per line
    python scripts/quality_matrix.py --check-rows   # fail if QUALITY_MATRIX.md's counts drifted
    python scripts/quality_matrix.py --write-rows   # rewrite them from this measurement
"""

from __future__ import annotations

import ast
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import NamedTuple

REPO = Path(__file__).resolve().parent.parent
WORKFLOWS = REPO / ".github" / "workflows"
MATRIX_DOC = REPO / "QUALITY_MATRIX.md"

# A crate exporting one of these owns a writer, so a round-trip invariant exists in
# it and a parse-only fuzz target structurally cannot reach it. Prefix matching, not
# word boundaries: `pyrs-json`'s writers are `to_json_text` / `to_json5_text`, which a
# `\b` after `to_json` silently drops - measured, and it hid a real round-trip hole.
SERIALIZER_RE = re.compile(
    r"^\s*pub fn (?:to_(?:string|yaml|json|jsonc|json5|toml)|dump|serialize|encode)[a-z_0-9]*\b",
    re.MULTILINE,
)


# The declaration that a crate owns an instruction-count harness: a `[[bench]]` target gated behind
# the `ir-gate` feature. Read from the manifest, not from the file list, because the gate builds what
# the manifest declares - a `benches/ir_gate.rs` with no such target is never compiled, and its
# scenarios would sit in the baseline unmeasured.
IR_GATE_TARGET_RE = re.compile(r'^\[\[bench\]\][^\[]*?required-features *= *\[[^\]]*"ir-gate"', re.S | re.M)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def run_commands(text: str) -> list[str]:
    """Every command a workflow can execute, block scalars included.

    Deliberately narrower than "the text of the file": a hook id merely mentioned in
    an explanatory comment would otherwise count as reachable, which is how a gate
    gets reported as wired while it never runs. Quoted spans are stripped for the same
    reason: `echo "cargo clippy is owned by ci.yml"` is a step, but it lints nothing.
    """
    lines = text.splitlines()
    commands = []
    index = 0
    while index < len(lines):
        line = lines[index]
        match = re.match(r"^(\s*)(?:-\s+)?run:\s*(.*)$", line)
        if not match:
            index += 1
            continue
        indent, rest = match.group(1), match.group(2).strip()
        if rest.startswith(("|", ">")):
            body = []
            index += 1
            while index < len(lines):
                following = lines[index]
                if following.strip() and len(re.match(r"^\s*", following).group(0)) <= len(indent):
                    break
                body.append(following.strip())
                index += 1
            commands.append(" ".join(body))
        else:
            commands.append(rest)
            index += 1
    return [re.sub(r"\"[^\"]*\"|'[^']*'", '""', command) for command in commands]


def workflow_commands() -> dict[str, list[str]]:
    return {p.name: run_commands(read(p)) for p in sorted(WORKFLOWS.glob("*.yml"))}


def prek_entries() -> dict[str, str]:
    """Hook id -> the command it runs, empty for builtin hooks."""
    text = read(REPO / "prek.toml")
    entries = {}
    for block in re.finditer(r"\[\[repos\.hooks\]\](.*?)(?=\[\[|\Z)", text, re.DOTALL):
        body = block.group(1)
        hook_id = re.search(r'id = "([a-z0-9-]+)"', body)
        entry = re.search(r'entry = "([^"]+)"', body)
        if hook_id:
            entries[hook_id.group(1)] = entry.group(1) if entry else ""
    for inline in re.finditer(r'\{ id = "([a-z0-9-]+)"([^}]*)\}', text):
        entries.setdefault(inline.group(1), "")
    return entries


def prek_hooks() -> list[str]:
    return sorted(prek_entries())


def fuzz_targets() -> list[str]:
    text = read(REPO / "fuzz" / "Cargo.toml")
    declared = re.findall(r'^\[\[bin\]\]\nname = "([a-z0-9_]+)"', text, re.MULTILINE)
    return sorted(set(declared))


def fuzz_ci_matrix() -> list[str]:
    text = read(WORKFLOWS / "fuzz.yml")
    match = re.search(r"target: \[([^\]]+)\]", text)
    if not match:
        return []
    return sorted(x.strip() for x in match.group(1).split(","))


def fuzz_crates() -> list[str]:
    """Crates the fuzz harness links, straight out of `fuzz/Cargo.toml`.

    This is the honest denominator: the harness's own dependency list says which engines
    the fuzz tier is responsible for. Inferring it from exported function names dropped
    `pyrs-json` and `pyrs-toml` (their entry points are not `parse*`/`from_str`), which
    would have read as "no round-trip hole there" - a false clean.
    """
    text = read(REPO / "fuzz" / "Cargo.toml")
    return sorted(set(re.findall(r'^([a-z0-9-]+) = \{ path = "\.\./crates/', text, re.MULTILINE)))


def fuzz_seed_inventory() -> dict:
    """The archived corpus, counted from `fuzz/seeds/**/*.seed`.

    Counted from the seeds rather than inferred from the targets: a target with no archived
    input still runs, but the PR replay tier has nothing to replay for it, so the target
    count alone would overstate what that tier can prove.
    """
    seeds = [path for path in (REPO / "fuzz" / "seeds").rglob("*.seed") if path.is_file()]
    return {"seeds": len(seeds), "former_crash": sum("former-crash-" in path.name for path in seeds)}


def engine_crates(fuzz_links: list[str]) -> list[str]:
    """The fuzzed crates that also own a writer, so a round trip exists to test."""
    found = []
    for crate in fuzz_links:
        src = REPO / "crates" / crate / "src"
        if not src.is_dir():
            continue
        if any(SERIALIZER_RE.search(read(py)) for py in sorted(src.rglob("*.rs"))):
            found.append(crate)
    return found


def roundtrip_target_for(crate: str, targets: list[str]) -> bool:
    """Does some fuzz target round-trip this engine's format?"""
    stem = crate.replace("pyrs-", "").replace("-core", "")
    family = {"ast": "yaml", "yaml": "yaml"}.get(stem, stem)
    return any("roundtrip" in target and family in target for target in targets)


#: One row of the per-format table: the crate that owns the engine, the name fragment that identifies
#: the format inside a fuzz target, the cross-library module's file stem, and the name fragments that
#: identify it inside a property function. Written as fragments that are then matched against the files,
#: not as a table of results, so adding a target or a module moves the row instead of leaving a
#: hand-copied cell behind. A property counts for a row when it lives in that crate or carries one of
#: those fragments.
#:
#: Two rows own no crate and no writer. JSON Schema validation is delegated to `jsonschema`, so its
#: property tier is found by fragment and its round-trip cell is `n/a` by construction rather than by
#: an empty measurement. The `rules:` schema language resolves scalars and owns no serializer of its
#: own, so its round-trip cell is `n/a` for the same reason - the document it influences is the YAML
#: row's, not a second one.
#:
#: `unit_scope` and `prop_scope` narrow a row to the files that declare the surface, because a
#: surface that shares a crate with a format is not defended by that crate's tests. The schema rule
#: language lives in `pyrs-yaml-core` beside the YAML engine: measured by the whole crate it would
#: read as defended by hundreds of tests that never mention it, so its cells are counted in
#: `src/parser/yaml/schema_language.rs` and in the one property function that names it.
class FormatRow(NamedTuple):
    label: str
    crate: str | None
    fragment: str
    crosslib_stem: str | None
    prop_fragments: tuple
    unit_scope: str | None = None
    prop_scope: str | None = None
    writer: bool = True


FORMAT_ROWS = (
    FormatRow("YAML", "pyrs-yaml-core", "yaml", "yaml", ("yaml",)),
    FormatRow("JSON / JSONC / JSON5", "pyrs-json", "json", "json", ("json",)),
    FormatRow("TOML", "pyrs-toml", "toml", "toml", ("toml",)),
    FormatRow("JSON Schema validation", None, "document_validate", None, ("json_schema",), writer=False),
    FormatRow(
        "Schema rule language",
        "pyrs-yaml-core",
        "parse_schema",
        None,
        ("schema",),
        unit_scope="src/parser/yaml/schema_language.rs",
        prop_scope="src/pbt.rs",
        writer=False,
    ),
)

#: An import in a cross-library module, mapped to the project dependency that has to be installed for
#: it to run. `json` is the one reader no dependency declaration can bound, and is listed because the
#: parity cell has to be able to find it at a call site.
ORACLE_IMPORTS = {
    "yaml": "PyYAML",
    "ruamel.yaml": "ruamel.yaml",
    "orjson": "orjson",
    "pyjson5": "pyjson5",
    "json5": "json5",
    "tomli": "tomli",
    "tomlkit": "tomlkit",
    "json": "json",
    "tomllib": "tomllib",
}

#: Readers that ship with the interpreter, and the version that introduced each. These are bounded by
#: the interpreter, not by `pyproject.toml`, and the question is answered against the floor the project
#: states: `tomllib` arrives with 3.11, so it cannot vouch for a `requires-python = ">=3.8"` package,
#: while `json` predates every tested leg. Neither is answered through `_dependency_marker`, whose
#: search matches any requirement that *starts with* the name - `_dependency_marker("json")` answers
#: `python_version < '3.9'`, from `jsonschema>=4.18.0,<4.25.1`, and would report the always-present
#: reader as bounded by a package nobody imports here.
STDLIB_SINCE = {"json": (3, 0), "tomllib": (3, 11)}

_IMPORT_LINE = re.compile(
    r"^\s*import ([a-z_][a-z0-9_]*(?:\.[a-z_][a-z0-9_]*)*)(?: as ([a-z_][a-z0-9_]*))?",
    re.IGNORECASE | re.MULTILINE,
)

_FLOOR_LINE = re.compile(r'requires-python\s*=\s*">=\s*(\d+)\.(\d+)')


def _declared_floor() -> tuple:
    """The interpreter this package promises, read off `requires-python`.

    A gate that cannot find the floor it is comparing readers against has no answer, so it says so
    rather than defaulting: a remembered `(3, 8)` would keep reporting `tomllib` as reaching the floor
    after the floor moved, which is the same class of stale constant this file exists to remove.
    """
    match = _FLOOR_LINE.search(read(REPO / "pyproject.toml"))
    if match is None:
        raise ValueError("pyproject.toml states no `requires-python >= X.Y` floor to read readers against")
    return int(match.group(1)), int(match.group(2))


def _importorskip_locals(text: str) -> list:
    """`(distribution, local name)` for readers a test asks for inside its own body.

    Read off the syntax, not the text: `tomllib` is not imported at module level anywhere, it is bound
    by `tomllib = pytest.importorskip("tomllib")` in the test that needs it, and a reader introduced
    that way is exactly as real - and as skippable - as one imported at the top.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return []
    found = []
    for node in ast.walk(tree):
        call = None
        local = None
        if isinstance(node, ast.Call):
            call = node
        elif isinstance(node, ast.Assign) and isinstance(node.value, ast.Call):
            call = node.value
            if len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                local = node.targets[0].id
        if call is None:
            continue
        func = call.func
        if (func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")) != "importorskip":
            continue
        if not call.args or not isinstance(call.args[0], ast.Constant):
            continue
        requested = call.args[0].value
        if not isinstance(requested, str):
            continue
        distribution = ORACLE_IMPORTS.get(requested)
        if distribution is not None:
            found.append((distribution, local or requested))
    return found


def _oracle_locals(text: str) -> dict:
    """`{distribution: [the names the module calls it by]}` for the outside readers it imports.

    Aliases are resolved because a call site writes the alias: `import yaml as pyyaml` is PyYAML and
    `_ruamel` is ruamel.yaml, and crediting a dialect has to find the reader where the test touches it.
    """
    found: dict[str, list[str]] = {}
    for match in _IMPORT_LINE.finditer(text):
        distribution = ORACLE_IMPORTS.get(match.group(1))
        if distribution is None:
            continue
        found.setdefault(distribution, []).append(match.group(2) or match.group(1))
    for distribution, local in _importorskip_locals(text):
        locals_of = found.setdefault(distribution, [])
        if local not in locals_of:
            locals_of.append(local)
    return found


def _dependency_marker(package: str) -> str | None:
    """The marker `pyproject.toml` puts on a package, or `None` when it is declared unconditionally.

    Names are matched the way PEP 503 normalizes them - `-`, `_` and `.` are interchangeable and case
    does not matter - because the import a test writes and the distribution a lock declares are two
    different spellings of one thing. Measured before this: `ruamel.yaml` answered the import name,
    the test group spells the requirement `ruamel-yaml`, and the miss was reported as `undeclared` -
    the column that claims to read how a dependency is bounded instead reported an oracle that is
    declared unconditionally as one that is not declared at all.

    A name that merely starts with this one is a different distribution: `tomli-w` is not `tomli`, so
    the character after the normalized name may not be a separator. Without that boundary the search
    for `tomli` answers the docs group's `tomli-w` requirement, and the marker it reports belongs to
    a package no test imports.
    """
    parts = [re.escape(part) for part in re.split(r"[-_.]+", package)]
    pattern = '"(' + r"[-_.]+".join(parts) + r')(?![-_.])[^;"]*(;([^"]*))?"'
    for match in re.finditer(pattern, read(REPO / "pyproject.toml"), re.IGNORECASE):
        return match.group(3).strip() if match.group(3) else None
    return "undeclared"


def _leg_restricted(marker: str | None) -> bool:
    """Does this marker keep the oracle off some supported interpreter or architecture?

    `partial` has to mean something measured: an unbounded dependency is installed on every leg, so the
    parity comparison runs everywhere, while one carrying `python_version < '3.14'` or a
    `platform_machine != ...` exclusion is absent - and its tests skipped - on those legs.
    """
    if not marker:
        return False
    return "python_version" in marker or "platform_machine" in marker


def _reader_bounded(name: str) -> bool:
    """Can this reader be missing from a leg the matrix tests?

    Two independent sources say no: a `pyproject.toml` marker, and the interpreter itself. Measured
    before the second was consulted, `tests/test_toml_crosslib.py` ran 13 of its 24 comparisons and skipped 11 on
    the floor leg while the cell read `yes`, because the reader it skipped - `tomllib`, new in 3.11 -
    was neither declared nor marker-bounded, and so was never asked about.
    """
    since = STDLIB_SINCE.get(name)
    if since is not None:
        return since > _declared_floor()
    return _leg_restricted(_dependency_marker(name))


def _reader_bound(name: str):
    """How this reader is bounded, in the words the row reports, or `None` when nothing bounds it."""
    since = STDLIB_SINCE.get(name)
    if since is None:
        return _dependency_marker(name)
    floor = _declared_floor()
    if since <= floor:
        return None
    return f"arrives with python {since[0]}.{since[1]}, above the {floor[0]}.{floor[1]} floor"


#: The shapes a comparison can be switched off by, inside a body. A decorator is judged on its own
#: (below), because a skip marker reached through a name - `@needs_pyjson5` - carries no such word.
SKIP_SHAPES = ("skipif", "xfail", "importorskip", "pytest.skip(")

#: The dialect a pyrs call asks for, read off the loader's own name. The alternation is ordered so
#: `load_json5` cannot be credited to the narrower `json`. The unsuffixed verbs are YAML's own:
#: `test_yaml_crosslib.py` compares through `safe_load`, `safe_dump` and `parse`, which carry no
#: dialect suffix because YAML is the default format.
DIALECT_CALL = re.compile(r"pyrs_yaml\.(?:[a-z0-9_]*_(json5|jsonc|json|toml|yaml)|safe_load|safe_dump|parse)\b")


def _dialect(match) -> str:
    """The dialect a `DIALECT_CALL` match asks for: the suffix, or `yaml` for the default verbs."""
    return match.group(1) or "yaml"


def _test_units(text: str) -> tuple:
    """Each `def test_*` as `([decorator sources], body)`, and whether the whole module is gated.

    Parsed rather than sliced by line: a unit ends at its own last statement, so a constant or a
    helper defined after the test is not read as part of it - measured on the JSON module, where the
    next block after a test held an oracle call and would have handed that test a reader it never
    used. A docstring that mentions a reader is excluded the same way, and a module that does not
    parse is reported as gated: an unreadable file cannot vouch for a tier.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return [], True
    lines = text.splitlines()
    units = []
    for node in tree.body:
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) or not node.name.startswith("test"):
            continue
        decorators = [ast.get_source_segment(text, item) or "" for item in node.decorator_list]
        start = node.body[0].lineno - 1
        first = node.body[0]
        if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) and isinstance(first.value.value, str):
            start = first.end_lineno  # past the docstring, which is prose about calls, not a call
        units.append((decorators, "\n".join(lines[start : node.end_lineno])))
    blanket = any(
        isinstance(node, ast.Assign) and any(getattr(target, "id", "") == "pytestmark" for target in node.targets)
        for node in tree.body
    )
    return units, blanket


def _probe_helpers(text: str, free: list) -> set:
    """Module-level functions whose whole answer is whether a free reader is installed.

    `tests/test_yaml_crosslib.py` guards its ruamel comparisons with
    `skipif(not _ruamel_available())`, and the helper's body is `return _ruamel is not None`. Resolving
    that name is what separates a guard that cannot fire on a matrix leg from one that can.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return set()
    names = set()
    for node in tree.body:
        if not isinstance(node, ast.FunctionDef) or node.args.args or node.args.kwonlyargs:
            continue
        if node.args.vararg or node.args.kwarg:
            continue
        body = node.body
        first = body[0] if body else None
        if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) and isinstance(first.value.value, str):
            body = body[1:]
        if len(body) != 1 or not isinstance(body[0], ast.Return) or body[0].value is None:
            continue
        compare = body[0].value
        if not isinstance(compare, ast.Compare) or len(compare.ops) != 1:
            continue
        if not isinstance(compare.ops[0], (ast.Is, ast.IsNot)):
            continue
        sides = [compare.left, *compare.comparators]
        if not any(isinstance(side, ast.Constant) and side.value is None for side in sides):
            continue
        if any(isinstance(side, ast.Name) and side.id in free for side in sides):
            names.add(node.name)
    return names


def _decoration_gates(decorator: str, free: list, probes: set) -> bool:
    """Can this decorator drop its unit on a leg where every free reader is installed?

    A `skipif` on the availability of a reader nothing bounds cannot: `pyproject.toml` installs that
    reader on every leg, so the comparison runs everywhere and the guard is dead while the cell is
    green. Anything else can gate - an interpreter version, a bounded reader, a module-level flag - and
    a decorator that is not `parametrize` is a skip marker wearing a name.
    """
    if "parametrize" in decorator:
        return False
    try:
        expr = ast.parse(decorator.strip(), mode="eval").body
    except SyntaxError:
        return True
    if not (isinstance(expr, ast.Call) and isinstance(expr.func, ast.Attribute) and expr.func.attr == "skipif"):
        return True
    if not expr.args:
        return True
    allowed = set(free) | probes
    for node in ast.walk(expr.args[0]):
        if isinstance(node, ast.Name) and node.id not in allowed:
            return True
        if isinstance(node, ast.Attribute):
            return True
        if isinstance(node, ast.Call) and not (isinstance(node.func, ast.Name) and node.func.id in probes):
            return True
    return False


def _credited(decorators: list, body: str, free: list, probes: set | None = None) -> bool:
    """Does this unit compare its dialect against a reader that no leg is missing?

    Credit needs a call to a reader no marker bounds, in a test whose decorators cannot gate it.
    `probes` is the parity cell's reading and there is no default: a set resolves the availability
    guards of free readers, which are dead, while `None` is the authority column's stricter reading,
    where any decorator but `parametrize` denies because a skip there can be the one that removes a
    reader from a leg. A denied credit leaves a hole registered, which is the safe error, while an
    undetected one closes a hole that is open.
    """
    if any(shape in body for shape in SKIP_SHAPES):
        return False
    if probes is None:
        gated = any("parametrize" not in item for item in decorators)
    else:
        gated = any(_decoration_gates(item, free, probes) for item in decorators)
    if gated:
        return False
    return any(re.search(rf"\b{re.escape(local)}\.\w+\(", body) for local in free)


def crosslib_parity(label: str, module) -> dict:
    """The parity cell of one family: which readers it asks, and which dialects they cover everywhere.

    `partial` used to be a property of the family, decided by the markers of its readers alone. That
    cannot describe a family whose dialects have different readers, and the JSON family is exactly
    that: RFC 8259 is asked of `json`, which every leg has, while JSONC and JSON5 are asked of
    `pyjson5`, which `pyproject.toml` keeps off the floor interpreter. So the cell is per dialect, and
    a dialect counts as covered everywhere when some test asks it of an unbounded reader in a unit
    nothing can skip.

    The unit rule is asked of every module, not only the ones with a bounded reader. The cell used to
    return on the bounded-reader check before reading a single unit, so a `test_*_crosslib.py` whose
    every test carried `@pytest.mark.skip` read `yes` when all of its readers were installed
    everywhere. A `skipif` naming the availability of a reader nothing bounds is not a gate - that
    reader is installed on every leg - which is why `test_yaml_crosslib.py` keeps its `yes`;
    `skipif(sys.version_info < ...)` still denies.
    """
    text = read(module)
    readers = _oracle_locals(text)
    oracles = sorted(readers)
    bounded = {name for name in oracles if _reader_bounded(name)}
    markers = {name: _reader_bound(name) for name in oracles}

    units, blanket = _test_units(text)
    free = [local for name, aliases in readers.items() if name not in bounded for local in aliases]
    probes = _probe_helpers(text, free)
    covered: set = set()
    if not blanket:
        for decorators, body in units:
            if _credited(decorators, body, free, probes):
                covered.update(_dialect(match) for match in DIALECT_CALL.finditer(body))
    dialects = [_slug(part) for part in label.split("/") if _slug(part)]
    restricted = sorted(name for name in dialects if name not in covered)
    return {
        "verdict": "partial" if restricted else "yes",
        "oracles": oracles,
        "restricted": restricted,
        "covered": sorted(covered),
        "marker": markers,
    }


#: A test file that asks the format's own grammar, or an official suite's cases, against an authority
#: this project does not own. Distinct from the cross-library parity column: parity compares two
#: implementations and answers "do we agree", these answer "are we right", and only one of the two can
#: find a syntax the reader never accepted - a path that does not exist has no second implementation to
#: disagree with, and no property or fuzz oracle to fail.
AUTHORITY_SUFFIXES = ("_suite.py", "_spec_table.py")

#: Where the official corpora live. A no-reader authority file is credited by the corpus it replays,
#: and every corpus suite names this root in a string it hands to its loader.
CORPUS_ROOT = "Reference/"


def _names_corpus(text: str) -> bool:
    """Does this module name the corpus it replays, rather than only carrying a suite's name?

    A string constant is what the loader receives; a comment that mentions the corpus is not a load.
    The suffix alone used to be the credit, so an empty `*_suite.py` measured `yes`.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False
    return any(
        isinstance(node, ast.Constant) and isinstance(node.value, str) and CORPUS_ROOT in node.value
        for node in ast.walk(tree)
    )


def _blanket_gate(text: str) -> str:
    """The source of a module-level gate: `pytestmark = ...` or `x = pytest.importorskip(...)`.

    `_test_units` reduces this to a boolean. The authority cell needs the condition as text, because
    two kinds of module gate mean opposite things: `skipif(not SUITE_DIR.is_dir())` skips on a corpus
    checkout the CI performs, while `skipif(pyjson5 is None)` skips wherever a reader is missing - and
    that second shape is what let the JSON5 spec table read as coverage on the 3.8 floor.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return text
    parts = []
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(getattr(target, "id", "") == "pytestmark" for target in node.targets):
            parts.append(ast.get_source_segment(text, node) or "")
        value = node.value if isinstance(node, (ast.Assign, ast.AnnAssign)) else None
        if not isinstance(value, ast.Call):
            continue
        func = value.func
        name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")
        if name == "importorskip":
            parts.append(ast.get_source_segment(text, node) or "")
    return "\n".join(parts)


def _uses_reader(text: str, local: str) -> bool:
    """Does this module reach into the reader, rather than only importing it?

    The pattern this denies: a stale `import json` left beside comparisons that all ask a bounded
    reader, which credited the file for an authority its tests never touch. Reaching into the
    reader's API is the use - `json.loads` in a test body, or the same attribute wired into a
    module-level table (`ROUTES = {..., "to_json": json.loads}`) that a parametrized test calls
    through. A bare name is not: `json is None` is a capability probe, and `@skipif(reader is None)`
    or its marker definition reaches for the reader exactly when the test will not run.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False
    root = local.split(".")[0]
    decorators = {
        id(node)
        for parent in ast.walk(tree)
        for decorator in getattr(parent, "decorator_list", ())
        for node in ast.walk(decorator)
    }
    for node in ast.walk(tree):
        if id(node) in decorators:
            continue
        if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == root:
            return True
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == local:
            return True
    return False


def _gate_references(source: str, locals_of: list[str]) -> bool:
    """Does a module-level gate *name* one of the readers, rather than merely contain its letters?

    The check used to be `local in gate` on the gate's source text, so a corpus gate whose reason
    carried the suite's URL (`.../json-schema-org/JSON-Schema-Test-Suite`) was read as a gate on the
    `json` reader and lost its credit. A gate that cannot run without a reader mentions it as a
    name - `pyjson5 is None` - so only identifiers and attribute roots count.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return any(local in source for local in locals_of)
    roots = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            roots.add(node.id)
        elif isinstance(node, ast.Attribute):
            base = node
            while isinstance(base, ast.Attribute):
                base = base.value
            if isinstance(base, ast.Name):
                roots.add(base.id)
    return any(local.split(".")[0] in roots for local in locals_of)


def authority_cell(path) -> dict:
    """Whether this file asks the format's rules of an authority that runs on every tested leg.

    Two kinds of authority exist in this tree: a reader the tests call - `json`, PyYAML, `json5` - and
    a corpus committed under `Reference/`. A corpus file has no reader to be absent, so the corpus it
    names is the credit; the suffix alone let an empty suite read `yes`. A reader file is credited
    only when the module evaluates a reader nothing bounds, or, if every reader is bounded, when a
    call site no skip guards still asks one. Importing alone is not credit: a free reader's import
    can outlive the comparisons that used it, and the file then reads as coverage while its tests ask
    only a reader some leg lacks. The shape this measurement
    exists for is the one it denies: `test_json5_spec_table.py` imported only `pyjson5`, whose
    markers keep it off the 3.8 floor and off 3.14+, and then gated the entire module on that import -
    so on those legs the JSON5 grammar was judged by nothing while the column still read a file name.
    """
    text = read(path)
    readers = _oracle_locals(text)
    bounded = sorted(name for name in readers if _reader_bounded(name))
    every_leg = [local for name, aliases in readers.items() if name not in bounded for local in aliases]
    locals_of = [local for aliases in readers.values() for local in aliases]
    gate = _blanket_gate(text)
    if gate and _gate_references(gate, locals_of):
        return {"verdict": "no", "readers": sorted(readers), "bounded": bounded}
    if not readers:
        return {"verdict": "yes" if _names_corpus(text) else "no", "readers": [], "bounded": []}
    if every_leg:
        used = any(_uses_reader(text, local) for local in every_leg)
        return {"verdict": "yes" if used else "no", "readers": sorted(readers), "bounded": bounded}
    units, _ = _test_units(text)
    live = any(_credited(decorators, body, locals_of) for decorators, body in units)
    return {"verdict": "yes" if live else "no", "readers": sorted(readers), "bounded": bounded}


def _unit_test_count(crate_dir: Path | None, scope: str | None) -> int:
    """The `#[test]`s that defend this row, counted in the files the row names.

    A row that owns its crate is measured by the whole crate. A row for a surface that shares
    one is measured by the file that declares it: the schema rule language lives in
    `pyrs-yaml-core` beside the YAML engine, and crediting that crate's tests to it would say
    the row is defended by hundreds of tests that never mention it.
    """
    if not crate_dir or not crate_dir.is_dir():
        return 0
    if scope:
        target = crate_dir / scope
        if not target.exists():
            return 0
        paths = [target] if target.is_file() else sorted(target.rglob("*.rs"))
    else:
        paths = sorted(crate_dir.rglob("*.rs"))
    return sum(len(re.findall(r"^\s*#\[test\]", read(path), re.MULTILINE)) for path in paths)


def _row_properties(props: list[str], row: FormatRow) -> list[str]:
    """The property functions this row's property cell counts.

    An unscoped row is credited by its crate or by a name fragment in the function name. A
    scoped row is credited only where the scope and the fragment agree: its own property has to
    live in the file that declares the surface and carry the name that identifies it.
    """
    credited = []
    for name in props:
        path, _, function = name.partition("::")
        if row.prop_scope is not None:
            if row.prop_scope in path and any(frag in function for frag in row.prop_fragments):
                credited.append(name)
        elif (row.crate and f"crates/{row.crate}/" in path) or any(frag in function for frag in row.prop_fragments):
            credited.append(name)
    return credited


def format_matrix() -> dict:
    """The per-format tier table, derived from the files that declare the defence.

    The point of deriving it is not tidiness. The hand-written table was wrong in cells that only a
    measurement finds: the `property` column rested on a Rust-only count of `prop_*` functions while the
    dialect rows are actually held up by the Python tier (`tests/test_property_dialects.py`, 10 `@given`
    cases with stdlib `json`/`tomllib` and optional `pyjson5` as external oracles).
    """
    targets = fuzz_targets()
    props = property_functions()
    crosslib_dir = REPO / "tests"
    rows: dict[str, dict] = {}

    for row in FORMAT_ROWS:
        label, crate, fragment = row.label, row.crate, row.fragment
        crate_dir = REPO / "crates" / crate if crate else None
        unit = _unit_test_count(crate_dir, row.unit_scope)
        rust_props = _row_properties(props, row)

        python_props = []
        for path in sorted((REPO / "tests").glob("test_property*.py")):
            text = read(path)
            if "@given" in text and re.search(rf"(load|to|_){fragment}\b", text):
                python_props.append(path.name)

        fuzz_parse = sorted(name for name in targets if fragment in name and re.match(r"(parse|validate)", name))
        fuzz_round = sorted(name for name in targets if fragment in name and "roundtrip" in name)

        parity: dict = {"verdict": "n/a", "oracles": [], "restricted": []}
        if row.crosslib_stem:
            module = crosslib_dir / f"test_{row.crosslib_stem}_crosslib.py"
            if module.exists():
                parity = crosslib_parity(label, module)

        # Which dialects have their grammar asked of an outside authority, derived from the file names
        # so that adding one is measured rather than remembered. The JSON row counts
        # `test_json5_spec_table.py` because that row covers the whole JSON family. The cell carries a
        # verdict now: a file name alone is what let the spec table read as coverage while its whole
        # module skipped behind `pyjson5` on the floor.
        files = sorted(
            path
            for path in (REPO / "tests").glob("test_*.py")
            if fragment in path.name and path.name.endswith(AUTHORITY_SUFFIXES)
        )
        cells = {path.name: authority_cell(path) for path in files}
        authority = {
            "verdict": "yes" if cells and all(cell["verdict"] == "yes" for cell in cells.values()) else "no",
            "files": sorted(cells),
            "bounded": sorted(name for name, cell in cells.items() if cell["verdict"] == "no"),
        }

        rows[label] = {
            "unit": unit,
            "property_rust": len(rust_props),
            "property_python": python_props,
            "fuzz_parse": fuzz_parse,
            "fuzz_round_trip": fuzz_round,
            "cross_library": parity,
            "authority": authority,
            "writer": row.writer,
        }
    return rows


def _slug(label: str) -> str:
    """A format label as a hole-id name: `JSON / JSONC / JSON5` -> `json-jsonc-json5`."""
    return re.sub(r"[^a-z0-9]+", "-", label.lower()).strip("-")


def authority_gaps(rows: dict) -> list[str]:
    """The format labels with no authority file at all, slugified for a hole id.

    An empty column is a boundary the other tiers cannot cross by construction: unit and property
    tiers ask this engine what it believes, and parity needs a second implementation to disagree
    with. Reading it off `format_matrix()` keeps `holes: []` from being quoted as "no short board"
    while a cell in the same file quietly reads `no`.
    """
    return sorted(_slug(label) for label, row in rows.items() if not row["authority"]["files"])


def authority_file_gaps(rows: dict) -> list[tuple]:
    """`(hole name, file)` for an authority that runs only where its reader installs.

    The column used to be a list, so `test_json5_spec_table.py` was credited by its name while the
    whole module sat behind `pytestmark = skipif(pyjson5 is None)` and `pyjson5`'s markers keep it off
    the 3.8 floor and off 3.14+. A file is denied here when it reaches no reader every leg installs:
    every reader it asks is bounded and its call sites are gated, or the only unbounded reader is
    imported and never used. The name in the hole is the file, because that is the unit that has to
    gain an unbounded authority.
    """

    def slug(name: str) -> str:
        stem = Path(name).stem
        return _slug(stem[len("test_") :] if stem.startswith("test_") else stem)

    return sorted((slug(name), name) for row in rows.values() for name in row["authority"].get("bounded", []))


def parity_gaps(rows: dict) -> list:
    """The format labels whose cross-library column reads `partial`, with the dialects that cause it.

    `partial` is measured but only *described* unless it is emitted: a dialect whose outside reader is
    absent on some leg compares nothing there, and that leg reports the same green as one where the
    comparison ran. The names are dialects, not readers - a family can cover one of its three dialects
    everywhere and none of another - so the hole says which half of the family is unjudged.
    """
    return sorted(
        (_slug(label), row["cross_library"]["restricted"])
        for label, row in rows.items()
        if row["cross_library"]["verdict"] == "partial"
    )


def check_scripts() -> list[str]:
    return sorted(p.name for p in (REPO / "scripts").glob("check_*.py"))


def ir_harness_owners() -> list[str]:
    """Crates declaring an `ir_gate` bench target: the channels the gate can measure.

    Two since the binding layer was closed - `pyrs-yaml-core` (parse, serialize, the JSON and TOML
    bridges in both directions) and `pyrs-yaml` (AST-to-Python). Derived from the manifests rather
    than hardcoded, so adding a third crate adds scenarios to the comparison without editing this
    file.
    """
    return sorted(
        manifest.parent.name
        for manifest in (REPO / "crates").glob("*/Cargo.toml")
        if IR_GATE_TARGET_RE.search(read(manifest))
    )


def ir_harness_files() -> list[str]:
    """Crates that carry a `benches/ir_gate.rs`, whether or not anything builds it."""
    return sorted(path.relative_to(REPO).parts[1] for path in (REPO / "crates").glob("*/benches/ir_gate.rs"))


def ir_harness_channels() -> dict:
    """crate -> the scenario names its harness declares, for every crate the gate builds.

    The old probe read `crates/pyrs-yaml-core/benches/ir_gate.rs` by name, which is the same failure
    class as the graph probe that stopped at the first dependency section: when a second harness
    appeared, the scenarios only it lists would have been absent from the comparison, so the committed
    baseline could carry numbers nobody re-measured and the gate could not say a channel went missing.
    """
    channels = {}
    for crate in ir_harness_owners():
        path = REPO / "crates" / crate / "benches" / "ir_gate.rs"
        if not path.exists():
            channels[crate] = []
            continue
        text = read(path)
        body = text.split("fn scenarios()", 1)[-1].split("fn main()", 1)[0]
        # Scenario names are the lowercase string literals; the inputs are `SCREAMING_CASE`
        # constants and `Work::Variant` paths. Matching the whole tuple instead missed
        # `serialize_block_scalars`, whose name sits on its own line after a wrap.
        channels[crate] = sorted(set(re.findall(r'"([a-z_0-9]{4,})"', body)))
    return channels


def ir_scenarios_in_baseline() -> list[str]:
    return sorted(json.loads(read(REPO / ".ci" / "ir-baseline.json"))["scenarios"])


# Formats the hub AST is written *as* and read *back from*. `python` is deliberately absent:
# `to_python_*` is the language binding, not a text format, and it has no reader twin - the
# conversion runs once, in one direction, from bytes the YAML scenarios already measure.
CROSS_FORMAT_WRITERS = ("json", "jsonc", "json5", "toml", "yaml")


def ir_bridge_pairs(channels: dict) -> list[str]:
    """Cross-format scenarios that are measured in one direction only.

    `to_<format>_<size>` and `from_<format>_<size>` are the two halves of one bridge, and the
    halves cost different things: the writer decides how to *emit* a key, the reader decides what
    the key *means*. #292 adds per-key work to the reading half (`load_toml` has to quote a key
    that either YAML schema would re-type, or the conversion changes what the document says) and
    the gate had a number for the writing half only, so the cost of that decision was unmeasurable
    - which is how a change can be adjudicated on the wrong channel and closed.
    """
    names = {name for values in channels.values() for name in values}
    unpaired = []
    for name in sorted(names):
        parts = name.split("_")
        if len(parts) < 3 or parts[0] not in ("to", "from") or parts[1] not in CROSS_FORMAT_WRITERS:
            continue
        twin = "_".join(("from" if parts[0] == "to" else "to", *parts[1:]))
        if twin not in names:
            unpaired.append(name)
    return unpaired


def ir_bridges_absent(channels: dict) -> list[str]:
    """Formats the hub converts to and from that the instruction gate has no number for at all.

    `ir_bridge_pairs` can only notice a missing twin for a format that already has one half, so a
    format with neither half is invisible to it. That is how the JSONC and JSON5 paths - the comment
    scanner, the unquoted-key grammar, the two dialect writers - carried no instruction count while
    the gate printed green over the bridge it could see.
    """
    names = {name for values in channels.values() for name in values}
    absent = []
    for fmt in CROSS_FORMAT_WRITERS:
        if fmt == "yaml":
            continue  # the hub's own text, measured by `parse_*` and `serialize_*`, not a bridge
        halves = [
            name
            for name in names
            if len(name.split("_")) > 1 and name.split("_")[0] in ("to", "from") and name.split("_")[1] == fmt
        ]
        if not halves:
            absent.append(fmt)
    return absent


def rust_test_count() -> int:
    """`#[test]` attributes under `crates/`, the quantity the page calls Rust tests."""
    return sum(len(re.findall(r"^\s*#\[test\]", read(path), re.MULTILINE)) for path in (REPO / "crates").rglob("*.rs"))


def python_test_functions() -> int:
    """`def test_*` under `tests/`, before pytest's parametrisation multiplies them."""
    return sum(
        len(re.findall(r"^\s*(?:async )?def test_", read(path), re.MULTILINE))
        for path in (REPO / "tests").rglob("*.py")
    )


def _closing_brace(text: str, open_index: int) -> int:
    """The index of the brace that closes the one at `open_index`, or the end of the text."""
    depth = 0
    for index in range(open_index, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    return len(text)


def _property_spans(text: str) -> list:
    """`(offset, name)` for every proptest property in a file.

    Two shapes count, because the repository uses both: a function whose name starts with `prop_`,
    and a function declared inside a `proptest! { .. }` block whose header takes generated arguments and
    opens its body straight away (`fn name(x in strategy) {`). The block's other functions are not
    properties: a strategy helper returns (`-> impl Strategy`), and a function declared inside a
    property's body returns too - which is how `key_order`, `mapping_key_orders` and `tag_texts` in
    `crates/pyrs-yaml-core/src/pbt.rs` appear, and why counting every `fn` in the block measured 30
    where the tier holds 27. Dropping the second shape entirely was the original error: it missed the
    two properties in `crates/pyrs-event/src/lib.rs`, named in prose rather than with the prefix, so the
    page reported 25 while the defence held 27. A tool that counts a naming convention measures the
    convention, not the defence.
    """
    found = [(match.start(), match.group(1)) for match in re.finditer(r"\bfn (prop_[a-z0-9_]+)\s*\(", text)]
    known = {offset for offset, _ in found}
    for block in re.finditer(r"\bproptest\s*!\s*\{", text):
        brace = text.index("{", block.start())
        end = _closing_brace(text, brace)
        # proptest requires `#[test]` on every property it generates, and that is what separates a
        # property from the functions a property body declares inside it: `mapping_key_orders` in
        # `crates/pyrs-yaml-core/src/pbt.rs` is a nested helper that takes three plain Rust arguments
        # and returns unit, so neither "has parameters" nor "has no return type" excludes it, while the
        # attribute does. Doc comments may sit between the attribute and the header.
        pattern = re.compile(r"#\[test\]\s*(?:(?:///|[/*])[^\n]*\s*)*fn ([a-z0-9_]+)\(")
        for match in pattern.finditer(text[block.start() : end]):
            offset = block.start() + match.start()
            name = match.group(1)
            prop_match = re.search(r"\bfn " + re.escape(name) + r"\s*\(", text[offset:])
            if prop_match is None:
                continue
            start = offset + prop_match.start()
            if start not in known:
                known.add(start)
                found.append((start, name))
    return sorted(found)


def property_functions() -> list[str]:
    names = []
    for rs in sorted((REPO / "crates").rglob("*.rs")):
        for _, name in _property_spans(read(rs)):
            names.append(f"{rs.relative_to(REPO).as_posix()}::{name}")
    return sorted(names)


_GUARDED_ASSERT = re.compile(r"if let Some\([^=]*=\s*try_[a-z0-9_]+\(")


def conditional_property_assertions(root: Path | None = None) -> list[str]:
    """Property functions whose assertion only runs when a `try_*` helper returned `Some`.

    A body written `if let Some(parsed) = try_roundtrip(&node) { prop_assert!(...) }` states nothing
    about the samples the helper declined, so the property's coverage is whatever survives the filter.
    That is not hypothetical: measured 2026-10-14 on `crates/pyrs-yaml-core/src/pbt.rs`, the round-trip
    helper filtered 3423 of 4000 drawn trees, and the tree-changing defect that hid behind it emitted a
    mapping one entry shorter than the one it was given.

    Each slice runs from one property header to the next, over the same set `_property_spans` returns,
    so a guard belongs to the function it is written in, the helper definitions between properties
    cannot be mistaken for one, and a property named in prose inside a `proptest!` block is examined
    like every other.
    """
    base = root or REPO
    found: list[str] = []
    for rs in sorted((base / "crates").rglob("*.rs")):
        text = read(rs)
        headers = _property_spans(text)
        for index, (start, name) in enumerate(headers):
            end = headers[index + 1][0] if index + 1 < len(headers) else len(text)
            if _GUARDED_ASSERT.search(text[start:end]):
                found.append(f"{rs.relative_to(base).as_posix()}::{name}")
    return sorted(found)


DOCS_LOCALES = ("en", "ja", "ko", "zh")


def mkdocstrings_declared(root: Path | None = None) -> bool:
    """Whether the site configuration declares the Python handler at all."""
    base = root or REPO
    config = base / "zensical.toml"
    if not config.is_file():
        return False
    return bool(re.search(r"^\[project\.plugins\.mkdocstrings\.handlers\.python\]", read(config), re.M))


def mkdocstrings_directives(root: Path | None = None) -> list[str]:
    """Every published page that asks the handler for something, i.e. contains a `:::` directive.

    A configured handler is not a used one. This distinction is what makes the measurement worth its
    cost: `zensical.toml` declares the Python handler, its load paths and a page of render options,
    while the docs tree contains no directive - so those options are inert, the API reference is
    hand-typed signatures, and no dependency bump can be checked against what it was supposed to
    improve. Indented directives count because a `:::` block inside an admonition or a tab is still a
    directive; `:::` with no identifier after it is a Markdown divider, not a request.
    """
    base = root or REPO
    found = []
    for locale in DOCS_LOCALES:
        directory = base / "docs" / locale
        if not directory.is_dir():
            continue
        for page in sorted(directory.rglob("*.md")):
            if re.search(r"^\s*:::\s+\S", read(page), re.M):
                found.append(page.relative_to(base).as_posix())
    return found


def renders_the_site() -> bool:
    """Whether a workflow that runs on a pull request also runs the documentation build.

    Asked of the declaring files rather than of a list kept beside them: a job counts only if its own
    workflow triggers on `pull_request` *and* one of its `run:` steps invokes `scripts/build-docs.py`.
    Mentioning the script in a comment, or in a job that only runs on push, is not a gate - which is the
    distinction that let four changelog pages carry unreadable metadata through three merges while
    `.github/workflows/docs.yml` printed green on every push.
    """
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        text = read(workflow)
        triggers = text.split("jobs:")[0]
        if "pull_request" not in triggers:
            continue
        if any("build-docs.py" in command for command in run_commands(text)):
            return True
    return False


def declared_artifacts() -> list[tuple[str, bool]]:
    """Every repository path this repo's own prose promises exists, and whether it does.

    `ROADMAP.md` and `QUALITY_MATRIX.md` are written in backticked file names, and they
    record gates as delivered (``note_survival.rs`` runs on every PR). When the ledger and
    the tree disagree the ledger wins in the reader's head, which is the failure this probe
    exists for: it was found by noticing that this repository's own ledger described a
    regression tier that had been reverted by a later commit.
    """
    found: dict[str, bool] = {}
    for rel in ("ROADMAP.md", "QUALITY_MATRIX.md"):
        text = read(REPO / rel)
        for token in re.findall(r"`([^`]+)`", text):
            # A backticked span often carries a path plus prose (`crates/x/tests/y.rs runs
            # on every PR`); take the first whitespace-delimited word of it.
            token = token.strip().split()[0] if token.strip() else ""
            # Strip trailing punctuation only: `.ci/quality-holes.json` and
            # `crates/pyrs-json/tests/roundtrip_corpus.rs` are both real paths whose leading
            # dot is part of the name, and an early version of this line reported them as
            # missing artifacts — a gate that invents holes is as bad as one that misses them.
            token = token.rstrip(".,:;")
            if "/" not in token or any(ch in token for ch in " *()…"):
                continue
            if not re.match(r"^[.,a-zA-Z0-9_-][^ ]*$", token):
                continue
            path = REPO / token
            if path.suffix not in {".rs", ".py", ".toml", ".yml", ".md", ".json", ".sh"}:
                continue
            if token not in found:
                found[token] = path.exists()
    return sorted(found.items())


def ir_harness_build_graph() -> set[str]:
    """Crates the reproducible instrument can actually reach: bench owner + its dependencies.

    `cargo bench --bench ir_gate` compiles the owning crate and what it links, so anything outside
    that graph is invisible to instruction-count gating no matter what the scenario is named.
    """
    crates: set[str] = set()
    for crate in ir_harness_owners():
        manifest = REPO / "crates" / crate / "Cargo.toml"
        text = read(manifest)
        crates.add(manifest.parent.name)
        # Every dependency section, not the first one. `re.search` here reported a three-crate graph
        # while `to_json_medium` / `to_toml_medium` link `pyrs-json` and `pyrs-toml` through
        # `[dev-dependencies]` - the hole's own text claimed five. Printing the derived set in the
        # summary (`ir_harness_crates`) is the only reason that mismatch was visible at all.
        for section in re.findall(r"^\[(?:dev-)?dependencies\](.*?)(?=^\[|\Z)", text, re.S | re.M):
            linked = re.findall(r"^([a-z0-9_-]+) *=", section, re.M)
            # Workspace members only: the point of the set is which of *this* repository's layers
            # the instrument can reach, and third-party links say nothing about that boundary.
            crates.update(name for name in linked if (REPO / "crates" / name).is_dir())
    return {name.replace("_", "-") for name in crates}


def binding_crate() -> str | None:
    """The crate that serves the Python API - the one layer a user actually calls into.

    Found by layout (`crates/*/src/py/`), not by name, so renaming the directory moves the answer
    instead of leaving a stale hard-coded crate behind.
    """
    for directory in sorted((REPO / "crates").glob("*/src/py")):
        if any(directory.glob("*.rs")):
            return directory.parent.parent.name
    return None


def changelog_mirror_counts() -> dict:
    """Per-mirror [Unreleased] entry counts, by section position, from the mirror checker itself.

    This calls `scripts/check_changelog_mirrors.py` instead of parsing the changelogs a second way:
    the hook and this probe have to agree about what counts as an entry, or the matrix reports clean
    while the gate that enforces the same rule disagrees (and vice versa).
    """
    path = REPO / "scripts" / "check_changelog_mirrors.py"
    spec = importlib.util.spec_from_file_location("check_changelog_mirrors", path)
    assert spec is not None and spec.loader is not None, f"cannot load {path}"
    checker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(checker)
    return {mirror.relative_to(REPO).as_posix(): checker.positioned_counts(read(mirror)) for mirror in checker.FILES}


def timing_floors_unpaired(root: Path | None = None) -> list[str]:
    """Test files that time wall-clock in ordinary CI without sampling their two phases together.

    Three of them did until this was measured: the TOML parse gate ran every candidate block before
    every reference block, and the TOML serialize and both JSON gates took a single block per side.
    A block is only comparable inside itself, so a scheduler spike on one phase flips the verdict on a
    step that really is twice as cheap - the recorded macos incident measured that pair at 1.10x. The
    shared sampler in `tests/timing.py` alternates the phases per block and reports a majority.

    CodSpeed-owned files are excluded: their timing is the product being measured, not a gate about
    another path, and they run on a pinned runner rather than alongside the whole matrix.
    """
    out = []
    for path in sorted(((root or REPO) / "tests").glob("test_*.py")):
        text = read(path)
        if "perf_counter" not in text:
            continue
        if "pytest.mark.benchmark" in text or "codspeed" in text.lower():
            continue
        if "from tests.timing import" not in text:
            out.append(f"tests/{path.name}")
    return out


def format_verdict_lines(rows: dict) -> list:
    """One rendered line per format, so `--json` consumers and the CLI see the same table."""
    lines = []
    for label, row in rows.items():
        property_yes = bool(row["property_rust"]) or bool(row["property_python"])
        parity = row["cross_library"]["verdict"]
        # A surface that owns no serializer has nothing to round-trip: the JSON Schema row judges a
        # document through a delegated engine, and the schema rule language resolves scalars.
        round_trip = "n/a" if not row["writer"] else ("yes" if row["fuzz_round_trip"] else "no")
        lines.append(
            f"{label}: unit={'yes' if row['unit'] else 'no'} property={'yes' if property_yes else 'no'} "
            f"fuzz-parse={'yes' if row['fuzz_parse'] else 'no'} round-trip={round_trip} "
            f"parity={parity} authority={row['authority']['verdict']}"
        )
    return lines


def _reference_binding_lines(text: str) -> list[str]:
    """Lines that bind a reference, comment lines excluded.

    Reachability has to mean "a step sets it": the first draft of this probe matched the
    whole file, so a workflow that only *talks about* binding a reference read as bound.
    """
    return [
        line.strip() for line in text.splitlines() if "PYQ_DIALECT_YQ" in line and not line.lstrip().startswith("#")
    ]


def dialect_reference_bound(root: Path | None = None) -> bool:
    """Whether a CI leg binds the reference binaries the pyq dialect tier compares against.

    That tier skips when the host has neither `jq` nor mikefarah `yq`, and a skip is
    invisible inside captured `cargo test` output, so what CI judges depends on the runner
    image. The test reads an explicit binding, which makes the premise measurable from the
    tree: a leg that sets it is a leg that compares.
    """
    workflows = (root or REPO) / ".github" / "workflows"
    if not workflows.is_dir():
        return False
    return any(_reference_binding_lines(read(path)) for path in sorted(workflows.glob("*.yml")))


def measure() -> dict:
    commands_by_workflow = workflow_commands()
    all_commands = " ".join(command for commands in commands_by_workflow.values() for command in commands)
    entries = prek_entries()
    hooks = prek_hooks()
    targets = fuzz_targets()
    ci_targets = fuzz_ci_matrix()
    engines = engine_crates(fuzz_crates())
    scripts = check_scripts()
    channels = ir_harness_channels()
    bench_scenarios = sorted({name for names in channels.values() for name in names})
    baseline_scenarios = ir_scenarios_in_baseline()
    props = property_functions()
    formats = format_matrix()
    guarded_props = conditional_property_assertions()
    seeds = fuzz_seed_inventory()
    rust_tests = rust_test_count()
    python_tests = python_test_functions()

    hygiene_path = WORKFLOWS / "hygiene.yml"
    skip_match = re.search(r"SKIP: ([A-Za-z0-9,_-]+)", read(hygiene_path)) if hygiene_path.exists() else None
    skipped = sorted(x.strip() for x in skip_match.group(1).split(",")) if skip_match else []

    holes = []

    for target in sorted(set(ci_targets) - set(targets)):
        holes.append(["fuzz-target-undeclared", target, "the CI matrix names a target fuzz/Cargo.toml does not build"])
    for target in sorted(set(targets) - set(ci_targets)):
        holes.append(["fuzz-target-uncovered", target, "a built target is never run by CI"])
    for crate in engines:
        if not roundtrip_target_for(crate, targets):
            holes.append(
                [
                    "fuzz-no-roundtrip",
                    crate,
                    "this engine has a writer, and a parse-only target cannot reach the round-trip invariant",
                ]
            )

    # The authority column is a boundary, not a score: parity answers "do we agree" and cannot find a
    # rule that both sides never implemented. Emitting a hole for an empty column is what keeps the
    # registry's meaning - `holes: []` says nothing is unmeasured, not that nothing is unjudged.
    for label in authority_gaps(formats):
        holes.append(
            [
                "authority-tier",
                label,
                "no test asks this surface's rules of something this repository does not own, so a grammar "
                "invented here and implemented here has nobody left to contradict it",
            ]
        )
    for name, file in authority_file_gaps(formats):
        holes.append(
            [
                "authority-tier",
                name,
                f"{file} asks the format's rules only of readers some tested leg does not install, and the "
                "tests that ask them can skip, so those legs judge the grammar with nothing while the "
                "column still reads a file name",
            ]
        )

    # A `partial` parity cell is a tier that runs on some legs and not others, and the skipped legs are
    # the green ones. Registering it keeps "the JSON family has a cross-library tier" from being read as
    # "on every leg this package promises", and names the dialects that do not hold that second half.
    for slug, restricted in parity_gaps(formats):
        holes.append(
            [
                "parity-tier",
                slug,
                "{} has no outside reader that every tested leg installs, so the legs that skip the "
                "comparison report the same green as the ones that ran it".format(", ".join(restricted)),
            ]
        )

    if "prek run --all-files" not in all_commands:
        holes.append(["hook-tier-unwired", "prek", "no CI job runs the hook set over the tree"])
    for hook in skipped:
        entry = entries.get(hook, "")
        command = entry.split(" --")[0] if entry else hook
        if command not in all_commands:
            holes.append(
                ["hook-skipped-uncovered", hook, "hygiene.yml skips it and no workflow runs its command either"]
            )

    for script in scripts:
        stem = script[: -len(".py")]
        if stem not in all_commands and stem not in read(REPO / "prek.toml"):
            holes.append(["orphan-gate", script, "a checker exists that no workflow command and no hook invokes"])

    if "PROPTEST_CASES" not in all_commands:
        holes.append(
            [
                "property-tier",
                "default-case-count",
                "every CI run uses proptest's default case count, so low-frequency shapes never surface",
            ]
        )

    if guarded_props:
        holes.append(
            [
                "property-tier",
                "conditional-assert",
                "a property that asserts only inside `if let Some(_) = try_*(..)` states nothing about "
                "the samples its helper filtered away, so the tier's coverage is what survives the filter",
            ]
        )

    if not dialect_reference_bound():
        holes.append(
            [
                "dialect-tier",
                "reference-unbound",
                "the pyq dialect tier compares against jq and mikefarah yq only on a host that "
                "carries them, and no CI leg binds a reference, so where the tools are missing "
                "the tier passes while judging nothing",
            ]
        )

    for scenario in sorted(set(bench_scenarios) - set(baseline_scenarios)):
        holes.append(["ir-unbaselined", scenario, "a measured scenario has no committed baseline number"])
    for scenario in sorted(set(baseline_scenarios) - set(bench_scenarios)):
        holes.append(["ir-stale-baseline", scenario, "the baseline carries a scenario the bench no longer defines"])

    # A bridge measured in one direction is a hole in the instrument rather than in the code, so it
    # is reported here instead of being asserted: the fix is a scenario, and until someone adds the
    # twin the gate should keep saying which half it cannot see.
    for scenario in ir_bridge_pairs(channels):
        holes.append(
            [
                "ir-bridge-unidirectional",
                scenario,
                "one half of a cross-format bridge has a scenario and the other does not, so per-key "
                "work on the missing half changes nothing the gate can observe",
            ]
        )

    for fmt in ir_bridges_absent(channels):
        holes.append(
            [
                "ir-bridge-absent",
                fmt,
                "the hub reads and writes this format, and the instruction gate has no scenario in "
                "either direction, so work added to that hot path cannot be adjudicated at all",
            ]
        )

    for rel in timing_floors_unpaired():
        holes.append(
            [
                "timing-floor-unpaired",
                rel,
                "a wall-clock gate in ordinary CI times its candidate and its reference in separate "
                "blocks, so one scheduler spike decides the verdict; sample the pair through "
                "tests/timing.py instead",
            ]
        )

    # The harness set is measured in both directions, because each half has already been wrong once.
    # A declared target whose file is missing leaves the gate silently shorter; a file whose crate
    # declares no target compiles never, so its scenarios would be numbers in a JSON file and nothing
    # else. And two harnesses naming one scenario makes the baseline ambiguous - `ir_gate.py` refuses
    # that at run time, and the probe refuses it here so the refusal is not the only defence.
    for crate in sorted(set(channels) - set(ir_harness_files())):
        holes.append(
            [
                "ir-harness-missing",
                crate,
                "the gate declares an ir_gate target for this crate, but benches/ir_gate.rs is not there",
            ]
        )
    for crate in sorted(set(ir_harness_files()) - set(channels)):
        holes.append(
            [
                "ir-harness-undeclared",
                crate,
                "a benches/ir_gate.rs exists in a crate with no `ir-gate` bench target, so nothing builds it",
            ]
        )
    owner_of: dict = {}
    for crate, names in channels.items():
        for name in names:
            if name in owner_of:
                holes.append(
                    [
                        "ir-scenario-duplicate",
                        name,
                        f"{owner_of[name]} and {crate} both list it, so one baseline number cannot say which was measured",
                    ]
                )
            else:
                owner_of[name] = crate

    ci_commands = " ".join(commands_by_workflow.get("ci.yml", []))
    if "--all-targets" not in ci_commands:
        holes.append(
            ["lint-scope", "clippy-all-targets", "CI's clippy command does not lint tests, benches or examples"]
        )

    # Whether any pull request *renders* the published pages. The site is built by a workflow that
    # triggers on push to `main`, so every property that only a generator decides - a metadata block it
    # must parse, Markdown inside an HTML block, an anchor a locale translated away - is discovered by
    # whoever opens the deployed page. Measured rather than asserted, because the alternative is a claim
    # about coverage that no file makes.
    if not renders_the_site():
        holes.append(
            [
                "docs-rendering",
                "unbuilt-on-pr",
                "no pull-request job builds the site, so a page that fails only when rendered reaches main",
            ]
        )

    # Whether the Python docstring handler is actually asked for anything, as opposed to merely declared
    # with a load path and a page of render options. Measured because the difference decides whether the
    # docs toolchain does work at all: a bump of `zensical` or `mkdocstrings-python` cannot improve
    # generated quality when nothing is being generated, and hand-typed API signatures are the one
    # artefact class this repository has repeatedly had to correct against the code.
    if mkdocstrings_declared() and not mkdocstrings_directives():
        holes.append(
            [
                "docs-generation",
                "plugin-unused",
                "the python handler is declared with render options and no page uses `:::`, so the API reference is hand-typed",
            ]
        )

    # The ledger and this document promise files by name (`note_survival.rs` runs on every
    # PR). A promise that outlives the thing it promises is the quietly-worse kind of stale:
    # the reader trusts the gate exists. So every repository path named in backticks in
    # those two files has to exist, and a missing one is a hole like any other.
    declared = declared_artifacts()
    for path, exists in declared:
        if not exists:
            holes.append(
                [
                    "artifact-missing",
                    path,
                    "the ledger or the assessment promises this file, and the tree does not contain it",
                ]
            )

    # The two YAML writers (`Serializer` over parsed nodes, `direct_dump` over Python
    # objects) mirror each other by design instead of sharing code, which is precisely
    # how #287 ended up needing the same fix twice. A probe that only asked "does any
    # test mention the fast path" passed while no test compared the two routes, so the
    # requirement is the table itself: a module whose name says it holds the parity
    # table. Adding a writer route without extending that table is then a red test.
    route_parity = sorted(p.name for p in (REPO / "tests").glob("test_route_parity*.py"))
    if not route_parity:
        holes.append(
            [
                "route-parity",
                "node-writer-vs-direct-dump",
                "no table-driven test compares the two YAML writer routes on shared shapes",
            ]
        )

    # The instruction gate is the only reproducible instrument in the set (wall time swings by
    # points the Ir gate resolves to 0.0018%), so which crates it can *compile against* is the
    # boundary of what can be gated at all. Derived from the bench target's own manifest rather
    # than from a list of scenario names: the binding layer is where `safe_load` turns an AST into
    # Python objects, it is what #292 changed, and it appears in no scenario because no scenario
    # can - the harness never links that crate. Wall-time CodSpeed covers it, at the precision this
    # repository's own notes call unusable below ~10%.
    ir_graph = ir_harness_build_graph()
    served_by = binding_crate()
    if served_by and served_by not in ir_graph:
        holes.append(
            [
                "perf-coverage",
                "binding-layer",
                f"the instruction gate compiles against {sorted(ir_graph)} and never reaches {served_by}, "
                "so the AST-to-Python conversion the package is used for has no reproducible perf number",
            ]
        )

    # `AGENTS.md` says never commit partial changelog updates, and section positions are
    # locale-independent, so equal [Unreleased] counts at the same positions are the measurable form
    # of the rule. `check_changelog_mirrors.py` now asserts that instead of printing it, which closed
    # the registered hole this block used to derive; the counts stay reported here because a measure the
    # probe publishes is a measure a test can fire on.
    mirrors = changelog_mirror_counts()

    return {
        "ir_harness_crates": sorted(ir_graph),
        "ir_harness_owners": sorted(channels),
        "python_serving_crate": [served_by] if served_by else [],
        "engines_with_serializer": engines,
        "fuzz_targets": targets,
        "fuzz_ci_matrix": ci_targets,
        "fuzz_seed_count": seeds["seeds"],
        "fuzz_former_crash_count": seeds["former_crash"],
        "rust_test_count": rust_tests,
        "python_test_function_count": python_tests,
        "prek_hooks": hooks,
        "hooks_skipped_in_ci": skipped,
        "check_scripts": scripts,
        "ir_scenarios_bench": bench_scenarios,
        "ir_scenario_channels": [f"{crate}:{len(names)}" for crate, names in sorted(channels.items())],
        "ir_scenarios_baseline": baseline_scenarios,
        "property_function_count": len(props),
        "property_functions": props,
        "property_conditional_assertions": guarded_props,
        "artifact_paths": {"scanned": len(declared), "missing": sum(1 for _p, ok in declared if not ok)},
        "changelog_entry_counts": [f"{name}={counts}" for name, counts in sorted(mirrors.items())],
        "format_matrix": formats,
        "format_verdicts": format_verdict_lines(formats),
        "route_parity_tests": route_parity,
        "holes": sorted(holes),
    }


SUMMARY_KEYS = (
    "ir_harness_crates",
    "ir_harness_owners",
    "python_serving_crate",
    "engines_with_serializer",
    "fuzz_targets",
    "fuzz_ci_matrix",
    "fuzz_seed_count",
    "fuzz_former_crash_count",
    "rust_test_count",
    "python_test_function_count",
    "prek_hooks",
    "hooks_skipped_in_ci",
    "check_scripts",
    "ir_scenarios_bench",
    "ir_scenario_channels",
    "ir_scenarios_baseline",
    "property_function_count",
    "property_conditional_assertions",
    "route_parity_tests",
    "format_verdicts",
    "changelog_entry_counts",
)


#: The count-bearing sentences of `QUALITY_MATRIX.md`, each with the numbers the probe owns.
#:
#: The page was pinned to the measurement by a test that compared a parsed number with the probe, so a
#: stale row is a red run rather than a lie - which is also how PR #549 came to redden all 14 test legs:
#: the author had to *type* the new number. These rules let the same code write the row it checks, so
#: growing a tier costs one flag instead of a hand-transcribed digit that two open pull requests then
#: disagree about.
#:
#: Each rule keeps the prose around the number: group 1 is the text before it, the last group the text
#: after, and everything in between is what this file recomputes.
ROW_RULES = (
    (
        "unit / integration row",
        re.compile(r"^(\| unit / integration \| )([\d,]+) Rust `#\[test\]`, ([\d,]+) Python test functions( \|)", re.M),
        lambda data: "{:,} Rust `#[test]`, {:,} Python test functions".format(
            data["rust_test_count"], data["python_test_function_count"]
        ),
    ),
    (
        "property row",
        re.compile(r"^(\| property \| )(\d+)( proptest properties)", re.M),
        lambda data: "{:d}".format(data["property_function_count"]),
    ),
    (
        "fuzz row",
        re.compile(r"^(\| fuzz \| )(\d+) targets, (\d+) seeds of which (\d+)( are `former-crash-\*`)", re.M),
        lambda data: "{:d} targets, {:d} seeds of which {:d}".format(
            len(data["fuzz_targets"]), data["fuzz_seed_count"], data["fuzz_former_crash_count"]
        ),
    ),
    (
        "section-4 former-crash sentence",
        re.compile(r"(former-crash-<hash>\.seed` \()(\d+)( of them\))"),
        lambda data: "{:d}".format(data["fuzz_former_crash_count"]),
    ),
)


def _rebuilt(match: re.Match, pattern: re.Pattern, expected: str) -> str:
    """The sentence with its number replaced by the measurement, prose around it untouched.

    Group 1 is everything before the number and the last group everything after it, which is how
    `ROW_RULES` can rewrite a count without restating the sentence that carries it.
    """
    return match.group(1) + expected + match.group(pattern.groups)


def row_drift(text: str, data: dict) -> list:
    """How each count-bearing sentence differs from what the probe measures.

    A rule whose pattern no longer matches is reported as drift too: the silent failure here is a page
    whose table is reworded until no number is checked at all, which is the hole this file exists to
    close.
    """
    problems = []
    for label, pattern, expected_fn in ROW_RULES:
        match = pattern.search(text)
        if match is None:
            problems.append(f"{label}: no sentence matches, so the page no longer states this count")
            continue
        rebuilt = _rebuilt(match, pattern, expected_fn(data))
        if rebuilt != match.group(0):
            problems.append(f"{label}: page reads {match.group(0)!r}, the probe measures {rebuilt!r}")
    return problems


def apply_rows(text: str, data: dict) -> tuple:
    """Rewrite the count-bearing sentences from the measurement. Returns (new text, changed labels)."""
    changed = []
    for label, pattern, expected_fn in ROW_RULES:
        match = pattern.search(text)
        if match is None:
            continue
        rebuilt = _rebuilt(match, pattern, expected_fn(data))
        if rebuilt == match.group(0):
            continue
        text = text[: match.start()] + rebuilt + text[match.end() :]
        changed.append(label)
    return text, changed


def main(argv: list[str]) -> int:
    if "--check-rows" in argv or "--write-rows" in argv:
        data = measure()
        page = MATRIX_DOC
        text = page.read_text(encoding="utf-8")
        problems = row_drift(text, data)
        if "--write-rows" in argv:
            new_text, changed = apply_rows(text, data)
            remaining = row_drift(new_text, data)
            if remaining:
                for problem in remaining:
                    print(f"cannot write: {problem}", file=sys.stderr)
                return 2
            if new_text != text:
                with page.open("w", encoding="utf-8", newline="\n") as handle:
                    handle.write(new_text)
            print(
                "wrote {:d} row sentence(s): {}".format(
                    len(changed), ", ".join(changed) if changed else "none - already current"
                )
            )
            return 0
        for problem in problems:
            print(problem, file=sys.stderr)
        if problems:
            print(
                "run: python scripts/quality_matrix.py --write-rows",
                file=sys.stderr,
            )
            return 1
        print("OK: every count in QUALITY_MATRIX.md's tier table is what the probe measures")
        return 0
    data = measure()
    if "--holes" in argv:
        for kind, name, _why in data["holes"]:
            print(f"{kind}:{name}")
        return 0
    if "--json" in argv:
        print(json.dumps(data, indent=2, sort_keys=True))
        return 0
    for key in SUMMARY_KEYS:
        value = data[key]
        rendered = str(value) if isinstance(value, int) else ", ".join(value)
        print(f"{key:24} {rendered}")
    print("")
    print(f"derived holes ({len(data['holes'])}):")
    for kind, name, why in data["holes"]:
        print(f"  - {kind}:{name} | {why}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
