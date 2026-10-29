"""The test group's version caps are checked against a recorded wheel measurement, not remembered.

A `python_version < 'X.Y'` on a counterparty library is a claim about PyPI. Too low and a scheduled
leg silently installs no oracle and runs fewer assertions; too high and the leg cannot install the
group at all. On 2026-10-28 five of the nine caps still read `< '3.14'` while `pyjson5`, `numpy`,
`ujson` and `python-rapidjson` each published 3.14 wheels, and the ≥3.14 legs were running 236 fewer
cross-library tests than the 3.13 leg — measured from the per-leg summaries of PR #665's twelve green
legs, and attributed per package by making each module unimportable in turn: `pyjson5` alone is 141
assertions, `orjson` 31, `numpy` 54, and `ujson`/`python-rapidjson`/`ryaml`/`yaml-edit`/`yaml-rs`
none of them on the ordinary legs. Nothing read the claim against the fact, because the claim lives in
`pyproject.toml` and the fact lives on PyPI.

`scripts/counterparty_wheels.py` records the fact per OS and band, and
`scripts/check_counterparty_caps.py` compares the two. The legs come from `scripts/ci_matrix_plan.py`
plus the free-threaded matrix in `ci.yml`, so the question is always asked of the interpreters CI
actually runs, and the PEP 508 asymmetry is respected: `3.14t` is admitted by the same
`python_version` test as `3.14`, which is why one missing free-threaded wheel keeps `orjson` and
`rtoml` capped below 3.14 while `pyjson5` moves to 3.15.

Every rule is tested by mutation as well as by reading: a checker that cannot fail is the same defect
as the cap that went stale.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
CHECKER = REPO / "scripts" / "check_counterparty_caps.py"
PROBE = REPO / "scripts" / "counterparty_wheels.py"
RECORD = REPO / ".ci" / "counterparty-wheels.json"
PYPROJECT = REPO / "pyproject.toml"
CI_PLAN = REPO / "scripts" / "ci_matrix_plan.py"


def run_checker(pyproject: pathlib.Path, record: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [
            sys.executable,
            str(CHECKER),
            "--pyproject",
            str(pyproject),
            "--record",
            str(record),
            "--ci",
            str(REPO / ".github" / "workflows" / "ci.yml"),
            "--plan",
            str(CI_PLAN),
        ],
        capture_output=True,
        text=True,
        check=False,
    )


def mutated_manifest(tmp_path: pathlib.Path, old: str, new: str) -> pathlib.Path:
    """The live pyproject with one cap rewritten, and a guard that the rewrite found its target."""
    text = PYPROJECT.read_text(encoding="utf-8")
    assert text.count(old) == 1, f"the control expects to rewrite one cap, found: {text.count(old)}"
    path = tmp_path / "pyproject.toml"
    path.write_text(text.replace(old, new), encoding="utf-8")
    return path


def test_the_live_caps_match_the_record() -> None:
    proc = run_checker(PYPROJECT, RECORD)
    assert proc.returncode == 0, proc.stdout + proc.stderr
    assert proc.stdout.startswith("OK:"), proc.stdout


def test_a_cap_admitting_a_band_with_no_wheel_is_refused(tmp_path: pathlib.Path) -> None:
    # orjson has a 3.14 wheel and no 3.14t wheel, and PEP 508 admits both with one comparison, so a
    # cap of < '3.15' would leave the free-threaded leg unable to install the group.
    proc = run_checker(
        mutated_manifest(
            tmp_path,
            "\"orjson>=3.9; python_version >= '3.9' and python_version < '3.14'",
            "\"orjson>=3.9; python_version >= '3.9' and python_version < '3.15'",
        ),
        RECORD,
    )
    assert proc.returncode == 1, proc.stdout + proc.stderr
    assert "orjson admits linux 3.14t" in proc.stdout, proc.stdout
    assert "cannot install the test group" in proc.stdout, proc.stdout


def test_a_cap_below_a_band_with_wheels_everywhere_is_refused(tmp_path: pathlib.Path) -> None:
    # This is the original defect: pyjson5 at < '3.14' while 3.14 and 3.14t both have wheels.
    proc = run_checker(
        mutated_manifest(
            tmp_path,
            "\"pyjson5>=1.6; python_version >= '3.9' and python_version < '3.15'",
            "\"pyjson5>=1.6; python_version >= '3.9' and python_version < '3.14'",
        ),
        RECORD,
    )
    assert proc.returncode == 1, proc.stdout + proc.stderr
    assert "pyjson5 caps at < '3.14'" in proc.stdout, proc.stdout
    assert "throwing away coverage" in proc.stdout, proc.stdout


def test_a_capped_requirement_with_no_measurement_is_refused(tmp_path: pathlib.Path) -> None:
    text = PYPROJECT.read_text(encoding="utf-8")
    anchor = "    \"tomli>=2.0; python_version < '3.11'\",\n"
    assert anchor in text
    path = tmp_path / "pyproject.toml"
    path.write_text(
        text.replace(anchor, anchor + "    \"some-new-oracle>=1.0; python_version < '3.13'\",\n"),
        encoding="utf-8",
    )
    proc = run_checker(path, RECORD)
    assert proc.returncode == 1, proc.stdout + proc.stderr
    assert "some-new-oracle carries a cap and the record holds no row" in proc.stdout, proc.stdout


def test_a_record_with_no_rows_refuses_instead_of_passing(tmp_path: pathlib.Path) -> None:
    empty = tmp_path / "counterparty-wheels.json"
    empty.write_text(json.dumps({"rows": {}}) + "\n", encoding="utf-8")
    proc = run_checker(PYPROJECT, empty)
    assert proc.returncode == 2, proc.stdout + proc.stderr
    assert "cannot measure" in proc.stdout, proc.stdout


def test_the_probe_and_the_manifest_name_the_same_capped_counterparties() -> None:
    """A new capped counterparty has to be probed, not just capped.

    The record's own rows are the measurement set; the probe's table is the intention. Requiring them
    to agree, and each to cover every capped test-group requirement except the ones recorded as
    bounded by something other than a wheel, is what stops a tenth cap from arriving unmeasured.
    """
    spec = importlib.util.spec_from_file_location("counterparty_wheels", PROBE)
    assert spec and spec.loader
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)

    payload = json.loads(RECORD.read_text(encoding="utf-8"))
    measured = {name for per_os in payload["rows"].values() for per_band in per_os.values() for name in per_band}
    assert measured == set(probe.COUNTERPARTIES), (measured, set(probe.COUNTERPARTIES))
    assert payload["rows"], "the record holds no rows"
    assert all(
        entry["state"] in {"wheel", "no-wheel"}
        for per_os in payload["rows"].values()
        for per_band in per_os.values()
        for entry in per_band.values()
    ), "an unmeasured row was committed: re-run the probe until every cell answers"

    checker_spec = importlib.util.spec_from_file_location("check_counterparty_caps", CHECKER)
    assert checker_spec and checker_spec.loader
    checker = importlib.util.module_from_spec(checker_spec)
    checker_spec.loader.exec_module(checker)
    requirements = checker.test_group_requirements(PYPROJECT.read_text(encoding="utf-8"))
    capped = {name for name, marker in requirements.items() if checker.CAP_RE.search(marker)}
    exceptions = set(payload.get("not_wheel_bounded", {}))
    assert capped == measured | exceptions, (sorted(capped), sorted(measured | exceptions))
