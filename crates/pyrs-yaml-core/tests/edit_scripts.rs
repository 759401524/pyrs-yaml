//! The `edit_yaml` corpus, replayed on every `cargo nextest`.
//!
//! Every seed under `fuzz/seeds/edit_yaml` is a `document NUL script` pair, and each was committed
//! because an edit session over that document broke a promise: text the reader refused, an emission
//! that did not settle, a value that did not read back, or a tree that one of the sibling writers
//! could not carry. Those promises were asserted by the fuzz target, which runs them only when a
//! fuzzer happens to run — so between fuzz runs the corpus was a regression set nobody replayed, and
//! a fix could regress without a red check. `note_survival.rs` charges the *document* half of the same
//! files for note conservation; the script half had no deterministic replay at all.
//!
//! This test runs the target's own driver (`fuzz/edit_driver.rs`, included by path, not copied) over
//! every committed seed. One driver, two consumers: the target keeps the assertions authoritative and
//! this replay cannot quietly drift into asking less. A panic is re-raised with the seed's file name,
//! because the driver's message carries the input text and the file is what a reader needs first.
//!
//! The corpus floors below are what keeps this from passing vacuously: a seed whose document the
//! parser refuses contributes nothing, and a corpus that lost its scripts would otherwise replay
//! clean. Measured 2026-11-18 over the 69 seeds, the replay reads 937 records and applies 734
//! edits. A growing corpus clears the floors and they stay put; one that shrinks, or a driver that
//! starts refusing edits it used to apply, reddens and asks for a re-measure instead of passing on
//! less. The corpus total is not the only floor: each seed is also held to at least one accepted
//! edit of its own, because the document half is charged by `note_survival.rs` too and a script
//! whose every verb refuses would sit in the totals while the edit tier never ran. Measured the same
//! day, every seed clears that per-seed floor - the smallest is one accepted edit, and every seed
//! whose script carries a single record applies that one - which is the claim `seed_reach.rs`
//! names as the gap its document-only predicate leaves. The replay earned its keep on its first
//! run: `former-crash-fb62e776.seed` drove an edit whose emission settled only on its second round,
//! a writer gap the fuzz tier could only find when a fuzzer happened to run it.

#[path = "../../../fuzz/edit_driver.rs"]
mod driver;

use std::path::Path;

#[test]
fn every_committed_edit_script_replays() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/edit_yaml");
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "{} holds no seeds; a replay of nothing is not a pass",
        dir.display()
    );

    let (mut records, mut applied) = (0usize, 0usize);
    for path in &entries {
        let raw = std::fs::read(path).expect("seed must be readable");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let report = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| driver::run(&raw)))
            .unwrap_or_else(|payload| {
                let detail = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("<non-string panic>");
                panic!("{name}: {detail}");
            });
        assert!(
            report.applied > 0,
            "{name}: the script applied no edit (records={}), so the document half is all this seed \
             charges and the edit tier never ran; a seed whose every verb refuses must not pass on \
             the corpus floors alone",
            report.records
        );
        records += report.records;
        applied += report.applied;
    }

    assert!(
        entries.len() >= 69,
        "expected at least 69 edit seeds, got {}",
        entries.len()
    );
    assert!(
        records >= 937,
        "expected at least 937 script records to replay, got {records}"
    );
    assert!(
        applied >= 734,
        "expected at least 734 edits to be applied across the corpus, got {applied}"
    );
}
