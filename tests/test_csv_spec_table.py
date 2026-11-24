"""RFC 4180, clause by clause, against the standard library's own `csv` module.

The corpus in `tests/test_csv_crosslib.py` proves that this engine and Python's
`csv` module agree on the *bytes* of a file. It cannot prove that the *rules*
agree, because a corpus only holds the shapes somebody thought to write down: a
grammar rule the engine breaks on a shape nobody put in the corpus is invisible
to it, and the round trip passes anyway.

So this file is the rule table. Each row is one clause of RFC 4180 - the
grammar of §2, the file layout of §3 and §4, the quoting rules of §5 - with the
document that clause describes and what a reader must make of it. The authority
is not this repository: it is `csv`, the module the interpreter has carried
since 3.0, and every row is put to both readers, so a rule that drifts shows up
as a disagreement between two implementations rather than as a green run.

`csv` is imported outright and is never skipped. A cross-library table that
skips itself on the leg where it matters is a table that measures nothing, and
the standard library's `csv` module is on every leg there is.

A row carries up to three expectations, because a clause can be about reading,
about writing, or about refusing:

- `records` - the fields a reader must produce, as text.
- `written` - the CSV text the engine must write back.
- `refusal` - the message the engine must refuse with.

and `reference_records` - what the reference makes of a row the engine refuses -
records the one place where this engine is deliberately the stricter of the two.

The rows that need an empty cell - the trailing separator, the empty field -
are not in the shared table. An empty cell is null in this engine and an empty
string in Python's reader, and that difference is a fact about the hub's null
rule rather than about RFC 4180, so it gets its own test instead of being
smoothed over inside a shared row.

The divergences between the two implementations - a blank line, an unclosed
quoted field, the whitespace at the edge of a field, the record separator - are
pinned by the tests at the bottom of this file, each one named after the shape
it is about. A difference that is only written down in a comment is a difference
that gets "fixed" by accident.
"""

from __future__ import annotations

import csv
import io
from typing import NamedTuple

import pytest

import pyrs_yaml


class Clause(NamedTuple):
    """One row of the RFC 4180 table."""

    #: What the RFC says, in the RFC's own terms.
    clause: str
    #: The document the clause describes.
    text: str
    #: The fields a reader must produce, as text.
    records: list[list[str]] | None = None
    #: The CSV text the engine must write back.
    written: str | None = None
    #: The message the engine must refuse with.
    refusal: str | None = None
    #: What the reference makes of a row the engine refuses.
    reference_records: list[list[str]] | None = None


CLAUSES: tuple[Clause, ...] = (
    # --- §2 the grammar -------------------------------------------------
    Clause(
        clause="RFC 4180 §2.1 - a record is one or more fields separated by commas",
        text="a,b,c\n",
        records=[["a", "b", "c"]],
    ),
    Clause(
        clause="RFC 4180 §2.1 - a record may hold a single field",
        text="a\n",
        records=[["a"]],
    ),
    Clause(
        clause="RFC 4180 §2.2 - a field is either escaped or non-escaped",
        text='a\n"x"\n',
        records=[["a"], ["x"]],
    ),
    Clause(
        clause="RFC 4180 §2.3 - a quoted field may hold the comma",
        text='a\n"x,y"\n',
        records=[["a"], ["x,y"]],
    ),
    Clause(
        clause="RFC 4180 §2.3 - a quoted field may hold a CR",
        text='a\n"x\ry"\n',
        records=[["a"], ["x\ry"]],
    ),
    Clause(
        clause="RFC 4180 §2.3 - a quoted field may hold a CRLF",
        text='a\n"x\r\ny"\n',
        records=[["a"], ["x\r\ny"]],
    ),
    Clause(
        clause="RFC 4180 §2.3 - a quoted field may hold an LF",
        text='a\n"x\ny"\n',
        records=[["a"], ["x\ny"]],
    ),
    Clause(
        clause="RFC 4180 §2.4 - a non-escaped field is plain text",
        text="a\nxyz\n",
        records=[["a"], ["xyz"]],
    ),
    Clause(
        clause="RFC 4180 §2.6 - a doubled quote inside a quoted field is one quote",
        text='a\n"x""y"\n',
        records=[["a"], ['x"y']],
    ),
    Clause(
        clause="RFC 4180 §2.7 - records are separated by CRLF",
        text="a,b\r\n1,2\r\n",
        records=[["a", "b"], ["1", "2"]],
    ),
    Clause(
        clause="RFC 4180 §2.7 - a bare LF separates records too",
        text="a,b\n1,2\n",
        records=[["a", "b"], ["1", "2"]],
    ),
    # --- §3 and §4 the file ---------------------------------------------
    Clause(
        clause="RFC 4180 §3.1 - the first record names the columns",
        text="id,name\n1,web-1\n",
        records=[["id", "name"], ["1", "web-1"]],
    ),
    Clause(
        clause="RFC 4180 §4.1 - the file is a header and the records that follow it",
        text="a\n1\n2\n",
        records=[["a"], ["1"], ["2"]],
    ),
    # --- §5 the quoting rules -------------------------------------------
    Clause(
        clause="RFC 4180 §5.3 - the spaces of a field belong to the field",
        text="a\n x \n",
        records=[["a"], [" x "]],
        written='a\n" x "\n',
    ),
    Clause(
        clause="RFC 4180 §5.1 - a field holding the comma is written quoted",
        text='a\n"x,y"\n',
        records=[["a"], ["x,y"]],
        written='a\n"x,y"\n',
    ),
    Clause(
        clause="RFC 4180 §5.2 - a field holding a line break is written quoted",
        text='a\n"x\ny"\n',
        records=[["a"], ["x\ny"]],
        written='a\n"x\ny"\n',
    ),
    Clause(
        clause="RFC 4180 §5.7 - a field holding a quote is written with the quote doubled",
        text='a\n"x""y"\n',
        records=[["a"], ['x"y']],
        written='a\n"x""y"\n',
    ),
    Clause(
        clause="RFC 4180 §2.7 - the record separator this engine writes is LF",
        text="a,b\r\n1,2\r\n",
        records=[["a", "b"], ["1", "2"]],
        written="a,b\n1,2\n",
    ),
    # --- the one invariant the engine exists to keep --------------------
    Clause(
        clause="RFC 4180 §5.3 - a leading zero is text and is not re-serialized",
        text="code\n007\n",
        records=[["code"], ["007"]],
        written="code\n007\n",
    ),
    Clause(
        clause="RFC 4180 §3.1 - a column name is a name and not a value",
        text="007\n007\n",
        records=[["007"], ["007"]],
        written="007\n007\n",
    ),
    # --- the rows this engine refuses -----------------------------------
    Clause(
        clause="RFC 4180 §3.1 - the header names every column once",
        text="a,a\n1,2\n",
        refusal='duplicate header column "a" at line 1',
        reference_records=[["a", "a"], ["1", "2"]],
    ),
    Clause(
        clause="RFC 4180 §2.1 - every record holds the fields the header declares",
        text="a,b\n1\n",
        refusal="ragged row at line 2: expected 2 fields, found 1",
        reference_records=[["a", "b"], ["1"]],
    ),
    Clause(
        clause="RFC 4180 §2.3 - a quoted field is closed",
        text='a\n"abc\n',
        refusal="unterminated quoted field starting at line 2, column 1",
        reference_records=[["a"], ["abc\n"]],
    ),
)


def _reference_records(text: str) -> list[list[str]]:
    """The fields Python's own reader makes of a document."""
    return list(csv.reader(io.StringIO(text, newline="")))


def _reference_writes(records: list[list[str]]) -> str:
    """The CSV text Python's own writer produces for these records.

    The record separator is LF, because that is the separator this engine
    writes; the reference's own default is CRLF, and a difference that is only
    a dialect default is not a rule.
    """
    buffer = io.StringIO(newline="")
    csv.writer(buffer, lineterminator="\n").writerows(records)
    return buffer.getvalue()


def _engine_records(text: str) -> list[list[str | None]]:
    """The fields this engine reads, as the text the file spells.

    `infer=False` makes every cell a quoted scalar, so the projection keeps the
    cell's spelling instead of the type the hub would resolve it to; the header
    is switched off because RFC 4180's grammar is about records, and a header
    row is a record like any other.
    """
    return pyrs_yaml.safe_load(pyrs_yaml.from_csv(text, header=False, infer=False))


def _engine_writes(text: str) -> str:
    """The CSV text this engine writes back for a document."""
    return pyrs_yaml.to_csv(pyrs_yaml.from_csv(text))


READ_CLAUSES = tuple(clause for clause in CLAUSES if clause.records is not None)
WRITE_CLAUSES = tuple(clause for clause in CLAUSES if clause.written is not None)
REFUSE_CLAUSES = tuple(clause for clause in CLAUSES if clause.refusal is not None)


@pytest.mark.parametrize("clause", READ_CLAUSES, ids=lambda clause: clause.clause)
def test_the_reference_and_the_engine_read_the_same_fields(clause: Clause) -> None:
    """Both readers must make the same fields of the document the clause describes."""
    assert _reference_records(clause.text) == clause.records, clause.clause
    assert _engine_records(clause.text) == clause.records, clause.clause


@pytest.mark.parametrize("clause", WRITE_CLAUSES, ids=lambda clause: clause.clause)
def test_the_engine_writes_back_the_csv_the_clause_requires(clause: Clause) -> None:
    """The writer must spell the document the way the clause spells it.

    The reference is asked the same question, and where the two spellings
    differ the difference is a measured one: this engine quotes a field whose
    edge whitespace it would otherwise let a YAML plain scalar trim.
    """
    assert _engine_writes(clause.text) == clause.written, clause.clause
    reference = _reference_writes(clause.records)
    if clause.clause.startswith("RFC 4180 §5.3 - the spaces"):
        assert reference == "a\n x \n"
        assert reference != clause.written
    else:
        assert reference == clause.written, clause.clause


@pytest.mark.parametrize("clause", REFUSE_CLAUSES, ids=lambda clause: clause.clause)
def test_the_engine_refuses_what_the_clause_refuses(clause: Clause) -> None:
    """A refusal must be a refusal, with the position in the file.

    The reference is lenient on all three of these rows - it hands the document
    back rather than refusing it - and this engine is the stricter of the two
    on purpose: a file that says two different things about its own shape is a
    file whose table nobody can read the same way twice.
    """
    with pytest.raises(pyrs_yaml.YamlParseError) as caught:
        pyrs_yaml.from_csv(clause.text)
    assert str(caught.value) == clause.refusal, clause.clause
    assert _reference_records(clause.text) == clause.reference_records, clause.clause


def test_the_header_row_names_the_columns() -> None:
    """§3.1: the first record is the header, and the records after it are keyed by it."""
    text = "id,name\n1,web-1\n"
    assert _reference_records(text) == [["id", "name"], ["1", "web-1"]]
    assert pyrs_yaml.load_csv(text) == [{"id": 1, "name": "web-1"}]


def test_a_column_name_is_a_name_and_not_a_value() -> None:
    """§3.1 read against §5.3: `007` names a column.

    The name is the text `007` and the cell under it is the number seven, and
    the two are not allowed to trade places on the way out.
    """
    text = "007\n007\n"
    assert _reference_records(text) == [["007"], ["007"]]
    assert pyrs_yaml.load_csv(text) == [{"007": 7}]
    assert _engine_writes(text) == "007\n007\n"


def test_a_leading_zero_is_never_re_serialized() -> None:
    """The invariant the whole engine exists to keep.

    `007` is the integer seven and still the text `007`, and no clause of the
    table above may turn it into `7`. This is the row that goes red if the
    engine ever re-serializes a cell from its resolved type instead of from its
    source text.
    """
    text = "code\n007\n"
    assert _reference_records(text) == [["code"], ["007"]]
    assert _engine_records(text) == [["code"], ["007"]]
    assert pyrs_yaml.from_csv(text) == "- code: 007\n"
    assert _engine_writes(text) == "code\n007\n"


def test_a_trailing_separator_opens_a_final_empty_column() -> None:
    """§2.1 read to the letter: `field *(COMMA field)`.

    A trailing separator is a separator and then a field, and that field is
    empty - so the record holds three fields, not two. Both readers agree on
    the count; they part ways on what an empty field is.
    """
    text = "a,b,\n1,2,\n"
    assert _reference_records(text) == [["a", "b", ""], ["1", "2", ""]]
    assert _engine_records(text) == [["a", "b", None], ["1", "2", None]]


def test_an_empty_cell_is_null_here_and_an_empty_string_there() -> None:
    """The hub's null rule, not RFC 4180's: an empty cell is null.

    The reference hands back the empty string, because a CSV field has no null
    of its own. This engine reads the cell as null, so that a CSV table and a
    YAML table mean the same thing.
    """
    text = "a,b\n,2\n"
    assert _reference_records(text) == [["a", "b"], ["", "2"]]
    assert _engine_records(text) == [["a", "b"], [None, "2"]]
    assert pyrs_yaml.load_csv(text) == [{"a": None, "b": 2}]


def test_a_blank_line_is_a_record_for_the_reference_and_nothing_for_this_engine() -> None:
    """§4.1: the file is a header and its records, and a blank line is not one.

    The reference yields an empty record; this engine skips the line, because a
    line that says nothing is not a row that says one empty cell.
    """
    text = "a,b\n\n1,2\n"
    assert _reference_records(text) == [["a", "b"], [], ["1", "2"]]
    assert _engine_records(text) == [["a", "b"], ["1", "2"]]


def test_an_unclosed_quoted_field_is_a_refusal_here_and_a_field_there() -> None:
    """§2.3: a quoted field is closed.

    The reference is lenient and swallows the rest of the file into the field.
    This engine refuses, because a field that was opened with a quote and never
    closed could have taken the whole file with it, and a reader that guesses
    where it ended turns a typo into a table.
    """
    text = 'a\n"abc\n'
    assert _reference_records(text) == [["a"], ["abc\n"]]
    with pytest.raises(pyrs_yaml.YamlParseError) as caught:
        pyrs_yaml.from_csv(text)
    assert str(caught.value) == "unterminated quoted field starting at line 2, column 1"


def test_the_record_separator_the_engine_reads_is_not_the_one_it_writes() -> None:
    """The separator is a property of the file, not of the table.

    The engine writes LF. A file that came in with CRLF comes back with LF,
    and the table inside it is untouched.
    """
    text = "a,b\r\n1,2\r\n"
    assert _reference_records(text) == [["a", "b"], ["1", "2"]]
    assert _engine_writes(text) == "a,b\n1,2\n"


def test_the_engine_quotes_edge_whitespace_where_the_reference_does_not() -> None:
    """§5.3 asks for a field with spaces not to be quoted; this engine quotes it.

    The reason is the hub, not the format: a YAML plain scalar trims the
    whitespace at the edge of its text, so an unquoted cell would lose the
    spaces on the way through the hub. The quoting is what keeps the text
    intact, and the reference's spelling is the one this engine does not use.
    """
    text = "a\n x \n"
    assert _reference_writes([["a"], [" x "]]) == "a\n x \n"
    assert _engine_writes(text) == 'a\n" x "\n'
