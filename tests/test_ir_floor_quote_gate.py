"""Guards `scripts/check_ir_floor_quotes.py`: the verdict, the refusals, and its wiring.

The class this exists to close is measured, not hypothetical. Nine minutes after an Ir baseline
refresh landed, `QUALITY_MATRIX.md` quoted nine floors that the refresh had moved — by 15 to 31,977
instructions — and every gate in the repository was green, because nothing read prose against
`.ci/ir-baseline.json`. Only the two numbers a human had re-pointed in the same PR were right.

The second class is the one #662 opened. Seven scenarios now carry one floor per runner image, and an
enforcing leg reads its own image's row, never the flat column — but the marks still compared against
that column, so a paragraph could pass the gate while quoting a number the leg it describes is not
judged against. The two columns are not close: `load_toml_medium` reads 144,902,515 on
`20261023.334.3` and 142,467,241 on `20261026.354.1`, 1.71 % apart, and which one the flat column holds
depends on the image the last refresh happened to land on. A keyed mark therefore has to name its row.

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
KEYED = "load_toml_medium"

MARK = r"([\d,]+)<!--\s*ir-floor:\s*([A-Za-z0-9_]+)(?:\s*@\s*([^\s>]+))?\s*-->"


def floors():
    """The committed baseline, read the way the checker reads it."""
    import json

    return json.loads((REPO_ROOT / ".ci" / "ir-baseline.json").read_text(encoding="utf-8"))["scenarios"]


def rows():
    """The keyed floors by full row key, exactly as the file spells them."""
    import json

    data = json.loads((REPO_ROOT / ".ci" / "ir-baseline.json").read_text(encoding="utf-8"))
    return {str(k): {str(s): int(v) for s, v in (row or {}).items()} for k, row in data["image_floors"].items()}


def versions():
    return sorted(key.rsplit("image ", 1)[-1] for key in rows())


def image_of(scenario):
    """The row keys carrying one floor for `scenario`, so the fixture follows the file."""
    return sorted(key for key, row in rows().items() if scenario in row)


def disagreeing():
    """A (scenario, version) pair whose row differs from the flat column — the confusion to refuse."""
    table = floors()
    pairs = [
        (scenario, key.rsplit("image ", 1)[-1])
        for key, row in rows().items()
        for scenario, value in row.items()
        if scenario in table and value != table[scenario]
    ]
    assert pairs, (
        "no keyed row disagrees with the flat column, so the control that separates the two columns "
        "cannot be built from this file — revisit the rule rather than deleting the control"
    )
    return pairs


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


def row_marked(scenario, value, image):
    return f"{value:,}<!-- ir-floor: {scenario} @ {image} -->"


def test_the_real_page_passes_and_declares_at_least_one_floor():
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr
    assert "equal the committed baseline" in result.stdout


def test_every_marker_on_the_real_page_resolves_against_the_file():
    """The shipped page is checked here too, so the governed set cannot rot silently in a leg.

    A mark that names an image resolves against that image's row; a bare mark resolves against the
    flat column — and a bare mark is only legal for a scenario the file does not key.
    """
    text = PAGE.read_text(encoding="utf-8")
    table = floors()
    table_rows = rows()
    keyed = {scenario for row in table_rows.values() for scenario in row}
    matches = re.findall(MARK, text)
    assert matches, "QUALITY_MATRIX.md declares no current floor, so the gate has no subject"
    for digits, scenario, image in matches:
        value = int(digits.replace(",", ""))
        if image:
            keys = [key for key in table_rows if key.endswith(f"image {image}")]
            assert len(keys) == 1, (scenario, image, keys)
            assert scenario in table_rows[keys[0]], (scenario, image)
            assert value == table_rows[keys[0]][scenario], (scenario, image, digits)
        else:
            assert scenario not in keyed, f"{scenario} is keyed by image and must name its row"
            assert scenario in table, scenario
            assert value == table[scenario], (scenario, digits)


def test_a_keyed_floor_marked_without_an_image_is_refused(tmp_path):
    """The flat column is not the floor an enforcing run compares against for these scenarios."""
    value = floors()[KEYED]
    body = "# t\n\n" + marked(KEYED, value) + "\n" + marked(SCENARIO_B, floors()[SCENARIO_B]) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "keys by runner image, naming no image" in result.stdout
    for version in versions():
        assert version in result.stdout, (version, result.stdout)


def test_a_keyed_floor_quoting_the_flat_column_under_a_named_image_is_refused(tmp_path):
    """The precise confusion the marker exists to refuse: the right image, the wrong column's number.

    This is only constructible when some row disagrees with the flat column, which it does by 1.71 %
    on `load_toml_medium` — so the fixture reads the disagreement out of the file instead of trusting
    a hand-typed pair.
    """
    scenario, version = disagreeing()[0]
    flat = floors()[scenario]
    key = next(k for k in rows() if k.endswith(f"image {version}"))
    assert rows()[key][scenario] != flat
    body = "# t\n\n" + row_marked(scenario, flat, version) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert f"{scenario} on {version}" in result.stdout
    assert f"{flat:,}" in result.stdout and f"{rows()[key][scenario]:,}" in result.stdout


def test_a_keyed_floor_at_its_own_row_value_passes(tmp_path):
    """The positive shape, so the refusal above is not the checker simply disliking the syntax."""
    scenario, version = disagreeing()[0]
    key = next(k for k in rows() if k.endswith(f"image {version}"))
    value = rows()[key][scenario]
    body = "# t\n\n" + row_marked(scenario, value, version) + "\n"
    body += marked(SCENARIO_B, floors()[SCENARIO_B]) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 0, result.stdout + result.stderr
    assert "quoted from a row" in result.stdout


def test_a_row_reading_that_drifted_by_one_digit_is_refused(tmp_path):
    scenario, version = disagreeing()[0]
    key = next(k for k in rows() if k.endswith(f"image {version}"))
    value = rows()[key][scenario]
    body = "# t\n\n" + row_marked(scenario, value + 1, version) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert f"{value + 1:,}" in result.stdout and f"{value:,}" in result.stdout


def test_an_image_the_file_has_no_row_for_is_refused(tmp_path):
    scenario = KEYED
    value = rows()[image_of(scenario)[0]][scenario]
    body = "# t\n\n" + row_marked(scenario, value, "20990101.1.1") + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "no row for" in result.stdout


def test_an_image_on_a_scenario_the_file_does_not_key_is_refused(tmp_path):
    """A row that does not exist is not a floor: the flat column is the only one for these."""
    body = "# t\n\n" + row_marked(SCENARIO_A, floors()[SCENARIO_A], versions()[0]) + "\n"
    result = run(page(tmp_path, body))
    assert result.returncode == 1, result.stdout
    assert "keys no floor for it" in result.stdout


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
