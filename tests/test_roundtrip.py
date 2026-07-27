"""
Round-trip preservation tests — parse → serialize → parse identity.
"""

import pyyaml_rs


class TestRoundTrip:
    """Test perfect round-trip preservation"""

    def test_roundtrip_simple(self):
        original = "key: value\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_with_comment(self):
        original = "# Comment\nkey: value\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_inline_comment(self):
        original = "key: value  # comment\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_anchor(self):
        original = "defaults: &defaults\n  timeout: 30\n"
        doc = pyyaml_rs.parse(original)
        assert "&defaults" in doc.to_yaml()

    def test_roundtrip_tag(self):
        original = "name: !!str John\n"
        doc = pyyaml_rs.parse(original)
        assert "!!str" in doc.to_yaml()

    def test_roundtrip_chomping(self):
        original = "key: |-\n  line1\n  line2\n"
        doc = pyyaml_rs.parse(original)
        assert "|-" in doc.to_yaml()

    def test_roundtrip_nested_mapping(self):
        original = "parent:\n  child1: value1\n  child2: value2\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_sequence(self):
        original = "- item1\n- item2\n- item3\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_mixed(self):
        original = "list:\n  - a\n  - b\nmapping:\n  key: value\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_chomping_variants(self):
        for indicator in ["|-", "|", "|+", ">-", ">", ">+"]:
            if indicator.startswith("|"):
                original = f"key: {indicator}\n  line1\n  line2\n"
            else:
                original = f"key: {indicator}\n  line1\n  line2\n"
            doc = pyyaml_rs.parse(original)
            assert indicator in doc.to_yaml(), f"Chomping indicator {indicator} lost in round-trip"

    def test_roundtrip_multiple_anchors(self):
        original = "a: &anchor1 val1\nb: &anchor2 val2\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "&anchor1" in output
        assert "&anchor2" in output

    def test_roundtrip_empty_values(self):
        original = "key1:\nkey2: value\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "key1:" in output
        assert "key2: value" in output

    def test_roundtrip_alias_reference(self):
        original = "defaults: &defaults\n  timeout: 30\nproduction:\n  <<: *defaults\n  host: x\n"
        doc = pyyaml_rs.parse(original, resolve_merges=False)
        output = doc.to_yaml()
        assert "*defaults" in output

    def test_roundtrip_scalar_styles(self):
        for style_yaml, expected_marker in [
            ("key: plain_value\n", "plain_value"),
            ("key: 'single quoted'\n", "'single quoted'"),
            ('key: "double quoted"\n', '"double quoted"'),
        ]:
            doc = pyyaml_rs.parse(style_yaml)
            assert expected_marker in doc.to_yaml(), f"Scalar style marker {expected_marker} lost"

    def test_roundtrip_local_tag(self):
        original = "key: !custom value\n"
        doc = pyyaml_rs.parse(original)
        assert "!custom" in doc.to_yaml()

    def test_roundtrip_complex_key(self):
        original = "? [key1, key2]\n: value\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "?" in output or "key1" in output

    def test_roundtrip_empty_mapping(self):
        original = "{}\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_empty_sequence(self):
        original = "[]\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_flow_mapping(self):
        original = "{a: 1, b: 2}\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_flow_sequence(self):
        original = "[1, 2, 3]\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_flow_nested(self):
        original = "{a: [1, 2], b: {c: 3}}\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_flow_mixed_with_block(self):
        """Flow mapping as a value inside block mapping"""
        original = "key: {a: 1, b: 2}\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_merge_key_unresolved(self):
        """Merge keys (<<) preserved when resolve_merges=False"""
        original = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2\n"
        doc = pyyaml_rs.parse(original, resolve_merges=False)
        assert "<<" in doc.to_yaml()

    def test_roundtrip_resolve_merges_default(self):
        """Merge keys resolved by default in round-trip"""
        doc = pyyaml_rs.parse("base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2\n")
        child = doc.get("child")
        assert child["x"] == 1
        assert child["y"] == 2
