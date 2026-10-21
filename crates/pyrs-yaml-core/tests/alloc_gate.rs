//! Ceilings on the heap allocations the medium-document hot paths make.
//!
//! The instruction gate (`.ci/ir-baseline.json`) and CodSpeed both measure time; neither can see
//! a writer that allocates one temporary per key, because the temporaries are small and short
//! lived. This file counts them directly: a `#[global_allocator]` wrapper over `System` with
//! per-thread counters, one warm-up run per scenario so one-time initialisation is not charged to
//! the measurement, then a single measured run.
//!
//! The fixtures come from `pyrs_yaml_core::bench_inputs`, the same constants the instruction gate
//! reads - a second copy of a fixture would let the two gates describe different documents.
//!
//! Ceilings are commitments, not observations: a change that lowers one is a win and may tighten
//! it, while a change that needs a higher one has to say why, the way `.ci/ir-baseline.json` is
//! refreshed by a decision instead of by a passing run.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

use pyrs_json::{from_json, to_json_text};
use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::bench_inputs::{MEDIUM_JSON, MEDIUM_TOML, MEDIUM_YAML};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

fn record(size: usize) {
    // `try_with`: the allocator serves every thread, including ones whose locals are already
    // destroyed during shutdown. A panicking allocator aborts the process.
    let _ = ALLOCS.try_with(|count| count.set(count.get() + 1));
    let _ = BYTES.try_with(|count| count.set(count.get() + size as u64));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn reset() {
    ALLOCS.with(|count| count.set(0));
    BYTES.with(|count| count.set(0));
}

fn read() -> (u64, u64) {
    (ALLOCS.with(Cell::get), BYTES.with(Cell::get))
}

struct Ceiling {
    allocs: u64,
    bytes: u64,
}

// Measured when this gate was written, 2026-10-17, with the compiler pinned by
// `.ci/ir-baseline.json`. `bytes` is the sum of requested sizes, with a reallocation charged at
// its new size. Raising either number is a decision that must be explained in the change.
const FROM_JSON_MEDIUM: Ceiling = Ceiling {
    allocs: 48,
    bytes: 9_444,
};
const TO_JSON_MEDIUM: Ceiling = Ceiling {
    allocs: 8,
    bytes: 1_048,
};
const FROM_TOML_MEDIUM: Ceiling = Ceiling {
    allocs: 97,
    bytes: 15_612,
};
const TO_TOML_MEDIUM: Ceiling = Ceiling {
    allocs: 12,
    bytes: 1_208,
};
// The parse scenario is the one number that depends on the feature set. The std build tracks
// seen scalar keys in a `HashSet` — one hash table per mapping — and keeps the committed line.
// `alloc` carries no hasher, so the no_std build tracks them in the crate's `NodeMap`, whose
// ordered entries vector is one more allocation per mapping; MEDIUM_YAML has five mappings, two
// of them outgrowing the first table. Measured 2026-10-21 on the no-default-features build:
// 74 allocations / 24952 bytes, against 67 / 24512 with std. The shipped engine is the std one,
// and its line did not move.
#[cfg(feature = "std")]
const PARSE_MEDIUM: Ceiling = Ceiling {
    allocs: 67,
    bytes: 24_512,
};
#[cfg(not(feature = "std"))]
const PARSE_MEDIUM: Ceiling = Ceiling {
    allocs: 74,
    bytes: 24_952,
};
const SERIALIZE_MEDIUM: Ceiling = Ceiling {
    allocs: 7,
    bytes: 1_016,
};

/// Measure one scenario after a warm-up, failing at the first ceiling it crosses.
macro_rules! measured {
    ($name:literal, $ceiling:expr, $work:expr) => {{
        let work = $work;
        work();
        reset();
        work();
        let (allocs, bytes) = read();
        let ceiling = $ceiling;
        println!(
            "{:<18} {allocs:>5} allocations {bytes:>8} bytes (ceiling {:>5} / {:>8})",
            $name, ceiling.allocs, ceiling.bytes
        );
        assert!(
            allocs <= ceiling.allocs,
            "{} made {allocs} allocations, {} past the committed ceiling of {}: a hot path grew \
             an allocation - remove it, or raise the ceiling with the reason",
            $name,
            allocs - ceiling.allocs,
            ceiling.allocs
        );
        assert!(
            bytes <= ceiling.bytes,
            "{} asked for {bytes} bytes, {} past the committed ceiling of {}: a hot path grew an \
             allocation - remove it, or raise the ceiling with the reason",
            $name,
            bytes - ceiling.bytes,
            ceiling.bytes
        );
    }};
}

#[test]
fn the_medium_hot_paths_allocate_within_their_ceilings() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the fixture parses");

    measured!("from_json_medium", FROM_JSON_MEDIUM, || {
        black_box(from_json(black_box(MEDIUM_JSON)).expect("the JSON fixture parses"));
    });
    measured!("to_json_medium", TO_JSON_MEDIUM, || {
        black_box(to_json_text(black_box(&ast)).expect("the hub AST writes as JSON"));
    });
    measured!("from_toml_medium", FROM_TOML_MEDIUM, || {
        black_box(from_toml(black_box(MEDIUM_TOML)).expect("the TOML fixture parses"));
    });
    measured!("to_toml_medium", TO_TOML_MEDIUM, || {
        black_box(to_toml(black_box(&ast)).expect("the hub AST writes as TOML"));
    });
    measured!("parse_medium", PARSE_MEDIUM, || {
        black_box(parse(black_box(MEDIUM_YAML), Schema::Core).expect("the YAML fixture parses"));
    });
    measured!("serialize_medium", SERIALIZE_MEDIUM, || {
        black_box(to_yaml(black_box(&ast)));
    });
}
