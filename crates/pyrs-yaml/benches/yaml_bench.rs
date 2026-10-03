use pyrs_yaml::ast::CustomNode;
use pyrs_yaml::parser::yaml::YamlSchema;
use pyrs_yaml::py::editing;
use pyrs_yaml::splice::SpliceState;
use pyrs_yaml_core::editing::Segment;
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

// Shared fixtures for the LARGE_* documents below. Each macro expands to a
// string literal so the variants can `concat!` them, keeping every byte
// (including newlines) identical to the hand-written originals.
macro_rules! items_block {
    () => {
        r#"items:
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

"#
    };
}

macro_rules! config_block {
    () => {
        r#"config:
  debug: false
  verbose: true
  limits:
    max_connections: 100
    request_timeout: 30
    idle_timeout: 300

"#
    };
}

/// Same as `config_block!` but with a merge key on `defaults`.
macro_rules! config_merge_block {
    () => {
        concat!(
            "config:\n",
            "  <<: *defaults\n",
            "  debug: false\n",
            "  verbose: true\n",
            "  limits:\n",
            "    max_connections: 100\n",
            "    request_timeout: 30\n",
            "    idle_timeout: 300\n",
            "\n",
        )
    };
}

macro_rules! database_block {
    () => {
        r#"database:
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
"#
    };
}

const LARGE_YAML: &str = concat!(
    "\n# Large YAML document for benchmarking\n",
    items_block!(),
    config_block!(),
    "# Comment before mapping\n",
    database_block!(),
);

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

/// LARGE_YAML with the two comment lines removed. No `#`, no `&`:
/// an anchor-free document for the pure-parse baseline.
const LARGE_NO_EXTRACT_YAML: &str =
    concat!("\n", items_block!(), config_block!(), database_block!(),);

/// LARGE_YAML with a defaults anchor + merge keys added: exercises the
/// full merge resolution path (contrast with parse_large fast path).
const LARGE_MERGE_YAML: &str = concat!(
    "\ndefaults: &defaults\n",
    "  timeout: 30\n",
    "  retries: 3\n",
    "  pool_size: 10\n",
    "\n",
    items_block!(),
    config_merge_block!(),
    database_block!(),
);

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

/// No `&` anchors — the common anchor-free document; per-event name resolution
/// is a no-op so this measures the pure parse path.
#[divan::bench]
fn parse_large_no_extract() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(LARGE_NO_EXTRACT_YAML, YamlSchema::Core).unwrap()
}

/// Has `<<` merges — exercises the full merge resolution path.
#[divan::bench]
fn parse_large_with_merges() -> pyrs_yaml::ast::CustomNode {
    pyrs_yaml::parser::parse(LARGE_MERGE_YAML, YamlSchema::Core).unwrap()
}

// ── Parse sub-step micro-benchmarks (decompose parse cost) ──

/// Receiver that only counts events: isolates granit's tokenizer/parser cost
/// from our AstReceiver's CustomNode construction.
struct CountReceiver {
    count: usize,
}

impl<'a> granit_parser::SpannedEventReceiver<'a> for CountReceiver {
    fn on_event(&mut self, _event: granit_parser::Event<'a>, _span: granit_parser::Span) {
        self.count += 1;
    }
}

fn granit_event_count(yaml: &str) -> usize {
    let mut receiver = CountReceiver { count: 0 };
    let mut parser = granit_parser::Parser::new_from_str(yaml);
    parser.load(&mut receiver, true).ok();
    receiver.count
}

#[divan::bench]
fn granit_events_small() -> usize {
    granit_event_count(SMALL_YAML)
}

#[divan::bench]
fn granit_events_medium() -> usize {
    granit_event_count(MEDIUM_YAML)
}

#[divan::bench]
fn granit_events_large() -> usize {
    granit_event_count(LARGE_YAML)
}

/// Scalar type resolution on a sampled set of scalar strings (schema dispatch
/// cost per scalar, amortized over the common-case string path).
#[divan::bench]
fn resolve_core_type_many(bencher: divan::Bencher) {
    let scalars = [
        "hello",
        "server",
        "database",
        "localhost",
        "item_001",
        "alpha",
        "test",
        "true",
        "false",
        "null",
        "42",
        "-17",
        "3.14",
        "0x1F",
        "0o17",
        "2024-01-01",
        "10.0.0.1",
        "postgres",
        "verbose",
        "stdout",
        "1234567890123",
    ];
    bencher.bench(|| {
        let mut n = 0usize;
        for s in scalars {
            if matches!(
                pyrs_yaml::parser::yaml::schema::resolve_yaml_type(s, YamlSchema::Core),
                pyrs_yaml::parser::yaml::types::YamlType::Str(_)
            ) {
                n += 1;
            }
        }
        n
    });
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
            ) && unit.eligible
            {
                state.apply(&unit).ok();
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

fn key(k: &str) -> Segment<'_> {
    Segment::Key(std::borrow::Cow::Borrowed(k))
}

fn parse_doc(yaml: &str) -> CustomNode {
    pyrs_yaml::parser::parse(yaml, YamlSchema::Core).unwrap()
}

/// Shared bodies for the large/complex document benches: the measured
/// operation is identical, only the fixture generator differs.
fn bench_serialize(bencher: divan::Bencher, yaml: String) {
    let ast = parse_doc(&yaml);
    bencher.bench(|| pyrs_yaml::serializer::to_yaml(&ast));
}

fn bench_serialize_with_clone(bencher: divan::Bencher, yaml: String) {
    let ast = parse_doc(&yaml);
    bencher.bench(|| {
        let a = ast.clone();
        pyrs_yaml::serializer::to_yaml(&a)
    });
}

fn bench_edit_flush_set(bencher: divan::Bencher, yaml: String) {
    let ast = parse_doc(&yaml);
    let source: Arc<str> = Arc::from(yaml);
    let segs = vec![key("group_000"), key("key_0000")];
    let new_value = CustomNode::plain_scalar("zzz");
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        if let Ok(unit) =
            editing::set_path(&mut a, &segs, new_value.clone(), true, &source, None, false)
            && unit.eligible
        {
            state.apply(&unit).ok();
        }
        state.materialize();
    });
}

#[divan::bench]
fn serialize_10mb(bencher: divan::Bencher) {
    bench_serialize(bencher, make_large_doc(10 * 1024 * 1024));
}

#[divan::bench]
fn clone_ast_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = parse_doc(&yaml);
    bencher.bench(|| ast.clone());
}

#[divan::bench]
fn serialize_with_clone_10mb(bencher: divan::Bencher) {
    bench_serialize_with_clone(bencher, make_large_doc(10 * 1024 * 1024));
}

#[divan::bench]
fn edit_flush_set_10mb(bencher: divan::Bencher) {
    bench_edit_flush_set(bencher, make_large_doc(10 * 1024 * 1024));
}

#[divan::bench]
fn edit_flush_burst5_10mb(bencher: divan::Bencher) {
    let yaml = make_large_doc(10 * 1024 * 1024);
    let ast = parse_doc(&yaml);
    let source: Arc<str> = Arc::from(yaml);
    let targets = [
        ("group_000", "key_0000"),
        ("group_000", "key_0001"),
        ("group_001", "key_0000"),
        ("group_001", "key_0001"),
        ("group_002", "key_0000"),
    ];
    let new_value = CustomNode::plain_scalar("zzz");
    bencher.bench(|| {
        let mut a = ast.clone();
        let mut state = SpliceState::new(source.clone());
        for (g, k) in &targets {
            let segs = vec![key(g), key(k)];
            if let Ok(unit) =
                editing::set_path(&mut a, &segs, new_value.clone(), true, &source, None, false)
                && unit.eligible
            {
                state.apply(&unit).ok();
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
    bench_serialize(bencher, make_complex_doc(2 * 1024 * 1024));
}

#[divan::bench]
fn serialize_with_clone_complex_2mb(bencher: divan::Bencher) {
    bench_serialize_with_clone(bencher, make_complex_doc(2 * 1024 * 1024));
}

#[divan::bench]
fn edit_flush_set_complex_2mb(bencher: divan::Bencher) {
    bench_edit_flush_set(bencher, make_complex_doc(2 * 1024 * 1024));
}

// ── D4: Walk/Scalars benchmarks (Rust-backed AST traversal) ──

#[divan::bench]
fn walk_small(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::walk(&ast));
}

#[divan::bench]
fn scalars_small(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::scalars(&ast));
}

#[divan::bench]
fn walk_medium(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::walk(&ast));
}

#[divan::bench]
fn scalars_medium(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::scalars(&ast));
}

#[divan::bench]
fn walk_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::walk(&ast));
}

#[divan::bench]
fn scalars_large(bencher: divan::Bencher) {
    let ast = pyrs_yaml::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    bencher.bench(|| pyrs_yaml::ast::scalars(&ast));
}
