"""The CSV corpus is Python's own `csv` module's canonical form.

The Rust engine's byte-for-byte claim is only worth something if the corpus
it is measured against is a form two independent implementations agree on.
Python's standard library `csv` module is that second implementation, and it
is already installed everywhere - so the corpus is *generated* by it rather
than hand-written, and this file re-generates every fixture and compares it
with the committed bytes.

That makes this file the live oracle for `crates/pyrs-csv/tests/fixtures/`:
the Rust test (`crates/pyrs-csv/tests/roundtrip_corpus.rs`) consumes the
same committed files, so the two halves meet on the same bytes - Python says
this is what the file must contain, and Rust says it can read the file and
write it back unchanged.

Three comparisons live here, and the last is the one the corpus exists for:

- The committed bytes are Python's `csv` module's own output, byte for byte.
- The *engine's* parse of every fixture is compared with the standard
  library's, cell by cell. Bytes that a fixture happens to hold prove only
  that the fixture was written correctly; what the engine *reads out of it*
  is the claim that can actually break, and it is checked here against an
  implementation that shares no code with this one.
- The round trip through the YAML hub - `from_csv` and then `to_csv` - is
  compared with the bytes the file was written with, for every fixture
  whose bytes are supposed to survive it.

Two fixtures in the corpus are refusals rather than round trips: a ragged
row and a header that names the same column twice. Python's `csv` module
writes them happily (it validates nothing), so they are still generated
here - and the Rust side is the one that has to refuse them. The cell-by-
cell comparison reads them in the no-header mode, where there is no header
to be ragged against and both readers see the same grid; the refusals are
pinned separately, by `crates/pyrs-csv/tests/roundtrip_corpus.rs` and by
`tests/test_csv_spec_table.py`.

The YAML hub has one spelling for null, so the empty cell and the cell that
spells `null` cannot both cross it: an empty cell comes back as the text
`null`. The AST-level round trip the Rust corpus test defends is exact -
there the empty cell is a `Null` node, and a `Null` node is an empty field
- but the hub in between is text, and text has one word for null. Three
fixtures do not survive the hub, and each one says why rather than being
quietly left out of the comparison.

The corpus is deliberately free of one shape where Python and this engine
disagree, and the disagreement is a choice rather than an accident:

- A field with leading or trailing whitespace is quoted by this engine and
  written raw by Python's writer, which treats whitespace as content. The
  engine quotes it because a YAML plain scalar would trim it, and the quoting
  is what keeps the text intact across the hub. RFC 4180 §5.3 asks for such a
  field *not* to be quoted; that is a SHOULD, and the engine answers it the
  other way on purpose.

That shape does not appear in the corpus, so the two implementations agree on
every byte of it - but a reader who diffs a Python-written file against an
engine-written one should know which shape differs and why.

The reader's rules differ in three more places, and each is pinned by its own
test rather than left to this file: a blank line is a record for Python and
nothing for this engine, an unclosed quoted field is a field for Python and a
refusal here, and an empty cell is null here while Python hands back an empty
string. `tests/test_csv_spec_table.py` is where those live, clause by clause
against RFC 4180.

The engine types a cell when it reads it and the standard library types
nothing at all, so the comparison against `load_csv`'s Python values goes
through the core schema's rules written out by hand in `_typed` below - a
comparison that asks the engine what it ought to have produced would not be
a comparison.

Run this file directly to (re)generate the corpus:

    python tests/test_csv_crosslib.py
"""

from __future__ import annotations

import csv
import io
import re
from pathlib import Path

import pytest

import pyrs_yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = REPO_ROOT / "crates" / "pyrs-csv" / "tests" / "fixtures"

# The corpus itself. Every entry names the rows the fixture holds and the
# dialect it is written in; the file name follows from the delimiter, so a
# tab-delimited fixture is a `.tsv` file and the CLI's extension-based
# routing finds it.
CORPUS = (
    {
        "name": "basic",
        "rows": (
            ("id", "name", "port"),
            ("1", "web-1", "8080"),
            ("2", "web-2", "443"),
        ),
    },
    {
        # A leading zero is the spelling the whole engine exists to keep:
        # `007` resolves to the integer 7 and is still written back as `007`.
        "name": "leading_zeros",
        "rows": (
            ("code", "amount"),
            ("007", "1"),
            ("0", "2"),
            ("00", "3"),
        ),
    },
    {
        # The typed cells: a boolean, a boolean and a null.
        "name": "types",
        "rows": (
            ("flag", "other", "nothing"),
            ("true", "false", "null"),
        ),
    },
    {
        # An empty cell is null, and a null is an empty field on the way out.
        "name": "empty_cells",
        "rows": (
            ("a", "b", "c"),
            ("", "x", ""),
            ("y", "", ""),
        ),
    },
    {
        # A quoted field may hold the separator.
        "name": "quoted_separator",
        "rows": (
            ("a", "b"),
            ("x,y", "z"),
        ),
    },
    {
        # A quoted field may hold a line break.
        "name": "embedded_newline",
        "rows": (
            ("a", "b"),
            ("line1\nline2", "z"),
        ),
    },
    {
        # `""` is one quote.
        "name": "doubled_quotes",
        "rows": (
            ("a", "b"),
            ('say "hi"', "z"),
        ),
    },
    {
        # Text that is not ASCII, in three scripts.
        "name": "non_ascii",
        "rows": (
            ("名前", "città"),
            ("日本語", "café"),
            ("🙂", "Ω"),
        ),
    },
    {
        # The same table written with the other record separator.
        "name": "crlf",
        "terminator": "\r\n",
        "rows": (
            ("id", "name"),
            ("1", "web-1"),
        ),
    },
    {
        # The same table behind a byte-order mark. The mark is an encoding
        # artifact rather than data, so the engine strips it on the way in -
        # which is why this one fixture is compared without it.
        "name": "bom",
        "bom": True,
        "rows": (
            ("id", "name"),
            ("1", "web-1"),
        ),
    },
    {
        # The same engine, a tab where the comma was.
        "name": "tabbed",
        "delimiter": "\t",
        "rows": (
            ("id", "name"),
            ("1", "web-1"),
        ),
    },
    {
        # A refusal: a row with fewer cells than the header declares.
        "name": "ragged_row",
        "rows": (
            ("a", "b"),
            ("1",),
        ),
    },
    {
        # A refusal: a header that names the same column twice.
        "name": "duplicate_header",
        "rows": (
            ("a", "a"),
            ("1", "2"),
        ),
    },
)


#: The two fixtures the engine refuses. They are in the corpus because a
#: refusal is a shape the corpus has to hold too, not because they round
#: trip - and Python's `csv` module writes them happily, since it validates
#: nothing.
REFUSALS = ("ragged_row", "duplicate_header")

#: The cells the corpus holds that the YAML core schema types, and the type
#: each one gets.
#:
#: The engine types a cell when it reads it; the standard library's reader
#: types nothing. Comparing the two cell by cell therefore needs a third
#: statement of what a cell means, and it is written here by hand rather
#: than borrowed from the engine.
TYPED_CELLS = {
    "": None,  # an empty cell is null
    "null": None,
    "~": None,
    "true": True,
    "false": False,
}

#: The fixtures whose bytes survive the YAML hub unchanged.
#:
#: The four that do not are named in
#: `test_the_fixtures_that_do_not_come_back_byte_for_byte_say_why`, each one
#: with the reason it differs: the record separator of `crlf`, the byte
#: order mark of `bom`, the empty cell of `empty_cells`, and the two
#: refusals, which have nothing to round trip at all.
ROUND_TRIP_EXACT = (
    "basic",
    "leading_zeros",
    "types",
    "quoted_separator",
    "embedded_newline",
    "doubled_quotes",
    "non_ascii",
    "tabbed",
)


def _delimiter_of(entry: dict) -> str:
    """The separator the fixture was written with."""
    return "\t" if entry.get("delimiter") == "\t" else ","


def _typed(text: str):
    """What the core schema makes of a cell the standard library hands back as text."""
    if text in TYPED_CELLS:
        return TYPED_CELLS[text]
    if re.fullmatch(r"[0-9]+", text):
        # A leading zero does not survive as a zero: `007` is the integer
        # seven. The *spelling* is what the writer keeps, not the value.
        return int(text)
    return text


def _entry(name: str) -> dict:
    """The corpus entry with the given name."""
    for entry in CORPUS:
        if entry["name"] == name:
            return entry
    raise KeyError(name)


def _file_name(entry: dict) -> str:
    """The fixture's file name: the extension follows the delimiter."""
    extension = "tsv" if entry.get("delimiter", ",") == "\t" else "csv"
    return f"{entry['name']}.{extension}"


def _render(entry: dict) -> bytes:
    """The bytes Python's own `csv` module writes for this entry."""
    buffer = io.StringIO(newline="")
    writer = csv.writer(
        buffer,
        delimiter=entry.get("delimiter", ","),
        lineterminator=entry.get("terminator", "\n"),
    )
    writer.writerows(entry["rows"])
    data = buffer.getvalue().encode("utf-8")
    if entry.get("bom", False):
        data = b"\xef\xbb\xbf" + data
    return data


def write_corpus(directory: Path) -> list[Path]:
    """Write every fixture of the corpus into `directory`."""
    directory.mkdir(parents=True, exist_ok=True)
    written = []
    for entry in CORPUS:
        path = directory / _file_name(entry)
        path.write_bytes(_render(entry))
        written.append(path)
    return written


def test_every_fixture_is_python_s_csv_module_own_bytes() -> None:
    """The committed corpus is byte for byte what Python's `csv` module writes."""
    for entry in CORPUS:
        path = FIXTURE_DIR / _file_name(entry)
        assert path.is_file(), f"{path} is missing from the corpus"
        expected = _render(entry)
        found = path.read_bytes()
        assert found == expected, (
            f"{path.name} is not Python's canonical form:\n  expected {expected!r}\n  found    {found!r}"
        )


def test_the_corpus_holds_no_file_it_does_not_describe() -> None:
    """A fixture nobody regenerates is a fixture nobody checks."""
    known = {_file_name(entry) for entry in CORPUS}
    found = {path.name for path in FIXTURE_DIR.glob("*") if path.is_file()}
    assert found == known, f"the corpus directory holds {sorted(found - known)} it does not describe"


def test_the_corpus_covers_the_shapes_the_engine_claims_to_handle() -> None:
    """The corpus is the evidence for the engine's claims, so it has to hold them."""
    names = {entry["name"] for entry in CORPUS}
    for required in (
        "basic",
        "leading_zeros",
        "types",
        "empty_cells",
        "quoted_separator",
        "embedded_newline",
        "doubled_quotes",
        "non_ascii",
        "crlf",
        "bom",
        "tabbed",
        "ragged_row",
        "duplicate_header",
    ):
        assert required in names, f"the corpus does not cover {required}"


def test_a_leading_zero_keeps_its_spelling_in_the_corpus() -> None:
    """The spelling is what travels: `007` is written as `007`, not as `7`."""
    data = (FIXTURE_DIR / "leading_zeros.csv").read_bytes()
    assert b"007" in data
    assert b"\n007," in data


def test_a_null_cell_is_not_written_as_an_empty_cell() -> None:
    """A cell that spells a null keeps that spelling; only an empty cell is empty."""
    data = (FIXTURE_DIR / "types.csv").read_bytes()
    assert b"null" in data


def test_the_engine_reads_every_cell_the_way_the_csv_module_does() -> None:
    """The engine's own parse of every fixture, cell by cell.

    The corpus proves the *files* are right: that each one holds the bytes
    Python's `csv` module writes for its rows. That is not the claim that can
    break. What the engine *reads out of* those bytes is, and this compares
    it against an implementation that shares no code with this one.

    Inference is off, so a cell is the text it was written as - which is what
    the standard library's reader hands back, and what the engine exists to
    preserve. The one difference is the empty cell: the engine reads it as
    null and the standard library as an empty string. The engine's null is
    reported as `None` here, so the difference is asserted rather than
    smoothed over.

    The no-header mode is the one that reads every fixture: there is no
    header for a row to be ragged against, so the two refusal fixtures are
    read here as the plain grids they are, and their refusals are pinned
    elsewhere - by `crates/pyrs-csv/tests/roundtrip_corpus.rs` for the
    engine's own reader, and by `tests/test_csv_spec_table.py` clause by
    clause.
    """
    for entry in CORPUS:
        path = FIXTURE_DIR / _file_name(entry)
        delimiter = _delimiter_of(entry)
        # The byte order mark is an encoding artifact and not data, so it is
        # stripped here and both readers are handed the same text.
        text = path.read_bytes().decode("utf-8-sig")
        expected = list(csv.reader(io.StringIO(text, newline=""), delimiter=delimiter))
        found = pyrs_yaml.safe_load(pyrs_yaml.from_csv(text, delimiter=delimiter, header=False, infer=False))
        assert len(found) == len(expected), path.name
        for found_row, expected_row in zip(found, expected):
            assert len(found_row) == len(expected_row), path.name
            for found_cell, expected_cell in zip(found_row, expected_row):
                if expected_cell == "":
                    assert found_cell is None, (path.name, found_cell)
                else:
                    assert found_cell == expected_cell, (path.name, found_cell)


def test_the_engine_names_the_columns_the_way_the_csv_module_does() -> None:
    """The header row names the same columns the standard library's reader does.

    The cell-by-cell comparison above reads the grid; this one reads the
    table, so the column names are compared as well - a header of `007`
    stays the name `007` and does not become the number seven.
    """
    for entry in CORPUS:
        if entry["name"] in REFUSALS:
            continue
        path = FIXTURE_DIR / _file_name(entry)
        delimiter = _delimiter_of(entry)
        text = path.read_bytes().decode("utf-8-sig")
        expected = list(csv.DictReader(io.StringIO(text, newline=""), delimiter=delimiter))
        found = pyrs_yaml.safe_load(pyrs_yaml.from_csv(text, delimiter=delimiter, infer=False))
        assert len(found) == len(expected), path.name
        for found_row, expected_row in zip(found, expected):
            assert set(found_row) == set(expected_row), path.name
            for column, expected_cell in expected_row.items():
                found_cell = found_row[column]
                if expected_cell == "":
                    assert found_cell is None, (path.name, column)
                else:
                    assert found_cell == expected_cell, (path.name, column)


def test_the_engine_refuses_the_two_fixtures_the_csv_module_accepts() -> None:
    """The refusals are the engine's own, and they are refusals.

    Python's `csv` module writes a ragged row and a twice-named column
    without complaint, because it validates nothing. The engine refuses
    both, and says which one it refused and where.
    """
    for entry in CORPUS:
        if entry["name"] not in REFUSALS:
            continue
        path = FIXTURE_DIR / _file_name(entry)
        delimiter = _delimiter_of(entry)
        text = path.read_bytes().decode("utf-8-sig")
        # The standard library reads both of them without complaint.
        assert list(csv.reader(io.StringIO(text, newline=""), delimiter=delimiter))
        with pytest.raises(pyrs_yaml.YamlParseError) as caught:
            pyrs_yaml.from_csv(text, delimiter=delimiter)
        message = str(caught.value)
        if entry["name"] == "ragged_row":
            assert "ragged row at line 2" in message, message
        else:
            assert 'duplicate header column "a" at line 1' in message, message


def test_load_csv_types_every_cell_the_way_the_core_schema_does() -> None:
    """`load_csv`'s Python values, against the standard library's cells.

    `load_csv` is the table-shaped door into the engine: it reads a CSV
    document into Python values, one dict per record. The standard library's
    reader hands back text and types nothing, so the two are compared
    through the core schema's rules - written out by hand in `_typed`,
    because asking the engine what it ought to have produced would not be a
    comparison.

    The one difference between the two readers is the empty cell: the engine
    reads it as null and the standard library as an empty string. That
    difference is asserted here rather than smoothed over.
    """
    for entry in CORPUS:
        path = FIXTURE_DIR / _file_name(entry)
        delimiter = _delimiter_of(entry)
        text = path.read_bytes().decode("utf-8-sig")
        if entry["name"] in REFUSALS:
            # The engine refuses these two; the standard library reads them
            # without complaint, and the refusal is the engine's own.
            with pytest.raises(pyrs_yaml.YamlParseError):
                pyrs_yaml.load_csv(text, delimiter=delimiter)
            continue
        expected = list(csv.reader(io.StringIO(text, newline=""), delimiter=delimiter))
        found = pyrs_yaml.load_csv(text, delimiter=delimiter)
        # The header is a row of names, not a record of its own.
        assert len(found) == len(expected) - 1, path.name
        assert list(found[0].keys()) == expected[0], path.name
        for found_row, expected_row in zip(found, expected[1:]):
            for column, stdlib_cell in zip(found_row.keys(), expected_row):
                found_cell = found_row[column]
                if stdlib_cell == "":
                    # The engine reads an empty cell as null; the standard
                    # library hands back an empty string.
                    assert found_cell is None, (path.name, column, found_cell)
                    continue
                typed = _typed(stdlib_cell)
                # The type is compared as well as the value: `True` and `1`
                # are equal in Python, and a cell that became the wrong one
                # of the two would slip through a bare equality.
                assert type(found_cell) is type(typed), (path.name, column, found_cell, typed)
                assert found_cell == typed, (path.name, column, found_cell, typed)


def test_a_csv_file_comes_back_byte_for_byte_through_the_yaml_hub() -> None:
    """The round trip the corpus exists to defend: CSV -> YAML -> CSV.

    The bytes a file was written with are the bytes it has to come back
    with, and the YAML in the middle is a hub, not a filter: `from_csv`
    reads the file into a YAML document and `to_csv` reads that document
    back into a file.
    """
    for entry in CORPUS:
        if entry["name"] not in ROUND_TRIP_EXACT:
            continue
        path = FIXTURE_DIR / _file_name(entry)
        delimiter = _delimiter_of(entry)
        raw = path.read_bytes()
        text = raw.decode("utf-8-sig")
        yaml = pyrs_yaml.from_csv(text, delimiter=delimiter)
        back = pyrs_yaml.to_csv(yaml, delimiter=delimiter)
        assert back.encode("utf-8") == raw, path.name


def test_the_fixtures_that_do_not_come_back_byte_for_byte_say_why() -> None:
    """The fixtures whose bytes do not survive the hub, and the reason each.

    The reasons are stated here rather than left to be found by whoever
    compares a file with its own round trip:

    - `crlf` - the writer emits `\\n`. The record separator belongs to the
      file, and the file's own bytes are not carried across the hub; the
      table is.
    - `bom` - the byte order mark is an encoding artifact, and an artifact
      is not written back.
    - `empty_cells` - YAML has one spelling for null, so the empty cell and
      the cell that spells `null` cannot both cross it. The empty cell
      comes back as the text `null`.
    - `ragged_row` and `duplicate_header` - the engine refuses them, so
      there is nothing to round trip. They are asserted here to refuse, so
      that no fixture is quietly skipped.
    """
    # The record separator is normalized to `\n`, and the table is not.
    entry = _entry("crlf")
    text = (FIXTURE_DIR / _file_name(entry)).read_bytes().decode("utf-8-sig")
    assert "\r\n" in text, "the fixture is the CRLF one"
    back = pyrs_yaml.to_csv(pyrs_yaml.from_csv(text, delimiter=","), delimiter=",")
    assert back == text.replace("\r\n", "\n")

    # The byte order mark is not written back.
    entry = _entry("bom")
    raw = (FIXTURE_DIR / _file_name(entry)).read_bytes()
    assert raw.startswith(b"\xef\xbb\xbf"), "the fixture is the BOM one"
    text = raw.decode("utf-8-sig")
    back = pyrs_yaml.to_csv(pyrs_yaml.from_csv(text, delimiter=","), delimiter=",")
    assert back == text
    assert not back.startswith("\ufeff")

    # The empty cell crosses the hub as a null and comes back as the text
    # `null` - the one spelling YAML has for it.
    entry = _entry("empty_cells")
    text = (FIXTURE_DIR / _file_name(entry)).read_bytes().decode("utf-8-sig")
    back = pyrs_yaml.to_csv(pyrs_yaml.from_csv(text, delimiter=","), delimiter=",")
    assert back == "a,b,c\nnull,x,null\ny,null,null\n"

    # The two refusals have no round trip at all.
    for name in REFUSALS:
        entry = _entry(name)
        text = (FIXTURE_DIR / _file_name(entry)).read_bytes().decode("utf-8-sig")
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_csv(text, delimiter=_delimiter_of(entry))


if __name__ == "__main__":
    for written in write_corpus(FIXTURE_DIR):
        print(f"wrote {written}")
