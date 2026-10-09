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


def test_ir_scenario_probe_reads_every_harness(matrix, measured):
    """Both channels, or the second one's numbers would be compared against nothing.

    The probe that shipped named one file - `crates/pyrs-yaml-core/benches/ir_gate.rs`. Adding the
    binding harness would then have left `to_python_*` out of the bench set, so `--update` could write
    a baseline without them and the `ir-unbaselined` probe could never fire for the channel it exists
    to protect. That is the same shape as the graph probe stopping at the first `[dependencies]`
    section (#303), measured here instead of assumed.
    """
    owners = matrix.ir_harness_owners()
    assert owners == ["pyrs-yaml", "pyrs-yaml-core"], owners
    channels = matrix.ir_harness_channels()
    assert {"to_python_small", "to_python_medium", "to_python_anchors"} <= set(channels["pyrs-yaml"]), channels
    # An owner that yields no scenario names is the empty-set failure again: the probe read the file
    # and found nothing, which has to look like a finding rather than like a channel with no work.
    assert all(channels[crate] for crate in owners), channels
    assert "to_python_small" in measured["ir_scenarios_bench"], measured["ir_scenarios_bench"]


def test_the_gate_measures_both_halves_of_every_bridge(matrix, measured):
    """A writer without its reader is a half-measured path, and #292's cost sat in the missing half.

    `to_toml_medium` numbers the outbound bridge only. The inbound bridge is where a key's meaning is
    decided - `load_toml` has to quote a key that either YAML schema would re-type - so per-key work
    added there moved nothing the gate could observe, and the change was adjudicated on wall-clock
    CodSpeed instead. The rule is the pair, not the name: a future `to_yaml_*` earns a `from_yaml_*`.
    """
    channels = matrix.ir_harness_channels()
    assert matrix.ir_bridge_pairs(channels) == [], channels
    assert matrix.ir_bridges_absent(channels) == [], channels
    bench = set(measured["ir_scenarios_bench"])
    assert {"to_json_medium", "from_json_medium", "to_toml_medium", "from_toml_medium"} <= bench, bench
    # The two dialects the hub also reads and writes. Their halves were all four missing until this
    # line existed: comment skipping, the unquoted-key grammar and both dialect writers carried no
    # instruction count, and the paired-half rule could not see that because it starts from a name.
    assert {
        "to_jsonc_medium",
        "from_jsonc_medium",
        "to_json5_medium",
        "from_json5_medium",
    } <= bench, bench
    # `to_python_*` is the language binding, not a text format, and has no reader twin; a probe that
    # invented a `from_python_*` requirement would open its first hole on itself.
    assert "to_python_medium" in bench and "from_python_medium" not in bench, bench


def test_a_format_with_no_bridge_at_all_is_a_hole(matrix, monkeypatch):
    """The other half of the same blindness, fired on injected names.

    Removing one scenario leaves a unidirectional bridge, which the rule above catches. Removing a
    whole dialect leaves nothing for that rule to start from - so this test drops all four JSON5
    scenarios and requires the hole to appear by format name, not by scenario name.
    """
    real = matrix.ir_harness_channels()
    dropped = ("to_json5_medium", "from_json5_medium")
    thin = {crate: [name for name in names if name not in dropped] for crate, names in real.items()}
    assert matrix.ir_bridges_absent(thin) == ["json5"], thin

    monkeypatch.setattr(matrix, "ir_harness_channels", lambda: thin)
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("ir-bridge-absent", "json5") in holes, sorted(holes)


def test_a_bridge_measured_one_way_is_a_hole(matrix, monkeypatch):
    """Both directions bite, or the rule is a comment about the names rather than a measurement."""
    real = matrix.ir_harness_channels()

    def thin(drop: str) -> dict:
        return {crate: [name for name in names if name != drop] for crate, names in real.items()}

    monkeypatch.setattr(matrix, "ir_harness_channels", lambda: thin("from_toml_medium"))
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("ir-bridge-unidirectional", "to_toml_medium") in holes, sorted(holes)

    monkeypatch.setattr(matrix, "ir_harness_channels", lambda: thin("to_json_medium"))
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("ir-bridge-unidirectional", "from_json_medium") in holes, sorted(holes)


def test_a_harness_the_gate_cannot_build_is_a_hole(matrix, monkeypatch):
    """The harness set is measured in both directions, and each direction has to bite.

    A declared target whose file is gone shortens the gate silently. A file whose crate declares no
    target is worse, because it looks like coverage: its scenarios would sit in the baseline as
    numbers that nothing compiles and nobody re-measures.
    """
    monkeypatch.setattr(matrix, "ir_harness_files", lambda: ["pyrs-yaml"])
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("ir-harness-missing", "pyrs-yaml-core") in holes, sorted(holes)

    monkeypatch.setattr(matrix, "ir_harness_files", lambda: ["pyrs-yaml", "pyrs-yaml-core", "pyrs-json"])
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("ir-harness-undeclared", "pyrs-json") in holes, sorted(holes)


def test_one_scenario_name_in_two_harnesses_is_a_hole(matrix, monkeypatch):
    """Ambiguity is a finding, not a tie-break.

    `ir_gate.py` refuses to run on a collision. That refusal is the last line of defence only if the
    probe also reports it: otherwise a duplicated name quietly measures one channel against the other
    channel's committed number.
    """
    monkeypatch.setattr(
        matrix,
        "ir_harness_channels",
        lambda: {"pyrs-yaml-core": ["parse_small"], "pyrs-yaml": ["parse_small", "to_python_small"]},
    )
    reported = [hole for hole in matrix.measure()["holes"] if hole[0] == "ir-scenario-duplicate"]
    assert [hole[1] for hole in reported] == ["parse_small"], reported


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
    """The probe is a measurement, in both the open and the closed direction.

    It was registered by #302 as the gap where `ir_gate` could not reach the crate serving the Python
    API; #305 added `crates/pyrs-yaml/benches/ir_gate.rs` and the hole disappeared from the
    derivation without anyone editing the registry to say so. Both directions need to be checkable, or
    the probe is a comment:

    * closed today: the binding crate is in the build graph, and a fresh measurement reports no
      `perf-coverage` hole;
    * still able to bite: claim the Python-facing crate is some layer the harness does not link, and
      the hole comes back on the same tree.

    The registry entry was deleted in the same changeset that closed the gap, which is what
    `test_measured_holes_match_the_registry` enforces from the other side - a stale registered hole
    reads as an open one.
    """
    graph = matrix.ir_harness_build_graph()
    served = matrix.binding_crate()
    assert served == "pyrs-yaml", "the crate is found by name, not by layout"
    assert served in graph, f"{served} left the harness graph again: {sorted(graph)}"
    assert [hole for hole in matrix.measure()["holes"] if hole[0] == "perf-coverage"] == []

    monkeypatch.setattr(matrix, "binding_crate", lambda: "pyrs-some-unlinked-layer")
    reported = [hole for hole in matrix.measure()["holes"] if hole[0] == "perf-coverage"]
    assert len(reported) == 1, "the probe cannot detect the gap it detected before, so it detects nothing"
    assert "pyrs-some-unlinked-layer" in reported[0][2], reported[0]


def test_the_graph_probe_reads_every_dependency_section(matrix):
    """Every workspace crate the harness links, read from every dependency section.

    `to_json_medium` and `to_toml_medium` reach the sibling engines through
    `[dev-dependencies]`, so a probe that stops at the first section understates what the instrument
    can reach - which is exactly what it did when it first shipped, reporting three crates while the
    registered hole's own text named five (#303). The set is pinned by content, because the graph is
    what `perf-coverage` compares against: an undercount there reads as "the binding is unreachable"
    when it is really "the probe stopped early".
    """
    graph = matrix.ir_harness_build_graph()
    assert {
        "pyrs-yaml-core",
        "pyrs-yaml",
        "pyrs-ast",
        "pyrs-schema",
        "pyrs-json",
        "pyrs-toml",
    } <= graph, graph
    assert "pyrs-yaml-cli" not in graph, "the CLI is not linked by either harness"


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


DECLARED_HANDLER = """\
[project]
docs_dir = "docs/en"

[project.plugins.mkdocstrings.handlers.python]
paths = ["python"]

[project.plugins.mkdocstrings.handlers.python.options]
heading_level = 3
"""


def test_a_declared_handler_with_no_directive_is_reported(matrix, tmp_path):
    """The probe distinguishes 'configured' from 'used', and says so when they disagree.

    `zensical.toml` declares the Python handler with fourteen render options while no page contains a
    `:::` directive, so the handler renders nothing and the API reference is hand-typed. The measurement
    has to notice that in both directions: a tree that uses the handler emits no hole, and a tree that
    declares none emits no hole either - only the mismatch is a finding, which is why both cases are
    asserted rather than the one that happens to be true today.
    """
    (tmp_path / "zensical.toml").write_text(DECLARED_HANDLER, encoding="utf-8")
    page = tmp_path / "docs" / "en" / "api"
    page.mkdir(parents=True)
    (page / "reference.md").write_text("## Reference\n\nHand-typed.\n", encoding="utf-8")

    assert matrix.mkdocstrings_declared(tmp_path) is True
    assert matrix.mkdocstrings_directives(tmp_path) == []

    (page / "reference.md").write_text("## Reference\n\n::: pyrs_yaml.parse\n", encoding="utf-8")
    assert matrix.mkdocstrings_directives(tmp_path) == ["docs/en/api/reference.md"]

    # A directive indented inside an admonition or a content tab is still a request to the handler.
    (page / "reference.md").write_text("!!! example\n\n    ::: pyrs_yaml.parse\n", encoding="utf-8")
    assert matrix.mkdocstrings_directives(tmp_path) == ["docs/en/api/reference.md"]

    # And the opposite shape: no handler configured at all is not a blind spot, just a choice.
    (tmp_path / "zensical.toml").write_text('[project]\ndocs_dir = "docs/en"\n', encoding="utf-8")
    assert matrix.mkdocstrings_declared(tmp_path) is False


def test_the_repository_state_the_probe_reports_is_the_state_of_the_tree(matrix):
    """Today's tree declares the handler and uses nothing, so the hole must be measured today.

    If this ever goes red because the docs tree *does* use `:::`, that is the hole closing: delete the
    registry entry in the same change, or the other direction of the equality test will fail.
    """
    assert matrix.mkdocstrings_declared() is True
    found = matrix.mkdocstrings_directives()
    measured = {f"{kind}:{name}" for kind, name, _why in matrix.measure()["holes"]}
    assert ("docs-generation:plugin-unused" in measured) == (not found), (found, sorted(measured))


def test_site_rendering_reachability_is_read_off_the_workflows(matrix, tmp_path, monkeypatch):
    """A job is a gate only when its own workflow triggers on a pull request.

    `docs.yml` renders the site on push to `main`, and that is the fact `docs-rendering:unbuilt-on-pr`
    records: four changelog pages carried metadata the generator cannot parse through three merged pull
    requests because nothing before merge asked a page to render. Two near-misses are included on
    purpose - a push-only build, and a step that merely quotes the build command - since either would
    satisfy a probe written as a substring search over the file.
    """
    workflows = tmp_path / ".github" / "workflows"
    workflows.mkdir(parents=True)
    monkeypatch.setattr(matrix, "WORKFLOWS", workflows)

    def write(name: str, text: str) -> None:
        (workflows / name).write_text(text, encoding="utf-8")

    write(
        "docs.yml",
        "name: Docs\non:\n  push:\n    branches: [main]\njobs:\n  build:\n    steps:\n"
        "      - run: python scripts/build-docs.py\n",
    )
    write(
        "mentions.yml",
        "name: Mentions\non:\n  pull_request:\njobs:\n  talk:\n    steps:\n"
        '      - run: echo "python scripts/build-docs.py is the deploy command"\n',
    )
    assert matrix.renders_the_site() is False, "a push-only build or a quoted mention is not a gate"

    write(
        "preview.yml",
        "name: Preview\non:\n  pull_request:\njobs:\n  site:\n    steps:\n      - run: python scripts/build-docs.py\n",
    )
    assert matrix.renders_the_site() is True, "a PR-triggered render must be recognised"
