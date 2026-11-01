"""Fail when the Markdown formatter is allowed to edit the inside of a code span.

`rumdl fmt` fixes MD038 ("Spaces inside code span elements") by *deleting* a space at either end of a
span. In this repository that byte is often content rather than typography: the docs quote raw YAML and
TOML, and a sample's indentation is the shape it names. Measured on 2026-11-01 with the pinned 0.2.78:
a page holding `` ` ? > #I` `` came out of `rumdl fmt` as `` `? > #I` ``, the run reporting only
`[MD038] Spaces inside code span elements [fixed]`, and the same rewrite landed on five changelog
mirrors plus `docs/dev/quality-ledger.md` while the hook printed `Passed` - a formatter whose fix is a
deletion will always win, so nothing in the tree showed that the byte had ever been there.

The rule is therefore disabled, and what this test holds is the disable *and its reason*: `rumdl check`
over all 178 tracked pages reports 0 MD038 findings today, so the rule buys nothing here, while the
cost of leaving it on is a silent content edit inside the very samples the documents exist to pin. A
pin without its reason written next to it is the pin that gets dropped by the next rule list edit, so
the reason is asserted too.

Read from the committed text rather than through `tomllib`: the Python floor of this project is 3.8 and
`tomllib` is 3.11+, and a guard that skips on the floor legs would be a check that never runs there.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CONFIG = REPO_ROOT / ".rumdl.toml"

DISABLE_LINE = re.compile(r"^extend-disable\s*=\s*\[(?P<array>[^\]]*)\](?P<comment>.*)$")

# The rules the repository had already decided to own elsewhere, each with its reason in the config
# comment. Adding a fourth must not quietly drop a prior one.
PRIOR_DISABLES = {"MD013", "MD051", "MD036"}


def disable_line() -> str:
    for line in CONFIG.read_text(encoding="utf-8").splitlines():
        if line.startswith("extend-disable"):
            return line
    raise AssertionError(".rumdl.toml no longer declares `extend-disable`")


def disabled_rules() -> tuple[set[str], str]:
    match = DISABLE_LINE.match(disable_line())
    assert match is not None, f"unrecognised extend-disable spelling: {disable_line()!r}"
    rules = {item.strip().strip("\"'") for item in match.group("array").split(",") if item.strip()}
    return rules, match.group("comment")


def test_md038_stays_disabled_because_its_fix_is_a_deletion():
    rules, _ = disabled_rules()
    assert "MD038" in rules, (
        "MD038 is enabled again, which lets `rumdl fmt` delete a space at the edge of a code span - "
        "for these pages that byte is a sample's indentation (see the module docstring for the "
        "reproduction). Keep it disabled, or name the byte that is not typography and move the "
        "sample into a fenced block."
    )


def test_the_disable_list_keeps_the_rules_it_already_owned():
    rules, _ = disabled_rules()
    missing = PRIOR_DISABLES - rules
    assert not missing, f"the disable list dropped {sorted(missing)}: each was a measured decision"


def test_the_config_carries_the_reason_next_to_the_rule():
    """A rule list without its reasons is a rule list that gets edited back."""
    _, comment = disabled_rules()
    assert "MD038" in comment, "MD038 is disabled by a list entry the comment does not explain"
    assert "code span" in comment, (
        f"the comment must name what the rule touches, so a reader can tell a sample's byte from "
        f"typography: {comment.strip()[:120]!r}"
    )
    assert re.search(r"delet|strip", comment), (
        f"the comment must say the fix removes a byte rather than restyling it: {comment.strip()[:120]!r}"
    )
