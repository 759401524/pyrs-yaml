//! Instruction-count gate, binding side: the AST-to-Python conversion.
//!
//! # Why this target exists
//!
//! `crates/pyrs-yaml-core/benches/ir_gate.rs` is the repository's only reproducible performance
//! instrument - two GitHub runner images measure the same commit to within 0.0018%, where the
//! wall-time channel has been recorded swinging -7.7%, -10.5% and -9.8% across three pushes that each
//! did strictly *less* work. But that harness lives inside the engine crate and never links
//! `pyrs-yaml`, so the layer every user actually reaches - `safe_load` turning a parsed document into
//! Python objects - had no reproducible number at all. `quality_matrix.py` derived the gap and
//! registered it as `perf-coverage:binding-layer`; this file is the exit criterion it names.
//!
//! It is the second instrument, not a copy of the first: the same scenarios on the same input bytes,
//! measured one layer higher. Where the engine harness says "parsing and serializing got 3% cheaper",
//! this one can say whether the object materialisation moved with it - which is the question PR #292
//! could not be answered with, and why two machines disagreed by "19% vs -10.5%" about the same patch.
//!
//! # Method
//!
//! Identical to the engine harness, for the same reasons:
//!
//! - `--setup-only` performs everything the full run does except the measured loop, including
//!   interpreter start-up. Subtracting it leaves the loop alone, which is what makes the number
//!   comparable across machines: callgrind counts the whole process, and CPython's own import and
//!   allocator warm-up would otherwise dominate a few percent of change.
//! - the iteration count is fixed, never auto-tuned, so the total does not depend on how fast the host
//!   happens to be.
//! - name-based callgrind filtering is unavailable (`lto = true` inlines the seam away), so the
//!   subtraction is the only way to isolate the work.
//!
//! The measured call deliberately skips the P3 direct-load fast path that `safe_load` prefers for
//! plain documents: the quantity of interest is the AST-to-Python conversion, and an anchor-free
//! fixture would otherwise report a shortcut while a tag- or anchor-bearing one reported the slow path
//! - two different things under one name. See `bench_to_python` in `src/py/functions.rs`.
//!
//! Each iteration must convert one document or the binary exits 3. The seam returns its error rather
//! than defaulting it, because the gate's tolerance only punishes growth: a harness that stopped doing
//! the work would report a huge improvement and pass.
//!
//! Run through `scripts/ir_gate.py`, which builds both harnesses, does the subtraction per scenario,
//! compares against `.ci/ir-baseline.json` and prints the table CI gates on.
//!
//! # Where it runs
//!
//! Unlike the engine harness, this binary links CPython: it needs an interpreter at *run* time, not
//! only at build time, which is why the `ir-gate` feature turns on `pyo3/auto-initialize` - the
//! shipped wheel runs inside an interpreter that is already up, this one has to start it. Measured:
//! the first refresh run built, listed its scenarios, and then failed in the measured loop with "The
//! Python interpreter is not initialized and the `auto-initialize` feature is not enabled", which is
//! also how `--list` succeeding and the measurement failing stay distinguishable.
//!
//! Measured on Windows - `cargo bench --no-run` succeeds and the built binary then dies at start-up
//! with `0xC000021A` (DLL not found) before printing anything, because the venv's `python3xx.dll` is
//! not on the search path for a bare exe. So this harness is Linux-only in practice, which is also
//! where `valgrind` lives: run it on the GitHub runner or in WSL, never directly on a Windows host.
//! The gate script does not attempt to hide that - if `--list` cannot run, the failure surfaces as a
//! build/exec error rather than an empty scenario list, which would read as "no scenarios to measure"
//! and pass.
use pyrs_yaml::py::functions::bench_to_python;
use pyrs_yaml_core::bench_inputs::{ANCHOR_YAML, MEDIUM_YAML, SMALL_YAML};

/// Fixed for every run and every machine. Smaller than the engine harness's 2 000 because each
/// iteration also allocates and releases Python objects: 500 iterations of `MEDIUM_YAML` already
/// counts seven-figure work, and a scenario that takes minutes under callgrind gets the gate skipped
/// rather than tolerated.
const ITERATIONS: u32 = 500;

/// Every scenario the binding-side gate measures. Names mirror the engine harness with a `to_python_`
/// prefix so the two channels can be read side by side in one baseline file.
fn scenarios() -> Vec<(&'static str, &'static str)> {
    vec![
        ("to_python_small", SMALL_YAML),
        ("to_python_medium", MEDIUM_YAML),
        // The one that resolves anchors instead of skipping them: `&` in the source switches the
        // conversion to the collecting path, so this is a different code path, not more of the same.
        ("to_python_anchors", ANCHOR_YAML),
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        for (name, _) in scenarios() {
            println!("{name}");
        }
        return;
    }
    let Some(name) = args.first() else {
        eprintln!("usage: ir_gate <scenario> [--setup-only] | --list");
        std::process::exit(2);
    };
    let setup_only = args.iter().any(|a| a == "--setup-only");
    let Some((_, src)) = scenarios().into_iter().find(|(n, _)| n == name) else {
        eprintln!("unknown scenario {name:?}");
        std::process::exit(2);
    };

    // Interpreter start-up is in both modes so it cancels: without this, the first measured call
    // would pay for initialising CPython and the subtraction would leave that in the number. Only
    // `black_box` touches `py`, deliberately: the point is the side effect of entering the
    // interpreter, not any particular API call, and the loop below would run regardless.
    pyo3::Python::attach(|py| {
        std::hint::black_box(&py);
    });

    let measured = if setup_only {
        std::hint::black_box(src);
        0
    } else {
        // Every iteration has to produce exactly one object. `bench_to_python` returns its error
        // instead of swallowing it, and a failed or empty conversion exits here rather than being
        // counted as a cheap iteration: the gate's tolerance is one-sided (it flags growth), so a
        // harness that quietly stopped doing the work would read as an enormous speedup and pass.
        let mut acc = 0usize;
        for index in 0..ITERATIONS {
            match std::hint::black_box(bench_to_python(std::hint::black_box(src))) {
                Ok(1) => acc += 1,
                Ok(other) => {
                    eprintln!(
                        "{name} iteration {index}: the conversion returned {other}, expected 1 - \
                         the fixture no longer reaches the path being measured"
                    );
                    std::process::exit(3);
                }
                Err(error) => {
                    eprintln!(
                        "{name} iteration {index}: {error} - a harness that measured nothing must \
                         not report a speedup"
                    );
                    std::process::exit(3);
                }
            }
        }
        acc
    };

    // Printed so a human re-running the binary sees it did something, and so the result cannot be
    // optimised away before the print.
    println!(
        "{name} mode={} objects={measured}/{ITERATIONS}",
        if setup_only { "setup" } else { "full" }
    );
}
