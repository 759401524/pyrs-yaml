use pyrs_yaml::ast::CustomNode;
use pyrs_yaml::parser::yaml::YamlSchema;
use pyrs_yaml::py::editing::{self, Segment};
use pyrs_yaml::splice::SpliceState;
use std::sync::Arc;

const SMALL_YAML: &str = "key: value\nname: test\n";

const MEDIUM_YAML: &str = r#"server:
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

const LARGE_YAML: &str = r#"
# Large YAML document for benchmarking
items:
  - name: item_001
    value: 100
    tags: [alpha, beta]
    metadata:
      created: 2024-01-01
      author: test
  - name: item_002
    value: 200
    tags: [gamma, delta]
    metadata:
      created: 2024-01-02
      author: test
  - name: item_003
    value: 300
    tags: [epsilon, zeta]
    metadata:
      created: 2024-01-03
      author: test
  - name: item_004
    value: 400
    tags: [eta, theta]
    metadata:
      created: 2024-01-04
      author: test
  - name: item_005
    value: 500
    tags: [iota, kappa]
    metadata:
      created: 2024-01-05
      author: test

config:
  debug: false
  verbose: true
  limits:
    max_connections: 100
    request_timeout: 30
    idle_timeout: 300

# Comment before mapping
database:
  primary:
    host: primary.db.local
    port: 5432
    replicas:
      - host: replica1.db.local
        port: 5433
      - host: replica2.db.local
        port: 5434
  cache:
    host: cache.db.local
    port: 6379
    ttl: 3600
"#;

const ANCHOR_YAML: &str = r#"
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

const COMMENT_YAML: &str = r#"
# Server configuration
server:
  host: localhost  # bind address
  port: 8080  # listen port

# Database settings
database:
  # Primary database
  host: db.example.com
  port: 5432
"#;

const BLOCK_STYLE_YAML: &str = "key1: value1\nkey2: value2\nnested:\n  subkey1: subvalue1\n  subkey2: subvalue2\nlist:\n  - item1\n  - item2\n  - item3\n";

const BLOCK_SCALAR_YAML: &str = r#"
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

fn main() {
    divan::main();
}

// ── Parse benchmarks ──

#[divan::bench]
fn parse_small() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap()
}

#[divan::bench]
fn parse_medium() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap()
}

#[divan::bench]
fn parse_large() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap()
}

#[divan::bench]
fn parse_anchors() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(ANCHOR_YAML, YamlSchema::Core).unwrap()
}

#[divan::bench]
fn parse_comments() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(COMMENT_YAML, YamlSchema::Core).unwrap()
}

#[divan::bench]
fn parse_block_scalars() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(BLOCK_SCALAR_YAML, YamlSchema::Core).unwrap()
}

// ── Serialize benchmarks (setup separate from measurement) ──

#[divan::bench]
fn serialize_small(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn serialize_medium(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn serialize_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn serialize_anchors(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(ANCHOR_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn serialize_block_scalars(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(BLOCK_SCALAR_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

// ── Roundtrip benchmarks (parse + serialize, full pipeline) ──

#[divan::bench]
fn roundtrip_small() -> String {
    let ast = pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    pyrs_yaml::serializer::to_yaml(&ast)
}

#[divan::bench]
fn roundtrip_medium() -> String {
    let ast = pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    pyrs_yaml::serializer::to_yaml(&ast)
}

#[divan::bench]
fn roundtrip_large() -> String {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    pyrs_yaml::serializer::to_yaml(&ast)
}

// ── Block-style serialize ──

#[divan::bench]
fn serialize_block(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(BLOCK_STYLE_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

// ── Editing benchmarks (pure AST mutation; lazy sync defers serialization) ──

#[divan::bench]
fn edit_set_small(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    let segs = vec![Segment::Key(std::borrow::Cow::Borrowed("key"))];
    bencher.bench(|| {
        let mut a = ast.clone();
        editing::set_path(
            &mut a,
            &segs,
            CustomNode::plain_scalar("x"),
            true,
            SMALL_YAML,
            None,
            false,
        )
        .ok();
    });
}

#[divan::bench]
fn edit_set_medium(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    let segs = vec![Segment::Key(std::borrow::Cow::Borrowed("database"))];
    bencher.bench(|| {
        let mut a = ast.clone();
        editing::set_path(
            &mut a,
            &segs,
            CustomNode::plain_scalar("999"),
            true,
            MEDIUM_YAML,
            None,
            false,
        )
        .ok();
    });
}

#[divan::bench]
fn edit_set_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    let segs = vec![
        Segment::Key(std::borrow::Cow::Borrowed("config")),
        Segment::Key(std::borrow::Cow::Borrowed("limits")),
        Segment::Key(std::borrow::Cow::Borrowed("max_connections")),
    ];
    bencher.bench(|| {
        let mut a = ast.clone();
        editing::set_path(
            &mut a,
            &segs,
            CustomNode::plain_scalar("999"),
            true,
            LARGE_YAML,
            None,
            false,
        )
        .ok();
    });
}

#[divan::bench]
fn edit_insert_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    let segs = vec![Segment::Key(std::borrow::Cow::Borrowed("config"))];
    bencher.bench(|| {
        let mut a = ast.clone();
        editing::insert_path(
            &mut a,
            &segs,
            0,
            CustomNode::plain_scalar("x"),
            LARGE_YAML,
            None,
        )
        .ok();
    });
}

#[divan::bench]
fn edit_delete_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    let segs = vec![Segment::Key(std::borrow::Cow::Borrowed("config"))];
    bencher.bench(|| {
        let mut a = ast.clone();
        editing::delete_path(&mut a, &segs, LARGE_YAML, None).ok();
    });
}

#[divan::bench]
fn edit_batch_10(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    let source: Arc<str> = Arc::from(LARGE_YAML);
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        for i in 0..10 {
            let segs = vec![
                Segment::Key(std::borrow::Cow::Borrowed("items")),
                Segment::Index(i % 5),
            ];
            if let Ok(unit) = editing::set_path(
                &mut a,
                &segs,
                CustomNode::plain_scalar("x"),
                true,
                SMALL_YAML,
                None,
                false,
            ) {
                if unit.eligible {
                    state.apply(&unit).ok();
                }
            }
        }
        state.materialize();
    });
}

// ── 10MB edit-flush benches ──

fn make_large_doc(approx_bytes: usize) -> String {
    let num_groups = 500usize;
    let keys_per_group = approx_bytes / (num_groups * 25);
    let mut yaml = String::with_capacity(approx_bytes);
    for g in 0..num_groups {
        yaml.push_str(&format!("group_{g:03}:\n"));
        for k in 0..keys_per_group {
            yaml.push_str(&format!("  key_{k:04}: value\n"));
        }
    }
    yaml
}

#[divan::bench]
fn serialize_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn clone_ast_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    bencher.bench(|| ast.clone());
}

#[divan::bench]
fn serialize_with_clone_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    bencher.bench(|| {
        let a = ast.clone();
        pyrs_yaml::serializer::to_yaml(&a)
    });
}

#[divan::bench]
fn edit_flush_set_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    let source: Arc<str> = Arc::from(yaml);
    let segs = vec![
        Segment::Key(std::borrow::Cow::Borrowed("group_000")),
        Segment::Key(std::borrow::Cow::Borrowed("key_0000")),
    ];
    let new_value = CustomNode::plain_scalar("zzz");
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        if let Ok(unit) =
            editing::set_path(&mut a, &segs, new_value.clone(), true, &source, None, false)
        {
            if unit.eligible {
                state.apply(&unit).ok();
            }
        }
        state.materialize();
    });
}

#[divan::bench]
fn edit_flush_burst5_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    let source: Arc<str> = Arc::from(yaml);
    let targets = [
        (
            Segment::Key(std::borrow::Cow::Borrowed("group_000")),
            Segment::Key(std::borrow::Cow::Borrowed("key_0000")),
        ),
        (
            Segment::Key(std::borrow::Cow::Borrowed("group_000")),
            Segment::Key(std::borrow::Cow::Borrowed("key_0001")),
        ),
        (
            Segment::Key(std::borrow::Cow::Borrowed("group_001")),
            Segment::Key(std::borrow::Cow::Borrowed("key_0000")),
        ),
        (
            Segment::Key(std::borrow::Cow::Borrowed("group_001")),
            Segment::Key(std::borrow::Cow::Borrowed("key_0001")),
        ),
        (
            Segment::Key(std::borrow::Cow::Borrowed("group_002")),
            Segment::Key(std::borrow::Cow::Borrowed("key_0000")),
        ),
    ];
    let new_value = CustomNode::plain_scalar("zzz");
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        for (g, k) in &targets {
            let segs = vec![g.clone(), k.clone()];
            if let Ok(unit) =
                editing::set_path(&mut a, &segs, new_value.clone(), true, &source, None, false)
            {
                if unit.eligible {
                    state.apply(&unit).ok();
                }
            }
        }
        state.materialize();
    });
}

// ── Complex-doc benches (10MB with comments, anchors, tags, block scalars) ──

fn make_complex_doc(approx_bytes: usize) -> String {
    let num_groups = 200usize;
    let keys_per_group = approx_bytes / (num_groups * 30).max(1);
    let mut yaml = String::with_capacity(approx_bytes);
    for g in 0..num_groups {
        if g % 20 == 0 {
            yaml.push_str(&format!("group_{g:03}: &g{g:03}\n"));
        } else {
            yaml.push_str(&format!("group_{g:03}:\n"));
        }
        for k in 0..keys_per_group {
            if k % 20 == 5 {
                yaml.push_str(&format!("  key_{k:04}: |\n    value {g}_{k}\n"));
            } else if k % 20 == 0 {
                yaml.push_str(&format!("  key_{k:04}: !!str value  # inline\n"));
            } else if k % 10 == 0 {
                yaml.push_str(&format!("  key_{k:04}: value  # inline\n"));
            } else {
                yaml.push_str(&format!("  key_{k:04}: value\n"));
            }
        }
    }
    yaml
}

#[divan::bench]
fn serialize_complex_2mb(bencher: divan::Bencher) {
    let yaml = make_complex_doc(2 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

#[divan::bench]
fn serialize_with_clone_complex_2mb(bencher: divan::Bencher) {
    let yaml = make_complex_doc(2 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    bencher.bench(|| {
        let a = ast.clone();
        pyrs_yaml::serializer::to_yaml(&a)
    });
}

#[divan::bench]
fn edit_flush_set_complex_2mb(bencher: divan::Bencher) {
    let yaml = make_complex_doc(2 * 1024 * 1024);
    let ast = pyrs_yaml::parser::parse(&yaml, YamlSchema::Core).unwrap();
    let source: Arc<str> = Arc::from(yaml);
    let segs = vec![
        Segment::Key(std::borrow::Cow::Borrowed("group_000")),
        Segment::Key(std::borrow::Cow::Borrowed("key_0000")),
    ];
    let new_value = CustomNode::plain_scalar("zzz");
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        if let Ok(unit) =
            editing::set_path(&mut a, &segs, new_value.clone(), true, &source, None, false)
        {
            if unit.eligible {
                state.apply(&unit).ok();
            }
        }
        state.materialize();
    });
}
