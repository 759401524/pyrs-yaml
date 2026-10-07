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


def ir_scenarios_in_bench() -> list[str]:
    text = read(REPO / "crates" / "pyrs-yaml-core" / "benches" / "ir_gate.rs")
    body = text.split("fn scenarios()", 1)[-1].split("fn main()", 1)[0]
    # Scenario names are the lowercase string literals; the inputs are `SCREAMING_CASE`
    # constants and `Work::Variant` paths. Matching the whole tuple instead missed
    # `serialize_block_scalars`, whose name sits on its own line after a wrap.
    return sorted(set(re.findall(r'"([a-z_0-9]{4,})"', body)))


def ir_scenarios_in_baseline() -> list[str]:
    return sorted(json.loads(read(REPO / ".ci" / "ir-baseline.json"))["scenarios"])


def property_functions() -> list[str]:
    names = []
    for rs in sorted((REPO / "crates").rglob("*.rs")):
        for match in re.finditer(r"fn (prop_[a-z0-9_]+)\(", read(rs)):
            names.append(f"{rs.relative_to(REPO).as_posix()}::{match.group(1)}")
    return sorted(names)


def measure() -> dict:
    commands_by_workflow = workflow_commands()
    all_commands = " ".join(command for commands in commands_by_workflow.values() for command in commands)
    entries = prek_entries()
    hooks = prek_hooks()
    targets = fuzz_targets()
    ci_targets = fuzz_ci_matrix()
    engines = engine_crates(fuzz_crates())
    scripts = check_scripts()
    bench_scenarios = ir_scenarios_in_bench()
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

    ci_commands = " ".join(commands_by_workflow.get("ci.yml", []))
    if "--all-targets" not in ci_commands:
        holes.append(
            ["lint-scope", "clippy-all-targets", "CI's clippy command does not lint tests, benches or examples"]
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

    return {
        "engines_with_serializer": engines,
        "fuzz_targets": targets,
        "fuzz_ci_matrix": ci_targets,
        "prek_hooks": hooks,
        "hooks_skipped_in_ci": skipped,
        "check_scripts": scripts,
        "ir_scenarios_bench": bench_scenarios,
        "ir_scenarios_baseline": baseline_scenarios,
        "property_function_count": len(props),
        "property_functions": props,
        "route_parity_tests": route_parity,
        "holes": sorted(holes),
    }


SUMMARY_KEYS = (
    "engines_with_serializer",
    "fuzz_targets",
    "fuzz_ci_matrix",
    "prek_hooks",
    "hooks_skipped_in_ci",
    "check_scripts",
    "ir_scenarios_bench",
    "ir_scenarios_baseline",
    "property_function_count",
    "route_parity_tests",
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
