"""Property-based tests (Hypothesis) for round-trip fidelity.

These fuzz the dump/parse pipeline with randomly generated JSON-compatible
structures to catch (a) panics/crashes on arbitrary input and (b) structural
or scalar drift introduced by refactors.

The round-trip invariant now holds end-to-end: quoted scalars load as strings
(YAML 1.2), lone-quote keys and empty collections round-trip exactly, and only
plain scalars are schema-resolved. The only strings still excluded from the
*exact-equality* strategy are those the YAML plain-scalar grammar cannot
represent losslessly (leading/trailing whitespace, control characters) — a
serializer syntax concern, not a type-resolution one. ``test_dump_never_raises``
fuzzes *arbitrary* input to assert the dump step itself never panics.

Strategies are shared via ``tests/strategies.py``.
"""

import pytest
from hypothesis import HealthCheck, given, settings

import pyrs_yaml
from tests.strategies import (
    arbitrary_json,
    roundtrip_safe_json,
    roundtrip_safe_leaf,
    rt,
)

pytestmark = pytest.mark.slow


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_leaf)
def test_safe_scalar_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_json)
def test_safe_structure_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_json)
def test_roundtrip_idempotent(value):
    once = rt(value)
    twice = rt(once)
    assert twice == once


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(arbitrary_json)
def test_dump_never_raises(value):
    # The dump step must not panic or raise on any JSON-compatible structure,
    # even inputs whose output is not re-parseable (a separate load-side issue).
    out = pyrs_yaml.safe_dump(value)
    assert isinstance(out, str)
