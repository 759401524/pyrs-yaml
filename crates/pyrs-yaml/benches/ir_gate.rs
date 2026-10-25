//! Instruction-count gate, binding side: the YAML AST-to-Python conversion and the TOML loader.
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
//! The YAML family deliberately skips the P3 direct-load fast path that `safe_load` prefers for
//! plain documents: the quantity of interest is the AST-to-Python conversion, and an anchor-free
//! fixture would otherwise report a shortcut while a tag- or anchor-bearing one reported the slow path
//! - two different things under one name. See `bench_to_python` in `src/py/functions.rs`.
//!
//! The TOML family takes the other choice because the facts are the other way: the direct
//! IR-to-Python reader is the route an ordinary document serves, so `load_toml_medium` measures scan,
//! intermediate tree, scalar projection and object construction as the single quantity a caller
//! pays. Nothing is compiled away to get there: the harness asks `load_toml`'s own guard before the
//! counted loop and exits 4 rather than price the node model under this name. See `bench_load_toml`
//! and `toml_route_is_direct` in `src/py/functions.rs`.
//!
//! The JSON family takes the same choice for a different reason. `load_json`, `load_jsonc` and
//! `load_json5` each ask `json_fast::try_load` first, and for a canonical document that reader is
//! the whole route - so until `load_json_*` existed, a per-key allocation inside the fast path could
//! move no committed number at all: the engine's `from_json_*` stops at the node model, which the
//! loaders no longer build. The premise here is per-document rather than per-process (`try_load`
//! bails on the whole document for any non-canonical spelling), so the harness asks it of the
//! fixture's own bytes before the counted loop and exits 4 rather than price the fallback.
//! `load_jsonc` and `load_json5` share the row because the only thing that differs between them is
//! that fallback. See `bench_load_json` and `json_route_is_fast` in `src/py/functions.rs`.
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
use pyrs_yaml::py::functions::{
    bench_load_json, bench_load_toml, bench_to_python, json_route_is_fast, toml_route_is_direct,
};
use pyrs_yaml_core::bench_inputs::{
    ANCHOR_YAML, MEDIUM_JSON, MEDIUM_TOML, MEDIUM_YAML, SMALL_YAML, long_string_json,
    long_string_toml,
};

/// Fixed for every run and every machine, and the same 2 000 the engine harness uses - for a reason
/// measured on the runner rather than assumed. At 500 iterations two refresh runs of this file
/// disagreed by 0.83% on `to_python_small` (126,865 instructions on 15.2M) while the engine scenarios
/// agreed to 0.0009%: the loop was too small relative to the run-to-run variation of the interpreter
/// work inside it, and 0.83% is wider than the 0.5% line the gate holds. Four times the work puts the
/// smallest scenario in the same order of magnitude as its siblings, which is what makes the count
/// comparable across machines at all.
const ITERATIONS: u32 = 2_000;

/// The long-value TOML shape, generated once for the whole harness: `scenarios()` carries
/// `&'static str`, and these are the same bytes `from_toml_long_strings` measures one layer down -
/// one fixture, two harnesses, the standing rule.
static LONG_TOML: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_toml);

/// The long-value JSON shape, on the same rule as [`LONG_TOML`]: the same bytes
/// `to_json_long_strings` measures one layer down.
static LONG_JSON: std::sync::LazyLock<String> = std::sync::LazyLock::new(long_string_json);

/// Which loader the measured loop calls. One enum so the scenario list states in one column what each
/// baseline row prices, and so a name cannot silently change what it measures.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Channel {
    /// YAML text to Python objects through the AST.
    ToPython,
    /// TOML text to Python objects through `load_toml`.
    LoadToml,
    /// JSON text to Python objects through the AST-free reader `load_json` prefers.
    LoadJson,
}

/// Every scenario the binding-side gate measures. The `to_python_*` names mirror the engine harness so
/// the two channels can be read side by side in one baseline file; the `load_toml_*` names say the
/// entry point they price, because there is no engine-side twin at the same layer.
fn scenarios() -> Vec<(&'static str, &'static str, Channel)> {
    vec![
        ("to_python_small", SMALL_YAML, Channel::ToPython),
        ("to_python_medium", MEDIUM_YAML, Channel::ToPython),
        // The one that resolves anchors instead of skipping them: `&` in the source switches the
        // conversion to the collecting path, so this is a different code path, not more of the same.
        ("to_python_anchors", ANCHOR_YAML, Channel::ToPython),
        // The route an ordinary TOML document serves since the reader stopped building the node
        // model: scan, intermediate tree, scalar projection and object construction in one number,
        // so a regression cannot hide in whichever half this row does not name.
        ("load_toml_medium", MEDIUM_TOML, Channel::LoadToml),
        // The shape where a scalar costs more than its own length, which is what the reader hands to
        // the sink rather than what it decodes.
        (
            "load_toml_long_strings",
            LONG_TOML.as_str(),
            Channel::LoadToml,
        ),
        // The route an ordinary JSON document has taken since the family gained its fast reader:
        // scan plus object construction, with no node model in between to price separately.
        ("load_json_medium", MEDIUM_JSON, Channel::LoadJson),
        (
            "load_json_long_strings",
            LONG_JSON.as_str(),
            Channel::LoadJson,
        ),
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
    let Some((_, src, channel)) = scenarios().into_iter().find(|(n, _, _)| n == name) else {
        eprintln!("unknown scenario {name:?}");
        std::process::exit(2);
    };

    // The TOML floor names a route, so ask that route's own guard before measuring it rather than
    // trusting that nothing registered a handler: with one installed, `load_toml` answers from the
    // node model, and the same scenario would report a different quantity - which the one-sided
    // tolerance cannot catch, because the node model is the slower path and the difference reads as
    // a regression. Asked here, outside the counted loop, so the premise costs nothing per call.
    if channel == Channel::LoadToml && !std::hint::black_box(toml_route_is_direct()) {
        eprintln!(
            "{name}: a tag handler is registered, so load_toml would take the AST route - this \
             scenario measures the direct reader and refuses to price the other one"
        );
        std::process::exit(4);
    }
    if channel == Channel::LoadJson && !std::hint::black_box(json_route_is_fast(src)) {
        eprintln!(
            "{name}: {}",
            concat!(
                "json_fast::try_load bails on these bytes, so load_json would take the AST route - ",
                "this scenario measures the direct reader and refuses to price the other one"
            )
        );
        std::process::exit(4);
    }

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
        // Every iteration has to produce exactly one object. Both seams return their error instead of
        // swallowing it, and a failed or empty conversion exits here rather than being counted as a
        // cheap iteration: the gate's tolerance is one-sided (it flags growth), so a harness that
        // quietly stopped doing the work would read as an enormous speedup and pass.
        let mut acc = 0usize;
        for index in 0..ITERATIONS {
            let converted = match channel {
                Channel::ToPython => bench_to_python(std::hint::black_box(src)),
                Channel::LoadToml => bench_load_toml(std::hint::black_box(src)),
                Channel::LoadJson => bench_load_json(std::hint::black_box(src)),
            };
            match std::hint::black_box(converted) {
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
