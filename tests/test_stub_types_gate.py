"""Gate the gate: `scripts/check_stub_types.py` has to catch what users would see.

The finding that motivated it: mypy reports five errors inside `python/pyrs_yaml/pyrs_yaml.pyi` - `u32` and
`Callable` undefined, three `Invalid type comment or annotation` for `Py<PyAny>` - while every check this
repository had (including the `ast.parse` gate added at ledger (bb)) stayed green. "It parses" and "it type
checks" are different claims, and only the second one is what a user's editor asks.

So the tests here are the shapes that must bite: an error blamed on our stub, an error blamed on the user's
file (not ours), a note from `reveal_type` (not an error at all), and a checker that crashed - which is the
case where reporting success would be worst of all. Plus the real artifact: the committed stub must come out
clean when a type checker is available, and the gate must report 2, never 0, when it is not.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_stub_types.py"
STUB = REPO_ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"


@pytest.fixture(scope="module")
def gate():
    spec = importlib.util.spec_from_file_location("check_stub_types", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def runs_ok(command):
    """Whether a command can be run and reports success, with absence answered rather than raised.

    `subprocess.run(["ty", ...])` on a machine without `ty` raises FileNotFoundError instead of returning a
    non-zero code, and neither checker is a project dependency - so a probe that only reads `returncode` is
    green on the machine that has the tool and red on every CI leg that does not. Absence answers False here
    and exit 2 in the gate; a skip is honest, a pass earned by missing software is not.
    """
    try:
        probe = subprocess.run(command, capture_output=True, check=False, text=True, encoding="utf-8", errors="replace")
    except OSError:
        return False
    return probe.returncode == 0


def available(checker):
    """Whether the named checker is installed enough to be asked about the stub."""
    return runs_ok(["ty", "--version"] if checker == "ty" else [sys.executable, "-m", "mypy", "--version"])


def test_an_absent_binary_is_answered_not_raised():
    """The CI shape that turned every test leg red: asking for a tool that is not installed.

    Asserted against a genuinely missing binary rather than a patched `subprocess.run`, so the test breaks if
    the catch ever narrows back to `returncode` alone.
    """
    assert runs_ok(["pyrs-yaml-no-such-checker", "--version"]) is False
    assert runs_ok([sys.executable, "-c", "raise SystemExit(3)"]) is False


def test_a_checker_that_cannot_be_spawned_exits_two(gate, monkeypatch):
    """Same absence, seen from the gate: it must report "could not check", never "clean"."""

    def raiser(*_args, **_kwargs):
        raise FileNotFoundError(2, "No such file or directory", "ty")

    monkeypatch.setattr(gate.subprocess, "run", raiser)
    assert gate.main(["check_stub_types", "--checker", "ty", "--stub", str(STUB)]) == 2


def test_an_error_in_our_stub_is_a_finding(gate):
    output = (
        'pyrs_yaml\\pyrs_yaml.pyi:521: error: Name "u32" is not defined  [name-defined]\n'
        "Found 1 error in 1 file (checked 1 source file)\n"
    )
    assert gate.parse_findings(output) == [
        'pyrs_yaml\\pyrs_yaml.pyi:521: error: Name "u32" is not defined  [name-defined]'
    ]


def test_a_complaint_about_the_user_file_is_not_ours(gate):
    output = "user.py:3: error: Argument 1 has incompatible type  [arg-type]\n"
    assert gate.parse_findings(output) == []


def test_a_reveal_type_note_is_not_an_error(gate):
    output = 'user.py:4: note: Revealed type is "pyrs_yaml.pyrs_yaml.YamlDocument"\n'
    assert gate.parse_findings(output) == []


def test_a_crashed_checker_is_a_finding_not_a_pass(gate):
    """A tool that died detected nothing, and silence here is how a gate starts lying."""
    assert gate.parse_findings("error: INTERNAL ERROR -- Please try using mypy master\n")
    assert gate.parse_findings("Crash detected. Report a bug.\n")


def test_the_committed_stub_checks_clean(gate):
    """The artifact, not a fixture - and the exit code decides, not the printed words.

    Skipped where no type checker is importable, because the gate's own answer for that case is exit 2 and a
    skipped test is the honest report rather than a pass earned by absence.
    """
    if not available("mypy"):
        pytest.skip("mypy is not installed in this environment; the CI step runs it under an overlay")
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "--stub", str(STUB)],
        capture_output=True,
        check=False,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_a_missing_type_checker_exits_two_not_zero(gate, monkeypatch, tmp_path):
    """The absence of the check must never read as the check having passed."""
    monkeypatch.setattr(gate.subprocess, "run", lambda *a, **k: subprocess.CompletedProcess(a, 1, "", "no module"))
    assert gate.main(["check_stub_types", "--stub", str(tmp_path / "absent.pyi")]) == 2


def test_a_stub_that_is_not_there_exits_two(gate, tmp_path):
    assert gate.main(["check_stub_types", "--stub", str(tmp_path / "nope.pyi")]) == 2


def test_ty_findings_use_their_own_format(gate):
    """Ty brackets the rule and puts the column first; mypy tags it last. One pattern cannot cover both."""
    output = (
        "pyrs_yaml\\pyrs_yaml.pyi:521:64: error[unresolved-reference] Name `u32` used when not defined\n"
        "pyrs_yaml\\pyrs_yaml.pyi:526:51: error[invalid-syntax-in-forward-annotation] Syntax error\n"
        "Found 2 diagnostics\n"
    )
    assert len(gate.parse_ty_findings(output)) == 2


def test_ty_output_in_another_file_is_not_ours(gate):
    assert gate.parse_ty_findings("user.py:3:1: error[invalid-argument-type] bad call\n") == []


def test_a_run_that_found_no_files_is_never_a_pass(gate):
    """`ty check <dir>` answers "All checks passed!" having examined nothing; that is exit 2."""
    output = "All checks passed!\nWARN No python files found under the given path(s)\n"
    assert gate.vacuous(output) is True
    assert gate.vacuous("Found 5 diagnostics\n") is False


@pytest.mark.parametrize("checker", ["mypy", "ty"])
def test_the_committed_stub_checks_clean_under_both_checkers(checker, tmp_path):
    """Both parsers against the real artifact, so neither can be a gate that quietly checks nothing.

    Skipped when the tool is absent - the gate's own answer in that case is exit 2, and a skip is the honest
    report rather than a pass earned by the checker not being installed.
    """
    if not available(checker):
        pytest.skip(f"{checker} is not installed here; CI runs it under an overlay")
    script = REPO_ROOT / "scripts" / "check_stub_types.py"
    result = subprocess.run(
        [sys.executable, str(script), "--checker", checker, "--stub", str(STUB)],
        capture_output=True,
        check=False,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    assert result.returncode == 0, result.stdout + result.stderr


# The known-bad artifact, named by refs that cannot move. The commit is the exact file this fix replaced, so it
# carries every spelling; the release tag is the fallback for a checkout that has tags but not that commit.
BROKEN_REFS = ("5ea2c0a3", "v0.17.0")


def broken_stub(tmp_path):
    """Fetch the bytes that actually shipped the defect, or return None if no pinned ref yields them.

    Two properties are checked rather than assumed: the ref must exist, and the file it names must still
    contain `u32` and `Py<PyAny>`. The second one is what keeps this control honest - a control pointed at a
    moving ref (`origin/main`, until this fix landed) stops describing a broken artifact exactly when the fix
    succeeds, and then either passes for the wrong reason or fails for the right one.
    """
    for ref in BROKEN_REFS:
        shown = subprocess.run(
            ["git", "show", f"{ref}:python/pyrs_yaml/pyrs_yaml.pyi"],
            capture_output=True,
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        if shown.returncode != 0:
            continue
        if "u32" not in shown.stdout or "Py<PyAny>" not in shown.stdout:
            continue
        path = tmp_path / "broken.pyi"
        path.write_text(shown.stdout, encoding="utf-8", newline="\n")
        return path
    return None


def test_the_gate_catches_the_artifact_that_was_broken(tmp_path):
    """Negative control against a release tag: `v0.17.0`'s shipped stub must fail the gate.

    Asserting against the artifact rather than a reconstruction is the point - the file is what maturin 1.14.1
    emitted, with `u32`, three `Py<PyAny>` forward annotations and an unimported `Callable` in it.
    """
    broken = broken_stub(tmp_path)
    if broken is None:
        pytest.skip("no pinned ref in this checkout carries the known-bad stub")
    script = REPO_ROOT / "scripts" / "check_stub_types.py"
    ran = 0
    for checker in ("mypy", "ty"):
        if not available(checker):
            continue
        ran += 1
        result = subprocess.run(
            [sys.executable, str(script), "--checker", checker, "--stub", str(broken)],
            capture_output=True,
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        findings = [line for line in (result.stdout + result.stderr).splitlines() if "pyrs_yaml.pyi" in line]
        assert result.returncode == 1, f"{checker} passed on the broken stub: {result.stdout[:200]}"
        # Not "some finding" but the finding this file is about: a control that passes because a checker
        # complained about something unrelated keeps biting while proving nothing. Measured across the two
        # checkers and the two pinned refs the counts differ (5 and 3), so the count is a floor and the
        # spelling is the assertion.
        assert any("u32" in line for line in findings), (checker, findings)
        assert len(findings) >= 3, (checker, findings)
    if ran == 0:
        pytest.skip("neither checker is installed here; the stub-drift CI job runs both")
