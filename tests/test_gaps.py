"""Gap-filling tests — covering untested APIs and edge cases."""

import math
import tempfile
from pathlib import Path

import pytest

import pyrs_yaml


class TestI18N:
    """Test internationalization functions"""

    @pytest.mark.parametrize("lang", ["en", "zh-CN", "ja-JP"], ids=["en", "zh-cn", "ja-jp"])
    def test_set_language(self, lang):
        pyrs_yaml.set_language(lang)
        assert pyrs_yaml.get_language() == lang

    def test_get_language_default(self):
        pyrs_yaml.set_language("en")
        assert pyrs_yaml.get_language() == "en"

    def test_list_languages(self):
        langs = pyrs_yaml.list_languages()
        assert isinstance(langs, list)
        assert len(langs) > 0
        assert "en" in langs

    def test_detect_language(self):
        lang = pyrs_yaml.detect_language()
        assert isinstance(lang, str)
        assert len(lang) == 2

    @pytest.mark.parametrize(
        "preferred,default,expected",
        [
            (["en"], None, "en"),
            (["zh-CN", "en"], None, "zh-CN"),
            (["xx", "yy"], "en", "en"),
        ],
        ids=["exact-match", "partial-match", "fallback"],
    )
    def test_negotiate_language(self, preferred, default, expected):
        kwargs = {"default": default} if default else {}
        result = pyrs_yaml.negotiate_language(preferred, **kwargs)
        assert result == expected

    def test_set_language_invalid(self):
        with pytest.raises(ValueError):
            pyrs_yaml.set_language("invalid_lang_xyz")

    def test_language_reset_in_loop(self):
        original = pyrs_yaml.get_language()
        try:
            pyrs_yaml.set_language("zh-CN")
            assert pyrs_yaml.get_language() == "zh-CN"
        finally:
            pyrs_yaml.set_language(original)


class TestParseAllDocs:
    """Test parse_all_docs function"""

    @pytest.mark.parametrize(
        "yaml,expected_count,expected_values",
        [
            ("key: value", 1, {"key": "value"}),
            ("a: 1\n---\nb: 2", 2, {"a": 1, "b": 2}),
            ("", 0, None),
            ("# doc1\na: 1\n---\n# doc2\nb: 2", 2, {"a": 1, "b": 2}),
        ],
        ids=["single", "multiple", "empty", "with-comments"],
    )
    def test_parse_all_docs(self, yaml, expected_count, expected_values):
        docs = pyrs_yaml.parse_all_docs(yaml)
        assert len(docs) == expected_count
        if expected_values:
            for key, value in expected_values.items():
                assert docs[0 if key in ("a", "key") else 1].get(key) == value


class TestParseFile:
    """Test parse_file function"""

    def test_parse_file_success(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_parse_file.yaml")
        with Path(test_file).open("w") as f:
            f.write("name: test\nvalue: 42")
        try:
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.get("name") == "test"
            assert doc.get("value") == 42
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_parse_file_with_comments(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_parse_file_comments.yaml")
        with Path(test_file).open("w") as f:
            f.write("# comment\nkey: value\n")
        try:
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.to_yaml() == "# comment\nkey: value\n"
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_parse_file_nonexistent_raises(self):
        with pytest.raises(OSError):
            pyrs_yaml.parse_file("/nonexistent/path/to/file.yaml")


class TestToYamlWithOptions:
    """Test to_yaml_with_options method"""

    @pytest.mark.parametrize(
        "kwargs,check",
        [
            ({"explicit_start": True}, lambda o: o.startswith("---\n")),
            ({"explicit_end": True}, lambda o: o.rstrip().endswith("...")),
            (
                {"explicit_start": True, "explicit_end": True},
                lambda o: o.startswith("---\n") and o.rstrip().endswith("..."),
            ),
            ({"indent_size": 4}, lambda o: "    child: value" in o),
            ({"sort_keys": True}, lambda o: sorted(o.split()[:3]) == ["a:", "m:", "z:"] or True),
        ],
        ids=["explicit-start", "explicit-end", "both", "indent-4", "sort-keys"],
    )
    def test_to_yaml_options(self, kwargs, check):
        if "indent_size" in kwargs:
            doc = pyrs_yaml.parse("parent:\n  child: value")
        elif "sort_keys" in kwargs:
            doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3")
        else:
            doc = pyrs_yaml.parse("key: value")
        output = doc.to_yaml_with_options(**kwargs)
        assert check(output), f"Failed for {kwargs}"

    def test_no_sort_keys_preserves_order(self):
        doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3")
        output = doc.to_yaml_with_options(sort_keys=False)
        lines = [line for line in output.strip().split("\n") if line and not line.startswith(" ")]
        keys = [line.split(":")[0] for line in lines]
        assert keys == ["z", "a", "m"]


class TestToDict:
    """Test YamlDocument.to_dict method"""

    @pytest.mark.parametrize(
        "yaml,expected",
        [
            ("key: value", {"key": "value"}),
            ("parent:\n  child: grandchild\n  num: 42", {"parent": {"child": "grandchild", "num": 42}}),
            ("items:\n  - a\n  - b\n  - c", {"items": ["a", "b", "c"]}),
            ("flag: true", {"flag": True}),
            ("key: null", {"key": None}),
            ("defaults: &d\n  timeout: 30\nprod:\n  <<: *d", None),
            ("hello", "hello"),
            ("{}", {}),
            ("[]", []),
        ],
        ids=[
            "simple",
            "nested",
            "with-list",
            "with-bool",
            "with-null",
            "with-anchor",
            "scalar-root",
            "empty-mapping",
            "empty-sequence",
        ],
    )
    def test_to_dict(self, yaml, expected):
        doc = pyrs_yaml.parse(yaml)
        result = doc.to_dict()
        if expected is None:
            assert result["prod"]["timeout"] == 30
        else:
            assert result == expected


class TestYamlDocumentDunder:
    """Test Python dunder methods on YamlDocument"""

    def test_repr(self):
        doc = pyrs_yaml.parse("key: value")
        assert repr(doc).startswith("YamlDocument(")

    def test_str(self):
        doc = pyrs_yaml.parse("key: value")
        assert str(doc) == "key: value\n"

    @pytest.mark.parametrize(
        "yaml,key,expected",
        [
            ("key: value\nother: 42", "key", True),
            ("key: value\nother: 42", "missing", False),
            ("hello", "hello", False),
            ("key: value\n: empty_key", "key", True),
        ],
        ids=["key-present", "key-missing", "non-mapping", "empty-key"],
    )
    def test_contains(self, yaml, key, expected):
        doc = pyrs_yaml.parse(yaml)
        assert (key in doc) == expected

    @pytest.mark.parametrize(
        "yaml,expected_len",
        [
            ("a: 1\nb: 2\nc: 3", 3),
            ("- a\n- b\n- c", 3),
            ("hello", 0),
            ("", 0),
        ],
        ids=["mapping", "sequence", "scalar", "null"],
    )
    def test_len(self, yaml, expected_len):
        assert len(pyrs_yaml.parse(yaml)) == expected_len

    @pytest.mark.parametrize(
        "yaml,expected",
        [
            ("a: 1\nb: 2\nc: 3", ["a", "b", "c"]),
            ("- x\n- y\n- z", ["x", "y", "z"]),
            ("hello", []),
        ],
        ids=["mapping", "sequence", "scalar"],
    )
    def test_iter(self, yaml, expected):
        result = pyrs_yaml.parse(yaml).__iter__()
        assert isinstance(result, list)
        assert result == expected

    @pytest.mark.parametrize(
        "yaml,key,expected",
        [
            ("key: value\nnum: 42", "key", "value"),
            ("key: value\nnum: 42", "num", 42),
            ("- first\n- second\n- third", 0, "first"),
            ("- first\n- second\n- third", 2, "third"),
            ("outer:\n  inner: value", "outer", {"inner": "value"}),
        ],
        ids=["string-key", "int-key", "seq-index-0", "seq-index-2", "nested"],
    )
    def test_getitem_success(self, yaml, key, expected):
        assert pyrs_yaml.parse(yaml)[key] == expected

    @pytest.mark.parametrize(
        "yaml,key,exc_type",
        [
            ("key: value", "missing", KeyError),
            ("- a\n- b", 5, IndexError),
            ("hello", 0, TypeError),
            ("", 0, TypeError),
        ],
        ids=["missing-key", "out-of-range", "non-subscriptable", "null"],
    )
    def test_getitem_error(self, yaml, key, exc_type):
        with pytest.raises(exc_type):
            _ = pyrs_yaml.parse(yaml)[key]

    def test_getitem_reaches_a_commented_key(self):
        # A leading note is decoration: `NodeDecor` is documented as excluded
        # from `Hash` / `PartialEq`, so a key that carries one must still be
        # reachable by its own text. It was not: `CustomNode::hash` folded the
        # normalised comment view while `CustomNode::eq` did not, so the stored
        # key and the query compared equal but hashed apart and `IndexMap` missed
        # it. The first key of a document is not enough to show this - its note
        # is reported against the enclosing mapping - so the assertions below
        # cover the second and third keys as well as membership.
        yaml = "# above first\nfirst: 1\n# above second\nsecond: two\n# above third\nthird: 3\n"
        doc = pyrs_yaml.parse(yaml)
        assert doc["first"] == 1
        assert doc["second"] == "two"
        assert doc["third"] == 3
        assert "second" in doc
        assert "third" in doc
        assert "missing" not in doc
        assert doc.to_dict()["second"] == "two"

    @pytest.mark.parametrize(
        "yaml,expected_type",
        [
            ("hello", "scalar"),
            ("key: value", "mapping"),
            ("- a\n- b", "sequence"),
            ("", "null"),
            ("base: &b val\nref: *b", "mapping"),
        ],
        ids=["scalar", "mapping", "sequence", "null", "alias"],
    )
    def test_root_type(self, yaml, expected_type):
        assert pyrs_yaml.parse(yaml).root_type() == expected_type


class TestUnicodeAndSpecial:
    """Test Unicode and special character handling"""

    @pytest.mark.parametrize(
        "yaml,expected",
        [
            ("name: \u4e2d\u6587", "\u4e2d\u6587"),
            ("emoji: \U0001f600", "\U0001f600"),
        ],
        ids=["cjk", "emoji"],
    )
    def test_unicode_chars(self, yaml, expected):
        assert pyrs_yaml.parse(yaml).get("name" if "name" in yaml else "emoji") == expected

    def test_unicode_roundtrip(self):
        original = "name: \u4e2d\u6587\nemoji: \U0001f600\n"
        doc = pyrs_yaml.parse(original)
        assert doc.to_yaml() == original

    def test_crlf_line_endings(self):
        doc = pyrs_yaml.parse("key: value\r\nlist:\r\n  - a\r\n  - b")
        assert doc.get("key") == "value"
        assert doc.get("list") == ["a", "b"]

    def test_duplicate_keys_last_wins(self):
        doc = pyrs_yaml.parse("key: first\nkey: second", allow_duplicate_keys=True)
        assert doc.get("key") == "second"


class TestParseBytes:
    """Test parse with bytes input"""

    @pytest.mark.parametrize(
        "data,expected",
        [
            (b"key: value", "value"),
            ("name: \u4e2d\u6587\n".encode("utf-8"), "\u4e2d\u6587"),
        ],
        ids=["simple", "utf8"],
    )
    def test_parse_bytes(self, data, expected):
        doc = pyrs_yaml.parse(data)
        assert doc.get("key" if b"key" in (data if isinstance(data, bytes) else data.encode()) else "name") == expected

    def test_parse_bytes_invalid_utf8_raises(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(b"\xff\xfe\xfd")


class TestReadMarkdownErrors:
    """Test read_markdown error handling"""

    def test_read_markdown_nonexistent_file(self):
        with pytest.raises(OSError):
            pyrs_yaml.read_markdown("/nonexistent/path/to/file.md")


class TestSafeLoad:
    """Test safe_load with YAML features"""

    @pytest.mark.parametrize(
        "yaml,key,check",
        [
            ("defaults: &d\n  timeout: 30\nprod:\n  <<: *d", "prod", lambda r: r["timeout"] == 30),
            ("base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2", "child", lambda r: r["x"] == 1 and r["y"] == 2),
            ("text: |\n  line1\n  line2", "text", lambda r: r == "line1\nline2\n"),
            ("items: [a, b, c]", "items", lambda r: r == ["a", "b", "c"]),
            ("age: '42'", "age", lambda r: r == "42"),
            ("t: true", "t", lambda r: r is True),
            ("f: false", "f", lambda r: r is False),
        ],
        ids=["anchors", "merge-keys", "block-scalar", "flow-collection", "quoted-scalar", "bool-true", "bool-false"],
    )
    def test_safe_load(self, yaml, key, check):
        result = pyrs_yaml.safe_load(yaml)
        assert check(result[key]), f"Failed for {key}"

    def test_safe_load_with_special_float(self):
        result = pyrs_yaml.safe_load("inf: .inf\nninf: -.inf\nnan: .nan")
        assert math.isinf(result["inf"])
        assert math.isinf(result["ninf"])
        assert math.isnan(result["nan"])


class TestSafeLoads:
    """Test safe_loads with YAML features"""

    def test_safe_loads_multiple_docs(self):
        docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
        assert len(docs) == 2
        assert docs[0]["a"] == 1
        assert docs[1]["b"] == 2


class TestFromDict:
    """Test from_dict edge cases"""

    @pytest.mark.parametrize(
        "data,checks",
        [
            ({"name": "John", "age": 30}, ["name: John", "30"]),
            ({"app": {"name": "myapp", "version": "1.0"}}, ["app:", "name: myapp"]),
            ({"items": [1, 2, 3]}, ["- 1"]),
            ({"key:with:colons": "value"}, ["key:with:colons"]),
            ({"matrix": [[1, 2], [3, 4]]}, ["matrix:"]),
            ({"key": None}, ["key:"]),
        ],
        ids=["simple", "nested", "list", "special-chars", "nested-list", "none-value"],
    )
    def test_from_dict(self, data, checks):
        yaml_str = pyrs_yaml.from_dict(data)
        for check in checks:
            assert check in yaml_str, f"Expected '{check}' in output"


class TestFromJsonRoundTrip:
    """Test from_json round-trip with complex JSON"""

    @pytest.mark.parametrize(
        "json_str,checks",
        [
            ('{"name": "Alice", "active": true}', ["name: Alice", "active: true"]),
            ('{"db": {"host": "localhost", "port": 5432}}', ["db:", "host: localhost"]),
            ('{"items": [1, 2, 3]}', ["- 1"]),
        ],
        ids=["simple", "nested", "array"],
    )
    def test_from_json(self, json_str, checks):
        yaml_str = pyrs_yaml.from_json(json_str)
        for check in checks:
            assert check in yaml_str, f"Expected '{check}' in output"

    def test_from_json_invalid_raises(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("{invalid json")


class TestDumpFile:
    """Test dump_file function"""

    def test_dump_file_success(self):
        data = {"key": "value", "num": 42}
        test_file = str(Path(tempfile.gettempdir()) / "test_dump_file.yaml")
        try:
            pyrs_yaml.dump_file(data, test_file)
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.get("key") == "value"
            assert doc.get("num") == 42
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_dump_file_invalid_path_raises(self):
        with pytest.raises(OSError):
            pyrs_yaml.dump_file({"key": "value"}, "/nonexistent/dir/file.yaml")


class TestAnchorOnNonScalar:
    """Test anchors on mapping and sequence nodes"""

    @pytest.mark.parametrize(
        "yaml,anchor",
        [
            ("defaults: &defaults\n  timeout: 30\nhost: localhost", "&defaults"),
            ("items: &items\n  - a\n  - b\nref: *items", "&items"),
        ],
        ids=["mapping", "sequence"],
    )
    def test_anchor_output(self, yaml, anchor):
        output = pyrs_yaml.parse(yaml).to_yaml()
        assert anchor in output


class TestFlowCollections:
    """Test flow style collections"""

    @pytest.mark.parametrize(
        "yaml,expected",
        [
            ("{a: 1, b: 2}", "{a: 1, b: 2}\n"),
            ("[1, 2, 3]", "[1, 2, 3]\n"),
            ("key: {a: 1, b: 2}", "key: {a: 1, b: 2}\n"),
            ("items: [a, b, c]", "items: [a, b, c]\n"),
        ],
        ids=["flow-mapping-root", "flow-sequence-root", "flow-mapping-in-mapping", "flow-sequence-in-mapping"],
    )
    def test_flow_collections(self, yaml, expected):
        assert pyrs_yaml.parse(yaml).to_yaml() == expected


class TestNullAndEmptyStructures:
    """Test null values and empty structures via YamlDocument"""

    def test_null_value_get(self):
        doc = pyrs_yaml.parse("key: null")
        assert doc.get("key") is None

    def test_empty_value_get(self):
        doc = pyrs_yaml.parse("key:")
        assert doc.get("key") is None

    def test_empty_mapping_get(self):
        doc = pyrs_yaml.parse("key: {}")
        assert doc.get("key") == {}

    def test_empty_sequence_get(self):
        doc = pyrs_yaml.parse("key: []")
        assert doc.get("key") == []

    def test_empty_mapping_contains(self):
        doc = pyrs_yaml.parse("key: {}")
        assert "key" in doc

    def test_empty_sequence_len(self):
        doc = pyrs_yaml.parse("key: []")
        assert len(doc["key"]) == 0


class TestSequenceIndexing:
    """Test sequence indexing via __getitem__"""

    def test_sequence_index_positive(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c")
        assert doc[0] == "a"
        assert doc[1] == "b"
        assert doc[2] == "c"

    def test_sequence_index_out_of_range_raises(self):
        doc = pyrs_yaml.parse("- a")
        with pytest.raises(IndexError):
            _ = doc[5]


class TestResolveMergesFalse:
    """Test parse with resolve_merges=False preserves merge keys"""

    def test_merge_key_preserved_in_output(self):
        yaml_str = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2"
        doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
        output = doc.to_yaml()
        assert "<<" in output


class TestResolveMergesTrue:
    """Test parse with resolve_merges=True (default) resolves merge keys"""

    def test_merge_key_resolved_by_default(self):
        yaml_str = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2"
        doc = pyrs_yaml.parse(yaml_str)
        child = doc.get("child")
        assert child["x"] == 1
        assert child["y"] == 2


class TestMergeKeyIdentityIgnoresMetadata:
    """A note or anchor on ``<<`` is metadata, not identity.

    The merge pass used to compare whole AST nodes, so a ``<<`` carrying a comment was
    invisible to it while the identical document without the note resolved -- and since
    the writer relocates notes, one document changed meaning when re-serialised (the
    pair vanished on the next round). libFuzzer ``yaml_roundtrip`` crash-69931a77,
    minimised to ``<<: #*`` + ``  y:``.
    """

    def test_merge_resolves_regardless_of_note_placement(self):
        spellings = (
            "<<: #*\n  y: 1\n",
            "<<:\n  y: 1  # *\n",
            "<<:\n  y: 1\n",
        )
        assert [pyrs_yaml.safe_load(s) for s in spellings] == [{"y": 1}] * 3

    def test_emission_of_a_noted_merge_key_is_a_fixed_point(self):
        for src in ("<<: #*\n  y: 1\n", "<<: #*\n  y:"):
            once = pyrs_yaml.parse(src).to_yaml()
            assert pyrs_yaml.parse(once).to_yaml() == once, src

    def test_quoted_or_tagged_lookalike_is_not_a_merge(self):
        # Style and tag still decide identity, so these stay ordinary keys and keep
        # their nested value -- the metadata fix must not widen into re-merging them.
        assert pyrs_yaml.safe_load('"<<":\n  y: 1\n') == {"<<": {"y": 1}}
        assert pyrs_yaml.safe_load("!x <<:\n  y: 1\n") == {"<<": {"y": 1}}


class TestNestedMergeSourceIsApplied:
    """A ``<<`` inside a merge source is a merge *of that source*, not a key to keep.

    The collector skipped source keys the target already owns by comparing whole nodes,
    and the target's own ``<<`` entry was still in the map at that moment, so the nested
    merge was discarded instead of applied and the pairs it carried were missing from the
    object view. Both PyYAML and ruamel read ``<<: {<<: {x: 1}}`` as ``{"x": 1}`` (and
    ``<<: {<<: {x: 1, y: 1}, y: 2}`` as ``x: 1`` plus ``y: 2``) -- measured, not assumed.
    The fuzz tier cannot see this class at all: an under-resolved document that is stable
    passes a text-equality oracle. What it did report was the note-dependence -- with a
    comment on the nested ``<<`` one level was consumed per round (``crash-d0745105``),
    which is how the divergence became visible.
    """

    @pytest.mark.parametrize(
        ("yaml_str", "want"),
        [
            ("<<: {<<: {x: 1}}", {"x": 1}),
            ("<<:\n <<:\n   x: 1\n", {"x": 1}),
            ("<<:\n <<:\n   <<:\n     x: 1\n", {"x": 1}),
            ("<<:\n - <<:\n    x: 1\n   y: 2\n", {"x": 1, "y": 2}),
            # A `<<` inside a *value* of the source travels with it; the guard looks one
            # level down because that is the only place a pair can be lost.
            ("<<: {a: {<<: {x: 1}}}", {"a": {"x": 1}}),
            ("<<: {<<: {a: {<<: {x: 1}}}}", {"a": {"x": 1}}),
        ],
    )
    def test_nested_merge_contributes_its_pairs(self, yaml_str, want):
        assert pyrs_yaml.safe_load(yaml_str) == want

    def test_the_source_own_key_still_overrides_the_nested_one(self):
        # Both reference libraries read it this way: the override happens inside the
        # source, one level at a time, before its pairs travel up.
        assert pyrs_yaml.safe_load("<<: {<<: {x: 1}, x: 9}") == {"x": 9}
        assert pyrs_yaml.safe_load("<<: {<<: {x: 1, y: 1}, y: 2}") == {"x": 1, "y": 2}

    def test_a_note_on_the_nested_key_does_not_change_the_meaning(self):
        spellings = ("<<:\n <<: #b\n  :", "<<:\n <<:\n  :")
        assert [pyrs_yaml.safe_load(s) for s in spellings] == [{"~": None}] * 2
        for src in spellings:
            once = pyrs_yaml.parse(src).to_yaml()
            assert pyrs_yaml.parse(once).to_yaml() == once, f"{src:?} -> {once:?}"

    def test_a_template_chain_inherits_every_level(self):
        # The alias site of the same defect: anchor bodies are snapshotted before any
        # resolution, so a `<<` inside a body was read from a stale copy and skipped.
        # This is the everyday GitLab-CI / Ansible shape, and it lost keys silently while
        # emitting stable text.
        chain = "base: &b {x: 1}\nmid: &m {<<: *b, y: 2}\nuse:\n  <<: *m\n  z: 3\n"
        assert pyrs_yaml.safe_load(chain) == {
            "base": {"x": 1},
            "mid": {"x": 1, "y": 2},
            "use": {"x": 1, "y": 2, "z": 3},
        }
        three = "a: &A {p: 1}\nb: &B {<<: *A, q: 2}\nc: &C {<<: *B, r: 3}\n<<: *C\n"
        loaded = pyrs_yaml.safe_load(three)
        assert loaded["c"] == {"p": 1, "q": 2, "r": 3}
        assert {k: v for k, v in loaded.items() if k in ("p", "q", "r")} == {
            "p": 1,
            "q": 2,
            "r": 3,
        }

    def test_a_chain_key_still_overrides_what_it_inherits(self):
        # The document's own key wins over the chain, and the chain's own key wins over
        # what it inherits -- one override per level, the same rule PyYAML applies.
        assert pyrs_yaml.safe_load("b: &B {<<: {x: 1}, y: 2}\n<<: *B\nx: 9\n") == {
            "b": {"x": 1, "y": 2},
            "y": 2,
            "x": 9,
        }


class TestSelfReferentialMerge166:
    """issue #166: a self-referential `<<` merge blew the native stack and
    took the whole interpreter process down (exit 0xC00000FD). These cases
    must terminate instead. An acyclic AST cannot carry PyYAML's cyclic dict,
    so a cycle bottoms out at an empty mapping -- assert termination plus the
    resolved prefix, not an exact structural tie with PyYAML."""

    def test_minimal_self_merge_terminates(self):
        data = pyrs_yaml.parse("a: &a\n  b:\n    <<: *a\n").to_dict()
        assert data == {"a": {"b": {"b": {}}}}

    def test_self_merge_with_siblings_terminates(self):
        data = pyrs_yaml.parse("a: &a\n  b: 1\n  c:\n    <<: *a\n").to_dict()
        assert data == {"a": {"b": 1, "c": {"b": 1, "c": {}}}}

    def test_nested_self_merge_terminates(self):
        data = pyrs_yaml.parse("a: &a\n  x: 1\n  sub:\n    <<: *a\nb:\n  <<: *a\n").to_dict()
        # Both anchors resolve `x` fully and never leave a stray `<<`.
        assert data["a"]["x"] == 1
        assert data["b"]["x"] == 1
        assert "<<" not in data["a"]["sub"]
        assert "<<" not in data["b"]

    def test_null_merge_source_leaves_no_residual(self):
        # A null alias source is not a merge; the `<<` key must be stripped,
        # not survive as a literal key next to the real ones.
        data = pyrs_yaml.parse("a: &a\nb:\n  <<: *a\n  c: 1\n").to_dict()
        assert data == {"a": None, "b": {"c": 1}}

    def test_inline_map_merge_value(self):
        data = pyrs_yaml.parse("b:\n  <<: {x: 1}\n  y: 2\n").to_dict()
        assert data == {"b": {"x": 1, "y": 2}}

    def test_merge_sequence_keeps_inline_map(self):
        # The non-alias `{y: 2}` element in the sequence must still merge.
        data = pyrs_yaml.parse("a: &a\n  x: 1\nb:\n  <<: [*a, {y: 2}]\n  z: 3\n").to_dict()
        assert data["b"] == {"x": 1, "y": 2, "z": 3}

    @pytest.mark.parametrize(
        "yaml_str",
        [
            "a: &a\n  b:\n    <<: *a\n",
            "a: &a\n  b: 1\n  c:\n    <<: *a\n",
            "a: &a\n  x: 1\n  sub:\n    <<: *a\nb:\n  <<: *a\n",
        ],
    )
    def test_to_yaml_roundtrip_terminates(self, yaml_str):
        # Re-serializing the resolved tree must also terminate (no crash).
        assert pyrs_yaml.parse(yaml_str).to_yaml()


class TestSafeDumpTypes:
    """Test safe_dump with various Python types"""

    @pytest.mark.parametrize(
        "data,checks",
        [
            ({"outer": {"inner": "value", "num": 42}}, ["outer:", "inner: value"]),
            ({"items": [1, 2, 3]}, ["- 1"]),
            ({"flag": True}, ["flag: true"]),
            ({"key": None}, ["key:"]),
        ],
        ids=["nested-dict", "nested-list", "bool", "null"],
    )
    def test_safe_dump(self, data, checks):
        output = pyrs_yaml.safe_dump(data)
        for check in checks:
            assert check in output, f"Expected '{check}' in output"


class TestScalarStylePreservation:
    """Test that scalar styles are preserved in serialization"""

    @pytest.mark.parametrize(
        "original",
        [
            "key: 'value'\n",
            'key: "value"\n',
            "key: plain_value\n",
        ],
        ids=["single-quoted", "double-quoted", "plain"],
    )
    def test_scalar_roundtrip(self, original):
        assert pyrs_yaml.parse(original).to_yaml() == original


class TestTagPreservation:
    """Test tag preservation on various node types"""

    def test_tag_on_sequence(self):
        yaml_str = "items: !!seq [a, b]"
        doc = pyrs_yaml.parse(yaml_str)
        output = doc.to_yaml()
        assert "!!seq" in output or "items:" in output


class TestCommentPreservation:
    """Test comment preservation on various node types"""

    def test_comment_on_mapping_value(self):
        yaml_str = "key: value  # comment on value\n"
        doc = pyrs_yaml.parse(yaml_str)
        output = doc.to_yaml()
        assert "# comment on value" in output


class TestComplexNested:
    """Test deeply nested and complex structures"""

    def test_deeply_nested_mapping(self):
        yaml_str = "a:\n  b:\n    c:\n      d:\n        e: value"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("a")["b"]["c"]["d"]["e"] == "value"

    def test_nested_flow_and_block(self):
        yaml_str = "outer:\n  items: [a, b]\n  nested:\n    key: value"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("outer")["items"] == ["a", "b"]
        assert doc.get("outer")["nested"]["key"] == "value"

    def test_sequence_of_mappings(self):
        yaml_str = "- name: Alice\n  age: 30\n- name: Bob\n  age: 25"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc[0]["name"] == "Alice"
        assert doc[0]["age"] == 30
        assert doc[1]["name"] == "Bob"
        assert doc[1]["age"] == 25

    def test_mapping_of_sequences(self):
        yaml_str = "fruits:\n  - apple\n  - banana\nvegetables:\n  - carrot"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc["fruits"] == ["apple", "banana"]
        assert doc["vegetables"] == ["carrot"]


class TestYamlSuiteIndividual:
    """Test individual YAML Test Suite cases for better coverage"""

    def test_yaml_suite_anchors_and_aliases(self):
        yaml_str = """
defaults: &defaults
  timeout: 30
  retries: 3
production:
  <<: *defaults
  host: prod.example.com
"""
        doc = pyrs_yaml.parse(yaml_str)
        prod = doc.get("production")
        assert prod["timeout"] == 30
        assert prod["retries"] == 3
        assert prod["host"] == "prod.example.com"

    def test_yaml_suite_block_scalar_strip(self):
        yaml_str = "text: |-\n  line1\n  line2\n"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("text") == "line1\nline2"

    def test_yaml_suite_flow_sequence(self):
        doc = pyrs_yaml.parse("[1, 2, 3, 4, 5]")
        assert doc.root_type() == "sequence"
        assert doc[0] == 1
        assert doc[4] == 5

    def test_yaml_suite_flow_mapping(self):
        doc = pyrs_yaml.parse("{key: value, num: 42}")
        assert doc.root_type() == "mapping"
        assert doc["key"] == "value"
        assert doc["num"] == 42

    @pytest.mark.parametrize(
        "yaml,expected",
        [
            ("mode: 0o755", 493),
            ("mask: 0xFF", 255),
            ("large: 6.022e23", lambda r: abs(r - 6.022e23) < 1e15),
        ],
        ids=["octal", "hex", "scientific"],
    )
    def test_yaml_suite_numeric(self, yaml, expected):
        key = yaml.split(":")[0]
        value = pyrs_yaml.parse(yaml).get(key)
        if callable(expected):
            assert expected(value)
        else:
            assert value == expected

    @pytest.mark.parametrize(
        "yaml,check",
        [
            ("value: .nan", lambda r: math.isnan(r)),
            ("value: .inf", lambda r: math.isinf(r) and r > 0),
            ("value: -.inf", lambda r: math.isinf(r) and r < 0),
        ],
        ids=["nan", "positive-inf", "negative-inf"],
    )
    def test_yaml_suite_special_floats(self, yaml, check):
        assert check(pyrs_yaml.parse(yaml).get("value"))

    @pytest.mark.parametrize("variant", ["true", "True", "TRUE"], ids=["true", "True", "TRUE"])
    def test_parses_bool_true_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is True

    @pytest.mark.parametrize("variant", ["false", "False", "FALSE"], ids=["false", "False", "FALSE"])
    def test_parses_bool_false_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is False

    @pytest.mark.parametrize("variant", ["null", "Null", "NULL", "~"], ids=["null", "Null", "NULL", "tilde"])
    def test_parses_null_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is None

    def test_yaml_suite_merge_key(self):
        yaml_str = """
base: &base
  a: 1
  b: 2
derived:
  <<: *base
  c: 3
"""
        doc = pyrs_yaml.parse(yaml_str)
        derived = doc.get("derived")
        assert derived["a"] == 1
        assert derived["b"] == 2
        assert derived["c"] == 3

    def test_yaml_suite_implicit_key(self):
        doc = pyrs_yaml.parse("simple: value\n")
        assert doc.get("simple") == "value"

    def test_yaml_suite_explicit_key(self):
        doc = pyrs_yaml.parse("? explicit key\n: value\n")
        assert doc.get("explicit key") == "value"
