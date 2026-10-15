"""Cross-library correctness parity of the YAML engines against PyYAML and
ruamel.yaml.

Objective §测试覆盖 3 names `PyYAML` and `ruamel.yaml` as oracle targets.
Previously both libraries only appeared in `test_benchmark_crosslib.py`
(timed runs and a feature-support printout at L400-445), never as an
assertion that pyrs and the reference agree on the *value* produced for
a given YAML document. This module closes that gap: it pins the
canonical YAML Core 1.2 domain where the three engines must produce the
same Python object graph.

Deliberate divergences (YAML 1.1 legacy booleans, duplicate-key
strictness, comment/anchor preservation) are asserted explicitly and
documented rather than hidden -- see the divergence section at the
bottom. Optional deps guarded by `skipif`; suite runs cleanly without
them.
"""

from __future__ import annotations

import io

import pytest

import pyrs_yaml

try:
    import yaml as pyyaml
except ImportError:  # pragma: no cover - optional
    pyyaml = None

try:
    import ruamel.yaml as _ruamel
except ImportError:  # pragma: no cover - optional
    _ruamel = None


def _ruamel_available() -> bool:
    return _ruamel is not None


def _ruamel_safe(src: str):
    """Load via ruamel in safe-pure mode (plain dicts, no CommentedMap
    wrapper types), so equality against pyrs / PyYAML is direct."""
    y = _ruamel.YAML(typ="safe", pure=True)
    return y.load(src)


# ── Canonical YAML Core 1.2 domain ────────────────────────────────────────
# Every document here is well-defined under YAML 1.2 Core, PyYAML's
# safe_load, and ruamel's typ="safe" reader. All three engines must
# agree on the produced Python object graph.

CANONICAL_YAML_DOCS: list[str] = [
    # Scalars of each Core schema type.
    "a: 1\n",
    "a: -42\n",
    "a: 3.14\n",
    "a: true\n",
    "a: false\n",
    "a: null\n",
    "a: ~\n",
    "a: hello\n",
    'a: "hello world"\n',
    "a: 'single quoted'\n",
    # Sequences and mappings.
    "a: [1, 2, 3]\n",
    "a:\n  - 1\n  - 2\n  - 3\n",
    "a:\n  b: 1\n  c: 2\n",
    "a: {b: 1, c: 2}\n",
    # Nesting.
    "a:\n  b:\n    c: [1, {d: 2}, null]\n",
    # Multiple top-level keys.
    "a: 1\nb: two\nc: 3.5\nd: true\ne: null\n",
    # Empty containers.
    "a: []\nb: {}\n",
    # Unicode strings.
    'a: "café \U0001f600"\n',
    # Multi-line block scalars (values only; PyYAML/ruamel/pyrs must all
    # resolve them to the identical string).
    "a: |\n  line1\n  line2\n",
    "a: >\n  folded\n  text\n",
]


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_matches_pyyaml_safe_load(src):
    """pyrs.safe_load and PyYAML.safe_load produce identical Python values
    on the canonical YAML Core 1.2 domain."""
    assert pyrs_yaml.safe_load(src) == pyyaml.safe_load(src)


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_matches_ruamel_safe_load(src):
    """pyrs.safe_load and ruamel.safe_load (typ=safe, pure=True) produce
    identical Python values."""
    assert pyrs_yaml.safe_load(src) == _ruamel_safe(src)


@pytest.mark.skipif(pyyaml is None or not _ruamel_available(), reason="oracle missing")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_three_way_canonical_yaml_agreement(src):
    """Three-way check: pyrs, PyYAML, ruamel -- all agree byte-for-byte on
    the canonical domain. Any drift shows up on the pair that diverges."""
    p = pyrs_yaml.safe_load(src)
    y = pyyaml.safe_load(src)
    r = _ruamel_safe(src)
    assert p == y == r


# ── Round-trip dump parity ────────────────────────────────────────────────
# Every engine re-loads pyrs.safe_dump's output back to the same object,
# and vice-versa: pyrs re-loads PyYAML's / ruamel's dumps.


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_loads_what_pyyaml_dumps(src):
    data = pyyaml.safe_load(src)
    dumped = pyyaml.safe_dump(data, default_flow_style=False)
    assert pyrs_yaml.safe_load(dumped) == data


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_loads_what_ruamel_dumps(src):
    data = _ruamel_safe(src)
    buf = io.StringIO()
    y = _ruamel.YAML(typ="safe", pure=True)
    y.dump(data, buf)
    assert pyrs_yaml.safe_load(buf.getvalue()) == data


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyyaml_loads_what_pyrs_dumps(src):
    data = pyrs_yaml.safe_load(src)
    dumped = pyrs_yaml.safe_dump(data)
    assert pyyaml.safe_load(dumped) == data


# ── Documented deliberate divergences ────────────────────────────────────


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.parametrize(
    ("src", "key"),
    [
        pytest.param("? [1, 2]\n: v\n", (1, 2), id="sequence-key"),
        pytest.param('? [1, "x"]\n: v\n', (1, "x"), id="mixed-sequence-key"),
    ],
)
def test_sequence_keys_agree_with_ruamel(src, key):
    """A sequence in key position projects to the same tuple ruamel.yaml gives.

    Measured rather than assumed on 2026-10-14: ruamel's safe loader converts the outer
    container only, and this engine matches it item for item.
    """
    ours = pyrs_yaml.safe_load(src)
    theirs = _ruamel.YAML(typ="safe").load(io.StringIO(src))
    assert list(ours.keys()) == list(theirs.keys()) == [key]
    assert ours == theirs


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
def test_ruamel_reads_the_alias_key_we_write():
    """The alias-key emission is written for other readers, not only for our own round trip.

    Both ruamel versions in `uv.lock` — 0.18.16 for the Python 3.8 legs, 0.19.1 for the rest — load
    what we write and hand back an empty tuple for an alias key, which is the same reason this
    engine refuses the dict projection instead of inventing one (see
    `test_a_key_with_no_hashable_form_is_refused_not_replaced`).

    The compact `*x: v` is not the same story across versions, and that was measured after this
    test reddened the 3.8 legs with a blanket claim: 0.18.16 reads it as an empty-tuple key,
    0.19.1's scanner raises `while scanning a simple key`, and this engine's own reader has always
    refused it (`simple key expected ':'`). The refusal below is therefore scoped to the versions
    that have it; the durable fact is that only the marker form reads alike everywhere.
    """
    ours = pyrs_yaml.parse("a: &x [1, 2]\n? *x\n: v\n").to_yaml()
    theirs = _ruamel.YAML(typ="safe").load(io.StringIO(ours))
    assert list(theirs.keys()) == ["a", ()]

    ours_flow = pyrs_yaml.parse("{a: &x [1, 2], ? *x : v}\n").to_yaml()
    theirs_flow = _ruamel.YAML(typ="safe").load(io.StringIO(ours_flow))
    assert list(theirs_flow.keys()) == ["a", ()]

    ruamel_minor = tuple(int(part) for part in _ruamel.__version__.split(".")[:2])
    if ruamel_minor >= (0, 19):
        with pytest.raises(_ruamel.scanner.ScannerError):
            _ruamel.YAML(typ="safe").load(io.StringIO("a: &x [1, 2]\n*x: v\n"))


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
def test_ruamel_reads_the_flow_member_notes_we_write():
    """The flow-note emission is written for other readers, and the two gaps are named separately.

    Measured against the ruamel in `uv.lock`: a value's or an item's note survives its own
    round trip here (`{A: x  # V\n }` loads and re-dumps with `# V`), which is the shape this
    engine's writer used to drop. A *key's* note does not — ruamel reads `{A: # K\n x}` and writes
    `{A: x}` — so the slot we now fill is one ruamel leaves empty, and the assertion below pins
    that asymmetry rather than pretending the libraries agree. PyYAML is the syntax check: it
    parses every emission here without a scanner error.
    """
    for src in ["{A: x  # V\n}\n", "[x  # V\n]\n", "{A: # K\n  x}\n", "{? {} : v}\n"]:
        emitted = pyrs_yaml.parse(src).to_yaml()
        loaded = _ruamel.YAML(typ="rt").load(io.StringIO(emitted))
        buf = io.StringIO()
        _ruamel.YAML(typ="rt").dump(loaded, buf)
        assert loaded is not None, src
        if "# K" in emitted:
            assert "# K" not in buf.getvalue(), "ruamel now keeps key notes: update this claim"
        elif "# V" in emitted:
            assert "# V" in buf.getvalue(), f"ruamel dropped the note: {emitted!r} -> {buf.getvalue()!r}"


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_pyyaml_accepts_every_flow_note_emission():
    for src in ["{A: x  # V\n}\n", "{A: # K\n  x}\n", "[x  # V\n]\n", "{? {} : v}\n", "{? [1, 2] : v}\n"]:
        emitted = pyrs_yaml.parse(src).to_yaml()
        list(pyyaml.parse(emitted))


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_the_references_read_the_block_bodies_we_write():
    """A block scalar's body keeps its bytes when its header moves line or takes a note.

    Two emissions used to change the value they carried, for every reader alike:

    * an indentation indicator counts columns from the *parent node*, so a header pushed
      onto its own line by a leading note still measures from the pair line — laying the
      body under the header instead made the re-read value two spaces longer;
    * a simple key's note has no slot before its `:`, and "ride the end of the line"
      meant the end of the block's *last body line*, where `#` is content, not comment.

    The claim pinned here is the one a serializer has to be able to make: the document we
    hand back decodes, in all three engines, to the value the tree held.
    """
    for src in ["- \n  a:\n    # A\n    |-2\n       \n", "? a  # n\n: |-\n  abc\n"]:
        emitted = pyrs_yaml.parse(src).to_yaml()
        ours = pyrs_yaml.safe_load(emitted)
        assert ours == pyrs_yaml.safe_load(src), src
        assert ours == pyyaml.safe_load(emitted), emitted
        assert ours == _ruamel_safe(emitted), emitted


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_pyrs_projects_a_sequence_key_where_pyyaml_refuses_it():
    """Deliberate divergence, toward the library that carries the data.

    PyYAML constructs the key as a list and then `dict` refuses it, so
    `? [1, 2]` is unreadable there even though the tuple form is hashable and
    ruamel returns it. Producing `{(1, 2): 'v'}` loses nothing and is the
    stricter-to-nothing choice, so it is pinned here instead of being left to
    an argument.
    """
    with pytest.raises(pyyaml.constructor.ConstructorError):
        pyyaml.safe_load("? [1, 2]\n: v\n")
    assert pyrs_yaml.safe_load("? [1, 2]\n: v\n") == {(1, 2): "v"}


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize(
    "src",
    [
        pytest.param("? {a: 1}\n: v\n", id="mapping-key"),
        pytest.param("? ?\n", id="empty-nested-key"),
    ],
)
def test_a_mapping_key_is_refused_by_both_engines(src):
    """Neither engine invents a key for a nested mapping; both refuse at the same boundary.

    pyrs used to substitute the node's debug text, and because that stand-in is an ordinary
    string it re-emitted, so the change reached the next document as if it had been intended.
    """
    with pytest.raises(pyyaml.constructor.ConstructorError):
        pyyaml.safe_load(src)
    with pytest.raises(TypeError, match="unhashable key"):
        pyrs_yaml.safe_load(src)


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_duplicate_keys_are_deliberately_rejected_by_pyrs():
    """Objective §Known Engine Boundaries / v0.11.5 audit: pyrs rejects
    duplicate keys where PyYAML silently accepts (last wins). Pinned as
    a spec-strictness choice, not a bug."""
    src = "a: 1\na: 2\n"
    assert pyyaml.safe_load(src) == {"a": 2}  # PyYAML lenient
    with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
        pyrs_yaml.safe_load(src)


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_yaml_11_legacy_booleans_are_schema_scoped():
    """YAML 1.1 legacy booleans (`yes` / `on` / `off`) resolve to `bool`
    under the `yaml1.1` schema; YAML 1.2 Core keeps them as strings.
    PyYAML follows the 1.1 convention on its `safe_load`. Test pins that
    pyrs's schema selection is intentional and the string-vs-bool
    divergence is documented."""
    src = "a: yes\n"
    assert pyyaml.safe_load(src) is not None  # PyYAML gives True
    assert pyyaml.safe_load(src)["a"] is True
    # pyrs Core: `yes` is a plain string, matching YAML 1.2.
    assert pyrs_yaml.safe_load(src)["a"] == "yes"
    # pyrs yaml1.1 schema: resolves to bool like PyYAML.
    assert pyrs_yaml.safe_load(src, schema="yaml1.1")["a"] is True
