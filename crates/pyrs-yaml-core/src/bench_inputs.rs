//! Canonical benchmark inputs, shared by every benchmark harness in the
//! workspace.
//!
//! Two harnesses measure the same hot paths: `crates/pyrs-yaml/benches/yaml_bench.rs`
//! (divan, reported through CodSpeed) and `crates/pyrs-yaml-core/benches/ir_gate.rs`
//! (fixed-iteration, measured in counted instructions). If each kept its own copy
//! of these documents they would drift, and a change that regressed one would be
//! invisible to the other — which defeats the point of having a second measurement.
//! So the bytes live here once and both sides `use` them.
//!
//! Kept byte-exact against their originals, including leading newlines: an
//! implicit-document start changes indentation probing, so the instruction count
//! is part of the fixture's identity. The two non-YAML literals below are pinned the
//! same way, and `crates/pyrs-yaml-core/tests/ir_fixtures.rs` is what keeps them from
//! rotting into documents the engines no longer emit.
#![doc(hidden)]

use alloc::string::String;

/// Two plain pairs, no comments, no anchors: the smallest realistic mapping.
pub const SMALL_YAML: &str = "key: value\nname: test\n";

/// Three nesting levels, a sequence, and blank lines between sections.
pub const MEDIUM_YAML: &str = r#"server:
  host: localhost
  port: 8080
  timeout: 30

database:
  driver: postgres
  host: db.example.com
  port: 5432
  name: myapp
  pool_size: 10

logging:
  level: info
  format: json
  outputs:
    - stdout
    - file:/var/log/app.log

features:
  auth: true
  cache: true
  rate_limit: false
"#;

/// Anchors, aliases and merge keys: exercises metadata handling on both sides of
/// the round trip.
pub const ANCHOR_YAML: &str = r#"
defaults: &defaults
  timeout: 30
  retries: 3
  pool_size: 10

production:
  <<: *defaults
  host: prod.example.com
  debug: false

staging:
  <<: *defaults
  host: staging.example.com
  debug: true
"#;

/// Merge keys whose sources are written out rather than aliased: `<<: { … }` is folded
/// at parse time, and the `<<:` inside the second item of a merge sequence is the
/// nested case whose data loss #290 fixed. `ANCHOR_YAML` cannot reach it - every one
/// of its merges resolves through a single alias - so this input exists to keep the
/// inline path measured, not only asserted over.
pub const MERGE_INLINE_YAML: &str = r#"
base:
  <<: { timeout: 30, retries: 3, pool_size: 10 }
  host: inline.example.com
  debug: false

nested:
  <<:
    - { a: 1, b: 2 }
    - { <<: { c: 3 }, d: 4 }
  name: nested
"#;

/// Block mapping with a nested mapping and a block sequence.
pub const BLOCK_STYLE_YAML: &str = "key1: value1\nkey2: value2\nnested:\n  subkey1: subvalue1\n  subkey2: subvalue2\nlist:\n  - item1\n  - item2\n  - item3\n";

/// Literal (`|`) and folded (`>`) block scalars, including a preserved blank line.
pub const BLOCK_SCALAR_YAML: &str = r#"
description: |
  This is a multi-line
  literal block scalar.
  It preserves newlines exactly.

  Including blank lines.
folded: >
  This is a folded
  block scalar that
  will be folded into
  a single line.
"#;

/// [`MEDIUM_YAML`] rendered as JSON, committed as bytes rather than rendered at run
/// time. `to_json_medium` numbers the writer; `from_json_medium` numbers the reader, and
/// the reader must not inherit the writer's day - a change to how `to_json_text` spells
/// output would otherwise move the reading scenario and be read as a reading regression.
/// Re-derived deliberately, never accidentally: `tests/ir_fixtures.rs` requires these
/// bytes to equal what the writer produces today.
pub const MEDIUM_JSON: &str = r#"{"server":{"host":"localhost","port":8080,"timeout":30},"database":{"driver":"postgres","host":"db.example.com","port":5432,"name":"myapp","pool_size":10},"logging":{"level":"info","format":"json","outputs":["stdout","file:/var/log/app.log"]},"features":{"auth":true,"cache":true,"rate_limit":false}}"#;

/// The same document as JSON-with-comments: `//` after a value and a `/* … */` block between
/// members. This is the only reason the constant exists — the comment scanner in the JSON lexer is
/// a hot path the strict-JSON scenario never enters, so `load_jsonc` and `to_jsonc_text` had no
/// instruction-count number at all, and a change to comment skipping could not be adjudicated.
///
/// Not the writer's output, unlike [`MEDIUM_JSON`]: `to_jsonc_text` emits comments it is *handed*,
/// and the hub AST built from `MEDIUM_YAML` has none. Pinning this to a writer would therefore
/// erase the feature being measured, so `tests/ir_fixtures.rs` pins what the fixture must keep
/// instead — that it parses, that it parses to exactly the values `MEDIUM_JSON` does, and that the
/// comments are really in the bytes.
pub const MEDIUM_JSONC: &str = r#"{
  // server coordinates, read first by the boot sequence
  "server":{"host":"localhost","port":8080,"timeout":30},
  /* connection pool lives here, and the driver name is checked against the image */
  "database":{"driver":"postgres","host":"db.example.com","port":5432,"name":"myapp","pool_size":10},
  "logging":{"level":"info", // level is the only knob operators touch
    "format":"json","outputs":["stdout","file:/var/log/app.log"]},
  "features":{"auth":true,"cache":true,"rate_limit":false}
}"#;

/// The same document again in JSON5, whose reader path is wider still: unquoted keys, single-quoted
/// strings, and a trailing comma before every closing brace and bracket. Comment skipping is shared
/// with JSONC, so this fixture exists to enter the key-and-terminator grammar specifically.
///
/// `timeout` is spelled plainly on purpose. `3e1` was tried first, and the parity test then failed for
/// a reason that is now issue #312 rather than a mistake to smooth over: a JSON5 number whose spelling
/// YAML has no word for does not survive the hub as a number, and an exponent literal is preserved
/// verbatim into JSON, so byte parity between the two fixtures is exactly the property that bug
/// breaks. `port` keeps `0x1F90` because the JSON writer must canonicalise hex, which it does.
pub const MEDIUM_JSON5: &str = r#"{
  // the boot sequence reads these four sections in order
  server:{host:'localhost',port:0x1F90,timeout:30,},
  /* pool settings, spelled from the container image */
  database:{driver:'postgres',host:'db.example.com',port:5432,name:'myapp',pool_size:10,},
  logging:{level:'info',format:'json',outputs:['stdout','file:/var/log/app.log',],},
  features:{auth:true,cache:true,rate_limit:false,},
}"#;

/// [`MEDIUM_YAML`] rendered as TOML, for the reason [`MEDIUM_JSON`] states.
pub const MEDIUM_TOML: &str = r#"[server]
host = "localhost"
port = 8080
timeout = 30
[database]
driver = "postgres"
host = "db.example.com"
port = 5432
name = "myapp"
pool_size = 10
[logging]
level = "info"
format = "json"
outputs = ["stdout", "file:/var/log/app.log"]
[features]
auth = true
cache = true
rate_limit = false
"#;

// ---------------------------------------------------------------------------
// The long-string pair
//
// Every other document here has short values: `MEDIUM_JSON`'s longest is
// `file:/var/log/app.log`, so a reader of it asks the string-body question about
// sixteen bytes at a time and gets an answer either way. That is the shape the
// leaderboard documents have too, which is why the bulk scan's cost was only ever
// visible from a throwaway probe (`docs/dev/quality-ledger.md` (fh)). These two
// documents make it a measured thing with a floor of its own.
//
// They are generated, not literal, because the property that matters is the value's
// length and the absence of bytes the scanner stops on - a 2 KB run of letters
// committed as bytes would sit in every diff of this file and prove nothing the
// generator does not. `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins what the
// output must keep being.
// ---------------------------------------------------------------------------

/// How many records each long-string document carries.
pub const LONG_RECORDS: usize = 8;

/// The character length of each record's string value: sixteen 16-byte vectors, so a reader of
/// these documents spends most of its bytes inside the bulk scan and not in the tail a short
/// value falls to.
pub const LONG_VALUE: usize = 256;

/// One value: letters only, so neither quote of a pair, no backslash, no control byte and no
/// multi-byte character ends the run before its closing quote.
fn long_value() -> String {
    let mut run = String::with_capacity(LONG_VALUE);
    for index in 0..LONG_VALUE {
        run.push((b'a' + ((index * 7 + 11) % 26) as u8) as char);
    }
    run
}

/// `LONG_RECORDS` JSON pairs of a long string and a number - the same record list
/// [`long_string_toml`] renders, and the same shape as [`MEDIUM_JSON`] apart from the length of
/// its values. The pair is the point: read against `from_json_medium`, the only variable left is
/// how many bytes sit between one quote and the next.
pub fn long_string_json() -> String {
    let value = long_value();
    let mut doc = String::from("{");
    for index in 0..LONG_RECORDS {
        if index > 0 {
            doc.push(',');
        }
        doc.push_str(&format!(
            "\"k{index}\":\"{value}\",\"n{index}\":{}",
            index * 10
        ));
    }
    doc.push('}');
    doc
}

/// The same `LONG_RECORDS` pairs, one JSON object per line - the shape the line-delimited surface
/// reads, and the long-value twin of [`short_string_ndjson`]. The two differ only in how many bytes
/// sit between one quote and the next, so the pair prices a long body on a route no other scenario
/// enters: the AST scenarios build nodes, the stream scenarios report events, and this one walks
/// lines.
pub fn long_string_ndjson() -> String {
    ndjson(&long_value())
}

/// The line-delimited pair to [`long_string_ndjson`] with a short value: `localhost` is nine
/// characters, so a scanner never leaves its first vector, which is what makes the other fixture's
/// length the only variable.
pub fn short_string_ndjson() -> String {
    ndjson("localhost")
}

fn ndjson(value: &str) -> String {
    let mut doc = String::new();
    for index in 0..LONG_RECORDS {
        doc.push_str(&format!(
            "{{\"k{index}\":\"{value}\",\"n{index}\":{}}}\n",
            index * 10
        ));
    }
    doc
}

/// The same records in TOML, one pair per line.
pub fn long_string_toml() -> String {
    let value = long_value();
    let mut doc = String::new();
    for index in 0..LONG_RECORDS {
        doc.push_str(&format!(
            "k{index} = \"{value}\"\nn{index} = {}\n",
            index * 10
        ));
    }
    doc
}

/// The same record list in YAML, so the writer halves of the bridges can be measured over the
/// long-value shape too: every `to_*` scenario reads a YAML document, and a bridge whose reader half
/// has a floor while its writer half does not is the shape
/// `scripts/quality_matrix.py` reports as `ir-bridge-unidirectional`.
pub fn long_string_yaml() -> String {
    let value = long_value();
    let mut doc = String::new();
    for index in 0..LONG_RECORDS {
        doc.push_str(&format!("k{index}: {value}\nn{index}: {}\n", index * 10));
    }
    doc
}

/// The long JSON document with comments interleaved. The JSONC reader takes the same bulk string scan
/// as strict JSON, and the comment path is the part that differs, so this is the fixture that prices
/// both at once. The comments are printable text and line breaks: no byte the string scan stops on
/// enters one, and the values stay exactly [`LONG_VALUE`] characters wide.
pub fn long_string_jsonc() -> String {
    let value = long_value();
    let mut doc = String::from("{\n  // boot coordinates, read first\n");
    for index in 0..LONG_RECORDS {
        if index > 0 {
            doc.push_str(",\n  /* pool settings, spelled from the image */\n");
        }
        doc.push_str(&format!(
            "  \"k{index}\":\"{value}\",\"n{index}\":{}",
            index * 10
        ));
    }
    doc.push_str("\n}");
    doc
}

/// The same records in JSON5, whose values are single-quoted: the one dialect whose string body is
/// scanned by the single-quote instantiation of the vector rather than the double-quote one. Keys are
/// bare, and every member keeps the trailing comma the dialect allows.
pub fn long_string_json5() -> String {
    let value = long_value();
    let mut doc = String::from("{\n  // the boot sequence reads these in order\n");
    for index in 0..LONG_RECORDS {
        if index > 0 {
            doc.push_str("  /* and these are checked against the image */\n");
        }
        doc.push_str(&format!("  k{index}:'{value}',n{index}:{},\n", index * 10));
    }
    doc.push('}');
    doc
}
