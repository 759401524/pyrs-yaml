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
