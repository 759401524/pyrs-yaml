"""Guards `scripts/check_ir_floor_quotes.py`: the verdict, the four ways it refuses, and its wiring.

The class this exists to close is measured, not hypothetical. Nine minutes after an Ir baseline
refresh landed, `QUALITY_MATRIX.md` quoted nine floors that the refresh had moved — by 15 to 31,977
instructions — and every gate in the repository was green, because nothing read prose against
`.ci/ir-baseline.json`. Only the two numbers a human had re-pointed in the same PR were right.

Each control here is the shape that actually went wrong at least once while the checker was being
written: a one-digit drift, a renamed scenario, an orphan marker left behind when its number was
deleted, a reading without thousands separators, and a governed page that declares nothing.
"""

import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_ir_floor_quotes.py"
PAGE = REPO_ROOT / "QUALITY_MATRIX.md"

# One page, four readings, all four marked. `1234` is not the committed floor of anything, so the
# fixture is only ever handed to the checker as an explicit path.
SCENARIO_A = "parse_small"
SCENARIO_B = "serialize_small"


def floors():
    """The committed baseline, read the way the checker reads it."""
    import json

    return json.loads((REPO_ROOT / ".ci" / "ir-baseline.json").read_text(encoding="utf-8"))["scenarios"]


def run(*paths):
    return subprocess.run(
        [sys.executable, str(SCRIPT)] + [str(p) for p in paths],
        capture_output=True,
        text=True,
    )


def page(tmp_path, body, name="page.md"):
    path = tmp_path / name
    path.write_text(body, encoding="utf-8")
    return path


def marked(scenario, value):
    return f"{value:,}<!-- ir-floor: {scenario} -->"


def test_the_real_page_passes_and_declares_at_least_one_floor():
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr
    assert "equal the committed baseline" in result.stdout


def test_every_marker_on_the_real_page_resolves_against_the_file():
    """The shipped page is checked here too, so the governed set cannot rot silently in a leg."""
    text = PAGE.read_text(encoding="utf-8")
    table = floors()
    matches = re.findall(r"([\d,]+)<!--\s*ir-floor:\s*([A-Za-z0-9_]+)\s*-->", text)
    assert matches, "QUALITY_MATRIX.md declares no current floor, so the gate has no subject"
    for digits, scenario in matches:
        assert scenario in table, scenario
        assert int(digits.replace(",", "")) == table[scenario], (scenario, digits)


def test_a_reading_that_drifted_by_one_digit_is_refused(tmp_path):
    value = floors()[SCENARIO_A]
    bad = value + 1
    body = "# t\n\n" + marked(SCENARIO_A, bad) + "\n" + marked(SCENARIO_B, floors()[SCENARIO_B]) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert SCENARIO_A in result.stdout and f"{bad:,}" in result.stdout and f"{value:,}" in result.stdout


def test_a_marker_naming_an_unknown_scenario_is_refused(tmp_path):
    body = "# t\n\n1,234<!-- ir-floor: no_such_scenario_anywhere -->\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "does not contain" in result.stdout


def test_an_orphan_marker_is_refused_rather_than_ignored(tmp_path):
    """The false green found while writing the checker.

    A pattern that matched a number *and* a marker let a page whose number had been deleted, leaving
    the declaration behind, report OK with one fewer reading than there were markers.
    """
    body = "# t\n\n" + marked(SCENARIO_A, floors()[SCENARIO_A]) + "\n\n<!-- ir-floor: " + SCENARIO_B + " -->\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "carries no reading" in result.stdout


def test_an_unseparated_reading_is_refused(tmp_path):
    value = floors()[SCENARIO_A]
    body = "# t\n\n" + str(value) + "<!-- ir-floor: " + SCENARIO_A + " -->\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "thousands-separated" in result.stdout


def test_a_page_declaring_nothing_is_not_a_pass(tmp_path):
    """Nothing measured reads as a scope change, so it must not reach the merge verdict as green."""
    body = "# t\n\nSome prose quoting 321,690,260 instructions without declaring it current.\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 2, result.stdout
    assert "cannot measure" in result.stdout


def test_an_unreadable_governed_page_is_not_a_pass(tmp_path):
    result = run(tmp_path / "there_is_no_such_page.md")
    assert result.returncode == 2, result.stdout
    assert "cannot measure" in result.stdout


def test_history_without_a_marker_is_left_alone(tmp_path):
    """The reason the rule is a declaration: unmarked numbers are the page's right to keep."""
    past = floors()[SCENARIO_A] + 999_999
    body = (
        "# t\n\n"
        + "The runner measured {} on the unpinned pair, ITERATIONS = 2_000, at widths of 9 and 256.\n".format(
            f"{past:,}"
        )
        + marked(SCENARIO_B, floors()[SCENARIO_B])
        + "\n"
    )
    result = run(page(tmp_path, body))
    assert result.returncode == 0, result.stdout + result.stderr


# ── wiring: a checker nobody invokes is a report, not a gate ─────────────────────────


def hook_block(text, hook_id):
    for block in text.split("[[repos.hooks]]"):
        if f'id = "{hook_id}"' in block:
            return block
    return ""


def test_the_hook_is_registered_and_its_files_pattern_reaches_its_pages():
    """`files:` is evaluated before the script runs, so a pattern that never matches is a silent pass."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    block = hook_block(prek, "ir-floor-quotes")
    assert block, "the ir-floor-quotes hook is not in prek.toml"
    assert "python scripts/check_ir_floor_quotes.py" in block

    files = re.search(r"files\s*=\s*\"([^\"]*)\"", block)
    assert files, block
    pattern = files.group(1).encode("utf-8").decode("unicode_escape")
    for path in ("QUALITY_MATRIX.md", ".ci/ir-baseline.json"):
        assert re.match(pattern, path), (pattern, path)
    # And it must not fire on every markdown page in the tree.
    assert not re.match(pattern, "docs/en/changelog.md")
    assert not re.match(pattern, "docs/dev/perf.md")


def test_the_workflow_runs_the_checker_directly():
    """The hook is the commit path; the `docs-gates` job needs its own line, or the verdict never sees it."""
    ci = (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    assert "python scripts/check_ir_floor_quotes.py" in ci


@pytest.mark.parametrize(
    "scenario",
    ["serialize_note_flow", "to_jsonc_notes", "to_json5_notes", "from_jsonc_notes", "from_json5_notes"],
)
def test_the_note_bearing_floors_are_declared_on_the_page(scenario):
    """The five readings a refresh moves most often are among the marked ones."""
    text = PAGE.read_text(encoding="utf-8")
    assert f"<!-- ir-floor: {scenario} -->" in text, scenario
