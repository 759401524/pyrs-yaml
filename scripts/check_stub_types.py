#!/usr/bin/env python3
"""Fail when a type checker reports diagnostics against the shipped stub.

`scripts/check_stub_drift.py` proves two things about `python/pyrs_yaml/pyrs_yaml.pyi`: that it matches what the
declared route derives, and - since ledger (bb) - that it parses. Neither is the question a user asks. A `.pyi`
is not merely Python, it is *typing*, and the gap was measured directly: mypy reported five errors **inside our
file** (`Name "u32" is not defined`, `Name "Callable" is not defined`, three `Invalid type comment or
annotation` for `Py<PyAny>`), while every check this repository had stayed green. Those diagnostics are
attributed to this library in the user's editor, in the one artifact `py.typed` advertises.

Two checkers, because they see different things and both are worth having:

* `mypy` - what a large share of users actually run, so it is the authority here. It is checked through a
  scratch package (`py.typed` plus an importing module) because that is the shape user tooling sees; pointing
  it at a bare `.pyi` is a different question and crashes outright on this file.
* `ty` - faster, and it catches the same defects under different rule names (`unresolved-reference`,
  `invalid-syntax-in-forward-annotation`). It is scoped with `--ignore all --error <rule>` because unscoped it
  also raises 32 strictness findings (`missing-type-argument`, `missing-override-decorator`) that are real
  observations about generator output and not this gate's subject; they are recorded in the ledger rather than
  silently suppressed or turned into a red nobody acts on.

Both are run parse-only, and both are guarded against the failure this repository keeps meeting: a checker that
reports success having examined nothing. `ty check <directory>` answers `All checks passed!` with
`WARN No python files found` for a package whose only content is a `.pyi`, which is why the file is named
explicitly and why "no files found" exits 2 rather than 0.

Usage:
    uv run --with 'mypy==2.4.0' python scripts/check_stub_types.py
    uv run --with 'ty==0.0.85'   python scripts/check_stub_types.py --checker ty
"""

from __future__ import annotations

import argparse
import ast
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
STUB = ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"
PACKAGE = ROOT / "python" / "pyrs_yaml"

# A user-side snippet that touches each part of the contract that was broken: a call whose parameter was
# spelled `u32`, an annotation referencing an unimported `Callable`, an exception class from the declarations
# the route appends (ledger (bf)), and a member from the declarations it attaches (ROADMAP row 1).
USER_SNIPPET = """import pyrs_yaml

document = pyrs_yaml.parse("a: 1")
reveal_type(document)
pyrs_yaml.register_tag("x", None, 1)
pyrs_yaml.parse_stream("a: 1", None)
reveal_type(document.node("a"))
document.set("a", 2)
reveal_type(document.walk())

try:
    pyrs_yaml.parse("[")
except pyrs_yaml.YamlParseError as error:
    reveal_type(error)
"""

# Rules that mean "this file does not describe a checkable type". Kept narrow on purpose: ty's other findings
# are strictness opinions about generator output, listed in docs/dev/quality-ledger.md (bg).
TY_CORRECTNESS_RULES = ("unresolved-reference", "invalid-syntax-in-forward-annotation", "invalid-syntax")


class PackageSourceError(RuntimeError):
    """The scratch package cannot be made to match the stub.

    Raised instead of reporting a finding: a stub that imports a module the package does not have is a real
    defect, but "mypy cannot find it" is an answer about this scratch directory, and a gate that folds the two
    together reads as a clean run on the machine that never built the missing file. Exit code 2 says the check
    could not run.
    """


def relative_import_modules(text: str) -> list[str]:
    """The sibling module names a stub imports from inside its own package.

    Read from the syntax tree rather than a regular expression, because the same shape appears inside
    docstrings, and a docstring is not an import.
    """
    modules = []
    for node in ast.walk(ast.parse(text)):
        if isinstance(node, ast.ImportFrom) and node.level == 1 and node.module:
            modules.append(node.module)
    return sorted(set(modules))


def parse_findings(output: str, stub_name: str = "pyrs_yaml.pyi") -> list[str]:
    """Lines of mypy output that blame the stub itself, plus anything meaning the run is unusable.

    A `note:` from `reveal_type` in the user's file is not a finding, and neither is a complaint about the
    snippet - this is about our artifact. A crash is a finding: a checker that died detected nothing, and
    reporting success over that is the silent pass this gate exists to prevent.
    """
    findings = []
    for line in output.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if (
            "INTERNAL ERROR" in stripped
            or "mypy: error:" in stripped
            or "Crash detected" in stripped
            or (stub_name in stripped and re.search(r":\s*error:", stripped))
        ):
            findings.append(stripped)
    return findings


def parse_ty_findings(output: str, stub_name: str = "pyrs_yaml.pyi") -> list[str]:
    """Ty's `path:line:column: error[rule]: message` lines, for the stub only.

    Kept separate from `parse_findings` because the formats genuinely differ - ty brackets the rule and mypy
    tags it at the end - and one regular expression pretending to cover both would quietly match neither.
    """
    return [line.strip() for line in output.splitlines() if stub_name in line and re.search(r"error\[[\w-]+\]", line)]


def vacuous(output: str) -> bool:
    """Whether the run examined nothing, whatever it claimed afterwards."""
    return "No python files found" in output


def run(checker: str, stub: pathlib.Path, work: pathlib.Path) -> tuple[list[str], str, int]:
    """Materialise the scratch package the way a user's tooling sees it, and ask the checker about it.

    The stub's own package-relative imports are copied in from the real package, because the wheel ships those
    modules beside the stub: leaving them out makes the checker report `import-not-found` against our artifact
    for a file the user does have, which is a finding about this scratch directory rather than about the stub.

    Raises:
        PackageSourceError: when the stub imports a sibling module the package does not contain.
    """
    package = work / "pyrs_yaml"
    package.mkdir(parents=True, exist_ok=True)
    shutil.copy2(stub, package / "pyrs_yaml.pyi")
    (package / "py.typed").write_text("", encoding="utf-8")
    for module in relative_import_modules(stub.read_text(encoding="utf-8")):
        source = PACKAGE / f"{module}.py"
        if not source.is_file():
            raise PackageSourceError(
                f"{stub.name} imports `.{module}` but the package has no {module}.py; a wheel whose typing "
                "contract references a module it does not ship is a defect, and a checker that cannot find it "
                "is not the way to report one"
            )
        shutil.copy2(source, package / f"{module}.py")

    if checker == "mypy":
        (package / "__init__.py").write_text("from .pyrs_yaml import *\n", encoding="utf-8")
        # `newline=` on `write_text` is a 3.10 keyword, and the mypy overlay is written from the
        # floor interpreter too; `open` has accepted it since 3.5 and keeps the LF untranslated.
        with (work / "user.py").open("w", encoding="utf-8", newline="\n") as handle:
            handle.write(USER_SNIPPET)
        command = [
            sys.executable,
            "-m",
            "mypy",
            "--no-site-packages",
            "--follow-imports=silent",
            "--show-error-codes",
            "--cache-dir",
            ".mypy_cache",
            "user.py",
        ]
        parse = parse_findings
    else:
        # The file is named explicitly: `ty check <dir>` reports "All checks passed!" for a package holding
        # only a `.pyi`, having found no files to check.
        command = ["ty", "check", "--output-format", "concise", "--ignore", "all"]
        for rule in TY_CORRECTNESS_RULES:
            command += ["--error", rule]
        command.append(str(package / "pyrs_yaml.pyi"))
        parse = parse_ty_findings

    result = subprocess.run(
        command, cwd=work, capture_output=True, check=False, text=True, encoding="utf-8", errors="replace"
    )
    output = (result.stdout or "") + (result.stderr or "")
    return parse(output), output, result.returncode


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Check the shipped type stub with a type checker.")
    parser.add_argument("--stub", type=pathlib.Path, default=STUB, help="stub to check")
    parser.add_argument("--checker", choices=("mypy", "ty"), default="mypy", help="which checker to ask")
    args = parser.parse_args(argv[1:])

    if not args.stub.is_file():
        print(f"ERROR: {args.stub} not found", file=sys.stderr)
        return 2

    # Neither checker is a project dependency, so "the binary is not here" is the normal state outside the
    # CI overlay that installs it. `ty` absent does not return a non-zero code, it raises FileNotFoundError -
    # and a gate that dies on that is a gate that stays green on the machine that authored it (ledger (bg)).
    command = [sys.executable, "-m", "mypy", "--version"] if args.checker == "mypy" else ["ty", "--version"]
    try:
        probe = subprocess.run(command, capture_output=True, check=False, text=True, encoding="utf-8", errors="replace")
    except OSError:
        probe = None
    if probe is None or probe.returncode != 0:
        print(
            f"ERROR: {args.checker} is not available; run the gate as\n"
            f"  uv run --with '{args.checker}==*' python scripts/check_stub_types.py --checker {args.checker}",
            file=sys.stderr,
        )
        return 2

    work = pathlib.Path(tempfile.mkdtemp(prefix="stub-types-"))
    try:
        try:
            findings, output, code = run(args.checker, args.stub, work)
        except (PackageSourceError, SyntaxError) as error:
            # A stub that is not Python at all lands here too: the scratch package cannot be built from it, so
            # the check could not run - which is exit 2, not the 1 that means "the checker blamed our file".
            print(f"ERROR: {error}", file=sys.stderr)
            return 2
        if vacuous(output):
            print("ERROR: the checker examined no files; that is not a pass", file=sys.stderr)
            print("  " + output.strip().splitlines()[-1][:160], file=sys.stderr)
            return 2
        if findings:
            print(
                f"ERROR: {args.checker} reports the shipped stub does not check clean ({len(findings)} finding(s)):",
                file=sys.stderr,
            )
            for line in findings:
                print("  " + line, file=sys.stderr)
            print(
                "\nThe stub is machine output; repair it in scripts/check_stub_drift.py's declared transforms\n"
                "and re-run the route, never by editing python/pyrs_yaml/pyrs_yaml.pyi.",
                file=sys.stderr,
            )
            return 1
        verdict = f"{args.checker}: {args.stub.name} checks clean"
        # Only quote the tool's own exit code when it agrees with the verdict. `ty` returns 1 while
        # reporting rules the gate does not enforce, and "checks clean (exit 1)" is a log line that
        # reads as a failure long after anyone remembers why.
        print(verdict if code == 0 else verdict + " (checker reported non-target rules as well)")
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
