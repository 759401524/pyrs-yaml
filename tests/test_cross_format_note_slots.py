"""A note in any slot survives every comment-carrying export, or the export names why it cannot.

Written after two blind slots in the same family were closed in one release: the TOML writer read the
mapping's own inline note and never the key node's, which lost `sec: # note` on the section header
(#618, ledger `(fx)`) and then `b: # note` on an entry line inside a section body (#621, `(fy)`). Both
were silent drops, and nothing in the suite asked the question of the other writers at all.

So this is the family pinned as a table rather than as two anecdotes. Every row is a YAML document with
one note in one slot, exported four ways: `to_yaml` (the editing substrate), `to_toml`, `to_jsonc` and
`to_json5` - the three other formats with comment syntax. A row passes when the note's token reaches the
output, or when the export *refuses* with a reason that is true about the target format rather than
about notes. What is never allowed is the third outcome: text in, text silently gone.

Tokens are compared bare (`note03`, not `# note03`) because every writer is allowed to restyle the
comment - TOML writes `# x`, YAML may write `#x` - and restyling is not the claim being tested.
"""

from __future__ import annotations

import pytest

import pyrs_yaml

# (slot, document, token). Each document is written the way a user would write it; the indentation of a
# nested body is part of the case, not decoration - `b: # note` over a block mapping is the shape that
# reaches the writer on the KEY node, which is what both closed defects were about.
SLOTS = [
    ("leading above a root key", "# note01\na: 1\n", "note01"),
    ("inline on a root leaf", "a: 1 # note02\n", "note02"),
    ("key line of a root section", "m: # note03\n  x: 1\n", "note03"),
    ("key line of a second section", "p:\n  q: # note04\n    x: 1\n", "note04"),
    ("key line above a sequence", "s: # note05\n  - 1\n", "note05"),
    ("inline on a sequence item", "arr:\n  - 1 # note06\n", "note06"),
    ("leaf inside a map in a sequence", "arr2:\n  - a: 1 # note07\n", "note07"),
    ("key line inside a map in a sequence", "arr2:\n  - b: # note08\n      c: 1\n", "note08"),
    ("key line above a table array", "p:\n  q: # note09\n    - a: 1\n", "note09"),
    ("key line two levels deep", "p:\n  q:\n    r: # note10\n      x: 1\n", "note10"),
    ("key line above a block scalar", "p:\n  s: # note11\n    |\n    line\n", "note11"),
]

EXPORTS = {
    "to_yaml": lambda src: pyrs_yaml.parse(src).to_yaml(),
    "to_toml": lambda src: pyrs_yaml.to_toml(src),
    "to_jsonc": lambda src: pyrs_yaml.parse(src).to_jsonc(),
    "to_json5": lambda src: pyrs_yaml.parse(src).to_json5(),
}

# Refusals that are true statements about the target format - an alias or a null has nowhere to go in
# TOML or JSON. Every row below is representable in all four, so the sets stay empty on purpose: a
# refusal on this table is the blind-slot bug showing up, not a limitation to be signed off.
HONEST_REFUSALS: dict[str, tuple[str, ...]] = {
    "to_yaml": (),
    "to_toml": (),
    "to_jsonc": (),
    "to_json5": (),
}


@pytest.mark.parametrize("export", sorted(EXPORTS))
@pytest.mark.parametrize("slot,document,token", SLOTS, ids=[row[0] for row in SLOTS])
def test_a_note_is_written_or_the_export_says_why(slot, document, token, export):
    emit = EXPORTS[export]
    try:
        out = emit(document)
    except pyrs_yaml.YamlSerializeError as error:
        reason = str(error)
        allowed = HONEST_REFUSALS[export]
        assert reason in allowed, (
            f"{slot!r} -> {export} refused with {reason!r}; this row is representable, so the only "
            f"accepted refusals are {allowed or 'none'}"
        )
        return
    assert token in out, f"the note in slot {slot!r} vanished on the way to {export}: input {document!r} wrote {out!r}"
