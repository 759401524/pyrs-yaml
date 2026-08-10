"""Tests for splice eligibility: CRLF, BOM, flow-style fallback to full serialize."""

import pyrs_yaml


class TestCRLFDocuments:
    """CRLF documents should fall back to full serialize (not splice)."""

    def test_crlf_document_edit_produces_correct_output(self):
        doc = pyrs_yaml.parse("a: 1\r\nb: 2\r\n")
        doc._set_path(["a"], 9)
        out = doc.to_yaml()
        assert "a: 9" in out
        assert "b: 2" in out

    def test_crlf_document_round_trip(self):
        doc = pyrs_yaml.parse("a: 1\r\nb: 2\r\n")
        # CRLF should be normalized to LF on re-serialization
        out = doc.to_yaml()
        assert "\r\n" not in out


class TestBOMDocuments:
    """BOM documents should fall back to full serialize (not splice)."""

    def test_bom_document_edit_produces_correct_output(self):
        doc = pyrs_yaml.parse("\ufeffa: 1\nb: 2\n")
        doc._set_path(["a"], 9)
        out = doc.to_yaml()
        assert "a: 9" in out
        assert "b: 2" in out


class TestFlowStyleContainers:
    """Flow-style containers should fall back to full serialize (not splice)."""

    def test_flow_mapping_edit_produces_correct_output(self):
        doc = pyrs_yaml.parse("{a: 1, b: 2}\n")
        doc._set_path(["a"], 9)
        out = doc.to_yaml()
        assert "a: 9" in out

    def test_flow_sequence_edit_produces_correct_output(self):
        doc = pyrs_yaml.parse("[1, 2, 3]\n")
        doc._set_path([1], 99)
        out = doc.to_yaml()
        assert "99" in out


class TestBlockStyleEligible:
    """Block-style documents with source ranges should use splice."""

    def test_block_mapping_edit(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["a"], 10)
        out = doc.to_yaml()
        assert out == "a: 10\nb: 2\nc: 3\n"

    def test_block_sequence_edit(self):
        doc = pyrs_yaml.parse("- a\n- b\n")
        doc._set_path([0], "x")
        out = doc.to_yaml()
        assert out == "- x\n- b\n"

    def test_nested_block_mapping_edit(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
        doc._set_path(["a", "b"], 9)
        out = doc.to_yaml()
        assert out == "a:\n  b: 9\n  c: 2\n"


class TestSplicePreservesUntouchedBytes:
    """Verify that splice preserves untouched bytes (zero-copy)."""

    def test_untouched_prefix_preserved(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["b"], 9)
        out = doc.to_yaml()
        assert out.startswith("a: 1\n")
        assert out.endswith("c: 3\n")

    def test_untouched_suffix_preserved(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["b"], 9)
        out = doc.to_yaml()
        assert "c: 3" in out


class TestMultiEditBurst:
    """Multiple edits should accumulate in the splice state."""

    def test_multiple_edits_single_burst(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["a"], 10)
        doc._set_path(["c"], 30)
        out = doc.to_yaml()
        assert out == "a: 10\nb: 2\nc: 30\n"

    def test_edit_then_full_serialize_after_splice_exhausted(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["a"], 10)
        # After materialize, splice is consumed
        _ = doc.to_yaml()
        # Second edit should fall back to full serialize but still work
        doc._set_path(["b"], 20)
        out = doc.to_yaml()
        assert "a: 10" in out
        assert "b: 20" in out
        assert "c: 3" in out
