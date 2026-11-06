"""Gate the unreachable-seed measurement against the two ways it could lie.

`scripts/quality_matrix.py:seed_reach_gaps()` reads `.ci/seed-reach.txt` and emits one
`seed-unreachable:<target>` hole per target that has entries. Two failures are invisible in the
output unless something asks: a missing or empty registry reads as "no blind spots", and a registry
that no longer matches the seeds it names reads as a defence that grew. The Rust half
(`crates/pyrs-yaml-core/tests/seed_reach.rs`) holds the registry and the engine in agreement; this
module holds the *reader* honest - the same "a checker that measured nothing must not report a
pass" rule the other gates follow, applied to the file the matrix derives holes from.
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


def test_the_registry_exists_and_is_not_an_empty_pass():
    assert REGISTRY.exists(), (
        f"{REGISTRY} is missing; `seed_reach_gaps()` would return no counts and the matrix would "
        "report no blind spot, which is a pass measured from nothing"
    )
    lines = [
        line.strip()
        for line in REGISTRY.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]
    assert lines, "every entry is commented out: the registry cannot vouch for a hole it does not list"


def test_the_gaps_counted_from_the_registry_match_its_lines():
    counts = _load_module().seed_reach_gaps()
    entries = [
        line.split()
        for line in REGISTRY.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]
    assert counts == dict(Counter(target for target, _seed in entries)), (
        "the per-target counts the matrix quotes are not the lines the registry carries"
    )
    assert set(counts) == {"yaml_roundtrip", "toml_roundtrip", "json_roundtrip"}, counts
    assert sum(counts.values()) == len(entries), "a line without a count is a line nobody reads"


def test_a_missing_registry_yields_no_holes_and_that_is_why_the_file_is_tracked(tmp_path):
    """The derivation is a reader, not a judge: assert what it does when the file is absent.

    Recording this here is the point - `quality_matrix.py` cannot tell an empty corpus from a
    deleted registry, so the suite is what keeps the file in the tree.
    """
    assert _load_module().seed_reach_gaps(tmp_path / "absent.txt") == {}
