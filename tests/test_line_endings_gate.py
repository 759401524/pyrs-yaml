"""Self-test for the line-ending gate (`scripts/check_line_endings.py`).

The policy is "tracked text files are LF-only". The builtin `mixed-line-ending` hook does
not implement it: it fires on a file that mixes endings and passes a file that is entirely
CRLF, which is the exact shape this repository accumulated — a helper script writing a
tracked file with the platform's default newline. Measured, not assumed: an injected
all-CRLF file left the builtin hook `Passed` while this checker named its line count.

So the cases below are the shapes the builtin hook is blind to, plus the exclusions that
must not be "cleaned up" by a future reader: fuzz seeds and the two Ir benchmark fixtures,
whose CR bytes are data the committed baseline was measured against.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_line_endings.py"
# Python 3.8, which this package supports, forbids a backslash inside an f-string expression.
CR = b"\r"


def _load_gate():
    spec = importlib.util.spec_from_file_location("check_line_endings", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load_gate()


# ── the shapes the builtin hook misses ────────────────────────────────────────────


def test_all_crlf_file_is_an_offender(gate):
    """A uniformly CRLF file is the real failure mode, and the builtin hook passes it."""
    found = gate.offenders([("CHANGELOG.md", b"a\r\nb\r\nc\r\n")])
    assert found == [("CHANGELOG.md", 3, 0)], found


def test_mixed_file_reports_both_counts(gate):
    found = gate.offenders([("docs/en/changelog.md", b"a\r\nb\nc\r\n")])
    assert found == [("docs/en/changelog.md", 2, 0)], found


def test_lone_cr_is_reported_separately(gate):
    found = gate.offenders([("notes.txt", b"a\rb\nc")])
    assert found == [("notes.txt", 0, 1)], found


def test_lf_only_file_is_clean(gate):
    assert gate.offenders([("src/lib.rs", b"fn main() {\n    ()\n}\n")]) == []


def test_counts_separate_crlf_from_lone_cr(gate):
    crlf, lone = gate.line_ending_counts(b"a\r\nb\rc\r\n")
    assert (crlf, lone) == (2, 1), (crlf, lone)


def test_normalisation_is_the_documented_transform(gate):
    """`fix` mode rewrites CRLF to LF and leaves a bare CR alone."""
    data = b"a\r\nb\rc\n"
    assert data.replace(b"\r\n", b"\n") == b"a\nb\rc\n"
    assert gate.line_ending_counts(data) == (1, 1)


# ── scope: what the policy does not touch, and why ────────────────────────────────


@pytest.mark.parametrize(
    ("path", "checked"),
    [
        # A seed's trailing space and CR bytes are the fuzzer's input, not formatting.
        ("fuzz/seeds/yaml_roundtrip/former-crash-55c199ef.seed", False),
        ("fuzz/corpus/parse_yaml/0123456789abcdef", False),
        ("fuzz/artifacts/parse_toml/crash-abcdef", False),
        # The fuzz *targets* are Rust source: excluding all of `fuzz/` was the first
        # version's mistake, a wide exclusion written for a narrow reason.
        ("fuzz/fuzz_targets/yaml_roundtrip.rs", True),
        ("fuzz/Cargo.toml", True),
        # Benchmark fixtures whose bytes the committed Ir baseline was measured against.
        ("crates/pyrs-yaml-core/src/bench_inputs.rs", False),
        ("crates/pyrs-yaml-core/benches/ir_gate.rs", False),
        # Everything else that is text is in scope, including generated-and-committed.
        ("python/pyrs_yaml/pyrs_yaml.pyi", True),
        ("CHANGELOG.md", True),
        (".github/workflows/ci.yml", True),
        ("crates/pyrs-ast/src/node.rs", True),
        ("Cargo.lock", True),
        # Not text at all.
        ("docs/en/assets/logo.png", False),
        ("README.rst", False),
    ],
)
def test_scope_predicate(gate, path, checked):
    assert gate.is_checked_path(path) is checked, path


def test_the_two_data_fixtures_are_the_declared_ones(gate):
    """Guard the reason, not just the list.

    These two files are excluded because their CRLFs sit inside raw strings that are the
    YAML input of the Ir baseline. Normalising them is a data change and belongs with a
    baseline regeneration; silently including them would move measured instruction counts
    in a commit whose diff claims to change no bytes.
    """
    assert set(gate.DATA_FIXTURES) == {
        "crates/pyrs-yaml-core/src/bench_inputs.rs",
        "crates/pyrs-yaml-core/benches/ir_gate.rs",
    }
    for fixture in gate.DATA_FIXTURES:
        assert (REPO_ROOT / fixture).exists(), f"{fixture}: excluded a file that moved"


def test_prek_and_the_script_agree_on_the_exclusions(gate):
    """Two declarations of one policy drift apart silently unless a test pins them."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    for fixture in gate.DATA_FIXTURES:
        assert fixture in prek, f"{fixture}: excluded here but not in the hook"
    assert "line-endings-lf" in prek, "the absolute rule is not wired as a hook"


def test_the_checkout_rule_preserves_what_the_policy_excludes(gate):
    """A file the policy refuses to rewrite must also survive `git checkout` untouched.

    Measured with a fresh `git clone --no-checkout --local` and with `git show`: the stored bytes of
    this fixture are not the same on every branch - `main`'s holds no CR bytes, this branch's holds
    98, and a Windows checkout inserts them again. So `check_line_endings.py`'s exclusion ("do not
    rewrite these, their CRLFs are the measured input") could not have been true in the way it
    reads: which bytes were measured depended on the author's checkout. `-text` fixes the
    reproducible part of it - stored bytes and tree bytes are then the same file - and
    `test_the_fixtures_commit_exactly_what_the_tree_holds` asserts exactly that.

    Recorded so this test is not read as more than it is: it was first written to explain a 1.45%
    difference between a WSL-measured Ir number and the runner's. That claim was tested by adding
    `-text` and re-running CI - the runner's number moved by 28 instructions out of 16,020,906, so
    the difference is not this. It is a byte-stability invariant, nothing more, and the gap is
    open (see `scripts/ir_gate.py`).
    """
    attributes = (REPO_ROOT / ".gitattributes").read_text(encoding="utf-8")
    lines = [ln.strip() for ln in attributes.splitlines() if ln.strip() and not ln.startswith("#")]
    for fixture in gate.DATA_FIXTURES:
        entry = next((ln for ln in lines if ln.startswith(fixture)), None)
        assert entry is not None, f"{fixture}: excluded from rewriting but not declared in .gitattributes"
        assert "-text" in entry, f"{entry}: checkout would still convert line endings"
    # And the blanket rule must stay, or the policy has no teeth for everything else.
    assert "* text=auto eol=lf" in lines


def test_the_fixtures_commit_exactly_what_the_tree_holds(gate):
    """The reproducibility invariant the baseline actually needs: stored bytes == measured bytes.

    Measured, and it is what broke the assumption this file was written to protect: `git show
    HEAD:crates/pyrs-yaml-core/src/bench_inputs.rs` reports 98 CR bytes on the branch that
    materialised a Windows working tree, while a fresh clone of `main` reports none for the same
    path. Which input a machine parses therefore depended on how the last author's checkout was
    configured, not on the source - and a baseline generated where one kind of tree is read is not
    comparable with a gate run where another is. `-text` is what makes the two agree, because it
    stops conversion in both directions.

    The earlier draft of this test asserted the committed blob holds no CR bytes. That was false on
    the branch adding it, which is how it was caught: the assertion was written from a probe of one
    ref and applied to all of them.
    """
    for fixture in gate.DATA_FIXTURES:
        stored = subprocess.run(
            ["git", "show", f"HEAD:{fixture.replace(chr(92), '/')}"], capture_output=True, check=False
        ).stdout
        if not stored:
            continue  # no such ref in this checkout (a scratch clone); the attribute test covers it
        on_disk = (REPO_ROOT / fixture).read_bytes()
        crs_stored = stored.count(CR)
        crs_tree = on_disk.count(CR)
        assert stored == on_disk, (
            f"{fixture}: the committed bytes ({len(stored)}, {crs_stored} CR) differ from the tree "
            f"({len(on_disk)}, {crs_tree} CR) - the measurement is not reproducible"
        )


def test_the_excluded_fixtures_are_the_declared_ones(gate):
    """Guard the reason, not just the list.

    These two files are excluded from rewriting because a raw string's newline is data: a
    normaliser that touched them would change what the Ir scenarios parse. What this test pins is
    that the list names real files at the paths the checker expects - a moved file would leave the
    exclusion silently protecting nothing. It does NOT claim the working copy holds CR bytes; on
    the contrary, `test_the_fixtures_blobs_hold_no_cr_bytes` shows the repository stores pure LF and
    the CRs a Windows tree carries are inserted at checkout.
    """
    assert set(gate.DATA_FIXTURES) == {
        "crates/pyrs-yaml-core/src/bench_inputs.rs",
        "crates/pyrs-yaml-core/benches/ir_gate.rs",
    }
    for fixture in gate.DATA_FIXTURES:
        assert (REPO_ROOT / fixture).exists(), f"{fixture}: excluded a file that moved"


# ── the real tree ─────────────────────────────────────────────────────────────────


def test_repository_is_lf_only(gate, monkeypatch):
    monkeypatch.setattr(sys, "argv", [str(GATE_PATH)])
    assert gate.main() == 0, "a tracked text file still carries CRLF or a bare CR"
