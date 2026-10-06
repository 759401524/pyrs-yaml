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
//! same binary, so a committed baseline can gate them with a one-percent
//! tolerance and say something the wall-time number cannot.
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
use pyrs_yaml_core::bench_inputs::{
    ANCHOR_YAML, BLOCK_SCALAR_YAML, BLOCK_STYLE_YAML, MEDIUM_YAML, SMALL_YAML,
};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

/// Fixed for every run and every machine.
const ITERATIONS: u32 = 2_000;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Work {
    Parse,
    Serialize,
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
        Work::Serialize => {
            // Setup is outside the loop on both modes, so it cancels exactly.
            let ast = parse(src, Schema::Core).expect("setup parse");
            if setup_only {
                std::hint::black_box(&ast);
                0
            } else {
                let mut acc = 0usize;
                for _ in 0..ITERATIONS {
                    acc += std::hint::black_box(to_yaml(std::hint::black_box(&ast))).len();
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
    };

    // Print so a human re-running the binary by hand sees it did something, and
    // so the result cannot be optimised out before the print.
    println!(
        "{} mode={} iterations={ITERATIONS} touched={measured}",
        name,
        if setup_only { "setup" } else { "full" }
    );
}
