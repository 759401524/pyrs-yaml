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

- **Table-header inline comment (`[sec] # note`).** The shared YAML engine cannot keep a comment on
  a container's key line (verified independently: pure YAML `sec: # note` loses that position too),
  so the note cannot stay where it was written. It is no longer *dropped* — the leading-comment list
  keeps the text — and since the container-inline-note fix it no longer leaves the table either: the
  note rides the header's own mapping, and a container's inline note is written on the last line of
  that container's body, which `to_toml` renders as a trailing comment on the table's last
  `key = value` line (`a = 1` + `[sec] # note` + `x = 1` → `a = 1` + `[sec]` + `x = 1 # note`),
  already stable after one round. What remains unfaithful is the position — not the existence of the
  comment, and not which table it belongs to; pinned by
  `TestSectionHeaderCommentBoundary.test_header_inline_comment_is_relocated_not_dropped` and
  `test_relocated_note_lands_on_the_tables_last_entry`, which assert survival, the in-table slot,
  and the one-round fixed point. Root-fixing the position needs a change to the locked granit
  comment-capture model — high blast radius across all YAML comment output and the 99.75 %
  compliance guarantee — so it is explicitly declined rather than silently shipped.
  Standalone/leading comments above a header and trailing comments on a leaf `key = value` line
  *are* preserved in place.
- **Binary integer source (`0b1010`).** Canonicalised to decimal on the round trip because YAML Core
  has no `0b` spelling (a faithful `0b` in YAML would re-resolve to a string, corrupting the value).
  Hex/octal source *is* preserved (YAML Core resolves them back to the same integer). Recorded in
  `toml/parser.rs` (`parse_prefixed_body_via_dispatch`) as a deliberate choice, not a defect.

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
- **Explicit-key markers.** A note that trailed a marker-only line is reported one level shallower
  than it was recorded, so the writer bubbles it up the first-pair-key spine
  (`former-crash-ac5d9043.seed`, minimised 8 bytes); a note riding the entry *after* a marker needed
  granit's span backed over the line break it swallows (`former-crash-{0e1c4378,105de752}.seed`); a
  container's own inline note has to borrow the last value line that can host one
  (`former-crash-11ced252.seed`, 13 bytes); the two lifts a marker line performs have to compose
  (`former-crash-456176be.seed`); the whole spine, not its first stack, has to be taken
  (`former-crash-{f8525a9e,c9031de4}.seed`); and every stack clears a tag header in one round
  (`former-crash-{e6551c75,fbc8f2ae,8f7085b0,77a8039b}.seed`).
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
