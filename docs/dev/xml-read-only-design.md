# XML slice 2 — read-only XML → YAML engine design

**Assumption (locked by team lead as `[假设]`, not by the maintainer): the key convention is the
`+`-prefixed family — `+@attr`, `+content`, `+c_<n>`, `+p_<n>`.** `+` is outside the XML
`NameStartChar` / `NameChar` productions, so no real XML name can collide with a reserved key.
Re-confirm the spelling before slice 3 opens the writer: at that point golden YAML fixtures,
four-locale docs and the Python stubs will have to be rewritten together, so the cost of changing it
grows from "one document edit" to "fixture + docs + stub migration".

Status: proposed. Slice 1 (CSV/TSV) is approved and merged at `eba45b30`; this document is the
contract for slice 2. Language: English, following `docs/dev/csv-tsv-support-design.md`.

## 0. Locked decisions

1. **Direction and scope.** Slice 2 is **read-only**: XML → YAML only. There is no `to_xml`, no
   `to-xml` verb, no writer module, and no `Write`-side error code. YAML → XML is slice 3.
2. **Full stack.** One new engine crate (`pyrs-xml`), the CLI `from-xml` verb, the Python `from_xml`
   / `load_xml` API, four-locale docs, a quality-matrix row, and the no_std gates — same blast
   radius as slice 1.
3. **Convention family.** Round-trip-first custom convention with a `+` prefix. The `@attr` /
   `#text` family is refused. In-repo measured evidence: `docs/dev/quality-ledger.md:15963-15969`
   records that both facts were checked in this repository — `@` is a YAML reserved indicator, so a
   key spelled `@attr` is a parse error in this engine; `#text` is read as a comment, i.e. **silent
   data loss**, not a visible refusal. The same is restated in
   `docs/dev/csv-tsv-support-design.md:20-29` and `ROADMAP.md:139`. A prefix outside YAML's reserved
   indicators is therefore mandatory; `+` is also outside the XML Name production, so the prefix
   cannot collide with a real element name.
4. **Reader-only fidelity claim.** The mapping is **infoset-preserving, not lexically lossless**. It
   preserves element/attribute/comment/PI structure and character data; it does not preserve the XML
   declaration, BOM, CDATA delimiters, entity spelling, self-closing vs explicit end tags, attribute
   quote style, or inter-attribute whitespace. These six normalisations are part of the contract and
   must appear in the docs.
5. **DTD is refused structurally.** No DTD parsing, no entity table, no external fetch. See §5.
6. **No third-party XML crate.** Self-written restricted byte scanner (§2). This keeps `pyrs-xml`
   inside the no_std foundation.

## 1. Verified repository facts

Verified by reading the tree on 2026-11-24. Line numbers are from the current HEAD.

- **Workspace** — `Cargo.toml:1-4`: nine members (`pyrs-ast`, `pyrs-event`, `pyrs-schema`,
  `pyrs-csv`, `pyrs-json`, `pyrs-toml`, `pyrs-yaml-core`, `pyrs-yaml`, `pyrs-yaml-cli`),
  `resolver = "2"`, `package.edition = "2024"`. Every member's `Cargo.toml:5` declares
  `rust-version = "1.96"` (all nine checked). `Cargo.lock` contains **no** XML crate (`quick-xml`,
  `roxmltree`, `xml-rs`, `xmlparser`, `serde-xml-rs`, `strong-xml`: all absent).
- **Spoke layout** — `crates/pyrs-csv/`: `Cargo.toml`, `src/{lib,error,parser,writer}.rs`,
  `tests/{roundtrip_corpus,reference_parity}.rs`, `tests/fixtures/*`. `crates/pyrs-csv/Cargo.toml`:
  `indexmap 2.7 (default-features=false)`, `pyrs-ast`, `pyrs-schema`; `default = ["std"]`,
  `std = ["indexmap/std","pyrs-ast/std"]`; **no `pyrs-event`, no `pyrs-yaml-core`**.
  `src/lib.rs:61-71` declares `#![no_std]`, `#[macro_use] extern crate alloc`, and
  `#[cfg(feature="std")] extern crate std`.
- **Schema helpers** — `crates/pyrs-schema/src/schema.rs:31`
  `resolve_core_type(value: &str) -> YamlType`, `:118`
  `core_type_is_non_string(value: &str) -> bool`, `:617` `plain_text_is_typed(text: &str) -> bool`
  (covers core + YAML 1.1 lexemes). `crates/pyrs-csv/src/parser.rs:255` already uses
  `plain_text_is_typed` for keys.
- **AST** — `crates/pyrs-ast/src/ast.rs:800-836`:
  `CustomNode::{Scalar{value:Arc<str>,style,chomping,block_indent,meta}, Mapping{pairs:NodeMap,flow_style,meta}, Sequence{items,flow_style,meta}, Null{meta}, Alias{name}}`;
  `ScalarStyle` at `:94-105` (`Plain`, `SingleQuoted`, `DoubleQuoted`, `Literal`, `Folded`);
  `NodeMap<K,V> = IndexMap<K,V,BuildHasherDefault<FxHasher>>` at `:36`.
- **i18n** — four catalogs:
  `crates/pyrs-yaml-core/src/i18n/locales/{en.yml,ja-JP.yml,ko-KR.yml,zh-CN.yml}`. Key format is
  `key: "%{detail}"`; the CSV pair sits at line 7–8 in **each** of the four files
  (`csv-parse-error`, `csv-serialize-error`). Slice 2 adds **one** key, `xml-parse-error`, at the
  same position in all four. No `xml-serialize-error`: no writer means no consumer (orphan
  capability, delete it).
- **CLI** — `crates/pyrs-yaml-cli/src/lib.rs`: `DIALECT_HELP` at `:31-52` (native verb list at
  `:34-36`, `--xml-*` refusal named at `:47-51`); `Command` enum at `:66-348` with
  `FromJson :267-286`, `FromToml :287-298`, `FromIni :299-310`, `FromCsv :311-332`,
  `ToCsv :333-342`; `InputOpts :350-360`; `Format` enum `:362-382`
  (`Auto,Yaml,Json,Jsonc,Json5,Toml,Ini,Csv,Tsv`); `CsvDialect :397-427`; verb dispatch
  `FromCsv :793-813`, `ToCsv :814-837`; `load_docs` non-streamable format list `:1032-1037`;
  `load_source :1074-1119` (extension map `:1080-1092`, CSV dispatch `:1115-1116`, helper
  `csv_to_node :1122-1131`). `compat.rs`: `NATIVE_COMMANDS :30-52`; `--xml-*` refusal flags
  `:383-389`; refusal test `:885-900`. `crates/pyrs-yaml-cli/Cargo.toml:20` is where `pyrs-xml`
  joins.
- **Python binding** — `crates/pyrs-yaml/src/py/functions.rs`: `csv_delimiter :870-881`;
  `from_csv :883-911` with
  `#[pyo3(signature = (csv_str: "str", delimiter: "str" = ",", header: "bool" = true, infer: "bool" = true) -> "str")]`
  at `:884`; `to_csv :913-937`; `load_csv :939-966`. All parse failures map through
  `YamlParseError::new_err(format_i18n_error("csv-parse-error", &[("detail", &e.to_string())]))`.
  Export block `crates/pyrs-yaml/src/py/mod.rs:195-255` (`from_csv :205`, `load_csv :227`). Wheel
  surface: `python/pyrs_yaml/__init__.py:59` import, `:457` `__all__`;
  `python/pyrs_yaml/pyrs_yaml.pyi:451` stub. `crates/pyrs-yaml/Cargo.toml:26` is where `pyrs-xml`
  joins.
- **Four-locale docs** — `docs/{en,ja,ko,zh}/api/reference.md`,
  `docs/{en,ja,ko,zh}/guides/formats.md`, `docs/{en,ja,ko,zh}/changelog.md` (en examples:
  `reference.md:688`, `guides/formats.md:131`), plus root `CHANGELOG.md`. Gates:
  `scripts/check_doc_signatures.py` (authority = shipped extension),
  `scripts/check_doc_wrapping.py`, `scripts/check_doc_headings.py`,
  `scripts/check_changelog_coupling.py --paths ...`, `scripts/check_changelog_mirrors.py`.
- **Quality matrix** — `scripts/quality_matrix.py`: `SERIALIZER_RE :41-44`; `FormatRow :228-236`
  (`writer: bool = True`); `FORMAT_ROWS :239-255` (CSV at `:243`); `ORACLE_IMPORTS :260-272`
  (`csv :271`); `STDLIB_SINCE :281` (`requires-python >= 3.8`, `pyproject.toml:11`);
  `AUTHORITY_SUFFIXES = ("_suite.py","_spec_table.py") :601`; `DIALECT_CALL :429`
  (`(json5|jsonc|json|toml|yaml|csv)` — **does not currently match `xml`**); crosslib module path is
  composed as `tests/test_{crosslib_stem}_crosslib.py` at `:823`; per-row assembly `:806-854`;
  round-trip cell derivation `:1321` (`"n/a" if not row["writer"]`).
  `tests/test_quality_matrix.py:1204-1237`: label regex `:1213`, `len(claimed) == 6` at `:1223`.
  `QUALITY_MATRIX.md:96-106` holds the derived table; CSV row `:104` is `yes|no|no|no|yes|yes`.
- **no_std gate** — `.github/workflows/ci.yml`: `no_std` job `:476-...`; three crate-listing steps —
  `Build no_std foundation crates :511-524` (`-p pyrs-csv :523`),
  `Run no_std unit tests on the host :545-554` (`:553`),
  `Build with the default std feature :562-571` (`:570`). `tests/test_no_std_gate.py`:
  `_ATTR_RE :56`, `_no_std_foundation :59-72` derives the set from every `crates/*/src/lib.rs`
  declaring `#![no_std]` — **no source edit needed here, but the CI lists and the two docs must
  follow**; coverage test `:147-164`, live-document test `:288-303`. Docs: `AGENTS.md:145-154`
  ("seven crates", names all seven); `docs/dev/boundaries.md:266-269` (names all seven).

## 2. Crate layout and dependency choice

```text
crates/pyrs-xml/
├── Cargo.toml          # pyrs-xml, lib pyrs_xml, edition 2024, rust-version 1.96
├── src/
│   ├── lib.rs          # #![no_std], re-exports
│   ├── error.rs        # XmlError with stable codes
│   ├── scanner.rs      # bytes -> Token {kind, byte_range, line, column}
│   ├── model.rs        # Token stream -> XmlNode (stack, depth/node caps)
│   └── mapping.rs      # XmlNode -> CustomNode (+ prefix), XmlOptions
└── tests/
    ├── mapping_golden.rs
    ├── security_refusals.rs
    └── fixtures/*.xml
```

Dependencies mirror `pyrs-csv` exactly: `indexmap` (`default-features=false`), `pyrs-ast`,
`pyrs-schema`; `default=["std"]`, `std=["indexmap/std","pyrs-ast/std"]`. **No `pyrs-event`, no
`pyrs-yaml-core`.**

Selection (three options, cost stated for each):

| Option | What it is | Score | Cost of choosing it |
|---|---|---|---|
| **A. Self-written restricted scanner (chosen)** | ~600-900 LOC tokenizer for the XML subset actually needed; DTD/entity refusal at token boundary | 9/10 | We own XML well-formedness conformance; must pin every edge with fixtures. No external CVE surface, no lockfile churn. |
| B. `quick-xml 0.42` | Mature pull parser, `Event::{Start,End,Text,CData,Comment,PI,DocType,GeneralRef}` | 3/10 | **Disqualified by no_std**: cached crate manifest says `edition 2024`, `rust-version 1.86`, feature list has no `std` toggle, `src/lib.rs` uses `std::borrow::Cow` unconditionally with no `#![no_std]`; upstream issue #390 states "quick_xml does not support no_std mode". Adding it removes `pyrs-xml` from the seven-crate foundation and breaks `tests/test_no_std_gate.py`. Also brings `Event::DocType`/`GeneralRef` that would have to be rejected *after* tokenisation. |
| C. `xml-rs` / `roxmltree` | Full DOM/SAX XML conformance | 2/10 | std-only, larger dependency tree, and their entity resolvers are exactly the machinery §5 must not have. |

Layered dependency direction (one way, no cycles):

```text
CLI / Python binding  (consumers)
        |
        v
   mapping.rs  -- XmlOptions default + validate, XmlNode -> CustomNode
        ^                              |
        |                              v
   model.rs  <--- scanner.rs      pyrs-ast / pyrs-schema / indexmap
   (stack, caps)   (bytes -> Token)
```

Capability seam closure (no orphan "future extension" hooks):

| Capability | Definition | Implementation | Consumer | Default | Failure semantics |
|---|---|---|---|---|---|
| XML text → AST | `from_xml_with_options` | `scanner`+`model`+`mapping` | CLI, Python | comments/PI on, depth 256, nodes 1,000,000 | `XmlError`, never a partial tree |
| Comment/PI retention | `XmlOptions.comments` / `.processing_instructions` | `mapping` part emitters | CLI flags, Python kwargs | `true` | `false` drops nodes, no error |
| Resource caps | `XmlOptions.max_depth` / `.max_nodes` | `model` push guard | `from_xml_with_options` | 256 / 1,000,000 | `E-XML-011` / `E-XML-012` |
| DTD / entities | **no capability** | none by design | none | always refused | `E-XML-001` / `E-XML-002` / `E-XML-003` |

Terms that must be understood before implementing:

```text
[术语] `+` 前缀
[人话] YAML 键前面加一个加号，标明"这是 XML 的结构信息，不是元素名"
[决定] 所有保留键以 + 开头；+ 不在 XML Name 生产式里，永不与真实元素名冲突
[代价] 中：切片 3 之前改拼写只需改本文档；之后要改 golden fixtures + 四语言文档 + stub

[术语] infoset-preserving
[人话] 元素、属性、文字、注释、处理指令都保住；尖括号怎么排版不保
[决定] 六项词法规范化写进文档，不作为 bug 处理
[代价] 中：写回时不能承诺 XML 字节级一致，只能承诺结构一致

[术语] no_std 基础 crate
[人话] 一个没有操作系统、没有标准库的嵌入式目标也能编译的 crate
[决定] pyrs-xml 声明 #![no_std]，并加入 CI 三处 -p 列表
[代价] 高：选了 std-only 解析器就得放弃这个属性，门禁立刻变红
```

## 3. Public API contract

```rust
// crates/pyrs-xml/src/lib.rs
#![no_std]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod scanner;
pub mod model;
pub mod mapping;

pub use error::XmlError;
pub use mapping::{from_xml, from_xml_with_options, XmlOptions};
// No `writer` module. No `to_xml` symbol. No `pub fn to_*` at all
// (keeps SERIALIZER_RE at quality_matrix.py:41 silent for this crate).
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlOptions {
    /// Retain comments inside the root element as `+c_<n>` parts. Default: true.
    pub comments: bool,
    /// Retain processing instructions inside the root element as `+p_<n>` parts. Default: true.
    pub processing_instructions: bool,
    /// Maximum element nesting depth. Default: 256. Must be >= 1.
    pub max_depth: usize,
    /// Maximum number of materialised XML nodes. Default: 1_000_000. Must be >= 1.
    pub max_nodes: usize,
}

impl Default for XmlOptions { /* comments:true, processing_instructions:true, 256, 1_000_000 */ }
impl XmlOptions {
    /// Explicit resolve/validate step. Called by `from_xml_with_options` before the scanner runs.
    /// A zero cap is a configuration error, not a silent unlimited fallback.
    pub fn validate(&self) -> Result<(), XmlError>;
}

pub fn from_xml(text: &str) -> Result<CustomNode, XmlError>;
pub fn from_xml_with_options(text: &str, options: &XmlOptions) -> Result<CustomNode, XmlError>;
```

`XmlError` follows `CsvError` (`crates/pyrs-csv/src/error.rs:23-133`): stable `code()`,
`is_parse_error()` (always `true` here — there is no writer), `Display` naming a position, and
`impl core::error::Error`.

| Code | Variant | Meaning |
|---|---|---|
| `E-XML-001` | `DtdDeclared { line, column }` | `<!DOCTYPE` token seen; refused at the markup boundary |
| `E-XML-002` | `EntityDeclaration { line, column }` | `<!ENTITY` token seen (including a standalone one) |
| `E-XML-003` | `UndeclaredEntity { name: String, line, column }` | named reference outside the five predefined ones |
| `E-XML-004` | `MalformedMarkup { line, column, detail: String }` | unclosed tag, bad name, bad comment (`--` inside), bad PI, unsupported `<!` form |
| `E-XML-005` | `MismatchedEndTag { expected: String, found: String, line, column }` | end tag does not match the open tag literally |
| `E-XML-006` | `MultipleRoots { line, column }` | a second root-level element |
| `E-XML-007` | `UnsupportedDocumentNode { kind: String, line, column }` | comment or PI outside the root element |
| `E-XML-008` | `UnsupportedDeclaration { detail: String }` | XML declaration other than version 1.0 / encoding UTF-8 |
| `E-XML-009` | `DuplicateAttribute { name: String, line, column }` | same attribute name twice on one element |
| `E-XML-010` | `InvalidCharacterReference { text: String, line, column }` | numeric reference outside XML 1.0 valid character ranges (incl. surrogates) |
| `E-XML-011` | `DepthLimitExceeded { limit: usize }` | `max_depth` exceeded |
| `E-XML-012` | `NodeLimitExceeded { limit: usize }` | `max_nodes` exceeded |
| `E-XML-013` | `UndeclaredPrefix { prefix: String, line, column }` | QName prefix with no in-scope `xmlns:` binding |
| `E-XML-014` | `InvalidOptions { field: &'static str, detail: String }` | `validate()` failed before any input was read |

## 4. XML → YAML mapping semantics

### 4.1 Content preprocessing (applied before any mapping decision)

1. Strip a leading UTF-8 BOM (`EF BB BF`); input is otherwise `&str` only — no transcoding.
2. Consume the XML declaration `<?xml ... ?>` if present. Accept only `version="1.0"` (or omitted)
   and `encoding="UTF-8"` (or omitted, case-insensitive); otherwise `E-XML-008`. The declaration is
   **not** emitted (normalisation 1).
3. Build an ordered `parts` list per element: adjacent text/CDATA runs are merged (normalisation 2);
   a text part consisting only of XML whitespace (`U+0020`, `U+0009`, `U+000D`, `U+000A`) is
   **dropped iff the element has at least one element child** (normalisation 3 — this is what keeps
   pretty-printed XML readable). If the element has no element children, whitespace text is data and
   is kept.
4. Document-level comments/PIs outside the root: `E-XML-007` (see §4.7).
5. Every part carries its 0-based global occurrence index among preserved comments/PIs, used for
   `+c_<n>` / `+p_<n>` uniqueness.

### 4.2 Element value shapes

Let `element` have attributes `A`, content parts `P` (after §4.1).

| Condition | YAML shape |
|---|---|
| `A` empty, `P` empty | `CustomNode::Null` — covers `<a/>` and `<a></a>` (normalisation 4) |
| `A` empty, `P` = one text part | scalar via `xml_scalar(text)` — **no `+content` wrapper** |
| `A` non-empty, `P` empty | mapping with only `+@name` entries |
| `A` non-empty, `P` = one text part | mapping with `+@name` entries plus `+content: <scalar>` |
| `P` contains only element children (no text/comment/PI) | mapping with `+@name` entries plus one key per child name, in document order; repeated sibling names become a sequence under that one key |
| any other case (comment, PI, or text mixed with elements) | mapping with `+@name` entries plus `+content: <sequence of parts>` |

The text-only short circuit is deliberate: wrapping `<name>Ada</name>` as `{+content: Ada}` loses
the readability that makes this convention usable by hand.

### 4.3 Parts in a `+content` sequence

- character data → scalar string via `xml_scalar`
- child element → single-entry mapping `{ "<qname>": <element value> }`
- comment → `{ "+c_<n>": <comment text as scalar> }` (`+c_` prefix)
- PI → `{ "+p_<n>": { target: <target>, data: <data-or-empty-string> } }` (`+p_` prefix); `target`
  equal to `xml` in any case is `E-XML-004`

Because any comment/PI forces the mixed path, **all** sibling nodes go into `+content` — order is
never encoded by two separate key groups.

### 4.4 Attributes and namespaces

- Attribute name `name` → key `+@<name>`; value is a scalar string via `xml_scalar`. Attribute
  values are never type-inferred into numbers/bools; they are strings that YAML must read back as
  strings.
- Duplicate attribute name on one element → `E-XML-009` (a mapping cannot hold both).
- QNames are preserved **lexically**, including prefixes: `<p:outer xmlns:p="urn:x">` yields key
  `p:outer` and attribute `+@xmlns:p: "urn:x"`. No URI resolution, no prefix rewriting, no
  default-namespace expansion (normalisation 5).
- A prefix used in an element or attribute name must have an in-scope `xmlns:<prefix>` binding; the
  implicit `xml` prefix is allowed. Otherwise `E-XML-013`. Prefix scope is a stack on `model.rs`.

### 4.5 Scalars and keys (the collision trap)

`xml_scalar(s)` returns a `ScalarStyle`:

- `DoubleQuoted` when `s.is_empty()`, or `pyrs_schema::plain_text_is_typed(s)` (so `007`, `true`,
  `null`, `+7`, `yes` stay strings), or the first byte is a YAML indicator
  (`- ? : , [ ] { } # & * ! | > ' " % @ \`` plus U+FEFF), or `s` starts/ends with whitespace, or contains `\n`, `\r`, `\t`.
- `Plain` otherwise.

Element/attribute **keys** are additionally `DoubleQuoted` when they contain `:` or start with an
indicator; reserved keys (`+content`, `+@...`, `+c_<n>`, `+p_<n>`) are plain-safe by construction.
This is what keeps `+@id: "7"` valid while a hand-written `@attr:` would be a parse error and
`#text:` would be a comment.

Entity handling in text/attribute values: only `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and
numeric character references `&#n;` / `&#xH;` are decoded; every other `&name;` → `E-XML-003`.
Numeric references are validated against XML 1.0 valid character ranges → `E-XML-010`. Decoded text
is never re-scanned for markup (normalisation 6: `&lt;b&gt;` stays the string `<b>`, not an
element).

### 4.6 Golden example

Input:

```xml
<root id="7"><name>Ada</name><item>a</item><item>b</item><!--note--><?pi go?><p>Hello <b>x</b>!</p><empty/></root>
```

Output:

```yaml
root:
  +@id: "7"
  +content:
    - name: Ada
    - item: a
    - item: b
    - +c_0: note
    - +p_1:
        target: pi
        data: go
    - p:
        +content:
          - "Hello "
          - b: x
          - "!"
    - empty:
```

Note the comment/PI force the mixed path, so `name` and `item` appear as parts rather than direct
keys. Without them, the same document would be
`root: {+@id: "7", name: Ada, item: [a, b], p: {+content: [...]}, empty: null}` — the more readable
shape.

### 4.7 Document-level nodes

XML allows comments and PIs before/after the root element. Slice 2 refuses them with `E-XML-007`
rather than introducing a `+doc` wrapper around every document. Rationale: the wrapper would make
every output less readable to serve a rare prolog case, and there is no way to encode
before/after-root position in a flat `+c_<n>` key without a second ordering key. If the maintainer
requires them, that is a contract change: a wrapper key must be added and the golden fixtures
regenerated — do not implement it locally.

## 5. Security boundary (structural, not parse-then-filter)

Attack surface: DTD/internal-subset parsing + entity expansion + external fetch. `pyrs-xml` contains
**none** of these capabilities.

**Rejection point.** In `scanner.rs`, at every `<` that begins markup, classify the following bytes
*before* reading any content:

- `<!--` → comment token
- `<![CDATA[` → CDATA token
- `<!D`/`<!d` followed by case-insensitive `OCTYPE` → `E-XML-001` **immediately**
- `<!E`/`<!e` followed by case-insensitive `NTITY` → `E-XML-002` **immediately**
- any other `<!` → `E-XML-004`

The error carries line/column and the scanner returns; no internal-subset scan, no `SYSTEM`/`PUBLIC`
interpretation, no buffered DTD body. There is no code path that could open a file or socket — the
crate has no I/O dependency and takes `&str`, not a reader.

**Entity expansion.** With no DTD there is no declaration source for a custom entity, so `&name;`
outside the five predefined names is `E-XML-003` at the first byte. Predefined/numeric references
decode to characters in one step, are never recursively rescanned, and cannot produce markup. This
is what makes billion laughs impossible by construction: the primitive it needs (a recursively
expandable declaration table) does not exist.

**Resource caps.** `max_depth` 256 and `max_nodes` 1,000,000 are checked on every stack push/node
creation, so deeply nested or wide documents fail with `E-XML-011` / `E-XML-012` before allocation
grows. `XmlOptions::validate()` rejects zero caps before reading input (`E-XML-014`); there is no "0
means unlimited" fallback and no feature flag that relaxes any of this.

**Why not parse then filter.** Filtering after tokenisation still expands entities first, so a
10-level bomb is materialised before the filter sees a `DOCTYPE`; a 2 MB input can become gigabytes.
Refusing at the `<!` boundary removes the expansion step entirely.

**Oracle caveat.** Python's `xml.etree.ElementTree` *does* expand internal entities. The crosslib
tests must never pass DTD/entity fixtures to the oracle; those fixtures are matched against exact
`XmlError` codes only. This is a test-design requirement, not a detail.

## 6. CLI contract

```text
pyq from-xml [file] [--get <path>] [--json] [--raw] [--no-comments] [--no-pi]
```

No `to-xml`. `--no-comments` / `--no-pi` map onto `XmlOptions.comments` /
`.processing_instructions`. Output goes through the existing
`emit_selected(node, &get, &verbs, json, raw)` tail (`crates/pyrs-yaml-cli/src/lib.rs:847`). Exit
codes: `0` success, `1` with `pyq: <message>` on stderr for a refusal, `2` for
compatibility-shell "refused by name".

Sync points (all verified):

1. `lib.rs:36` — add `from-xml` to the native verb list in `DIALECT_HELP`.
2. `lib.rs:66-348` — add `Command::FromXml { file, get, verbs, json, raw, no_comments, no_pi }` next
   to `FromIni`/`FromCsv`.
3. `lib.rs:362-382` — add `Format::Xml` to the `Format` enum (needed by `validate`, `diff`, `merge`,
   `--input xml`).
4. `lib.rs:793` — add the `Command::FromXml` dispatch arm: build `pyrs_xml::XmlOptions`, call
   `pyrs_xml::from_xml_with_options`, then `emit_selected`.
5. `lib.rs:1032-1037` — add `Format::Xml` to the non-streamable list for `--all-docs`.
6. `lib.rs:1080-1092` — map extension `xml` → `Format::Xml`; `:1095-1118` — add `Format::Xml`
   dispatch (reader only).
7. `crates/pyrs-yaml-cli/Cargo.toml:20` — add
   `pyrs-xml = { path = "../pyrs-xml", version = "0.19.1" }`.
8. `compat.rs:30-52` — add `"from-xml"` to `NATIVE_COMMANDS`; keep all `--xml-*` flags at `:383-389`
   in the refusal list (they are yq flags, not pyq flags), and add each of them to the refusal-name
   test list at `:885-890`. `DIALECT_HELP:49` already names the refused `--xml-*` family — keep it.

## 7. Python API contract

```python
pyrs_yaml.from_xml(xml_str: str, comments: bool = True, pi: bool = True) -> str
pyrs_yaml.load_xml(xml_str: str, comments: bool = True, pi: bool = True) -> object
```

No `to_xml`. Both raise the repository's `YamlParseError` carrying the localized `xml-parse-error`
message with the engine's stable code in the detail. `from_xml` returns YAML text; `load_xml`
returns the Python value graph (dict/list/str/None) through `resolve_tags` +
`node_to_pyobject_resolving_anchors`, mirroring `load_csv` (`functions.rs:964-965`).

Rust signatures follow the house style exactly:

```rust
#[pyfunction]
#[pyo3(signature = (xml_str: "str", comments: "bool" = true, pi: "bool" = true) -> "str")]
pub(crate) fn from_xml(xml_str: &str, comments: bool, pi: bool) -> PyResult<String> { /* from_xml_with_options -> yaml_of_node */ }

#[pyfunction]
#[pyo3(signature = (xml_str: "str", comments: "bool" = true, pi: "bool" = true) -> "object")]
pub(crate) fn load_xml(py: Python, xml_str: &str, comments: bool, pi: bool) -> PyResult<Py<PyAny>> { /* ... */ }
```

Sync points: `functions.rs` (new functions beside `from_csv:883` / `load_csv:939`);
`py/mod.rs:195-255` (`#[pymodule_export]` for both); `python/pyrs_yaml/__init__.py:59,457` (import +
`__all__`); `python/pyrs_yaml/pyrs_yaml.pyi` (stubs beside `:451`); `crates/pyrs-yaml/Cargo.toml:26`
(`pyrs-xml` dep); four locale catalogs (`xml-parse-error` at line 8 of each);
`docs/{en,ja,ko,zh}/api/reference.md`, `guides/formats.md`, `changelog.md`, root `CHANGELOG.md`.
`scripts/check_doc_signatures.py` reads the built extension, so signatures must match the real
functions, not only the stub.

## 8. Quality matrix contract

Add one row to `FORMAT_ROWS` (`scripts/quality_matrix.py:239-255`):

```python
FormatRow("XML", "pyrs-xml", "xml", "xml", ("xml",), writer=False)
```

`crosslib_stem="xml"` makes the probe look for `tests/test_xml_crosslib.py` (composition at `:823`)
— that filename is mandatory. `writer=False` makes the round-trip cell derive as `n/a` at `:1321`.
**Do not** use `no` here: CSV's row is `no` because `pyrs-csv` *does* own a writer (`to_csv_text`)
but has no `roundtrip` fuzz target (`:1321`); XML owns no writer at all in slice 2, so the invariant
does not exist to be untested.

Three more script edits:

- `ORACLE_IMPORTS` (`:260-272`): add `"xml.etree.ElementTree": "xml.etree.ElementTree"`. The oracle
  module must use `import xml.etree.ElementTree as ET` (the `_IMPORT_LINE` regex at `:283-286`
  handles dotted names + alias); `from xml.etree import ElementTree` is **not** detected and would
  silently drop the parity cell.
- `STDLIB_SINCE` (`:281`): add `"xml.etree.ElementTree": (3, 0)` — the floor is `>=3.8`
  (`pyproject.toml:11`), so the reader is always present. Otherwise `_reader_bounded` (`:394-405`)
  falls through to `_dependency_marker`, which reports `undeclared` for a package nobody imports.
- `DIALECT_CALL` (`:429`): add `xml` to the suffix alternation, e.g.
  `(json5|jsonc|json|toml|yaml|csv|xml)`. Without it, `pyrs_yaml.from_xml` / `load_xml` calls in the
  crosslib body are not credited and the parity cell reads `partial` instead of `yes`.
- `tests/test_quality_matrix.py:1213`: add `XML` to the label alternation; `:1223`: change
  `len(claimed) == 6` to `== 7`.

`QUALITY_MATRIX.md:96-106` derived table gains exactly:

```text
| XML | yes | no | no | n/a | yes | yes |
```

Cell rationale: `unit=yes` (crate unit + golden/security tests); `property=no` (no proptest tier in
slice 2); `fuzz-parse=no` (no fuzz target; adding one is allowed but must also be registered in
`fuzz/Cargo.toml` and `.github/workflows/fuzz.yml`); `round-trip=n/a` (no writer); `parity=yes`
(stdlib ElementTree oracle, run on every leg); `authority=yes` (`tests/test_xml_spec_table.py`,
matched by fragment `xml` + `_spec_table.py` at `:601,:832-836`).

## 9. Test matrix

- **Crate unit tests** (`mapping.rs`, `scanner.rs`, `model.rs`): token classification, QName
  validation, prefix stack, cap boundaries, scalar quoting (`007`, `true`, `null`, `+7`, `Ada`),
  attributes-only elements, repeated siblings, whitespace-only text, `+c_`/`+p_` indices, all 14
  error codes.
- **Golden corpus** `crates/pyrs-xml/tests/mapping_golden.rs` + `tests/fixtures/`: `basic.xml`,
  `attrs.xml`, `text_only.xml`, `repeated_children.xml`, `mixed_content.xml`, `namespaces.xml`,
  `cdata.xml`, `comments.xml`, `pi.xml`, `empty.xml`, `bom.xml`, `non_ascii.xml`, `crlf.xml`,
  `numeric_ref.xml`, `predefined_entities.xml`. Each fixture has a committed `.yaml` golden; the
  test compares `from_xml` output exactly.
- **Security refusals** `crates/pyrs-xml/tests/security_refusals.rs`: `dtd_doctype.xml`,
  `internal_entity.xml`, `external_entity.xml`, `billion_laughs.xml` (10-level nested entity bomb),
  `undeclared_entity.xml`, `standalone_entity.xml`, `unsupported_bang.xml`, `undeclared_prefix.xml`,
  `duplicate_attr.xml`, `depth_bomb.xml`. Assert exact code + line/column. The bomb fixture also
  asserts the error is returned before any node is materialised (`max_nodes` counter unchanged).
- **Cross-library oracle** `tests/test_xml_crosslib.py`: `import xml.etree.ElementTree as ET`; parse
  commits with `ET.XMLParser(target=ET.TreeBuilder(insert_comments=True, insert_pis=True))`;
  normalise the six lexical differences + whitespace-only-text drop from §4.1 before comparing
  ordered children, QNames, attribute order/values and text. DTD/entity fixtures are **excluded**
  from the oracle (§5).
- **Spec table** `tests/test_xml_spec_table.py`: one row per XML 1.0 well-formedness clause
  exercised (matching end tag, single root, quoted attribute values, no duplicate attributes, legal
  Name chars, no `--` in comments, no `xml` PI target), with ElementTree as second implementation.
- **CLI** `crates/pyrs-yaml-cli/tests/cli.rs`: `from-xml` on a fixture, `--get`, `--json`,
  `--no-comments`, `--no-pi`, exit codes 0/1, and that `to-xml` is an unknown subcommand.
- **Compat** `compat.rs:885-900` test list extended with every `--xml-*` flag from `:383-389`.
- **Python** `tests/test_xml_api.py`: `from_xml` / `load_xml`, default and explicit flags,
  `YamlParseError` on each refusal family, and `assert not hasattr(pyrs_yaml, "to_xml")`.
- **Gates** `tests/test_no_std_gate.py` (CI three lists + two docs), `tests/test_quality_matrix.py`
  (7 rows), `scripts/check_doc_signatures.py`, wrapping/headings/changelog scripts.

## 10. Out of scope for slice 2

- YAML → XML write-back (`to_xml` / `to-xml`) — slice 3.
- DTD of any kind, external entities, entity declarations, entity resolvers — refused (§5),
  never "supported later behind a flag" in this slice.
- XPath, XSLT, XSD validation, namespace URI rewriting, XML Schema type inference.
- XML 1.1, non-UTF-8 encodings, UTF-16 decoding, DTD-defined default attributes.
- Editing substrate: no `pyq set` into XML, no comment/style splice engine — YAML stays the only
  editable representation.
- Document-level comments/PIs (§4.7).

## 11. The three traps most likely to bite the implementer

1. **XML strings re-typing as YAML scalars.** `<n>007</n>` emitted plain reads back as integer 7; an
   attribute `id="true"` reads back as a boolean; an element named `xmlns:p` produces a key with `:`
   that can break plain-scalar parsing. Use `pyrs_schema::plain_text_is_typed` plus the
   leading-indicator rule for **every** scalar and key (§4.5). Skipping this silently changes data
   rather than erroring.
2. **Pretty-print whitespace turning everything into mixed content.** Without the
   whitespace-only-text drop rule (§4.1.3), an indented document puts `\n  ` parts between every
   child, forcing the `+content` sequence path and producing unreadable golden YAML — and the
   ElementTree oracle must apply the same rule or every parity test fails. The rule is conditional
   on "element has ≥1 element child" so that `<a> </a>` keeps its significant space.
3. **Entity rejection at the wrong time.** A `parse then strip DTD nodes` implementation lets a
   10-level entity bomb expand before the filter runs. Refuse at the `<!` classification step (§5),
   and never hand a DTD fixture to the ElementTree oracle (it expands). A second-order version of
   the same trap: allowing `&foo;` "as literal text" instead of `E-XML-003` makes an undeclared
   reference indistinguishable from a suppressed entity definition.

## 12. Acceptance criteria (measured, not asserted)

1. `<!DOCTYPE ... [<!ENTITY ...>]>` and a 10-level billion-laughs input each return `E-XML-001` /
   `E-XML-002` with line/column before any node is allocated; the crate contains no `std::fs`,
   network, or I/O dependency usage (grep-verified in review).
2. Every golden fixture maps to its committed YAML exactly, and `tests/test_xml_crosslib.py` agrees
   with ElementTree on 100% of the non-DTD corpus after the documented normalisation.
3. Golden YAML contains `+@attr`, `+content`, `+c_<n>`, `+p_<n>`; no emitted key starts with a bare
   `@` or is `#text`; a text-only element with no attributes stays a plain scalar.
4. `cargo build --no-default-features --target thumbv7em-none-eabi -p pyrs-xml` passes, and
   `python -m pytest tests/test_no_std_gate.py` is green with `pyrs-xml` in all three CI crate lists
   and both live documents.
5. `QUALITY_MATRIX.md` derives a 7-row table with `| XML | yes | no | no | n/a | yes | yes |`, and
   the parity probe finds `tests/test_xml_crosslib.py` (not any other filename).
6. `pyq from-xml` works; `pyq to-xml` and `pyrs_yaml.to_xml` do not exist; each yq `--xml-*` flag is
   refused by name with exit 2.

## 13. End-to-end verification

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --release
cargo build --no-default-features --target thumbv7em-none-eabi -p pyrs-xml
python -m pytest tests/test_xml_crosslib.py tests/test_xml_spec_table.py \
    tests/test_no_std_gate.py tests/test_quality_matrix.py tests/test_xml_api.py
python scripts/quality_matrix.py --check
python scripts/check_doc_wrapping.py <changed docs>
python scripts/check_doc_headings.py <changed docs>
python scripts/check_doc_signatures.py
python scripts/check_changelog_coupling.py --paths <changed paths>
python scripts/check_changelog_mirrors.py
```

Manual smoke: `pyq from-xml fixture.xml | pyq get '.root'`; feed the same fixture to `from_xml` and
`load_xml`; attempt `pyq to-xml` and confirm the unknown-subcommand error.
