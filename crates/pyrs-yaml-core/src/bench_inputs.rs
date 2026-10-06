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
//! is part of the fixture's identity.
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
