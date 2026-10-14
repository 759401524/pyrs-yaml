"""What the gate's `--profile` reader is allowed to say about a callgrind table.

`scripts/ir_gate.py --profile SCENARIO` exists because per-function attribution had been done by a
one-off script for three sessions running, and every number in `docs/dev/perf.md` rests on that
reading. Two failures are within reach and both look like a result: reading the raw event file by
hand (an early version summed 8.8M instructions of "self cost" for a 4.5M run, because callgrind's
compressed positions and call-cost records were reimplemented wrong), and grouping rows by a pattern
over the function name - which is how a 12.46% "hash" row appeared for TOML when `Hasher` matched the
*type parameter* of `IndexMap<.., BuildHasherDefault<..>>::insert*`. The table below has the shape
`callgrind_annotate` really prints (a `serialize_small` run, names and costs shortened), with one row
duplicated under a second object file because that is the case the reader has to fold.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

TABLE = """
  4,506,831 (100.0%)  PROGRAM TOTALS

  file:function

    574,000 (12.74%)  ???:<pyrs_yaml_core::serializer::Serializer>::serialize_node_internal'2 [/home/mulong/target/release/deps/ir_gate-abc]
    506,000 (11.23%)  ???:<pyrs_yaml_core::serializer::Serializer>::write_mapping_pair [/home/mulong/target/release/deps/ir_gate-abc]
    402,000 ( 8.92%)  ???:<pyrs_yaml_core::serializer::Serializer>::write_scalar_for_key [/home/mulong/target/release/deps/ir_gate-abc]
    370,000 ( 8.21%)  ???:pyrs_yaml_core::serializer::write_plain_scalar [/home/mulong/target/release/deps/ir_gate-abc]
    294,000 ( 6.52%)  ???:<pyrs_yaml_core::serializer::Serializer>::serialize_node_internal [/home/mulong/target/release/deps/ir_gate-abc]
    255,460 ( 5.67%)  ./malloc/malloc.c:_int_free [/usr/lib/x86_64-linux-gnu/libc.so.6]
    230,125 ( 5.11%)  ./malloc/malloc.c:realloc [/usr/lib/x86_64-linux-gnu/libc.so.6]
    184,000 ( 4.08%)  ???:<pyrs_yaml_core::serializer::Serializer>::write_indent [/home/mulong/target/release/deps/ir_gate-abc]
     68,000 ( 1.51%)  ???:<pyrs_yaml_core::serializer::Serializer>::write_anchor_tag [/home/mulong/target/release/deps/ir_gate-abc]
     68,000 ( 1.51%)  ???:<pyrs_yaml_core::serializer::Serializer>::write_anchor_tag [/usr/lib/x86_64-linux-gnu/libc.so.6]
      4,273 ( 0.09%)  ./string/memmove-vec-unaligned-erms.S:__memcpy_avx_unaligned_erms [/usr/lib/x86_64-linux-gnu/libc.so.6]
"""


def _gate():
    spec = importlib.util.spec_from_file_location("ir_gate", REPO_ROOT / "scripts" / "ir_gate.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_a_row_becomes_one_function_and_its_self_cost():
    per = _gate().parse_annotation(TABLE)
    assert per["???:pyrs_yaml_core::serializer::write_plain_scalar"] == 370_000
    assert per["./malloc/malloc.c:realloc"] == 230_125
    assert per["./string/memmove-vec-unaligned-erms.S:__memcpy_avx_unaligned_erms"] == 4_273
    # The table's own total is a row like any other, and a profile that counted it would report one
    # function as big as everything measured.
    assert not any("PROGRAM TOTALS" in name for name in per)
    # The `file:function` caption carries no cost to read, and the cost-less separator lines are not
    # rows either.
    assert not any("file:function" in name for name in per)


def test_two_rows_for_one_function_are_the_same_function():
    """The same symbol can appear under two object files; the profile answers per function."""
    per = _gate().parse_annotation(TABLE)
    name = "???:<pyrs_yaml_core::serializer::Serializer>::write_anchor_tag"
    assert per[name] == 136_000


def test_nothing_is_grouped_by_a_pattern_over_the_name():
    """The tool has no bucket concept, so the false positive cannot be produced by it.

    `BuildHasherDefault` in a name is a type parameter of the map's own method, not digest work, and
    a regex that matched it once cost a session an hour of chasing a hash cost that was already gone.
    """
    rows = [
        "   10,000,000 (20.0%)  ???:<indexmap::map::IndexMap<String, Table, core::hash::"
        "BuildHasherDefault<FxHasher>>>::insert [/x]\n",
        "      100,000 ( 0.2%)  ???:<CustomNode as core::hash::Hash>::hash::<FxHasher> [/x]\n",
    ]
    per = _gate().parse_annotation("".join(rows))
    assert len(per) == 2
    assert any("insert" in name for name in per)
    assert all("hash::<FxHasher>" in name or "insert" in name for name in per)
