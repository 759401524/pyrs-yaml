//! The edit verbs, driven over a hostile document.
//!
//! Nothing in the fuzz tier reached this layer. The eight targets before this one hand a *document* to a
//! reader or a writer; the code that rewrites one region of a document - what `doc.set()` and the `pyq`
//! verbs call, and what the round-trip promise is actually about - was only ever asked by named unit
//! tests and by the proptest suite over *generated* trees. A generated tree cannot hold the shapes that
//! break region splicing: a note under a tag-only line, a CR-only break, an alias in key position. Fuzzed
//! bytes can, and each of those shapes has already cost a pull request.
//!
//! The input is `document NUL script`. A script is pairs of bytes: the first picks a path the reader
//! really found in the document (paths are collected from the tree, so mutations land inside the document
//! instead of being refused by a path that names nothing), the second picks the verb and its operand.
//! Every accepted edit is followed by its emission and a re-parse, and the next edit works on that
//! re-read tree - so one corpus entry can drive a whole editing session.
//!
//! What this asserts, and each of these can fail:
//!
//! - an accepted edit emits text the parser accepts (the writer's own contract, as in `yaml_roundtrip`);
//! - the emission is a *fixed point*: re-parsing it and serializing again gives the same bytes. An
//!   edit whose text needs a second round to settle is the `(ds)`/`(dq)` species, caught per document
//!   now rather than in a hand-written case;
//! - `set` reads back the text it wrote at its path; `rename` finds one pair fewer answering the name
//!   it left and the new name answering; `move` finds one pair fewer answering its source path;
//!   `delete` - asked with the `all` policy - finds nothing answering the path it deleted; a sequence
//!   item is a position the next item shifts into, so its parent holds one item fewer; `insert` and
//!   `append` find exactly one more item holding the written text at its index; `sort-keys` finds the
//!   mapping's keys in the order the engine sorts them; and `set-comment` is held to the structural
//!   claims only - see the boundary below;
//! - the settled tree is also what the other spokes write. Each of `to_toml`, `to_json_text`,
//!   `to_jsonc_text` and `to_json5_text` is handed that tree, and its text must re-read in its own
//!   dialect and survive a second emission. `toml_roundtrip` and `json_roundtrip` make those two
//!   claims about a *reader's* tree; this is the composition the CLI's TOML/JSON output and
//!   `doc.to_toml()` reach after an edit, which no tier had asked about.
//!
//! Four claims are narrower than they look, and each was narrowed by a measurement rather than by
//! taste. Sorting is asked only where the verb has work to do: `sort_keys_path` answers "ok" for a
//! path naming a scalar and changes nothing, which is a question about a refusal the CLI would show,
//! not about emitted text. A note is followed only onto the line it was written on: asked for on a
//! block sequence it lands on the last item's line and re-reads as that item's, because there is no
//! other place in the text it could live, and exploration found a scalar shape doing the same. The
//! position itself is decided now - `note_placement`, default `note_first`, writes a note that
//! cannot ride its line above the `&a` / `!tag` header line, where a read files it, so the first
//! emission settles - but the settling position can hand the note to the enclosing node, so the
//! owner claim is withheld rather than written as a permission. Style is the third: the writer
//! refuses a style a text cannot carry (a
//! block scalar cannot hold a trailing space) and that fallback is correct, so for style, flow, tag
//! and anchor the claim here is only re-parse and settle, while `prop_style_setting_roundtrip` asks
//! the equality question on trees where the answer is determined. The fourth is what an identity
//! claim can ask at all: a path resolves through the lookup's identity pass, but a key text the
//! document spells twice - the siblings told apart only by a tag - is answered after the edit by
//! whichever pair still spells it, through the spelling pass. So `rename` and `move` are held to a
//! count, not to the old name being gone: measured on the 19-byte `!base k: 1` beside `k: 2`, the
//! rename wrote `k0: 2`, the emission settled in one round, and `[Key("k")]` still read the tagged
//! pair that was never its target - the same asymmetry `(iv)`'s default delete documents as the
//! engine's answer.
//!
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../edit_driver.rs"]
mod driver;

fuzz_target!(|data: &[u8]| {
    driver::run(data);
});
