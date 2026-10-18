"""Every JSON writer in the hub, judged by readers this repository does not own.

The write side had no outside authority. The tiers that exist compare our writer with our own
reader - a fixed point, which an implementation that is wrong twice in the same way satisfies just
as happily as one that is right twice. `QUALITY_MATRIX.md` credits the JSON family with
`test_json_test_suite.py` and `test_json5_spec_table.py`, and both of those ask the reader. So the
trees below are generated, emitted by `to_json`, `to_jsonc` and `to_json5`, and handed to Python's
`json` and to `pyjson5` - neither of which has ever seen our encoder - and what comes back has to
be the tree that went in.

Measured before it was pinned: 1,500 generated trees per writer, 0 refusals and 0 value changes on
all three routes. This is insurance rather than a bug report, and what it insures against is the
class #501 just fixed on the other side of the pipe: a dialect rule that reaches one direction and
not the other. A writer that decides a tab may be emitted raw, or that `NaN` needs no quotes, is
refused here by a stranger rather than agreed with by itself.

The number alphabet is limited to values all four readers represent identically, because the
comparison below is exact - a boundary of the claim, stated rather than hidden.
"""

import json
import random

import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - platform without a wheel
    pyjson5 = None

needs_pyjson5 = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 (JSON5 reference) not installed")

# Spelled by code point rather than by glyph: a raw line separator in this file is what ruff's
# RUF001 calls ambiguous, and a character no reviewer can see is a weak way to name a case.
NBSP = chr(0x00A0)
LINE_SEP = chr(0x2028)
E_ACUTE = chr(0x00E9)

# Text that a writer could mistake for something else: words the core schema resolves, numbers in
# a spelling that is not a number here, characters a dialect may not carry raw.
TEXTS = [
    "plain",
    "1",
    "true",
    "null",
    "",
    "   ",
    "a b",
    "\t",
    "x\x00y",
    "x\x1fy",
    E_ACUTE,
    NBSP,
    LINE_SEP,
    'quote"',
    "it's",
    "{}",
    "[]",
    "#h",
    ":c",
    "- i",
    "line\nbreak",
    "back\\slash",
    "NaN",
    "Infinity",
    "-Infinity",
    "1e5",
    ".5",
    "01",
    "+7",
    "k" * 40,
]
NUMBERS = [0, 1, -1, 3.5, -2.5, 0.5, 1_000_000, -7]
CHOICES = [True, False, None]

ROUTES = {
    "to_json": (lambda doc: doc.to_json(), json.loads),
    "to_jsonc": (lambda doc: doc.to_jsonc(), json.loads),
    "to_json5": (lambda doc: doc.to_json5(), lambda text: pyjson5.loads(text)),
}


def gen(depth, rng):
    if depth <= 0 or rng.random() < 0.35:
        kind = rng.random()
        if kind < 0.6:
            return rng.choice(TEXTS)
        if kind < 0.9:
            return rng.choice(NUMBERS)
        return rng.choice(CHOICES)
    if rng.random() < 0.5:
        return [gen(depth - 1, rng) for _ in range(rng.randint(0, 3))]
    return {rng.choice(TEXTS): gen(depth - 1, rng) for _ in range(rng.randint(0, 3))}


TREES = [gen(3, random.Random(seed)) for seed in range(1, 1501)]


@pytest.mark.parametrize(
    "route",
    [
        "to_json",
        "to_jsonc",
        pytest.param("to_json5", marks=needs_pyjson5),
    ],
)
def test_a_stranger_reads_what_the_writer_emitted(route):
    emit, read = ROUTES[route]
    for tree in TREES:
        # The tree enters the hub through the JSON reader, which is not the side under test, so
        # the document the writer produces is the only thing this route contributes.
        document = pyrs_yaml.parse(pyrs_yaml.from_json(json.dumps(tree)))
        text = emit(document)
        assert read(text) == tree, f"{route}: {text!r} decoded {read(text)!r}, not {tree!r}"


def test_the_generated_forest_is_the_one_the_gate_claims():
    # The count and the shapes are what the docstring measured; a silent shrink here would turn
    # the claim above into a number nobody checked.
    assert len(TREES) == 1500
    assert any(isinstance(t, dict) for t in TREES)
    assert any(isinstance(t, list) for t in TREES)
    assert {"NaN", "Infinity", "\t", "x\x00y"} <= set(TEXTS)
    assert {NBSP, LINE_SEP, E_ACUTE} <= set(TEXTS)
