import pytest

import pyrs_yaml


@pytest.mark.parametrize(
    "yaml,expected",
    [
        ('key: "value:with:colons"', "value:with:colons"),
        ('key: "value#with#hash"', "value#with#hash"),
        ('key: "-value"', "-value"),
        ('key: "value[0]"', "value[0]"),
        ('key: "value{a: 1}"', "value{a: 1}"),
    ],
    ids=["colon", "hash", "dash", "brackets", "curly"],
)
def test_special_chars(yaml, expected):
    assert pyrs_yaml.parse(yaml).get("key") == expected


@pytest.mark.parametrize(
    "yaml,expected",
    [
        ("key: 'single'", "single"),
        ('key: "double"', "double"),
        ("key: plain", "plain"),
    ],
    ids=["single-quoted", "double-quoted", "plain"],
)
def test_quoted_scalars(yaml, expected):
    assert pyrs_yaml.parse(yaml).get("key") == expected


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("key: |\n  line1\n  line2", "mapping"),
        ("key: >\n  this is\n  folded", "mapping"),
    ],
    ids=["literal-block", "folded-block"],
)
def test_block_scalars(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("{}", "mapping"),
        ("[]", "sequence"),
    ],
    ids=["empty-mapping", "empty-sequence"],
)
def test_empty_collections(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("anchor: &a value\nalias: *a", "mapping"),
        ("defaults: &defaults\n  timeout: 30\ndevelopment:\n  <<: *defaults", "mapping"),
    ],
    ids=["anchor-alias", "merge-keys"],
)
def test_anchor_and_alias(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


def test_nested_sequence():
    yaml_str = "- - nested1\n  - nested2\n- top"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "sequence"


# Inputs that carry a second document, with how many the reader counts. `a: 1\n---\n` is here
# because the empty document after the separator is a document: the single route used to answer it
# with null, discarding the one the caller wrote. `a: 1\n...\nb: 2\n` is here because the shape is
# over-accepted (YAML 1.2 wants `---` to open that document) and the arity contract is a different
# question from the grammar one — the stream routes still read it, tested below.
STREAMS = {
    "explicit separator": ("---\nfirst: 1\n---\nsecond: 2", 2),
    "separator after content": ("a: 1\n---\nb: 2\n", 2),
    "trailing separator": ("a: 1\n---\n", 2),
    "three documents": ("a: 1\n---\nb: 2\n---\nc: 3\n", 3),
    "second document is a note": ("a: 1\n---\n# just a note\n", 2),
    "empty document between separators": ("a: 1\n---\n---\nb: 2\n", 3),
    "scalar documents": ("1\n---\n2\n", 2),
    "flow documents": ("[1, 2]\n---\n[3]\n", 2),
    "document after end marker": ("a: 1\n...\nb: 2\n", 2),
    "declared second document": ("a: 1\n...\n%YAML 1.1\n---\nb: 2\n", 2),
}

# One document, in every marker spelling that could be mistaken for a second one. PyYAML and
# ruamel.yaml read each of these as a single document too.
SINGLES = {
    "plain": "a: 1\n",
    "explicit start": "---\na: 1\n",
    "explicit end": "a: 1\n...\n",
    "directive then start": "%YAML 1.1\n---\na: 1\n",
    "bare start marker": "---\n",
    "note only": "# only a note\n",
    "empty": "",
}


def test_a_stream_refuses_the_single_document_routes():
    routes = {
        "parse": pyrs_yaml.parse,
        "safe_load": pyrs_yaml.safe_load,
        "YAML().parse": pyrs_yaml.YAML().parse,
        "YAML().safe_load": pyrs_yaml.YAML().safe_load,
    }
    for label, (src, _count) in STREAMS.items():
        for name, route in routes.items():
            with pytest.raises(pyrs_yaml.YamlParseError, match="multi-document-input") as caught:
                route(src)
            text = str(caught.value)
            assert "safe_loads()" in text and "parse_all_docs()" in text and "parse_stream()" in text, (
                f"{label} via {name}: the refusal must name the routes that accept a stream"
            )


def test_the_refusal_reports_where_the_second_document_starts():
    # One-based, at the separator the caller would cut on: `---` is line 2 of `a: 1\n---\nb: 2\n`.
    with pytest.raises(pyrs_yaml.YamlParseError, match=r"second document at line 2, column 1"):
        pyrs_yaml.parse("a: 1\n---\nb: 2\n")
    # A declared second document is reported at its own `---`, which is line 4 here. PyYAML marks
    # that document's `%YAML` line instead — the verdicts agree, the anchor is this reader's.
    with pytest.raises(pyrs_yaml.YamlParseError, match=r"second document at line 4, column 1"):
        pyrs_yaml.parse("a: 1\n...\n%YAML 1.1\n---\nb: 2\n")


def test_the_stream_routes_still_read_every_document():
    for label, (src, count) in STREAMS.items():
        assert len(pyrs_yaml.safe_loads(src)) == count, label
        assert len(pyrs_yaml.parse_all_docs(src)) == count, label
        starts = [event for event in pyrs_yaml.parse_stream(src) if event["type"] == "document_start"]
        assert len(starts) == count, f"{label}: the event route counted {len(starts)} of {count}"
    assert pyrs_yaml.safe_loads("a: 1\n---\nb: 2\n") == [{"a": 1}, {"b": 2}]
    assert pyrs_yaml.safe_loads("a: 1\n---\n") == [{"a": 1}, None]


def test_one_document_in_any_marker_spelling_is_not_refused():
    for src in SINGLES.values():
        pyrs_yaml.parse(src)
        pyrs_yaml.safe_load(src)
    assert pyrs_yaml.safe_load("---\na: 1\n") == {"a": 1}
    assert pyrs_yaml.safe_load("a: 1\n...\n") == {"a": 1}
    assert pyrs_yaml.safe_load("---\n") is None
    assert pyrs_yaml.safe_load("") is None
    assert pyrs_yaml.safe_load("%YAML 1.1\n---\na: 1\n") == {"a": 1}


def test_parse_file_refuses_a_stream_and_reads_one_document(tmp_path):
    stream = tmp_path / "stream.yaml"
    stream.write_text("a: 1\n---\nb: 2\n", encoding="utf-8")
    with pytest.raises(pyrs_yaml.YamlParseError, match="multi-document-input"):
        pyrs_yaml.parse_file(str(stream))
    single = tmp_path / "single.yaml"
    single.write_text("---\na: 1\n...\n", encoding="utf-8")
    assert pyrs_yaml.parse_file(str(single)).to_yaml() == "a: 1\n"


def test_pyyaml_refuses_every_input_this_one_refuses():
    """Verdict-level agreement with the outside authority, one direction only.

    Each input the arity contract refuses is refused by `yaml.safe_load` too. It is not refused
    for the same reason everywhere: `a: 1\n...\nb: 2\n` is a grammar error to PyYAML (`expected
    '<document start>'`) and a second document here, and the `%YAML` case puts the mark on the
    directive line where this reports the separator. The single-document inputs are the other
    half of the probe — an authority that refused those would mean the guard over-fires.
    """
    yaml = pytest.importorskip("yaml")

    def refused(src):
        try:
            yaml.safe_load(src)
            return False
        except Exception:
            return True

    for src, _ in STREAMS.values():
        assert refused(src), f"PyYAML accepted the stream {src!r}"
    for src in SINGLES.values():
        assert not refused(src), f"PyYAML refused the single document {src!r}"


def test_complex_nested():
    yaml_str = """
server:
  host: localhost
  port: 8080
database:
  driver: postgresql
  host: db.example.com
  port: 5432
"""
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("server") is not None
    assert doc.get("database") is not None
