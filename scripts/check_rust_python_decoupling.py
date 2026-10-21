#!/usr/bin/env python3
"""Keep the Rust crate matrix free of the Python runtime, and prove it every run.

The coupling this gate measures is a build-time one, and the distinction decides what the rule
has to be. Measured on this tree: 848 artifacts under `target/debug/deps` were scanned and exactly
one names a `python*.dll` - `pyrs_yaml.dll`, the extension cdylib itself. No Rust test binary
links one. The reason is that the linker prunes a dependency no symbol references, so the *binary*
was never coupled. The *build* is: `pyo3`'s build script (`pyo3-build-config`) probes for an
interpreter to configure linkage, and the extension crate is a workspace member, so
`cargo test --workspace` needs a Python installation even when the thing being tested is a
parser. That is why `ci.yml`'s Rust legs set `PYO3_PYTHON: python`.

So the invariant is not "no Rust artifact mentions Python" - it is "the crates the Rust legs build
must not need an interpreter, and the one crate that does must be the extension, declared as such".
Four rules, each derived from the tree rather than from a list written by hand:

  R1  The cdylib crate and the Python-runtime crates are the same set. A crate that produces no
      cdylib must carry no Python runtime crate in its dependency closure (dependencies, dev
      dependencies and build dependencies, transitively through the workspace's path edges); the
      crate that produces the cdylib must carry one. A crate that is Python-coupled without being
      the extension is the hole this whole sequence exists to close.

  R2  The extension crate holds no Rust test tier: no `#[cfg(test)]` block under `src/`, no
      `tests/` directory, no `[[test]]` target - and it declares `[lib] test = false`, so cargo
      does not even build the empty test binary. The declaration is the half that matters: the
      tests moved to the engine crate, and a manifest that still says `test = false` on a crate
      that has tests again is the way that move silently reverses.

  R3  Decoupling is the absence of the coupling, not the absence of the tests. No other crate may
      disable its test tier, and no other crate may hold an empty one. A crate whose tests were
      deleted to make it "decoupled" is a finding, not a fix.

  R4  A feature-gated target must name a feature the crate declares and does not enable by
      default. That is what keeps the one Python-dependent Rust target in this repository - the
      instruction-count harness in the extension crate, which starts CPython itself - out of every
      default build.

The manifests are read by a reader written for exactly the constructs Cargo manifests use, because
`tomllib` is 3.11+ and this package supports 3.8. The reader refuses a construct it does not
understand (exit 2), so an unfamiliar manifest shape stops the gate instead of being guessed at,
and `tests/test_rust_python_decoupling_gate.py` cross-checks it against `tomllib`/`tomli` on every
manifest in the tree.

Usage:
    python scripts/check_rust_python_decoupling.py            # this repository
    python scripts/check_rust_python_decoupling.py <root>     # another workspace

Exit code 0 = the matrix is decoupled; 1 = a rule is broken; 2 = the gate could not measure
anything, which never reads as a pass.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent

# A Python-runtime crate: one that needs a Python interpreter to build or run. `pyo3` and its
# build-time halves (`pyo3-ffi`, `pyo3-macros`, `pyo3-build-config`) and the NumPy binding are
# the whole vocabulary today. The rule is a prefix, not a list, so a new `pyo3-*` crate is caught
# by the same rule rather than by a later edit to this file.
PYTHON_RUNTIME_NAMES = frozenset({"numpy"})
PYTHON_RUNTIME_PREFIXES = ("pyo3",)

# The dependency tables that form a crate's closure. A dev-dependency is in the closure because
# `cargo test` builds it, and a build-dependency is in it because the build script runs.
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")

# `#[cfg(test)]` and its relatives. `#[cfg(any(feature = "std", test))]` is a test-gated block
# too, so the predicate is read rather than matched literally - but `not(test)` compiles the block
# *out* of a test build, so it is not a test tier and is not counted as one.
_CFG_ATTRIBUTE = re.compile(r"#\s*\[\s*cfg\s*\(([^)]*)\)\s*\]")
_NOT_TEST = re.compile(r"\bnot\s*\(\s*test\b")
_TEST_TOKEN = re.compile(r"\btest\b")


class ManifestError(Exception):
    """A manifest shape this reader does not understand.

    Raised rather than guessed at: a wrong answer here would be a gate that waves a broken
    matrix through, which is the failure this repository keeps paying for elsewhere.
    """


# ── a reader for the manifest subset Cargo uses ─────────────────────────────────────────


def _skip(text: str, index: int) -> int:
    """Skip whitespace and comments."""
    length = len(text)
    while index < length:
        char = text[index]
        if char in " \t\r\n":
            index += 1
        elif char == "#":
            newline = text.find("\n", index)
            index = length if newline < 0 else newline + 1
        else:
            break
    return index


def _skip_blanks(text: str, index: int) -> int:
    """Skip whitespace only, for the places where a comment cannot legally appear."""
    length = len(text)
    while index < length and text[index] in " \t\r\n":
        index += 1
    return index


_BARE_KEY = re.compile(r"[A-Za-z0-9_-]+")
_BARE_VALUE = re.compile(r"[A-Za-z0-9_+.-]+")
_ESCAPES = {'"': '"', "\\": "\\", "b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t"}


def _parse_string(text: str, index: int) -> tuple:
    """A basic or literal string. Multi-line strings are not supported and are refused."""
    quote = text[index]
    index += 1
    length = len(text)
    out = []
    while index < length:
        char = text[index]
        if quote == "'":
            if char == "'":
                return "".join(out), index + 1
            out.append(char)
            index += 1
            continue
        if char == "\\":
            index += 1
            if index >= length:
                raise ManifestError("a string ends with a backslash")
            escape = text[index]
            if escape in _ESCAPES:
                out.append(_ESCAPES[escape])
                index += 1
            elif escape == "u":
                digits = text[index + 1 : index + 5]
                if len(digits) != 4:
                    raise ManifestError("a \\u escape needs four hex digits")
                try:
                    out.append(chr(int(digits, 16)))
                except ValueError:
                    raise ManifestError("a \\u escape is not four hex digits") from None
                index += 5
            else:
                raise ManifestError(f"unsupported escape \\{escape}")
            continue
        if char == '"':
            return "".join(out), index + 1
        out.append(char)
        index += 1
    raise ManifestError("an unterminated string")


def _parse_key(text: str, index: int) -> tuple:
    char = text[index]
    if char in "\"'":
        return _parse_string(text, index)
    match = _BARE_KEY.match(text, index)
    if not match:
        raise ManifestError(f"cannot read a key at offset {index}")
    return match.group(0), match.end()


def _parse_value(text: str, index: int) -> tuple:
    index = _skip(text, index)
    if index >= len(text):
        raise ManifestError("a value was expected")
    char = text[index]
    if char in "\"'":
        return _parse_string(text, index)
    if char == "[":
        return _parse_array(text, index)
    if char == "{":
        return _parse_inline_table(text, index)
    match = _BARE_VALUE.match(text, index)
    if not match:
        raise ManifestError(f"cannot read a value at offset {index}")
    token = match.group(0)
    if token == "true":
        return True, match.end()
    if token == "false":
        return False, match.end()
    try:
        return int(token), match.end()
    except ValueError:
        pass
    try:
        return float(token), match.end()
    except ValueError:
        raise ManifestError(f"cannot read the value {token!r}") from None


def _parse_array(text: str, index: int) -> tuple:
    index += 1
    items = []
    while True:
        index = _skip(text, index)
        if index >= len(text):
            raise ManifestError("an unterminated array")
        if text[index] == "]":
            return items, index + 1
        value, index = _parse_value(text, index)
        items.append(value)
        index = _skip(text, index)
        if index < len(text) and text[index] == ",":
            index += 1


def _parse_inline_table(text: str, index: int) -> tuple:
    index += 1
    table = {}
    while True:
        index = _skip(text, index)
        if index >= len(text):
            raise ManifestError("an unterminated inline table")
        if text[index] == "}":
            return table, index + 1
        key, index = _parse_key(text, index)
        index = _skip(text, index)
        if index >= len(text) or text[index] != "=":
            raise ManifestError("expected '=' in an inline table")
        value, index = _parse_value(text, index + 1)
        table[key] = value
        index = _skip(text, index)
        if index < len(text) and text[index] == ",":
            index += 1


def _parse_key_path(text: str, index: int) -> tuple:
    parts = []
    while True:
        key, index = _parse_key(text, index)
        parts.append(key)
        index = _skip_blanks(text, index)
        if index < len(text) and text[index] == ".":
            index = _skip_blanks(text, index + 1)
            continue
        return parts, index


def _table(root: dict, path: list) -> dict:
    node = root
    for part in path:
        child = node.setdefault(part, {})
        if isinstance(child, list):
            raise ManifestError("{} is an array of tables".format(".".join(path)))
        if not isinstance(child, dict):
            raise ManifestError("{} is not a table".format(".".join(path)))
        node = child
    return node


def _array_table(root: dict, path: list) -> dict:
    parent = _table(root, path[:-1])
    name = path[-1]
    entries = parent.setdefault(name, [])
    if not isinstance(entries, list):
        raise ManifestError("{} is a table, not an array of tables".format(".".join(path)))
    entry = {}
    entries.append(entry)
    return entry


def parse_manifest(text: str) -> dict:
    """The manifest as nested tables, arrays and scalars.

    Only the constructs a Cargo manifest uses are accepted; anything else raises `ManifestError`.
    """
    root: dict = {}
    current = root
    index = 0
    length = len(text)
    while True:
        index = _skip(text, index)
        if index >= length:
            return root
        if text[index] == "[":
            if text.startswith("[[", index):
                path, index = _parse_key_path(text, index + 2)
                index = _skip_blanks(text, index)
                if not text.startswith("]]", index):
                    raise ManifestError("an unterminated array-of-tables header")
                current = _array_table(root, path)
                index += 2
            else:
                path, index = _parse_key_path(text, index + 1)
                index = _skip_blanks(text, index)
                if index >= length or text[index] != "]":
                    raise ManifestError("an unterminated table header")
                current = _table(root, path)
                index += 1
            continue
        key, index = _parse_key(text, index)
        index = _skip(text, index)
        if index >= length or text[index] != "=":
            raise ManifestError(f"expected '=' after the key {key!r}")
        value, index = _parse_value(text, index + 1)
        current[key] = value


# ── the crate matrix ─────────────────────────────────────────────────────────────────────


class Crate:
    """One workspace member, as the manifest and the tree describe it."""

    def __init__(self, name: str, path: pathlib.Path, manifest: dict):
        self.name = name
        self.path = path
        self.manifest = manifest
        package = manifest.get("package", {})
        self.declared_name = package.get("name", name)
        lib = manifest.get("lib", {})
        crate_types = lib.get("crate-type", [])
        if isinstance(crate_types, str):
            crate_types = [crate_types]
        self.crate_types = list(crate_types)
        self.lib = lib
        self.features = manifest.get("features", {})
        self.default_features = list(self.features.get("default", []))
        self.dependencies = {}
        for table in DEPENDENCY_TABLES:
            for dependency in manifest.get(table, {}):
                self.dependencies.setdefault(dependency, table)
        self.benches = list(manifest.get("bench", []))
        self.tests = list(manifest.get("test", []))

    @property
    def is_extension(self) -> bool:
        return "cdylib" in self.crate_types

    def test_tier(self) -> list:
        """Every sign that this crate holds Rust tests.

        The manifest's own `test = false` is not one of them: for the extension crate that
        declaration is the required state, and for every other crate it is the finding.
        """
        signs = []
        for source in sorted((self.path / "src").rglob("*.rs")) if (self.path / "src").is_dir() else []:
            for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), start=1):
                if _is_test_gated(line):
                    signs.append(f"{source.relative_to(self.path)}:{number}")
        if (self.path / "tests").is_dir():
            signs.append("tests/")
        for target in self.tests:
            signs.append("[[test]] {}".format(target.get("name", "?")))
        return signs


def _is_test_gated(line: str) -> bool:
    """Whether a line opens a `#[cfg(...)]` block that a test build compiles in."""
    for match in _CFG_ATTRIBUTE.finditer(line):
        predicate = match.group(1)
        if _NOT_TEST.search(predicate):
            continue
        if _TEST_TOKEN.search(predicate):
            return True
    return False


def read_crate(path: pathlib.Path) -> Crate:
    manifest_path = path / "Cargo.toml"
    try:
        text = manifest_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ManifestError(f"cannot read {manifest_path}: {exc}") from exc
    manifest = parse_manifest(text)
    name = path.name
    return Crate(name, path, manifest)


def workspace_members(root: pathlib.Path) -> list:
    """The members the root manifest declares, in the order it declares them."""
    manifest_path = root / "Cargo.toml"
    try:
        text = manifest_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ManifestError(f"cannot read {manifest_path}: {exc}") from exc
    manifest = parse_manifest(text)
    members = manifest.get("workspace", {}).get("members", [])
    if not isinstance(members, list) or not members:
        raise ManifestError(f"{manifest_path} declares no workspace members")
    return members


def is_python_runtime(name: str) -> bool:
    """Whether a dependency name is a Python-runtime crate."""
    return name in PYTHON_RUNTIME_NAMES or name.startswith(PYTHON_RUNTIME_PREFIXES)


def closure(crate: Crate, by_name: dict) -> set:
    """Every crate name the crate pulls in, transitively through the workspace's path edges."""
    found = set()
    pending = list(crate.dependencies)
    while pending:
        name = pending.pop()
        if name in found:
            continue
        found.add(name)
        inner = by_name.get(name)
        if inner is not None:
            pending.extend(inner.dependencies)
    return found


# ── the rules ───────────────────────────────────────────────────────────────────────────


def find_violations(root: pathlib.Path) -> tuple:
    """The findings, and the measurements they were derived from."""
    members = workspace_members(root)
    crates = []
    for member in members:
        path = (root / member).resolve()
        if not (path / "Cargo.toml").is_file():
            raise ManifestError(f"the workspace member {member} has no manifest")
        crates.append(read_crate(path))
    by_name = {crate.declared_name: crate for crate in crates}
    for crate in crates:
        if crate.declared_name in by_name and by_name[crate.declared_name] is not crate:
            raise ManifestError(f"two crates are named {crate.declared_name}")

    findings = []
    extensions = [crate for crate in crates if crate.is_extension]

    # R1 - the cdylib crate and the Python-runtime crates are the same set.
    if len(extensions) != 1:
        names = ", ".join(crate.declared_name for crate in extensions) or "none"
        findings.append(
            (
                "R1",
                "workspace",
                f"exactly one crate may produce a cdylib (the extension); found {names}",
            )
        )
    for crate in crates:
        python = sorted(name for name in closure(crate, by_name) if is_python_runtime(name))
        if crate.is_extension:
            if not python:
                findings.append(
                    (
                        "R1",
                        crate.declared_name,
                        "the extension crate has no Python runtime crate in its closure; "
                        "it is not a Python binding any more",
                    )
                )
        elif python:
            findings.append(
                (
                    "R1",
                    crate.declared_name,
                    "a crate that produces no cdylib pulls {} into its closure; "
                    "the Rust legs build it without an interpreter".format(", ".join(python)),
                )
            )

    # R2 - the extension holds no Rust test tier.
    for crate in extensions:
        signs = crate.test_tier()
        if signs:
            findings.append(
                (
                    "R2",
                    crate.declared_name,
                    "the extension crate holds a Rust test tier ({}); the Rust test legs "
                    "would need an interpreter".format(", ".join(signs)),
                )
            )
        if crate.lib.get("test") is not False:
            findings.append(
                (
                    "R2",
                    crate.declared_name,
                    "the extension crate's lib test target must be disabled with [lib] test = false",
                )
            )

    # R3 - decoupling is the absence of the coupling, not the absence of the tests.
    for crate in crates:
        if crate.is_extension:
            continue
        if crate.lib.get("test") is False:
            findings.append(
                (
                    "R3",
                    crate.declared_name,
                    "this crate disables its test tier; decoupling is the absence of the "
                    "coupling, not the absence of the tests",
                )
            )
        elif not crate.test_tier():
            findings.append(
                (
                    "R3",
                    crate.declared_name,
                    "this crate holds no Rust test at all; decoupling is the absence of the "
                    "coupling, not the absence of the tests",
                )
            )

    # R4 - a feature-gated target names a declared feature that is off by default.
    for crate in crates:
        for target in crate.benches + crate.tests:
            required = target.get("required-features", [])
            for feature in required:
                if feature not in crate.features:
                    findings.append(
                        (
                            "R4",
                            crate.declared_name,
                            "the target {} requires the feature {}, which this crate does not declare".format(
                                target.get("name", "?"), feature
                            ),
                        )
                    )
                elif feature in crate.default_features:
                    findings.append(
                        (
                            "R4",
                            crate.declared_name,
                            "the target {} requires {}, which this crate enables by default; "
                            "the gate does not keep the target out of the default build".format(
                                target.get("name", "?"), feature
                            ),
                        )
                    )

    measurements = {
        "crates": len(crates),
        "extension": extensions[0].declared_name if extensions else None,
        "python_free": sum(
            1
            for crate in crates
            if not crate.is_extension and not any(is_python_runtime(name) for name in closure(crate, by_name))
        ),
    }
    return findings, measurements


def main(argv: list) -> int:
    roots = [arg for arg in argv if not arg.startswith("-")]
    root = pathlib.Path(roots[0]).resolve() if roots else REPO
    try:
        findings, measurements = find_violations(root)
    except ManifestError as exc:
        print(f"cannot measure: {exc}", file=sys.stderr)
        return 2
    except OSError as exc:
        print(f"cannot measure: {exc}", file=sys.stderr)
        return 2
    if findings:
        print("the Rust crate matrix is not decoupled from Python:", file=sys.stderr)
        for rule, subject, detail in findings:
            print(f"  [{rule}] {subject}: {detail}", file=sys.stderr)
        print(
            "\nR1 the cdylib crate and the Python-runtime crates are the same set.\n"
            "R2 the extension crate holds no Rust test tier, and says so with [lib] test = false.\n"
            "R3 no other crate may lose its tests - decoupling is the absence of the coupling.\n"
            "R4 a feature-gated target names a declared feature that is off by default.",
            file=sys.stderr,
        )
        return 1
    print(
        "OK: {} crate(s), {} of them free of the Python runtime; the extension is {}".format(
            measurements["crates"], measurements["python_free"], measurements["extension"]
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
