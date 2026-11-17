"""Gate the unreachable-seed measurement against the two ways it could lie.

`scripts/quality_matrix.py:seed_reach_gaps()` reads `.ci/seed-reach.txt` and emits one
`seed-unreachable:<target>` hole per target that has entries. Two failures are invisible in the
output unless something asks: a missing or empty registry reads as "no blind spots", and a registry
that no longer matches the seeds it names reads as a defence that grew. The Rust half
(`crates/pyrs-yaml-core/tests/seed_reach.rs`) holds the registry and the engine in agreement; this
module holds the *reader* honest - the same "a checker that measured nothing must not report a
pass" rule the other gates follow, applied to the file the matrix derives holes from.

The registry has been empty since 2026-11-17, when the last three `seed-unreachable` holes closed
by re-mining nine inputs and moving the tenth to `parse_yaml` (ledger (lu)). Emptiness is allowed
here only because the guard against "the measurement looked at nothing" moved into the Rust half,
where it checks every gated target still carries seeds and that `reaches` still refuses two known
shapes; what this module refuses is the file going missing or a line that is not a pair.
"""

from __future__ import annotations

import importlib.util
import sys
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT_PATH = REPO_ROOT / "scripts" / "quality_matrix.py"
REGISTRY = REPO_ROOT / ".ci" / "seed-reach.txt"


def _load_module():
    """Load the probe as a file, not as an import: `scripts/` is not a distribution.

    `tests/test_wheel_requirements_gate.py` reads module-scope imports to decide what the wheel
    legs must install, and a local helper imported by name would be demanded as a package -
    the shape `tests/test_quality_matrix.py` already avoids this way.
    """
    spec = importlib.util.spec_from_file_location("quality_matrix", SCRIPT_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_registry_exists_and_its_entries_are_plain_lines():
    """Absence is still a failure; emptiness is not, since 2026-11-17 it is the measurement.

    The registry was header-only from the day it was written until 2026-11-06, held ten lines, and
    was emptied again when the last three `seed-unreachable` holes closed by re-mining nine inputs
    and moving the tenth to `parse_yaml` (ledger (lu)). A file that has to be non-empty to pass
    cannot express that state, so the guard against "the measurement looked at nothing" lives where
    the measurement happens: `seed_reach.rs` asserts every gated target still carries seeds in its
    directory and keeps `reaches` refusing two known shapes. What this module still refuses is a
    registry that is missing, or whose lines are not `<target> <seed>` pairs.
    """
    assert REGISTRY.exists(), (
        f"{REGISTRY} is missing; `seed_reach_gaps()` would return no counts and the matrix would "
        "report no blind spot, which is a pass measured from nothing"
    )
    for line in REGISTRY.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        parts = stripped.split()
        assert len(parts) == 2, (
            f"{stripped!r} is not a `<target> <seed>` line, so the count it belongs to is not "
            "the count the matrix quotes"
        )


def test_the_gaps_counted_from_the_registry_match_its_lines():
    """The reader invariant, not the registry's current contents.

    The registry is empty since 2026-11-17 (ledger (lu)) and this module does not assert that:
    emptiness is a state the Rust half measures and the matrix derives from, so pinning it here
    would redden on a legitimate new hole while adding nothing the other two gates miss - the
    Rust gate refuses a registry that disagrees with its measurement in either direction, and
    `tests/test_quality_matrix.py` refuses a measured hole that `.ci/quality-holes.json` does not
    declare with an exit. What only this module can say is that the counts the matrix quotes are
    the lines the file carries.
    """
    counts = _load_module().seed_reach_gaps()
    entries = [
        line.split()
        for line in REGISTRY.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]
    assert counts == dict(Counter(target for target, _seed in entries)), (
        "the per-target counts the matrix quotes are not the lines the registry carries"
    )
    assert sum(counts.values()) == len(entries), "a line without a count is a line nobody reads"


def test_a_missing_registry_yields_no_holes_and_that_is_why_the_file_is_tracked(tmp_path):
    """The derivation is a reader, not a judge: assert what it does when the file is absent.

    Recording this here is the point - `quality_matrix.py` cannot tell an empty corpus from a
    deleted registry, so the suite is what keeps the file in the tree.
    """
    assert _load_module().seed_reach_gaps(tmp_path / "absent.txt") == {}
