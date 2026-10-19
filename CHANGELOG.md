# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **The aarch64 wheel legs build on native arm64 runners** — `publish.yml`'s `linux` and
  `musllinux` aarch64 rows move from `ubuntu-22.04` under qemu to `ubuntu-24.04-arm`, and the
  glibc leg runs its suite there through `uv` instead of emulation. s390x is the only wheel suite
  still emulated; the `pyq` aarch64 cross-build, armv7 and ppc64le are unchanged.

- **`main`'s post-merge job now proves the claim it used to print** — the only job trunk runs for a
  code change asserted in an `echo` that the pushed tip's "identical tree already passed the full CI
  matrix", with nothing measuring it and no path that could fail. `scripts/check_main_provenance.py`
  asks that sentence's three questions against the API — which pull request produced the commit,
  whether the pushed tree is one of that pull request's commit trees, and whether
  `Test matrix (all legs)` reported `success` for the commit carrying it — and refuses otherwise.
  The content claim held on every tip measured (`07f988fa`, `60f670d0`, `225cf373`), so the hole
  was in the shape of the defence. The gate's own first trunk run then went red for the wrong
  reason, and the fix is part of the same change: a commit can carry several records of that check
  name, the API does not list them oldest-first, and a `workflow_dispatch` run of this workflow
  leaves a `skipped` beside the pull request's real verdict — its verdict job only runs on
  `pull_request`. Reading the last row read that absence as a refusal. `newest_verdict` drops the
  records that judged nothing and sorts the rest by the time each job finished, so `fddcc76e`
  passes and seven ancestors still refuse, each for a verdict that exists.

- **The per-format defence table is derived, and two of its cells were wrong** —
  `scripts/quality_matrix.py` now computes `QUALITY_MATRIX.md`'s format table from the files
  that declare the defence (the fuzz targets, the `proptest!` blocks, each `test_*_crosslib.py`
  and the dependency markers in `pyproject.toml`), and `tests/test_quality_matrix.py` pins every
  cell. Measured against it, `schema validation` was under-reported — `validate_schema.rs`
  parses before it validates, so its `fuzz parse` cell is `yes` — and the property tier is 27,
  not 25, because the count demanded a `prop_` name while `pyrs-event`'s two properties are
  named in prose. `partial` in the parity column now means a declared interpreter or
  architecture bound on the oracle, not an impression.

- **JSON5 `\xHH` escapes parse — the reader had every other JSON5 escape** — `load_json5` and
  `from_json5` accepted `\v`, `\0` and the line continuation but rejected `\x` followed by two hex
  digits, which the JSON5 escape set includes, while the shipped stub already said `load_json5`
  "Accepts the full JSON5 grammar". Measured against the reference: pyjson5 gives `{'a': 'A'}` for
  `{a: '\x41'}` and pyrs reported "invalid escape sequence". The two digits are now accumulated one
  byte at a time — `from_str_radix` would accept a leading `+`, making `\x+1` a valid escape — and
  strict JSON and JSONC keep rejecting the form. Rust tests pin both quote styles, `\x0A` and
  `\x00`, the truncated and non-hex tails, and a malformed escape straddling a multibyte character;
  a pyjson5 parity test pins the acceptance and both narrower dialects' refusal.

- **JSON5 escaped keys and every line terminator, plus the table that keeps them** — two
  productions the same stub promised were still missing: a backslash-u escape inside an unquoted
  key, which the ES5 `IdentifierName` grammar JSON5 inherits allows wherever an identifier
  character may appear, and a backslash before CRLF, LS or PS inside a string. The continuation
  gap was also a disagreement between the two scanners of one dialect — the CRLF form parsed in a
  double-quoted string and failed in a single-quoted one. An escaped character is now taken as the
  code point it stands for instead of being re-checked against `ID_Start`/`ID_Continue`, which is
  what makes an escaped digit or dot a legal key while the literal character is not, and both
  quote styles now share one terminator rule. The audit that found them is a gate as well:
  `tests/test_json5_spec_table.py` compares 41 accepted and 11 rejected productions against
  `pyjson5`, because a syntax the reader never accepted is invisible to the property and fuzz
  tiers, whose oracles can only compare paths that already exist.

- **A flow mapping cannot spell a block key, and the generator no longer draws one** — a random
  20k-case property-tier run went red on `{? [null]: A, ? [null]: B}`: a mapping holding the same
  one-item sequence twice, spelled block once and flow once. Under a block parent that pair really
  is two keys — `? - null` beside `? [null]` re-reads as two entries, measured — but inside `{...}`
  a block container has no legal form, so the writer emits the flow token for both and the reader's
  duplicate-key refusal is the correct answer. The tree was the generator's rather than a
  document's: `arb_mapping` filtered its keys with a context-free rule. `keys_collide_in_flow`
  folds both keys to the only spelling a flow parent allows before comparing them, normalizing a
  copy so `KeyIdentity` stays the single model of what a spelling means, and the agreement pin now
  re-asks its whole table under a flow parent, holding the one direction the generator is
  accountable to: a pair the rule lets through has to survive the reader.

- **The defence matrix now measures its external authorities** — the per-format table in
  `QUALITY_MATRIX.md` gains a sixth column, `external authority`, derived by
  `scripts/quality_matrix.py`: `yes` when a `tests/test_*_suite.py` or
  `tests/test_*_spec_table.py` exists for that dialect. Until now the table credited the unit,
  property, fuzz and parity tiers only, so the strongest gates in the repository —
  `test_yaml_suite.py`, `test_json_test_suite.py`, `test_toml_test_suite.py` and
  `test_json5_spec_table.py` — were invisible on the page that lists the defences, while parity,
  which compares two implementations, was credited as if it asked the grammar.
  `schema validation` reads `no`, because nothing outside asks its rules.
  `tests/test_quality_matrix.py` asserts the column cell-by-cell and names one file per row, so
  dropping a suffix from the derivation reddens the test instead of quietly deleting a column.

- **A JSON string of nothing but a Unicode space kept its value** — `load_json5` answered
  `{None: 1}` for a document whose key is U+2028 alone, while `load_json` and `load_jsonc` gave back
  the string and `pyjson5` agreed with them. The hub stores a decoded JSON string as a YAML scalar,
  plain unless `needs_quotes` says otherwise, and the JSON resolver trims a plain scalar with
  Unicode rules — `resolve_json_type` keeps `str::trim` deliberately, because JSON5's structural
  whitespace really is that wide — so a text beginning or ending with a non-ASCII space lost that
  character and a text made only of one resolved to null. The quoting rule knew ASCII space and
  NBSP; it now knows both ends of all 25 Unicode `White_Space` code points, which is where the two
  tables stop disagreeing. It is asked on the character path the byte pass already reaches for any
  text holding a non-ASCII byte, so a text that is pure ASCII pays nothing.
  `tests/test_json_whitespace_scalar.py` sweeps the set in key and value position through the three
  dialects and asks `pyjson5` and `json` that the key is a string and not null.

- **A JSON5 string can hold a raw control byte** — the double-quoted scanner stopped on every
  byte below 0x20 and refused it, which is RFC 8259 §7's rule for strict JSON rather than JSON5's:
  the dialect forbids only a line terminator inside a quote. A document holding a real tab was
  therefore refused by `load_json5` while `pyjson5` read it, and this file's own single-quoted
  string had always copied the same byte through — one dialect answering twice, decided by which
  quote opened the string. The stop set is now the dialect's: JSON5 takes every C0 byte except CR
  and LF, strict JSON and JSONC keep refusing all 32, and the JSON5 writer escapes what it read,
  so the round trip is still one line. `tests/test_json5_spec_table.py` asks all 30 in both quote
  spellings against the reference, and the two strict loaders are held to the RFC for the same
  bytes.

- **The JSON writers gained a reader they do not own** — the write side of the family was checked
  only against our own reader, and a fixed point is satisfied just as happily by an implementation
  that is wrong twice the same way. `tests/test_json_writer_suite.py` now generates 1,500 trees,
  emits each with `to_json`, `to_jsonc` and `to_json5`, and hands the document to Python's `json`
  and to `pyjson5` — strangers that have never seen the encoder — requiring the value back to be the
  tree that went in. Measured before it was pinned: 0 refusals, 0 value changes on all three
  routes. It bites: making `write_json_string` emit a raw tab reddens both strict legs while the
  JSON5 one stays green.

- **A YAML string spelled the way 1.1 spells a number arrived as a number** —
  `safe_dump({"v": "1_000"})` emitted `v: 1_000`, and PyYAML and ruamel both handed back the
  integer 1000. Same for `8_0`, `0b101`, `+0x1f`, for the legacy bool words `yes`/`on`/`off`/`y`
  and for the date-only timestamp `1990-06-23`, which comes back as `datetime.date`. Twenty-six
  spellings per route, in key position as well as value, on the Python route and on the JSON→YAML
  one. The quoting guard asked only `resolve_core_type`, which is this engine's own 1.2 core
  reader, and every one of those spellings is a string there; a reader on the YAML 1.1 type
  repository types them, and so does this engine's own `schema="1.1"` profile for `y` and `yes` —
  the writer and one of the profiles disagreed about the same token. The guard now asks the 1.1
  table as a second question over four families: the bool words core does not list, `_` inside a
  numeric, a base prefix core cannot see (`0b`, and `0x`/`0o` behind a sign), and `YYYY-MM-DD`.
  Quoting is the safe side of the error — a quoted scalar is never resolved — so each family is
  the shape the table accepts rather than a parser: `a_b`, `0x` and `2026-10` stay plain, and
  `12:00` needs none of this because a colon already refuses plain emission one layer below.
  Output changes only for these spellings: `1_000` now writes as `"1_000"`.

- **The YAML writers gained a reader they do not own** — `tests/test_yaml_writer_suite.py` emits
  through `safe_dump`, `from_json` and the edit layer and hands each document to PyYAML and to
  ruamel.yaml, requiring the value back to be the one that went in; 400 generated trees per route
  and every legacy lexeme in key and value position. Measured after the fix: 0 refusals and 0
  value changes on all three routes, which is what `QUALITY_MATRIX.md` now credits the YAML row
  for beside the official-suite reader gate. The edit layer is held to a narrower rule than the
  data routes, and the file pins both halves: a path segment *addresses* the key a reader resolves
  (issue #336), so `$.~` and `$.42` must keep their plain fold, while a name that would lose its
  type between the two profiles — `rename("$.v", "yes")` wrote a key that reads `true` — is quoted.
  Temporal objects stay outside: `!date` is a local tag a SafeLoader has no constructor for, so the
  strangers refuse that document rather than change a value, which is a boundary of the tag design.

- **The TOML writer gained a parser it does not own** — the last of the three write-side cells.
  `test_json_writer_suite.py` and `test_yaml_writer_suite.py` covered two families, and TOML's
  outside tiers (`test_toml_test_suite.py`, `test_toml_crosslib.py`) both ask the reader, so the
  writer was checked only against our own. `tests/test_toml_writer_suite.py` emits 400 generated
  trees through `to_toml` and `YamlDocument.to_toml()` and hands each document to `tomlkit` and, on
  Python 3.11+, `tomllib`. Measured: 1,074 of 1,200 probe trees emitted with 0 refusals and 0 value
  changes, and the 126 that never reached a stranger were one typed refusal,
  `toml-cannot-represent-null` — a value the format has no word for is refused, not given an
  invented spelling. Two boundaries are pinned rather than smoothed over: a `datetime.time` goes out
  as a bare local time, which is TOML 1.1's (1.0.0 has no local-time type and the readers here
  decode it), and an integer beyond i64 goes out quoted, because a string is what the hub calls it
  on every route. No behaviour changed; the gate is insurance against a dialect rule that reaches
  one direction of the pipe and not the other, which is the class two of the last three fixes were.

- **A verbatim `!<tag:yaml.org,2002:X>` reaches the plugin registered as `!!X`** — YAML 1.2 §6.8
  makes the two spellings one tag, and the loader already honoured both for the five tags it names
  by hand (`!!str`, `!!int`, `!!float`, `!!bool`, `!!binary`). For every other name the plugin
  registry is keyed on the rendered text, so the verbatim form fell through to schema resolution:
  measured on the trunk of 2026-10-18, `!<tag:yaml.org,2002:widget> 7` ignored a plugin registered
  as `!!widget` and loaded the integer `7`. The spelling the document used is tried first, so the
  second look is paid for only on a miss, and a URI outside YAML's namespace keeps its current
  reading — resolved as if the tag were absent, not raised.

- **The parity column now recognises an oracle it declares under another name** — the probe
  reads how `pyproject.toml` bounds each cross-library oracle and matched the literal import
  name: `ruamel.yaml` is imported that way and declared as `ruamel-yaml`, so the
  lookup answered `undeclared` and the YAML row reported an oracle installed on every supported leg
  as one the project never declared. The lookup normalises `-`, `_`, `.` and case the way PEP 503
  does, and `tests/test_quality_matrix.py` now refuses any `undeclared` marker, so an oracle the
  probe loses track of fails the run instead of misreporting itself.

- **A schema that omits `extends` resolves the way its documentation says** — `extends` is optional
  and `docs/en/guides/custom-schema.md` states its default as `core`, but the parser only reached
  Core when the key was present with an empty value. A schema that left the key out carried no base
  resolver, which typed every scalar as its own text, and both directions of the verdict inverted
  without complaint: `type: int` refused `port: 80` - the error said `expected int but got
  Str("80")` - and `type: str` accepted it. The Rust row that pinned the old default is now two
  rows, the second recording the boundary that remains: a base name this engine cannot find still
  leaves no fallback, and refusing a typo'd name is a separate call about a public error surface.

- **An edit that orphans an anchor is refused, not written** — on `base: &x 1` with `use: *x`,
  renaming that anchor, replacing its node without carrying its metadata, and moving the definition
  away all answered success and emitted a document whose only remaining reference names an anchor
  nothing defines; this library's own reader refused `base: &a0 1 / use: *x` with
  `found unknown anchor`. The rule was already in the engine — `with_metadata_from` keeps the
  document's anchor name when a value is replaced with its metadata — and these three doors bypassed
  it, so the question is now asked where the drop happens: at `set_path`, which `set_many_path` and
  `move_path` share, at `delete_path`, and at the anchor verbs. The guard is wider than the set of
  aliases that would really dangle: an alias binds the *nearest preceding* anchor of its name, so a
  rename that a later `&x` would have kept alive, and a subtree holding both an anchor and the alias
  naming it, are refused too. Over-refusing costs the caller one more edit; under-refusing writes a
  file nothing can read. Making a rename carry its aliases is a decision about what the verb means,
  and is recorded as one.

- **The edit layer has a fuzz target of its own** — the workspace has nine targets now, and the
  eight before it handed the engine a *document*; nothing drove a *change* to one, which is the
  layer the round-trip promise is about. `fuzz_targets/edit_yaml.rs` reads `document NUL script`,
  picks paths the reader really found, and requires every accepted edit to re-parse and to leave the
  value it wrote where it says it wrote it — with removal claims stated per kind, because a key is
  an identity while a sequence index is a position the next item shifts into. It reached the anchor
  defect above on its first seed replay, again minutes into its first exploration, and reported
  three shapes it records rather than asserts: a sort asked of a scalar answers "ok" and changes
  nothing, a note asked of a node does not always stay on that node, and a flow member carrying a
  tag and a note settles only on the second round - an open defect in the note's flow attribution,
  committed as a seed so the fix inherits the input.

- **A `date` or `datetime` now leaves the YAML writer under the tag every reader knows** —
  `safe_dump()` wrote `!date` and `!timestamp`, and a local tag has no constructor outside this
  library. Measured on the trunk of 2026-10-18: `yaml.safe_load` and `ruamel.yaml` both answered
  such a document with `ConstructorError`, so the value was not mistyped, it was unrecoverable.
  Both objects now leave as `!!timestamp` (`tag:yaml.org,2002:timestamp`), the one temporal tag
  YAML defines, which those readers resolve to a `date` for a date-only text and a `datetime` for
  anything with a time part; this reader makes the same fork, so what a stranger can type, we type
  identically. `!date` and `!timestamp` stay registered and keep resolving, so a document an
  earlier version wrote still reads back as the same object, and a document that arrived with one
  is re-emitted with it unchanged - only objects built from Python take the standard tag. `!time`
  remains a local extension: YAML defines no tag for a bare time of day and PyYAML cannot
  represent one. The hub tag TOML's own values carry is moved by the entry below.

- **TOML's dates and datetimes enter the hub under the same standard tag** — the projection in
  `crates/pyrs-toml/src/lib.rs` tagged a date-only value `!date` and a date-time `!timestamp`, so a
  document that merely passed through (load TOML, write YAML) left with the local spelling the
  entry above stops writing at the Python boundary. Both now carry `!!timestamp` and the load path
  forks on the text, so a TOML date still becomes a `date`, not a midnight `datetime`. It is
  visible in `parse_stream`, whose `tag` field for a TOML temporal is now `!!timestamp`; a bare
  time keeps `!time` as the one local temporal tag, and `!date` still reads.

- **A flow member whose value is a tag with no content keeps its note on the first round** —
  `{a: !t }` spells its value as nothing, so a note written after the tag has no body to sit on:
  the reader hands the note to the enclosing pair and the tag moves behind it, and the document
  settles only on its second emission. Nothing is lost across the round, which is why no other
  tier saw it. The writer now spells that empty body `''` when a note follows it — the same
  decision that writes `~` for an empty value before a note (libFuzzer `edit_yaml`
  crash-b23eed05).

- **A note on a block sequence item keeps the dash line** — an item mapping whose note is set
  through the edit layer had no slot on the `-` line, so the writer pushed the whole mapping onto
  its own lines; the reader then filed the note with the pair that ends the block and the next
  emission compacted the line back, so the text took two rounds to settle. The compact line now
  carries the item's note (`- a: 1  # n`), and when the pair line has already claimed YAML's one
  trailing slot the note moves above the dash (`# n` then `- a: 1  # m`) — both spellings are
  fixed points (libFuzzer `edit_yaml` crash-61474da6).

- **An empty plain scalar is spelled as the null it resolves to** — a scalar with no text and no
  properties rendered as nothing, and the reader does not hand the same node back from an empty
  line: `k:` and `-` both re-read as a null whose own spelling is `~`, so the first emission settled
  only on the second round (libFuzzer `edit_yaml` crash-63749496, reached from `- >` with its style
  set to plain). The writer now writes the null text wherever the body would be empty — nested
  value, root value, sequence item, flow pair value, flow item. A tagged empty is left alone: the
  tag suppresses resolution, so its body keeps the `''` spelling that settles it (#514). An anchor
  is not the same case: it names the node without suppressing resolution, so `- &a3` is now written
  `- &a3 ~` (libFuzzer `edit_yaml` crash-07b6a10e).

- **The TOML writer's dialect promise gained a property tier** — 1.1's one visible freedom is a
  newline (and so a comment) inside an inline table, and nothing asked whether a tree that carries
  no note was handed 1.0-readable text. `prop_toml_undecorated_output_loads_as_1_0`
  (`crates/pyrs-yaml-core/src/fmt_pbt.rs`) now requires that a generated tree with no note anywhere
  reloads under `from_toml_v1_0`, while
  `an_undecorated_inline_table_writes_compact_and_loads_as_1_0` and
  `the_1_0_reader_refuses_the_1_1_inline_table_freedom` (`crates/pyrs-toml/src/writer.rs`) pin the
  inline-table arm by shape and its oracle premise. Measured: forcing the multi-line branch first
  reddened only the named test and left 20 000 generated cases green, so the property's input gained
  a strategy that always draws a document holding an inline table — the same mutation now reddens
  the property in 0.01 s. A third property states the equality claim the tier was missing:
  `prop_toml_roundtrip_preserves_the_tree` compares the tree before and after one round - values,
  shape and order - and two value-dropping mutations reddened it (the text fixed point reddened in
  both runs too, so this is the claim stated directly rather than a strictly stronger oracle). No
  behaviour changed; the counts move to 720 Rust tests and 29 properties.

- **The edit tier now judges what an edited document exports** — a hub document is what
  `doc.to_toml()`, `doc.to_json5()` and the CLI's TOML/JSON output write, so "edit, then export" is
  the route a user takes, and every tier before this one had only handed those writers a *reader's*
  tree: `toml_roundtrip` and `json_roundtrip` never see a tree the splice layer rewrote.
  `edit_yaml.rs` now writes each settled tree in every dialect, compact and pretty alike, and asks
  the two questions those targets already ask — the dialect's own reader must accept the text, and
  its second emission must be the same bytes. A dialect that refuses the tree is a pass, because
  TOML has no top-level scalar and strict JSON refuses a non-finite float. The edit verbs stay
  YAML-only by design (`crates/pyrs-toml/src/lib.rs` says so), so this is one target asking seven
  exporters, not seven edit targets. The same two questions went into `json_roundtrip` for the three
  pretty JSON spellings, which that tier had never asked of text nobody chose to generate.

- **A `set` that replaces a container no longer drops a referenced anchor defined inside it** — the
  guard at `set_path` asked only about the replaced node's own anchor, and `preserve_metadata: true`
  copies exactly that one property, so writing over a mapping whose member carried `&x` deleted the
  only definition of `x` and left `use: *x` naming nothing: text this library's own reader refuses.
  The subtree question `delete_path` already asked is now asked here too, at the same place, and
  refuses with the same `anchor-is-referenced` reason (libFuzzer `edit_yaml` crash-fbd364b3, reached
  by a `set` on a path whose node carries no anchor of its own).

- **A note written on a tag-only sequence item keeps that line** — `- !fuzz2` spells its value as
  nothing, so a note set after the tag had no body to sit on: the reader filed it as the item's
  leading note and the next emission wrote it as a line of its own above the dash, so the text moved
  on the second round although nothing was lost. The writer now closes the pending tag line before
  an inline note, exactly as it already does before a note line, and the item is written
  `- !fuzz2 ~  # n` (libFuzzer `edit_yaml` crash-3dd60cad).

- **A signed zero reaches TOML as the zero TOML reads back** — the integer pass-through keeps a
  plain scalar's own spelling when that spelling is legal TOML, and `-0` is legal text. But TOML has
  no negative zero: `ist = -0` re-reads as `0`, so the second emission moved. A fixed point is
  spelled the way the reader spells it, so a signed zero writes as `0` while the radix spellings the
  reader really does keep (`0xDEAD`, `0b1`) pass through unchanged (libFuzzer `edit_yaml`
  crash-917e103b).

- **An emptied block sequence keeps its dash line** — removing the last item of a block sequence
  leaves the flow flag behind, and the compactness rule vetoed a compactness the emission does not
  have: an empty container has no block form, so it is always spelled `[]` on the key's line. The
  first emission was `-\n  t: []`, the reader answered `- t: []`, and the text moved on the second
  round with nothing lost (libFuzzer `edit_yaml` crash-a58caa38).

- **A JSON string spelled with a long integer keeps its quotes across the hub** — the Core resolver
  answers `Str` for a digit run too long to fit an `i64`, while JSON puts no bound on an integer
  literal, so `"9999999999999999999"` projected as a plain scalar and `to_json` wrote it back as a
  number: `load_json` kept a string and one trip through the hub made it an integer. The projection
  now asks JSON's own number grammar before writing a string plain, which is the question the writer
  will ask of it (libFuzzer `json_roundtrip` crash-09341b75).

- **The `schema="1.1"` profile resolves the numeric families 1.1 defines** — `1_000` is the integer
  1000, `0b101` is 5, `+0x1f` is 31, `+.inf` is infinity and `017` is 15, each the answer PyYAML's
  loader gives the same text. The profile used to add only the legacy booleans to Core, while the
  writer clause #504 had been quoting these families for a reader this engine did not have — so
  `schema="1.1"` disagreed with the engine's own quoting rule. Timestamps and sexagesimal stay text
  (the temporal-tag ruling keeps untagged text text; a colon already forces quoting one layer
  below), and `0o17` stays typed here while PyYAML reads it as text, because the profile is Core
  *plus* and `0o` is a Core prefix.

- **A string that only a 1.1 reader types is no longer emitted plain** — `0x_1f` is 31 to PyYAML and
  text to Core, and the clause answering "would a 1.1 reader type this?" checked underscores only on
  decimal tokens, so the Python string `"0x_1f"` went out plain and came back a number. The gap was
  found by a new invariant test that asks, of every text this profile types, that `needs_quotes`
  agrees.

- **The TOML writer's section header stopped materialising its key** — `to_toml` collected each
  root table's header text into a `String` of its own before writing it after every top-level pair,
  while the section list already carried the key node; the header arm can append through
  `push_scalar_key` exactly as a pair line does. That was the writer's only materialised key:
  `to_toml_medium` reads 26,741,057 counted instructions against its 28,939,722 floor, -7.60%, one
  allocation and one copy fewer per header.

- **A root table that TOML cannot place after its pairs keeps its position inline** — TOML admits
  no root key/value after a root table header, so `to_toml` wrote every section after every pair and
  the mapping's order did not survive the round trip: a tree with `0` holding a table and then `a`
  came back as `a`, `0`. The generator reached it at the 20 000-case property tier
  (`prop_toml_roundtrip_preserves_the_tree`); such a table is now written inline, at its own place,
  while every order a parsed document can carry keeps the header form.

- **A sequence's own note keeps its dash line** — a note the edit layer attaches to a
  *container* was written above the line that ended that container's block. When the container
  is a sequence, the line that ends it is one of its items' own dash lines, and granit reports a
  note written below a dash line as that item's leading note — so the second emission moved the
  text off the sequence and into its item (`edit_yaml` crash-08b30707: `?ss:` over two tagged
  empty items). A sequence's note now goes above the sequence's first line, which is where the
  reader files a sequence's own note and what `lift_first_item_notes` already relies on, while a
  mapping's note stays above the pair that ended its block, because that is where a pair's note
  is read back.

- **The benchmark #522 broke now asserts its contract in the suite that runs** — the
  `Python benchmarks` leg went red on `806811c5` on `assert '[server]' in ...` while the required
  `Test matrix (all legs)` reported success. #522 makes a root table that sits before a later root
  pair go inline, so the fixture's `[server]` header is no longer the emission; the assertion was
  describing the pre-#522 spelling, not a property of the document. It now lives in the normal
  suite (`tests/test_toml.py::TestToToml::test_benchmark_fixture_roundtrips_by_values_and_order`),
  because benchmark modules are deselected without `--codspeed`. Reloading the emission catches a
  lost or altered key; comparing the root key list catches the order, and swapping two emitted root
  lines keeps the first comparison equal while flipping the second. The benchmark body is back to
  the measured call alone.

- **The instruction-count gate is now part of the merge verdict** — the `Instruction-count baseline`
  job lived in `.github/workflows/codspeed.yml`, but `Test matrix (all legs)` fans in over
  `ci.yml`'s jobs only, so its result was a report: #522's red `Python benchmarks` leg reached
  trunk while the required check was green, and the Ir gate could have followed it. The job moves
  to `ci.yml` unchanged and runs on the same `changes` classification as the other heavy legs;
  `matrix-verdict` now waits on it, so a performance regression refuses a merge instead of arriving
  on trunk. The two CodSpeed jobs stay advisory in `codspeed.yml`: sub-10% wall-time noise makes
  them a ranking channel, not a merge decision.

- **The Ir gate now refuses a floor the tree has outgrown** — the gate read one edge of the
  baseline: a scenario past the tolerance failed, while a floor far above what the tree now does
  was accepted silently, so the accepted slack widened every time the engine got faster. The
  2026-10-14 refresh had found six writer floors 22–46% below their tree and four readers
  4.7–11.2% below. `scripts/ir_gate.py` now reads both: a scenario more than 2% below its floor
  warns as `FLOOR_STALE`, more than 5% fails, and the failure names the refresh dispatch instead
  of `--update`, which would move the floor to the local host. The first enforcing-image refresh
  under the rule (run 39242949404, artifact `ir-baseline-39`) needed no `accept_drift` and moved
  `to_toml_medium` by −7.12% — accepted by the old gate, refused by the new one — and
  `from_toml_medium` by −3.00%; the largest upward move was `serialize_block` +0.32%, and every
  scenario repeated within a 0.000% spread. `.ci/ir-baseline.json` is that artifact unedited.

- **A mapping note over a nested sequence settles in one round** — the note attached to a mapping
  whose block ended inside a nested sequence was written above the last item's dash line, where the
  reader reports it as that item's leading note: `ss:\n  - !a\n  # fuzz 35\n  - !a\n` re-read as
  `ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n`, so the first emission was not a fixed point. The
  writer's `pair_line_start` field, which places a container's own inline note, was left pointing at
  the sequence item because a block value writes its own starts into the field; each pair now
  restores its own line before returning, so the note lands above the pair that ended the mapping's
  block. The note's owner still moves — the ruling on `set_comment` for a container leaves that as
  it is — but the text settles in one round, pinned over three shapes
  (`a_mapping_note_over_a_nested_sequence_settles_in_one_round`).

- **A TOML integer keeps the spelling it was written with** — `1_000`, `+5`, `-1_000` and the
  radix forms lost their source text on the way through the hub: the parser cleaned the token before
  recording it, so `to_toml(from_toml("a = 1_000"))` came back as `a = 1000`. Both separators and an
  explicit sign are legal TOML (`underscored` and `signed-int` in the ABNF), so the loss was in the
  recording, not in the grammar. The parser now keeps the token text whole; the projection tags a
  spelling only when YAML Core resolves a different value (`0x1f` and `+5` stay untagged, while
  `1_000`, `0b1101_0110` and `-1_000` carry `!!int`), and the writer's literal gate accepts the same
  grammar it emits. A tagged integer is authoritative over the quotes the YAML emitter needed for a
  leading `-`; an untagged quoted `"42"` stays a string. Values are unchanged and agree with
  `tomllib`; pinned by `TestIntegerSpellingFidelity`,
  `decimal_spellings_reach_the_hub_word_for_word` and
  `a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise`.

- **A declared `%YAML` version is honoured on request, and the output says so** — a document's
  header used to be reported by `version()` and ignored by the reader, so `%YAML 1.1` resolved
  under Core and the directive was dropped from the output. Passing
  `honor_version_directive=True` to `parse`, `parse_file`, `parse_all_docs`, `safe_load`,
  `safe_loads` or `YAML(...)` now lets a `1.1` declaration select the YAML 1.1 profile
  (`flag: yes` reads `True`, `012` reads `10`); `1.2`, `1.3` and an undeclared document still
  resolve under Core, and an explicit `schema` wins over the declaration. With the flag on, an
  honoured declaration is re-emitted as `%YAML <version>` before its `---` — after edits and
  `reparse()` too — so a Core reader cannot silently retype the text; a later declaration in a
  stream is preceded by the `...` end marker the grammar requires. The default is unchanged
  (flag off, Core resolution, no directive in the output); `schema` now defaults to `None` so
  "not given" and "explicitly Core" are distinguishable. Pinned by
  `TestYamlVersionNegotiation`.

## [v0.18.1] — 2026-10-17

### Added

- **Free-threaded wheels for Python 3.15t and Linux arm64** — the release now builds a
  `cp315-abi3.abi3t` wheel for Windows, macOS and glibc Linux (x86_64 and arm64), so 3.15t and later
  install a prebuilt artifact instead of compiling from source; Linux arm64 gains its `cp314t`
  and `abi3t` wheels from a native arm64 runner, where the interpreter exec that qemu could not
  perform is a native call. Each wheel is installed, and the suite run, on the interpreter it
  targets.

### Fixed

- **An alias-valued merge key keeps its inline note** — `<<: *d  # note` lost the note at ingest:
  an alias has no metadata slot, and the reader reported the note attached anyway. It now rides the
  pair's key, the slot the block writer emits it from, so the document round-trips unchanged; with
  merges resolved it re-homes onto the mapping's surviving entry.

- **A key's inline note below a tag-only line keeps its owner** — when the pair's value carries a
  note of its own, the key's note is written on a line above the pair; if the line above is a
  value spelled as nothing but its tag (`k: !:`), the reader handed the note to that value and the
  document settled only on its second emission. The writer now closes that line with the value's
  own `~` first, working from an offset captured when the pair started (libFuzzer
  `yaml_roundtrip` crash-08cf4fe4).

## [v0.18.0] — 2026-10-17

<details markdown="1">
<summary>five-format event streams · explicit tags · round-trip fidelity · faster I/O</summary>

### Added

- **Five-format event streams** — `parse_stream` gains a keyword-only
  `format="yaml"|"json"|"jsonc"|"json5"|"toml"`; the dicts keep the same seven keys, with `None`
  where a format has nothing to say, and YAML's `anchor`/`alias` stay YAML-only. The default is
  unchanged, event for event. `pyrs_json::parse_stream` and the validating
  `pyrs_json::from_events` are public, and TOML gained a tree-free producer.
- **The schema language can name a container** — `type: map` and `type: seq` assert a node's
  shape without constraining its contents; `mapping_of: seq` says every value of a mapping is a
  sequence; `mapping`, `object`, `sequence`, `array` and `list` are accepted spellings.
- **`no_std` foundation crates** — `pyrs-ast`, `pyrs-schema`, `pyrs-json` and `pyrs-toml` build
  against `alloc` alone. A new opt-in `std` feature restores `std::error::Error` and the
  `RandomState` hasher, and stays on by default.
- **Prebuilt `pyq` and Linux free-threaded wheels** — the release builds `pyq` for six platforms
  (Linux manylinux2014, glibc 2.17) and attaches the archives to the GitHub Release; the wheel
  matrix now ships Linux free-threaded wheels.
- **CI adds Python 3.15** — the matrix covers 3.8 through 3.15, free-threaded interpreters
  included.

### Changed

- **Standard `!!` tags decide the loaded type** — `!!str 1.20` is the string `"1.20"` (it was the
  float `1.2`, losing the trailing zero), `!!float 1` is `1.0`, `!!bool yes` is `True`,
  `!!binary aGk=` is `b'hi'`, and an `!!int` past i64 stays an integer. A text the tag cannot
  satisfy raises `YamlTypeError`; unknown application tags stay lenient.
- **A bare `!` states that a scalar takes no schema reading** — `! 12` loads as the string `"12"`,
  per YAML 1.2 §6.8. PyYAML and ruamel resolve it implicitly and answer `12`; this release follows
  the specification, and a plugin on `!` no longer overrides the tag. An explicit 0.18.0 behavior
  change.
- **`version()` reports the document's `%YAML` directive** — `%YAML 1.1` reports `1.1`, `%YAML 1.3`
  reports `1.3`, and an undeclared document still reports `1.2`. The writer does not re-emit the
  directive.
- **A `bytes` value is written as `!!binary`** — the built-in plugin emitted the local tag
  `!binary`, which PyYAML could not read and which broke on an empty payload; the local spelling
  stays accepted on load.
- **TOML binary integers keep their spelling** — `b = 0b1010` round-trips as `0b1010` (it was
  rewritten as `10`), carried as `!!int 0b1010` so every format reads one type. `0b1101_0110`
  still loses its separators, and a hand-built untagged `0b1010` still reads as a string.
- **Strict JSON refuses what RFC 8259 cannot spell** — the JSON writers raise on non-finite floats
  instead of respelling them; the JSON5 reader rejects leading zeros its grammar forbids; the
  JSON5 writer no longer emits a number its own reader refuses.
- **`granit-parser` 1.3 → 2.0** — the major bump needed no source change; the full comment,
  anchor and compliance tier passes, and `cargo tree` confirms the graph carries 2.0.
- **A `CustomNode` is 144 bytes, not 208** (128 with `no_std`) — metadata moved behind accessors
  and the rarely used anchor/tag slots share one pointer: `from_json_medium` −4.8%,
  `serialize_anchors` −4.0%, and the two sizes are pinned as part of the hot-path contract.

### Fixed

- **Comment, anchor and tag ownership** — a note rides its own node's line through markers, tags,
  nested containers and compact `- key: value` forms; a first entry's note is lifted above the
  header; an anchored flow member keeps its name; anchors survive `#` that only looks like a
  comment, unbalanced apostrophes and brackets. An alias binds the anchor in front of it, in key
  position too, and nested or aliased merge keys are applied instead of dropped.
- **Block scalars settle in one emission** — an empty block scalar keeps the header indicators it
  wrote, no trailing break is invented or dropped, and the indicators are detected correctly.
- **TOML round-trip** — a dotted key inside an inline table nests; a root-level inline table is no
  longer rewritten as a section header; a table header is identified by its segments, not by its
  dot-joined text; a comment inside a multi-line array stays with its element; a note on an
  inline-table member is written; multi-line strings keep their quotes, escaped breaks and
  trailing delimiters.
- **Nothing a writer emits is refused by its own reader** — `to_toml` emitted invalid TOML in
  several shapes (now gated); JSON5 output is read back by the JSON5 reader; the comments the
  JSONC/JSON5 writers used to drop now survive, at a measured `to_jsonc_medium` +4.77%,
  `to_json5_medium` +3.78% and `to_json_medium` +0.94%.
- **Duplicate and null keys** — a duplicate JSON key no longer takes its comment with it; a
  repeated name collapses by name even when a note rides the key; duplicate keys are rejected by
  resolved value; `null:` and `'null':` are different keys. A document with many null keys parses
  in linear time (2k distinct + 2k null keys grew 12.4× for 4× input).
- **A mapping key means what the same text means as a value** — key identity follows the reader's
  resolution, not the AST's node kind; a plain spelling that resolves to a number is not exempt;
  the pinned model grows from 15 to 18 key classes.
- **Schema validation reads the tag first** — `!!int 0b1010` validates as the integer 10,
  `!!int hello` is refused instead of re-typed, and a rule that names a path asserts the node that
  path selects rather than walking past the wrong kind. `$` is a rule target, and non-ASCII rule
  paths no longer abort the process.
- **The Python contract ships intact** — the committed stub parses and type-checks under mypy and
  ty, declares all ten extension exception classes, and matches the running object's parameter
  names for all 40 module functions; published signatures on 28 API pages are checked against the
  package; an inline dict schema is accepted and every key survives; the generated reference
  renders the four public helpers it had omitted (`to_toml`,
  `validate_against_registered_schema`, `validate_against_schema`, `validate_custom_types`).
- **Edge inputs** — CR-only breaks no longer move notes on the second round; Unicode blanks, BOMs
  and tag suffixes no longer corrupt a document; stream positions count characters, not bytes, for
  CJK and emoji keys; a Python integer wider than 64 bits no longer leaves the dumper as a float.
- **`pyq`** — `validate` accepts `--input` and honours the format it actually read; the test
  harness no longer panics when a refusal closes stdin.
- **The defence tier is measured end to end** — the quality matrix's counts come from the tree,
  the JSON and TOML fuzz targets assert that their own writers' output re-loads, PR-tier fuzzing
  is blocking, and allocation ceilings plus instruction floors sit on the hot paths.
- **The release's `pyq` assets get the toolchain they ask for** — `publish.yml` wrote
  `targets: ${ matrix.platform.std }`; a single brace is not a GitHub expression, so rustup
  received `${`, `matrix.platform.std` and `}` as three literal target names and every `pyq` leg
  stopped at toolchain setup. The step passes `${{ matrix.platform.std }}` now, and
  `tests/test_publish_toolchain.py` fails if the form returns.
- **The release's wheel tests get the tools the suite calls** — the wheel jobs run the whole suite
  inside environments that carried neither `git` (the Alpine containers) nor `tomli-w` (every
  leg), so nine repository gates and the script-import test failed after the wheels were built.
  `.ci/requirements-test.txt` ships `tomli-w`, the container steps install `git`, and the suite
  body is now `.ci/pytest-wheel.sh`, called by every container leg: embedding it in
  `docker run ... sh -c '...'` ended at the apostrophe in a comment (`container's`), so Alpine ran
  `apk add` and nothing else while the runner's toolcache Python ran `-m virtualenv`. The script
  calls the venv's binaries by path because Alpine's `/bin/sh` has no `source`, and
  `tests/test_publish_toolchain.py` fails if any of these inputs goes away.

### Performance

- **Readers do less per byte** — the TOML reader allocates each scalar once (148 → 97 allocations,
  16,362 → 15,612 bytes on `from_toml_medium`); the JSON reader's unescaped strings borrow the
  source (74 → 48 allocations, 9,675 → 9,444 bytes); JSON and TOML scanners copy runs instead of
  decoding per character; the scalar resolvers stop trimming text that has nothing to trim.
- **Writers allocate less** — the TOML writer emits into the buffer instead of formatting
  temporaries (it had counted 53.2M instructions against `to_json_medium`'s 21.3M for the same
  document); the YAML writer decides quoting in one byte pass; tag emission is table-driven and
  escaping no longer allocates.
- **The event stream borrows its scalar text** — `StreamEventType::Scalar` no longer copies every
  value out of granit's `Cow`; the move measured −9.192% on the event scenario.
- **Instruction floors, measured on the pinned `rustc 1.97.1`** — against the committed baseline:
  `from_toml_medium` −9.80%, `from_json_medium` −13.37%, `from_jsonc_medium` −11.28%,
  `from_json5_medium` −9.61%. The release wheels are built by the compiler the floors were
  measured on.

</details>

## [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq parity flags · JSONC/JSON5 input · pyq diff/merge · automated GitHub Release</summary>

### Added

- **pyq CLI parity flags** — `pyq fmt` gains `--indent N` (block indent, default 2), `--width N`
  (plain-scalar soft-wrap column, 0 disables), `--sort-keys` (serializer-level whole-document key
  sort) and `-i/--inplace` (rewrite the file), exposing the `pyrs-yaml-core::SerializeOptions` knobs
  and matching the Python CLI's `fmt --indent`. `pyq to-json` gains `--jsonc` / `--json5` dialect
  output (mutually exclusive), wiring the `pyrs-json` comment-preserving and JSON5-spelling writers
  (`to_jsonc_text*`, `to_json5_text*`) into the CLI.
- **pyq JSONC/JSON5 input dialects** — `--input jsonc|json5` joins the `Format` enum (auto-detection
  also keys off `.jsonc` / `.json5` extensions) and routes through the native `pyrs-json` dialect
  parsers, so comments and JSON5 spellings ride the AST; combined with `to-json --jsonc` this gives
  a comment-preserving JSONC→JSONC round-trip in one command. `--all-docs` rejects the
  single-document dialects with a stable message.
- **`pyq diff` / `pyq merge`** — semantic document comparison and right-biased deep merge (yq `*+`
  shape), new in the native CLI. `diff` walks both ASTs comparing resolved values, structure and
  tags (comments/quoting/layout never appear) printing `-/+ /~` path lines, exit 0 equal · 1
  differs. `merge` overlays mappings recursively, appends sequences (or swaps them with
  `--replace-arrays`), and emits round-trip YAML; both commands read any supported input dialect via
  `--input`/extension detection.

### Changed

- **GitHub Release is created by `publish.yml`** — the release page used to be made by hand
  (`gh release create`) after each publish, which is one more step to forget and one more place for
  the published version and the tag to drift apart. The `release` job now runs `gh release create`
  after `uv publish` succeeds, gated on the same `refs/tags/` condition, with generated notes and
  the built wheels attached — notes and assets come from the tag PyPI was published from.
  `workflow_dispatch` runs are unaffected (no release, no PyPI publish), matching the existing
  behavior.
- **`README.md` / `README.zh-CN.md` document the native `pyq` CLI** — both READMEs gained a `pyq`
  subsection next to the Python CLI section: how to install it from a checkout, three worked
  examples, and the full command list, pointing at the pyq guide for detail.

### Fixed

- **`pyrs-json` module documentation** — it still claimed comments were stripped on read and never
  re-emitted; since #122 they ride the AST comment slots and the JSONC/JSON5 writers reproduce them.

</details>

## [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>cross-library parity suites · load_json · TOML 1.1 + toml-test · JSON5/JSONC · pyq ·
perf</summary>

### Added

- **JSONC block-comment hot-sample bench** — objective §测试覆盖 5 explicitly names "block-comment"
  as a hot sample that must be quantified per change. Previously only inline `//` comments appeared
  in benchmarks; the `/* ... */` scanner branch was unmeasured. The new
  `BENCHMARK_CONFIG_JSON_BLOCK_COMMENTS` fixture (50 pairs, each with a leading standalone
  `/* item N */` plus a trailing-inline `value /* trailing */`, framed by header/footer blocks)
  drives `test_load_jsonc_block_comments`. A scanner regression on the block path now surfaces as a
  CodSpeed delta rather than silent drift.
- **PyYAML + ruamel.yaml cross-library parity for YAML** — objective §测试覆盖 3 names both
  libraries as oracle targets; previously they only appeared in `test_benchmark_crosslib.py` (timed
  runs + a feature-support printout), never as a value-parity assertion.
  `tests/test_yaml_crosslib.py` closes the gap: 20 canonical YAML Core 1.2 documents × 5 parity
  surfaces (pyrs-vs-PyYAML load, pyrs-vs- ruamel load, three-way, cross-engine dump-load round-trip)
    - 2 documented divergences (duplicate-key strictness, YAML 1.1 legacy booleans schema-scope) =
      122 tests. Optional deps guarded by `skipif`.
- **tomlkit cross-library parity for `load_toml`** — objective §测试覆盖 3 names tomlkit as an
  oracle target; previously tomlkit only appeared in `test_benchmark_crosslib.py` and the
  leaderboard floor. `tests/test_toml_crosslib.py` (24 tests) adds value-parity across 11 canonical
  constructs (primitives, arrays, tables, offset/local datetime, date-only, time-only, hex/oct/bin
  radix, escapes, multi-line strings, nested inline tables, underscore separators, exponent floats)
  with three-way agreement (pyrs / tomlkit / tomllib), pins the documented `>i64` divergence as
  spec- strict (TOML v1.0 §Integers: 64-bit signed), and asserts the `-2^63` boundary (PR #174 fix)
  still passes through tomlkit. Optional dep, `skipif` clean.
- **orjson as strict-JSON oracle for `load_json`** — Pillar 2 §测试覆盖 3「与 orjson 逐位比对」
  previously only ran through benchmark, not correctness parity. `tests/test_json_crosslib.py` now
  asserts `pyrs_yaml.load_json(doc) == orjson.loads(doc)` on 16 canonical documents
  (objects/arrays/scalars/simple escapes/\u escapes/raw non- ASCII/duplicate keys), and
  `test_load_json_rejects_what_orjson_rejects` cross-verifies 12 non-strict forms (comments,
  trailing commas, single quotes, bare `NaN`/`Infinity`/`-Infinity`, hex, leading zero, `+.5`,
  `5.`). orjson rejects `NaN`/`Infinity`/`-Infinity` while the stdlib `json.loads` accepts them
  under `allow_nan=True` — pinning orjson as the authority gives the strict loader a stronger
  spec-conformant oracle than stdlib alone. Tests are `skipif(orjson is None)` so the suite runs
  identically without the optional dep.
- **CLI ↔ Binding parity gate (`tests/test_cli_binding_parity.py`)** — Pillar 1's「CLI 与 Python
  Binding 两端均需具备同等功能」claim was documented in the changelog but only enforced by
  convention. This test module turns the invariant into an executable contract: the CLI's registered
  command set is asserted against a fixed 18-command inventory (filtering cyclopts's
  `--help`/`-h`/`--version` pseudo commands), every CLI `to-X` / `from-X` verb is asserted to have a
  matching `YamlDocument.to_X` method, a top-level `from_X` converter, and a `load_X` sibling where
  the family applies; `load_*` family symmetry (`load_json` / `load_jsonc` / `load_json5` /
  `load_toml`) is asserted; and every editing / validate / compliance verb is mapped to its live
  Python API. A binding-side rename or an un-re-exported symbol now breaks CI rather than silently
  drifting the two surfaces apart.
- **`load_json` property tests + CodSpeed benches** — the strict loader lands its oracle-parity
  guard: `test_load_json_matches_stdlib_json` and
  `test_load_json_matches_load_jsonc_on_strict_domain` (Hypothesis, 200 examples over the `json_doc`
  strategy) pin both the fast path and AST fallback against `json.loads` for every generated
  canonical document, while the sibling mirrors the two loaders agree byte-for-byte on the strict
  domain — a divergence between `load_json` and `load_jsonc` (widening the fast path, routing the
  fallback through `from_jsonc`, or a schema drift) shows up as a property failure, not a silent
  regression. Three CodSpeed wall-time benches (`test_load_json_large` / `test_load_json_floats` /
  `test_load_json_escapes`) mirror the existing `load_jsonc` samples so the strict binding layer
  itself is tracked.
- **`load_json` (strict) — completes the `load_*` family parity** — the binding already exposed
  `load_jsonc` / `load_json5` / `load_toml` for direct JSON-dialect → Python decoding; the strict
  RFC 8259 counterpart was missing. `pyrs_yaml.load_json(s)` now accepts exactly the strict grammar
  (matching `json.loads` on every canonical document) and rejects the JSONC / JSON5 extensions with
  a typed `YamlParseError` — `//` and `/* … */` comments, trailing commas, single-quoted strings,
  bare `Infinity` / `NaN`, and `0x…` forms. The fast path shares `json_fast::try_load` with
  `load_jsonc` (which bails on every non-canonical byte, so widening risk is zero); declined
  constructs route through the STRICT `from_json` AST parser, never `from_jsonc`. This closes the
  last gap under the CLI ↔ Binding parity entry below: every format the CLI reaches
  (`to/from json|jsonc|json5|toml`) now has the corresponding Python-side `load_*` sibling — Pillar
  1 is complete. `tests/test_json.py::TestLoadJson` pins canonical parity with `json.loads`,
  per-extension rejection (documenting where `json.loads` itself is looser than RFC 8259 under its
  default `allow_nan=True`), and the `\u` / out-of-i64 fallback route. Re-exported from
  `pyrs_yaml.__init__` and listed in `__all__`; `.pyi` regenerated via `maturin generate-stubs`.
- **Dialect writer fixed-point properties** — `fmt_pbt.rs`'s module header always promised a writer
  fixed point (re-serializing a writer's own re-parsed output reproduces it byte-for-byte) but never
  implemented one. Four proptests now hold that promise for JSON/JSONC/JSON5/TOML; the only input
  filter (`json_object_domain`) excludes hand-built ASTs whose distinct keys spell the same JSON
  name — outside RFC 8259's object domain, where no text round-trips losslessly by definition. The
  gate immediately surfaced three real comment-fidelity defects (Fixed below).
- **Hot-spot benchmark corpus** — seven CodSpeed wall-time benches target the historically fragile
  serialization paths: YAML block-scalar documents (all six header spellings `|`, `|-`, `|+`, `>`,
  `>-`, `>+`) and comment-dense documents, TOML multi-line strings / radix integers / underscore
  separators / exponents / datetimes, and JSON5 exotic number forms (hex, `+.1`, `5.`, `Infinity`,
  `NaN`, single quotes, trailing commas). Fixtures live in `tests/data/yaml_samples.py`, benches in
  `tests/test_benchmark_api.py`. Building this corpus is what surfaced the nested block-scalar
  indentation bug fixed below.
- **Text-level re-parse gate (`prop_output_always_parses`)** — the Rust proptest suite now asserts
  that every generated AST serializes to text the parser accepts again. The AST-vs-AST round-trip
  property silently skipped shapes whose serialized text could not re-parse (`try_roundtrip` returns
  `None`), leaving an entire defect class invisible; the new gate caught six real serializer bugs on
  its first runs (see the Fixed entries below), each now additionally pinned by targeted Rust unit
  tests and a Python regression class (`TestNestedBlockScalarIndent` in
  `tests/test_roundtrip_bugs.py`).
- **TOML datetime offset range** — a numeric UTC offset is now range-checked (hours 00..=23, minutes
  00..=59); `+12:60` / `+24:00` are rejected instead of accepted. The offset was shape-checked but
  its values never validated (toml-test `invalid/datetime/offset-overflow-minute`; valid in neither
  1.0 nor 1.1).
- **TOML table-redefinition strictness** — a table created implicitly by a dotted key is now closed:
  a later `[header]` may not re-open it (`[fruit]` + `apple.color` then `[fruit.apple]`; `[t1]` +
  `t2.t3.v` then `[t1.t2]`), and a table can no longer be redefined as an array of tables (`[tbl]`
  then `[[tbl]]`). These are invalid under BOTH TOML 1.0 and 1.1 (toml-test `invalid/table`
  `duplicate-key-*`/`redefine-*`); legitimate implicit super-tables and sibling dotted keys still
  parse (mis-acceptance total 30 -> 21, valid decode-match unchanged).
- **toml-test conformance harness** — `tests/test_toml_test_suite.py` runs the official
  language-agnostic [toml-test](https://github.com/toml-lang/toml-test) corpus the same way
  `test_yaml_suite.py` runs the YAML suite: an untracked local artifact (`Reference/toml-test`),
  every test `skipif` its absence, and honest measured-floor gates. A type-tag adapter maps
  `load_toml` output to toml-test's `{"type": ..., "value": ...}` form for decode comparison and
  invalid-rejection checks.
- **TOML temporal types decode correctly** — date-only (`1979-05-27`) and time-only (`07:32:00`)
  values now carry distinct `!date` / `!time` tags (local and offset date-times keep `!timestamp`),
  so they route to `date.fromisoformat` / `time.fromisoformat` instead of crashing in
  `datetime.fromisoformat`. Surfaced by toml-test: a bare local time (`07:32:00`), a seconds-less
  time (`13:37`), and a lowercase-delimiter offset datetime (`1987-07-05t17:45:00z`) each raised a
  raw `ValueError` on *valid* TOML. The `!time` plugin now pads omitted seconds for the 3.8 floor,
  `!timestamp` canonicalizes lowercase `t`/`z`, and both normalize a fractional-second field of any
  precision (TOML allows `.6`; `fromisoformat` pre-3.11 accepts only 3- or 6-digit fractions) to
  microseconds.
- **TOML control-character strictness** — raw C0 control codes (NUL, FF, DLE, US, ...) and DEL
  (U+007F) are now rejected inside basic, literal, and multi-line strings (only tab and, in
  multi-line forms, newlines are legal), where the single-line basic path had guarded C0 but every
  other form accepted them silently. toml-test's `invalid/control` corpus surfaced 13 such
  mis-accepted documents. Comment bodies now reject the same control codes (`# ...\u0000...` etc., 5
  further `invalid/control/comment-*` documents), and a bare carriage return (a `0x0D` outside a
  CRLF pair, in any context) is now rejected by a central scan at the parse entry (the last 4
  `invalid/control/*-cr` documents). The toml-test conformance harness also switched to reading
  corpus bytes verbatim — its earlier `read_text()` silently applied universal-newline translation,
  which had rewritten bare CR to LF before the parser ever saw it, masking that whole bug class.
- **TOML number-literal strictness** — decimal integers with leading zeros (`01`, `007`, `-01`,
  `01.5`), a sign on a radix-prefixed integer (`+0x1F`, `-0b101`, `+0o644` — `signed-int` only wraps
  a decimal integer), and trailing/double underscore separators (`1_`, `1__0`, `0x1_`) are now
  rejected rather than silently accepted. toml-test's `invalid/integer` and `invalid/float` corpora
  surfaced 23 such mis-acceptances (mis-acceptance total 71 -> 48). The earlier "radix integers can
  carry a sign" behavior was a spec violation and is now a rejection assertion.
- **TOML inline-table key-collision strictness** — an inline table now rejects a dotted key that
  equals, extends, or is shadowed by an already-defined path (`{ a = 1, a.b = 2 }`,
  `{ a.b = 1, a = 2 }`, `{ a.b = 1, a.b.c = 2 }`, `{ a = { b = 1 }, a.c = 2 }`), while sibling
  dotted paths (`{ a.b = 1, a.c = 2 }`) stay legal. toml-test's `invalid/inline-table`
  `duplicate-key-*` and `overwrite-*` groups surfaced these (mis-acceptance total 48 -> 39).
- **TOML non-ASCII string crash fixed** — the basic and multi-line basic string parsers advanced
  byte-wise and could slice a `&str` mid-character on multi-byte content (e.g. U+00A0, U+0251),
  panicking. Surfaced by the toml-test conformance corpus; both loops now consume whole characters.
  (Completes what #153 fixed for the single-line and JSON string paths.)
- **Format fuzz + robustness fixes** — new `proptest` property tests fuzz the TOML / JSON / JSONC /
  JSON5 parsers and writers with arbitrary input and assert no-panic and re-parseability. They
  surfaced and fix three real defects: a mid-character slice **panic** in the TOML and JSON
  basic-string parsers (on certain lossy-decoded inputs), and a JSONC/JSON5 writer bug where an
  inline `//` comment was not newline-terminated, so it swallowed the following `,` or `}` and
  produced output that could not re-parse.
- **YAML merge/alias property fuzz** — the `proptest` suite now generates well-formed anchor / alias
  / merge-key documents: single aliases, alias sequences, sequences carrying an inline map, the bare
  inline-map merge, and the scalar / null sources the #166 fix rejects. A toggle emits a
  self-referential anchor (`base0: &base0` whose body merges `*base0`) and forward chains, plus
  repeated alias references to one anchor. Previously every property test used `arb_custom_node()`,
  which emits `meta.anchor` but never an `Alias` node — so the alias-resolution and merge-expansion
  paths (the exact structure class behind #163 / #166) were never fuzzed in-process, only covered by
  hand-written unit tests and the subprocess crash harness. `prop_merge_alias_never_panics` asserts
  parse + merge resolution never panic or blow the native stack, and that any tree which parses
  re-serializes and re-parses cleanly.
- **CLI format parity** — the `pyrs-yaml` CLI gains `to-toml` / `from-toml`, `to-jsonc` /
  `from-jsonc`, and `to-json5` / `from-json5`, mirroring the existing `to-json` / `from-json`. Every
  format the Python binding handles is now reachable from the command line (the binding's document
  methods preserve comments, number source and multiline styles through the YAML hub).
- **JSON string-escape fast path (perf)** — `load_jsonc` now decodes the eight simple two-byte JSON
  escapes (quote, backslash, slash, backspace, form-feed, newline, carriage return, tab) inline
  instead of bailing the whole document to the AST path, so escape-bearing JSON (common in
  configs/logs) stays on the fast path — measured ~15x faster than the AST route on a log-style
  payload. Values match `json.loads` exactly; a `\u` escape or any invalid escape still routes
  through the AST path unchanged. A new benchmark exercises the branch.
- **JSON float fast path (perf)** — `load_jsonc` now parses canonical floats (decimals and
  exponents) straight into Python objects instead of bailing the whole document to the AST path.
  Float-bearing JSON — ubiquitous in configs and metrics — now uses the fast path; values match
  `json.loads` exactly (a correctly-rounded parse identical to CPython's `float`), and a new
  benchmark exercises the branch so it is tracked for regressions.
- **Faster JSON string serialization (perf)** — the writer now bulk-copies a string in a single
  `push_str` when no byte needs escaping, instead of re-encoding UTF-8 char-by-char. String-heavy
  `to_json` payloads serialize ~35% faster (measured 41 -> 27 ns/item); output is byte-identical
  (round-trip and serialization-snapshot tests unchanged).
- **`YamlDocument.to_toml()`** — documents now emit TOML directly from the AST, mirroring `to_json`
  / `to_jsonc` / `to_json5`. Previously getting TOML from a parsed document required
  `to_toml(doc.to_yaml())` — a serialize-to-YAML-then- re-parse round-trip; the new method calls the
  native writer once. Output is byte-identical to the round-trip and reloads to the same data. The
  writer itself measures ~4.3x faster than `tomli_w` (locked by a new serialize gate).
- **`to_json` native serializer (perf)** — `YamlDocument.to_json` now uses the native engine instead
  of `to_dict()` + Python `json.dumps` (a double conversion). Byte-identical for ASCII and ~10x
  faster (a 1200-item document drops ~1450µs → ~120µs, now beating `json.dumps`). Behavior change:
  non-ASCII is emitted as raw UTF-8 (matching `to_jsonc` / `to_json5`) rather than `\uXXXX` escapes;
  output stays valid JSON that `json.loads` parses the same.
- **JSON object keys written directly (perf)** — the native JSON/JSONC/JSON5 writer emits mapping
  keys straight into the output buffer instead of allocating a `String` per key via `key_text`.
  `to_json` on a 1200-item document is ~2x faster still (compact ~120µs → ~60µs), byte-identical
  output; JSON serialize now measures #2 against the installed field (behind only orjson).
- **JSON load fast path** — `load_jsonc` now parses canonical strict JSON (objects/arrays, `i64`
  integers, booleans, `null`, escape-free strings) straight into Python objects, skipping the
  intermediate `CustomNode` AST. Measured ~5-6x faster than the AST round-trip and now ahead of the
  stdlib `json.loads`. Any non-canonical input (floats, escape sequences, comments, out-of-range
  integers, trailing commas, invalid tokens) falls back to the general AST path, so values and error
  reporting are unchanged.
- **TOML multi-line string fidelity** — a TOML multi-line string (`"""…"""` or `'''…'''`) now keeps
  its shape through the hub: the parser marks it, it is projected as a `ScalarStyle::Literal` YAML
  block scalar (so the multi-line form survives the text round-trip), and `to_toml` re-emits it as a
  `"""` block instead of collapsing it to an escaped single line. Values round-trip byte-for-byte
  (quotes and backslashes re-escaped), the emission is idempotent, and a single-line `"a\nb"`
  correctly stays single-line. Uses the existing `ScalarStyle::Literal` representation — no AST
  struct change.
- **TOML document-level comment fidelity** — `to_toml` now emits the root mapping's leading comment,
  so a document-opening standalone `# note` survives a TOML → hub → TOML round trip instead of being
  dropped. This is the TOML counterpart of the JSON writer's `emit_root_leading` (#114 / #122). The
  pair and section emitters already handled their own slots; only the root-level note was
  unreachable. Native TOML parses (which place the first note on the first key) are unaffected, and
  a comment-free document emits no stray line.
- **JSON5 Unicode identifier keys** — unquoted object keys now accept the full Unicode `ID_Start` /
  `ID_Continue` set rather than ASCII only, so `from_json5` / `load_json5` parse
  `{ é: 1, 名: 2, हिन्दी: 3 }`. Implemented with the `unicode-ident` tables (the crate rustc’s own
  lexer uses) for exact per-script conformance, including combining marks mid-identifier. Gated on
  the JSON5 flag, so strict `from_json` and `from_jsonc` still require such keys to be quoted. Lone
  UTF-16 surrogates in `\uXXXX` escapes remain rejected (a Rust `String` cannot represent them
  losslessly). Adds a new dependency (`unicode-ident`).
- **JSON5 Unicode structural whitespace** — `from_json5` / `load_json5` now treat the whitespace
  JSON5 adds to RFC 8259's four (tab / space / LF / CR) as inter-token separators: vertical tab,
  form feed, NBSP (U+00A0), every Unicode `Zs` space separator, the LS/PS line terminators (U+2028 /
  U+2029) and ZWNBSP (U+FEFF). Implemented with `std`'s `char::is_whitespace` (minus NEL U+0085,
  which JSON5 does not classify as whitespace) plus an explicit U+FEFF — no new dependency. Gated on
  JSON5 mode, so strict `from_json` and `from_jsonc` still reject every one of them, byte-for-byte
  unchanged.
- **JSON5 line continuation and `\'` escape** — double-quoted JSON5 strings now accept the two
  escape forms JSON5 adds beyond the JSON set: a backslash immediately before a line terminator
  (line continuation, which removes both), and an escaped single quote (`\'` → `'`). Gated on JSON5
  mode, so strict `from_json` and `from_jsonc` still reject both exactly as before. Mirrors the
  single-quoted-string handling from #125, completing JSON5 string fidelity.
- **JSON5 string escapes `\v` and `\0`** — `from_json5` now accepts the two string escapes JSON5
  adds to the JSON set: `\v` (vertical tab U+000B) and `\0` (NUL U+0000), in both double- and
  single-quoted strings. Gated on JSON5 mode, so strict `from_json` and `from_jsonc` still reject
  them exactly as before. Completes the JSON5 grammar support alongside #120 (numeric forms)
  and #124 (value semantics).
- **JSON5 value semantics on load** — `load_json5` now resolves the JSON5-only numeric forms to real
  Python numbers instead of leaving them as strings: hexadecimal integers (`0x1F` → `31`), a leading
  `+` (`+7` → `7`), a trailing decimal point (`5.` → `5.0`), and `Infinity` / `NaN` → `float('inf')`
  / `float('nan')`. Implemented as a new `Schema::Json5` value resolver layered on the JSON one
  (shared forms delegate, so they can't drift), leaving the strict JSON and JSONC loaders
  byte-for-byte unchanged. Serialization fidelity is unaffected: `to_json5_text` still round-trips
  the original source spelling (`0x1F` stays `0x1F`) — load value and emit style are now
  independently correct.
- **JSON5/JSONC reachable through the public API** — `pyrs_yaml.from_json5` and
  `pyrs_yaml.load_json5` (the JSON5 counterparts of `from_jsonc` / `load_jsonc`), plus
  `YamlDocument.to_jsonc()` and `YamlDocument.to_json5()` emit methods that route through the native
  engine (so comments and JSON5 styles survive) rather than `json.dumps`. Also fixes a long-standing
  reachability gap: `from_jsonc` / `load_jsonc` were exported by the extension module but never
  re-exported from the `pyrs_yaml` package `__init__`, so `pyrs_yaml.from_jsonc(...)` raised
  `AttributeError` — they now appear alongside `from_json` and in `__all__`. `to_jsonc` / `to_json5`
  gained `emit_root_leading`: a document-level standalone comment (which lands on the root
  container's `leading_comment` slot) is now emitted before the outermost `{`/`[` instead of being
  dropped. New benchmark coverage for the whole JSON family (`from_jsonc` / `from_json5` /
  `load_json5` / `to_jsonc` / `to_json5`) joins `test_benchmark_api.py`.
- **JSON5 writer (`to_json5_text` / `to_json5_text_pretty`)** — contract-B step 2,
  complementing #120's parse side. Serializes the shared AST back to JSON5 text, restoring the
  JSON5-only spellings the parser preserves on the AST: single-quoted strings (any scalar tagged
  `ScalarStyle::SingleQuoted`, which only the JSON5 parser produces) and the numeric forms `0x…` /
  `.5` / `5.` / `+7` / `Infinity` / `NaN` as bare tokens. Comments (line and block, both normalized
  to `//`) are emitted exactly as in JSONC, using the `leading_comment` / `comment` slots
  from #114/#117. Object keys are always quoted — JSON5 permits bare identifier keys but quoting is
  lossless and keeps one code path. `to_json_text` / `to_jsonc_text` are unchanged; the writer now
  shares a single `Mode` (Json / Jsonc / Json5) internally instead of a boolean comment flag.
- **JSON5 numeric forms on the parser** — `from_json5` (and `from_json_with_options` with the new
  `allow_json5_numbers` axis) now accept the number spellings that are legal in JSON5 but invalid in
  RFC 8259: hexadecimal integers (`0xDECAF`, `0XFF`, `-0x1F`), a leading decimal point (`.5`), a
  trailing decimal point (`5.`), an explicit leading `+` (`+7`, `+.25`), a leading zero (`07`), and
  the bare `Infinity` / `NaN` literals (including signed `-Infinity`). Each form keeps its exact
  source text on the plain scalar (the same source-spelling strategy as TOML #108) so a follow-up
  JSON5 writer can reproduce it verbatim. `STRICT` and `JSONC` are gated off this axis, so they
  reject every one of these exactly as before — the RFC 8259 contract is unchanged. `from_jsonc`'s
  stale "comments are stripped" doc comment was corrected: since #112/#115 comments are preserved on
  the AST and round-trip through `to_jsonc_text`.
- **TOML inline-table interior comment fidelity** — PR #119 plumbs interior `# ...` comments through
  the inline-table IR so they survive the round trip instead of being silently dropped. A `# ...` on
  its own line above a member is captured as that member's leading note (onto its key's
  `leading_comment`); a `# ...` on the member's own line after its value is captured as trailing
  (onto the value's `comment`). Undecorated inline tables keep the compact single-line
  `{ a = 1, b = 2 }` form; a decorated inline table nested in an array promotes to the multi-line
  inline form (a line comment cannot live inside a one-line `{}`), while a decorated top-level
  inline table is promoted to a `[section]` (long-standing behaviour since #107) that now carries
  the comments. Fixing this also surfaced and repaired a latent #114 bug: `skip_all_blank` counted a
  standalone comment's own terminating newline as a blank line, so re-parsing emitted spurious blank
  lines before comment-led pairs.
- **YAML receiver writes standalone comments into `decor.leading_comment`** — PR #117b migrates the
  last remaining engine (the granit-parser receiver in `parser/mod.rs`) onto the slot convention
  PR #114 introduced for TOML and PR #115 for JSONC. Standalone `# ...` notes on scalars, mappings
  and sequences now land on `NodeMeta::decor.leading_comment` instead of the older
  `comment(standalone = true)` shape. Every existing hand-built fixture continues to compare equal
  thanks to #117's AST-layer normalisation, and `CustomNode::remove_comment` now clears **both**
  slots atomically so `Node.remove_comment()` still does the right thing on YAML-parsed docs
  (widened semantics).
- **Cross-slot standalone normalisation + Python `Node.leading_comment` API** — `NodeMeta::eq` /
  `Hash` now treat "the standalone note" as one concept regardless of whether it lives in the newer
  `decor.leading_comment` slot (introduced by PR #114 and used by the TOML / JSON engines) or the
  older `comment` slot with `standalone = true` (still written by the YAML receiver and every
  hand-built pre-#114 fixture). Parsed YAML documents compare equal to hand-built fixtures and vice
  versa, so a future YAML write-side migration is transparent. `CustomNode::set_leading_comment` /
  `remove_leading_comment` write / clear **both** slots atomically so the two conventions never
  disagree, and `set_leading_comment` preserves an inline trailing note still living in `comment`.
  The YAML serializer reads the normalised view, so `to_yaml(toml_ast)` / `to_yaml(json_ast)` keep
  their leading notes instead of dropping them. On the Python side, `Node.leading_comment` exposes a
  getter / setter / remover mirroring `Node.comment`, and TOML- or JSONC-parsed documents surface
  their standalone notes to Python callers for the first time.
- **TOML 1.1.0 grammar** — `from_toml` now parses to the [TOML v1.1.0
  spec](https://toml.io/en/v1.1.0) released 2025-12-18. Four concrete additions over 1.0.0: **(A1)**
  inline tables may span multiple lines and end with a trailing comma (`{\n a = 1,\n b = 2, \n}`);
  **(A2)** `\xHH` basic-string escape for codepoints 0x00..=0xFF; **(A3)** `\e` escape for U+001B
  (ESC); **(A4)** seconds are optional in time and date-time values, so `t = 14:15` and
  `dt = 2010-02-03 14:15` parse and round-trip byte-stable. `TomlDialect::V1_0` and `from_toml_v1_0`
  remain for consumers pinning the strict 1.0.0 grammar; every 1.0.0 document parses identically
  under both dialects. The parser-side off-by-one in `space_time_sep` detection (which meant
  `T`-less date-times were never recognised in 1.0 mode either) was fixed alongside.
- **JSON dual-slot comment fidelity** — the JSONC parser now writes standalone (`// ...` on its own
  line above a pair or element) notes onto the `leading_comment` slot PR #114 introduced, while
  inline (`... value // trailing`) notes stay on `comment`. Object members and array elements can
  therefore carry BOTH simultaneously — a shape the single-slot #112 model could not express.
  `to_jsonc_text_pretty` reads the new slot first and falls back to `comment` with
  `standalone = true` so hand-built fixtures keep rendering. Behaviour is otherwise unchanged for
  strict JSON (`to_json_text` still emits no `//`).
- **TOML blank-line and dual-slot comment fidelity** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both excluded from structural `Hash`
  / `PartialEq`), so a section header or array-of-tables element can carry BOTH the standalone note
  above it AND the inline trailing note after `]` without either displacing the other. `to_toml`
  reproduces the blank-line groupings the source had (`a = 1\n\nb = 2` round-trips byte-stable),
  while a design-review rule suppresses blank lines before the very first pair of a document.
  Hand-built / YAML-origin nodes that still put a standalone note into `comment` render identically
  thanks to the writer's fallback read.
- **JSON5 dialect** — `pyrs_yaml_core::json::from_json5(text)` and the general
  `from_json_with_options(text, JsonParseOptions)` accept the full JSON5 axis set: trailing commas
  in arrays and objects, single-quoted strings, unquoted identifier keys (`A-Z a-z _ $`), and
  line/block comments. Each axis is individually togglable on
  `JsonParseOptions { allow_trailing_commas, allow_single_quoted, allow_unquoted_keys, allow_comments }`; `STRICT`, `JSONC`, and `JSON5` constants ship as the common presets.
- **JSONC/JSON5 bindings and CLI** — `pyrs_yaml.from_jsonc(str)` returns YAML text (mirrors
  `from_json`), `pyrs_yaml.load_jsonc(str)` returns a Python dict / list directly (mirrors
  `load_toml`). The `pyq` command gains `--jsonc` and `--json5` flags on `from-json` so
  `tsconfig.json` / `settings.json` files feed straight into the existing verb pipeline.
- **JSONC comment preservation** — `from_jsonc` now attaches captured `//` line and `/* */` block
  comments onto the AST's `NodeMeta::comment` (standalone on the key node, inline on the value
  node), mirroring the TOML model from PR #109. The matching pair `to_jsonc_text(node)` /
  `to_jsonc_text_pretty(node, indent)` emits them back at their original positions; block comments
  collapse to `//` on output (the AST stores only the body text). Strict RFC 8259 writers
  `to_json_text` / `to_json_text_pretty` remain byte-identical: they ignore comments even when the
  AST carries them, so consumers can opt into preservation selectively.
- **pyq multi-document edits** — `-A/--all-docs` now covers every edit command
  (set/delete/rename/move/append/insert/sort-keys) plus `to-json -A` (JSON array, Python parity).
  Each document splices against its own segment of the stream (`MultiDocEditor` +
  `DirtyUnit::shifted`): untouched documents and all `---` separators keep their exact bytes, a plan
  miss skips its document (Python try/ skip semantics; all-miss still exits 1), and one layout-dirty
  document falls back alone instead of de-pinning its neighbours.
- **JSONC parsing** — `pyrs_yaml_core::json::from_jsonc(text)` and the more general
  `from_json_with_options(text, JsonParseOptions)` accept `// line` and `/* block */` comments at
  any position whitespace is legal (the dialect popularised by TypeScript's `tsconfig.json` and VS
  Code's `settings.json`). Comments are stripped, not preserved on re-serialisation; trailing commas
  and other JSON5-only forms are still rejected so the accepted language remains a strict superset
  of RFC 8259 JSON. `from_json` continues to enforce strict RFC 8259 by default; no existing API
  changes.
- **TOML comment fidelity** — the parser now captures both standalone (`# ...` on its own line above
  a pair or section header) and inline (`key = value # ...` / `[name] # ...`) comments and attaches
  them onto the shared AST through `NodeMeta::comment` (standalone on the key node, inline on the
  value node). `to_toml(from_toml(src))` re-emits those comments at their original positions, so
  `pyq edit` and `YamlDocument.set()` no longer strip annotation notes from a TOML round trip.
  Whitespace fidelity (blank-line separators between pairs) stays on the writer's default layout per
  the design doc.
- **TOML numeric source fidelity** — `to_toml(from_toml(src))` now preserves the source spelling of
  hex (`0xDEADBEEF`) and octal (`0o755`) integers and exponent-form floats (`1e10`, `-3.14e-2`,
  `6.02E23`). Radix prefixes are emitted verbatim so the round-trip is byte-stable; underscore
  separators and explicit `+` signs are canonicalised to plain decimal because YAML Core schema does
  not accept them (which keeps the shared AST interoperable with the `load_yaml`/`to_dict`
  pipeline). Negative radix forms (`-0x1F`) also canonicalise for the same reason. Binary (`0b101`)
  canonicalises to decimal because YAML Core has no binary rule. Comment fidelity and JSONC support
  land in follow-up PRs (design doc at `docs/superpowers/specs/toml-json-fidelity-design.md`).
- **pyq feature completion** — the CLI reaches parity with the Python CLI's surface:
  `rename`/`move`/`append`/`insert` splice edits, `validate` (parse check, or against
  schema-language rules via `--schema rules.yaml`), `frontmatter` (`--body-out` splits the body),
  and `-A/--all-docs` on `get`/`fmt`/`to-json` for multi-document streams. Fixing the wiring
  surfaced a core engine bug: `move_path` returned only the destination INSERT unit, so the spliced
  text kept a duplicate of the moved subtree (invisible whenever documents fell back to
  re-serialization); it now returns both units and the bindings apply them through their batch
  splice path.
- **`pyq` filter verbs** — structured jq-style post-processing on the match stream:
  `--select 'PATH OP LITERAL'`, `--sort-by PATH` / `--desc`, `--unique`, `--first` / `--last`,
  `--skip N` / `--take N`, and `--join SEP`, applied in the fixed pipeline order
  `select -> sort -> unique -> slice` then `join` on `get` and the `from-*` converters. Deliberately
  flags, not an expression language: the predicate is one micro-grammar parse (micro ~40 lines),
  mixed comparison kinds are `false` (documented divergence from jq's total order), and startup
  stays instant.
- **`pyq completion`** — print a shell completion script for bash, zsh, fish or PowerShell
  (`pyq completion bash > ...`), powered by `clap_complete` (approved addition to the CLI crate's
  dependencies; it stays inside the `pyrs-yaml-cli` binary and does not touch the Python
  distribution).
- **`pyq sort-keys`** — sort the keys of the mapping at any path (`$` for the root), in place or to
  stdout, through the same core plan/splice engine as `set`/`delete` - closing the parity gap with
  the Python CLI's `sort-keys` command.
- **Command-line interface** — a new `pyrs-yaml` command (opt-in via `pip install "pyrs-yaml[cli]"`,
  requires Python 3.10+) exposing the library from the terminal: `fmt` (round-trip reformat
  preserving comments/anchors/ order), `get` (JSONPath queries with `--format yaml|json|text`),
  `set` / `delete` / `rename` (path-based edits with `--inplace`, `--string`, `--create-missing`),
  `validate` (CI-friendly exit codes), and `to-json` / `from-json` conversions. All commands read
  stdin via `-` and default to stdout output. Implemented in pure Python (`python/pyrs_yaml/cli/`)
  on top of [Cyclopts](https://github.com/BrianPugh/cyclopts) as an optional extra, so the base
  install keeps zero extra dependencies and Python 3.8 support.
- **CLI expansion** — `sort-keys` (sort mapping keys at a path), `move` (relocate a subtree to an
  existing destination), `frontmatter` (extract Markdown front matter as YAML, optional body split),
  and `compliance` (YAML Test Suite report with `--json`) commands; `-A/--all-docs` multi-document
  mode on `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/ `validate`/`to-json`; and mutually
  exclusive `validate --schema <name>` vs `--schema-file <path>`. The undocumented
  `python -m pyrs_yaml.compliance` entry point was removed in favor of the subcommand.
- **`YamlStream` is now importable** — `from pyrs_yaml import YamlStream` works as documented in the
  API reference and type stubs; the class was previously returned by `YAML().load_stream*()` but
  never exported from the native module.
- **CLI `move --all-docs`** — `move` now accepts `-A/--all-docs`, applying the subtree move to every
  document where both paths resolve (same semantics as `set`/`delete`/`rename`), so the
  multi-document flag covers all edit commands.
- **Docs ↔ API consistency guard** — `tests/test_docs_api.py` scans every locale doc page
  (`docs/{en,zh,ja,ko}`) for `pyrs_yaml.…` attribute chains, `import pyrs_yaml…`, and
  `from pyrs_yaml … import …` claims, and fails if any referenced symbol does not exist at runtime
  (~965 claims checked). Guards against the class of drift that let the missing `YamlStream` export
  stay undocumented-by-tests.
- **Optional third-party type plugins** — `!duration` (`pendulum.Duration`), `!arrow`
  (`arrow.Arrow`), and `!ulid` (`ulid.ULID`) auto-register when the corresponding library is
  installed (`_register_third_party` in `python/pyrs_yaml/plugins/_builtin.py`). Each uses a
  distinct tag so existing `!timestamp` / `!date` / `!uuid` handlers are unaffected; a plain stdlib
  `timedelta` is never matched by `!duration`.
- **pydantic-settings YAML source** — `PyrsYamlConfigSettingsSource`
  (`python/pyrs_yaml/settings.py`) is a drop-in replacement for
  `pydantic_settings.YamlConfigSettingsSource` that parses with pyrs-yaml (YAML 1.2 core schema)
  instead of PyYAML. It is exported lazily so `import pyrs_yaml` never requires pydantic-settings;
  install with `pip install "pyrs-yaml[settings]"` (Python 3.10+). `dump_pydantic` and `parse_as`
  now use the same lazy module-level `__getattr__` export pattern.
- **`pyq` — native Rust CLI crate** — `crates/pyrs-yaml-cli` (workspace member, clap-based,
  installed via `cargo install --path crates/pyrs-yaml-cli`) puts `pyrs-yaml-core` directly behind a
  jq/yq-style command line, with no Python at runtime: `fmt` (comment/order-preserving round-trip),
  `get <path>` (JSONPath-lite: dot keys, `[n]`/`[-n]`, bracket keys, `*`; `--json`/`--raw` output),
  `set <path> <value>` and `delete <path>` (yq-style edits with `--create-missing` and
  `-i/--inplace` file rewrite; edited documents re-serialize through the round-trip serializer,
  keeping comments and the value's own style), `to-json` (order-preserving), `to-toml`, and the
  import commands `from-json` / `from-toml` / `from-ini`. Input formats resolve by extension
  (`--input` overrides; YAML content is always a JSON superset); stdin via `-` or omission; non-zero
  exit with the core's stable error text on failures. 12 integration/unit tests pin parity with the
  Python API.
- **TOML and INI exchange formats** — hub-and-spoke multi-format support with YAML as the single
  editable representation:
    - `from_toml(toml: str) -> str` and `to_toml(yaml: str, schema=...) -> str` convert TOML text ⇄
      YAML text (Rust `toml_edit` 0.25);
    - `load_toml(toml: str) -> dict` materializes TOML directly into Python values, with datetimes
      routed through the built-in `!timestamp` plugin; TOML strings never re-resolve (a `"true"`
      value stays a string);
    - `load_ini(text: str) -> dict` reads INI via stdlib configparser (strict, case-preserving;
      read-only by design — INI has no official grammar). TOML output rejects inexpressible shapes
      (null values, non-table roots, aliases, non-scalar keys) with stable error messages.
      Round-trip editing (comments, anchors, splices) remains YAML-only by design.

### Changed

- **granit-parser 1.1 → 1.3** — bumped the YAML event parser from 1.1.0 to 1.3.0. The upgrade is a
  semver-compatible minor bump inside the 1.x line: 1.2.0 added optional `Options` fields for
  exotic-document limits, 1.2.1 tightened a few parse results to match the YAML spec, and 1.3.0
  added two defaulted `Input` methods (`fetch_block_scalar_line` and
  `take_quoted_scalar_ascii_chunk`) that let the scanner step over block and quoted scalar bytes
  faster. The project consumes the parser through `Parser::new_from_str` and implements only
  `EventReceiver` / `SpannedEventReceiver`, never `Input`, so no source changes were required — the
  new trait methods resolve to their default implementations. Full suite green:
  `cargo nextest run --all` (359), `pytest` (1436 + 43 numpy), pure-Rust `--no-default-features`
  build, and the YAML test-suite compliance gates unchanged.
- **Native JSON & TOML cores** — the `serde_json` and `toml_edit` dependencies are gone.
  `pyrs-yaml-core` ships an RFC 8259 JSON engine (byte-level scanner, verbatim number spelling so
  `from_json → to_json` is byte-stable and precision-preserving for large integers/doubles, typed
  line/column errors, strict rejections for trailing commas, leading zeros, lone surrogates,
  unescaped control characters, and multi-root documents) and a TOML 1.0 engine covering the full
  grammar — bare/quoted/dotted keys, basic/literal/multiline strings with every escape form, decimal
  / hexadecimal / octal / binary integers with underscore separators, floats including `inf` / `nan`
  / exponent forms, and offset / local date, time, and date-time — every rejection surfacing as
  `ParseError::Syntax` with 0-indexed `line`/`col` in the granit-parser house style. CLI, bindings
  and pyproject surface unchanged; round-trip tests and `tests/test_toml.py` green with the native
  path.
- **Internal duplication cleanup** — benchmark fixtures composed from shared blocks, PyO3 path-edit
  methods delegate to the existing `apply_metadata_edit` helper, and repeated
  file-read/error-mapping and line-offset boilerplate collapsed into shared functions. No public
  behavior change; the duplicate-code rate measured by jscpd drops from 5.25% to 3.45%.
- **`YamlDocument.validate()` caches compiled validators** — the first successful validation against
  a schema (JSON text *or* dict) caches a compiled `jsonschema` validator; later calls skip schema
  parsing, meta-schema checking, and validator construction. Dict schemas are keyed by object
  identity with a deep-copy snapshot guard: an in-place mutation is detected by `==` on the next use
  and transparently recompiles. The cached path raises
  `exceptions.best_match(validator.iter_errors(instance))`, identical to `jsonschema.validate()`
  semantics. WSL wall-time: `document_validate` −98% (660µs → 13µs).
- **Structural dedupe of parser/serializer kernels** — mapping and sequence rendering share one
  `write_container_node` skeleton (output byte-identical, `serialize_*` medians −5~11%); single- and
  multi-document parse entry points share one `load_ast` error contract; the schema resolution
  chains share `bool_word`/`numeric_tail` and YAML 1.1 no longer re-checks the core's null/bool
  words per scalar; anchor registration (`register_anchor`) and the standalone-vs-inline comment
  taxonomy (`is_standalone_placement`) are single-sourced across the AST and stream receivers. Repo
  duplicate rate 3.38% → 2.60%.

### Fixed

- **`\u` / `\x` escapes followed by a multibyte char panicked the parser** — the fixed-width escape
  readers sliced `&self.text[pos..pos+width]` by byte offset assuming one byte per char. When a JSON
  `\u`, or a TOML `\xHH` / `\uXXXX` / `\UXXXXXXXX`, escape was followed by a multibyte character the
  slice end landed mid-character and Rust panicked (aborting the process), the sibling of the #153
  non-ASCII slice crashes. The readers now slice the byte buffer and UTF-8-validate, so a malformed
  escape returns a clean parse error instead of aborting. Found by the dialect no-panic fuzz; pinned
  by deterministic Rust regression tests in both parsers.
- **A literal `<<` key with a non-merge value was silently dropped** — `safe_dump({"<<": None})` →
  `<<: null` → `load` returned `{}`, losing the key (same for `<<: 1`, `<<: "x"`, or `<<:` alongside
  other keys). The merge resolver treated every `<<` as a merge key and, for a Null/Scalar source,
  consumed it while merging nothing. Per YAML a `<<` is only a merge when its value is an
  alias-to-mapping, an inline mapping, or a sequence of those; a Null or plain-scalar `<<`, and an
  alias-free `<<` that yields nothing to merge (`<<: []`, `<<: [1, 2]`, `<<: {}`), is an ordinary
  key and now survives the round trip. Alias / alias-bearing merge paths are always consumed
  (the #166 self-referential-anchor guard still holds and a `<<: [*a, *a]` cannot re-expand), and
  the yaml-test-suite stays at 405/406. Surfaced non-deterministically by the round-trip property
  fuzz — exactly the #163/#165/#166-class defect the audit targets — and pinned by deterministic
  Rust regression tests covering null, scalar, and sequence forms.
- **TOML deep nesting overflowed the native stack and aborted the process** — the TOML parser
  carried no nesting budget (unlike JSON's `DEFAULT_MAX_DEPTH` and YAML's `parse` `max_depth`), so
  `parse_value` → `parse_array` / `parse_inline_table` recursed without limit. A deeply nested array
  or inline table (`load_toml("a = " + "["*5000 + "]"*5000)`) crashed the interpreter outright
  (verified: exit `0xC00000FD` STACK_OVERFLOW, no Python exception) — the TOML analogue of the #166
  YAML self-referential-merge stack overflow. The parser now tracks a `depth` counter and returns a
  typed `ParseError::MaxDepthExceeded` past 1000, mirroring JSON exactly. Guarded by an in-process
  Python boundary test (≤500 parses, >1000 raises), a subprocess-isolated crash canary (a regression
  can no longer kill the whole pytest runner), and a big-stack Rust unit test.
- **Dialect writers and parsers lost or misplaced document-level comments** — three defects the new
  fixed-point properties caught: (a) a file-leading `// note` before a JSONC/JSON5 value was
  classified as an *inline* note (no newline precedes offset 0 in the whitespace scanner's
  heuristic) and was claimed by the first object member instead of the root container the writer's
  `emit_root_leading` had annotated — on an empty `{}` or a root scalar it vanished entirely; (b)
  the JSON/JSONC/JSON5 **and TOML** writers emitted comment bodies verbatim while every parser
  stored them trimmed, so an untrimmed comment (`text = " "`) oscillated trailing whitespace across
  passes — the writers now trim at emit, making the first spelling stable; (c) a comment-only TOML
  document (`# note` with no key to consume it) dropped its note on re-parse, serializing the empty
  root to `""` — the leftover standalone note now attaches to the otherwise-empty root table. A
  hand-built non-empty TOML root carrying a standalone note *and* a first-element note renders as
  two adjacent `#` lines no TOML text can re-attribute; that shape (never produced by a real parse)
  is filtered from the fixed-point property by `toml_root_note_ok`, mirroring the JSON object domain
  filter. Together the five formats now round-trip their leading comments to a byte-stable fixed
  point. Pinned by `jsonc_file_leading_comment_stays_on_the_root`,
  `jsonc_comment_text_is_written_trimmed`, and `comment_only_document_keeps_its_note_on_the_root`.
- **Nested block scalar bodies kept their parent's indentation** — a literal or folded scalar under
  a nested key (`a:` ⏎ `b: |` ⏎ body) emitted its body lines one fixed indent step from column zero
  instead of one step below the `b: |` header line, so every nested block-scalar shape — pairs,
  sequence items, compact dash mappings, any depth — serialized to text that re-parsed as an error
  or a wrong value. The scalar writer now threads a `block_base` (the parent line's column) through
  every emission site; the round-trip is text-exact for all seven nesting shapes. Found by the TOML
  hot-spot bench materializing nested block scalars through the shared serializer.
- **The serializer emits only re-parseable YAML** — five smaller spelling defects, each caught by
  the new text-level gate: (a) a mapping value forced onto its own line pre-emitted the child's
  anchor/tag even when the child (scalars, nulls, flow containers) writes its own header, spelling
  double headers like `A: !a` … `!a null`; the pre-emit is now limited to block containers, which
  really do suppress it; (b) block scalars inside flow collections (`[|`, `{k: >}`) or key position
  demote to double-quoted — flow syntax has no room for an indented body region; (c) a flow
  container starting its own line (after a standalone comment forced the newline) lost its line
  indent, and complex keys (`? …`) emitted the value marker `:` at column zero, closing any
  enclosing collection — both now indent from their parent; (d) a complex key whose body carries a
  standalone comment or tag serialized ambiguous text (`? # c` mid-line, `?` and body split across
  mismatched indents): the note moves above the `?` marker and the key body always gets its own
  lines one step deeper; (e) plain scalars with edge whitespace (`" %"`, `" "`) or embedded flow
  indicators (`,`, `[`, `]`, `{`, `}`) inside flow collections are now quoted — unquoted they
  terminate the token mid-value or vanish on re-parse (the whitespace case even bypassed the
  schema-resolution exemption because a token that cannot re-parse has no type promise to keep).
  Tagged empty block containers additionally fold their header onto the `{}` / `[]` line, and
  compact `- key: value` items refuse to inline a value carrying a standalone comment. Nine pinned
  Rust tests (`nested_literal_block_scalar_…`, `plain_scalar_with_comma_in_flow_is_quoted`, …) and
  the parametrized Python regressions guard each class.
- **TOML rejected the legal minimum i64 integer** — `from_toml` / `load_toml` failed on
  `-9223372036854775808` (`i64::MIN`, in range by TOML definition): the signed path stripped the
  `-`, parsed the magnitude `9223372036854775808` as an unsigned i64, and overflowed before negation
  could run. The sign now parses *with* the digits (`i64::from_str` accumulates negatively); signed
  floats negate with their exponent spelling preserved, and the magnitude-then-negate pass is gone.
  Found by the new Python-side Hypothesis dialect fuzz (`tests/test_property_dialects.py` — property
  tests for TOML/JSON/JSONC/JSON5 against the stdlib `json` / `tomllib` / `pyjson5` oracles with
  type-strict equality, which also pins two known AST-ambiguity spellings: bare JSON5
  `Infinity`/`NaN` number literals and >i64 digit-string spellings that writers emit verbatim by
  fidelity contract). Repro: `pyrs_yaml.load_toml("a = -9223372036854775808")` now returns
  `{"a": -(2**63)}`. Covered by a Rust regression test
  (`toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`) and the TOML property tests.
- **Wrongly-indented flow sequence continuation is rejected again** — upgrading the YAML parser to
  granit-parser 1.3 (see *Changed*) silently began *accepting* a multi-line flow collection whose
  continuation line is indented no further than its enclosing block key (yaml-test-suite `9C9N`:
  `flow: [a,` then `b,` at column 0), regressing strictness `405/406 → 404/406` — invisible to the
  suite's ≥95% threshold gate, so it passed green CI. An in-tree post-parse guard in the AST
  receiver now tracks the enclosing block indentation and rejects an under-indented flow
  continuation, restoring `405/406`. The guard uses only spans the parser already computes, so
  correctly indented multi-line flows are untouched. `9C9N` is now pinned as a **per-case hard
  gate** (literal input, no `skipif`, so it cannot hide behind the threshold or an absent
  `Reference/` corpus) in `tests/test_yaml_suite.py`, plus a Rust unit test
  (`parser::tests::flow_continuation_under_indented_is_rejected`).
- **Self-referential merge keys no longer overflow the native stack** — a `<<` whose expansion
  points back at its own anchor (`a: &a` containing `b: {<<: *a}`) expanded forever inside
  `resolve_merge_keys`, exhausting the native stack and killing the whole interpreter process
  (Windows exit `0xC00000FD`, a segmentation fault). The cycle guard was applied only while
  *collecting* merged pairs, never while *walking* the expansion, so recursive re-entry was never
  caught. The anchor guard is now path-scoped, the same way alias expansion is guarded: an anchor's
  name stays on the recursion path while its expansion is walked, and a merge that resolves back to
  an ancestor already on that path terminates as an empty expansion instead of recursing. An acyclic
  AST cannot carry PyYAML's cyclic dict, so a self-merge now bottoms out at `{}` rather than
  crashing. Four related merge-semantics defects were fixed in the same pass: a null / scalar /
  sequence merge source no longer survives as a literal `<<` key, an inline mapping used directly as
  a merge value (`<<: {x: 1}`) now merges, and a non-alias element in a merge sequence
  (`<<: [*a, {y: 2}]`) keeps its inline map. Covered by six Rust and nine Python regression tests
  (`merge::tests`, `tests/test_gaps.py::TestSelfReferentialMerge166`). Reported by
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #166.
- **NumPy serialization no longer reads Python memory without the GIL** — the ndarray writer
  borrowed the array's data buffer via `unsafe { as_slice() }` and then iterated that borrowed slice
  *inside* `py.detach`, i.e. after releasing the GIL. `&[T]` is `Send` no matter its provenance, so
  the borrow checker could not catch it, but the memory is owned by Python and another thread could
  resize or write the array concurrently — an unsound data race / UB that only surfaces under
  concurrency. The buffer is now snapshotted into Rust-owned memory while the GIL is still held
  (`slice.to_vec()`), and only the scalar→node conversion runs off-thread. This was the *only*
  `unsafe` buffer borrow in the binding layer; every other `py.detach` site was audited and touches
  Rust-owned state only (AST, source text, `BufWriter<File>`). Covered by
  `tests/test_numpy.py::TestNumpyConcurrency`. Reported by
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #165.
- **Repeated alias references no longer resolve to `None`** — `to_dict()` expanded aliases behind a
  *global* visited-anchor set that was never cleared, so only the first reference to any anchor
  produced a value and every later one silently degraded to `None`:
    ```yaml a: &x 1 b: *x      # 1 c: *x      # was None, now 1```
    The blast radius was wider than "the second reference": two sibling references inside one
    container poisoned each other too (`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` gave `b` a value and
    `c` a `None`). The guard is now scoped to the current recursion path — pushed for the duration
    of one expansion, popped afterwards — so repeated and sibling references each get their own
    fully built value while genuine cycles still terminate. `<<` merge resolution and the AST itself
    were audited and are unaffected. Six new PyYAML-parity cases in `tests/test_direct_load.py` pin
    the agreement, and two tests that had pinned the buggy output as expected behaviour were
    rewritten. Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #163.
- **Document header comments no longer vanish on nested first values** — the parser kept one shared
  comment slot for all in-progress containers, so a nested container start clobbered a standalone
  header note before it reached the AST (dropped at parse time; invisible to `to_dict`, fatal for
  `dump`). Slot handling is now a per-container stack.
- **Splice edits no longer duplicate leading comments** — when a regenerated region text carried the
  pair/item's own standalone note, the replaced range skipped the old note line and both survived;
  the plan now widens the range over note lines (`pyq set`/`delete` and the bindings' splice path
  share the fix).
- **`!timestamp` accepts a trailing `Z` on every supported Python** — `datetime.fromisoformat` only
  recognizes the UTC-`Z` suffix from 3.11; the built-in plugin now normalizes `...Z` to `+00:00`
  first, fixing `ValueError: Invalid isoformat string` on 3.8–3.10 for YAML `!timestamp` scalars and
  TOML datetimes arriving through `load_toml` / `from_toml`.

### Performance

- **Direct event→Python load materialization** — `safe_load`, `safe_loads` and `YAML().safe_load*`
  now build Python objects in ONE pass over the granit event stream (collect + recursive descent)
  instead of constructing the full `CustomNode` AST only to walk it again in `convert.rs`. Plain
  scalars resolve through the same schema chain, mapping keys keep their raw-text semantics, and
  duplicate-key errors are reported identically; anchored/tagged/merge/multi-document shapes stay on
  the AST pipeline via zero-cost pre-vetoes (a `&` byte scan, a separator line-scan, and a
  payload-level `<<` check). WSL wall-time: `safe_load_scalar_types` −21~25%, `safe_load` family
  −13~18%, fallbacks unchanged. The event-span anchor-replay variant was implemented, measured (+48%
  on anchored docs), and reverted; the alias-parity tests in `tests/test_direct_load.py` pin the
  anchored-shape behavior spec either way.
- **Anchor extraction byte gate** — `extract_anchors` returns empty after a single `&`
  byte-containment check; documents without anchors (the common case) skip the per-character quote
  state machine entirely. Rust-side `parse_*` divan benches improve 11–18% at the median (e.g.
  `parse_large` 31.2µs → 26.3µs; the extraction scan itself drops 1.5µs → 38ns).
- **Interned stream-event dict keys** — the fixed keys emitted per event by `parse_stream` /
  `load_stream` (`line`, `column`, `type`, `value`, `style`, `anchor`, `tag`) now reuse interned
  string objects via `pyo3::intern!`, eliminating one Python string allocation per key insert. WSL
  wall-time: `parse_stream` −34%, `parse_stream_multidoc` −39%, `load_stream` −22%.
- **Decomposition micro-benchmarks** — new `granit_events_*` benches isolate the pure granit
  event-pipeline cost from AST construction (bench-only).
- **Multi-document parse without per-document clones** — `on_document_end` moves the completed
  document into the collection instead of deep-cloning it (the next document rebuilds the result;
  the clone was pure overhead). WSL wall-time: `parse_all_docs` −9.7%, `safe_loads` (multi-doc)
  −9.5%, `YAML().safe_loads` −6.7%.
- **Streaming write reuses one buffer across documents** — new `direct_dump_into` writes each
  document into a reused `String`, and `dump_iterable` skips `normalize_doc`'s re-copy when the text
  already ends with exactly one newline (the normal case). WSL wall-time: `dump_stream_multi_doc`
  −27.2%, `dump_stream` −4.4%.
- **Scalar fast paths in the AST builder** — `unescape_double_quoted` returns early for quoted
  strings without backslashes (no per-char state machine) and `detect_chomping` pulls lines lazily
  instead of collecting every line of the document per block scalar. WSL wall-time: `to_dict` family
  −4~9%, `safe_load_scalar_types[strings/numbers]` −3~4%, no regressions.
- **Memoized block indentation** — the serializer's indent cache is now also used by block-scalar
  header writing, and `test_benchmark_api.py` gained `validate` and `load_file` benchmarks.

### Docs

- **Corrected the numpy guide's 0-D scalar section (all locales)** — the ``0-D Scalar Arrays``
  snippet claimed 0-D arrays "reshape to a single-element list" (`assert data == [42]`); the shipped
  behavior (pinned by `tests/test_numpy.py`) serializes them as bare scalars (`assert data == 42`).
  Text corrected in `docs/{en,zh,ja,ko}/guides/numpy.md`, and the en page gained a warning
  admonition documenting the 0-D `bool` → `1.0` rust-numpy quirk.
- **Corrected stale references across all locale docs (en/zh/ja/ko)** — `saphyr-parser` →
  `granit-parser`, YAML compliance 98.1% → 99.75% (405/406 suite cases), ABI3 support 3.9–3.13 →
  3.8–3.15 (py3.9+ → py3.8+), and benchmark tables updated to current CodSpeed CI numbers (parse
  21–43×, serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated from
  Criterion to divan (`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`).

</details>

## [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node metadata and style API · schema file IO · deep editing · numpy back on
cp314t</summary>

### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` / `Node.tag` read properties
  and `set_comment` / `set_anchor` / `set_tag` (plus `remove_*` variants) on
  `python/pyrs_yaml/node.py`, backed by new path-based edit operations in `py/editing/mod.rs` and
  `#[pymethods]` (`_set_comment_path`, `_set_tag_path`, `_set_anchor_path`, `_remove_*_path`,
  `_get_comment`, `_get_anchor`, `_get_tag`). Editing an alias or a missing path raises; standalone
  comments on inline scalar values and sequence items are now serialized on their own indented lines
  (fixes pre-existing broken round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Node style/format setters/getters** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping`
  read properties and `set_scalar_style` / `set_flow_style` / `set_chomping` methods, backed by
  `#[pymethods]` (`_set_scalar_style_path`, `_set_flow_style_path`, `_set_chomping_path`,
  `_get_scalar_style`, `_get_flow_style`, `_get_chomping`). ScalarStyle/Chomping now derive `Copy`.
  Non-scalar nodes return `None` / are no-op; aliases and missing paths raise.
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a verbatim tag (empty
  handle), and verbatim tags parsed from source survive round-trip: `Tag`'s `Display` emits `!<...>`
  wrapping for empty-handle tags, `parse_tag` recognizes the `!<...>` form, and stream events
  serialize tags through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema definition from a file
  and registers it; `list_schemas()` returns all registered schema names (built-in
  `failsafe`/`json`/`core`/`yaml1.1` plus custom). Exposes the existing `registry::names()` and
  wraps `register_schema` with file I/O.
- **Schema structural validation** — a `validate` section in a schema definition adds structural
  checks (path-qualified scalar types, `sequence_of`/`mapping_of` containers, `required` presence);
  the new `validate_against_schema(data, schema_yaml)` raises `YamlValidateError` listing every
  failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value (dict/list/scalar),
  detached from the document, for pasting via `set_value()`.
- **Deep editing API** — `doc.set_many({"$.path": value})` sets multiple paths (with wildcard `[*]`
  and deep-scan `..` support) in a single splice burst; `doc.sort_keys()` orders mapping keys in
  place; `Node.move(new_path)` relocates a subtree atomically; `Node.path` / `Node.find_first()` /
  `Node.value_eq()` add path access, first-wildcard lookup, and value comparison. Rust primitives:
  `sort_keys_path` / `move_path` / `set_many_path` + `apply_batch_edit`.
- **Property-based testing for 0.14+ features** — Rust proptests for `validate_node` (no panic,
  error paths exist), schema parsing, and style-settings round-trip; Python hypothesis tests for
  `set_many` wildcard equivalence, metadata-edit value preservation, and `sort_keys` idempotency.
  `hypothesis` moved to the `test` dependency group so CI (`uv sync --group test`) actually runs
  property tests.
- **Serializer fix** — standalone comments on **empty** flow containers (`key: {}` / `key: []`) no
  longer serialize to invalid YAML; they are demoted to inline comments (`key: {}  # note`).

### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the `--no-default-features` flag is
  removed from the cp314t build lines in `publish.yml` and `ci.yml`; rust-numpy 0.29 (already
  pinned) supports free-threaded Python since v0.24.0, so `numpy.ndarray` serialization is now
  available on free-threaded wheels when NumPy is installed (auto-detected at runtime; inert when
  absent). Closes the deferred Research & Exploration item "Numpy free-threaded re-enable".
- **granit-parser upgraded to 1.1.0** — the YAML 1.2 parser dependency moves from the `1.0` to `1.1`
  semver line. Backward-compatible maintenance release: adds `Options`/`emit_comments`
  configuration, new `new_from_*_with_options` parser constructors, fuzz-testing hardening, and a
  performance fix for validating large plain/block scalars (ASCII fast path). No API changes
  required in `pyrs-yaml-core`.

</details>

## [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>quoting and escape edges: single-quote backslash, BOM, noncharacters, multibyte
wrap</summary>

### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — a quoted value containing a
  backslash was routed to single-quoting, but single quotes cannot escape control characters or
  Unicode noncharacters, so the emitted YAML was unparseable. Such values now use double-quoting
  (`direct_dump` and the shared `write_plain_scalar` single-quote branch).
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted` now treat Unicode
  noncharacters (U+FFFE/U+FFFF and the plane-end twins) and U+FEFF (BOM) as requiring quoting:
  granit drops a plain U+FEFF as a document-start BOM and rejects raw noncharacters even inside
  quoted scalars.
- **Double-quoted escape width** — `write_double_quoted_scalar` escapes noncharacters and control
  chars; for code points above U+FFFF it now emits the 8-digit `\Uxxxxxxxx` form (the 4-digit `\u`
  form is only valid for the BMP).
- **Folded plain-scalar continuation indent** — `wrap_plain_scalar` continuation indent is no longer
  a fixed 2 spaces; it is derived from the value's start column on the current line, so folded plain
  scalars inside nested sequence/mapping items stay indented past the parent block indent (granit
  otherwise reports "simple key expected ':'").
- **Multi-byte wrap boundary** — `wrap_plain_scalar` now floors the wrap slice to a char boundary
  instead of panicking when a 4-byte UTF-8 character straddles the wrap column.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt` now pins
  `hypothesis>=6.113.0` so the publish workflow (which does not install the `dev` dependency group)
  can run the property test suite.

### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness that bypasses pytest
  `@settings` caps with a hostile strategy (control chars, NBSP, backslashes, long multibyte runs)
  across dump/parse/edit/idempotency.

</details>

## [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML Schema language and pluggable resolution · quoted scalars · empty
collections</summary>

### Added

- **YAML Schema Language** — define custom schemas as YAML files with a `rules` list mapping regex
  patterns to YAML types (`null`/`bool`/`int`/ `float`/`str`), plus an optional `extends` base
  schema. Registered via `register_schema(name, schema_yaml)` and used as `YAML(schema=name)`.
- **Inline dict schema** — the `schema` parameter of `YAML()`, `parse()`, `parse_file()`,
  `parse_all_docs()`, `safe_load()`, and `safe_loads()` accepts an inline dict, serialized and
  registered automatically.
- **Community Plugins** — `CustomType` base class with `can_parse`/`from_yaml`/`to_yaml`/`validate`
  methods; register via `register_type()` (imperative or decorator). Custom types handle tagged
  scalars on load and Python objects on dump.
- **Built-in plugins** — `!timestamp` (maps to `datetime`) and `!set` registered by default in
  `pyrs_yaml/plugins/`.

### Changed

- **Schema resolution is pluggable** — `YamlSchema` enum refactored into a `SchemaResolver` trait +
  `Schema` enum with a global `SchemaRegistry` pre-loaded with the four built-in schemas
  (`failsafe`, `json`, `core`, `yaml1.1`). Custom schemas register via the registry; built-in Core
  keeps its zero-cost `match` dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** — tagged scalars convert
  via `from_yaml()` on load; matching Python objects serialize via `to_yaml()` on dump.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses JSONPath for keys
  containing `.` or `[`; every key is treated as a top-level mapping key, consistent with
  `__getitem__`/`__setitem__`. Path access stays available via `find()`/`node()`.

### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now applies only to plain
  scalars (YAML 1.2): `safe_load('"true"')` returns the string `"true"`, not `True`. The serializer
  keeps negative numbers round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"` are emitted as quoted
  scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no longer yields an empty
  document that re-parses as `None`.

</details>

## [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96 and edition 2024 · direct writer and load fast paths · granit migration</summary>

### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now declare
  `rust-version = "1.96"` and `edition = "2024"`; CI pins the `build`/`test-freethreaded` jobs to
  Rust 1.96 for deterministic wheel builds and adds an `msrv-check` job running
  `cargo check`/`cargo test` at the MSRV to prevent silent MSRV drift (the `rust-lint` job stays on
  `stable`). The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API headroom (e.g.
  `assert_matches!`, stabilized 1.96) with no code migration needed. `TAG_REGISTRY` (tag handler
  storage) refactored to `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer** — Python→YAML
  serialization without intermediate `CustomNode` AST. Single-pass `direct_dump` replaces the old
  two-pass `pyobject_to_node` + `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on
  `from_dict` (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking** — when input has no `&`
  characters, skip `collect_anchors` + anchor resolution and use the simpler
  `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/ non-boolean first bytes
  return `Str` immediately, avoiding schema resolution overhead for the common case. (#59)
- **granit-parser migration** — saphyr-parser replaced with granit-parser 1.0.1 for native
  `Event::Comment` emission, eliminating the full-text `scan_yaml()` pre-scan. parse_small -18%,
  parse_large -21%, roundtrip_large -18%.

### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust Display drops the decimal (`42`
  → `42.0`) so floats round-trip as floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost more than the
  reallocations it avoided (serialize_10mb was ~14% slower); buffer growth is left to the Vec.

### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)` accept
  `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit via core
  `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model to YAML string via
  `model_dump(mode='json')` + `safe_dump`; `parse_as()` parses YAML string into a Pydantic model
  instance. Both use lazy imports, no hard dependency on pydantic. (#61)

### Internal

- **Split `py/mod.rs`** — monolithic 1786-line module broken into `document.rs` (YamlDocument),
  `yaml_instance.rs` (YAML class), `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` guard + `double_quoted_scalar()` constructor** — strings like `'true'` / `'42'`
  / `'null'` now emit as double-quoted scalars under the core schema instead of being misread on
  re-parse (`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed benchmarks unified on `codspeed-divan-compat`** — `exclude-allocations` removes
  allocator noise; cross-library benchmarks consolidated into `tests/test_benchmark_crosslib.py`
  with shared `tests/data/yaml_samples.py` fixtures and streaming coverage.

</details>

## [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>set(create_missing) · walk/scalars · monorepo workspace · parse hot paths,
Arc<str></summary>

### Added

- **`set(create_missing=True)`** - missing intermediate mapping keys along the edit path are created
  as nested mappings (e.g. setting `a.b.c` on `a: 1` creates `b` and `c`); index segments that miss
  are still an error, and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal yielding `Node`
  objects, avoiding per-node `to_dict()` resolution. `walk()` returns all nodes; `scalars()` returns
  only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate` (key_eq, navigate,
  navigate_mut, normalize_index, mapping_key_index), `editing::region` (line helpers, node_is_flow,
  extend_delete_over_comments, nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null values, deeply nested,
  flow collections, mixed types.

### Changed

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/` (pure Rust, no PyO3) and
  `crates/pyrs-yaml/` (PyO3 bindings). Root `Cargo.toml` is now a workspace. Old `src/` directory
  and `build.rs` removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy duplicate-key detection,
  `shift_insert` merge prepending, and skipped `DocumentEnd` deep-clone for single-document parses
  cut large-document parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event text share allocations via
  `Arc<str>`; AST nodes shrink 8 bytes and clones become refcount bumps instead of deep copies.

### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping chain no longer duplicates
  the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now eligible for the value
  write (the eligibility check no longer runs after the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip previously dropped standalone
  comments attached to simple-key nodes; now preserved (two regression tests).

</details>

## [0.11.7] - 2026-08-04

<details markdown="1">
<summary>release-guard static assertions replace the always-red stub check · numpy
tracking</summary>

### Changed

- **stub-build-check replaced with release-guard** - the always-red container build (`validate.yml`)
  that deliberately failed to reproduce the v0.10.0 `--generate-stubs` failure mode is replaced with
  three static assertions that **pass** when the repo is correct: `grep` guards `publish.yml`
  against `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked, and `test -f`
  checks `py.typed` exists. The job now gives green CI on correct state, red only on regression.

### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free- threaded support
  status (PyO3/rust-numpy#476) as a dependency for re-enabling ndarray serialization on cp314t
  wheels when the Rust binding matures.

</details>

## [0.11.6] - 2026-08-04

<details markdown="1">
<summary>cp314t wheels drop numpy · free-threaded CI validation · install docs</summary>

### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with `--no-default-features`, so
  rust-numpy is excluded entirely (smaller binary, no runtime probe). `safe_dump` on a
  `numpy.ndarray` raises `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15) keep
  full ndarray serialization.

### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds and tests with
  `--no-default-features`, matching the shipped free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded wheels are numpy-free (ndarray
  serialization unavailable on cp314t).

</details>

## [0.11.5] - 2026-08-04

<details markdown="1">
<summary>parser robustness items closed by an audit that found no fixable case</summary>

### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus
  (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no
  fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 /
  yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key
  strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and
  `tests/test_strictness_audit.py`.

### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current
  rejection/acceptance behavior (both directions), so future parser changes cannot silently regress
  strictness or over-reject.

</details>

## [0.11.4] - 2026-08-04

<details markdown="1">
<summary>duplicate empty keys accepted (2JQS) · compliance counts correct rejections · tab
decoding</summary>

### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches
  yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate
  despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is
  one tab, fixing tab-encoded suite cases

### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2
  grammar, matches PyYAML/libyaml)

</details>

## [0.11.3] - 2026-08-03

<details markdown="1">
<summary>streaming write · line-offset cache · publish pre-validation · compliance report</summary>

### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with
  document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API
  unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures
  before Release

</details>

## [0.11.2] - 2026-08-03

<details markdown="1">
<summary>parse skips splice eligibility · linear-cursor layout check (v0.11.0 regression)</summary>

### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors
    - chunk) memory

### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on
  the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments
  -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets
  (monotonic source-order traversal)

### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now
  internal to `YamlDocument` and computed on demand

</details>

## [0.11.0] - 2026-08-02

<details markdown="1">
<summary>surgical serialization: an edit writes back without re-laying-out the document</summary>

### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based
  splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions,
  non-default layout documents, merged keys, CRLF/BOM documents, and after materialization
  (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization
  previously dropped them — deliberate behavior difference)

</details>

## [0.10.0] - 2026-08-01

<details markdown="1">
<summary>in-place editing API, with edit benchmarks</summary>

### Added

- **In-place editing** — edit parsed documents without losing formatting metadata:
    - Path API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`,
      `doc.delete(path)`, `doc.rename(path, new_key)` with JSONPath-style paths (`$.a.b[0]`); root
      sugar via `doc["key"] = value` and `del doc["key"]`
    - Node API: `doc.node()` / `doc.find(path)` return `Node` objects with `set_value` / `append` /
      `insert` / `delete` / `rename`, plus tree traversal (`parent`, `children`, `walk`, `filter`)
    - Full metadata preservation — replaced scalars keep comment/anchor/tag/quoting; renamed keys
      keep position and comments; mapping order preserved on delete
    - Atomic edits — failed operations leave the document (and its revision) untouched
    - Lazy source re-sync — `source()` / `to_yaml()` / `reparse()` re-serialize only after a
      successful edit
    - Stale-node detection — `Node` access after a document edit raises `YamlDocumentError` (with
      `RuntimeWarning`)
    - New exceptions: `YamlEditError`, `YamlPathError` (i18n across en/zh-CN/ja-JP/ko-KR)
    - Alias-aware editing — setting an alias's own path replaces it in place; editing through an
      alias raises `YamlEditError`
- **Edit benchmarks** — 6 new divan benchmarks in `benches/yaml_bench.rs` (set/insert/delete on
  small–large documents)

### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

</details>

## [0.9.0] - 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15 and no-GIL · tag handler registry · pydantic · .pyi stubs</summary>

### Added

- **Python 3.13, 3.14 and 3.15 support** — PyO3 `abi3-py38` wheel covers Python 3.8-3.15 (GIL
  build); `abi3t` + `abi3t-py315` provide free-threaded stable ABI
- **Free-threaded CPython (no-GIL) support** — `#[pymodule(gil_used = false)]` declares module as
  thread-safe for free-threaded Python; `Py_GIL_DISABLED` cfg flag gates numpy (rust-numpy has no
  free-threaded support yet — numpy feature must be disabled for free-threaded builds via
  `--no-default-features`)
- **CI free-threaded job** — new `test-freethreaded` workflow job validates compilation and tests
  against Python 3.14t
- **`pyo3-build-config` build dependency** — enables `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]`
  etc. compiler flags via `build.rs`
- **`numpy` made optional** — feature-gated behind `numpy` feature (default enabled); excluded
  automatically under `Py_GIL_DISABLED`
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`,
  `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs`
  all accept the flag; duplicate mapping keys raise `YamlDuplicateKeyError` by default,
  `last value wins` when allowed
- **`SerializeOptions` expansion** — `doc.to_yaml_with_options()` gains `width` (line wrapping, 0 =
  off), `indent_mapping`, `indent_sequence`, `indent_offset` alongside existing
  `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth` (`src/py/mod.rs:432`)
- **Tag handler registry** — `register_tag("!custom")` decorator and imperative forms +
  `clear_tag_handlers()`; scalar nodes carrying a registered tag are transformed through the handler
  (`src/py/tag_registry.rs`)
- **Tag handler chaining with priority** — multiple handlers per tag run in ascending `priority`
  order; `YamlTagSkip` lets a handler pass through to the next, fallback keeps the original value
- **Pydantic integration** — `parse_as(Model, yaml, **yaml_kwargs)` parses YAML and validates
  against a Pydantic v2 model; raises `ImportError` with guidance when pydantic is absent
  (`python/pyrs_yaml/pydantic.py`)
- **`.pyi` type stubs** — auto-generated by maturin and committed so `register_tag`, `parse_as`,
  `to_yaml_with_options` and the new exceptions are visible to type checkers

### Changed

- CI Python matrix expanded: 3.8-3.14 across ubuntu, windows, macos
- Stable ABI: `abi3-py39` → `abi3-py38` (wider Python 3.8+ support), added `abi3t` + `abi3t-py315`
  (free-threaded stable ABI)
- `pyproject.toml` classifiers updated with 3.13, 3.14, 3.15 entries
- **CI optimization: redundant Rust compilation eliminated** — a single `rust-lint` job runs
  `cargo clippy` + `cargo test` once; the build job produces one abi3 wheel per OS which test jobs
  install instead of running `maturin develop`, removing Rust compilation from 21 matrix jobs (~86%
  fewer compiles); `Swatinem/rust-cache` added to all jobs
- **pydantic test dependency** — `pydantic>=2.10.6` added to `[dependency-groups] test` and
  `.ci/requirements-test.txt` (SSOT via `uv sync` in ci.yml)

### Fixed

- **Windows DLL loading** — removed `#[cfg(test)]` block from `src/py/tag_registry.rs` which broke
  `import pyrs_yaml` on Windows (`250b8d0`)
- **Python 3.8 compatibility** — `from __future__ import annotations` in `pydantic.py` (`63d2495`)
- **CI pydantic skip** — `pytest.importorskip("pydantic")` so tests pass when pydantic is not
  installed (`7be011d`)
- **CI glob expansion on Windows** — `shell: bash` for `pip install dist/*.whl` (PowerShell does not
  expand `*`) (`2f7778d`)
- **Non-string tag handler returns now raise `YamlTagError`** — a handler returning a non-`str`
  value (previously silently ignored, keeping the original scalar) now errors with
  `Tag handler '!x' must return a string` (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` indent wiring** — `indent_mapping`/`indent_sequence`/`indent_offset` are
  now honored by the serializer (previously dead fields); each defaults to `indent_size`/0 when
  omitted (`src/serializer.rs`)
- **`width` no longer hangs for tiny values** — `width < continuation indent` falls back to emitting
  the remainder unwrapped instead of looping forever (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — new function to unregister a tag handler; complements
  `register_tag`/`clear_tag_handlers` (`src/py/tag_registry.rs`)
- **`duplicate-key` errors are i18n'd** — `YamlDuplicateKeyError` messages now flow through
  `format_i18n_error` across all 4 locales (`src/i18n/locales/*.yml`)

</details>

## [0.8.0] - 2026-07-30

<details markdown="1">
<summary>YAML() instance API · Python Node API · doc.version · MergedView · lifecycle
warnings</summary>

### Added

- **`YAML()` instance API** —
  `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` with reusable
  configuration; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()`
  methods
- **Python `Node` API** — `Node` class with `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`,
  `children`, `root_type`, `value` for AST navigation; JSONPath-like query language (`$.key.sub`,
  `$.arr[0]`, `$..deep`)
- **`doc.version` metadata** — `YamlDocument.version()` returns the YAML spec version
  (default "1.2")
- **`MergedView`** — `doc.merged()` returns a read-only dict-like view with merge keys resolved
- **Lifecycle warnings** — `Node.release()` to explicitly invalidate a node; stale access emits
  `RuntimeWarning` + `YamlDocumentError`

### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

</details>

## [0.7.1] - 2026-07-30

<details markdown="1">
<summary>ryaml benchmark comparison · compliance threshold raised · Divan benchmark
migration</summary>

### Added

- **ryaml benchmark comparison** — `tests/test_benchmark.py` now benchmarks against `ryaml` (Rust
  YAML library) alongside PyYAML and ruamel.yaml; `benchmark_compare.py` rewritten as a feature
  comparison report (`tests/test_benchmark.py:25-28`, `.github/workflows/ci.yml:219`)
- **CI compliance threshold raised** — YAML Test Suite compliance gate increased from 70% to 75% in
  `test_compliance_report()`; valid parse rate gate at 95% (`tests/test_yaml_suite.py:251`)
- **CI dependency consolidation** — added `.ci/requirements-test.txt` and
  `.ci/requirements-test-lite.txt` for unified test dependency management across publish workflow
  and local dev
- **Benchmark modernization** — migrated from `pytest-benchmark` to `pytest-codspeed` for faster
  C-extension-based statistical benchmarking; all CI jobs now use `-r .ci/requirements-test.txt`
- **Rust benchmarks migrated to Divan** — replaced `codspeed-criterion-compat` with
  `codspeed-divan-compat` v5.0.1; 16 benchmarks rewritten from Criterion groups to `#[divan::bench]`
  attributes (`Cargo.toml`, `benches/yaml_bench.rs`)

### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature
  comparison/reporting tool

</details>

## [0.7.0] - 2026-07-29

<details markdown="1">
<summary>serializer max_depth guard · hot-path optimization · pytest-benchmark migration</summary>

### Added

- **Serializer `max_depth` guard** — `serialize_node_internal` now tracks recursion depth and raises
  `YamlMaxDepthError` when exceeding the limit (default 1000), matching the parser's protection
  (`src/serializer.rs:135-145`)
- **Serializer hot-path optimization** — 5 optimizations targeting block-style serialization for
  ~4.9% roundtrip speedup:
    - Inlined `write_anchor_tag` and `write_inline_comment` None checks (eliminates method calls for
      ~99% of nodes)
    - `write_indent` hot/cold path split (direct index for cached levels ≤64)
    - `write_plain_scalar` fast path for short ASCII alphanumeric strings (≤8 chars)
    - `write_scalar_for_key` direct dispatch for Plain scalars (avoids dispatch chain)
- **pytest-benchmark migration** — Python benchmarks migrated from raw `time.perf_counter()` to
  `pytest-benchmark` for statistical rigor, structured JSON output, and CI integration
  (`tests/test_benchmark.py` + updated `tests/test_performance.py`)

### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

</details>

## [0.6.0] - 2026-07-27

<details markdown="1">
<summary>async serialization · JSON Schema validation · to_json() · incremental re-parse</summary>

### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`,
  `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)`
  method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`);
  `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
- **29 new tests** across `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

### Changed

- `YamlValidateError` registered as new custom exception (inherits `ValueError`)
- `rust_i18n::i18n!` macro path updated to `"src/i18n/locales"`
- `validate_translations()` test paths updated to match new locale directory

### Removed

- Deleted redundant `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` (never referenced by rust-i18n)
- Moved `locales/*.yml` → `src/i18n/locales/` (co-located with i18n module)

### Dependency Changes

- Runtime dependency: `jsonschema>=4.25.1`
- Dev dependency: `pytest-asyncio>=0.23` (moved from runtime, no longer pinned)

</details>

## [0.5.0] - 2026-07-27

<details markdown="1">
<summary>Serializer::write_node fix · YAML_SCHEMA constant · development documentation</summary>

### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in
  `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on
  edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec
  URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python
  commands and direct `cargo` for Rust commands

</details>

## [0.4.0] - 2026-07-27

<details markdown="1">
<summary>132 gap-filling tests: i18n, multi-document, bytes input, suite cases, fidelity</summary>

### Added

- **132 new gap-filling tests** — comprehensive coverage for previously untested APIs
- **i18n function tests** — `set_language`, `get_language`, `list_languages`, `detect_language`,
  `negotiate_language`
- **`parse_all_docs`** dedicated test suite — single doc, multiple docs, empty, comments
- **`parse_file` success case tests** — basic parsing, comments preservation, file-not-found error
- **`to_yaml_with_options` tests** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys`
  order preservation
- **`to_dict()` method tests** — scalar root, nested, list, bool, null, anchor resolution, empty
  mapping/sequence
- **YamlDocument dunder method tests** — `__repr__`, `__str__`, `__contains__`, `__len__`,
  `__iter__`, `__getitem__`, `root_type()`
- **Bytes input tests** — `parse(b"key: value")`, UTF-8 bytes, invalid UTF-8 error
- **Unicode & special character tests** — CJK, emoji, roundtrip, CRLF line endings, duplicate keys
- **`safe_load`/`safe_loads` feature coverage** — anchors, merge keys, block scalars, flow
  collections, special floats, type resolution
- **`from_dict` edge cases** — special characters in keys, nested lists, None values, empty
  dict/list
- **`from_json` round-trip** — nested structures, arrays, invalid JSON error
- **`dump_file` tests** — success path, invalid path error
- **YAML Test Suite individual case tests** — octal, hex, scientific notation, NaN, infinity, merge
  keys, explicit/implicit keys, bool/null variants, block scalar strip (`|-`), flow collections
- **`resolve_merges` parameter tests** — preserving `<<` when disabled, resolving by default
- **Flow collections roundtrip** — root-level and nested flow mapping/sequence
- **Anchor on non-scalar nodes** — mapping anchors (`&defaults`) and sequence anchors (`&items`)
- **Sequence indexing tests** — positive index, out-of-range error
- **Merge key integration** — roundtrip with resolved and unresolved merge keys
- **Tag preservation** — `!!seq` and `!!map` tag test coverage
- **Comment preservation** — inline and standalone comment tests on complex structures

### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to
  match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

</details>

## [0.3.0] - 2026-07-27

<details markdown="1">
<summary>NumPy ndarray serialization (N-D) · quoted-scalar typing · negative round-trip</summary>

### Added

- **NumPy ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()`
  now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring
  single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to
  `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D),
  negative numbers, infinity, NaN, empty arrays, and edge cases
- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST
  nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys`
  parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS x 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker

### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting
  with `-`; negative numbers are now quoted during serialization and correctly parsed back as
  integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any
  dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting
  (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)
- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values
  instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles

### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch
- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with
  `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`,
  `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context

</details>

## [0.1.0] - 2026-07-25

<details markdown="1">
<summary>initial release: YAML 1.2 AST with full metadata, round-trip, PyYAML-compatible
API</summary>

### Added

- Initial release with YAML 1.2 compliance via saphyr-parser
- Custom AST with full metadata (comments, anchors, tags, chomping, scalar styles)
- Round-trip preservation of comments, anchors, tags, and formatting
- PyYAML-compatible API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` conversion functions
- `read_markdown`/`read_markdown_str` for YAML frontmatter extraction
- Block scalars (`|`/`>`) with chomping indicators (`|-`/`|+`/`>-`/`>+`)
- Escape sequences (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 type resolution (null, bool, int, float, infinity, NaN)
- Merge key resolution (`<<: *alias`)
- Complex keys (sequence/mapping as key)

</details>
