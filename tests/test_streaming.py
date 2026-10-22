"""Streaming parse tests (migrated from Rust py::streaming tests)."""

import io

import pytest

import pyrs_yaml


def _docs_from_events(events):
    """Extract document content from raw event stream."""
    docs = []
    current = []
    in_doc = False
    for ev in events:
        t = ev["type"]
        if t == "document_start":
            in_doc = True
            current = []
        elif t == "document_end":
            if in_doc:
                docs.append(current)
                in_doc = False
        elif t == "stream_end":
            if current and in_doc:
                docs.append(current)
        elif in_doc:
            current.append(ev)
    return docs


class TestStreaming:
    """Streaming YAML parse behavior."""

    def test_simple_scalar(self):
        yaml = "hello\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        assert any(e["value"] == "hello" for e in docs[0])

    def test_mapping(self):
        yaml = "key: value\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        values = {e["value"] for e in docs[0] if e["type"] == "scalar"}
        assert "key" in values
        assert "value" in values

    def test_multiple_documents(self):
        yaml = "---\na: 1\n---\nb: 2\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 2

    def test_empty_input(self):
        buf = io.StringIO("")
        events = list(pyrs_yaml.YAML().load_stream(buf))
        assert len(events) >= 2  # stream_start + stream_end

    def test_sequence(self):
        yaml = "- one\n- two\n- three\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        scalars = [e["value"] for e in docs[0] if e["type"] == "scalar"]
        assert scalars == ["one", "two", "three"]

    def test_nested_mapping(self):
        yaml = "a:\n  b: 1\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        scalars = [e["value"] for e in docs[0] if e["type"] == "scalar"]
        assert "a" in scalars
        assert "b" in scalars
        assert "1" in scalars


class TestStreamEventFields:
    """Binding-layer event dict field stability (stream_events.rs)."""

    def test_scalar_event_fields(self):
        yaml = "hello\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        scalars = [e for e in events if e["type"] == "scalar"]
        assert len(scalars) == 1
        ev = scalars[0]
        assert ev["value"] == "hello"
        assert ev["style"] in ("plain", "single-quoted", "double-quoted")
        assert isinstance(ev["line"], int)
        assert isinstance(ev["column"], int)
        assert ev["anchor"] is None
        assert ev["tag"] is None

    def test_mapping_event_fields(self):
        yaml = "k: v\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        maps = [e for e in events if e["type"] == "mapping_start"]
        assert len(maps) == 1
        ev = maps[0]
        assert ev["value"] is None
        assert ev["style"] is None
        assert isinstance(ev["line"], int)
        assert isinstance(ev["column"], int)

    def test_sequence_event_fields(self):
        yaml = "- 1\n- 2\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        seqs = [e for e in events if e["type"] == "sequence_start"]
        assert len(seqs) == 1
        ev = seqs[0]
        assert ev["value"] is None
        assert ev["anchor"] is None
        assert ev["tag"] is None

    def test_anchor_tag_propagates(self):
        yaml = "x: &myanchor !tag hello\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        scalars = [e for e in events if e["type"] == "scalar"]
        assert len(scalars) == 2  # key= x, value= hello
        tagged = [e for e in scalars if e["tag"] is not None]
        assert len(tagged) == 1
        ev = tagged[0]
        assert ev["value"] == "hello"
        # Anchor names are auto-numbered by the parser; just verify propagation.
        assert ev["anchor"] is not None
        assert ev["tag"] == "!tag"

    def test_flow_anchors_are_named_from_their_own_node(self):
        """The event reader named a node after an `&` written inside its neighbour.

        `$,"` is a plain key, so its comma is the key's own text - read as a member separator it opens
        a quoted region at the quote after it and skews every quote state on the line: the `#` in the
        next member's quoted key reads as a comment, that member's own anchor is skipped as comment
        text, and the `&` written *inside* the key becomes the name. Both readers now bound the search
        by the node's own window - quality-ledger (dr).
        """
        src = '$,": {&k "<-&w #q": &v "x"}'
        events = list(pyrs_yaml.parse_stream(src))
        named = {e["value"]: e["anchor"] for e in events if e["type"] == "scalar"}
        assert named["<-&w #q"] == "k"
        assert named["x"] == "v", "not the `&` written inside the preceding key"
        emitted = pyrs_yaml.parse(src).to_yaml()
        assert "&k" in emitted and "&v" in emitted

    def test_multiple_document_event_structure(self):
        yaml = "---\na: 1\n---\nb: 2\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        types = [e["type"] for e in events]
        # stream_start → document_start → scalar/mapping → document_end → ...
        doc_starts = [i for i, t in enumerate(types) if t == "document_start"]
        doc_ends = [i for i, t in enumerate(types) if t == "document_end"]
        assert len(doc_starts) == 2
        assert len(doc_ends) == 2
        assert types[-1] == "stream_end"


class TestByteRanges:
    """`offset`/`end`: which source bytes an event names.

    The range is the event's own token, not its subtree, and a producer that records only a
    position answers `end == offset` rather than inventing an extent -- TOML's IR keeps starts,
    and YAML's block containers and stream markers are positions. A document marker covers
    `---`/`...` when the source wrote one and is a position when it did not.
    """

    @pytest.mark.parametrize(
        "text,fmt",
        [
            ("a: 1\nb: [x, 'y']\n# note\nc: |\n  body\n", "yaml"),
            ("日本語: 🎉\n", "yaml"),
            ('{\n  "a": [1, true, null],\n  "b": "s"\n}\n', "json"),
            ('{\n  // head\n  "a": 1, /* tail */\n  "b": 2\n}\n', "jsonc"),
            ("{名字: '值', n: .5}\n", "json5"),
            ("# head\n[t]\nx = 1\ny = [2]\n", "toml"),
        ],
    )
    def test_every_range_is_inside_the_source_and_never_reversed(self, text, fmt):
        source = text.encode()
        events = list(pyrs_yaml.parse_stream(text, format=fmt))
        assert events
        for event in events:
            assert 0 <= event["offset"] <= event["end"] <= len(source), event
            # Decoding proves both endpoints sit on character boundaries, so a consumer can slice
            # the source at them even when the line holds wide characters.
            source[event["offset"] : event["end"]].decode()

    @pytest.mark.parametrize(
        "text,fmt,value,token",
        [
            ("k: 'y'\n", "yaml", "y", "'y'"),
            ('k: "y"\n', "yaml", "y", '"y"'),
            ("k: |\n  body\n", "yaml", "body\n", "body\n"),
            ('{"a": "v"}', "json", "v", '"v"'),
            ("{a: 'v'}", "json5", "v", "'v'"),
            ('{"a": 1}', "json", "1", "1"),
        ],
    )
    def test_a_scalar_names_its_own_token_not_its_decoded_value(self, text, fmt, value, token):
        # A quoted scalar's range covers the quotes the source typed; the `value` is what the
        # reader resolved. Both facts are needed to re-read the token without the tree.
        events = list(pyrs_yaml.parse_stream(text, format=fmt))
        scalar = next(e for e in events if e["type"] == "scalar" and e["value"] == value)
        assert text.encode()[scalar["offset"] : scalar["end"]] == token.encode()

    def test_toml_names_each_token_start_and_says_it_has_no_end(self):
        # The IR records where a key or value begins and nothing about where it stops, so the
        # producer answers `end == offset`. The starts are pinned because they are the one thing
        # this route can promise, and a projection that lost them would have nothing left.
        text = 'a = 1\n"lit key" = "basic"\nn = 0b101\n'
        events = list(pyrs_yaml.parse_stream(text, format="toml"))
        assert [(e["value"], e["offset"]) for e in events if e["type"] == "scalar"] == [
            ("a", 0),
            ("1", 4),
            ("lit key", 6),
            ("basic", 18),
            ("n", 26),
            ("0b101", 30),
        ]
        assert all(e["end"] == e["offset"] for e in events)

    def test_a_flow_delimiter_names_its_own_byte(self):
        # A flow container is a real byte on the line; the block form is a position only, because
        # no character opens it. The asymmetry is the producers' own, and this is where it is
        # written down rather than discovered by a consumer.
        text = '{"a": [1]}'
        named = [
            (e["type"], text.encode()[e["offset"] : e["end"]].decode())
            for e in pyrs_yaml.parse_stream(text, format="json")
            if e["type"] in ("mapping_start", "mapping_end", "sequence_start", "sequence_end")
        ]
        assert named == [
            ("mapping_start", "{"),
            ("sequence_start", "["),
            ("sequence_end", "]"),
            ("mapping_end", "}"),
        ]
        block = list(pyrs_yaml.parse_stream("a:\n  b: 1\n", format="yaml"))
        for event in block:
            if event["type"].startswith(("mapping_", "stream_", "document_")):
                assert event["end"] == event["offset"], event

    def test_a_comment_range_covers_its_delimiters(self):
        # The note's text is stripped of them, so the range is the only place a consumer can read
        # what the source actually wrote.
        text = '{\n  // head\n  "a": 1, /* tail */\n  "b": 2\n}\n'
        got = [
            text.encode()[e["offset"] : e["end"]].decode()
            for e in pyrs_yaml.parse_stream(text, format="jsonc")
            if e["type"] == "comment"
        ]
        assert got == ["// head", "/* tail */"]
        yaml_text = "# note\na: 1\n"
        note = next(e for e in pyrs_yaml.parse_stream(yaml_text, format="yaml") if e["type"] == "comment")
        assert yaml_text.encode()[note["offset"] : note["end"]] == b"# note"

    def test_a_document_marker_covers_the_marker_the_source_wrote(self):
        # The markers that stand for a boundary rather than a character are empty ranges; the two
        # that an author may write are exactly their three bytes. A consumer restoring a document
        # from these ranges needs both facts.
        for text, document_start, document_end in [("---\na: 1\n", "---", ""), ("a: 1\n...\n", "", "...")]:
            ranges = {
                e["type"]: text.encode()[e["offset"] : e["end"]].decode()
                for e in pyrs_yaml.parse_stream(text, format="yaml")
                if e["type"].startswith(("document_", "stream_"))
            }
            assert ranges == {
                "stream_start": "",
                "document_start": document_start,
                "document_end": document_end,
                "stream_end": "",
            }, text

    def test_the_incremental_reader_names_the_same_bytes_as_the_text_route(self):
        # `YamlStream` keeps no copy of the source, so it rebuilds byte offsets from a sparse
        # char index while the text route converts granit's char spans against its line table.
        # A wide character is the only input where the two can disagree, and a one-byte-at-a-time
        # feed is where a chunk boundary can split one.
        class Trickle:
            def __init__(self, text):
                self.data = text.encode()

            def read(self, _size):
                chunk, self.data = self.data[:1], self.data[1:]
                return chunk

        for text in [
            "日本語: [🎉, 1]\n",
            "a: 🎉\n日本語: x\n",
            # A second document and a flow member after it: the reader forgets the marks behind
            # each event, so a document boundary is where a wrong base would surface.
            "a: 🎉\n---\n棒: [🎉, 2]\n",
        ]:
            as_text = list(pyrs_yaml.parse_stream(text, format="yaml"))
            for source in (io.StringIO(text), Trickle(text)):
                as_stream = list(pyrs_yaml.YAML().load_stream(source))
                assert as_stream == as_text, type(source).__name__
            assert any(e["offset"] for e in as_text)


class TestBlockScalarStyle:
    """stream_events style field for block scalars (audit §15)."""

    def test_literal_style_field(self):
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO("k: |\n  hello\n")))
        scalars = [e for e in events if e["type"] == "scalar" and e["value"] != "k"]
        assert len(scalars) == 1
        assert scalars[0]["value"] == "hello\n"
        assert scalars[0]["style"] == "literal"

    def test_folded_style_field(self):
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO("k: >\n  hello\n")))
        scalars = [e for e in events if e["type"] == "scalar" and e["value"] != "k"]
        assert len(scalars) == 1
        assert scalars[0]["value"] == "hello\n"
        assert scalars[0]["style"] == "folded"


def _skel(text, fmt):
    """The `(type, value, style)` skeleton of a stream, positions left out on purpose."""
    return [(event["type"], event["value"], event["style"]) for event in pyrs_yaml.parse_stream(text, format=fmt)]


def _notes(text, fmt):
    """The `(text, style, owner)` of every note a stream reports."""
    return [
        (event["value"], event["style"], event["owner"])
        for event in pyrs_yaml.parse_stream(text, format=fmt)
        if event["type"] == "comment"
    ]


class TestMultiFormatStream:
    """`parse_stream(format=...)`: the one vocabulary, produced per format.

    These are the Python-side half of plan B's last step -- `docs/dev/event-stream.md`
    "What the replay gate answered" names what the vocabulary can and cannot say, and the
    Rust tier (`crates/pyrs-json/tests/events_replay.rs`) is where a stream is checked
    against the tree. What is pinned here is the *boundary*: which producer answers, what a
    projection of a foreign event dict looks like, and that the default stays YAML.
    """

    def test_json_events_keep_the_quote_the_source_typed(self):
        # A JSON key and a JSON string value are quoted where a JSON number is not, so the
        # stream can tell `{"a": "1"}` from `{"a": 1}` -- the tree loses that on the way to
        # YAML, where both become the same plain scalar.
        assert _skel('{"a": 1}', "json") == [
            ("stream_start", None, None),
            ("document_start", None, None),
            ("mapping_start", None, None),
            ("scalar", "a", "double_quoted"),
            ("scalar", "1", "plain"),
            ("mapping_end", None, None),
            ("document_end", None, None),
            ("stream_end", None, None),
        ]

    def test_json_nesting_shape(self):
        assert [t for t, _v, _s in _skel('{"b": [2, {"c": "x"}]}', "json")] == [
            "stream_start",
            "document_start",
            "mapping_start",
            "scalar",
            "sequence_start",
            "scalar",
            "mapping_start",
            "scalar",
            "scalar",
            "mapping_end",
            "sequence_end",
            "mapping_end",
            "document_end",
            "stream_end",
        ]

    def test_positions_come_from_the_producer(self):
        events = list(pyrs_yaml.parse_stream('{\n  "a": 1\n}', format="json"))
        key = next(e for e in events if e["value"] == "a")
        assert (key["line"], key["column"]) == (1, 2)

    @pytest.mark.parametrize(
        "text,fmt",
        [
            ("日本語: 1\n", "yaml"),
            ('"🎉": 1\n', "yaml"),
            ('{"日本語": 1}\n', "json"),
            ('{"🎉": 1}\n', "json"),
            ('"日本語" = 1\n', "toml"),
            ('"🎉" = 1\n', "toml"),
        ],
    )
    def test_a_wide_key_costs_its_characters_not_its_bytes(self, text, fmt):
        # Three producers, one convention: a column counts characters. JSON and TOML convert a byte
        # offset with `pyrs_event::LineTable`, YAML reports the column granit's own marker carries,
        # and the two conversions live in different crates -- so nothing but these cases asks all
        # three the same question. A producer that started counting bytes would read +2 per CJK
        # glyph and +3 per emoji, and every consumer of the stream would see a position that no
        # longer matches the diagnostic the same engine prints. Parametrised rather than looped so
        # one run names every producer that disagrees, not only the first.
        events = list(pyrs_yaml.parse_stream(text, format=fmt))
        value = next(e for e in events if e["type"] == "scalar" and e["value"] == "1")
        assert (value["line"], value["column"]) == (0, text.splitlines()[0].index("1"))

    def test_jsonc_notes_ride_the_stream(self):
        skeleton = _skel('{\n  // head\n  "a": 1, /* tail */\n  "b": 2\n}\n', "jsonc")
        assert ("comment", "head", "standalone") in skeleton
        assert ("comment", "tail", "inline") in skeleton

    @pytest.mark.parametrize(
        "text,fmt,expected",
        [
            ("# above\na: 1\n", "yaml", ("above", "standalone", "following_leading")),
            ("a: 1 # on the value\n", "yaml", ("on the value", "inline", "preceding_inline")),
            ("a: 1\n# trailing\n", "yaml", ("trailing", "standalone", "preceding_inline")),
            ('{"a": 1 /* tail */}', "jsonc", ("tail", "inline", "preceding_inline")),
            ("# above\na = 1\n", "toml", ("above", "standalone", "following_leading")),
            ("a = 1 # tail\n", "toml", ("tail", "inline", "preceding_inline")),
        ],
    )
    def test_a_note_reports_its_source_fact_and_its_owner(self, text, fmt, expected):
        # One note, two facts, and the boundary names both: `style` answers the source's question
        # (did the note have a line to itself) and `owner` answers the tree's (which node the note
        # is filed under, and in which of its two slots). The three producers answer the source
        # question the same way -- only blanks before the `#` mean the note had a line of its own --
        # so the flag is one word across the dialects and the owner is what tells them apart.
        assert _notes(text, fmt) == [expected]

    def test_the_one_value_that_moved_is_the_shared_line_leading_note(self):
        # TOML's old flag answered the slot question ("is this note in the leading list?"), so a
        # note that shared its line with the text before it but still leads the next element used
        # to read `standalone`. It now reads `inline` -- the source's own answer, which the other
        # two producers always gave -- and `owner` carries the placement the old value carried.
        # This is the one shipped Python value the split moves, and it is pinned so a future
        # change to either half of the shape is a red run rather than a silent drift.
        assert _notes("a = { x = 1, # shared\n  y = 2 }\n", "toml") == [("shared", "inline", "following_leading")]

    def test_the_owner_key_is_only_on_a_comment(self):
        # The vocabulary is one shape with a per-event field: the other seven keys are untouched,
        # and a consumer that reads `owner` off a scalar would find nothing.
        events = list(pyrs_yaml.parse_stream("# above\na: 1\n", format="yaml"))
        assert [e["type"] for e in events if "owner" in e] == ["comment"]
        assert set(events[0]) == {
            "type",
            "value",
            "style",
            "anchor",
            "tag",
            "line",
            "column",
            "offset",
            "end",
        }

    def test_json5_reports_single_quoted_and_bare_forms_verbatim(self):
        # The number is the source's own spelling, not a canonicalised one: `.5` stays `.5`.
        assert _skel("{name: 'Alice', n: .5,}", "json5") == [
            ("stream_start", None, None),
            ("document_start", None, None),
            ("mapping_start", None, None),
            ("scalar", "name", "plain"),
            ("scalar", "Alice", "single_quoted"),
            ("scalar", "n", "plain"),
            ("scalar", ".5", "plain"),
            ("mapping_end", None, None),
            ("document_end", None, None),
            ("stream_end", None, None),
        ]

    def test_toml_quoting_reports_what_the_ir_keeps(self):
        # Which quote a TOML string used is not in the intermediate representation, so both forms
        # arrive alike; a bare key arrives plain unless YAML's own reader would type it, in which
        # case it is marked quoted so the projection re-reads as the same string.
        scalars = [
            (e["value"], e["style"])
            for e in pyrs_yaml.parse_stream("a = 'lit'\nb = \"basic\"\ny = 1\n", format="toml")
            if e["type"] == "scalar"
        ]
        assert scalars == [
            ("a", "plain"),
            ("lit", "double_quoted"),
            ("b", "plain"),
            ("basic", "double_quoted"),
            ("y", "double_quoted"),
            ("1", "plain"),
        ]

    def test_toml_tags_are_the_hub_tags_not_yamls_own(self):
        tagged = {
            (e["value"], e["tag"])
            for e in pyrs_yaml.parse_stream("d = 1979-05-27T07:32:00Z\nb = 0b101\n", format="toml")
            if e["type"] == "scalar"
        }
        assert ("1979-05-27T07:32:00Z", "!!timestamp") in tagged
        assert ("0b101", "!!int") in tagged

    def test_toml_table_headers_open_in_structural_order(self):
        # A header's `[` sits before the name it declares, so the key event precedes the
        # MappingStart that the header opens -- the order the tree has and byte order does not.
        types = [t for t, _v, _s in _skel("[s]\nx = 1\n", "toml")]
        assert types.index("scalar") < types.index("mapping_end")
        body = _skel("[s]\nx = 1\n", "toml")
        assert body[body.index(("scalar", "s", "plain")) + 1] == ("mapping_start", None, None)

    def test_anchor_and_alias_are_yamls_own(self):
        for fmt, text in [("json", "{}"), ("jsonc", "{}"), ("json5", "{}"), ("toml", "x = 1\n")]:
            events = list(pyrs_yaml.parse_stream(text, format=fmt))
            assert all(e["anchor"] is None for e in events), fmt
            assert "alias" not in [e["type"] for e in events], fmt
        anchored = list(pyrs_yaml.parse_stream("a: &n 1\nb: *n\n", format="yaml"))
        assert any(e["anchor"] == "n" for e in anchored)
        assert "alias" in [e["type"] for e in anchored]

    def test_default_format_stays_yaml(self):
        text = "a: 1\nb: [2, {c: x}]\n"
        assert list(pyrs_yaml.parse_stream(text)) == list(pyrs_yaml.parse_stream(text, format="yaml"))
        # The three shipped positional arguments keep working unchanged.
        assert list(pyrs_yaml.parse_stream(text, None, 1000)) == list(pyrs_yaml.parse_stream(text))

    def test_bytes_are_accepted_for_every_format(self):
        for fmt, text in [("json", b'{"a": 1}'), ("toml", b"x = 1\n"), ("yaml", b"a: 1\n")]:
            kinds = [e["type"] for e in pyrs_yaml.parse_stream(text, format=fmt)]
            assert "scalar" in kinds, fmt

    def test_max_depth_bounds_the_readers_that_carry_it(self):
        # JSON has a nesting limit of its own; the YAML reader has one too, and it answers with
        # the dedicated depth type. TOML's limit is the IR's, so `max_depth` is documented as
        # applying to the two readers that take the option.
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.parse_stream("[[[1]]]", max_depth=2, format="json"))
        with pytest.raises(pyrs_yaml.YamlMaxDepthError):
            list(pyrs_yaml.parse_stream("a: [[1]]", max_depth=2, format="yaml"))
        assert any(e["type"] == "scalar" for e in pyrs_yaml.parse_stream("[[[1]]]", format="json"))

    def test_the_dialects_stay_their_own_grammars(self):
        for fmt, text in [
            ("json", '{"a": 1 // note\n}'),
            ("json", "{'a': 1}"),
            ("json", '{"a": 1,}'),
            ("jsonc", "{a: 1}"),
            ("jsonc", "{'a': 1}"),
        ]:
            with pytest.raises(pyrs_yaml.YamlParseError):
                list(pyrs_yaml.parse_stream(text, format=fmt))
        assert _skel('{\n  // ok\n  "a": 1\n}', "jsonc")[3] == ("comment", "ok", "standalone")
        assert ("scalar", "Alice", "single_quoted") in _skel("{a: 'Alice'}", "json5")

    def test_a_refusal_raises_what_the_tree_route_raises(self):
        # One scanner answers both walks: the boundary must not invent a second verdict text.
        for route, text in [(pyrs_yaml.load_json, "{"), (pyrs_yaml.load_toml, "x = ")]:
            fmt = "json" if route is pyrs_yaml.load_json else "toml"
            with pytest.raises(pyrs_yaml.YamlParseError) as tree:
                route(text)
            with pytest.raises(pyrs_yaml.YamlParseError) as stream:
                list(pyrs_yaml.parse_stream(text, format=fmt))
            assert str(tree.value) == str(stream.value)

    def test_an_unknown_format_is_a_type_error_that_names_the_set(self):
        with pytest.raises(pyrs_yaml.YamlTypeError, match="Supported: yaml, json, jsonc, json5, toml"):
            pyrs_yaml.parse_stream("{}", format="xml")

    def test_the_callback_branch_sees_the_same_events(self):
        seen = []
        out = pyrs_yaml.parse_stream('{"a": 1}', on_event=lambda e: seen.append(e) or True, format="json")
        assert out is None
        assert [e["type"] for e in seen] == [t for t, _v, _s in _skel('{"a": 1}', "json")]

    def test_a_false_answer_stops_the_callback(self):
        seen = []
        pyrs_yaml.parse_stream('{"a": 1}', on_event=lambda e: seen.append(1) or False, format="json")
        assert len(seen) == 1

    def test_a_stopped_json_scan_never_reaches_the_error_after_it(self):
        # The callback runs while the scanner walks, so `False` stops the walk: `oops` is past the
        # stop and is never read. Without the stop the same document is refused, which is what
        # makes this a claim about the scan rather than about delivery.
        text = '{"a": 1, "b": oops}'
        seen = []
        out = pyrs_yaml.parse_stream(text, on_event=lambda e: seen.append(e) or len(seen) < 5, format="json")
        assert out is None
        assert [e["type"] for e in seen] == [
            "stream_start",
            "document_start",
            "mapping_start",
            "scalar",
            "scalar",
        ]
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.parse_stream(text, format="json"))

    def test_a_refused_toml_document_reaches_no_callback(self):
        # TOML projects events from the tree its scan built, so the verdict arrives first: a stop
        # cannot hide a refusal the document route would also make.
        seen = []
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse_stream("x = ", on_event=lambda e: seen.append(e) or False, format="toml")
        assert seen == []

    def test_a_callback_error_outranks_the_parse_error_behind_it(self):
        class StopError(Exception):
            pass

        def refuse_on_the_value(event):
            if event["type"] == "scalar" and event["value"] == "1":
                raise StopError("asked to stop")
            return True

        with pytest.raises(StopError):
            pyrs_yaml.parse_stream('{"a": 1, "b": oops}', on_event=refuse_on_the_value, format="json")

    def test_the_callback_sees_exactly_what_the_iterator_collects(self):
        # A long document, so a delivery that dropped or repeated events would show up as a
        # shorter or longer list than the collecting route's.
        text = "[" + ",".join(str(i) for i in range(80)) + "]"
        collected = list(pyrs_yaml.parse_stream(text, format="json"))
        seen = []
        pyrs_yaml.parse_stream(text, on_event=lambda e: seen.append(e) or True, format="json")
        assert len(collected) > 64
        assert seen == collected

    def test_a_stop_keeps_the_prefix_and_drops_the_rest(self):
        text = "[" + ",".join(str(i) for i in range(100)) + "]"
        full = list(pyrs_yaml.parse_stream(text, format="json"))
        seen = []
        pyrs_yaml.parse_stream(text, on_event=lambda e: seen.append(e) or len(seen) < 70, format="json")
        assert seen == full[:70]

    def test_the_toml_callback_gets_the_collected_stream(self):
        text = "\n".join(f"k{i} = {i}" for i in range(70)) + "\n"
        collected = list(pyrs_yaml.parse_stream(text, format="toml"))
        seen = []
        pyrs_yaml.parse_stream(text, on_event=lambda e: seen.append(e) or True, format="toml")
        assert len(collected) > 64
        assert seen == collected

    def test_the_yaml_text_producer_cannot_stop_its_scan(self):
        # The documented boundary: granit's push receiver is not abortable, so the document is
        # parsed in full before the first event is delivered and a refusal wins over the callback.
        seen = []
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse_stream("a: [", on_event=lambda e: seen.append(e) or False, format="yaml")
        assert seen == []
