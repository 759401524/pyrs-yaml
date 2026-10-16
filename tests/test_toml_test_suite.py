"""toml-test conformance runner (official language-agnostic TOML corpus).

Handled the same way as ``test_yaml_suite.py``: the corpus is an untracked local
dev artifact (``Reference/toml-test``, excluded via ``.git/info/exclude``), so
every test ``skipif`` the directory is absent.

**Both directions run, and they are different claims.** The decode half takes
each ``valid/<dir>/<name>.toml`` with its ``<name>.json`` (values described with
TOML type tags - ``{"type": "integer", "value": "42"}``, ``datetime``,
``date-local``, ``time-local``) and checks ``load_toml`` against it, plus every
``invalid/<...>.toml`` must be rejected. The encode half drives the writer from
the same 268 JSON files and judges the emitted text with **``tomllib``, CPython's
reference decoder** - never our own reader, because comparing a writer to the
parser beside it only proves two halves of one implementation agree, which is the
thing a conformance suite exists to refuse.

The encode gate holds at 100% and is asserted as such, which it only can because
the two writer defects it found are fixed: a raw DEL (U+007F) inside a basic
string, and a run of newlines written literally into a single-line string - both
forbidden by TOML and both rejected by this crate's own parser, so ``to_toml``
was emitting text no one could read back.

The decode gates hold at the *measured* compliance of the current parser. Measured 2026-10-16 on the
corpus CI clones: **all 266 valid documents decode to the tagged values TOML says**, so that gate
asserts an empty failure list rather than a percentage — it is the encoder gate's shape, earned here
by closing three defect classes, and any new gap reddens it by naming the file. The reject half is
interpreter-dependent and the floor describes the worst leg: **491 of 511 invalid documents are refused
on CPython 3.8-3.13 (`96.09%`) but 488 of 511 on 3.14 and 3.15 (`95.50%`)**, because three out-of-range
times (`24:00:00`, in `datetime`, `local-datetime` and `local-time`) are rejected by the host's
`datetime` constructor rather than by this parser, and a newer interpreter accepts the spelling — so
that floor is 0.954, the 3.14/3.15 reading with one case of slack. The engine owning the temporal range
check is the fix for that split, not a wider line.

The `0.85` these two gates used to carry was 22 cases of slack on the decode half and 57 on the reject
half, which is how whole defect classes hid behind a green percentage. Three of them are closed: nine
cases read `a = { x.y = 1 }` as one key whose text is `x.y` instead of the nested table the spec
defines; five lost the quote run or the line break at the end of a multi-line string, because the
scanner closed the string at the FIRST `'''` match and threw the rest of the run away
(`''''one quote''''` came back without its tail) and an escaped backslash ending a line read as a line
continuation; and four refused valid documents because the header bookkeeping keyed a table by its
dot-joined text, which made `[a."b.c"]` a duplicate of `[a.b.c]` and gave every element of `[[a.b]]`
the same identity as the first. What the corpus still accepts that it should refuse is the `invalid/`
list the assertion prints: sixteen files are the TOML 1.1 relaxations the default dialect takes on
purpose (optional seconds, multi-line keys, line breaks and trailing commas inside inline tables), and
four are strictness the engine does not yet police — a backslash-x byte escape in a basic string, and a
run of more than two quotes before a multi-line delimiter.
"""

import json
import math
from datetime import date, datetime, time
from pathlib import Path

import pytest

import pyrs_yaml

SUITE_DIR = Path("Reference/toml-test/tests")

pytestmark = pytest.mark.skipif(
    not SUITE_DIR.exists(),
    reason="toml-test corpus not found (clone https://github.com/toml-lang/toml-test into Reference/)",
)


def _iter_valid():
    for toml in sorted((SUITE_DIR / "valid").rglob("*.toml")):
        js = toml.with_suffix(".json")
        if js.exists():
            yield toml, js


def _iter_invalid():
    yield from sorted((SUITE_DIR / "invalid").rglob("*.toml"))


def _decode_expected(node):
    """toml-test expected JSON -> canonical comparable form."""
    if isinstance(node, dict):
        if set(node) == {"type", "value"}:
            return (node["type"], node["value"])
        return {k: _decode_expected(v) for k, v in node.items()}
    if isinstance(node, list):
        return [_decode_expected(v) for v in node]
    return node  # plain str / bool


def _encode_actual(val):
    """pyrs load_toml output -> the same tagged canonical form."""
    if isinstance(val, bool):
        return ("bool", "true" if val else "false")
    if isinstance(val, int):
        return ("integer", str(val))
    if isinstance(val, float):
        return ("float", val)
    if isinstance(val, datetime):
        kind = "datetime" if val.tzinfo is not None else "datetime-local"
        return (kind, val.isoformat())
    if isinstance(val, date):
        return ("date-local", val.isoformat())
    if isinstance(val, time):
        return ("time-local", val.isoformat())
    if isinstance(val, str):
        return ("string", val)
    if isinstance(val, list):
        return [_encode_actual(v) for v in val]
    if isinstance(val, dict):
        return {k: _encode_actual(v) for k, v in val.items()}
    return val


def _canon_temporal(v):
    """Normalize RFC 3339 spelling: 'Z'/'z' -> '+00:00', strip trailing micro 0s."""
    s = str(v).replace("Z", "+00:00").replace("z", "+00:00")
    if "." in s:
        head, _, rest = s.partition(".")
        frac, tz = rest, ""
        for i, ch in enumerate(rest):
            if ch in "+-":
                frac, tz = rest[:i], rest[i:]
                break
        frac = frac.rstrip("0")
        s = head + ("." + frac if frac else "") + tz
    return s


def _num_eq(expected, actual) -> bool:
    """Compare two numeric spellings, treating NaN as matching itself.

    IEEE says `nan != nan`, but a conformance comparison asks whether the *value survived*, and
    `nan = nan` is exactly what TOML requires of a writer. Without this the encoder gate fails four
    corpus cases that the reference decoder reads back identically - a comparator artifact, not a
    product defect, and worse to hide inside a percentage than to write down.
    """
    try:
        left = float(expected)
        right = float(actual)
    except (TypeError, ValueError):
        return False
    if math.isnan(left) or math.isnan(right):
        return math.isnan(left) and math.isnan(right)
    return left == right


def _matches(expected, actual):
    if isinstance(expected, tuple) and isinstance(actual, tuple):
        te, ve = expected
        ta, va = actual
        if te in ("integer", "float"):
            return _num_eq(ve, va)
        if te in ("datetime", "datetime-local", "date-local", "time-local"):
            return te == ta and _canon_temporal(ve) == _canon_temporal(va)
        return te == ta and ve == va
    if isinstance(expected, float) or isinstance(actual, float):
        return _num_eq(expected, actual)
    if isinstance(expected, dict) and isinstance(actual, dict):
        return set(expected) == set(actual) and all(_matches(expected[k], actual[k]) for k in expected)
    if isinstance(expected, list) and isinstance(actual, list):
        return len(expected) == len(actual) and all(_matches(a, b) for a, b in zip(expected, actual))
    return expected == actual


def test_toml_test_valid_decodes_match():
    """Every valid TOML document decodes to the expected tagged values (266 of 266)."""
    total = 0
    matched = 0
    failures: list[str] = []
    for toml, js in _iter_valid():
        total += 1
        name = toml.relative_to(SUITE_DIR).as_posix()
        try:
            expected = _decode_expected(json.loads(js.read_text(encoding="utf-8")))
            actual = _encode_actual(pyrs_yaml.load_toml(toml.read_bytes().decode("utf-8")))
        except Exception as exc:
            failures.append(f"{name}: {type(exc).__name__}: {str(exc)[:70]}")
            continue
        if _matches(expected, actual):
            matched += 1
        else:
            failures.append(f"{name}: decoded differs")
    assert total > 0, "no valid toml-test cases found"
    rate = matched / total
    assert not failures, f"valid decode failures ({len(failures)}/{total}):\n" + "\n".join(failures[:10])
    assert rate == 1.0, f"valid decode-match rate {rate:.1%} ({matched}/{total}) below floor"


def test_toml_test_invalid_rejected():
    """Invalid TOML is rejected rather than silently accepted (488/511 on the newest interpreters).

    A document is "rejected" when loading raises - either a typed
    ``YamlParseError`` or any conversion-time error (a bare ``ValueError`` from
    the temporal plugin is still a rejection, not a silent accept).
    """
    total = 0
    rejected = 0
    accepted: list[str] = []
    for toml in _iter_invalid():
        total += 1
        try:
            pyrs_yaml.load_toml(toml.read_bytes().decode("utf-8"))
        except Exception:
            rejected += 1
        else:
            accepted.append(toml.relative_to(SUITE_DIR).as_posix())
    assert total > 0, "no invalid toml-test cases found"
    rate = rejected / total
    assert rate >= 0.954, (
        f"invalid rejection rate {rate:.1%} ({rejected}/{total}) below floor 95.4% "
        f"(488/511 on CPython 3.14+, 491/511 through 3.13): silently accepted {accepted}"
    )


def _tagged_to_python(node):
    """toml-test expected JSON -> native Python, for driving the *writer* instead of the reader.

    The encoder half of toml-test feeds the implementation the decoded data and judges the TOML it
    writes by having a reference parser read it back. Same corpus, opposite direction: nothing here
    consults our own parser, so a shared bug cannot hide behind a shared reader.
    """
    if isinstance(node, dict):
        if set(node) == {"type", "value"}:
            kind, value = node["type"], node["value"]
            if kind == "integer":
                return int(value)
            if kind == "float":
                return float(value)
            if kind == "bool":
                return value == "true"
            if kind == "string":
                return value
            if kind == "datetime":
                return datetime.fromisoformat(value.replace("Z", "+00:00").replace("z", "+00:00"))
            if kind == "datetime-local":
                return datetime.fromisoformat(value)
            if kind == "date-local":
                return date.fromisoformat(value)
            if kind == "time-local":
                return time.fromisoformat(value)
            raise AssertionError(f"unknown toml-test type {kind!r}")
        return {key: _tagged_to_python(value) for key, value in node.items()}
    if isinstance(node, list):
        return [_tagged_to_python(item) for item in node]
    return node


def _iter_json_inputs():
    """Every `valid/*.json` - 268 of them, the encoder corpus."""
    yield from sorted((SUITE_DIR / "valid").rglob("*.json"))


def test_toml_test_encodes_and_reference_decodes():
    """The writer must round-trip the official corpus through someone else's parser.

    Every `valid` case is fed to `to_toml` as data, and the emitted text is decoded by `tomllib` -
    the CPython reference - not by `load_toml`. Comparing against our own reader would prove only
    that two halves of one implementation agree with each other, which is exactly the kind of claim
    a conformance suite exists to refuse.
    """
    tomllib = pytest.importorskip("tomllib", reason="reference TOML decoder needs Python 3.11+")
    total = 0
    matched = 0
    failures: list[str] = []
    for js in _iter_json_inputs():
        total += 1
        name = js.relative_to(SUITE_DIR).as_posix()
        try:
            data = _tagged_to_python(json.loads(js.read_text(encoding="utf-8")))
            text = pyrs_yaml.to_toml(pyrs_yaml.safe_dump(data))
            back = tomllib.loads(text)
        except Exception as exc:  # a writer that cannot express it, or a reference that rejects it
            failures.append(f"{name}: {type(exc).__name__}: {str(exc)[:80]}")
            continue
        if _matches(_decode_expected(json.loads(js.read_text(encoding="utf-8"))), _encode_actual(back)):
            matched += 1
        else:
            failures.append(f"{name}: decoded differs")
    assert total > 0, "no toml-test encoder cases found"
    rate = matched / total
    assert not failures, f"encoder failures ({len(failures)}/{total}):\n" + "\n".join(failures[:10])
    assert rate == 1.0, f"encoder match rate {rate:.1%} ({matched}/{total}) below floor"


def test_toml_test_datetime_never_crashes():
    """Regression: temporal values decode via typed plugins, never a leaked error.

    toml-test surfaced that local-time (``07:32:00``) and lowercase-delimiter
    offset datetimes (``1987-07-05t17:45:00z``) reached ``datetime.fromisoformat``
    and raised a raw ``ValueError`` - a crash on *valid* TOML. A clean
    ``YamlParseError`` on a genuinely unsupported shape (e.g. TOML 1.1
    super-table re-opening) is a typed limitation, not a crash, so it is allowed.
    """
    for toml, _js in _iter_valid():
        try:
            pyrs_yaml.load_toml(toml.read_bytes().decode("utf-8"))
        except pyrs_yaml.YamlParseError:
            continue  # typed parse error on an unsupported-but-finite grammar
        except ValueError as exc:  # the plugin-crash class we pinned
            pytest.fail(f"valid TOML {toml.relative_to(SUITE_DIR)} raised {type(exc).__name__}: {exc}")
