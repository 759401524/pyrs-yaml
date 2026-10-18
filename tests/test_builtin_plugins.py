"""Test built-in plugins (enriched set)."""

import uuid
from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal

import pytest

import pyrs_yaml


def test_builtin_plugins_load():
    """All enriched built-in plugins parse tagged scalars."""
    doc = pyrs_yaml.parse(
        "ts: !timestamp 2026-08-11T10:30:00\n"
        "d: !date 2026-08-11\n"
        "t: !time 10:30:00\n"
        "u: !uuid 550e8400-e29b-41d4-a716-446655440000\n"
        "dec: !decimal 3.14159\n"
        "b: !binary aGVsbG8=\n"
        "r: !regex ^0x[0-9a-fA-F]+$\n"
    )
    assert isinstance(doc.get("ts"), datetime)
    assert isinstance(doc.get("d"), date)
    assert isinstance(doc.get("t"), time)
    assert isinstance(doc.get("u"), uuid.UUID)
    assert isinstance(doc.get("dec"), Decimal)
    assert doc.get("b") == b"hello"
    assert doc.get("r").pattern == "^0x[0-9a-fA-F]+$"


def test_builtin_plugins_dump_roundtrip():
    """All enriched built-in plugins serialize and round-trip."""
    data = {
        "ts": datetime(2026, 8, 11, 10, 30),
        "d": date(2026, 8, 11),
        "t": time(10, 30),
        "u": uuid.UUID("550e8400-e29b-41d4-a716-446655440000"),
        "dec": Decimal("3.14159"),
    }
    out = pyrs_yaml.safe_dump(data)
    # The tag text is the contract with every other reader, so it is asserted as spelled rather than
    # as a substring: `!!timestamp` for both the date and the datetime, and `!time` stays local
    # because YAML defines no tag for a bare time of day.
    assert "!!timestamp 2026-08-11T10:30:00" in out, out
    assert "!!timestamp 2026-08-11" in out, out
    assert "!time 10:30:00" in out, out
    assert "!uuid 550e8400-e29b-41d4-a716-446655440000" in out, out
    assert "!decimal 3.14159" in out, out
    assert "!date" not in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("ts"), datetime)
    # A date read back as a midnight datetime would still satisfy `isinstance(..., date)`, so the
    # date branch is pinned by what it is not.
    assert isinstance(doc.get("d"), date) and not isinstance(doc.get("d"), datetime)
    assert isinstance(doc.get("t"), time)
    assert isinstance(doc.get("u"), uuid.UUID)
    assert isinstance(doc.get("dec"), Decimal)


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("!!timestamp 2026-08-11", date(2026, 8, 11)),
        ("!!timestamp 2026-08-11T10:30:00", datetime(2026, 8, 11, 10, 30)),
        ("!!timestamp 2026-08-11 10:30:00", datetime(2026, 8, 11, 10, 30)),
        ("!!timestamp 2026-08-11t10:30:00z", datetime(2026, 8, 11, 10, 30, tzinfo=timezone.utc)),
        (
            "!!timestamp 2026-08-11T10:30:00+08:00",
            datetime(2026, 8, 11, 10, 30, tzinfo=timezone(timedelta(hours=8))),
        ),
        ("!!timestamp 2026-08-11T10:30:00.123456789", datetime(2026, 8, 11, 10, 30, 0, 123456)),
    ],
)
def test_the_standard_tag_resolves_the_way_the_standard_readers_do(text, expected):
    # PyYAML and ruamel.yaml both hand back a `date` for a date-only text and a `datetime` for
    # anything with a time part, so this reader has to make the same fork or one document means two
    # things depending on who reads it.
    got = pyrs_yaml.safe_load(f"v: {text}\n")["v"]
    assert got == expected, f"{text} resolved to {got!r}"
    assert isinstance(got, type(expected)), f"{text} resolved to a {type(got).__name__}"


def test_binary_plugin_roundtrip():
    """!binary encodes/decodes bytes via base64."""
    b = b"hello world"
    out = pyrs_yaml.safe_dump({"b": b})
    assert "!binary" in out, out
    doc = pyrs_yaml.parse(out)
    assert doc.get("b") == b"hello world"


def test_regex_plugin_roundtrip():
    """!regex compiles/decompiles regex patterns."""
    import re

    pattern = re.compile("^0x[0-9a-fA-F]+$")
    out = pyrs_yaml.safe_dump({"r": pattern})
    assert "!regex" in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("r"), re.Pattern)
    assert doc.get("r").pattern == pattern.pattern


def test_pendulum_duration_roundtrip():
    """!duration serializes/deserializes pendulum.Duration."""
    pendulum = pytest.importorskip("pendulum")
    d = pendulum.duration(days=1, hours=2, seconds=3)
    out = pyrs_yaml.safe_dump({"d": d})
    assert "!duration" in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("d"), pendulum.Duration)
    assert doc.get("d").total_seconds() == d.total_seconds()


def test_timedelta_not_matched_by_duration():
    """stdlib timedelta is never matched by !duration."""
    pytest.importorskip("pendulum")
    from datetime import timedelta

    with pytest.raises(pyrs_yaml.YamlTypeError):
        pyrs_yaml.safe_dump({"td": timedelta(days=1)})


def test_arrow_roundtrip():
    """!arrow serializes/deserializes arrow.Arrow."""
    arrow = pytest.importorskip("arrow")
    a = arrow.get("2026-08-19T10:30:00+00:00")
    out = pyrs_yaml.safe_dump({"a": a})
    assert "!arrow" in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("a"), arrow.Arrow)
    assert doc.get("a") == a


def test_ulid_roundtrip():
    """!ulid serializes/deserializes ulid.ULID."""
    ulid_mod = pytest.importorskip("ulid")
    u = ulid_mod.ULID()
    out = pyrs_yaml.safe_dump({"u": u})
    assert "!ulid" in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("u"), ulid_mod.ULID)
    assert doc.get("u") == u


def test_timestamp_still_parses_to_datetime():
    """!timestamp still returns datetime when third-party libs are present."""
    pytest.importorskip("arrow")
    pytest.importorskip("pendulum")
    doc = pyrs_yaml.parse("ts: !timestamp 2026-08-11T10:30:00")
    assert isinstance(doc.get("ts"), datetime)


def test_timestamp_utc_z_suffix_on_all_supported_pythons():
    # datetime.fromisoformat only gained `Z` support in 3.11; the plugin
    # normalizes it so UTC-Z timestamps work from 3.8 up (TOML datetimes
    # route through this same path).
    d = pyrs_yaml.safe_load("t: !timestamp 2026-01-02T03:04:05Z\n")
    assert d["t"] == datetime(2026, 1, 2, 3, 4, 5, tzinfo=timezone.utc)


def test_third_party_plugins_listed():
    """list_plugins() reports third-party tags when libraries are importable."""
    expected = set()
    for mod_name in ("pendulum", "arrow", "ulid"):
        try:
            __import__(mod_name)
        except ImportError:
            continue
        expected.add({"pendulum": "!duration", "arrow": "!arrow", "ulid": "!ulid"}[mod_name])
    if not expected:
        pytest.skip("no third-party libraries installed")
    tags = {tag for tag, _ in pyrs_yaml.list_plugins()}
    assert expected <= tags, f"missing tags {expected - tags} in {tags}"


def test_third_party_plugins_skip_when_absent(monkeypatch):
    """_register_third_party() is a no-op when libraries are not importable."""
    from pyrs_yaml.plugins import _builtin

    def fake_import(name, *args, **kwargs):
        if name in ("pendulum", "arrow", "ulid"):
            raise ImportError(f"No module named {name!r}")
        return __import__(name, *args, **kwargs)

    monkeypatch.setattr("builtins.__import__", fake_import)
    _builtin._register_third_party()
