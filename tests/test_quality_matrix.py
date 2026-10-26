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
import re
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


def test_dialect_binding_probe_reads_a_step_not_a_sentence(matrix, tmp_path):
    """A workflow that only mentions the binding has not bound anything.

    The pyq dialect tier skips when the host carries neither jq nor mikefarah yq, and the
    skip is captured by cargo, so the CI premise has to be read off the tree: a leg that
    sets `PYQ_DIALECT_YQ` compares, one that does not says nothing. Matching the whole
    file would have called a comment a gate - the same trap `run_commands` was rewritten
    for.
    """
    workflows = tmp_path / ".github" / "workflows"
    workflows.mkdir(parents=True)
    leg = workflows / "ci.yml"
    leg.write_text("# someday we will set PYQ_DIALECT_YQ here\njobs: {}\n", encoding="utf-8")
    assert not matrix.dialect_reference_bound(tmp_path), "a comment is not a binding"
    leg.write_text(
        "jobs:\n  a:\n    steps:\n      - env:\n          PYQ_DIALECT_YQ: /usr/local/bin/yq\n",
        encoding="utf-8",
    )
    assert matrix.dialect_reference_bound(tmp_path), "an env step does bind one"
    assert not matrix.dialect_reference_bound(tmp_path / "nowhere"), "a missing tree reads unbound"


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
    whole dialect leaves nothing for that rule to start from - so this test drops all of a dialect's
    scenarios and requires the hole to appear by format name, not by scenario name.
    """
    real = matrix.ir_harness_channels()
    # The same structural rule the production code uses to decide a scenario's format, so dropping
    # "all of json5" cannot leave a half behind as the scenario list grows. The assertion names what
    # is dropped: adding a JSON5 scenario has to come through here and say so.
    dropped = {
        name
        for names in real.values()
        for name in names
        if name.split("_")[0] in ("to", "from") and name.split("_")[1:2] == ["json5"]
    }
    assert dropped == {
        "to_json5_medium",
        "from_json5_medium",
        "to_json5_long_strings",
        "from_json5_long_strings",
        "to_json5_notes",
        "from_json5_notes",
    }, sorted(dropped)
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


def test_a_property_that_filters_its_own_samples_is_named(matrix, tmp_path):
    """The guard is the finding, so the probe must not read a plain property as silent.

    Written against the way the tier hid a lost mapping key: seven properties draw a tree, ask a
    helper for its re-parse, and assert only `if let Some(..)`. A detector matching any mention of
    `try_roundtrip` would instead have named the file that *defines* the helper - a count of files,
    not a measurement of assertions.
    """
    src = tmp_path / "crates" / "demo" / "src"
    src.mkdir(parents=True)
    (src / "pbt.rs").write_text(
        "fn try_roundtrip(node: &Node) -> Option<Node> {\n"
        "    None\n"
        "}\n"
        "fn prop_guarded(node in arb()) {\n"
        "    if let Some(parsed) = try_roundtrip(&node) {\n"
        "        assert!(same(node, parsed));\n"
        "    }\n"
        "}\n"
        "fn prop_asserts(node in arb()) {\n"
        '    let parsed = try_roundtrip(&node).expect("round trips");\n'
        "    assert!(same(node, parsed));\n"
        "}\n"
        "fn prop_untouched(node in arb()) {\n"
        "    assert!(parses(node));\n"
        "}\n",
        encoding="utf-8",
    )
    found = matrix.conditional_property_assertions(root=tmp_path)
    assert found == ["crates/demo/src/pbt.rs::prop_guarded"], found


def test_the_conditional_guard_hole_bites_in_both_directions(matrix, monkeypatch):
    """Counting properties measures the wrong quantity, and a closed guard must read as closed."""
    real = matrix.conditional_property_assertions()
    assert real == [], sorted(real)
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("property-tier", "conditional-assert") not in holes, sorted(holes)

    monkeypatch.setattr(
        matrix,
        "conditional_property_assertions",
        lambda root=None: ["crates/pyrs-yaml-core/src/pbt.rs::prop_roundtrip"],
    )
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("property-tier", "conditional-assert") in holes, sorted(holes)


def test_the_page_states_the_counts_the_probe_measures(measured):
    """A number on the page is a measurement claim, so the measurement has to agree with it.

    The sentence written here first said 7 of 23, and PR #410 landed the property that made the
    total 24 - the rot this row describes, reproduced inside the row that reports it. Pinning the
    phrase to the probe is what keeps a tier that grows from leaving the page behind.
    """
    text = DOC_PATH.read_text(encoding="utf-8")
    claimed = {(int(guarded), int(total)) for guarded, total in re.findall(r"(\d+) of the (\d+) properties", text)}
    measured_pair = (
        len(measured["property_conditional_assertions"]),
        measured["property_function_count"],
    )
    assert claimed == {measured_pair}, (
        f"QUALITY_MATRIX.md claims {sorted(claimed)}, the probe measures {measured_pair[0]} of {measured_pair[1]}"
    )


def test_the_test_matrix_rows_state_the_counts_the_probe_measures(measured):
    """Every number in the section-1 table is a claim about the tree, so the page is pinned to it.

    The page's header promises the numbers cannot rot, but only the property sentence had a test
    behind it: the table still read 509 Rust tests, 1,216 Python test functions, 22 ``prop_*``
    functions and 7 fuzz targets with 118 seeds (60 former crashes) while the probe measured 688,
    1,685, 25, 8 and 141 (66) - this test is itself counted, which is why the fix moved the Python
    total by one; the section-4 sentence repeated the seed count.
    """
    text = DOC_PATH.read_text(encoding="utf-8")
    unit = re.search(r"\| unit / integration \| ([\d,]+) Rust `#\[test\]`, ([\d,]+) Python test functions \|", text)
    prop = re.search(r"\| property \| (\d+) proptest properties", text)
    fuzz = re.search(r"\| fuzz \| (\d+) targets, (\d+) seeds of which (\d+) are `former-crash-\*` \|", text)
    regress = re.search(r"former-crash-<hash>\.seed` \((\d+) of them\)", text)
    assert unit and prop and fuzz and regress, "QUALITY_MATRIX.md no longer states every section-1 count"
    claimed = (
        int(unit.group(1).replace(",", "")),
        int(unit.group(2).replace(",", "")),
        int(prop.group(1)),
        int(fuzz.group(1)),
        int(fuzz.group(2)),
        int(fuzz.group(3)),
        int(regress.group(1)),
    )
    measured_counts = (
        measured["rust_test_count"],
        measured["python_test_function_count"],
        measured["property_function_count"],
        len(measured["fuzz_targets"]),
        measured["fuzz_seed_count"],
        measured["fuzz_former_crash_count"],
        measured["fuzz_former_crash_count"],
    )
    assert claimed == measured_counts, f"QUALITY_MATRIX.md claims {claimed}, the probe measures {measured_counts}"


def test_the_committed_rows_are_what_the_derivation_writes(matrix, measured):
    """The row is now generated from the measurement, so the page and the probe cannot disagree.

    The test above compares a parsed number with the probe, which catches a stale row but leaves the
    author to type the new one: PR #549 reddened all 14 test legs because a new test file moved the
    count and nobody re-derived the digit. `--write-rows` removes that step, and this asserts the
    committed page is byte-identical to what the same code writes.
    """
    text = DOC_PATH.read_text(encoding="utf-8")
    assert matrix.row_drift(text, measured) == []
    written, changed = matrix.apply_rows(text, measured)
    assert written == text and changed == [], f"the page is not in derived form: {changed}"


def test_a_stale_digit_and_a_reworded_row_are_both_named(matrix, measured):
    """Both failure directions of the derivation, proven on the real page.

    A checker that only fires when the number is wrong would pass on a table reworded until no rule
    matches it - the same silent-off shape this file keeps refusing for role conditions.
    """
    text = DOC_PATH.read_text(encoding="utf-8")
    stale_number = re.search(r"\| unit / integration \| ([\d,]+) Rust", text)
    assert stale_number, "the unit row no longer states its Rust count where the rule expects it"
    wrong = "{:,}".format(int(stale_number.group(1).replace(",", "")) - 20)
    problems = matrix.row_drift(text.replace(stale_number.group(1), wrong, 1), measured)
    assert any(problem.startswith("unit / integration row") for problem in problems), problems

    reworded = text.replace("| unit / integration |", "| unit tests |", 1)
    problems = matrix.row_drift(reworded, measured)
    assert any("no sentence matches" in problem for problem in problems), problems

    # The property row's parenthetical is inside the derived sentence now, so a breakdown that no
    # longer splits the tier is named rather than waved through because the leading count still reads
    # right. That half was hand-typed and rotted once: it read "2 in `pyrs-event`" while two crates
    # held four prose-named properties between them.
    stale_breakdown = text.replace("3 in `pyrs-event`", "2 in `pyrs-event`", 1)
    assert stale_breakdown != text, "the breakdown this control edits is no longer on the page"
    problems = matrix.row_drift(stale_breakdown, measured)
    assert any(problem.startswith("property row") for problem in problems), problems

    # A rewrite repairs a stale number and is idempotent; a sentence the rules cannot find is not
    # something it can invent, so the reworded row stays reported.
    fixed, changed = matrix.apply_rows(text.replace(stale_number.group(1), wrong, 1), measured)
    assert matrix.row_drift(fixed, measured) == []
    assert "unit / integration row" in changed, changed
    again, second = matrix.apply_rows(fixed, measured)
    assert again == fixed and second == [], second
    unfixed, changed = matrix.apply_rows(reworded, measured)
    assert unfixed == reworded and changed == [], changed


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


def test_an_empty_authority_column_is_a_hole_and_a_filled_one_is_not():
    """Both directions, on rows the probe need not reach through the repository to be shown.

    The column used to be prose: the table said `no` and the registry stayed empty, so `holes: []`
    could be quoted as "no short board" while the file itself named one. A row is only filled when
    every file it lists answers `yes`; a file gated on a reader is a hole of its own, named by path.
    """
    matrix = _load_module()
    empty = {"verdict": "no", "files": [], "bounded": []}
    filled = {"verdict": "yes", "files": ["test_schema_suite.py"], "bounded": []}
    assert matrix.authority_gaps({"schema validation": {"authority": empty}, "YAML": {"authority": filled}}) == [
        "schema-validation"
    ]
    assert matrix.authority_gaps({"schema validation": {"authority": filled}}) == []
    bounded = {"verdict": "no", "files": ["test_json5_spec_table.py"], "bounded": ["test_json5_spec_table.py"]}
    assert matrix.authority_file_gaps({"JSON / JSONC / JSON5": {"authority": bounded}}) == [
        ("json5-spec-table", "test_json5_spec_table.py")
    ]


AUTHORITY_GATED = """\
import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - the floor interpreter has no wheel
    pyjson5 = None

pytestmark = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 not installed")


def test_the_reference_accepts(doc):
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc)
"""

AUTHORITY_FREE = """\
import json5
import pytest

import pyrs_yaml


def test_the_reference_accepts(doc):
    assert pyrs_yaml.load_json5(doc) == json5.loads(doc)
"""


def test_an_authority_file_gated_on_its_reader_does_not_count(matrix, tmp_path):
    """Both directions on the shape that produced the hole: the same table, gated and ungated.

    `test_json5_spec_table.py` imported only `pyjson5`, whose markers exclude the 3.8 floor and
    3.14+, and then skipped the whole module on that import, so every case was absent exactly where
    nothing else judged JSON5. The unbounded `json5` reader is what turns the same file into credit.
    """
    gated = tmp_path / "test_json5_spec_table.py"
    gated.write_text(AUTHORITY_GATED, encoding="utf-8")
    assert matrix.authority_cell(gated)["verdict"] == "no"
    free = tmp_path / "test_json5_table_free.py"
    free.write_text(AUTHORITY_FREE, encoding="utf-8")
    assert matrix.authority_cell(free)["verdict"] == "yes"


AUTHORITY_USES = [
    (
        "the reader is imported and never reached",
        "import json\n\nimport pyrs_yaml\n\n\n"
        "def test_the_reference_accepts(doc):\n"
        "    assert pyrs_yaml.load_json(doc) is not None\n",
        "no",
    ),
    (
        "the only reference is a capability probe",
        "import json\n\nimport pyrs_yaml\n\nHAS_JSON = json is not None\n\n\n"
        "def test_the_reference_accepts(doc):\n"
        "    assert pyrs_yaml.load_json(doc) is not None\n",
        "no",
    ),
    (
        "the reader is wired into a module-level table",
        "import json\n\nimport pyrs_yaml\n\n"
        'ROUTES = {"to_json": (lambda doc: doc.to_json(), json.loads)}\n\n\n'
        "def test_a_stranger_reads_what_the_writer_emitted():\n"
        "    for emit, read in ROUTES.values():\n"
        '        assert read(emit(pyrs_yaml.parse("a: 1"))) is not None\n',
        "yes",
    ),
    (
        "a corpus with no reader to be absent",
        "import pytest\n\nimport pyrs_yaml\nfrom pyrs_yaml.compliance import load_test_cases\n\n"
        'SUITE_DIR = "Reference/yaml-test-suite"\n\n\n'
        '@pytest.mark.skipif(not __import__("pathlib").Path(SUITE_DIR).exists(), reason="corpus not found")\n'
        "def test_every_case_parses():\n"
        "    assert load_test_cases(SUITE_DIR)\n",
        "yes",
    ),
    (
        "a corpus gate whose reason carries the suite's URL",
        "import json\n\nimport pytest\n\nimport pyrs_yaml\n\n"
        'SUITE_DIR = "Reference/JSON-Schema-Test-Suite"\n'
        "pytestmark = pytest.mark.skipif(\n"
        '    not __import__("pathlib").Path(SUITE_DIR).is_dir(),\n'
        '    reason="clone https://github.com/json-schema-org/JSON-Schema-Test-Suite",\n'
        ")\n\n\n"
        "def test_every_case_validates():\n"
        "    case = json.loads('{\"a\": 1}')\n"
        '    assert pyrs_yaml.parse(json.dumps(case)).validate({"type": "object"}) is None\n',
        "yes",
    ),
    (
        "a suite with the suffix and no corpus to replay",
        'import pyrs_yaml\n\n\ndef test_placeholder():\n    assert pyrs_yaml.parse("a: 1") is not None\n',
        "no",
    ),
    (
        "a corpus named only in a comment",
        "# runs the Reference/ corpus\nimport pyrs_yaml\n\n\n"
        "def test_placeholder():\n"
        '    assert pyrs_yaml.parse("a: 1") is not None\n',
        "no",
    ),
]


@pytest.mark.parametrize(
    ("why", "source", "verdict"),
    AUTHORITY_USES,
    ids=[case[0] for case in AUTHORITY_USES],
)
def test_a_free_reader_is_credited_by_a_use_not_by_its_import(matrix, tmp_path, why, source, verdict):
    """An import that outlives its comparisons must not credit the file.

    `test_yaml_suite.py` carried exactly this shape: `import yaml` sat beside a capability flag while
    the comparison ran through the corpus. The fix there was to take the flag from the module that
    loads the corpus; here the same file reads `no` until the reader's API is reached. `json is None`
    is a probe, not a comparison, and neither is a stale import. Reaching into the reader through a
    module-level table counts - the writer suites call their outside readers that way - and a corpus
    with no reader to be absent is credited by the corpus it names, which is the `Reference/` path the
    loader receives; the suffix alone credits nothing.
    """
    module = tmp_path / "test_json_spec_table.py"
    module.write_text(source, encoding="utf-8")
    assert matrix.authority_cell(module)["verdict"] == verdict, why


def test_a_partial_parity_column_is_a_hole_and_a_full_one_is_not(measured):
    """`partial` means the tier is absent on the legs its oracles exclude, and those legs are green.

    The column was already measured - `format_matrix()` reads the oracle imports and their markers - but
    it stayed a description, so the site could say "the JSON family has a cross-library tier" about a
    floor interpreter where nothing imports an outside reader. Both directions: a `yes` row must not
    emit, an empty `restricted` list must not either.
    """
    matrix = _load_module()
    rows = {
        "JSON / JSONC / JSON5": {"cross_library": {"verdict": "partial", "restricted": ["json5", "jsonc"]}},
        "YAML": {"cross_library": {"verdict": "yes", "restricted": []}},
        "schema validation": {"cross_library": {"verdict": "n/a", "restricted": []}},
    }
    assert matrix.parity_gaps(rows) == [("json-jsonc-json5", ["json5", "jsonc"])]
    assert matrix.parity_gaps({"TOML": {"cross_library": {"verdict": "yes", "restricted": []}}}) == []
    holes = [hole[:2] for hole in measured["holes"]]
    assert ["parity-tier", "json-jsonc-json5"] not in holes, holes
    assert ["parity-tier", "yaml"] not in holes, holes
    measured_json = measured["format_matrix"]["JSON / JSONC / JSON5"]["cross_library"]
    assert measured_json["verdict"] == "yes" and not measured_json["restricted"], measured_json


CROSSLIB_HEADER = """\
import json

import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - the floor interpreter has no wheel
    pyjson5 = None
"""

CROSSLIB_CASES = [
    (
        "the test asks only the reader pyproject bounds, and names the free one in prose",
        "def test_load_json_matches_the_reference(doc):\n"
        '    """pyrs and stdlib json.loads agree on this corpus."""\n'
        "    assert pyrs_yaml.load_json(doc) == pyjson5.loads(doc)\n",
    ),
    (
        "the free reader is reached through a helper defined after the test",
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == _read(doc)\n"
        "\n"
        "\n"
        "def _read(doc):\n"
        "    return json.loads(doc)\n",
    ),
    (
        "the guard is a skip marker reached through a name",
        'needs_pyjson5 = pytest.mark.skipif(pyjson5 is None, reason="no wheel here")\n'
        "\n"
        "\n"
        "@needs_pyjson5\n"
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
    ),
    (
        "the whole module is gated, which is how the strict dialect lost the floor leg",
        'pytestmark = pytest.mark.skipif(pyjson5 is None, reason="no wheel here")\n'
        "\n"
        "\n"
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
    ),
]


@pytest.mark.parametrize(
    ("why", "body"),
    CROSSLIB_CASES,
    ids=[case[0] for case in CROSSLIB_CASES],
)
def test_a_dialect_is_credited_only_by_a_comparison_that_cannot_be_skipped(matrix, tmp_path, why, body):
    """Four ways a `json` comparison can fail to exist on a leg, and none of them earns a `yes`.

    Written because the first cut of the per-dialect cell did exactly that. It matched the reader's
    name anywhere in a test's text, so a docstring that mentions `json.loads` credited the dialect, a
    helper defined after the last test was read as part of it, and `@needs_pyjson5` - a skipif wearing
    another name - looked like no guard at all. Each of those reads a leg that compares nothing as a leg
    that compares something, which is the one error this column must not make.
    """
    module = tmp_path / "test_json_crosslib.py"
    module.write_text(CROSSLIB_HEADER + body, encoding="utf-8")
    row = matrix.crosslib_parity("JSON / JSONC / JSON5", module)
    assert row["covered"] == [], (why, row)
    assert row["restricted"] == ["json", "json5", "jsonc"], (why, row)


def test_a_comparison_a_leg_cannot_skip_does_credit_its_dialect(matrix, tmp_path):
    """The other direction: the rule must not be so blunt that the real tier reads as absent.

    `tests/test_json_crosslib.py` calls stdlib `json` in a test that carries nothing but
    `parametrize`, and that is what takes RFC 8259 out of the restricted list while JSONC and JSON5
    stay in it.
    """
    module = tmp_path / "test_json_crosslib.py"
    module.write_text(
        CROSSLIB_HEADER + '@pytest.mark.parametrize("doc", ["{}"])\n'
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
        encoding="utf-8",
    )
    row = matrix.crosslib_parity("JSON / JSONC / JSON5", module)
    assert row["covered"] == ["json"], row
    assert row["restricted"] == ["json5", "jsonc"], row
    assert row["marker"]["json"] is None, row


def test_every_dialect_is_credited_by_a_reader_every_leg_installs(measured):
    """The tree, not a fixture: `json5` is the reference both non-strict dialects now share.

    The cell used to be a property of the family, so it could only read `partial` while JSONC and
    JSON5 were asked of `pyjson5`, whose wheel is missing on the floor. `tests/test_json_crosslib.py`
    imports the pure-Python `json5` package unconditionally now, and every leg installs it, so the
    row has a reference for each of its dialects and nothing left in `restricted`.
    """
    row = measured["format_matrix"]["JSON / JSONC / JSON5"]["cross_library"]
    assert row["covered"] == ["json", "json5", "jsonc"], row
    assert row["restricted"] == [], row
    assert row["verdict"] == "yes", row


#: A crosslib module whose readers are all installed on every leg. The bounded-reader set is empty,
#: which is the shape the cell used to answer before reading a single test.
CROSSLIB_INSTALLED_HEADER = "import pytest\n\nimport pyrs_yaml\n\nimport json\nimport sys\n\n\n"

CROSSLIB_NO_LIVE_COMPARISON = [
    (
        "every test is marked skip",
        '@pytest.mark.skip(reason="not today")\n'
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
    ),
    ("the file has no test at all", ""),
    (
        "the module is gated",
        'pytestmark = pytest.mark.skip(reason="not today")\n'
        "\n\n"
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
    ),
    (
        "the guard fires on a leg the matrix tests",
        '@pytest.mark.skipif(sys.version_info < (3, 9), reason="needs 3.9")\n'
        '@pytest.mark.parametrize("doc", ["{}"])\n'
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n",
    ),
]


@pytest.mark.parametrize(
    ("why", "body"),
    CROSSLIB_NO_LIVE_COMPARISON,
    ids=[case[0] for case in CROSSLIB_NO_LIVE_COMPARISON],
)
def test_a_family_with_no_bounded_reader_still_needs_a_live_comparison(matrix, tmp_path, why, body):
    """The cell returned `yes` before it read a unit whenever nothing bounded the readers.

    `tests/test_yaml_crosslib.py` imports two readers that no marker bounds, so the early return was
    the branch that credited it - and a module whose every comparison carried `@pytest.mark.skip` took
    the same branch. A reader every leg installs removes one way to lose a comparison, not the
    requirement that a comparison exist. The last case is the control for the relaxation below: a
    `skipif` on an interpreter version is still a guard a leg can lose.
    """
    module = tmp_path / "test_json_crosslib.py"
    module.write_text(CROSSLIB_INSTALLED_HEADER + body, encoding="utf-8")
    row = matrix.crosslib_parity("JSON / JSONC / JSON5", module)
    assert row["covered"] == [], (why, row)
    assert row["restricted"] == ["json", "json5", "jsonc"], (why, row)
    assert row["verdict"] == "partial", (why, row)


def test_a_guard_on_the_availability_of_an_installed_reader_is_dead(matrix, tmp_path):
    """The positive control: the guard `test_yaml_crosslib.py` writes cannot fire on a matrix leg.

    `pyproject.toml` declares PyYAML with no marker, so every tested interpreter installs it; a
    `skipif(pyyaml is None)` and a helper whose whole body is `return pyyaml is not None` describe a
    dependency that is never absent. Reading either as a gate would turn the one family whose readers
    all reach every leg into a hole the measurement invented.
    """
    module = tmp_path / "test_yaml_crosslib.py"
    module.write_text(
        "import pytest\n\nimport pyrs_yaml\n\n"
        "try:\n    import yaml as pyyaml\nexcept ImportError:\n    pyyaml = None\n\n\n"
        "def _pyyaml_available():\n"
        "    return pyyaml is not None\n\n\n"
        '@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")\n'
        '@pytest.mark.parametrize("doc", ["a: 1"])\n'
        "def test_direct(doc):\n"
        "    assert pyrs_yaml.safe_load(doc) == pyyaml.safe_load(doc)\n\n\n"
        '@pytest.mark.skipif(not _pyyaml_available(), reason="PyYAML not installed")\n'
        '@pytest.mark.parametrize("doc", ["a: 1"])\n'
        "def test_through_helper(doc):\n"
        "    assert pyrs_yaml.safe_load(doc) == pyyaml.safe_load(doc)\n",
        encoding="utf-8",
    )
    row = matrix.crosslib_parity("YAML", module)
    assert row["covered"] == ["yaml"], row
    assert row["restricted"] == [], row
    assert row["verdict"] == "yes", row


def test_a_helper_that_asks_the_interpreter_is_still_a_gate(matrix, tmp_path):
    """The helper form is resolved by its body, not by its name: only a reader probe is dead."""
    module = tmp_path / "test_yaml_crosslib.py"
    module.write_text(
        "import pytest\n\nimport pyrs_yaml\n\nimport sys\n\n"
        "try:\n    import yaml as pyyaml\nexcept ImportError:\n    pyyaml = None\n\n\n"
        "def _modern():\n"
        "    return sys.version_info >= (3, 9)\n\n\n"
        '@pytest.mark.skipif(not _modern(), reason="needs 3.9")\n'
        '@pytest.mark.parametrize("doc", ["a: 1"])\n'
        "def test_x(doc):\n"
        "    assert pyrs_yaml.safe_load(doc) == pyyaml.safe_load(doc)\n",
        encoding="utf-8",
    )
    row = matrix.crosslib_parity("YAML", module)
    assert row["covered"] == [], row
    assert row["verdict"] == "partial", row


def test_the_yaml_row_is_credited_by_a_comparison_it_really_runs(measured):
    """The tree: every PyYAML/ruamel comparison carries a skipif, and none of them can skip.

    The row is the reason the availability guards are resolved rather than denied: both readers are
    declared without a marker, `tests/test_yaml_crosslib.py` compares `safe_load`/`safe_dump`/`parse`
    in tests that carry nothing else, and the cell still reads `yes` after the unit rule replaced the
    early return.
    """
    row = measured["format_matrix"]["YAML"]["cross_library"]
    assert row["covered"] == ["yaml"], row
    assert row["restricted"] == [], row
    assert row["marker"] == {"PyYAML": None, "ruamel.yaml": None}, row
    assert row["verdict"] == "yes", row


def test_a_reader_the_interpreter_bounds_is_found_and_bounded(matrix, tmp_path):
    """An oracle can be kept off a leg by the interpreter, not only by a dependency marker.

    Measured on the tree: `tests/test_toml_crosslib.py` ran 13 of its 24 comparisons and skipped 11 on the floor
    leg while its cell named one reader. The reader it skipped is `tomllib`, which no `pyproject.toml`
    marker bounds and no module-level `import` line binds - the test takes it through
    `pytest.importorskip` - so neither half of the derivation was asked about it. The version that
    introduces a stdlib reader is compared against the floor the project states in `requires-python`,
    so moving the floor moves this answer instead of leaving a remembered number behind.
    """
    module = tmp_path / "test_crosslib.py"
    module.write_text(
        CROSSLIB_HEADER + '@pytest.mark.parametrize("doc", ["{}"])\n'
        "def test_load_json_matches_the_reference(doc):\n"
        "    assert pyrs_yaml.load_json(doc) == json.loads(doc)\n"
        "def test_load_json_matches_a_later_reader(doc):\n"
        '    tomllib = pytest.importorskip("tomllib")\n'
        "    assert pyrs_yaml.load_json(doc) == tomllib.loads(doc)\n",
        encoding="utf-8",
    )
    row = matrix.crosslib_parity("JSON / JSONC / JSON5", module)
    assert "tomllib" in row["oracles"], row
    assert row["marker"]["tomllib"].startswith("arrives with python"), row
    assert row["marker"]["json"] is None, row
    assert row["covered"] == ["json"], row


def test_the_toml_row_names_both_readers_and_the_bound_of_each(measured):
    """The tree, not a fixture: three readers, two of them split across the 3.11 line, and a verdict that holds.

    The row read `oracles: ["tomlkit"]` while the file asks two readers, and the 11 comparisons the
    floor and the two next legs skip were therefore invisible. `tomllib` arrives with 3.11 and `tomli`
    is declared below it, so each reader is bounded to one side of the same line and the pair covers
    every leg; `tomlkit` carries no marker and judges the dialect everywhere, which is what keeps the
    cell at `yes` rather than turning a reader bound into a hole.
    """
    row = measured["format_matrix"]["TOML"]["cross_library"]
    assert row["oracles"] == ["tomli", "tomlkit", "tomllib"], row
    assert row["marker"]["tomlkit"] is None, row
    assert row["marker"]["tomli"] == "python_version < '3.11'", row
    assert row["marker"]["tomllib"].startswith("arrives with python"), row
    assert row["verdict"] == "yes", row


def test_a_marker_is_read_off_the_declared_name_and_not_a_longer_one(matrix, tmp_path, monkeypatch):
    """`tomli-w` is a different distribution from `tomli`, and the search must not answer with it.

    The tree declares `tomli` for `python_version < '3.11'` in the test group and `tomli-w`
    unconditionally in the docs group. A search that only constrained the prefix answered whichever
    line came first, so the backport's bound was read off a package no test imports - the column
    claiming how a reader is bounded would report someone else's requirement.
    """
    (tmp_path / "pyproject.toml").write_text(
        '[project]\nrequires-python = ">=3.8"\n'
        "[dependency-groups]\n"
        'docs = ["tomli-w>=1.0.0"]\n'
        "test = [\"tomli>=2.0; python_version < '3.11'\"]\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(matrix, "REPO", tmp_path)
    assert matrix._dependency_marker("tomli") == "python_version < '3.11'"
    assert matrix._dependency_marker("tomli-w") is None


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


def test_the_per_format_table_is_the_derived_one(measured):
    """Every cell of the per-format table is now computed, so the page and the computation must agree.

    Deriving the rows is what found a cell the prose had wrong in the direction that understates the
    defence: the property tier counted `fn prop_*` names only, missing the two `proptest!` properties
    `crates/pyrs-event/src/lib.rs` declares with a `#[test]` attribute and a prose name.
    """
    text = DOC_PATH.read_text(encoding="utf-8")
    cells = re.findall(
        r"^\| (YAML|JSON / JSONC / JSON5|TOML|JSON Schema validation|Schema rule language)"
        r" \| (yes|no) \| (yes|no) \| (yes|no) \| (yes|no|n/a) \| (yes|no|partial|n/a)"
        r" \| (yes|no) \|$",
        text,
        re.MULTILINE,
    )
    claimed = {
        label: (unit, prop, parse, trip, parity, authority)
        for label, unit, prop, parse, trip, parity, authority in cells
    }
    assert len(claimed) == 5, f"the page no longer states all five surface rows: {claimed}"

    derived = {}
    for line in measured["format_verdicts"]:
        label, rest = line.split(": ", 1)
        fields = dict(item.split("=", 1) for item in rest.split(" "))
        derived[label] = (
            fields["unit"],
            fields["property"],
            fields["fuzz-parse"],
            fields["round-trip"],
            fields["parity"],
            fields["authority"],
        )
    assert claimed == derived, f"the page says {claimed}, the probe derives {derived}"


def test_the_derived_table_is_not_vacuous(measured):
    """A row whose fragments matched nothing would still print `no`s, so the inputs are pinned too."""
    rows = measured["format_matrix"]
    assert rows["YAML"]["unit"] > 0, rows["YAML"]
    assert rows["YAML"]["fuzz_parse"], "parse_yaml.rs must count as fuzz coverage"
    assert measured["property_function_count"] >= 27, "the two pyrs-event properties must be counted"
    json_parity = rows["JSON / JSONC / JSON5"]["cross_library"]
    assert json_parity["verdict"] == "yes" and not json_parity["restricted"], (
        "the pure-Python json5 reader is undeclared by markers, so every leg has a reference"
    )
    assert not rows["TOML"]["cross_library"]["restricted"], "tomlkit is unbounded, so TOML reads `yes`"
    authority = {label: row["authority"] for label, row in rows.items()}
    assert authority["YAML"]["files"] == ["test_yaml_suite.py", "test_yaml_writer_suite.py"], authority
    assert authority["TOML"]["files"] == ["test_toml_test_suite.py", "test_toml_writer_suite.py"], authority
    # Both suffixes have to be live: the official-suite one for three rows, the production-table one
    # for the JSON5 half of the JSON family.
    assert "test_json5_spec_table.py" in authority["JSON / JSONC / JSON5"]["files"], authority
    assert authority["JSON / JSONC / JSON5"]["verdict"] == "yes", authority
    # The schema surface is one row: its authority is the official draft2020-12 corpus replayed
    # against `Document.validate()`, the only validator this package still ships.
    assert authority["JSON Schema validation"]["files"] == ["test_document_validate_suite.py"], authority
    assert authority["JSON Schema validation"]["verdict"] == "yes", authority
    assert not rows["JSON Schema validation"]["fuzz_parse"], "no Rust fuzz target names the delegated surface"

    # The rule language is a row of its own, and the cells are counted in the file that declares
    # it: it shares `pyrs-yaml-core` with the YAML engine, so a crate-wide count would read as
    # defended by hundreds of tests that never mention it.
    rule = rows["Schema rule language"]
    assert rule["unit"] == 11, rule
    assert rule["property_rust"] == 1, rule
    assert rule["fuzz_parse"] == ["parse_schema"], rule
    assert not rule["fuzz_round_trip"], rule
    assert rule["cross_library"]["verdict"] == "n/a", rule
    # Its authority cell is now filled: `tests/test_parse_schema_spec_table.py` asks each `type:`
    # name the grammar accepts whether the value it resolves is an instance of the JSON Schema type
    # that name denotes, judged by `jsonschema` - a reader this repository does not own. The gap that
    # was `authority-tier:schema-rule-language` is closed, so neither the file nor the row is a hole.
    assert rule["authority"]["files"] == ["test_parse_schema_spec_table.py"], rule
    assert rule["authority"]["verdict"] == "yes", rule
    assert rule["authority"]["bounded"] == [], rule

    # The parity column claims to read how `pyproject.toml` bounds each oracle, so every oracle a
    # cross-library test imports has to be found there at all. `ruamel.yaml` is imported under that
    # name and declared as `ruamel-yaml`; the miss used to answer `undeclared`, which reads as
    # "not declared" about a dependency installed on every leg - the cell understated the defence.
    markers = {
        oracle: marker for row in rows.values() for oracle, marker in row["cross_library"].get("marker", {}).items()
    }
    assert markers, "the parity column measured nothing"
    assert "undeclared" not in markers.values(), sorted(markers)
    assert markers["ruamel.yaml"] is None, markers


def test_the_retired_validator_is_gone_and_the_rule_language_is_not():
    """The document separates two surfaces; the tree has to agree with both halves.

    The page once closed the rule language's hole by saying the surface was retired and that
    "its Rust entry points are deleted", while `parse_schema_yaml`, `RuleResolver` and
    `YamlTypeKind` were still exported by `pyrs-yaml-core`, called by the binding, property-tested
    and fuzzed. The validator beside the language really is gone, so the two halves are checked
    apart: the retired symbols must not come back, and the surviving language must stay, because
    the matrix row would otherwise measure a surface nobody ships.
    """
    sources = "\n".join(
        path.read_text(encoding="utf-8", errors="replace") for path in sorted((REPO_ROOT / "crates").rglob("*.rs"))
    )
    sources += "\n".join(
        path.read_text(encoding="utf-8", errors="replace")
        for path in sorted((REPO_ROOT / "python" / "pyrs_yaml").glob("*.py"))
    )
    sources += (REPO_ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi").read_text(encoding="utf-8")
    for retired in (
        "TypeSpec",
        "ValidateKind",
        "ValidateRule",
        "SchemaValidationError",
        "validate_against_schema",
        "validate_against_registered_schema",
    ):
        assert retired not in sources, f"{retired} was retired with the rule validator"
    for alive in ("parse_schema_yaml", "RuleResolver", "YamlTypeKind"):
        assert alive in sources, f"{alive} is the surviving rule language"
    binding = (REPO_ROOT / "crates" / "pyrs-yaml" / "src" / "py" / "functions.rs").read_text(encoding="utf-8")
    assert "parse_schema_yaml" in binding, "the binding still reads a `rules:` block"


def test_the_blind_spot_count_names_the_entries_the_list_carries():
    """A count typed into prose is a floor typed into prose: it goes stale the day the list grows.

    Found while writing the hook-set record: the sentence said "Seven blind spots" while the list it
    introduces already carried eight entries. Nothing could see the difference, because the derived row
    counts tests and scenarios, not paragraphs - and the sentence reads as a measurement long after the
    measurement stopped being true. So the number is re-derived here from the list under it: adding a
    blind spot and forgetting its count is a red run, in either direction.
    """
    lines = DOC_PATH.read_text(encoding="utf-8").splitlines()
    said = next((line for line in lines if "blind spots were found by measuring" in line), None)
    assert said is not None, "the sentence this test counts is gone; retire this test with it"
    found = re.search(r"(?i)\b(\w+)\s+blind spots were found by measuring\b", said)
    assert found, f"no count in: {said!r}"
    spelled = {
        "zero": 0,
        "one": 1,
        "two": 2,
        "three": 3,
        "four": 4,
        "five": 5,
        "six": 6,
        "seven": 7,
        "eight": 8,
        "nine": 9,
        "ten": 10,
        "eleven": 11,
        "twelve": 12,
        "thirteen": 13,
        "fourteen": 14,
        "fifteen": 15,
        "sixteen": 16,
        "seventeen": 17,
        "eighteen": 18,
        "nineteen": 19,
        "twenty": 20,
    }
    token = found.group(1).lower()
    assert token in spelled or token.isdigit(), f"{found.group(1)!r} is neither a spelled number nor a digit"
    claimed = int(token) if token.isdigit() else spelled[token]

    first = next(
        (i for i in range(lines.index(said) + 1, len(lines)) if lines[i].startswith("- ")),
        None,
    )
    assert first is not None, "the sentence introduces no bullet list, so its count measures nothing"
    counted = 0
    for line in lines[first:]:
        if line.startswith("- "):
            counted += 1
        elif not line.strip() or line[:1] in (" ", "\t", ">", "|", "`"):
            continue
        else:
            break
    assert counted == claimed, (
        f"the prose says {claimed} blind spot(s); the list under it carries {counted} - "
        f"the sentence is a reading that stopped being taken"
    )
