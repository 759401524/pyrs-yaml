"""Guards `scripts/check_ledger_order.py`: the verdict, both failure modes, the refusals, and its wiring.

The rule itself is the ledger's citation scheme. `(details: quality-ledger (bf))` names a section only
if the `### (xy)` letters run increasing and each appears once; `(bh)` was found after `(bi)` and `(ch)`
after `(cj)`, and with two sections sharing a letter a pointer names neither. Both fail invisibly in the
file, because the ledger grows by insertion and each branch anchors wherever its neighbour happened to be.

What moved here from the test that used to own the rule is the measurement, not the assumption: a head
whose only defect was one section placed a letter early passed `prek hook set on the full tree`, the docs
gates, `rust-lint` and the Ir leg on 2026-10-26, and then reddened all twelve `test` legs - three OSes,
3.8 through 3.15 - because the rule was judged only by the Python tier, ten minutes and twelve runners
away from the edit. The script is the single implementation; the hook is the cheap gate; these tests are
what stops either of them from passing on nothing.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_ledger_order.py"
LEDGER = REPO_ROOT / "docs" / "dev" / "quality-ledger.md"
HEADING = re.compile(r"^### \(([a-z]{2})\)", re.M)


def run(*paths):
    return subprocess.run(
        [sys.executable, str(SCRIPT)] + [str(p) for p in paths],
        capture_output=True,
        text=True,
    )


def ledger(tmp_path, body, name="ledger.md"):
    path = tmp_path / name
    path.write_text(body, encoding="utf-8")
    return path


def test_the_live_ledger_passes_and_measures_something():
    """A run that found no section at all would report the same clean line, so the count is read back."""
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr
    counted = re.match(r"OK: (\d+) lettered section", result.stdout)
    assert counted, result.stdout
    assert int(counted.group(1)) == len(HEADING.findall(LEDGER.read_text(encoding="utf-8")))


def test_a_section_placed_one_letter_early_is_refused_by_name(tmp_path):
    path = ledger(tmp_path, "### (ab) later\n\ntext\n\n### (aa) earlier\n\nmore\n")
    result = run(path)
    assert result.returncode == 1
    assert "(aa) follows (ab)" in result.stdout, result.stdout


def test_a_repeated_letter_is_named_as_ambiguous_not_merely_out_of_order(tmp_path):
    """A repeat is also a step backwards, so both messages have to appear: only the second one tells the
    author that a citation to `(aa)` now names two sections."""
    path = ledger(tmp_path, "### (aa) first\n\ntext\n\n### (aa) second\n\nmore\n")
    result = run(path)
    assert result.returncode == 1
    assert "(aa) follows (aa)" in result.stdout
    assert "(aa) is used twice" in result.stdout, result.stdout


def test_gaps_in_the_letter_sequence_are_not_reported(tmp_path):
    """Branches claim letters and close by other routes; `ak`-`an` belong to the older entry form."""
    path = ledger(tmp_path, "### (aa) one\n\ntext\n\n### (ac) three\n\nmore\n")
    assert run(path).returncode == 0


def test_the_older_bold_lead_in_form_stays_out_of_the_rule(tmp_path):
    """The same file carries `**(am) …**` entries under family headings. Folding them in would assert a
    convention the ledger does not have, so they must not move the verdict in either direction."""
    path = ledger(tmp_path, "**(zz) an older entry**\n\ntext\n\n### (aa) one\n\nmore\n")
    assert run(path).returncode == 0


def test_a_ledger_carrying_no_lettered_section_refuses_rather_than_passes(tmp_path):
    """`files:` matched, the script ran, the pattern found nothing - that is the shape of a silent pass."""
    path = ledger(tmp_path, "## Family\n\nnothing in the `### (xy)` form here\n")
    result = run(path)
    assert result.returncode == 2, result.stdout + result.stderr
    assert "no '### (xy)' section" in result.stderr


def test_an_unreadable_ledger_refuses(tmp_path):
    result = run(tmp_path / "absent.md")
    assert result.returncode == 2, result.stdout + result.stderr
    assert "cannot read" in result.stderr


def test_the_real_shape_of_the_incident_is_refused(tmp_path):
    """The control that prices this gate: the live ledger with its last section moved above the one
    before it, which is exactly what `(gs)` did to `(gr)` on 2026-10-26."""
    text = LEDGER.read_text(encoding="utf-8")
    heads = list(HEADING.finditer(text))
    assert len(heads) >= 2, "the live ledger has no adjacent sections to swap"
    last, prev = heads[-1], heads[-2]
    moved = text[: prev.start()] + text[last.start() :] + text[prev.start() : last.start()]
    result = run(ledger(tmp_path, moved))
    assert result.returncode == 1, result.stdout + result.stderr
    assert f"({prev.group(1)}) follows ({last.group(1)})" in result.stdout, result.stdout


def hook_block(text, hook_id):
    for block in text.split("[[repos.hooks]]"):
        if f'id = "{hook_id}"' in block:
            return block
    return ""


def test_the_hook_is_registered_and_its_files_pattern_reaches_the_ledger():
    """`files:` is evaluated before the script runs, so a pattern that never matches is a silent pass."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    block = hook_block(prek, "ledger-order")
    assert block, "the ledger-order hook is not in prek.toml"
    assert "python scripts/check_ledger_order.py" in block

    files = re.search(r"files\s*=\s*\"([^\"]*)\"", block)
    assert files, block
    pattern = files.group(1).encode("utf-8").decode("unicode_escape")
    assert re.match(pattern, "docs/dev/quality-ledger.md"), pattern
    # And it must not run on every markdown page in the tree.
    assert not re.match(pattern, "CHANGELOG.md")
    assert not re.match(pattern, "docs/dev/perf.md")
