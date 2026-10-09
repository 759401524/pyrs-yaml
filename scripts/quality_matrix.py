"""Measure the quality-defence matrix from the files that declare it.

The four questions a "we have tests" answer cannot answer: which tiers exist per
format, which of them can actually stop a merge, what each tier is blind to, and
whether a hole that was closed stays closed. Every number here is parsed out of the
declaring files (`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`,
`scripts/check_*.py`, the Ir bench, `.ci/ir-baseline.json`), never transcribed — a
transcribed matrix rots the moment a workflow is edited, and the document describing
the defence is the one artefact nothing checks.

`tests/test_quality_matrix.py` compares the derived holes against the committed
registry in `.ci/quality-holes.json`, so an undeclared blind spot fails CI and a hole
that has quietly been closed also fails CI until it is deleted from the registry.

Usage:
    python scripts/quality_matrix.py            # human-readable report
    python scripts/quality_matrix.py --json     # machine-readable dump
    python scripts/quality_matrix.py --holes    # derived hole ids, one per line
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
WORKFLOWS = REPO / ".github" / "workflows"

# A crate exporting one of these owns a writer, so a round-trip invariant exists in
# it and a parse-only fuzz target structurally cannot reach it. Prefix matching, not
# word boundaries: `pyrs-json`'s writers are `to_json_text` / `to_json5_text`, which a
# `\b` after `to_json` silently drops - measured, and it hid a real round-trip hole.
SERIALIZER_RE = re.compile(
    r"^\s*pub fn (?:to_(?:string|yaml|json|jsonc|json5|toml)|dump|serialize|encode)[a-z_0-9]*\b",
    re.MULTILINE,
)


# The declaration that a crate owns an instruction-count harness: a `[[bench]]` target gated behind
# the `ir-gate` feature. Read from the manifest, not from the file list, because the gate builds what
# the manifest declares - a `benches/ir_gate.rs` with no such target is never compiled, and its
# scenarios would sit in the baseline unmeasured.
IR_GATE_TARGET_RE = re.compile(r'^\[\[bench\]\][^\[]*?required-features *= *\[[^\]]*"ir-gate"', re.S | re.M)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def run_commands(text: str) -> list[str]:
    """Every command a workflow can execute, block scalars included.

    Deliberately narrower than "the text of the file": a hook id merely mentioned in
    an explanatory comment would otherwise count as reachable, which is how a gate
    gets reported as wired while it never runs. Quoted spans are stripped for the same
    reason: `echo "cargo clippy is owned by ci.yml"` is a step, but it lints nothing.
    """
    lines = text.splitlines()
    commands = []
    index = 0
    while index < len(lines):
        line = lines[index]
        match = re.match(r"^(\s*)(?:-\s+)?run:\s*(.*)$", line)
        if not match:
            index += 1
            continue
        indent, rest = match.group(1), match.group(2).strip()
        if rest.startswith(("|", ">")):
            body = []
            index += 1
            while index < len(lines):
                following = lines[index]
                if following.strip() and len(re.match(r"^\s*", following).group(0)) <= len(indent):
                    break
                body.append(following.strip())
                index += 1
            commands.append(" ".join(body))
        else:
            commands.append(rest)
            index += 1
    return [re.sub(r"\"[^\"]*\"|'[^']*'", '""', command) for command in commands]


def workflow_commands() -> dict[str, list[str]]:
    return {p.name: run_commands(read(p)) for p in sorted(WORKFLOWS.glob("*.yml"))}


def prek_entries() -> dict[str, str]:
    """Hook id -> the command it runs, empty for builtin hooks."""
    text = read(REPO / "prek.toml")
    entries = {}
    for block in re.finditer(r"\[\[repos\.hooks\]\](.*?)(?=\[\[|\Z)", text, re.DOTALL):
        body = block.group(1)
        hook_id = re.search(r'id = "([a-z0-9-]+)"', body)
        entry = re.search(r'entry = "([^"]+)"', body)
        if hook_id:
            entries[hook_id.group(1)] = entry.group(1) if entry else ""
    for inline in re.finditer(r'\{ id = "([a-z0-9-]+)"([^}]*)\}', text):
        entries.setdefault(inline.group(1), "")
    return entries


def prek_hooks() -> list[str]:
    return sorted(prek_entries())


def fuzz_targets() -> list[str]:
    text = read(REPO / "fuzz" / "Cargo.toml")
    declared = re.findall(r'^\[\[bin\]\]\nname = "([a-z0-9_]+)"', text, re.MULTILINE)
    return sorted(set(declared))


def fuzz_ci_matrix() -> list[str]:
    text = read(WORKFLOWS / "fuzz.yml")
    match = re.search(r"target: \[([^\]]+)\]", text)
    if not match:
        return []
    return sorted(x.strip() for x in match.group(1).split(","))


def fuzz_crates() -> list[str]:
    """Crates the fuzz harness links, straight out of `fuzz/Cargo.toml`.

    This is the honest denominator: the harness's own dependency list says which engines
    the fuzz tier is responsible for. Inferring it from exported function names dropped
    `pyrs-json` and `pyrs-toml` (their entry points are not `parse*`/`from_str`), which
    would have read as "no round-trip hole there" - a false clean.
    """
    text = read(REPO / "fuzz" / "Cargo.toml")
    return sorted(set(re.findall(r'^([a-z0-9-]+) = \{ path = "\.\./crates/', text, re.MULTILINE)))


def engine_crates(fuzz_links: list[str]) -> list[str]:
    """The fuzzed crates that also own a writer, so a round trip exists to test."""
    found = []
    for crate in fuzz_links:
        src = REPO / "crates" / crate / "src"
        if not src.is_dir():
            continue
        if any(SERIALIZER_RE.search(read(py)) for py in sorted(src.rglob("*.rs"))):
            found.append(crate)
    return found


def roundtrip_target_for(crate: str, targets: list[str]) -> bool:
    """Does some fuzz target round-trip this engine's format?"""
    stem = crate.replace("pyrs-", "").replace("-core", "")
    family = {"ast": "yaml", "yaml": "yaml"}.get(stem, stem)
    return any("roundtrip" in target and family in target for target in targets)


def check_scripts() -> list[str]:
    return sorted(p.name for p in (REPO / "scripts").glob("check_*.py"))


def ir_harness_owners() -> list[str]:
    """Crates declaring an `ir_gate` bench target: the channels the gate can measure.

    Two since the binding layer was closed - `pyrs-yaml-core` (parse, serialize, the JSON and TOML
    bridges in both directions) and `pyrs-yaml` (AST-to-Python). Derived from the manifests rather
    than hardcoded, so adding a third crate adds scenarios to the comparison without editing this
    file.
    """
    return sorted(
        manifest.parent.name
        for manifest in (REPO / "crates").glob("*/Cargo.toml")
        if IR_GATE_TARGET_RE.search(read(manifest))
    )


def ir_harness_files() -> list[str]:
    """Crates that carry a `benches/ir_gate.rs`, whether or not anything builds it."""
    return sorted(path.relative_to(REPO).parts[1] for path in (REPO / "crates").glob("*/benches/ir_gate.rs"))


def ir_harness_channels() -> dict:
    """crate -> the scenario names its harness declares, for every crate the gate builds.

    The old probe read `crates/pyrs-yaml-core/benches/ir_gate.rs` by name, which is the same failure
    class as the graph probe that stopped at the first dependency section: when a second harness
    appeared, the scenarios only it lists would have been absent from the comparison, so the committed
    baseline could carry numbers nobody re-measured and the gate could not say a channel went missing.
    """
    channels = {}
    for crate in ir_harness_owners():
        path = REPO / "crates" / crate / "benches" / "ir_gate.rs"
        if not path.exists():
            channels[crate] = []
            continue
        text = read(path)
        body = text.split("fn scenarios()", 1)[-1].split("fn main()", 1)[0]
        # Scenario names are the lowercase string literals; the inputs are `SCREAMING_CASE`
        # constants and `Work::Variant` paths. Matching the whole tuple instead missed
        # `serialize_block_scalars`, whose name sits on its own line after a wrap.
        channels[crate] = sorted(set(re.findall(r'"([a-z_0-9]{4,})"', body)))
    return channels


def ir_scenarios_in_baseline() -> list[str]:
    return sorted(json.loads(read(REPO / ".ci" / "ir-baseline.json"))["scenarios"])


# Formats the hub AST is written *as* and read *back from*. `python` is deliberately absent:
# `to_python_*` is the language binding, not a text format, and it has no reader twin - the
# conversion runs once, in one direction, from bytes the YAML scenarios already measure.
CROSS_FORMAT_WRITERS = ("json", "jsonc", "json5", "toml", "yaml")


def ir_bridge_pairs(channels: dict) -> list[str]:
    """Cross-format scenarios that are measured in one direction only.

    `to_<format>_<size>` and `from_<format>_<size>` are the two halves of one bridge, and the
    halves cost different things: the writer decides how to *emit* a key, the reader decides what
    the key *means*. #292 adds per-key work to the reading half (`load_toml` has to quote a key
    that either YAML schema would re-type, or the conversion changes what the document says) and
    the gate had a number for the writing half only, so the cost of that decision was unmeasurable
    - which is how a change can be adjudicated on the wrong channel and closed.
    """
    names = {name for values in channels.values() for name in values}
    unpaired = []
    for name in sorted(names):
        parts = name.split("_")
        if len(parts) < 3 or parts[0] not in ("to", "from") or parts[1] not in CROSS_FORMAT_WRITERS:
            continue
        twin = "_".join(("from" if parts[0] == "to" else "to", *parts[1:]))
        if twin not in names:
            unpaired.append(name)
    return unpaired


def ir_bridges_absent(channels: dict) -> list[str]:
    """Formats the hub converts to and from that the instruction gate has no number for at all.

    `ir_bridge_pairs` can only notice a missing twin for a format that already has one half, so a
    format with neither half is invisible to it. That is how the JSONC and JSON5 paths - the comment
    scanner, the unquoted-key grammar, the two dialect writers - carried no instruction count while
    the gate printed green over the bridge it could see.
    """
    names = {name for values in channels.values() for name in values}
    absent = []
    for fmt in CROSS_FORMAT_WRITERS:
        if fmt == "yaml":
            continue  # the hub's own text, measured by `parse_*` and `serialize_*`, not a bridge
        halves = [
            name
            for name in names
            if len(name.split("_")) > 1 and name.split("_")[0] in ("to", "from") and name.split("_")[1] == fmt
        ]
        if not halves:
            absent.append(fmt)
    return absent


def property_functions() -> list[str]:
    names = []
    for rs in sorted((REPO / "crates").rglob("*.rs")):
        for match in re.finditer(r"fn (prop_[a-z0-9_]+)\(", read(rs)):
            names.append(f"{rs.relative_to(REPO).as_posix()}::{match.group(1)}")
    return sorted(names)


def renders_the_site() -> bool:
    """Whether a workflow that runs on a pull request also runs the documentation build.

    Asked of the declaring files rather than of a list kept beside them: a job counts only if its own
    workflow triggers on `pull_request` *and* one of its `run:` steps invokes `scripts/build-docs.py`.
    Mentioning the script in a comment, or in a job that only runs on push, is not a gate - which is the
    distinction that let four changelog pages carry unreadable metadata through three merges while
    `.github/workflows/docs.yml` printed green on every push.
    """
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        text = read(workflow)
        triggers = text.split("jobs:")[0]
        if "pull_request" not in triggers:
            continue
        if any("build-docs.py" in command for command in run_commands(text)):
            return True
    return False


def declared_artifacts() -> list[tuple[str, bool]]:
    """Every repository path this repo's own prose promises exists, and whether it does.

    `ROADMAP.md` and `QUALITY_MATRIX.md` are written in backticked file names, and they
    record gates as delivered (``note_survival.rs`` runs on every PR). When the ledger and
    the tree disagree the ledger wins in the reader's head, which is the failure this probe
    exists for: it was found by noticing that this repository's own ledger described a
    regression tier that had been reverted by a later commit.
    """
    found: dict[str, bool] = {}
    for rel in ("ROADMAP.md", "QUALITY_MATRIX.md"):
        text = read(REPO / rel)
        for token in re.findall(r"`([^`]+)`", text):
            # A backticked span often carries a path plus prose (`crates/x/tests/y.rs runs
            # on every PR`); take the first whitespace-delimited word of it.
            token = token.strip().split()[0] if token.strip() else ""
            # Strip trailing punctuation only: `.ci/quality-holes.json` and
            # `crates/pyrs-json/tests/roundtrip_corpus.rs` are both real paths whose leading
            # dot is part of the name, and an early version of this line reported them as
            # missing artifacts — a gate that invents holes is as bad as one that misses them.
            token = token.rstrip(".,:;")
            if "/" not in token or any(ch in token for ch in " *()…"):
                continue
            if not re.match(r"^[.,a-zA-Z0-9_-][^ ]*$", token):
                continue
            path = REPO / token
            if path.suffix not in {".rs", ".py", ".toml", ".yml", ".md", ".json", ".sh"}:
                continue
            if token not in found:
                found[token] = path.exists()
    return sorted(found.items())


def ir_harness_build_graph() -> set[str]:
    """Crates the reproducible instrument can actually reach: bench owner + its dependencies.

    `cargo bench --bench ir_gate` compiles the owning crate and what it links, so anything outside
    that graph is invisible to instruction-count gating no matter what the scenario is named.
    """
    crates: set[str] = set()
    for crate in ir_harness_owners():
        manifest = REPO / "crates" / crate / "Cargo.toml"
        text = read(manifest)
        crates.add(manifest.parent.name)
        # Every dependency section, not the first one. `re.search` here reported a three-crate graph
        # while `to_json_medium` / `to_toml_medium` link `pyrs-json` and `pyrs-toml` through
        # `[dev-dependencies]` - the hole's own text claimed five. Printing the derived set in the
        # summary (`ir_harness_crates`) is the only reason that mismatch was visible at all.
        for section in re.findall(r"^\[(?:dev-)?dependencies\](.*?)(?=^\[|\Z)", text, re.S | re.M):
            linked = re.findall(r"^([a-z0-9_-]+) *=", section, re.M)
            # Workspace members only: the point of the set is which of *this* repository's layers
            # the instrument can reach, and third-party links say nothing about that boundary.
            crates.update(name for name in linked if (REPO / "crates" / name).is_dir())
    return {name.replace("_", "-") for name in crates}


def binding_crate() -> str | None:
    """The crate that serves the Python API - the one layer a user actually calls into.

    Found by layout (`crates/*/src/py/`), not by name, so renaming the directory moves the answer
    instead of leaving a stale hard-coded crate behind.
    """
    for directory in sorted((REPO / "crates").glob("*/src/py")):
        if any(directory.glob("*.rs")):
            return directory.parent.parent.name
    return None


def changelog_mirror_counts() -> dict:
    """Per-mirror [Unreleased] entry counts, by section position, from the mirror checker itself.

    This calls `scripts/check_changelog_mirrors.py` instead of parsing the changelogs a second way:
    the hook and this probe have to agree about what counts as an entry, or the matrix reports clean
    while the gate that enforces the same rule disagrees (and vice versa).
    """
    path = REPO / "scripts" / "check_changelog_mirrors.py"
    spec = importlib.util.spec_from_file_location("check_changelog_mirrors", path)
    assert spec is not None and spec.loader is not None, f"cannot load {path}"
    checker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(checker)
    return {mirror.relative_to(REPO).as_posix(): checker.positioned_counts(read(mirror)) for mirror in checker.FILES}


def timing_floors_unpaired(root: Path | None = None) -> list[str]:
    """Test files that time wall-clock in ordinary CI without sampling their two phases together.

    Three of them did until this was measured: the TOML parse gate ran every candidate block before
    every reference block, and the TOML serialize and both JSON gates took a single block per side.
    A block is only comparable inside itself, so a scheduler spike on one phase flips the verdict on a
    step that really is twice as cheap - the recorded macos incident measured that pair at 1.10x. The
    shared sampler in `tests/timing.py` alternates the phases per block and reports a majority.

    CodSpeed-owned files are excluded: their timing is the product being measured, not a gate about
    another path, and they run on a pinned runner rather than alongside the whole matrix.
    """
    out = []
    for path in sorted(((root or REPO) / "tests").glob("test_*.py")):
        text = read(path)
        if "perf_counter" not in text:
            continue
        if "pytest.mark.benchmark" in text or "codspeed" in text.lower():
            continue
        if "from tests.timing import" not in text:
            out.append(f"tests/{path.name}")
    return out


def measure() -> dict:
    commands_by_workflow = workflow_commands()
    all_commands = " ".join(command for commands in commands_by_workflow.values() for command in commands)
    entries = prek_entries()
    hooks = prek_hooks()
    targets = fuzz_targets()
    ci_targets = fuzz_ci_matrix()
    engines = engine_crates(fuzz_crates())
    scripts = check_scripts()
    channels = ir_harness_channels()
    bench_scenarios = sorted({name for names in channels.values() for name in names})
    baseline_scenarios = ir_scenarios_in_baseline()
    props = property_functions()

    hygiene_path = WORKFLOWS / "hygiene.yml"
    skip_match = re.search(r"SKIP: ([A-Za-z0-9,_-]+)", read(hygiene_path)) if hygiene_path.exists() else None
    skipped = sorted(x.strip() for x in skip_match.group(1).split(",")) if skip_match else []

    holes = []

    for target in sorted(set(ci_targets) - set(targets)):
        holes.append(["fuzz-target-undeclared", target, "the CI matrix names a target fuzz/Cargo.toml does not build"])
    for target in sorted(set(targets) - set(ci_targets)):
        holes.append(["fuzz-target-uncovered", target, "a built target is never run by CI"])
    for crate in engines:
        if not roundtrip_target_for(crate, targets):
            holes.append(
                [
                    "fuzz-no-roundtrip",
                    crate,
                    "this engine has a writer, and a parse-only target cannot reach the round-trip invariant",
                ]
            )

    if "prek run --all-files" not in all_commands:
        holes.append(["hook-tier-unwired", "prek", "no CI job runs the hook set over the tree"])
    for hook in skipped:
        entry = entries.get(hook, "")
        command = entry.split(" --")[0] if entry else hook
        if command not in all_commands:
            holes.append(
                ["hook-skipped-uncovered", hook, "hygiene.yml skips it and no workflow runs its command either"]
            )

    for script in scripts:
        stem = script[: -len(".py")]
        if stem not in all_commands and stem not in read(REPO / "prek.toml"):
            holes.append(["orphan-gate", script, "a checker exists that no workflow command and no hook invokes"])

    if "PROPTEST_CASES" not in all_commands:
        holes.append(
            [
                "property-tier",
                "default-case-count",
                "every CI run uses proptest's default case count, so low-frequency shapes never surface",
            ]
        )

    for scenario in sorted(set(bench_scenarios) - set(baseline_scenarios)):
        holes.append(["ir-unbaselined", scenario, "a measured scenario has no committed baseline number"])
    for scenario in sorted(set(baseline_scenarios) - set(bench_scenarios)):
        holes.append(["ir-stale-baseline", scenario, "the baseline carries a scenario the bench no longer defines"])

    # A bridge measured in one direction is a hole in the instrument rather than in the code, so it
    # is reported here instead of being asserted: the fix is a scenario, and until someone adds the
    # twin the gate should keep saying which half it cannot see.
    for scenario in ir_bridge_pairs(channels):
        holes.append(
            [
                "ir-bridge-unidirectional",
                scenario,
                "one half of a cross-format bridge has a scenario and the other does not, so per-key "
                "work on the missing half changes nothing the gate can observe",
            ]
        )

    for fmt in ir_bridges_absent(channels):
        holes.append(
            [
                "ir-bridge-absent",
                fmt,
                "the hub reads and writes this format, and the instruction gate has no scenario in "
                "either direction, so work added to that hot path cannot be adjudicated at all",
            ]
        )

    for rel in timing_floors_unpaired():
        holes.append(
            [
                "timing-floor-unpaired",
                rel,
                "a wall-clock gate in ordinary CI times its candidate and its reference in separate "
                "blocks, so one scheduler spike decides the verdict; sample the pair through "
                "tests/timing.py instead",
            ]
        )

    # The harness set is measured in both directions, because each half has already been wrong once.
    # A declared target whose file is missing leaves the gate silently shorter; a file whose crate
    # declares no target compiles never, so its scenarios would be numbers in a JSON file and nothing
    # else. And two harnesses naming one scenario makes the baseline ambiguous - `ir_gate.py` refuses
    # that at run time, and the probe refuses it here so the refusal is not the only defence.
    for crate in sorted(set(channels) - set(ir_harness_files())):
        holes.append(
            [
                "ir-harness-missing",
                crate,
                "the gate declares an ir_gate target for this crate, but benches/ir_gate.rs is not there",
            ]
        )
    for crate in sorted(set(ir_harness_files()) - set(channels)):
        holes.append(
            [
                "ir-harness-undeclared",
                crate,
                "a benches/ir_gate.rs exists in a crate with no `ir-gate` bench target, so nothing builds it",
            ]
        )
    owner_of: dict = {}
    for crate, names in channels.items():
        for name in names:
            if name in owner_of:
                holes.append(
                    [
                        "ir-scenario-duplicate",
                        name,
                        f"{owner_of[name]} and {crate} both list it, so one baseline number cannot say which was measured",
                    ]
                )
            else:
                owner_of[name] = crate

    ci_commands = " ".join(commands_by_workflow.get("ci.yml", []))
    if "--all-targets" not in ci_commands:
        holes.append(
            ["lint-scope", "clippy-all-targets", "CI's clippy command does not lint tests, benches or examples"]
        )

    # Whether any pull request *renders* the published pages. The site is built by a workflow that
    # triggers on push to `main`, so every property that only a generator decides - a metadata block it
    # must parse, Markdown inside an HTML block, an anchor a locale translated away - is discovered by
    # whoever opens the deployed page. Measured rather than asserted, because the alternative is a claim
    # about coverage that no file makes.
    if not renders_the_site():
        holes.append(
            [
                "docs-rendering",
                "unbuilt-on-pr",
                "no pull-request job builds the site, so a page that fails only when rendered reaches main",
            ]
        )

    # The ledger and this document promise files by name (`note_survival.rs` runs on every
    # PR). A promise that outlives the thing it promises is the quietly-worse kind of stale:
    # the reader trusts the gate exists. So every repository path named in backticks in
    # those two files has to exist, and a missing one is a hole like any other.
    declared = declared_artifacts()
    for path, exists in declared:
        if not exists:
            holes.append(
                [
                    "artifact-missing",
                    path,
                    "the ledger or the assessment promises this file, and the tree does not contain it",
                ]
            )

    # The two YAML writers (`Serializer` over parsed nodes, `direct_dump` over Python
    # objects) mirror each other by design instead of sharing code, which is precisely
    # how #287 ended up needing the same fix twice. A probe that only asked "does any
    # test mention the fast path" passed while no test compared the two routes, so the
    # requirement is the table itself: a module whose name says it holds the parity
    # table. Adding a writer route without extending that table is then a red test.
    route_parity = sorted(p.name for p in (REPO / "tests").glob("test_route_parity*.py"))
    if not route_parity:
        holes.append(
            [
                "route-parity",
                "node-writer-vs-direct-dump",
                "no table-driven test compares the two YAML writer routes on shared shapes",
            ]
        )

    # The instruction gate is the only reproducible instrument in the set (wall time swings by
    # points the Ir gate resolves to 0.0018%), so which crates it can *compile against* is the
    # boundary of what can be gated at all. Derived from the bench target's own manifest rather
    # than from a list of scenario names: the binding layer is where `safe_load` turns an AST into
    # Python objects, it is what #292 changed, and it appears in no scenario because no scenario
    # can - the harness never links that crate. Wall-time CodSpeed covers it, at the precision this
    # repository's own notes call unusable below ~10%.
    ir_graph = ir_harness_build_graph()
    served_by = binding_crate()
    if served_by and served_by not in ir_graph:
        holes.append(
            [
                "perf-coverage",
                "binding-layer",
                f"the instruction gate compiles against {sorted(ir_graph)} and never reaches {served_by}, "
                "so the AST-to-Python conversion the package is used for has no reproducible perf number",
            ]
        )

    # `AGENTS.md` says never commit partial changelog updates, and section positions are
    # locale-independent, so equal [Unreleased] counts at the same positions are the measurable form
    # of the rule. `check_changelog_mirrors.py` now asserts that instead of printing it, which closed
    # the registered hole this block used to derive; the counts stay reported here because a measure the
    # probe publishes is a measure a test can fire on.
    mirrors = changelog_mirror_counts()

    return {
        "ir_harness_crates": sorted(ir_graph),
        "ir_harness_owners": sorted(channels),
        "python_serving_crate": [served_by] if served_by else [],
        "engines_with_serializer": engines,
        "fuzz_targets": targets,
        "fuzz_ci_matrix": ci_targets,
        "prek_hooks": hooks,
        "hooks_skipped_in_ci": skipped,
        "check_scripts": scripts,
        "ir_scenarios_bench": bench_scenarios,
        "ir_scenario_channels": [f"{crate}:{len(names)}" for crate, names in sorted(channels.items())],
        "ir_scenarios_baseline": baseline_scenarios,
        "property_function_count": len(props),
        "property_functions": props,
        "artifact_paths": {"scanned": len(declared), "missing": sum(1 for _p, ok in declared if not ok)},
        "changelog_entry_counts": [f"{name}={counts}" for name, counts in sorted(mirrors.items())],
        "route_parity_tests": route_parity,
        "holes": sorted(holes),
    }


SUMMARY_KEYS = (
    "ir_harness_crates",
    "ir_harness_owners",
    "python_serving_crate",
    "engines_with_serializer",
    "fuzz_targets",
    "fuzz_ci_matrix",
    "prek_hooks",
    "hooks_skipped_in_ci",
    "check_scripts",
    "ir_scenarios_bench",
    "ir_scenario_channels",
    "ir_scenarios_baseline",
    "property_function_count",
    "route_parity_tests",
    "changelog_entry_counts",
)


def main(argv: list[str]) -> int:
    data = measure()
    if "--holes" in argv:
        for kind, name, _why in data["holes"]:
            print(f"{kind}:{name}")
        return 0
    if "--json" in argv:
        print(json.dumps(data, indent=2, sort_keys=True))
        return 0
    for key in SUMMARY_KEYS:
        value = data[key]
        rendered = str(value) if isinstance(value, int) else ", ".join(value)
        print(f"{key:24} {rendered}")
    print("")
    print(f"derived holes ({len(data['holes'])}):")
    for kind, name, why in data["holes"]:
        print(f"  - {kind}:{name} | {why}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
