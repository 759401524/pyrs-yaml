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
