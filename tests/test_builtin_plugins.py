"""Test built-in plugins (enriched set)."""

import uuid
from datetime import date, datetime, time
from decimal import Decimal

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
    assert "!timestamp" in out, out
    assert "!date" in out, out
    assert "!time" in out, out
    assert "!uuid" in out, out
    assert "!decimal" in out, out
    doc = pyrs_yaml.parse(out)
    assert isinstance(doc.get("ts"), datetime)
    assert isinstance(doc.get("d"), date)
    assert isinstance(doc.get("t"), time)
    assert isinstance(doc.get("u"), uuid.UUID)
    assert isinstance(doc.get("dec"), Decimal)


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
