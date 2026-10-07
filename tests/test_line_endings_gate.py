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
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_line_endings.py"


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


# ── the real tree ─────────────────────────────────────────────────────────────────


def test_repository_is_lf_only(gate, monkeypatch):
    monkeypatch.setattr(sys, "argv", [str(GATE_PATH)])
    assert gate.main() == 0, "a tracked text file still carries CRLF or a bare CR"
