"""End-to-end integration tests: Schema Language + Community Plugins (Spiral 4)."""

from datetime import datetime

import pyrs_yaml
from tests.test_community_plugins import TimestampType

HEX_SCHEMA = """\
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
"""


class TestSchemaPluginIntegration:
    """Schema language and community plugins used together."""

    def setup_method(self):
        pyrs_yaml.clear_type_handlers()
        pyrs_yaml.register_type("!ts", TimestampType())

    def test_schema_and_plugin_same_doc(self):
        """Custom schema resolves plain scalars; plugin handles tagged scalar."""
        pyrs_yaml.register_schema("int_hex", HEX_SCHEMA)
        doc = pyrs_yaml.parse(
            "addr: 0xFF\nwhen: !ts 2026-08-11T10:30:00\nname: hello\n",
            schema="int_hex",
        )
        assert doc.get("addr") == 255
        assert isinstance(doc.get("when"), datetime)
        assert doc.get("name") == "hello"

    def test_inline_schema_and_plugin(self):
        """Inline dict schema + plugin in the same YAML instance."""
        y = pyrs_yaml.YAML(
            schema={
                "extends": "core",
                "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
            }
        )
        doc = y.parse("addr: 0x1F\nwhen: !ts 2026-08-11T10:30:00\n")
        assert doc.get("addr") == 31
        assert isinstance(doc.get("when"), datetime)

    def test_multi_doc_schema_dict(self):
        """parse_all_docs with inline schema dict."""
        docs = pyrs_yaml.parse_all_docs(
            "---\na: 0xFF\n---\nb: 0x10\n",
            schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]},
        )
        assert len(docs) == 2
        assert docs[0].get("a") == 255
        assert docs[1].get("b") == 16

    def test_plugin_serialization_round_trip(self):
        """safe_dump then safe_load round-trips a custom type."""
        out = pyrs_yaml.safe_dump({"ts": datetime(2026, 8, 11, 10, 30)})
        assert "!ts" in out and "2026-08-11T10:30:00" in out
        # Manual re-parse validates the tag round-trips
        doc = pyrs_yaml.parse(out)
        assert isinstance(doc.get("ts"), datetime)

    def test_plugin_and_plain_dict_round_trip(self):
        """Dump a mix of plain values and custom types, then reload."""
        data = {"name": "x", "count": 3, "ts": datetime(2026, 8, 11, 10, 30)}
        out = pyrs_yaml.safe_dump(data)
        assert "name: x" in out
        assert "count: 3" in out
        assert "!ts" in out
        doc = pyrs_yaml.parse(out)
        assert doc.get("name") == "x"
        assert doc.get("count") == 3
        assert isinstance(doc.get("ts"), datetime)
