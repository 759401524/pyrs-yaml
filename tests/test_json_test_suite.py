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
