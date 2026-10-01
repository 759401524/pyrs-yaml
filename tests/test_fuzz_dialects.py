"""CI-gating no-panic fuzz for every format's loader / converter.

The objective (item 3) asks for a no-panic property test across all five
formats so a future #163 / #165 / #166-class structural defect is caught
automatically in CI, not just by the high-volume standalone
`scripts/fuzz_panics.py` (which is run manually and therefore cannot gate a
merge). This module runs a *bounded* Hypothesis sweep on every push.

Inputs come from `tests.strategies.dialect_text`: real grammar fragments
(comments, trailing commas, unquoted keys, hex / `.5` / `5.` / Infinity /
NaN spellings, leading zeros, underscores, invalid UTF-8, unterminated
strings) concatenated at random, so the fuzzers actually reach the dialect
parsers' interesting paths instead of tripping the first-token "expected a
value" exit.

Contract: for hostile text, every loader/converter must EITHER return a
value OR raise a typed parse/serialize error -- it must never surface
anything else (a `pyo3_runtime.PanicException`, an `AttributeError`, an
`IndexError`, etc. all mean the Rust core leaked an unexpected failure).
The converters additionally must emit YAML that re-parses without panicking
(the Python-side mirror of `pbt.rs`'s `prop_output_always_parses`, now
extended across the TOML / JSON dialect spokes).
"""

from __future__ import annotations

import contextlib

import pytest
from hypothesis import HealthCheck, given, settings

import pyrs_yaml
from tests.strategies import dialect_text

# Errors that represent a legitimate rejection of hostile input. Anything
# NOT in this tuple escaping the call is a bug: the core panicked or raised a
# Python-level exception it should never produce.
_ACCEPTED = (
    pyrs_yaml.YamlParseError,
    pyrs_yaml.YamlSerializeError,
    pyrs_yaml.YamlTypeError,
    pyrs_yaml.YamlMaxDepthError,
    ValueError,
)

# Every public text-consuming entry point across the five formats.
_LOADERS = [
    "load_json",
    "load_jsonc",
    "load_json5",
    "load_toml",
    "from_json",
    "from_jsonc",
    "from_json5",
    "from_toml",
    "safe_load",
]

_MAX = dict(max_examples=250, deadline=None, suppress_health_check=[HealthCheck.too_slow])


@pytest.mark.parametrize("fn_name", _LOADERS)
@settings(**_MAX)
@given(dialect_text)
def test_loader_never_panics(fn_name, text):
    """Hostile dialect text must yield a value or a typed error -- never a panic."""
    fn = getattr(pyrs_yaml, fn_name)
    # A typed parse/serialize error is a legitimate rejection; any other
    # exception (PanicException, AttributeError, IndexError, ...) propagates
    # and Hypothesis reports it as a shrunk failing input.
    with contextlib.suppress(_ACCEPTED):
        fn(text)


@pytest.mark.parametrize("fn_name", ["from_json", "from_jsonc", "from_json5", "from_toml"])
@settings(**_MAX)
@given(dialect_text)
def test_converter_output_reparses(fn_name, text):
    """A converter that succeeds emits YAML the parser re-accepts without panic."""
    fn = getattr(pyrs_yaml, fn_name)
    try:
        yaml_text = fn(text)
    except _ACCEPTED:
        return  # rejection is a valid outcome for hostile input
    assert isinstance(yaml_text, str)
    try:
        pyrs_yaml.parse(yaml_text).to_dict()
    except _ACCEPTED:
        # The produced YAML must at least never panic the parser. A typed
        # error here would still be a defect, but parse of our own emitted
        # YAML should not raise -- surface it below if it does.
        pytest.fail(f"{fn_name} emitted non-reparseable YAML: {yaml_text!r}")
