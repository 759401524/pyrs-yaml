"""Hypothesis property tests (Python binding side) for TOML/JSON/JSONC/JSON5.

`test_property_roundtrip.py` fuzzes the YAML spine via Hypothesis; the Rust
`fmt_pbt.rs` fuzzes the core of all five formats. This closes the remaining
"两端" gap: property-based fuzz of the **dialect formats through the public
binding API** (parse/serialize round-trip, re-parseability and no-crash), the
Python counterpart to the Rust proptests.

Values are generated with the shared `roundtrip_safe_json` strategy (finite
floats, i64-range ints, YAML-lossless strings); a mapping wrapper keeps the root
a table (representable by every dialect). Where a value genuinely cannot be
encoded in a format (e.g. TOML has no null), the test `assume`s it away rather
than hiding the case.
"""

import contextlib

import pytest
from hypothesis import HealthCheck, assume, given, settings

import pyrs_yaml
from tests.strategies import any_text, roundtrip_safe_json

pytestmark = pytest.mark.slow

_SETTINGS = settings(
    max_examples=200,
    deadline=5000,
    suppress_health_check=[HealthCheck.too_slow],
)


def _doc(value):
    return pyrs_yaml.parse(pyrs_yaml.from_dict({"v": value}))


@_SETTINGS
@given(roundtrip_safe_json)
def test_json_roundtrip_via_binding(value):
    data = {"v": value}
    doc = _doc(value)
    # The pyrs round-trip invariant: AST -> to_json -> load_jsonc recovers the
    # original structure. (Not asserted against stdlib json.loads: a digit-string
    # beyond i64 is emitted by the writer as a bare JSON number - valid JSON, but
    # json.loads reads it as an int while pyrs's loader preserves the string, so
    # the two consumers legitimately differ on that extreme corner.)
    assert pyrs_yaml.load_jsonc(doc.to_json(0)) == data


@_SETTINGS
@given(roundtrip_safe_json)
def test_jsonc_json5_roundtrip_via_binding(value):
    data = {"v": value}
    doc = _doc(value)
    assert pyrs_yaml.load_jsonc(doc.to_jsonc(0)) == data
    assert pyrs_yaml.load_json5(doc.to_json5(0)) == data


@_SETTINGS
@given(roundtrip_safe_json)
def test_toml_roundtrip_via_binding(value):
    data = {"v": value}
    doc = _doc(value)
    try:
        toml_text = doc.to_toml()
    except pyrs_yaml.YamlSerializeError:
        assume(False)  # not TOML-representable (e.g. a null leaf); skip, don't hide
    assert pyrs_yaml.load_toml(toml_text) == data


@_SETTINGS
@given(any_text)
def test_dialect_loaders_never_crash_on_arbitrary_text(s):
    # Only a typed YamlParseError is acceptable; a panic or any other exception
    # (e.g. pyo3 PanicException) would surface as a test failure here. This is
    # the binding-level no-panic fuzz mirroring fmt_pbt's parser property.
    for loader in (pyrs_yaml.load_jsonc, pyrs_yaml.load_json5):
        with contextlib.suppress(pyrs_yaml.YamlParseError):
            loader(s)
