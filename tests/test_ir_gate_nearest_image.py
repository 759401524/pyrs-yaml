"""The Ir gate names the nearest image row next to a failure on a non-runner host.

(script-ir-gate-failure-avoid-a-stale-verdict) measured while writing the #111 mitigation: a local
reading is judged against the flat table, which carries no verdict, so the script now names the
nearest image row next to the failure - the verdict that row would have refused - without changing
the exit code or the verdict itself. The helper is pure, so the verdict dressing is held here:
the case that matters is that the nearest row is chosen by smallest below-fraction and not by
arbitrary order, and that a name absent from every image floor has nothing to name.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE = REPO_ROOT / "scripts" / "ir_gate.py"


def _gate():
    spec = importlib.util.spec_from_file_location("ir_gate", GATE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _floors(**rows: dict[str, int]) -> dict[str, dict[str, int]]:
    return {
        "github-runner ubuntu24, image 20261026.354.1": rows,
        "github-runner ubuntu24, image 20261023.334.3": rows,
    }


def test_nearest_row_chosen_by_smallest_below_fraction():
    module = _gate()
    floors = _floors(
        load_toml_medium=142_478_660,
        load_toml_long_strings=139_573_881,
    )
    nearest = module.nearest_image_row(floors, "load_toml_medium", 139_573_881)
    assert nearest is not None, "a measured value must name a row"
    key, base, below = nearest
    # The 20261026.354.1 row sits 2.99% below the value on both floors; 20261023.334.3 sits far below
    # the shorter floor, so the two-rows spread has to move it to the nearer one.
    assert key == "github-runner ubuntu24, image 20261026.354.1"
    assert base == 142_478_660
    assert abs(below - (142_478_660 - 139_573_881) / 142_478_660) < 1e-12


def test_nearest_row_skips_names_absent_from_every_floor():
    module = _gate()
    floors = _floors(load_toml_medium=142_478_660)
    assert module.nearest_image_row(floors, "serialize_small", 1_000) is None
    assert module.nearest_image_row({}, "load_toml_medium", 1_000) is None


def test_nearest_row_skips_zero_or_absent_bases():
    module = _gate()
    floors = _floors(load_toml_medium=0, load_toml_long_strings=139_573_881)
    # A zero base is skipped, so the query names nothing for that name: the row is absent from every
    # image floor and the helper returns None rather than a row built on a zero floor.
    assert module.nearest_image_row(floors, "load_toml_medium", 139_573_881) is None
    assert module.nearest_image_row(floors, "serialize_small", 139_573_881) is None
    assert module.nearest_image_row(floors, "load_toml_long_strings", 139_573_881) is not None
