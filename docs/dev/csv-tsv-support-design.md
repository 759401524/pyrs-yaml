# CSV / TSV engine — slice 1 design

Status: **approved** (decisions locked 2026-11-23 by the maintainer; slice 1 = CSV/TSV,
XML is slice 2).

Language: English per the `pyq-multidoc-edit-design.md` precedent.

## Locked decisions

1. **CSV shape** — the header row becomes the keys of a sequence of mappings
   (one mapping per data record). A no-header mode produces a sequence of
   sequences instead.
2. **Cell semantics** — cells are typed by inference against the YAML core
   schema, and the cell's **source text is kept verbatim** so a
   CSV → YAML → CSV cycle reproduces the original bytes. Inference is what
   decides the *type*; the spelling is what travels.
3. **Scope of slice 1** — the whole stack: a new engine crate, the `pyq`
   verbs, the Python API, the four-locale docs, the changelog and the CI
   gates. TSV is the same engine with a tab delimiter, not a second engine.
4. **XML is slice 2 — direction locked, spelling open.** The direction is
   round-trip-first with a custom convention, *not* the `@attr` / `#text`
   family. Two facts found in this repository make that family unusable
   here: `@` is a YAML reserved indicator (a key spelled `@attr` is a
   parse error), and a key spelled `#text` is read as a comment, which is
   silent data loss. A slice-2 convention therefore needs a prefix family
   outside YAML's reserved indicators; `+` is outside the XML Name
   production, so it cannot collide with a real element name. `+@attr` /
   `+content` / `+p_` are the candidates to evaluate when slice 2 opens —
   they are not decided here, and this document does not fix a mapping.

## Investigated facts that shape the design

Verified in this repository on 2026-11-23.

- **`pyrs-schema` already owns the type rules.**
  `resolve_core_type(text) -> YamlType` and `core_type_is_non_string(text) -> bool`
  are the same resolver the YAML hub uses. CSV inference reuses them, so a
  CSV cell and a YAML scalar of the same text resolve to the same type —
  there is no second, drifting set of rules.
- **The AST stores source text, not parsed numbers.**
  `CustomNode::Scalar { value: Arc<str>, style, .. }` keeps the bytes as
  written; the type is a read-time resolution. A cell reading `007` can
  therefore stay `007` in the text while resolving to an integer — this is
  what makes decision 2 lossless in text instead of lossy in both.
- **The house style is a self-written engine.**
  `pyrs-json` (`#![no_std]` + alloc) depends only on `indexmap`,
  `unicode-ident` and `pyrs-ast`; `pyrs-toml` adds `pyrs-schema` and
  `pyrs-event`. `pyrs-csv` follows the same shape — `indexmap`,
  `pyrs-ast` and `pyrs-schema`, and nothing else. No XML crate appears
  anywhere in `Cargo.lock`: XML is slice 2, and this document does not
  decide its mapping.
- **A new `no_std` crate touches two gates.**
  The `no_std` CI job builds a fixed crate list for `thumbv7em-none-eabi`,
  and `tests/test_no_std_gate.py` fails when that list stops covering every
  crate whose source declares `#![no_std]`. Adding `pyrs-csv` therefore
  means editing `.github/workflows/ci.yml` **and** the expected crate sets
  in that test — the job's own comment says the list is not written here
  on purpose.
- **Error strings are localized through `rust-i18n`.**
  The catalogs live in `crates/pyrs-yaml-core/src/i18n/locales/`
  (`en.yml`, `ja-JP.yml`, `ko-KR.yml`, `zh-CN.yml`); a new key has to land
  in all four.
- **The compatibility shell already refuses `yq --csv-*` "by name"**
  (`crates/pyrs-yaml-cli/src/compat.rs`). That refusal stays — the native
  verbs are the supported path — but the help text that lists the refused
  families changes.

## Design

### 1. Crate layout

```text
crates/pyrs-csv/
├── Cargo.toml          # pyrs-csv, lib pyrs_csv, edition 2024, rust-version 1.96
├── src/
│   ├── lib.rs          # crate docs, public API re-exports
│   ├── error.rs        # CsvError with stable codes
│   ├── parser.rs       # byte-level scanner
│   └── writer.rs       # renderer
└── tests/
    ├── roundtrip_corpus.rs
    └── reference_parity.rs
```

`Cargo.toml` mirrors `pyrs-toml`: `indexmap` (`default-features = false`),
`pyrs-ast`, `pyrs-schema`; features `default = ["std"]`,
`std = ["indexmap/std", "pyrs-ast/std"]`. **No `pyrs-event`**: slice 1 has
no event vocabulary for CSV, and an unused dependency is a false claim of
capability.

`pyrs-csv` sits beside `pyrs-json` / `pyrs-toml` and does **not** depend on
`pyrs-yaml-core`. YAML stays the only editable representation; CSV is an
exchange format here, exactly like TOML.

### 2. Public API

```rust
// crates/pyrs-csv/src/lib.rs
#![no_std]
extern crate alloc;

pub mod error;
pub mod parser;
pub mod writer;

pub use error::CsvError;
pub use parser::{CsvOptions, Delimiter, LineTerminator, from_csv, from_csv_with_options};
pub use writer::{to_csv_text, to_csv_text_with_options};
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Delimiter { #[default] Comma, Tab, Semicolon, Pipe }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineTerminator { #[default] Lf, CrLf }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvOptions {
    pub delimiter: Delimiter,
    /// Header row present (default true).
    pub header: bool,
    /// Quote character (default b'"').
    pub quote: u8,
    /// Type inference on (default true).
    pub infer_types: bool,
    pub line_terminator: LineTerminator,
    /// Skip empty records (default true).
    pub skip_blank_lines: bool,
}

impl Default for CsvOptions { /* header: true, quote: b'"', infer_types: true, ... */ }

pub fn from_csv(text: &str) -> Result<CustomNode, CsvError>;
pub fn from_csv_with_options(text: &str, options: &CsvOptions) -> Result<CustomNode, CsvError>;
pub fn to_csv_text(node: &CustomNode) -> Result<String, CsvError>;
pub fn to_csv_text_with_options(node: &CustomNode, options: &CsvOptions) -> Result<String, CsvError>;
```

`CsvError` carries a stable code, the way this repository's other engines do:

| Code | Variant | Meaning |
|------|---------|---------|
| `E-CSV-001` | `UnterminatedQuote { line, column }` | a quoted field was never closed |
| `E-CSV-002` | `RaggedRow { line, expected, found }` | a record's field count differs from the header's |
| `E-CSV-003` | `DuplicateHeader { name, line }` | two columns carry the same name |
| `E-CSV-004` | `UnsupportedShape { detail }` | the AST shape is not a table |
| `E-CSV-005` | `UnsupportedCell { path, detail }` | a cell (or key) cannot be written to CSV |

Each variant has a `Display` message that names the position, and the
Python layer maps the codes onto the localized `csv-parse-error` /
`csv-serialize-error` keys.

### 3. Reading

RFC 4180 with these decisions, each of them pinned by a fixture:

- Record separator: `LF` or `CRLF` are both accepted; the writer emits `LF`
  by default and `CRLF` on request.
- Field separator: the configured delimiter (`,` default, `\t` for TSV,
  `;`, `|`).
- A quoted field may contain the separator, `CR`, `LF`, and doubled quotes
  (`""` is one quote). An unclosed quote is `E-CSV-001` — there is no
  silent recovery.
- A UTF-8 BOM at the head of the input is stripped.
- Blank records are skipped (`skip_blank_lines`, default on).
- **Header mode** (default): the first record is the header; each data
  record becomes a mapping from column name to cell.
    - A duplicate column name is `E-CSV-003` — a mapping cannot hold two
      equal keys without losing one of them.
    - A record whose field count differs from the header's is `E-CSV-002` —
      no silent padding, no silent truncation.
- **No-header mode**: every record becomes a sequence of cells; no header
  record is written back.
- **Cell typing** (inference on): the cell text is resolved with
  `pyrs_schema::resolve_core_type`.
    - resolves to int / float / bool → `CustomNode::Scalar { value: <source
      text verbatim>, style: Plain }`
    - resolves to String → `CustomNode::Scalar { value: <source text>,
      style: Plain }`
    - resolves to Null → `CustomNode::Scalar { value: <source text verbatim>,
      style: Plain }`: the spelling travels, so a cell reading `null` is
      written back as `null` and not as an empty field
    - an empty cell → `CustomNode::Null`: there is no source text to carry,
      and an empty field is what it writes back as
- **Cell text is never trimmed or normalized.** A quoted field keeps its
  embedded `CRLF` exactly.
- **Inference off**: every non-empty cell becomes a *double-quoted* scalar
  so the YAML side keeps it a string — the same rule TOML follows ("TOML
  strings keep quoting so values never re-resolve"). An empty cell is still
  `Null`.

### 4. Writing

Accepted AST shapes:

| Shape | Result |
|-------|--------|
| Sequence of Mappings | header row from the first mapping's keys, then one row per mapping |
| Sequence of Sequences | rows only, no header record |
| Mapping | a single row, header from its keys |
| anything else | `E-CSV-004` |

Cell rendering:

- A scalar is written from its **source text**, never from a re-serialized
  numeric value — this is what makes the round trip byte-exact.
- A `Null` node is an empty field.
- A quoted scalar (double- or single-quoted) is a *string*: it is written
  quoted when its text would otherwise re-infer as a non-string
  (`pyrs_schema::core_type_is_non_string`), otherwise plain.
- A plain scalar is written unquoted unless it contains the delimiter, a
  quote, `CR`, `LF`, or leading/trailing whitespace — then it is quoted,
  and an embedded `"` is doubled.
- A nested mapping or sequence inside a cell is `E-CSV-005`; CSV has no
  cell for it. A non-scalar key is `E-CSV-005` too.
- Every mapping in a sequence must carry the same key set as the first
  one; otherwise `E-CSV-005`.
- The writer emits a record separator after the last record.

### 5. CLI contract

```text
pyq from-csv [file] [--input csv|tsv] [--no-header] [--no-infer]
pyq to-csv   [file] [--input csv|tsv] [--no-header]
```

- `--input csv|tsv` picks the delimiter; `auto` (the default) resolves by
  extension (`.csv` → comma, `.tsv` → tab).
- `pyq to-csv` reads the input as YAML/JSON/TOML (whatever `--input`
  says) and writes CSV text.
- Exit codes follow the existing convention: `0` success, `1` with
  `pyq: <message>` on stderr for a parse or serialize failure, `2` for the
  compatibility shell's "refused by name".
- `DIALECT_HELP` in `crates/pyrs-yaml-cli/src/lib.rs` lists the new verbs
  and the new `--input` values; the refused `yq --csv-*` family stays in
  the refusal list.

### 6. Python API contract

```python
pyrs_yaml.from_csv(text, delimiter=",", header=True, infer=True) -> str
pyrs_yaml.to_csv(yaml, delimiter=",") -> str
pyrs_yaml.load_csv(text, delimiter=",", header=True) -> list
```

- `delimiter` accepts one of `,` `\t` `;` `|`; anything else raises
  `ValueError`.
- `from_csv` / `to_csv` raise the repository's YAML error types carrying
  the localized `csv-parse-error` / `csv-serialize-error` messages; the
  keys land in all four locale catalogs.
- `load_csv` returns a list of dicts (header mode) or a list of lists
  (no-header mode).

### 7. Test matrix

- Unit tests inside the crate for the scanner and the renderer (quoting,
  escapes, ragged rows, duplicate headers, BOM, CRLF, inference on/off).
- `crates/pyrs-csv/tests/roundtrip_corpus.rs` — a fixture corpus under
  `crates/pyrs-csv/tests/fixtures/`; every fixture must survive
  CSV → AST → CSV **byte for byte**. The corpus covers: BOM, CRLF, quoted
  separators, embedded newlines, doubled quotes, empty cells, leading
  zeros, `null` / `true` / `false` cells, non-ASCII text, a ragged row and a
  duplicate header (rejection fixtures).
- `tests/test_csv_crosslib.py` (root, pytest) — regenerates the corpus with
  Python's `csv` module and compares byte for byte with the committed files,
  and compares the engine's own parse of every fixture against the standard
  library's, cell by cell. This is the live oracle; the Rust tests consume the
  same fixtures.
- `tests/test_csv_spec_table.py` (root, pytest) — the RFC 4180 clauses, one row
  per clause, with the standard library as the second implementation.
- `crates/pyrs-yaml-cli/tests/cli.rs` — the new verbs, `--no-header`,
  `--no-infer`, and the error exit codes.
- `tests/test_no_std_gate.py` — its expected crate sets grow with
  `pyrs-csv`, and `.github/workflows/ci.yml`'s `no_std` job gains
  `-p pyrs-csv`.

### 8. Out of scope for slice 1

- XML (slice 2).
- CSV as an *editing* substrate: no `pyq set` into a CSV file, no splice
  engine, no comment preservation — CSV has no comment syntax.
- Multi-table files (one table per file).
- Delimiter sniffing: the extension and the explicit flag decide.
- Excel dialect quirks: a cell starting with `=`, `+`, `-` or `@` is
  written as-is, not escaped. Escaping would change the data, and a
  spreadsheet formula injected by a *read* file is out of this tool's
  threat model. Documented, not silently handled.

### 9. End-to-end verification

All of these pass before the change is proposed:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --release
cargo build --no-default-features --target thumbv7em-none-eabi -p pyrs-csv
python -m pytest tests/test_csv_crosslib.py tests/test_csv_spec_table.py tests/test_no_std_gate.py
python scripts/check_doc_wrapping.py <changed docs>
python scripts/check_doc_headings.py <changed docs>
python scripts/check_changelog_coupling.py --paths <changed docs>
python scripts/check_changelog_mirrors.py
```

Plus a manual smoke: `pyq from-csv` a fixture, `pyq to-csv` it back, and
diff the result against the original file. The byte-for-byte claim is the
engine's own — CSV → AST → CSV — and it is the one the corpus exists to
defend. Through the YAML *text* hop the promise is narrower, and three things
normalize: CRLF becomes LF, a BOM is stripped and not written back, and an
empty cell and a cell spelled `null` are the same document, because YAML has
one null spelling. The AST keeps the two apart; the text cannot. Quoting is
syntax, not a type assertion, and is not preserved through the text hop: a
cell written `"true"` resolves to a boolean and comes back unquoted. Callers
who need every cell to stay a string use `--no-infer` (or `infer=False`),
which also covers strings that look like numbers or dates. The hub boundary
is therefore not a closed list of three normalisations — these are the three
that are structural to the YAML text format; the guides point at the
whitespace and reader differences that the cross-library tests measure.

### 10. Acceptance criteria that must be demonstrated, not asserted

1. A cell reading `007` comes back as `007` — not `7`. If the YAML writer
   canonicalizes the text, inference and verbatim spelling cannot both
   hold, and the design has to change before the PR lands.
2. A cell reading `true` comes back as `true` (a boolean), and a YAML
   string `"true"` comes back as a *quoted* CSV cell so it re-imports as a
   string.
3. `E-CSV-002` and `E-CSV-003` are raised, not repaired.
4. The no_std job builds `pyrs-csv` for `thumbv7em-none-eabi`.
