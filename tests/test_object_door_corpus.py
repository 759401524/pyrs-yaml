"""The object door over the committed YAML corpus: conserve or refuse, never reshape.

`seed_reach.rs` charges `parse()` -> `str()` over the same seeds, and `test_reader_parity.py`
charges the two readers against each other. Neither touches the door most users actually call:
`safe_load` builds Python objects and `safe_dumps` writes them back through `direct_dump`, a
writer whose input is an object rather than text, so no Rust fixed-point gate can reach it.
`test_route_parity.py` compares the two writers, but only over hand-picked documents.

What is asked here, per seed: the read either refuses in a class the conversion policy documents,
or it succeeds and the value survives `safe_dumps` -> `safe_load` unchanged and re-writes to the
same text. A refusal at the write stage, a reshaped value, or a second text that differs is a
failure. The documented refusal classes come from `crates/pyrs-yaml/src/py/convert.rs`: a
mapping used as a key answers `YamlTypeError` ("found unhashable key"), two distinct YAML keys
projecting to one Python key answer `YamlTypeError` ("two distinct YAML keys project..."), and a
sequence key whose member Python cannot hash surfaces Python's own `TypeError` carrying an
`unhashable type:` clause because the tuple is hashed by the interpreter, matching ruamel
(the surrounding wording of that clause changed in CPython 3.14, so it is matched as a substring).

What this gate cannot see, stated because a green check invites trust in it: the object door
carries no comments, anchors or styles, so nothing here speaks to their fidelity - `str(parse())`
owns that claim. It also cannot tell whether `safe_load` answered through direct load or fell
back to the AST; that question belongs to `test_reader_parity.py`.
"""

from __future__ import annotations

from pathlib import Path

import pytest

import pyrs_yaml

REPO = Path(__file__).resolve().parent.parent
CORPUS = REPO / "fuzz" / "seeds" / "yaml_roundtrip"

# The first two are raised by this library (convert.rs) and keep an exact prefix. The third is
# CPython's own TypeError for hashing a tuple that holds a mapping: 3.8-3.13 spell it
# "unhashable type: 'dict'", while 3.14+ prepend "cannot use 'tuple' as a dict key (" and keep
# "unhashable type:" as the identifying clause. ruamel gets the same interpreter message. Match
# that one as a substring so the documented class survives CPython's wording change.
DOCUMENTED_REFUSALS = (
    "found unhashable key:",
    "two distinct YAML keys project to the same Python key",
)
CPYTHON_UNHASHABLE = "unhashable type:"


def render(value: object) -> str:
    """A rendering that keeps mapping order and distinguishes a tuple key from a scalar.

    Plain `repr` is not enough on its own: a set iterates in hash order, so two equal sets can
    render differently, and `json.dumps` refuses a tuple key - the only spelling a non-scalar
    YAML key has in Python.
    """
    if isinstance(value, dict):
        return "{" + ", ".join(f"{render(k)}: {render(v)}" for k, v in value.items()) + "}"
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(render(v) for v in value) + "]"
    if isinstance(value, (set, frozenset)):
        return "set{" + ", ".join(sorted(render(v) for v in value)) + "}"
    return repr(value)


def cases() -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    if not CORPUS.is_dir():
        return out
    for path in sorted(CORPUS.iterdir()):
        if path.suffix != ".seed":
            continue
        try:
            src = path.read_bytes().decode("utf-8")
        except UnicodeDecodeError:
            continue  # the Rust door takes `&str`; a non-UTF-8 seed is not an input here
        out.append((path.name, src))
    return out


def charge(src: str) -> str | None:
    """`None` when the document conserved, otherwise the refusal message it answered with."""
    try:
        value = pyrs_yaml.safe_load(src)
    except (pyrs_yaml.YamlTypeError, TypeError) as refusal:
        return str(refusal)
    once = pyrs_yaml.safe_dumps(value)
    again = pyrs_yaml.safe_load(once)
    assert render(again) == render(value), f"the object door reshaped the value; it wrote: {once}"
    assert pyrs_yaml.safe_dumps(again) == once, f"the object door is not a text fixed point: {once}"
    return None


@pytest.mark.parametrize("name,src", cases(), ids=[name for name, _ in cases()])
def test_the_object_door_conserves_or_refuses_in_a_documented_class(name: str, src: str):
    refusal = charge(src)
    if refusal is None:
        return
    assert refusal.startswith(DOCUMENTED_REFUSALS) or CPYTHON_UNHASHABLE in refusal, (
        f"{name}: refused with a class the conversion policy does not document: {refusal}"
    )


def test_the_object_door_corpus_still_conserves_most_of_its_members():
    docs = cases()
    assert len(docs) >= 50, "the object-door gate walked an unexpectedly small corpus"
    # Measured 2026-11-18: 112 of 161 conserved; the 49 refusals were 46 mapping keys, 1
    # `<<` identity collision and 2 sequence keys holding a mapping. The floor sits well below
    # that so the gate fails on a wholesale shift (every document refused), not on one seed
    # moving between the documented classes.
    conserved = sum(1 for _name, src in docs if charge(src) is None)
    assert conserved >= 100, (
        f"only {conserved} of {len(docs)} corpus documents survived the object door; the "
        "conservation claim has narrowed to a refusal check"
    )
