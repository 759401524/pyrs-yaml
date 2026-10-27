# Engine boundaries

Deliberate limits of the hub model, and the fuzz findings that closed engine surfaces. The
boundaries are pinned by characterization tests so they cannot drift silently; values always
round-trip, and what is not reproducible is certain TOML-only *stylistic* forms.

Not site content: `zensical.toml` builds `docs/en`, and `check_i18n.py` mirrors only pages under
it, so this file has no `zh`/`ja`/`ko` twin and is not in the published navigation. It is an
engineering record, in the same category as `QUALITY_MATRIX.md` and `ROADMAP.md` at the root.

Related documents: [quality ledger](quality-ledger.md), [performance status](perf.md).

## Known Engine Boundaries (2026-09-29)

Deliberate hub-model limits in the TOML spoke, pinned by characterization tests in
`tests/test_toml.py` (classes `TestSectionHeaderCommentBoundary`) so they cannot silently drift.
Both stem from the shared YAML text hub (`from_toml` → YAML → `to_toml`), where YAML is the
interchange format. **Values always round-trip losslessly**; only certain TOML-only *stylistic*
forms are not reproducible.

- **Table-header inline comment (`[sec] # note`).** A TOML header's inline note rides the header's
  own mapping through the hub, and the YAML writer places a container's inline note on the last line
  of that container's body, which `to_toml` then renders as a trailing comment on the table's last
  `key = value` line (`a = 1` + `[sec] # note` + `x = 1` → `a = 1` + `[sec]` + `x = 1 # note`),
  already stable after one round. What remains unfaithful is the position — not the existence of the
  comment, and not which table it belongs to; pinned by
  `TestSectionHeaderCommentBoundary.test_header_inline_comment_is_relocated_not_dropped` and
  `test_relocated_note_lands_on_the_tables_last_entry`, which assert survival, the in-table slot,
  and the one-round fixed point. This bullet used to blame "the locked granit comment-capture model"
  and to claim pure YAML loses the position too; both measured false on 2026-10-25. granit is not in
  this path — `pyrs_toml`'s own scanner captures the header note — and a YAML document's
  `sec: # note` does keep its position through the hub, reaching the TOML header line from the key
  node's own inline slot (`test_a_note_on_a_yaml_key_line_reaches_the_toml_header`). What is left is
  the container-note placement rule the `api:set-comment-container-owner` residual records, whose
  blast radius is every comment the YAML writer emits — so changing it stays explicitly declined
  rather than silently shipped. Standalone/leading comments above a header and trailing comments on
  a leaf `key = value` line *are* preserved in place. The same key-node slot is also read now for an
  entry *inside* a section body whose value is a table or an array
  (`test_a_note_on_a_key_line_inside_a_section_body_survives`): the inline-table spelling settles in
  one round, the array spelling expands and moves the note onto the last element's line, settling
  there — a relocation, never a loss.
- **Integer source spelling (`0b1010`, `1_000`, `+5`).** Closed by `(cp)`, widened by `(eq)` — the
  spelling now survives the round trip. It was canonicalised to decimal while the bridges read
  scalar text only: YAML Core has no `0b` spelling and cannot read `_` separators, so a faithful
  untagged spelling re-resolves to a string and design rule D1 ("only spellings YAML Core also
  accepts get preserved") excluded it. Once a standard tag decided the type in every format (`(bu)`,
  PR #340), the projection could state the type instead of dropping the text:
  `from_toml("b = 0b1010")` is the hub scalar `!!int 0b1010`, which the loader reads as `10`,
  `to_toml` writes back as `0b1010`, and the JSON writers project as `10`. Since `(eq)` the parser
  records the token text whole instead of a cleaned copy, so `_` separators and an explicit `+`
  survive too — both are legal TOML spellings (`underscored` and `signed-int` in the ABNF), and the
  writer's literal gate accepts the same grammar it emits. Pinned by
  `tests/test_toml.py::TestBinaryIntegerFidelity`, `TestIntegerSpellingFidelity`,
  `binary_spelling_reaches_the_hub_under_an_int_tag` and
  `decimal_spellings_reach_the_hub_word_for_word`. Two things remain deliberately not faithful: a
  sign on a radix literal (`-0x1F`) is not a TOML integer at all — the reader refuses it, so there
  is no spelling for the writer to keep — and an *untagged* `0b1010` or `1_000` in a hand-built node
  still re-reads as a string, because the tag is what carries the type. Hex and octal written in
  canonical digits need no tag at all, since YAML Core resolves them back to the same integer.

### Fuzz findings (weekly-scheduled `fuzz.yml`, engine surfaces, 2026-10-02)

`fuzz.yml` now fuzzes all four engine surfaces on a weekly cron (plus `workflow_dispatch` and a
`fuzz/**` path trigger), uploading minimized crash artifacts on failure so the finding pipeline is
closed: crash -> regression test -> seed -> fix. Eleven crashes have been surfaced and fixed to
date:

- The JSON comment char-boundary panic (#213).
- Unterminated-quote anchor names (#215).
- Double-decoded double-quoted scalars (#216).
- The 12-byte `&&&&:<LF>#&&&:&` anchor restructure (#218 — the raw anchor scanner took `:`+EOL as
  name material, re-harvested phantom anchors from comment text and overlapping `&` runs, and
  shifted every later id-name pairing; the emitted `&&&&: v` then re-parsed as anchor `&&&` plus a
  value indicator). The full parse -> to_yaml -> re-parse loop is now pinned at pipeline level
  (#224).
- The nested self-referential merge stack-overflow (#226 — `resolve_mapping_merges` re-walked
  freshly prepended anchor clones with the path cycle-guard already popped, so a `&b` body re-using
  `*b` expanded a fresh clone every round and the descent overflowed the native stack; the tail walk
  now recurses only into the mapping's own children, plus a `MAX_MERGE_DEPTH` budget).
- The trailing-colon anchor-name emit drift (#227 — `write_anchor_tag` emitted `&name` bare, so a
  name ending in `:` merged with the appended space into a value indicator and lost one character
  per round; unsafe names are now emitted as quoted `&"name"` anchors).
- The quoted anchor name swallowing a line break (#228 — `scan_anchor_name`'s quoted branch read the
  name across a carriage return to a closing quote on a later line, producing `X-\r:&`; the growth
  per round was only caught at pipeline level. granit ends an anchor token at CR/LF, so a closing
  quote past a line break no longer qualifies).

Four of the seven above were one subsystem: the raw anchor scanner's name grammar drifting from
granit's tokenizer. Root-fixed in #230 by mirroring `scan_anchor` exactly - maximal `is_anchor_char`
run, no invented quoted-anchor or value-indicator-colon branch, anchor tokens skipped atomically -
which subsumes #215/#218/#227/#228 by construction rather than per shape. Block scalars got the same
treatment (#231 plus the fold-aware newline writer, closing cfb3fa83, c18cb1fd, 490c4beb and
6288e5be): read the authoritative reader first, then close the writer against its exact read map.

**The rest, by family.** Each line is one root cause, the inputs it closed, and the assertion that
holds it - not a per-shape patch.

- **Blank set.** Six places asked `char::is_whitespace()` / `str::trim()`, which is Unicode and also
  matches NBSP, U+0085 and U+2028/U+2029, while granit's blanks are SP and TAB. So `<NBSP>42`
  resolved to the integer 42, a NBSP-only document resolved to `Null`, and a multi-line NBSP scalar
  resolved to `Null` - which short-circuits `needs_double_quoted`'s "a non-string may be emitted
  plain" clause, so the writer emitted raw line breaks that collapsed on re-read. All six now use
  `pyrs_schema::is_yaml_blank`; the JSON-family resolvers keep Unicode whitespace on purpose,
  because JSON5 treats it as structural. Seeded as `former-crash-{e92ce66f,512814,b44481b2}.seed`,
  with crash-a1516147 and crash-79fef575 subsumed by the same per-form tag encoder.
- **Line break set.** `line_break_between`, the test that decides whether a trailing note belongs to
  the node before it or the one after, scanned the gap for `\n` while granit splits lines on `\n`,
  `\r\n` and a lone `\r`. A CR-only document therefore read as one long line, every note after a
  break bound backwards onto the previous container, and that container's emission settled only on
  the second round. Seeded as `cr-only-break-note-ownership.seed` and pinned by
  `settles_in_one_round::a_note_after_a_cr_only_break_annotates_the_item_that_follows`, which also
  requires a document and its LF spelling to emit one text. The comment scanner, the scalar trim and
  this function's own blank trim already asked for both bytes; the one remaining `\n`-only table is
  `scan_yaml`'s line offsets, which serve only the splice layout gate, and that gate returns false
  for any text containing `\r`.
- **Explicit-key markers.** A note that trailed a marker-only line is reported one level shallower
  than it was recorded, so the writer bubbles it up the first-pair-key spine
  (`former-crash-ac5d9043.seed`, minimised 8 bytes); a note riding the entry *after* a marker needed
  granit's span backed over the line break it swallows (`former-crash-{0e1c4378,105de752}.seed`); a
  container's own inline note has to borrow the last value line that can host one
  (`former-crash-11ced252.seed`, 13 bytes); the two lifts a marker line performs have to compose
  (`former-crash-456176be.seed`); the whole spine, not its first stack, has to be taken
  (`former-crash-{f8525a9e,c9031de4}.seed`); and every stack clears a tag header in one round
  (`former-crash-{e6551c75,fbc8f2ae,8f7085b0,77a8039b}.seed`).
  granit 2.0 also *accepts* an explicit key with nothing in it, which the writer cannot spell back:
  a bare `?`, a `?` followed by a space, a `?` followed by a tab and `?\n:\n` all parse to
  the mapping whose key is the null
  scalar, and the only honest emission is the null pair `~: ~` — so those four converge in one
  round while the `?` marker itself is lost, exactly as it is lost for a scalar explicit key
  (`? a\n: b` → `a: b\n`). Ruled 2026-10-14: accepted rather than refused, because the pinned
  parser decides what is valid YAML and this is a spelling change, not a data change — the document
  re-reads to the same mapping and the second round equals the first. A key that cannot round-trip
  without the marker keeps it: `? ?` emits the nested-explicit form `?\n  ~: ~\n:\n  ~`. Pinned by
  `serializer::tests::a_degenerate_explicit_key_is_accepted_and_re_emits_the_null_pair`.
- **Quoted text is not a comment.** `tail_line_has_note` asked whether a finished line "already has
  a `#`" by scanning raw bytes, so `"#": !-  # note` refused to host its own note - the predicate
  now walks the line with quote state and applies YAML's own space-before-`#` rule
  (`former-crash-22cb5f67.seed`, plus `comment_marker_scan_respects_quoting` on the predicate
  alone).
- **Merge identity.** The merge pass looked `<<` up by whole-node equality, so a `<<` carrying a
  comment was invisible to it and one round trip changed the document's *meaning*; `is_merge_key`
  now asks what YAML asks and the entry is addressed by position
  (`former-crash-{69931a77,0a6fe677,2d3dab18,f88c2382}.seed`, minimised to 10 bytes). A merged pair
  that repeats a key the mapping owns is now overridden instead of silently surviving twice
  (`former-crash-3495cc86.seed`, 19 bytes).
- **Anchor recovery.** A name granit never reports is recovered by scanning left from the content,
  which used to accept a `&` on a line that had already opened a comment
  (`former-crash-04fddeb8.seed`); the same rule refused a `&` after `!!` mid-comment
  (`former-crash-68adf94c.seed`).
- **Empty block headers.** granit reports the *default* chomping for a header with no content to act
  on, so `>+` cannot survive; `effective_chomping` now drops the indicator the way both writers
  already dropped the indent (`former-crash-{89d81d99,b5dcc38f}.seed`).
- **Null keys.** Two spellings of null differ in metadata, so `IndexMap` kept both and the document
  shrank a line per round; ingest folds them to the entry a re-read produces
  (`former-crash-00e31785.seed`, minimised 9 bytes), and `<<` folds the same way
  (`former-crash-973bd522.seed`).
- **Note survival.** A single pending slot overwrote every standalone line but the last - `# alpha`
    - `# beta` + `key: 1` kept one note - and the round-trip tier could not see the class at all,
      because a text that is stable and merely short a note passes an idempotence oracle.
      `leading_comments` is a `Vec` now (the representation decision this ledger asked about first),
      and `crates/pyrs-yaml-core/tests/note_survival.rs` is the assertion that was missing.

**Four method rules came out of them, and each is a gate now.** A symptom shared by two inputs is
not a shared cause until the ASTs say so - the "confirmed twice" call in this ledger was wrong, and
so were three other groupings. A seed is a promise that an input passes, so an unfixed input stays
an artifact (`ce106ccc` was seeded only once (n) made it pass). An oracle that reddes correct output
is worse than none - the note-survival probe was fixed three times before it was trusted. And an
inferred cost is not a reason to defer: the deferred spine walk was refused by a sentence nobody had
measured, and the measurement reversed it.

## What the split actually buys, counted (2026-10-14)

The architecture pillar claims two things for the crate matrix - fine-grained dependency
introduction, and lower incremental compile cost. One of them is now measured and the other is not,
and this section records the difference rather than letting the second claim ride on the first.

**Dependency closure, measured.** `cargo tree -p <crate> -e normal --prefix none`, unique crate
names, on `main` at `27031d12`:

| crate a consumer can depend on alone | crates pulled in |
|:--|--:|
| `pyrs-schema` | 1 |
| `pyrs-ast` | 10 |
| `pyrs-event` | 11 |
| `pyrs-json` (JSON + JSONC + JSON5) | 12 |
| `pyrs-toml` | 12 |
| `pyrs-yaml-core` | 75 |

A qualifier the first draft lacked, because the table over-reads: `cargo tree -e normal` counts the
closure a consumer *builds*, not what ends up linked. Part of `pyrs-yaml-core`'s 75 is host-only.
The concrete case is a second `granit-parser`, 1.3.0, arriving through

```text
granit-parser v1.3.0 <- serde-saphyr <- rust-i18n-support (feature "codegen")
  <- rust-i18n-macro (proc-macro) <- rust-i18n <- pyrs-yaml-core
```

`rust_i18n::i18n!("src/i18n/locales")` (`crates/pyrs-yaml-core/src/lib.rs:27`) parses the locale
YAML at compile time, and `rust-i18n-macro` requests `rust-i18n-support/codegen` unconditionally, so
that parser is neither dead weight nor removable by a narrower feature selection - and a
proc-macro's dependencies are host artifacts loaded by rustc, not something the wheel or the `pyq`
binary links. What it does cost is compile time, which is a real column of this pillar but a
different one, and the honest form of the claim is: twelve crates to build for a JSON5 consumer
against seventy-five for the YAML engine, of which some build only.

So a project that only needs to read JSON5 carries twelve crates, not seventy-five: the YAML
engine's parser, its schema resolver, `granit-parser`, `rust-i18n`, `regex`, `proptest`-adjacent
machinery and the PyO3 stack are simply not in that graph. That is the fine-grained claim, and
`AGENTS.md`'s `no_std` story is the same graph proved a second way - the `no_std` job builds
`pyrs-ast`, `pyrs-event`, `pyrs-schema`, `pyrs-json`, `pyrs-toml` and `pyrs-yaml-core` for
`thumbv7em-none-eabi`, a target with no std at all, so those closures cannot be reaching `std`
anywhere.

**Incremental cost, probed - and the probe says the claim is only half true.** Unit counts, not wall
clock: this box's timings move with the shared-folder filesystem, and `cargo check` prints
`Checking` rather than `Compiling`, which is why a first attempt counted zero units by grepping the
wrong verb. Method that works: warm `cargo check --workspace` until a clean tree reports 0, append
one comment line, count units, restore.

- Edit `crates/pyrs-ast/src/ast.rs` (the hub) -> `cargo check --workspace` re-checks **7** units.
- Edit `crates/pyrs-yaml-core/src/serializer.rs` (one engine) -> `cargo check -p pyrs-yaml-core`
  re-checks **1**, and `cargo check -p pyrs-json -p pyrs-toml` re-checks **1**: `pyrs-json`, while
  `pyrs-toml` stayed fresh.

So the YAML engine does not touch the TOML engine, and the hub's 7 is the price of the shared node
model - both are what the split is supposed to buy. The anomaly reported here in an earlier draft
did not survive a controlled re-run. With one comment line added to
`crates/pyrs-yaml-core/src/serializer.rs`, `cargo check -p pyrs-json -p pyrs-toml` re-checks **0**
units both in an isolated `CARGO_TARGET_DIR` and in the shared `target/` directory - verified with
the marker read back out of the file (count 1) and a warmed baseline of 0. So the reading that a
YAML-engine edit pulled the JSON engine in was produced by the probe itself: the `cp`/`mv`
round-trip used to restore the edited files bumped the mtimes of unrelated sources, and the recheck
attributed to coupling was the probe's own residue. Engine isolation therefore holds as the closure
table predicts, and the hub's 7 is the only real fan-out.

Two traps this chased, both worth naming because they fabricate results rather than hiding them:
`cargo check` prints `Checking`, so counting `Compiling` yields a convincing zero for every run; and
on a shared-folder filesystem a lockfile cargo has just rewritten can still read back as the old
version, so `cargo tree` - not the lock text - is the control that says which version was measured.

This file therefore claims the closure table, the hub fan-out of 7, and that editing one
engine costs the others nothing - each with the command that shows it.

Two environment facts belong to this measurement because they wasted it first: `cargo +1.97.1` on
the Windows host fails with `missing manifest in toolchain 1.97.1-x86_64-pc-windows-msvc` (that
install is broken; use WSL for the pinned toolchain), and a `touch` is not an edit here - cargo
fingerprints contents, so a mtime-only change legitimately rebuilds nothing.

## The Rust test tier against the Python runtime (2026-10-21)

The decoupling sequence started from a premise - "the Rust test and benchmark tiers depend on
libpython" - and the premise is false at the binary level, so this section records what is true
instead of what was assumed. The measurement: build every test artifact
(`cargo test --workspace --no-run`), then read each PE import table for `python*.dll`.

| reading | result |
|:--|:--|
| artifacts scanned under `target/debug/deps` | 848 |
| artifacts naming a `python*.dll` | 1 - `pyrs_yaml.dll`, the extension cdylib itself |
| Rust test binaries naming one | 0 |

The linker prunes a dependency no symbol references, so a crate that compiles `pyo3` but never
calls into it links no interpreter. The coupling that survives is a *build-graph* coupling, not a
link one: `cargo tree -p pyrs-yaml -e normal --prefix none` still names `pyo3`, `pyo3-ffi`,
`pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config` and `numpy`, so a consumer of the binding
crate builds a Python runtime's worth of crates whether or not it links one.

**What moved, and why it was movable.** The binding crate held exactly two `#[cfg(test)]` tests,
both in `crates/pyrs-yaml/src/py/direct_load.rs`, and they pinned one decision: whether a document
is inside the direct-load surface at all. That decision is a property of the document and the
grammar - two byte-level pre-vetoes and the event layer's own answer - and not of the Python object
the value becomes, so it now lives in the engine (`crates/pyrs-yaml-core/src/parser/yaml/direct.rs`)
with its two tests beside it. The binding crate keeps the materialization, which is the only part
that needs a Python object to exist.

Measured after the move:

- the binding crate's Rust test tier holds **0** tests (2 before);
- the engine crate's closure names 75 unique crates and **none** of them is a Python-runtime crate;
- the new module builds for `thumbv7em-none-eabi`
  (`cargo build --locked --no-default-features --target thumbv7em-none-eabi -p pyrs-yaml-core`), so
  the decision it holds is `no_std`-clean like the rest of the engine.

The closure table above re-measured on this tree with its own method (unique crate *names*) still
reads 75 for `pyrs-yaml-core`, at `27031d12` and at `018bcc00` alike - the version-level difference
that makes a raw `sort -u` report 76 is `granit-parser` appearing at two versions, one of which is
the compile-time-only 1.3.0 described above.

**What is not claimed.** The extension cdylib still links `python3.dll`, by definition: it is the
Python extension. The binding crate's package closure still carries the six Python-runtime crates.
And the binding crate's `ir_gate` bench harness uses `pyo3/auto-initialize`, so a build that selects
it starts CPython to measure instruction counts - the one Rust target in this repository that
genuinely needs an interpreter. That is what it measures, not a coupling to remove, and it is
independent of the CI legs: `scripts/ir_gate.py` turns the feature on deliberately, and nothing in
CI's default build set does.

## The matrix above is gated, not described (2026-10-21)

The section above records what the crate matrix is; nothing read it back. `pyrs-yaml-core` could
grow a `pyo3` edge, the extension could take a Rust test block back, and the symptom would arrive
later as a Rust leg that cannot start without an interpreter.
`scripts/check_rust_python_decoupling.py` derives the matrix from the manifests themselves - the
members come from the root manifest, the Python-runtime set from the `pyo3` prefix and the `numpy`
name, the test tier from the sources - and enforces five rules.

| rule | what it holds |
|:--|:--|
| R1 | exactly one crate produces a cdylib, no other crate's dependency closure carries a Python-runtime crate, and the extension's closure does |
| R2 | the extension holds no `#[cfg(test)]` block, no `tests/` directory and no `[[test]]` target, and declares `[lib] test = false` |
| R3 | no other crate disables its test tier and no other crate holds an empty one |
| R4 | every `required-features` entry names a declared feature that default does not enable |
| R5 | the extension crate holds no `[[bench]]` target without `required-features`, so a workspace bench command cannot build the interpreter probe through it |

R3 is what makes this a decoupling gate rather than a test-suppression gate: the target is the
absence of the coupling, not the absence of the tests, so a crate whose tests were deleted to make
it "clean" is a finding.

Two shape decisions carry the weight. The gate reads the manifests with a reader written for the
constructs Cargo actually uses, because `tomllib` is 3.11+ while this package supports 3.8, and
because the check has to run where cargo does not: the Python test legs install a wheel and never
build one, so a `cargo metadata` gate would have no CI-runnable falsification control. A manifest
the reader cannot parse is exit 2 - a refusal, never a pass. And the control itself,
`tests/test_rust_python_decoupling_gate.py`, breaks each rule on a synthetic tree and requires the
verdict to flip. Measured against the file as it ships: all 13 negative controls bite, and removing
any one of the 8 rule bodies blinds exactly the controls that body stands behind, no more and no
fewer - which is the only evidence that a passing control means anything.

**Where the coupling is actually felt, and what CI does about it.** The graph, not the binaries:
`cargo test --workspace --no-run` resolves 250 compilation units over 153 packages and names `pyo3`,
`pyo3-ffi`, `pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config` and `numpy`, while the same
command with `--exclude pyrs-yaml` resolves 220 units over 139 packages and names none of them. Both
run the same 748 Rust tests, and `cargo test -p pyrs-yaml --doc` reports 0, so the exclusion gives
up nothing - which is a licence, not a proof, and the proof is the pairing
`tests/test_ci_rust_leg_exclusion.py` enforces: an engine leg that stops excluding the crate the
gate names as the extension reddens, and so does the MSRV leg if it ever starts, because that job's
purpose is the declared floor for every crate, the shipped one included.

**What this gate does not see.** The closure follows the workspace's `path` edges, so a
Python-runtime crate reached only through a registry dependency's own dependency tree is outside its
view; the direct `pyo3`/`numpy` edges it does see are the ones this repository controls. And R2
covers the Rust test tier alone: the extension's `ir_gate` harness needs an interpreter when a build
selects it, because measuring `to_python_*` instruction counts is the point of that target.

**The CI Rust bench leg builds without an interpreter too (2026-10-21).** The last leg that still
pulled the runtime was CodSpeed's `Rust benchmarks`, and the reason was the same as the test legs -
workspace *membership*, not a dependency edge: `cargo codspeed build` selects every member, so it
compiled the `pyrs-yaml` cdylib and its `pyo3-build-config` build script, which probes for an
interpreter, even though neither `yaml_bench` (now in `pyrs-yaml-core`) nor `cli_bench` (in
`pyrs-yaml-cli`) depends on the binding - `cargo tree -i pyrs-yaml` names no workspace member. The
leg now runs `cargo codspeed build --workspace --exclude pyrs-yaml`, the same exclusion the CI test
legs use. Measured on this tree: the scoped build selects `yaml_bench` and `cli_bench` and compiles
zero `pyo3`/`numpy`, and the job drops its `setup-python` step because nothing in it probes for an
interpreter anymore. The `ir_gate` target is unaffected - it is gated on `ir-gate`, was never a
CodSpeed benchmark, and is built only by `scripts/ir_gate.py`.

## The mmap read path is a gated feature, not a default (2026-10-22)

`pyrs-yaml-core` gained an `mmap` feature that maps a regular file with `memmap2` and parses
straight from the page cache, dropping the `read_to_string` heap copy from the read path; the
`Arc<str>` an editable document keeps for round-trip is still allocated, because a document's
lifetime must not ride on a mapping of a file the caller can truncate from under it (a truncated map
faults `SIGBUS`). That caveat is the whole reason it is a feature and not the default, the same
backward-compatible shape the `std` split took: `default = ["numpy"]` is unchanged, so the wheel
ships the read path it always had and only an opt-in rebuild turns the map on.

The gate is the seam's behaviour, not its speed. `read_file_text` maps only a regular, non-empty
file and falls back to `std::fs::read_to_string` for an empty file, a directory, or a non-regular
path, so those keep the IO error bytes Python already saw, and a non-UTF-8 mapped file is rejected
with the same `InvalidData` message — `crates/pyrs-yaml-core/src/file.rs` asserts the byte-for-byte
match and every error-parity case. The `no-std-check` job now builds the crate `--features mmap` so
a default-off feature cannot rot the way the no_std crate list once did silently; because `mmap`
implies `std`, the `thumbv7em-none-eabi` build never enables it and the no_std foundation is
unaffected. A measured instruction-count figure belongs in `perf.md` only once taken on the
enforcing image; none is claimed here.
