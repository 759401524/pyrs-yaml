"""Assert the committed type stub equals what the declared regeneration route derives.

`python/pyrs_yaml/pyrs_yaml.pyi` ships inside every wheel and is the public typing
contract, but the file is machine output: AGENTS.md and `ruff.toml` both forbid
hand-editing it. Before this check the only stub assertions in CI were existence
ones (`git ls-files --error-unmatch` + `test -f py.typed`), so a binding signature
change could leave the committed stub silently behind until a user's type checker
noticed.

This script compares the *tracked* stub against a freshly generated one and exits
non-zero on any difference. Two transforms stand between raw generator output and
the bytes the repository legitimately stores; both are applied here so that the
tracked file stays fully derived instead of being patched by hand:

1. Trailing whitespace is stripped per line. `prek.toml` runs the built-in
   `trailing-whitespace` hook on commit, and the generator emits whitespace-only
   lines inside docstrings, so the committed form is the stripped form. Ignoring
   this would keep the check permanently red for a non-semantic reason.
2. `FIDELITY_FIXES` rewrites returns the generator is known to model unfaithfully.
   maturin 1.14.1 special-cases `__next__` and emits only the iterator's yield
   type, dropping the `Option` the bindings actually return
   (`PyResult<Option<Bound<'a, PyDict>>>` in `crates/pyrs-yaml/src/py/`). The
   generator gets every other `Option` right (`get_plugin`, `parse_stream`), so
   this is a narrow generator gap, not a project convention. Each fix declares how
   many times it must match; an unexpected count fails instead of silently
   rewriting, so a changed return type or a fixed upstream forces a review.
3. `escape_docstring_backslashes` repairs a defect with a wider blast radius: the
   generator writes a runtime `__doc__` into a triple-quoted string *verbatim*, so a
   doc comment containing a backslash lands in the stub unescaped and the file stops
   being valid Python. Measured on `maturin 1.14.1`: two docstring lines carry one
   (`to_json`'s prose about `json.dumps` escaping, and the JSON5 loader's note about
   exotic float spellings), and `ast.parse` fails at the first with
   `'unicodeescape' codec can't decode bytes … truncated \\uXXXX escape`. That is not
   cosmetic: the stub ships inside every wheel as the public typing contract, so
   mypy, pyright and griffe each read a file they cannot parse. Doubling the
   backslash inside the docstring restores the exact text `help()` already shows.
   The transform is generic, and `EXPECTED_DOCSTRING_ESCAPES` is the tripwire that
   makes a third site (or an upstream fix that removes one) a review rather than a
   silent change of count.

After all transforms the derived text must parse: `verify_parses` re-reads it with
`ast`, so a generator misbehaviour that is not yet described by a declared fix fails
the gate naming its line instead of shipping as an unusable artifact.

4. `exception_block` appends what the generator omits completely: the extension's ten exception types.
   maturin 1.14.1 walks the module's own classes, and a PyO3 `import_exception!` type is not among the
   objects it emits, so the stub - the file `py.typed` advertises to mypy and pyright inside every wheel -
   declared no error type at all, and `except pyrs_yaml.YamlParseError:` was unresolvable. The names, base
   classes and docstrings are read from the built extension rather than kept in a list here, so the
   declarations cannot go stale; `EXPECTED_EXCEPTION_CLASSES` is the tripwire for an exception appearing or
   disappearing, and an unimportable extension exits 2 instead of quietly producing a thinner contract.

Usage:
    uv run maturin generate-stubs --out target/stubs
    python scripts/check_stub_drift.py            # verify only
    python scripts/check_stub_drift.py --fix      # write the derived stub

Exit code 0 = in sync; 1 = drift; 2 = the generated stub is missing or unusable.
"""

from __future__ import annotations

import argparse
import ast
import builtins
import difflib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GENERATED = ROOT / "target" / "stubs" / "pyrs_yaml.pyi"
TRACKED = ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"

# Anchored on the generator's exact output so a real signature change stops
# matching (and fails) rather than being rewritten into something untrue.
NEXT_RETURN = re.compile(r"^(\s*)def __next__\(self, /\) -> dict(: \.\.\.|:)$", re.MULTILINE)

FIDELITY_FIXES = (
    {
        "name": "next-return-drops-optional",
        "pattern": NEXT_RETURN,
        "expected_matches": 2,
        "template": r"\g<1>def __next__(self, /) -> dict[Any, Any] |None\g<2>",
        "reason": (
            "maturin 1.14.1 emits the yield type for __next__ and drops the Option "
            "the binding returns; both __next__ methods return dict | None, spelled "
            "dict[Any, Any] because the key type is only known at the call site."
        ),
    },
)

MAX_DIFF_LINES = 120

# The extension's exception types never reach the generator's output at all: maturin 1.14.1 introspects the
# built module, and a PyO3 `import_exception!` class is not the kind of object its emitter walks. The package
# exports ten of them - measured against the built extension, not against a list someone believes - so the
# public typing contract that ships inside every wheel declared no exception type, and `except
# pyrs_yaml.YamlParseError:` is invisible to mypy and pyright. The declarations below are derived from the
# live classes at derivation time (names, bases, docstrings), so the stub cannot drift from the bindings it
# describes; `EXPECTED_EXCEPTION_CLASSES` is the tripwire that makes a new or removed exception a review.
EXPECTED_EXCEPTION_CLASSES = 10

EXCEPTION_SUFFIXES = ("Error", "Exception")
EXCEPTION_ALIASES = ("YamlTagSkip",)

# Measured on the pinned generator: two docstring lines contain a backslash, both written by maturin
# 1.14.1 straight out of the Rust doc comment. A third site or a fixed upstream changes the count, and
# that change is exactly what should make a person look at this file.
EXPECTED_DOCSTRING_ESCAPES = 2

DOCSTRING_DELIMITERS = ('"""', "'''")

# The generator also copies Rust-side spellings into annotations, and mypy - the tool `py.typed` exists to
# serve - reports them as errors in our file: `Name "u32" is not defined` and three
# `Invalid type comment or annotation` for `Py<PyAny>`, plus `Name "Callable" is not defined` because the
# emitter writes an annotation referencing `typing.Callable` without importing it. Each mapping is a Python
# spelling for the same value, and the site count is declared so a fourth `Py<PyAny>` is a review.
RUST_TYPE_SPELLINGS = {"Py<PyAny>": "Any", "u32": "int", "usize": "int"}
EXPECTED_RUST_TYPE_SITES = 4


def rewrite_rust_spellings(text: str) -> tuple[str, int]:
    """Replace Rust-side type spellings inside annotations with their Python equivalent.

    Only quoted annotations are touched - a bare `u32` in the file would be a name someone defined, not a
    generator artifact - and the site count is returned so the caller can hold it to
    `EXPECTED_RUST_TYPE_SITES`.
    """
    count = 0
    for spelling, replacement in RUST_TYPE_SPELLINGS.items():
        pattern = re.compile(r'"([^"]*)' + re.escape(spelling) + r'([^"]*)"')
        # `replacement` is bound as a default rather than closed over: a lambda that reads a loop variable is
        # correct only while the call stays inside the loop, and that is a constraint nobody writes down.
        text, hits = pattern.subn(
            lambda match, value=replacement: '"' + match.group(1) + value + match.group(2) + '"', text
        )
        count += hits
    return text, count


def missing_annotation_imports(text: str) -> list[str]:
    """Names used in annotations that the file neither defines nor imports.

    Annotations, not every string: an early attempt here scanned all quoted text and reported `AST`, `Accepts`
    and `Community` as undefined names, because those are words inside docstrings. `ast` knows which strings
    are annotations, so it is asked instead of a regular expression.
    """
    tree = ast.parse(text)
    declared = {
        node.name for node in tree.body if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))
    }
    declared.update(dir(builtins))  # `int`, `str`, `bytes` and friends appear in annotations and import nothing
    for node in ast.walk(tree):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            declared.update((alias.asname or alias.name).split(".")[0] for alias in node.names)
        if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
            declared.add(node.id)

    used: set[str] = set()

    def names_of(node):
        for child in ast.walk(node):
            if isinstance(child, ast.Name):
                used.add(child.id)
            elif isinstance(child, ast.Constant) and isinstance(child.value, str):
                # Quoted (forward) annotations are strings to the parser and types to the checker.
                try:
                    expression = ast.parse(child.value, mode="eval")
                except SyntaxError:
                    continue
                names_of(expression)

    for node in ast.walk(tree):
        if isinstance(node, ast.arg) and node.annotation is not None:
            names_of(node.annotation)
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.returns is not None:
            names_of(node.returns)
        elif isinstance(node, ast.AnnAssign) and node.annotation is not None:
            names_of(node.annotation)
    return sorted(used - declared)


def add_typing_imports(text: str) -> tuple[str, list[str], list[str]]:
    """Merge annotation-only missing names into the file's `typing` import, and report what happened.

    Returns:
        The rewritten text, the names added, and the names this cannot fix. A name `typing` does not export
        is a problem rather than a guess: emitting `from typing import Whatever` for it would trade one
        undefined name for an import error, and the caller is expected to fail on a non-empty third element.
    """
    import typing

    exports = set(getattr(typing, "__all__", ())) | set(dir(typing))
    absent = missing_annotation_imports(text)
    missing = [name for name in absent if name not in exports]
    addable = [name for name in absent if name in exports]
    if not addable:
        return text, addable, missing
    pattern = re.compile(r"^from typing import (?P<names>.+)$", re.M)
    match = pattern.search(text)
    if match:
        names = sorted({part.strip() for part in match.group("names").split(",") if part.strip()} | set(addable))
        replacement = "from typing import " + ", ".join(names)
        return text[: match.start()] + replacement + text[match.end() :], addable, missing
    lines = text.split("\n")
    insert_at = next((n for n, line in enumerate(lines) if line.startswith(("from ", "import "))), 0)
    lines[insert_at:insert_at] = ["from typing import " + ", ".join(sorted(addable)), ""]
    return "\n".join(lines), addable, missing


def docstring_body_lines(text: str) -> set[int]:
    """The 1-based numbers of lines that sit inside a triple-quoted string body.

    Scanned line by line instead of parsed with `ast`, because the whole point of this transform is that
    the text it receives is not parseable yet. Generator output opens a docstring on a line of its own and
    closes it on a line of its own, so the body is everything strictly between those two markers; a
    delimiter that opens and closes on one line has no body lines and is handled by the same state change.
    """
    bodies: set[int] = set()
    delimiter = None
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if delimiter is None:
            for candidate in DOCSTRING_DELIMITERS:
                if not stripped.startswith(candidate):
                    continue
                # A closing delimiter later on the same line means the string ends where it started.
                if stripped.count(candidate) >= 2:
                    break
                delimiter = candidate
                break
            continue
        if delimiter in stripped:
            delimiter = None
            continue
        bodies.add(number)
    return bodies


def escape_docstring_backslashes(text: str) -> tuple[str, int]:
    """Double every backslash inside a docstring, and report how many lines changed.

    Args:
        text: freshly generated stub text.

    Returns:
        The text with backslashes escaped the way Python source requires, plus the number of lines that
        needed it - the caller compares that count with `EXPECTED_DOCSTRING_ESCAPES` so an undocumented
        change of shape is a failure rather than a quiet improvement.
    """
    bodies = docstring_body_lines(text)
    changed = 0
    lines = []
    for number, line in enumerate(text.splitlines(), 1):
        if number in bodies and "\\" in line:
            lines.append(line.replace("\\", "\\\\"))
            changed += 1
        else:
            lines.append(line)
    trailing = "\n" if text.endswith("\n") else ""
    return "\n".join(lines) + trailing, changed


class StubInputError(RuntimeError):
    """An input the derivation cannot do without.

    Raised instead of guessing: a missing extension or an unexpected shape means the derived stub would be
    incomplete in a way a reader would never see, so the route stops with exit code 2 and says what to run.
    """


def order_by_base(entries: list[tuple[str, str, str]]) -> list[tuple[str, str, str]]:
    """Reorder `(name, base, docstring)` triples so a declared base precedes its subclass.

    A `.pyi` is Python source, so `class YamlTagSkip(YamlTagError)` only parses if `YamlTagError` was declared
    above it - and the natural alphabetical listing is not guaranteed to get that right. Bases outside the set
    (`ValueError`, `TypeError`) are already declared by whoever reads the file, so they impose no order.

    Raises:
        StubInputError: when no remaining class can be placed, i.e. the bases form a cycle or name a class
            this block does not declare. Both would emit a file that fails to parse, and neither is worth
            discovering from a syntax error in an artifact nobody may hand-edit.
    """
    declared = {name for name, _base, _doc in entries}
    ordered: list[tuple[str, str, str]] = []
    pending = sorted(entries)
    while pending:
        placed = {name for name, _base, _doc in ordered}
        ready = [item for item in pending if item[1] not in declared or item[1] in placed]
        if not ready:
            raise StubInputError(f"exception bases form a cycle or name an undeclared base: {pending}")
        for item in ready:
            ordered.append(item)
            pending.remove(item)
    return ordered


def extension_exceptions() -> list[tuple[str, str, str]]:
    """The extension's exported exception types, as `(name, base, docstring)`, ordered and deterministic.

    Raises:
        StubInputError: when the built extension cannot be imported, or an error type does not have exactly
            one base. Reading these from the live classes is the point - a hand-kept list would go stale the
            day an exception is added, and the alternative is shipping a typing contract that quietly omits
            them.

    Returns:
        One triple per exported error type, ordered so that a base declared inside the set precedes the
        class that inherits from it, which is what makes the appended declarations parse.
    """
    try:
        import pyrs_yaml
    except ImportError as error:
        raise StubInputError(
            f"cannot import pyrs_yaml to read its exception types ({error}); the committed stub is only "
            "derivable while the built extension is importable - run `uv run maturin develop` first"
        ) from error

    found = []
    for name in dir(pyrs_yaml):
        if name.startswith("_"):
            continue
        if not (name.endswith(EXCEPTION_SUFFIXES) or name in EXCEPTION_ALIASES):
            continue
        obj = getattr(pyrs_yaml, name)
        if not isinstance(obj, type) or getattr(obj, "__module__", "") != "pyrs_yaml":
            continue
        bases = [base.__name__ for base in obj.__bases__]
        if len(bases) != 1:
            raise StubInputError(f"{name} has bases {bases}; this route declares exactly one")
        found.append((name, bases[0], (obj.__doc__ or "").strip()))

    return order_by_base(found)


def exception_block(text: str) -> tuple[str, int]:
    """Append the derived exception declarations, and report how many were written.

    Args:
        text: the stub text after the other fidelity transforms.

    Returns:
        The text with one `class Name(Base)` declaration per extension exception - carrying the live
        docstring, with backslashes doubled the way Python source requires for a string literal - plus the
        count, which the caller compares with `EXPECTED_EXCEPTION_CLASSES`.
    """
    entries = extension_exceptions()
    lines = [text.rstrip("\n"), ""]
    for name, base, doc in entries:
        lines.append(f"class {name}({base}):")
        if doc:
            lines.append('    """')
            for row in doc.splitlines():
                lines.append(("    " + row).replace("\\", "\\\\"))
            lines.append('    """')
        else:
            lines.append("    ...")
        lines.append("")
    return "\n".join(lines) + "\n", len(entries)


def verify_parses(text: str, origin: str) -> str | None:
    """Return a problem description when the derived stub is not valid Python, otherwise None.

    The stub is the public typing contract shipped inside every wheel, so "it parses" is not a nicety: an
    unparseable file is one that mypy, pyright and griffe each refuse to read. Naming the line is the point
    - a syntax error is otherwise reported against a generated artifact nobody is allowed to edit by hand.
    """
    try:
        ast.parse(text)
    except SyntaxError as error:
        return (
            f"{origin} is not valid Python: {error.msg} (line {error.lineno}). "
            "If the generator introduced a new unescaped character in a docstring, declare it in "
            "escape_docstring_backslashes' expected count instead of hand-patching the artifact."
        )
    return None


def display(path: Path) -> str:
    """Render a path relative to the repository when it lives inside it."""
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(ROOT))
    except ValueError:  # caller passed a path outside the repository
        return str(resolved)


def normalize(text: str) -> str:
    """Strip trailing whitespace per line, matching prek's trailing-whitespace hook."""
    return "\n".join(line.rstrip() for line in text.split("\n"))


def derived_text(generated: str) -> tuple[str, list[str]]:
    """Turn raw generator output into the bytes the repository is expected to hold.

    Args:
        generated: the text of a freshly generated stub.

    Returns:
        The normalized text and a list of problems found while applying the
        declared fidelity fixes. A non-empty problem list means the caller must
        fail: the rewrite was not applied for that fix. Escaping docstring
        backslashes is different in kind - the transform is generic, so it always
        runs - but its site count is still checked, and the result still has to
        parse, so a generator change is noticed instead of absorbed. The same
        holds for the exception declarations, which the generator omits entirely:
        they are appended from the live classes, counted against
        `EXPECTED_EXCEPTION_CLASSES`, and the extension being unimportable is an
        error (`StubInputError`) rather than a thinner contract shipped quietly.
    """
    problems: list[str] = []
    text = normalize(generated)
    text, escapes = escape_docstring_backslashes(text)
    if escapes != EXPECTED_DOCSTRING_ESCAPES:
        problems.append(
            f"docstring backslash escapes: rewrote {escapes} line(s), expected "
            f"{EXPECTED_DOCSTRING_ESCAPES}. maturin 1.14.1 copies __doc__ into the stub "
            "without escaping, so a new line is a new unparseable site and a disappeared one "
            "is an upstream fix; either way review EXPECTED_DOCSTRING_ESCAPES instead of "
            "trusting the rewrite."
        )
    text, exceptions = exception_block(text)
    if exceptions != EXPECTED_EXCEPTION_CLASSES:
        problems.append(
            f"exception declarations: wrote {exceptions} class(es), expected "
            f"{EXPECTED_EXCEPTION_CLASSES}. These are read from the built extension because "
            "maturin 1.14.1 emits none of them; a count change means an exception was added "
            "or removed, and the public typing contract has to be re-checked either way."
        )
    text, rust_sites = rewrite_rust_spellings(text)
    if rust_sites != EXPECTED_RUST_TYPE_SITES:
        problems.append(
            f"rust type spellings: rewrote {rust_sites} annotation site(s), expected "
            f"{EXPECTED_RUST_TYPE_SITES}. maturin 1.14.1 copies them out of the binding signatures and a "
            "type checker rejects them in our file; a new site is a new Rust type reaching the public "
            "contract, which needs a Python spelling decided rather than absorbed."
        )
    text, added, unfixed = add_typing_imports(text)
    if unfixed:
        problems.append(
            "annotations reference names no import can fix: " + ", ".join(unfixed) + "; the generator "
            "emitted a type this file cannot name, so declare the mapping in RUST_TYPE_SPELLINGS."
        )
    if added:
        # Informational, not a failure: importing what an annotation references is the repair, and the
        # type-checker gate downstream is what proves it worked. Reporting it keeps the run honest about the
        # generator still omitting the import without turning a fixed file into a red one.
        print("route note: imported " + ", ".join(added) + " for annotations that referenced it")
    for fix in FIDELITY_FIXES:
        matches = fix["pattern"].findall(text)
        if len(matches) != fix["expected_matches"]:
            problems.append(
                f"fidelity fix {fix['name']!r} matched {len(matches)} site(s), "
                f"expected {fix['expected_matches']}: {fix['reason']} "
                "Review the fix declaration before trusting the comparison."
            )
            continue
        text = fix["pattern"].sub(fix["template"], text)
    parse_problem = verify_parses(text, "the derived stub")
    if parse_problem:
        problems.append(parse_problem)
    return text, problems


def read_text(path: Path, hint: str) -> str:
    if not path.is_file():
        print(f"ERROR: {display(path)} not found. {hint}", file=sys.stderr)
        raise SystemExit(2)
    # Line endings are not part of the contract this gate checks. The generator writes
    # with the platform default, so a Windows box produced CRLF, `--fix` wrote that back
    # over the committed stub, and the next Linux run saw the whole file as drift - 597
    # lines of churn with no content change, and a PR gate that only agreed with whoever
    # ran it last. Normalize on read, and write LF on the fix path.
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def report_drift(expected: str, actual: str, tracked: Path) -> int:
    diff = list(
        difflib.unified_diff(
            expected.split("\n"),
            actual.split("\n"),
            fromfile="derived (maturin generate-stubs + declared fixes)",
            tofile=display(tracked),
            lineterm="",
        )
    )
    print(
        f"ERROR: {display(tracked)} is not what the regeneration route derives "
        f"({sum(1 for line in diff if line.startswith('+'))} added, "
        f"{sum(1 for line in diff if line.startswith('-'))} removed).",
        file=sys.stderr,
    )
    for line in diff[:MAX_DIFF_LINES]:
        print(line, file=sys.stderr)
    if len(diff) > MAX_DIFF_LINES:
        print(f"... ({len(diff) - MAX_DIFF_LINES} more diff lines)", file=sys.stderr)
    print(
        "\nFix by running the declared route, not by editing the stub:\n"
        "  uv run maturin generate-stubs --out target/stubs\n"
        "  python scripts/check_stub_drift.py --fix",
        file=sys.stderr,
    )
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify the committed type stub matches what the regeneration route derives."
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="write the derived stub to the tracked path instead of only verifying it",
    )
    parser.add_argument("--generated", type=Path, default=GENERATED, help="generated stub to compare from")
    parser.add_argument("--tracked", type=Path, default=TRACKED, help="committed stub to compare against")
    args = parser.parse_args()

    hint = "Run: uv run maturin generate-stubs --out target/stubs"
    try:
        expected, problems = derived_text(read_text(args.generated, hint))
    except StubInputError as error:
        # Exit 2, the same code as an unusable generated file: the derivation could not be done, so neither
        # "in sync" nor "drift" is a truthful answer, and a green run here must never mean "the exception
        # declarations were skipped today".
        print(f"ERROR: {error}", file=sys.stderr)
        return 2
    if problems:
        for problem in problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 2

    actual = normalize(read_text(args.tracked, "It should be committed; check .gitignore."))
    if args.fix:
        if actual != expected:
            args.tracked.write_text(expected, encoding="utf-8", newline="\n")
            print(f"Updated {display(args.tracked)} from the regeneration route.")
        else:
            print(f"{display(args.tracked)} already matches the regeneration route.")
        return 0

    if actual == expected:
        print(f"{display(args.tracked)} matches the regeneration route.")
        return 0
    return report_drift(expected, actual, args.tracked)


if __name__ == "__main__":
    sys.exit(main())
