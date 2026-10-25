//! Instruction-count gate: the same hot paths as the divan suite, but measured
//! in *counted instructions* instead of wall time.
//!
//! # Why a second harness exists
//!
//! The divan benchmarks report through CodSpeed, whose numbers compare a run on
//! today's runner against a baseline recorded on a different machine. For
//! sub-10% effects that comparison is not reproducible: while landing a fidelity
//! change, three consecutive pushes — each doing *strictly less work* than the
//! previous one — reported −7.7%, −10.5% and −9.8% for the same benchmark set.
//! `callgrind` instruction counts, by contrast, repeat to within 0.004% on the
//! same binary, so a committed baseline can gate them with a real tolerance and
//! say something the wall-time number cannot. The tolerance is not 0.1% either:
//! instruction totals include the dynamic loader and malloc of the host image, so
//! the same code measured +1.45% apart between two Linux images — see
//! `DEFAULT_TOLERANCE` in `scripts/ir_gate.py`.
//!
//! # Method
//!
//! Each scenario runs in two modes from the same binary:
//!
//! - `--setup-only` performs everything the full run does *except* the measured
//!   loop (for serialize scenarios that includes the parse that builds the AST,
//!   exactly as `yaml_bench.rs` keeps setup outside `bencher.bench`);
//! - the default mode adds `ITERATIONS` calls.
//!
//! Subtracting the first from the second leaves the loop alone. That matters
//! because callgrind counts the whole process: startup, the test framework and
//! the setup phase would otherwise swamp a few percent of change. Name-based
//! filtering (`--toggle-collect` etc.) is not usable here because `lto = true`
//! inlines these functions away, leaving no symbol to match.
//!
//! Iteration counts are fixed, never auto-tuned: a harness that adapts its own
//! sample count to elapsed time would make the instruction total depend on the
//! machine, which is the exact property this gate exists to remove.
//!
//! Run through `scripts/ir_gate.py`, which does the subtraction, compares against
//! `.ci/ir-baseline.json` and prints the table CI gates on.
use pyrs_json::{
    from_json, from_json5, from_jsonc, parse_documents, to_json_text, to_json5_text, to_jsonc_text,
};
use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::bench_inputs::{
    ANCHOR_YAML, BLOCK_SCALAR_YAML, BLOCK_STYLE_YAML, MEDIUM_JSON, MEDIUM_JSON5, MEDIUM_JSONC,
    MEDIUM_TOML, MEDIUM_YAML, MERGE_INLINE_YAML, SMALL_YAML, long_string_json, long_string_json5,
    long_string_jsonc, long_string_ndjson, long_string_toml, long_string_yaml, short_string_ndjson,
};
use pyrs_yaml_core::editing::navigate::Segment;
use pyrs_yaml_core::editing::plan::{delete_path, insert_path, set_path};
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::parser::{parse, parse_stream};
use pyrs_yaml_core::serializer::to_yaml;
use std::borrow::Cow;

/// Fixed for every run and every machine.
const ITERATIONS: u32 = 2_000;

/// The long-string pair, generated once for the whole harness: `scenarios()` carries
/// `&'static str`, and these are the same bytes `yaml_bench` measures - one fixture, two
/// harnesses, the standing rule.
static LONG_JSON: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_json);
static LONG_TOML: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_toml);
static LONG_YAML: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_yaml);
static LONG_JSONC: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_jsonc);
static LONG_JSON5: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_json5);
static LONG_NDJSON: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_ndjson);
static SHORT_NDJSON: std::sync::LazyLock<String> = std::sync::LazyLock::new(short_string_ndjson);

#[derive(Copy, Clone, PartialEq, Eq)]
enum Work {
    Parse,
    Serialize,
    /// The hub AST rendered as JSON: the path `YamlDocument.to_json()` takes, and the
    /// one the instruction gate had no line on until now.
    ToJson,
    /// The hub AST rendered as TOML, same reasoning.
    ToToml,
    /// TOML text read into the hub AST: the path `load_toml()` takes. The two writers
    /// above measure the outbound half of each bridge only. The inbound half is where a
    /// key's *style* is decided - a TOML or JSON key is a string by its own grammar, and
    /// the shared AST marks that with quoting - so per-key work added there is invisible
    /// to every scenario that exists today.
    FromToml,
    /// JSON text read into the hub AST, same reasoning as [`Work::FromToml`].
    FromJson,
    /// The two JSON dialects the hub also exposes - `load_jsonc`/`from_jsonc` and their JSON5
    /// siblings. Their reading half skips comments and, for JSON5, accepts a key grammar strict JSON
    /// rejects, none of which any existing scenario entered: the gate had no instruction number for
    /// the comment scanner at all, so a change to it could not be adjudicated.
    FromJsonc,
    /// JSON5 read into the hub AST: comment skipping plus the unquoted-key, single-quote and
    /// trailing-comma grammar.
    FromJson5,
    /// The hub AST rendered as JSONC, the writer half of the same two bridges. `to_jsonc_text` and
    /// `to_json5_text` decide how to spell a key or a note per node, and both were unmeasured.
    ToJsonc,
    /// The hub AST rendered as JSON5.
    ToJson5,
    /// The event stream path: text read into `Vec<StreamEvent>` instead of an AST. The
    /// vocabulary's scalar text was owned `String` at the time this scenario was added, and the
    /// maintainer ruled it should borrow (`Cow<'a, str>`, owning only what an escape decoded).
    /// That is a claim about allocations on a path no other scenario enters - `parse_*` builds
    /// nodes, the bridges build nodes too - so the ruling needed a number of its own before and
    /// after, not an argument. Nothing measured `parse_stream` until now.
    Stream,
    /// The line-delimited surface: one document per line, read into `Vec<StreamEvent>` by
    /// `parse_documents`. It divides the text at its own line table and restarts the scanner per
    /// record, so neither the AST scenarios (which build nodes from one document) nor `Stream`
    /// (which hands one document to granit) enters this route. The fixture pair prices the
    /// record's body length here, which is what a string scan is asked about.
    Documents,
    /// The JSON event producer - the tree-free walk `pyrs_json::parse_stream` runs, which is a
    /// different route through the same bytes the `from_json_*` scenarios take (those build the node
    /// model the producer never allocates) and had no number of its own. `Stream` above is YAML's
    /// path over granit; nothing here entered the producer that ruling B made sink-driven.
    StreamJson,
    /// The cancellation half of that producer: a sink that answers `ControlFlow::Break` a quarter of
    /// the way in. `StreamJson` cannot see a regression that only shows up on the stop - the check
    /// that ends the walk, the state dropped on the way out, an event built that nobody asked for -
    /// and the stop is what `parse_stream(..., on_event=...)` sells. Paired with `StreamJson` so a
    /// red reading can be told apart: prefix-only, or everything.
    StreamJsonStop,
    /// The edit layer: the third of the three expensive pure-Rust computations `AGENTS.md` names
    /// beside `parse` and AST serialize, and the only one of the three with no number here. A
    /// regression in path navigation, in the dirty-plan build, or in the way a note or anchor is
    /// carried across a moved node was free to land while every floor above stayed green. One
    /// scenario per verb, so a red reading says which verb got more expensive - `move_path` is a
    /// `set_path` plus a `delete_path` and needs no floor of its own.
    EditSet,
    /// `insert_path` into a block sequence: the verb that writes a new position into the text
    /// instead of replacing a node that already had one.
    EditInsert,
    /// `delete_path` of a mapping key: the verb that must not strand an alias on an anchor defined
    /// inside the subtree it removes, which is the invariant `plan.rs` checks at the drop site.
    EditDelete,
}

/// Every scenario the gate measures. Names mirror `yaml_bench.rs` where an
/// equivalent divan benchmark exists, so the two reports can be read side by
/// side; `parse_*` have no divan twin of the same size and are gate-only.
fn scenarios() -> Vec<(&'static str, &'static str, Work)> {
    vec![
        ("serialize_small", SMALL_YAML, Work::Serialize),
        ("serialize_medium", MEDIUM_YAML, Work::Serialize),
        ("serialize_block", BLOCK_STYLE_YAML, Work::Serialize),
        (
            "serialize_block_scalars",
            BLOCK_SCALAR_YAML,
            Work::Serialize,
        ),
        ("serialize_anchors", ANCHOR_YAML, Work::Serialize),
        ("parse_small", SMALL_YAML, Work::Parse),
        ("parse_medium", MEDIUM_YAML, Work::Parse),
        ("parse_block", BLOCK_STYLE_YAML, Work::Parse),
        ("parse_anchors", ANCHOR_YAML, Work::Parse),
        // Merge sources written inline instead of aliased: the folding path the
        // anchor scenario never enters.
        ("parse_inline_merge", MERGE_INLINE_YAML, Work::Parse),
        // The two cross-format writers the binding exposes. `MEDIUM_YAML` is the input
        // both can represent - plain scalars, nested mappings, a block sequence, no
        // null and no alias - so neither scenario measures a failure path.
        ("to_json_medium", MEDIUM_YAML, Work::ToJson),
        ("to_toml_medium", MEDIUM_YAML, Work::ToToml),
        // The same two bridges read back, from committed bytes rather than bytes
        // rendered here. `tests/ir_fixtures.rs` requires the literals to equal what the
        // writers emit today, so a change in how the writers *spell* output re-derives
        // them on purpose instead of quietly moving what the reader is measured on.
        //
        // Rendering them here was tried first, on the theory that a second call site in
        // setup would shift the writer scenarios; the measurement refuted that theory.
        // `to_json_medium` came out +1.52% against `main` with the call and +1.489% with
        // the bytes committed - the movement tracks the harness binary changing shape, not
        // how the fixture got there, and `serialize_*` / `parse_*` in that same binary
        // stayed inside 0.042%. Unresolved, and stated rather than smoothed over: a
        // re-baseline that follows a harness edit mixes method with code, so the change
        // that moves a number for a reason the library did not do has to say so.
        ("from_json_medium", MEDIUM_JSON, Work::FromJson),
        ("from_toml_medium", MEDIUM_TOML, Work::FromToml),
        // The dialects, both halves. The reader fixtures are authored bytes rather than writer
        // output on purpose - a fixture derived from `to_jsonc_text` of a comment-free AST would
        // contain no comments, and the comment scanner is the path being measured. `tests/ir_fixtures.rs`
        // pins that they still parse and still mean `MEDIUM_JSON`.
        ("from_jsonc_medium", MEDIUM_JSONC, Work::FromJsonc),
        ("from_json5_medium", MEDIUM_JSON5, Work::FromJson5),
        // The long-value shape for every reader this release moved: strict JSON, TOML, JSONC and
        // JSON5. `MEDIUM_JSON`'s longest value is 21 characters, which never leaves the first
        // sixteen-byte vector, so the search that was vectorized is nearly invisible there: the
        // medium floors prove the change did not make the short path do more work, these prove the
        // long path does less. Each document differs from its medium twin in string length (and, for
        // the two dialects, in the comments and quoting their own grammar needs) and in nothing else.
        ("from_json_long_strings", LONG_JSON.as_str(), Work::FromJson),
        ("from_toml_long_strings", LONG_TOML.as_str(), Work::FromToml),
        (
            "from_jsonc_long_strings",
            LONG_JSONC.as_str(),
            Work::FromJsonc,
        ),
        // JSON5 is the dialect whose string body is scanned by the single-quote instantiation of the
        // vector, so its long shape is a different code path from the three above, not a spelling.
        (
            "from_json5_long_strings",
            LONG_JSON5.as_str(),
            Work::FromJson5,
        ),
        // The outbound halves of the same four bridges, over the same records: the gate pairs a
        // bridge's reader with its writer and reports a one-sided pair as a blind spot, and the
        // writers are where a long value is asked whether it needs quoting at all. These four are
        // also the floors the writer-side scan will be priced against before anyone vectorizes it.
        ("to_json_long_strings", LONG_YAML.as_str(), Work::ToJson),
        ("to_toml_long_strings", LONG_YAML.as_str(), Work::ToToml),
        ("to_jsonc_long_strings", LONG_YAML.as_str(), Work::ToJsonc),
        ("to_json5_long_strings", LONG_YAML.as_str(), Work::ToJson5),
        ("to_jsonc_medium", MEDIUM_YAML, Work::ToJsonc),
        ("to_json5_medium", MEDIUM_YAML, Work::ToJson5),
        // The event path over the same fixture the AST scenarios read, so the two are comparable:
        // `parse_medium` builds nodes here, and whatever this reports is the price of the *other*
        // shape the same text can take. The ratio is the claim; the instruction counts behind it move
        // with every refresh, so none is quoted in a comment nothing reads.
        ("stream_medium", MEDIUM_YAML, Work::Stream),
        // The event path over the long-value shape. `stream_medium`'s longest value is 21
        // characters, so the stream never leaves the first sixteen-byte vector and a regression
        // that only shows up on a long body is invisible to it - the same reason the four
        // `*_long_strings` readers exist beside `from_*_medium`. One scenario, not four: the
        // stream's dialect is chosen by the caller, and the path being priced is granit's scalar
        // handoff, which the YAML fixture already enters.
        ("stream_long_strings", LONG_YAML.as_str(), Work::Stream),
        // The line-delimited pair, one scenario per side. A short value keeps the record inside the
        // first vector of the string scan, so the only difference between these two numbers is what
        // a long body costs the scanner per record - and the per-record restart is the part neither
        // `from_json_long_strings` (one document, one scan) nor `stream_*` (granit's scalar handoff)
        // measures. The pair is what makes each one readable on its own.
        (
            "documents_short_strings",
            SHORT_NDJSON.as_str(),
            Work::Documents,
        ),
        (
            "documents_long_strings",
            LONG_NDJSON.as_str(),
            Work::Documents,
        ),
        // The JSON producer, whole and stopped, over the fixture the AST scenarios already read, so
        // the difference between `stream_json_medium` and `from_json_medium` is the price of the node
        // model the producer never builds. No number is quoted here: a floor written into a comment
        // goes stale at the next refresh and nothing reads it, which is the defect `(gh)` records.
        // The stop is a quarter of the 49 events this document emits - twelve delivered, thirty-seven
        // never scanned - measured rather than assumed, and pinned by
        // `crates/pyrs-yaml-core/tests/ir_fixtures.rs::the_json_stream_stop_leaves_a_tail_to_skip_and_skips_it`.
        ("stream_json_medium", MEDIUM_JSON, Work::StreamJson),
        ("stream_json_stop", MEDIUM_JSON, Work::StreamJsonStop),
        // The three edit verbs over `MEDIUM_YAML`, one scenario each, so a red reading names the verb
        // that got more expensive. The arm explains what is inside and outside the measured span;
        // `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins that each path below still resolves and
        // that each verb still changes the emitted text.
        //
        // Named as gate-only, beside `parse_*`, rather than as mirrors of the divan twins: divan's
        // `edit_set_medium` replaces the whole `database` section with a scalar, which is a different
        // edit from the nested key this prices, and its insert/delete twins run on `LARGE_YAML`. A
        // shared name over different work would make the two reports read as one measurement.
        ("edit_set_nested", MEDIUM_YAML, Work::EditSet),
        ("edit_insert_sequence", MEDIUM_YAML, Work::EditInsert),
        ("edit_delete_key", MEDIUM_YAML, Work::EditDelete),
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        for (name, _, _) in scenarios() {
            println!("{name}");
        }
        return;
    }
    let Some(name) = args.first() else {
        eprintln!("usage: ir_gate <scenario> [--setup-only] | --list");
        std::process::exit(2);
    };
    let setup_only = args.iter().any(|a| a == "--setup-only");
    let Some((_, src, work)) = scenarios().into_iter().find(|(n, _, _)| n == name) else {
        eprintln!("unknown scenario {name:?}");
        std::process::exit(2);
    };

    let measured = match work {
        Work::Serialize | Work::ToJson | Work::ToToml | Work::ToJsonc | Work::ToJson5 => {
            // Setup is outside the loop on both modes, so it cancels exactly.
            let ast = parse(src, Schema::Core).expect("setup parse");
            if setup_only {
                std::hint::black_box(&ast);
                0
            } else {
                let mut acc = 0usize;
                for _ in 0..ITERATIONS {
                    let text = match work {
                        Work::Serialize => to_yaml(std::hint::black_box(&ast)),
                        Work::ToJson => {
                            std::hint::black_box(to_json_text(std::hint::black_box(&ast)))
                                .expect("json writes the hub AST")
                        }
                        Work::ToToml => std::hint::black_box(to_toml(std::hint::black_box(&ast)))
                            .expect("toml writes the hub AST"),
                        Work::ToJsonc => {
                            std::hint::black_box(to_jsonc_text(std::hint::black_box(&ast)))
                                .expect("jsonc writes the hub AST")
                        }
                        Work::ToJson5 => {
                            std::hint::black_box(to_json5_text(std::hint::black_box(&ast)))
                                .expect("json5 writes the hub AST")
                        }
                        _ => unreachable!("handled by the other arms"),
                    };
                    acc += std::hint::black_box(text).len();
                }
                acc
            }
        }
        Work::FromJson | Work::FromToml | Work::FromJsonc | Work::FromJson5 => {
            if setup_only {
                std::hint::black_box(src);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let parsed = match work {
                        Work::FromToml => from_toml(std::hint::black_box(src)),
                        Work::FromJsonc => from_jsonc(std::hint::black_box(src)),
                        Work::FromJson5 => from_json5(std::hint::black_box(src)),
                        _ => from_json(std::hint::black_box(src)),
                    };
                    // A reader that started failing would still loop, still return a
                    // number, and that number would be read as a speedup. Same rule the
                    // binding harness prints at exit 3: a harness that measured nothing
                    // does not get to report a result.
                    let node = parsed.unwrap_or_else(|error| {
                        eprintln!(
                            "{name} iteration {index}: {error} - the fixture no longer reaches \
                             the path being measured"
                        );
                        std::process::exit(3);
                    });
                    acc += usize::from(std::hint::black_box(&node).comment().is_some());
                }
                acc
            }
        }
        Work::Parse => {
            if setup_only {
                std::hint::black_box(src);
                0
            } else {
                let mut acc = 0usize;
                for _ in 0..ITERATIONS {
                    let node = parse(std::hint::black_box(src), Schema::Core).expect("parse");
                    // Any stable, cheap read works: it only has to keep the AST
                    // alive past the black_box so the parse cannot be elided.
                    // Deliberately not a newer accessor, so the gate keeps
                    // compiling against whatever is on `main`.
                    acc += usize::from(std::hint::black_box(&node).comment().is_some());
                }
                acc
            }
        }
        Work::Stream => {
            if setup_only {
                std::hint::black_box(src);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let events = std::hint::black_box(parse_stream(std::hint::black_box(src)))
                        .unwrap_or_else(|error| {
                            // A stream that started failing would still loop and still report a
                            // smaller number, which this one-sided gate reads as a speedup.
                            eprintln!(
                                "{name} iteration {index}: {error} - the fixture no longer reaches \
                                 the path being measured"
                            );
                            std::process::exit(3);
                        });
                    // Structural count, not text length: it is what keeps every event alive to
                    // this point, so the receiver cannot be optimised out of existing.
                    acc += std::hint::black_box(&events).len();
                }
                acc
            }
        }
        Work::Documents => {
            if setup_only {
                std::hint::black_box(src);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let events = std::hint::black_box(parse_documents(std::hint::black_box(src)))
                        .unwrap_or_else(|error| {
                            // Same one-sided-gate rule as `Work::Stream` above: a stream that began
                            // failing would loop anyway and report a smaller number.
                            eprintln!(
                                "{name} iteration {index}: {error} - the fixture no longer reaches \
                                 the path being measured"
                            );
                            std::process::exit(3);
                        });
                    acc += std::hint::black_box(&events).len();
                }
                acc
            }
        }
        Work::StreamJson => {
            if setup_only {
                std::hint::black_box(src);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let events =
                        std::hint::black_box(pyrs_json::parse_stream(std::hint::black_box(src)))
                            .unwrap_or_else(|error| {
                                eprintln!(
                                    "{name} iteration {index}: {error} - the fixture no longer \
                                     reaches the path being measured"
                                );
                                std::process::exit(3);
                            });
                    // Structural count, as for `Work::Stream`: it is what keeps every event alive to
                    // this point, so the producer cannot be optimised out of existing.
                    acc += std::hint::black_box(&events).len();
                }
                acc
            }
        }
        Work::StreamJsonStop => {
            // The quarter is derived here rather than written down, and in setup on both modes, so
            // `--setup-only` subtracts the derivation and the loop prices only the walk to the stop
            // and the refusal to read on. A limit of zero would price the producer's two opening
            // events; a limit equal to the total would price the whole stream under a name that says
            // otherwise. Both are refusals, not readings, so they exit 3.
            let total = pyrs_json::parse_stream(src).unwrap_or_else(|error| {
                eprintln!("{name} setup: {error} - the fixture no longer streams");
                std::process::exit(3);
            });
            let total = std::hint::black_box(total).len();
            let limit = total / 4;
            if limit == 0 || limit >= total {
                eprintln!(
                    "{name}: this document streams {total} events, so a quarter asks for {limit} - \
                     either nothing or everything"
                );
                std::process::exit(3);
            }
            if setup_only {
                std::hint::black_box(limit);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let mut delivered = 0usize;
                    {
                        let mut sink = |event: pyrs_json::StreamEvent<'_>| {
                            // Every event handed out is read before the answer, so the stop cannot
                            // be reached by a producer that stopped building them.
                            std::hint::black_box(&event);
                            delivered += 1;
                            if delivered == limit {
                                std::ops::ControlFlow::Break(())
                            } else {
                                std::ops::ControlFlow::Continue(())
                            }
                        };
                        pyrs_json::parse_stream_with_options_to(
                            std::hint::black_box(src),
                            pyrs_json::JsonParseOptions::STRICT,
                            &mut sink,
                        )
                        .unwrap_or_else(|error| {
                            eprintln!(
                                "{name} iteration {index}: {error} - the fixture no longer reaches \
                                 the path being measured"
                            );
                            std::process::exit(3);
                        });
                    }
                    let delivered = std::hint::black_box(delivered);
                    if delivered != limit {
                        // Either `ControlFlow::Break` no longer ends the walk, or the tail this
                        // scenario exists to skip has gone. Both would report a full stream.
                        eprintln!(
                            "{name} iteration {index}: the sink saw {delivered} events where the \
                             stop asks for {limit} of {total}"
                        );
                        std::process::exit(3);
                    }
                    acc += delivered;
                }
                acc
            }
        }
        Work::EditSet | Work::EditInsert | Work::EditDelete => {
            // Setup outside the loop on both modes, as for the writers: the document is parsed once,
            // and only the verb is priced. The tree is cloned per iteration because an edit applied
            // twice to the same tree is not the same edit, and `--setup-only` subtracts that clone
            // exactly as it subtracts the parse, so what is left is the verb.
            let ast = parse(src, Schema::Core).expect("setup parse");
            if setup_only {
                std::hint::black_box(&ast);
                0
            } else {
                let mut acc = 0usize;
                for index in 0..ITERATIONS {
                    let mut tree = std::hint::black_box(ast.clone());
                    let unit = match work {
                        // `server.port` -> 9090: the replacement a user asks for most often, and the
                        // branch that carries the old node's metadata and anchor forward.
                        Work::EditSet => set_path(
                            &mut tree,
                            &[
                                Segment::Key(Cow::Borrowed("server")),
                                Segment::Key(Cow::Borrowed("port")),
                            ],
                            CustomNode::plain_scalar("9090"),
                            true,
                            src,
                            None,
                            false,
                        ),
                        // `logging.outputs` index 1 <- stderr: a block sequence, so the line has to be
                        // placed rather than substituted.
                        Work::EditInsert => insert_path(
                            &mut tree,
                            &[
                                Segment::Key(Cow::Borrowed("logging")),
                                Segment::Key(Cow::Borrowed("outputs")),
                            ],
                            1,
                            CustomNode::plain_scalar("stderr"),
                            src,
                            None,
                        ),
                        // `features.rate_limit` removed: a key of the last section, so the emit also
                        // decides what the document's trailing break becomes.
                        Work::EditDelete => delete_path(
                            &mut tree,
                            &[
                                Segment::Key(Cow::Borrowed("features")),
                                Segment::Key(Cow::Borrowed("rate_limit")),
                            ],
                            src,
                            None,
                        ),
                        _ => unreachable!("handled by the other arms"),
                    };
                    let unit = unit.unwrap_or_else(|error| {
                        // A script that stopped resolving would loop anyway and report a smaller
                        // number, which reads as a speedup: the same one-sided rule the readers apply.
                        eprintln!(
                            "{name} iteration {index}: {error} - the script no longer reaches the \
                             path being measured"
                        );
                        std::process::exit(3);
                    });
                    acc += usize::from(std::hint::black_box(&unit).eligible)
                        + usize::from(std::hint::black_box(&tree).comment().is_none());
                }
                acc
            }
        }
    };

    // Print so a human re-running the binary by hand sees it did something, and
    // so the result cannot be optimised out before the print.
    println!(
        "{} mode={} iterations={ITERATIONS} touched={measured}",
        name,
        if setup_only { "setup" } else { "full" }
    );
}
