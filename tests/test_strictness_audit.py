"""Phase 0 strictness audit regression — locks in current rejection/acceptance
behavior for the invalid-YAML probe corpus (v0.11.5, 2026-08-04).

The yaml-test-suite pass rate is saturated at 405/406, so these probes have no
in-suite target. This corpus was compared against a PyYAML oracle:

  ACCEPTED-BUT-INVALID (5)   — accepted by us, rejected by PyYAML. ALL are
                              deliberate YAML 1.2 / suite-required divergences
                              (empty keys 2JQS/CFD4/FRK4, local tags C4HZ,
                              implicit doc after `...`). NOT bugs.
  REJECTED-BUT-VALID  (1)    — rejected by us, accepted by PyYAML. Deliberate
                              duplicate-key strictness (no suite test requires
                              accepting `{a: 1, a: 2}`).
  MATCH-reject        (26)   — both reject.
  MATCH-accept        (38)   — both accept.

Gate outcome: no fixable accepted-but-invalid probe → items 3/4/5 collapse to a
documentation release. These tests pin the behavior so future parser changes
cannot silently regress strictness or over-reject.
"""

import pyrs_yaml
import pytest

# Probes that MUST raise (either YamlParseError or YamlDuplicateKeyError).
REJECT = [
    # ---- Indentation ----
    "indent_tab_block_mapping",
    "indent_below_parent",
    "indent_blank_line_dedent",
    "indent_nested_mismatch",
    "indent_tab_in_block_seq",
    "indent_mapping_under_seq_item",
    "indent_hanging_indent_mismatch",
    "indent_block_literal_tab",
    "indent_explicit_key_indent",
    "indent_collection_then_scalar",
    "indent_seq_item_then_scalar",
    # ---- Block mapping keys ----
    "key_multi_colon_flow",
    "key_indent_seq_then_map",
    "key_double_colon",
    "key_colon_empty_value",
    "key_multiline_value_colon",
    "key_value_after_seq",
    "key_double_colon_seq",
    # ---- Flow context ----
    "flow_map_missing_comma",
    "flow_alias",
    "flow_seq_block_item",
    "flow_double_comma",
    "flow_early_close",
    "flow_unclosed_brace",
    "flow_unclosed_bracket",
    "flow_adjacent_collections",
    # Deliberate duplicate-key strictness (REJECTED-BUT-VALID vs PyYAML)
    "flow_map_dup_key",
]

# Probes that MUST parse without error.
ACCEPT = [
    # ---- Indentation (valid / PyYAML-agreed) ----
    "indent_jump_mapping",
    "indent_seq_continuation",
    "indent_underflow_seq",
    "indent_seq_bad_col",
    "indent_dash_not_at_col",
    "indent_overflow_seq_item",
    "indent_seq_under_mapping",
    "indent_dash_deeper_then_item",
    "indent_scalar_continuation_bad",
    # ---- Block mapping keys (valid / PyYAML-agreed) ----
    "key_colon_at_line_start",
    "key_question_in_block",
    "key_null_then_value",
    "key_seq_of_maps_trailing_colon",
    "key_colon_in_plain",
    "key_trailing_colon",
    "key_explicit_bad_indent",
    "key_simple_key_colon_space",
    "key_spaced_colon",
    # ---- Flow context (valid / PyYAML-agreed) ----
    "flow_seq_missing_comma_items",
    "flow_seq_newline_no_comma",
    "flow_seq_of_maps",
    "flow_nested_mismatch",
    "flow_empty_map",
    "flow_empty_seq",
    "flow_anchor",
    "flow_quote_in_seq",
    "flow_plain_containing_bracket",
    "flow_map_key_colon_in_str",
    "flow_mapping_value_in_seq",
    "flow_seq_no_comma",
    "flow_trailing_comma_seq",
    "flow_trailing_comma_map",
    "flow_colon_in_plain_scalar",
    "flow_seq_in_map_no_comma",
    "flow_colon_no_space_in_seq",
    "flow_double_colon",
    "flow_question_in_map",
    "flow_missing_colon",
    # ---- Deliberate accepted-but-invalid (suite/spec-required) ----
    "key_empty_then_dup",  # 2JQS empty-key block mapping
    "key_in_seq_missing_value",  # 2JQS empty-key family in sequence
    "key_bang_tagged",  # local-tag syntax (C4HZ)
    "key_after_doc_end",  # implicit doc after `...` (YAML 1.2 grammar)
    "flow_colon_after_comma",  # flow empty-key entry (CFD4/FRK4)
]

PROBES = {
    # ---- Indentation ----
    "indent_tab_block_mapping": "a:\n\tb: 1\n",
    "indent_jump_mapping": "a:\n    b: 1\n",
    "indent_seq_continuation": "- a\n  - b\n",
    "indent_below_parent": "a:\n  b:\n c: 1\n",
    "indent_blank_line_dedent": "a:\n  b: 1\n\n c: 2\n",
    "indent_underflow_seq": "a:\n - 1\n  - 2\n",
    "indent_seq_bad_col": "- 1\n   - 2\n",
    "indent_nested_mismatch": "a:\n    b: 1\n  c: 2\n",
    "indent_dash_not_at_col": "a:\n b:\n   - 1\n",
    "indent_overflow_seq_item": "- - 1\n   - 2\n",
    "indent_tab_in_block_seq": "- 1\n\t- 2\n",
    "indent_seq_under_mapping": "a:\n- 1\n- 2\n",
    "indent_mapping_under_seq_item": "- a: 1\n  b: 2\n   c: 3\n",
    "indent_hanging_indent_mismatch": "a: b\n    c: d\n",
    "indent_dash_deeper_then_item": "- a\n  - b\n   - c\n",
    "indent_scalar_continuation_bad": "a: |\n  text\n   more\n",
    "indent_block_literal_tab": "a: |\n\ttext\n",
    "indent_explicit_key_indent": "? a\n    : b\n",
    "indent_collection_then_scalar": "a:\n  - 1\n b: 2\n",
    "indent_seq_item_then_scalar": "- 1\n  - 2\nb: 3\n",
    # ---- Block mapping keys ----
    "key_colon_at_line_start": ":b\n",
    "key_multi_colon_flow": "{a: b: c}\n",
    "key_question_in_block": "? a\n? b\n",
    "key_null_then_value": "~: 1\nb: 2\n",
    "key_seq_of_maps_trailing_colon": "- a:\n  b:\n",
    "key_empty_then_dup": ": a\n: b\n",
    "key_colon_in_plain": "a:b: c\n",
    "key_bang_tagged": "!tag key: value\n",
    "key_after_doc_end": "a: 1\n...\nb: 2\n",
    "key_indent_seq_then_map": "- a: 1\n  - 2\n",
    "key_double_colon": "a: b: c\n",
    "key_trailing_colon": "a:\n",
    "key_colon_empty_value": "a: : b\n",
    "key_explicit_bad_indent": "? a\n  ? b\n",
    "key_simple_key_colon_space": "a : b\n",
    "key_multiline_value_colon": "a: 1\n  b: c: d\n",
    "key_value_after_seq": "- a: 1\n  b: 2: 3\n",
    "key_in_seq_missing_value": "- a: 1\n- : 2\n",
    "key_double_colon_seq": "- a: b: c\n",
    "key_spaced_colon": "a: 1\nb : 2\nc: 3\n",
    # ---- Flow context ----
    "flow_mapping_value_in_seq": "[a, b: c]\n",
    "flow_seq_missing_comma_items": "[1 2]\n",
    "flow_map_missing_comma": "{a: 1 b: 2}\n",
    "flow_seq_newline_no_comma": "[1\n2]\n",
    "flow_map_dup_key": "{a: 1, a: 2}\n",
    "flow_seq_of_maps": "[{a: 1}, {b: 2}]\n",
    "flow_nested_mismatch": "[{a: 1}]\n",
    "flow_colon_after_comma": "[a, : b]\n",
    "flow_empty_map": "{}\n",
    "flow_empty_seq": "[]\n",
    "flow_anchor": "&a [1, 2]\n",
    "flow_alias": "*a\n",
    "flow_quote_in_seq": "['a', \"b\"]\n",
    "flow_plain_containing_bracket": "a[b]\n",
    "flow_map_key_colon_in_str": '{"a:b": 1}\n',
    "flow_seq_block_item": "[a, b]\n  [c]\n",
    "flow_seq_no_comma": "{a: [1 2]}\n",
    "flow_double_comma": "[a,, b]\n",
    "flow_trailing_comma_seq": "[a,]\n",
    "flow_trailing_comma_map": "{a: 1,}\n",
    "flow_colon_in_plain_scalar": "{a: b:c}\n",
    "flow_early_close": "[a, ] ]\n",
    "flow_unclosed_brace": "{a: 1\n",
    "flow_unclosed_bracket": "[a, b\n",
    "flow_seq_in_map_no_comma": "{a: [1], b: [2]}\n",
    "flow_colon_no_space_in_seq": "[a:1]\n",
    "flow_double_colon": "[a::b]\n",
    "flow_question_in_map": "{? a: b}\n",
    "flow_missing_colon": "{a b}\n",
    "flow_adjacent_collections": "[a][b]\n",
}


@pytest.mark.parametrize("label", REJECT)
def test_probe_rejects(label):
    with pytest.raises((pyrs_yaml.YamlParseError, pyrs_yaml.YamlDuplicateKeyError)):
        pyrs_yaml.parse(PROBES[label])


@pytest.mark.parametrize("label", ACCEPT)
def test_probe_accepts(label):
    pyrs_yaml.parse(PROBES[label])


def test_probe_coverage_complete():
    assert set(REJECT) | set(ACCEPT) == set(PROBES)
    assert len(PROBES) == 70
