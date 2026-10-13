"""JSONTestSuite conformance runner (the de-facto language-agnostic JSON corpus).

Handled like ``test_toml_test_suite.py``: the corpus is an untracked local dev artifact
(``Reference/JSONTestSuite``, excluded by ``.git/info/exclude``) and CI clones it, so every test
``skipif`` the directory is absent.

Each file in ``test_parsing`` is named ``y_*`` (must parse), ``n_*`` (must be rejected) or ``i_*``
(the corpus itself declares the outcome implementation-defined). Only the first two are asserted -
excluding ``i_*`` is the corpus's own instruction, not a licence to skip inconvenient cases, and the
counts are pinned below so the exclusion cannot quietly grow.

Measured on this repository's strict RFC 8259 loader: **283 determinate cases, 0 disagreements**, so
the floor is ``1.0`` rather than a comfort value. A second test asserts the *shape* of every
rejection: a typed ``YamlParseError``, because "it raised something" is not the same claim as "it
refused the document". Twelve corpus files are stored as bytes that are not valid UTF-8 and are
excluded from that second check - they cannot reach any parser through a text API.

The value fidelity of accepted documents is covered elsewhere (``test_property_dialects.py`` against
stdlib ``json``, ``test_json.py``); this file is about the accept/reject boundary, which is where
malformed input hits.
"""

from __future__ import annotations

from pathlib import Path

import pytest

import pyrs_yaml

SUITE_DIR = Path("Reference/JSONTestSuite/test_parsing")

pytestmark = pytest.mark.skipif(
    not SUITE_DIR.is_dir(),
    reason=(
        "JSONTestSuite corpus not found "
        "(clone https://github.com/nst/JSONTestSuite into Reference/; CI does this for you)"
    ),
)


def _cases():
    for path in sorted(SUITE_DIR.iterdir()):
        if not path.is_file():
            continue
        kind = path.name[:2]
        if kind in ("y_", "n_"):
            yield path, kind == "y_"


def _text(path: Path) -> str:
    """File contents as text; raises `UnicodeDecodeError` for the corpus files that are not UTF-8."""
    return path.read_bytes().decode("utf-8")


def _decodable_cases() -> tuple[list[tuple[Path, bool]], int]:
    """Corpus cases that can reach a text-taking loader, plus how many could not.

    Twelve corpus files are stored as bytes that are not valid UTF-8, and `load_json*` takes text, so
    those never reach any parser through this binding. They are counted, not quietly dropped: the
    number is asserted by the caller, so the boundary cannot grow unnoticed.
    """
    usable: list[tuple[Path, bool]] = []
    undecodable = 0
    for path, accept in _cases():
        try:
            _text(path)
        except UnicodeDecodeError:
            undecodable += 1
            continue
        usable.append((path, accept))
    return usable, undecodable


def _load(path: Path):
    """Feed the file the way the loader is reached in practice: text decoded from bytes.

    A file that is not valid UTF-8 cannot be handed over at all, and refusing to decode it *is* the
    rejection the corpus expects of ``n_*``. For ``y_*`` it would be a defect, which the caller
    reports because it passes the decode check the same way as any other.
    """
    raw = path.read_bytes()
    text = raw.decode("utf-8")
    return pyrs_yaml.load_json(text)


def test_every_determinate_case_agrees_with_the_corpus():
    """95 `y_*` accepted, 188 `n_*` refused - no partial credit, no drift."""
    cases = list(_cases())
    assert len(cases) >= 280, f"only {len(cases)} determinate cases found; the corpus did not unpack"
    indeterminate = [p for p in SUITE_DIR.iterdir() if p.name.startswith("i_")]
    assert len(indeterminate) >= 30, f"{len(indeterminate)} `i_*` cases: the corpus changed, re-read it"

    failures = []
    for path, should_accept in cases:
        try:
            _load(path)
            accepted = True
            detail = ""
        except Exception as exc:  # an unexpected exception is a data point, not a pass
            accepted = False
            detail = f"{type(exc).__name__}: {str(exc)[:90]}"
        if accepted is not should_accept:
            verb = "rejected" if not accepted else "accepted"
            failures.append(f"{path.name}: {verb}, corpus says {'accept' if should_accept else 'reject'} {detail}")
    assert not failures, f"{len(failures)}/{len(cases)} disagreements:\n" + "\n".join(failures[:20])


def test_every_valid_json_document_is_accepted_by_the_looser_dialects():
    """Containment: JSONC and JSON5 are supersets, so they must accept all 95 valid documents.

    This was measured before it was asserted, and it found nothing wrong - which is the point of
    pinning it. A dialect that grows a rejection it should not have is exactly as broken as one that
    grows an acceptance, and only the corpus can see either.
    """
    cases = [(path, accept) for path, accept in _cases() if accept]
    assert len(cases) >= 90, f"only {len(cases)} valid cases found"
    for loader in (pyrs_yaml.load_jsonc, pyrs_yaml.load_json5):
        rejected = []
        for path, _accept in cases:
            try:
                text = _text(path)
            except UnicodeDecodeError:
                continue  # cannot reach a text loader; counted by the boundary test
            try:
                loader(text)
            except Exception as exc:
                rejected.append(f"{path.name}: {type(exc).__name__}: {str(exc)[:70]}")
        assert not rejected, f"{loader.__name__} rejects valid JSON:\n  " + "\n  ".join(rejected[:10])


def test_json5_agrees_with_the_reference_where_the_corpus_is_invalid():
    """Where `pyjson5` refuses an invalid-JSON document, so must we.

    This is the rule that found the leading-zero defect: the parser accepted `[012]`, `[-01]` and
    `[-012]` under a comment claiming JSON5 allows a leading zero, while JSON5's
    `DecimalIntegerLiteral` is `0 | NonZeroDigit DigitalDigits?` - the production RFC 8259 uses - and
    the reference implementation rejects all three. Files the reference *accepts* are skipped on
    purpose: those are the dialect's legitimate relaxations (hex, `.5`, `5.`, single quotes, unquoted
    keys, trailing commas, comments), and demanding rejection there would be a worse contract than the
    spec.
    """
    pyjson5 = pytest.importorskip("pyjson5", reason="reference JSON5 parser not installed")
    cases, undecodable = _decodable_cases()
    assert undecodable == 12, f"{undecodable} non-UTF-8 corpus files, expected 12"
    # One file is exempt, and the exemption is decided on evidence rather than on either side's
    # authority: `pyjson5` rejects `[0.e1]` while accepting `[1.e1]`, an asymmetry no production in
    # the grammar justifies, and ECMAScript - the language JSON5's numeric grammar is defined in -
    # evaluates `0.e1` to 0 (`node -e "eval('0.e1')"`). So the reference is the outlier here, the
    # parser is right, and the case is carried explicitly instead of silently weakening the rule.
    exempt = {"n_number_0.e1.json"}
    diverging = []
    checked = 0
    for path, accept in cases:
        if accept or path.name in exempt:
            continue
        text = _text(path)
        try:
            pyjson5.loads(text)
            continue  # the reference accepts it, so JSON5 latitude covers it
        except Exception:
            pass
        checked += 1
        try:
            pyrs_yaml.load_json5(text)
            diverging.append(f"{path.name}: {text.strip()[:40]!r} accepted by us, refused by pyjson5")
        except pyrs_yaml.YamlParseError:
            pass
    assert checked >= 120, f"only {checked} cases cross-checked against the reference"
    assert not diverging, f"JSON5 accepts {len(diverging)} document(s) the reference rejects:\n  " + "\n  ".join(
        diverging[:15]
    )


def test_jsonc_loosens_only_where_a_comment_is_present():
    """JSONC adds comments and nothing else, so a refusal change must involve one.

    Derived from the corpus rather than guessed: every invalid document JSONC accepts has to carry a
    `/`, because that is the only latitude the dialect claims over RFC 8259. An acceptance without one
    means the parser grew a rule nobody specified.
    """
    cases, _undecodable = _decodable_cases()
    widened = []
    for path, accept in cases:
        if accept:
            continue
        text = _text(path)
        try:
            pyrs_yaml.load_jsonc(text)
        except pyrs_yaml.YamlParseError:
            continue
        except Exception as exc:
            widened.append(f"{path.name}: untyped failure {type(exc).__name__}")
            continue
        if "/" not in text:
            widened.append(f"{path.name}: accepted with no comment marker: {text.strip()[:40]!r}")
    assert not widened, "JSONC widened past comments:\n  " + "\n  ".join(widened[:10])


def test_rejections_are_typed_parse_errors():
    """Once a document reaches the parser, refusing it must name itself as a parse error.

    The distinction is the whole point of a typed error: `n_structure_100000_opening_arrays.json`
    refused with a stack overflow, an unwrap panic or a `RecursionError` would satisfy acceptance
    logic and still be a crash on hostile input.

    Twelve cases are excluded, and they are a real boundary rather than an excuse: the corpus stores
    them as bytes that are not valid UTF-8, so they never reach the parser at all. The loader's
    surface takes text, so `UnicodeDecodeError` *is* the refusal, and counting it as a defect would
    just mean writing the expectation against a bytes API this binding does not have.
    """
    offenders = []
    undecodable = []
    checked = 0
    for path, should_accept in _cases():
        if should_accept:
            continue
        checked += 1
        try:
            _load(path)
        except pyrs_yaml.YamlParseError:
            continue
        except UnicodeDecodeError:
            undecodable.append(path.name)
        except Exception as exc:  # reporting is the purpose of this test
            offenders.append(f"{path.name}: {type(exc).__name__}: {str(exc)[:90]}")
    assert checked >= 180, f"only {checked} rejection cases ran"
    assert len(undecodable) == 12, (
        f"{len(undecodable)} non-UTF-8 cases, expected 12: the corpus or the decode boundary "
        "changed, re-read both before trusting this test"
    )
    assert not offenders, "untagged rejections (crash or wrong error type):\n" + "\n".join(offenders[:20])


# ── The write side, read back by a reader that is not this codebase ─────────
#
# Everything above is about which documents the parsers accept. The other direction was unpinned:
# the JSON-family writers had no cross-library gate, while the YAML side of the same corpus was
# covered by `test_property_dialects.py`. The measurement that motivated this exists already - all
# 95 valid documents re-emitted by `to_jsonc` read back through stdlib `json` as the same value, 0
# disagreements, which is how (cf) closed its wide-integer defect. Pinning it is the missing half.


def _valid_texts() -> list[tuple[Path, str]]:
    """`y_*` cases that can reach a text API, i.e. the corpus documents a writer is expected to emit."""
    out: list[tuple[Path, str]] = []
    for path, accept in _cases():
        if not accept:
            continue
        try:
            out.append((path, _text(path)))
        except UnicodeDecodeError:
            continue  # counted by the accept/reject tests; no text API can reach it
    return out


def _same_leaf(a: object, b: object) -> bool:
    """Equal, and equal *as the same JSON type*.

    `1 == 1.0` and `0 == False` in Python, so a plain `==` on decoded documents cannot see the exact
    defect class this repository has been bitten by twice: an integer written as a float, or a bool
    written as a number. Comparing `type()` alongside the value costs nothing and closes that hole;
    without it the gate below would pass on the #358 regression.
    """
    return type(a) is type(b) and a == b


def _same_value(a: object, b: object) -> bool:
    """Recursive `_same_leaf` over arrays and objects, with keys compared in document order."""
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(_same_value(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return list(a) == list(b) and all(_same_value(a[k], b[k]) for k in a)
    return _same_leaf(a, b)


def _pairs(text: str) -> list:
    """The document as stdlib sees it, with object key order and repetition intact."""
    import json

    return json.loads(text, object_pairs_hook=lambda p: [(k, v) for k, v in p])


def _has_repeated_name(node: object) -> bool:
    """Does any object in this pair-structure carry the same name twice?

    Derived from the document rather than listed: the exemption below has to follow the corpus, not a
    hand-written set of file names that quietly stops matching when the corpus is updated.
    """
    if isinstance(node, list) and node and isinstance(node[0], tuple):
        names = [key for key, _value in node]
        if len(set(names)) != len(names):
            return True
        return any(_has_repeated_name(value) for _key, value in node)
    if isinstance(node, list):
        return any(_has_repeated_name(item) for item in node)
    return False


def test_the_json_writers_are_read_by_the_reference_implementation():
    """`to_json` and `to_jsonc` of every valid document must re-read as the *same value and the same keys*.

    The oracle is stdlib `json`, which shares no code with this repository. Two comparisons run per
    document. The first is the contract a user cares about - whitespace, escaping and number spelling
    are the writer's business, the value is not - and it walks object keys in document order, so a
    member that moved is reported there. The second compares the `object_pairs_hook` pair lists for
    what dict equality genuinely cannot see: a writer that emits a member the input never repeated
    collapses to the same dict on both sides, and only the pair count tells. Both halves are measured
    rather than assumed - the control that reverses the writer's pair loop reports `value changed` for
    six documents and the `elif` below keeps those from being named twice, while the control that
    writes every member twice (still valid JSON, still equal as dicts, because the repeat collapses)
    reddens 18 of 190 and the value comparison never mentions it.

    Equality is type-strict: `1` re-emitted as `1.0` is a disagreement here even though Python calls
    them equal, because that substitution is the defect #358 fixed and a value-only comparison cannot
    see it. Same for a bool written as a number. The control that performs the substitution names 38
    of 190 re-reads with their input, output and emitted text, and every one of those pairs satisfies
    `==` (`[1] == [1.0]` and `0 == False` both return `True`), so the type check is the only thing in
    the gate that reddens. The JSON5 gate below fails on four cases for the same reason.

    What this gate cannot see, named because the sentence above is a limit and not a footnote: the
    corpus is comment-free and anchor-free, so no defect in the *comment* machinery reaches it - issue
    #359's dropped slots are pinned by the eleven-position table in `tests/test_json.py`, and only
    there. It also cannot reach the writer's number-*normalising* arm, and the control found that by
    failing first: every document here arrives through `from_json`, so its numbers already spell as
    JSON, and the fidelity pass returns them verbatim before the arm is consulted. Making the
    substitution observable took that pass switched off as well. The YAML spellings the arm exists for
    (`0x1F` as `31`, `!!str 7` as `"7"`) are Rust-side territory, pinned there by
    `exotic_scalars_quote_or_normalize` in `crates/pyrs-yaml-core/src/integration/json_family.rs`.
    A gate is worth what its fixture can show.
    """
    import json

    cases = _valid_texts()
    assert len(cases) >= 90, f"only {len(cases)} valid cases reached a writer"

    unparseable: list[str] = []
    refused: list[str] = []
    diverging: list[str] = []
    reordered: list[str] = []
    repeated_name: list[str] = []
    checked = 0
    for path, text in cases:
        try:
            expected = json.loads(text)
            expected_pairs = _pairs(text)
        except Exception as exc:
            unparseable.append(f"{path.name}: stdlib json refuses the valid case: {str(exc)[:60]}")
            continue
        # One exemption, and it is a property of the input rather than a concession: a document that
        # repeats an object member has no answer in terms of *pairs*, because RFC 8259 declares the
        # behaviour unpredictable and every implementation - stdlib `json` included, and this hub as
        # well, which is an `IndexMap` keyed by the node - resolves it to the last value. Checking
        # pair order there would assert a behaviour no JSON reader has. The value check above still
        # runs on these documents, and the count is pinned so the exemption cannot widen unnoticed:
        # it shrinks to zero the day the hub can hold two indistinguishable keys, and that is the
        # outcome worth having.
        skip_pairs = _has_repeated_name(expected_pairs)
        if skip_pairs:
            repeated_name.append(path.name)
        doc = pyrs_yaml.parse(pyrs_yaml.from_json(text))
        for name in ("to_json", "to_jsonc"):
            try:
                emitted = getattr(doc, name)()
            except pyrs_yaml.YamlSerializeError as exc:
                refused.append(f"{path.name} {name}(): {exc}")
                continue
            checked += 1
            try:
                got = json.loads(emitted)
                got_pairs = _pairs(emitted)
            except Exception as exc:
                diverging.append(f"{path.name} {name}(): re-read failed ({str(exc)[:50]}) in {emitted[:80]!r}")
                continue
            if not _same_value(expected, got):
                diverging.append(
                    f"{path.name} {name}(): value changed\n    in:  {expected!r}\n    out: {got!r}\n    text: {emitted[:120]!r}"
                )
            elif not skip_pairs and got_pairs != expected_pairs:
                reordered.append(
                    f"{path.name} {name}(): keys moved or collapsed\n    in:  {expected_pairs!r}\n    out: {got_pairs!r}"
                )
    assert checked >= 180, f"only {checked} writer re-reads ran; the matrix of cases x writers is short"
    assert sorted(repeated_name) == [
        "y_object_duplicated_key.json",
        "y_object_duplicated_key_and_value.json",
    ], f"the repeated-name exemption now covers {sorted(repeated_name)}; re-read the corpus"
    assert not unparseable, "the corpus marks these valid:\n  " + "\n  ".join(unparseable[:10])
    assert not refused, "the writers refused documents their reader accepted:\n  " + "\n  ".join(refused[:10])
    assert not diverging, f"{len(diverging)} of {checked} re-reads disagree with stdlib json:\n  " + "\n  ".join(
        diverging[:8]
    )
    assert not reordered, f"{len(reordered)} of {checked} re-reads changed key order or count:\n  " + "\n  ".join(
        reordered[:8]
    )


def test_the_json5_writer_is_read_by_the_reference_parser():
    """`to_json5` of every valid document must re-read as the same value under `pyjson5`.

    JSON5 is the dialect whose writer may legitimately change the *spelling* - a non-finite float
    becomes the bare `Infinity` / `NaN` token that strict JSON has no literal for - so stdlib `json`
    cannot be the oracle here and the reference JSON5 parser can. Skipped when that package is not
    installed, which is the marker's business (`python_version < '3.14'`), not this test's.
    """
    pyjson5 = pytest.importorskip("pyjson5", reason="reference JSON5 parser not installed")
    cases = _valid_texts()
    assert len(cases) >= 90, f"only {len(cases)} valid cases reached a writer"

    diverging: list[str] = []
    checked = 0
    for path, text in cases:
        try:
            expected = pyjson5.loads(text)
        except Exception:
            continue  # the strict reading is already the subject of the test above
        emitted = pyrs_yaml.parse(pyrs_yaml.from_json(text)).to_json5()
        checked += 1
        try:
            got = pyjson5.loads(emitted)
        except Exception as exc:
            diverging.append(f"{path.name}: re-read failed ({str(exc)[:60]}) in {emitted[:80]!r}")
            continue
        if got != expected:
            diverging.append(
                f"{path.name}: value changed\n    in:  {expected!r}\n    out: {got!r}\n    text: {emitted[:120]!r}"
            )
    assert checked >= 90, f"only {checked} JSON5 re-reads ran"
    assert not diverging, f"{len(diverging)} of {checked} re-reads disagree with pyjson5:\n  " + "\n  ".join(
        diverging[:8]
    )
