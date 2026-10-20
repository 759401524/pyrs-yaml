# JSON Schema evaluation in Rust — design for review

Status: **design for review, no implementation.** This page exists because the defence matrix
measures one empty cell and the honest reading of it turned out to be an engine question, not a
test question. Nothing here is built; the sequencing at the end is what a reviewer is being asked
to accept or reject.

## The measured fact

`scripts/quality_matrix.py --json` reports, per format, which tiers exist. Every row has an
`authority` cell filled except one:

| row | authority | fuzz round-trip | property (rust) | unit |
|:----|:----------|:----------------|:----------------|:-----|
| YAML | `test_yaml_suite.py` | `yaml_roundtrip` | 25 | 408 |
| TOML | `test_toml_test_suite.py` | `toml_roundtrip` | 2 | 88 |
| JSON / JSONC / JSON5 | `test_json_test_suite.py`, … | `json_roundtrip` | 6 | 81 |
| schema validation | *(empty)* | *(empty)* | 2 | 39 |

`QUALITY_MATRIX.md` already says why, and says it plainly: the row reads `no` "because nothing
outside asks its rules; that is the honest state of the row, and the one a future JSON Schema suite
would change."

The investigation that precedes this document establishes that the sentence understates the case:

- `Document.validate()` does not evaluate a schema. It imports the third-party Python package
  `jsonschema` and forwards (`crates/pyrs-yaml/src/py/document.rs:1133-1147`), and its cached path
  calls `jsonschema.validators.validator_for` and reproduces
  `exceptions.best_match(iter_errors(instance))` semantics (`:1149-1173`, dispatch at `:994-1012`).
  `jsonschema` is a **test-group** dependency (`pyproject.toml:90-91`), so the shipped base install
  answers `validate()` only when the caller has installed a package we do not own.
- `crates/pyrs-schema` is not a schema validator and is deliberately not one: its manifest states
  "zero-dependency and `no_std`: no AST, no parser, no i18n, no regex", and CI proves the property
  with the `no-std-check` gate. It resolves scalar *tags* for every format engine.
- Grep across that crate for `$ref`, `format`, `patternProperties`, `oneOf`, `enum`, `minimum`: zero
  hits. No draft is claimed anywhere.
- The in-house validator that does exist is a different language: `schema_language.rs` understands
  `rules` / `extends` / `validate`, `pattern` + `type`, `path` / `sequence_of` / `mapping_of` /
  `required` (`crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs:412-578`). Unrecognised keys
  are dropped by a `_ => {}` arm (`:427`, `:494`, `:579`).

So the empty cell cannot be filled by adding a test file. A suite run against `Document.validate()`
would certify `jsonschema`, not this codebase, and a green authority cell obtained that way would be
the most expensive kind of lie this matrix can tell: it would convert the one honestly-empty cell
into a claimed defence we do not have.

## What "no short board" would require

A JSON Schema evaluator owned by this repository, judged by the official suite
(`json-schema-org/JSON-Schema-Test-Suite`), with `jsonschema` demoted from *the implementation* to
*one more cross-library oracle* — the role the matrix's parity column already gives PyYAML, tomlkit,
orjson and pyjson5. That reading makes the row honest in two directions at once: authority asks what
the specification says, parity asks whether we agree with the de-facto reference.

## Decision 1 — a new crate, not a grown one

`pyrs-json-schema` (working name), not an expansion of `pyrs-schema`.

`pyrs-schema`'s identity is a zero-dependency scalar tag vocabulary that cross-compiles to bare
metal. A JSON Schema evaluator needs a schema document model, URI resolution for `$id`/`$ref`,
an instance path representation, and — for `pattern` — a regular-expression engine. Merging the two
would delete the sentence in the manifest that the `no-std-check` gate exists to protect, and would
tax every format engine that only needs tag resolution. This follows the crate-matrix pillar
(single responsibility, fine-grained dependency) and the naming decision already recorded: the
`pyrs-` prefix stays, no `rs-` rename.

The new crate is `alloc`-based with `std` on by default, and the keyword set that needs more
capability sits behind features, so a bare-metal consumer can still link it:

| feature | supplies | cost when off |
|:--------|:---------|:--------------|
| `pattern` | regex evaluation for `pattern`, `patternProperties` | those keywords refuse with a typed error, not silence |
| `format` | the `format` vocabulary as assertion | `format` is annotation-only, which is the draft 2020-12 default anyway |
| `content` | `contentEncoding`, `contentMediaType` | refused with a typed error |

The regex choice is the one real dependency question in this design and it needs the reviewer's
ruling, not a default: (a) `regex` — fast, proven, but `std`-only and a new tree dependency; (b) an
in-house ECMA-262-subset backtracker over `alloc`, bounded by an explicit step budget, which keeps
the no_std story and puts the correctness burden on us; (c) ship `pattern` in stage 2 after the rest
of the core evaluates. Recommendation: (c), then (a) for the `std` path — a validator whose
`pattern` silently disagrees with the specification is worse than one that refuses the keyword and
says so. The precedent for refusing rather than guessing is `schema_language.rs`'s silent drop,
which this design treats as the failure mode to avoid.

## Decision 2 — scope, stated as a keyword table, not a draft number

Draft 2020-12 core, with each keyword in one of four states, and the table itself in the crate's
`lib.rs` doc so it is reviewable as code:

- **stage 1 (must):** `type` (incl. the array-of-types form), `enum`, `const`, `anyOf`, `allOf`,
  `oneOf`, `not`, `if`/`then`/`else`, `properties`, `additionalProperties`, `propertyNames`,
  `required`, `dependentSchemas`, `dependentRequired`, `items`/`prefixItems`/`contains`/
  `minContains`/`maxContains`, `minProperties`/`maxProperties`, `minimum`/`maximum`/
  `exclusiveMinimum`/`exclusiveMaximum`/`multipleOf`, `minLength`/`maxLength`/`pattern`(refused),
  `minItems`/`maxItems`/`uniqueItems`, `$defs` + `$ref`, `$id` (base URIs only), `$anchor`,
  `$schema`, `$vocabulary` (declared-and-honoured or refused, never ignored).
- **stage 2:** `pattern`, `format` (optional assertion), `$dynamicRef`/`$dynamicAnchor`,
  `$recursiveRef` (2019-09 compat only), remote `$ref` over an injected, non-networked resolver.
- **refused with a typed error:** anything requiring an authority we do not ship — `contentSchema`
  dereferencing, unknown `$vocabulary` with `required: true`.
- **annotation, succeeds:** every unrecognised keyword, and known keywords in annotation position.
  The specification says an unknown keyword is not an error; the suite tests that, and a validator
  that treats it as one fails cases that are correct by design.

Numbers to hold the design honest: stage 1 covers the keyword families in roughly 80% of the suite's
case files by count; the measured claim belongs in the stage-1 PR, computed from the suite itself,
not asserted here.

## Decision 3 — the error surface is a public API change, so it is Ask First

Today: one message. `YamlValidateError::new_err(msg)` from a single best-match exception
(`document.rs:1121-1130`). The suite cannot be driven without more, and neither can anyone using
this as a CI gate: "the first error the reference implementation happened to pick" is not a report.

Proposed shape, additive, with the current behaviour untouched:

- `Document.validate()` keeps its signature, keeps raising the first error, and keeps the message
  text the existing tests pin. Its *source* changes when the Rust engine replaces the delegation —
  which is why the replacement is a version bump with a compatibility note, not a silent flip.
- New: `Document.validate_all(schema) -> list[SchemaViolation]`, where a violation carries
  `instance_location` (JSON Pointer), `keyword_location`, `keyword`, `message`, and the failing
  `schema_value` only when it is cheap. Ordering is defined (schema traversal order), because a
  list whose order is undefined cannot be asserted in a test.
- Rust side: `validate_node` already returns `Result<(), Vec<SchemaValidationError>>` in the YAML
  language path, so a Vec-of-errors surface is not a new idea in this tree.
- Consequences that must move together, per this repo's own gates: regenerate the stub
  (`maturin generate-stubs` + `scripts/check_stub_drift.py`, never a hand edit), add the new type to
  `EXPECTED_*` tables if the generator omits it, and document in four locales.

The reviewer is asked to rule on the name (`validate_all` vs an `errors=` keyword) and on whether
`SchemaViolation` is a dataclass-shaped PyClass or a dict; both are public-surface commitments.

## Decision 4 — `$ref` without a network

`$ref` is where validators acquire remote fetches, and a library that silently reaches the network
from `validate()` is a supply-chain and CI hazard. Rules: local resolution only (`#/$defs/…`,
`$anchor`, document-relative `$id`); a resolver is injectable from Rust and unavailable from Python
until the surface is designed; any absolute `http(s)` ref that is not pre-registered refuses with a
typed error naming the ref. Depth and recursion are bounded by the existing `max_depth` discipline
that already protects alias/merge cycles, so a self-referential schema cannot hang a call — and the
case that proves it belongs to the stage-1 tests, because a bound nobody exercised is a guess.

## Decision 5 — the tiers, wired the way this repo already wires them

1. **Authority.** `tests/test_schema_suite.py` — the name matters: `quality_matrix.py` matches the
   row by filename fragment and `AUTHORITY_SUFFIXES = ("_suite.py", "_spec_table.py")` (`:226`,
   `:282-286`), and `test_json_schema_suite.py` would land in the JSON row instead. The corpus is an
   untracked `Reference/` checkout like the other three; the clone is a **fourth step in
   `.github/actions/fetch-corpora/action.yml`**, not a new block in `ci.yml`, because
   `tests/test_ci_corpus_gates.py:27` derives the requirement by scanning `tests/*.py` for
   `Reference/<name>` and fails if the step is missing. Missing corpus → skip with the reason string
   the other suites use.
2. **No percentage floor.** `test_toml_test_suite.py`'s own history records why: a `0.85` floor was
   "22 cases of slack… whole defect classes hid behind a green percentage", and it now asserts
   `rate == 1.0` plus an empty failure list. The schema tier follows the JSON pattern instead:
   **named exclusion sets asserted in both directions** — a case that starts passing reddens the run
   as loudly as one that starts failing — plus exact counts for the corpus shape (files, groups,
   cases) so a silently truncated checkout cannot pass.
3. **Cross-library parity.** `jsonschema` stays a test-group dep and becomes the parity oracle,
   joining the existing column rather than answering for us.
4. **Property tier.** Generated (schema, instance) pairs with two invariants: an accepted instance
   makes every weaker bound hold (monotonicity on numeric/length keywords), and the error list is
   non-empty exactly when the boolean answer is "invalid". A property that only asserts the boolean
   is a re-run of the unit tests with more steps.
5. **Fuzz.** A `json_schema_eval` target in `fuzz/fuzz_targets/`, which must move
   `fuzz/Cargo.toml`'s `[[bin]]` **and** the `fuzz.yml` matrix together —
   `tests/test_quality_matrix.py` reddens on a half-move — with a typed-refusal-is-a-pass oracle.
6. **Performance floor.** Validation walks two trees, so it is a hot path with a price: new
   scenarios in the bench harness (both harnesses read `pyrs_yaml_core::bench_inputs`, never a
   second copy of a fixture), committed rows in `.ci/ir-baseline.json` produced by the enforcing
   image, and a measured `validate()` number before/after the delegation flip — the delegation is
   Python, so the Rust engine is expected to *win*, and the claim must be a command output, not a
   hope.

## Sequencing, each stage judged by its own output

| stage | ships | pass condition |
|:------|:------|:---------------|
| 1 | crate skeleton, keyword table, stage-1 keywords, Vec-of-errors, stub + locales | unit + property tiers green; `pattern` refuses by name; `Document.validate()` unchanged |
| 2 | `$ref`/`$defs`/`$anchor`/`$id`, bounded recursion | the suite runs locally, exclusion sets named and two-way asserted |
| 3 | CI corpus step, `tests/test_schema_suite.py`, matrix row flip | `no-std-check`, `test_ci_corpus_gates.py`, `test_quality_matrix.py`, `QUALITY_MATRIX.md` all consistent in one commit |
| 4 | `pattern`, optional `format`, `$dynamicRef` | each with its own exclusion-set shrink |
| 5 | default flip: Rust engine answers `validate()`, `jsonschema` becomes oracle | measured parity + Ir numbers + a compat note in five changelog mirrors |

Stage 1 is where a reviewer's "no" is cheapest, and it is the only stage that adds a crate.

## Explicitly not claimed here

- That this is worth doing before the two open rulings this project already carries (#113 note
  ownership, #109 YAML 1.1 numeric families) — those are user rulings and unchanged by this page.
- Any coverage number. Every count in this document is stated as something a later PR must measure.
- A date. The engine is new code with a specification-behaviour surface; the suite is the schedule.
- That `jsonschema` is being removed. It stays as a dependency of the *test* tree, which is what
  makes the parity column possible.
