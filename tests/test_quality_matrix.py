"""Gate the quality-defence matrix against its own declared blind spots.

`scripts/quality_matrix.py` measures which defence tiers exist per format, which of
them can stop a merge, and what each is blind to. `tests/../.ci/quality-holes.json`
is the registry of holes that measurement found. This module is what makes either of
them more than documentation:

* a hole the measurement finds and the registry does not declare fails the suite, so a
  new blind spot has to be written down with an exit criterion before it can ship;
* a declared hole the measurement no longer finds fails the suite too, so the registry
  cannot quietly rot into a list of things that were fixed long ago;
* `QUALITY_MATRIX.md` has to carry exactly the registered ids, so the document that
  describes the defence is checked rather than trusted;
* the probes themselves are tested against the specific ways they were first written
  wrong, because an instrument that reports "no hole" for the wrong reason is worse
  than no instrument.

Everything here is a repository-state assertion or a pure-function check: no build, no
network, no fuzz run, so it costs milliseconds and lands in the tier CI already runs.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT_PATH = REPO_ROOT / "scripts" / "quality_matrix.py"
REGISTRY_PATH = REPO_ROOT / ".ci" / "quality-holes.json"
DOC_PATH = REPO_ROOT / "QUALITY_MATRIX.md"


def _load_module():
    spec = importlib.util.spec_from_file_location("quality_matrix", SCRIPT_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def matrix():
    return _load_module()


@pytest.fixture(scope="module")
def measured(matrix):
    return matrix.measure()


@pytest.fixture(scope="module")
def registry():
    return json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))


def ids(holes):
    """Normalise measured triples and registry entries onto the same `kind:name` id."""
    return {f"{kind}:{name}" for kind, name, _why in holes}


# ── the gate: measured set == declared set, in both directions ────────────────────


def test_measured_holes_match_the_registry(measured, registry):
    declared = {entry["id"] for entry in registry["holes"]}
    found = ids(measured["holes"])
    new = sorted(found - declared)
    stale = sorted(declared - found)
    lines = []
    if new:
        lines.append(
            "undeclared blind spot(s) - add an entry to .ci/quality-holes.json with an "
            "exit criterion, or close the hole:"
        )
        lines.extend(f"  + {hole}" for hole in new)
    if stale:
        lines.append(
            "registered hole(s) the measurement can no longer reproduce - the defence "
            "grew, delete the entry so the registry stays true:"
        )
        lines.extend(f"  - {hole}" for hole in stale)
    assert not lines, "\n".join(lines)


def test_registry_entries_are_actionable(registry):
    seen = set()
    for entry in registry["holes"]:
        hole_id = entry["id"]
        assert hole_id.count(":") == 1, f"{hole_id}: id must be exactly '<kind>:<name>'"
        assert hole_id not in seen, f"{hole_id}: registered twice"
        seen.add(hole_id)
        assert entry["opened"], f"{hole_id}: an undated hole cannot be aged"
        exit_criterion = entry["exit"]
        assert len(exit_criterion) > 40, f"{hole_id}: the exit criterion is not concrete"
        assert not exit_criterion.lower().startswith(("fix ", "improve ", "todo")), (
            f"{hole_id}: {exit_criterion!r} names an action, not a checkable state"
        )


# ── the document carries exactly the registered holes ─────────────────────────────


def test_doc_lists_every_registered_hole(measured, registry):
    assert DOC_PATH.exists(), "QUALITY_MATRIX.md is the assessment this gate enforces"
    doc = DOC_PATH.read_text(encoding="utf-8")
    declared = {entry["id"] for entry in registry["holes"]}
    missing = sorted(declared - {hole for hole in declared if hole in doc})
    assert not missing, f"QUALITY_MATRIX.md does not mention: {missing}"
    # Any `kind:name` token in the document that the measurement does not report is a
    # claim about a blind spot nobody measured, which is how an assessment drifts.
    kinds = {kind for kind, _name, _why in measured["holes"]}
    stray = sorted(
        token
        for token in (line.strip("`") for line in doc.replace("`", "\n").splitlines())
        if ":" in token and token.split(":")[0] in kinds and token not in declared
    )
    assert not stray, f"QUALITY_MATRIX.md names unmeasured hole(s): {stray}"


# ── instrument accuracy: each probe tested against the way it was first wrong ──────


def test_run_commands_ignores_mentions_in_comments_and_quotes(matrix):
    """Reachability must mean "a step executes it", not "the file says it".

    The first version searched the whole workflow text, so a hook named in an
    explanatory comment counted as wired: exactly the mistake that left the hook tier
    reported as covered while no job ever ran it. The same trap in the other direction is
    a step that *prints* a command name — an `echo` of a plan is not an execution.
    """
    text = "\n".join(
        [
            "# prek run --all-files is what we mean to run someday",
            "jobs:",
            "  a:",
            "    steps:",
            "      - name: Run it",
            "        run: prek run --all-files",
            "      - name: Explain",
            "        run: |",
            '          echo "cargo clippy is owned by ci.yml"',
            "      - name: Plan",
            "        run: >",
            "          echo 'cargo fmt --check happens later'",
        ]
    )
    commands = matrix.run_commands(text)
    assert "prek run --all-files" in " ".join(commands)
    joined = " ".join(commands)
    assert "cargo clippy" not in joined, "a quoted mention must not count as a gate"
    assert "cargo fmt" not in joined, "an echo of a command is not the command"


def test_run_commands_captures_folded_and_inline_steps(matrix):
    text = "\n".join(
        [
            "steps:",
            "  - run: >",
            "      sudo apt-get install -y valgrind",
            "  - run: python3 scripts/ir_gate.py",
        ]
    )
    joined = " ".join(matrix.run_commands(text))
    assert "valgrind" in joined
    assert "ir_gate.py" in joined


def test_engine_detection_catches_the_prefixed_writer_names(matrix):
    """`to_json_text` is a writer; matching `to_json\\b` missed it."""
    for name in ("to_json_text", "to_json5_text_pretty", "to_toml", "dump", "serialize"):
        assert matrix.SERIALIZER_RE.search(f"pub fn {name}(node: &CustomNode) -> String {{"), name


def test_fuzz_denominator_is_the_harness_dependency_list(matrix):
    crates = matrix.fuzz_crates()
    assert crates, "the fuzz harness links no crate: the matrix would report zero engines"
    # A crate the harness links is, by construction, present under crates/.
    for crate in crates:
        assert (REPO_ROOT / "crates" / crate / "Cargo.toml").exists(), crate


def test_roundtrip_requirement_is_per_format_family(matrix):
    targets = ["yaml_roundtrip", "parse_json"]
    assert matrix.roundtrip_target_for("pyrs-yaml-core", targets)
    assert matrix.roundtrip_target_for("pyrs-ast", targets)
    assert not matrix.roundtrip_target_for("pyrs-json", targets)


def test_ir_scenario_probe_reads_a_wrapped_tuple(matrix, measured):
    """The first probe matched `("name",` and lost a scenario whose name wrapped."""
    bench = measured["ir_scenarios_bench"]
    baseline = measured["ir_scenarios_baseline"]
    assert "serialize_block_scalars" in bench, bench
    assert bench == baseline, "bench and baseline scenario sets must agree by construction"


def test_property_probe_finds_the_real_properties(matrix, measured):
    names = measured["property_functions"]
    assert measured["property_function_count"] == len(names)
    assert any(name.endswith("::prop_tag_preserved") for name in names), names
    # The three writer fixed points behind the `property-tier` hole must be nameable,
    # or the hole's exit criterion could never be checked.
    for prop in (
        "prop_json_writers_are_fixed_points",
        "prop_json5_writer_is_fixed_point",
        "prop_toml_writer_is_fixed_point",
    ):
        assert any(name.endswith(f"::{prop}") for name in names), prop


def test_the_perf_coverage_probe_describes_a_real_boundary(matrix, monkeypatch):
    """Discrimination for `perf-coverage:binding-layer`, the newest registered hole.

    The probe compares the crates the reproducible instrument can link against the crate that serves
    the Python API. If it fired whatever the build graph contained, it would be decoration, so the
    test moves the graph under it: claim the serving crate is one the harness already links, and the
    hole has to disappear from a fresh measurement of the same tree.
    """
    graph = matrix.ir_harness_build_graph()
    assert "pyrs-yaml-core" in graph, "the bench owner dropped out of its own build graph"
    assert "pyrs-yaml" not in graph, "the registered hole went stale: the binding is now linked"
    assert matrix.binding_crate() == "pyrs-yaml", "the crate was found by name, not by layout"

    monkeypatch.setattr(matrix, "binding_crate", lambda: "pyrs-yaml-core")
    reported = [hole for hole in matrix.measure()["holes"] if hole[0] == "perf-coverage"]
    assert reported == [], "the probe reports the gap regardless of the graph it is measuring"


def test_the_graph_probe_reads_every_dependency_section(matrix):
    """The set the registered hole describes, pinned by content.

    `to_json_medium` and `to_toml_medium` link the sibling engines through
    `[dev-dependencies]`, so a probe that stops at the first section understates what the
    instrument can reach - which is exactly what this did when it first shipped, reporting three
    crates while the hole's own text named five. The `why` of the registered hole enumerates the
    graph, so the enumeration is now checked instead of quoted.
    """
    graph = matrix.ir_harness_build_graph()
    assert {"pyrs-yaml-core", "pyrs-ast", "pyrs-schema", "pyrs-json", "pyrs-toml"} <= graph, graph
    assert "pyrs-yaml-cli" not in graph, "the CLI is not linked by the harness either way"
    registered = json.loads((REPO_ROOT / ".ci" / "quality-holes.json").read_text(encoding="utf-8"))["holes"]
    for hole in registered:
        if hole["id"].startswith("perf-coverage"):
            for crate in sorted(graph & {"pyrs-json", "pyrs-toml", "pyrs-ast", "pyrs-schema", "pyrs-yaml-core"}):
                assert crate in hole["why"], f"{crate}: in the measured graph, missing from the registry text"


def test_holes_are_derived_not_transcribed(measured):
    """The measurement has to be reproducible twice in one process, and stable."""
    again = _load_module().measure()
    assert again["holes"] == measured["holes"]
    for kind, name, why in measured["holes"]:
        assert kind and name and why, (kind, name, why)


def test_declared_hole_kinds_are_kinds_the_probe_can_emit(measured, registry):
    measurable = {kind for kind, _name, _why in measured["holes"]}
    declared = {entry["id"].split(":")[0] for entry in registry["holes"]}
    assert declared <= measurable, (
        f"registry declares kind(s) the measurement cannot emit: {sorted(declared - measurable)}"
    )
