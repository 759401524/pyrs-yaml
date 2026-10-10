"""The standard `!!` tags decide the type - and both reference libraries agree that they do.

Measured before writing: PyYAML and ruamel both read `!!str 1.20` as the string `"1.20"`, `!!float 1` as `1.0`,
`!!bool yes` as `True`, and `!!binary aGk=` as `b'hi'`. The loader used to answer `1.2`, `1`, `'yes'` and
`'aGk='` - it resolved the text as though the tag were absent, so an explicit statement of type changed the value
silently, and in the `!!str` case changed its spelling too.

The parity test at the bottom is the point of the file; the rest pin the parts of it a reader touches first.
"""

import pytest

import pyrs_yaml


@pytest.fixture(autouse=True)
def _ensure_builtin_plugins():
    """Re-register the built-in plugins, because the community-plugin suites clear the shared registry.

    The same guard `tests/test_toml.py` carries: `_register_builtins` is idempotent, and a `bytes` value
    reaches its tag through a registered plugin - so a run ordered after `clear_type_handlers()` would be
    measuring the registry's absence rather than the tag.
    """
    from pyrs_yaml.plugins import _builtin

    _builtin._register_builtins()


CASES = [
    ("str", "version: !!str 1.20\n", {"version": "1.20"}),
    ("str-yes", "b: !!str yes\n", {"b": "yes"}),
    ("int-hex", "n: !!int 0x1F\n", {"n": 31}),
    ("int-octal", "n: !!int 0o17\n", {"n": 15}),
    ("int-binary", "n: !!int 0b101\n", {"n": 5}),
    ("int-underscore", "n: !!int 1_000\n", {"n": 1000}),
    ("int-signed", "n: !!int +5\n", {"n": 5}),
    ("int-beyond-i64", "n: !!int 123456789012345678901234567890\n", {"n": 123456789012345678901234567890}),
    ("float-from-int", "f: !!float 1\n", {"f": 1.0}),
    ("float-inf", "f: !!float .inf\n", {"f": float("inf")}),
    ("float-negative-inf", "f: !!float -.Inf\n", {"f": float("-inf")}),
    ("float-underscore", "f: !!float 1_0.5\n", {"f": 10.5}),
    ("bool-yes", "b: !!bool yes\n", {"b": True}),
    ("bool-off", "b: !!bool OFF\n", {"b": False}),
    ("bool-true", "b: !!bool True\n", {"b": True}),
    ("binary", "d: !!binary aGk=\n", {"d": b"hi"}),
    ("binary-empty", "d: !!binary ''\n", {"d": b""}),
    ("null", "n: !!null ~\n", {"n": None}),
    ("map", "m: !!map\n  a: 1\n", {"m": {"a": 1}}),
    ("seq", "s: !!seq\n  - 1\n", {"s": [1]}),
]


class TestTagDecidesTheType:
    @pytest.mark.parametrize(("case", "text", "want"), CASES, ids=[c[0] for c in CASES])
    def test_loaded_value(self, case, text, want):
        assert pyrs_yaml.safe_load(text) == want, case

    @pytest.mark.parametrize(("case", "text", "want"), CASES, ids=[c[0] for c in CASES])
    def test_the_tag_survives_the_round_trip(self, case, text, want):
        """The text is preserved as written, and re-reading it gives the same value back."""
        emitted = pyrs_yaml.parse(text).to_yaml()
        assert emitted == text, case
        assert pyrs_yaml.safe_load(emitted) == want, case

    def test_the_verbatim_uri_form_agrees_with_the_shorthand(self):
        assert pyrs_yaml.safe_load("v: !<tag:yaml.org,2002:str> 1.20\n") == {"v": "1.20"}

    def test_an_empty_tagged_value_keeps_its_meaning_though_not_its_bytes(self):
        """`n: !!null` re-emits as `n: !!null ` - a trailing space, not a lost value.

        Recorded here so the writer's tag-then-empty-scalar path is not a surprise to whoever makes the
        emission byte-exact: the value and the idempotence both hold today.
        """
        emitted = pyrs_yaml.parse("n: !!null\n").to_yaml()
        assert emitted == "n: !!null \n"
        assert pyrs_yaml.safe_load(emitted) == {"n": None}
        assert pyrs_yaml.parse(emitted).to_yaml() == emitted

    def test_a_tagged_mapping_key_is_read_by_the_same_rule(self):
        # `!!str 1` as a key is the string "1", so it is a different key from the integer 1 - the
        # tag has to be honoured on the key side too, which is where a value-only fix would stop.
        loaded = pyrs_yaml.safe_load("k:\n  !!str 1: a\n  1: b\n")
        assert loaded == {"k": {"1": "a", 1: "b"}}

    def test_the_tag_is_visible_on_the_node(self):
        node = pyrs_yaml.Node(pyrs_yaml.parse("d: !!binary aGk=\n")).find("$.d")
        assert node.tag == "!!binary"


class TestTextThatIsNotTheTaggedType:
    @pytest.mark.parametrize(
        "text",
        [
            "n: !!int hello\n",
            "n: !!int 1.0\n",
            "f: !!float hello\n",
            "f: !!float inf\n",
            "b: !!bool maybe\n",
            "d: !!binary 'ab$'\n",
            "d: !!binary 'A='\n",
        ],
    )
    def test_raises_rather_than_answering_with_a_string(self, text):
        # The old loader returned the text for all of these, which is the silent half of the defect:
        # a tag that cannot be honoured has to say so instead of quietly changing the type.
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.safe_load(text)

    def test_an_unknown_tag_stays_lenient_by_design(self):
        """Both reference libraries raise on `!!weird`; this engine does not.

        Application tags are how the plugin system works, and the tag survives the round trip either
        way, so the lenient answer is kept and recorded here rather than being an untested accident.
        """
        assert pyrs_yaml.safe_load("u: !!weird x\n") == {"u": "x"}
        assert pyrs_yaml.parse("u: !!weird x\n").to_yaml() == "u: !!weird x\n"


class TestBytesWriteRoute:
    """The write side used to emit `!binary`, which is not the tag it means.

    `!!binary` is what YAML names it and what PyYAML and ruamel both read; a local `!binary` is an
    application tag to them, so every document this library dumped with a bytes value was unreadable
    elsewhere while looking standard at a glance. The plugin keeps the local spelling registered, so
    the documents already written still load.
    """

    def test_dumping_bytes_emits_the_standard_tag(self):
        text = pyrs_yaml.safe_dump({"d": b"hi"})
        assert "!!binary" in text, text
        assert "!binary" not in text.replace("!!binary", ""), text
        assert pyrs_yaml.safe_load(text) == {"d": b"hi"}

    def test_pyyaml_can_read_what_we_dump(self):
        yaml = pytest.importorskip("yaml", reason="PyYAML is the interop reader")
        text = pyrs_yaml.safe_dump({"d": b"hello bytes"})
        assert yaml.safe_load(text) == {"d": b"hello bytes"}

    def test_empty_bytes_round_trip(self):
        # The old route wrote `!binary ` with nothing after it, which re-read as None and then raised
        # inside the plugin - the value could not survive its own empty case.
        text = pyrs_yaml.safe_dump({"d": b""})
        assert pyrs_yaml.safe_load(text) == {"d": b""}

    def test_every_length_survives_dump_then_load(self):
        for length in range(0, 24):
            payload = bytes((i * 37 + 11) % 256 for i in range(length))
            assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump({"d": payload})) == {"d": payload}, length

    def test_the_local_spelling_still_loads(self):
        # Documents already on disk. Dropping this would turn a spelling fix into data loss.
        assert pyrs_yaml.safe_load("d: !binary aGk=\n") == {"d": b"hi"}

    def test_the_edit_route_drops_the_tag_for_now(self):
        """Known gap, asserted rather than hoped away: `set` splices text, not metadata.

        The splicing editor writes a replaced scalar's text and leaves its anchor/tag behind, so a bytes
        value assigned through `YamlDocument.set` loses the tag - the same gap any tagged value has on
        that route, and the reason the fix belongs to the editor rather than to the tag reading.
        """
        doc = pyrs_yaml.parse("d: x\n")
        doc.set("$.d", b"hi")
        assert doc.to_yaml() == "d: aGk=\n"


class TestParityWithPyYAML:
    """The claim is not "we chose these readings", it is "these are what the reference answers are"."""

    @pytest.mark.parametrize(("case", "text", "want"), CASES, ids=[c[0] for c in CASES])
    def test_safe_load_agrees_with_pyyaml(self, case, text, want):
        yaml = pytest.importorskip("yaml", reason="PyYAML is the reference oracle")

        assert yaml.safe_load(text) == want, case
        assert pyrs_yaml.safe_load(text) == yaml.safe_load(text), case
