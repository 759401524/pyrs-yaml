"""The two YAML readers must agree on the value they build, key order included.

`safe_load` tries the direct-load fast path first and falls back to the AST the event receiver
builds (`crates/pyrs-yaml/src/py/functions.rs`). That is two readers of the same grammar inside
one library, and the existing tiers ask each of them their own questions: the direct tier pins
anchor replay shapes by hand, the round-trip tier pins the AST. Neither asks whether they
**disagree** — and a divergence there is invisible to users as a class, showing up only as one
document loading differently through `load()` than through `parse().to_dict()`.

The ask is the stronger of the two available: not equal contents, but equal *order* too, because
mapping order is this library's own promise. Inputs that either reader rejects are skipped
together (a shared refusal is agreement); inputs that both accept must produce the same graph.

**What this gate cannot see, stated because a green check invites trust in it.** Nothing tells the
test whether the fast path actually *served* an input or bailed to the AST, and a bail makes the
comparison trivially true. What it can check is reachability, so it does: `reaches_fast_path`
mirrors direct load's textual vetoes and the corpus floor requires a floor of inputs that clear all
of them (measured 2026-10-20: 37 of 106). That is eligibility, not selection — proving selection
needs instrumentation inside the binding — but a corpus that drifts entirely into anchor/tag shapes
would narrow the claim to the AST agreeing with itself, and that drift is now a red run.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import pyrs_yaml

REPO = Path(__file__).resolve().parent.parent
SEED_DIRS = ["yaml_roundtrip", "parse_yaml", "edit_yaml"]


def document_of(raw: bytes) -> str | None:
    """The document half of a seed; `edit_yaml` inputs append an edit script after a NUL."""
    cut = raw.find(b"\x00")
    text = raw if cut < 0 else raw[:cut]
    try:
        return text.decode("utf-8")
    except UnicodeDecodeError:
        return None


def _jsonable(value: object) -> object:
    """A JSON-safe shape for any value, with a mapping's order kept as a list of pairs."""
    if isinstance(value, dict):
        return {"__pairs__": [[_jsonable(k), _jsonable(v)] for k, v in value.items()]}
    if isinstance(value, (list, tuple)):
        return [_jsonable(v) for v in value]
    if isinstance(value, (set, frozenset)):
        return {"__set__": sorted(json.dumps(_jsonable(v), default=repr) for v in value)}
    return value


def canonical(value: object) -> str:
    """A rendering that keeps mapping order, so a reordered pair is a failure.

    Keys are rendered by the same rule as values. A complex YAML key arrives as a tuple - the only
    hashable spelling a non-scalar key has in Python - and plain `json.dumps` refuses a tuple as a
    key, which is what a seed-corpus document raised: both readers had already built the same
    `{('x',): 'y'}`, so the refusal was the comparison's, not the engine's.
    """
    return json.dumps(_jsonable(value), sort_keys=False, default=repr)


def cases() -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for name in SEED_DIRS:
        directory = REPO / "fuzz" / "seeds" / name
        if not directory.is_dir():
            continue
        for path in sorted(directory.iterdir()):
            if path.suffix != ".seed":
                continue
            src = document_of(path.read_bytes())
            if src is None:
                continue
            out.append((path.name, src))
    return out


@pytest.mark.parametrize("name,src", cases(), ids=[name for name, _ in cases()])
def test_both_readers_build_the_same_value_graph(name: str, src: str):
    # A refusal is a result, so both readers are asked and both answers are kept: the `as err`
    # binding of an `except` clause is deleted when the block ends, which is why the reason is
    # captured as data here rather than re-read from the exception name later.
    fast: object = None
    fast_error: tuple[str, str] | None = None
    try:
        fast = pyrs_yaml.safe_load(src)
    except Exception as error:
        fast_error = (type(error).__name__, str(error))

    ast: object = None
    ast_error: tuple[str, str] | None = None
    try:
        ast = pyrs_yaml.parse(src).to_dict()
    except Exception as error:
        ast_error = (type(error).__name__, str(error))

    if fast_error is not None and ast_error is not None:
        assert fast_error[0] == ast_error[0], (
            f"{name}: both readers refused, but not as the same kind: {fast_error} vs {ast_error}"
        )
        return
    assert fast_error is None, f"{name}: the fast path refused input the AST reader accepted: {fast_error}"
    assert ast_error is None, f"{name}: the AST reader refused input the fast path accepted: {ast_error}"
    assert canonical(fast) == canonical(ast), f"{name}: readers diverge"


def test_the_canonical_rendering_keeps_mapping_order():
    # Without this, the ask above could silently degrade to set equality: two readers that agree
    # on contents but disagree on order would pass, and order is the promise this library makes.
    assert canonical({"a": 1, "b": 2}) != canonical({"b": 2, "a": 1})
    assert canonical([{"a": 1, "b": 2}]) != canonical([{"b": 2, "a": 1}])
    # And a key must not be flattened into the value it holds. `flow-sequence-key-two-note-stack.seed`
    # reads as `{("x",): "y"}` through both readers; were the tuple rendered as `"x"`, the gate would
    # call that document identical to `{x: y}` and could not see a key-shape divergence at all.
    assert canonical({("x",): "y"}) != canonical({"x": "y"})
    assert canonical({("x",): "y"}) != canonical({("x", "z"): "y"})
    assert canonical({("x",): "y"}) == canonical({("x",): "y"})


def reaches_fast_path(src: str) -> bool:
    """Whether `safe_load` can answer this document through direct load at all.

    Mirrors `replicable_events` in `crates/pyrs-yaml-core/src/parser/yaml/direct.rs`, which is the
    decision itself and is now pinned there by two Rust tests (13 shapes). The mirror is line-for-line
    on purpose: the byte scan for `&`, then a separator *line* after the first - `---`, `...`, or
    either with a trailing space and text - then the event layer's bails on any tag sigil and on a
    `<<` key. The tag and merge checks are deliberately stricter than the event layer here (a `!`
    inside a quoted scalar is not a tag), so an input this calls eligible is one direct load will not
    bail on for those reasons.

    The decision moved into the engine crate because it is a property of the document and the
    grammar, not of the Python object the value becomes - which is also why its own test tier needs
    no Python runtime.

    Eligible is not the same as *selected* — only instrumentation at the call site could prove that
    — but a corpus with no eligible document would make the comparison above structurally unable to
    reach the fast path, and nothing else in the repo would notice.
    """
    if "&" in src or "!" in src or "<<" in src:
        return False
    lines = src.splitlines()[1:]
    return not any(
        line == "---" or line == "..." or line.startswith("--- ") or line.startswith("... ") for line in lines
    )


def test_the_parity_corpus_is_not_empty():
    docs = cases()
    assert len(docs) >= 50, "the reader-parity gate walked an unexpectedly small corpus"
    # Measured 2026-10-20: 37 of the 106 corpus documents clear every textual veto, so roughly a
    # third of this gate's inputs can reach direct load at all. The floor is set well below that
    # because a corpus that drifts toward anchor/tag shapes would quietly narrow the claim to the
    # AST agreeing with itself.
    eligible = sum(1 for _name, src in docs if reaches_fast_path(src))
    assert eligible >= 15, (
        f"only {eligible} corpus documents can reach the fast path; the parity claim has become an "
        "AST-against-itself comparison for nearly every input"
    )
