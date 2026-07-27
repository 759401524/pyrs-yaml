use criterion::{criterion_group, criterion_main, Criterion};
use pyyaml_rs::parser::yaml::YamlSchema;

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

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    group.bench_function("small", |b| {
        b.iter(|| pyyaml_rs::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap());
    });
    group.bench_function("medium", |b| {
        b.iter(|| pyyaml_rs::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap());
    });
    group.bench_function("large", |b| {
        b.iter(|| pyyaml_rs::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap());
    });
    group.bench_function("anchors", |b| {
        b.iter(|| pyyaml_rs::parser::parse(ANCHOR_YAML, YamlSchema::Core).unwrap());
    });
    group.bench_function("comments", |b| {
        b.iter(|| pyyaml_rs::parser::parse(COMMENT_YAML, YamlSchema::Core).unwrap());
    });
    group.bench_function("block_scalars", |b| {
        b.iter(|| pyyaml_rs::parser::parse(BLOCK_SCALAR_YAML, YamlSchema::Core).unwrap());
    });
    group.finish();
}

fn bench_serialize(c: &mut Criterion) {
    let small_ast = pyyaml_rs::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
    let medium_ast = pyyaml_rs::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
    let large_ast = pyyaml_rs::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
    let anchor_ast = pyyaml_rs::parser::parse(ANCHOR_YAML, YamlSchema::Core).unwrap();
    let block_ast = pyyaml_rs::parser::parse(BLOCK_SCALAR_YAML, YamlSchema::Core).unwrap();

    let mut group = c.benchmark_group("serialize");
    group.bench_function("small", |b| {
        b.iter(|| pyyaml_rs::serializer::to_yaml(&small_ast));
    });
    group.bench_function("medium", |b| {
        b.iter(|| pyyaml_rs::serializer::to_yaml(&medium_ast));
    });
    group.bench_function("large", |b| {
        b.iter(|| pyyaml_rs::serializer::to_yaml(&large_ast));
    });
    group.bench_function("anchors", |b| {
        b.iter(|| pyyaml_rs::serializer::to_yaml(&anchor_ast));
    });
    group.bench_function("block_scalars", |b| {
        b.iter(|| pyyaml_rs::serializer::to_yaml(&block_ast));
    });
    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");
    group.bench_function("small", |b| {
        b.iter(|| {
            let ast = pyyaml_rs::parser::parse(SMALL_YAML, YamlSchema::Core).unwrap();
            pyyaml_rs::serializer::to_yaml(&ast)
        });
    });
    group.bench_function("medium", |b| {
        b.iter(|| {
            let ast = pyyaml_rs::parser::parse(MEDIUM_YAML, YamlSchema::Core).unwrap();
            pyyaml_rs::serializer::to_yaml(&ast)
        });
    });
    group.bench_function("large", |b| {
        b.iter(|| {
            let ast = pyyaml_rs::parser::parse(LARGE_YAML, YamlSchema::Core).unwrap();
            pyyaml_rs::serializer::to_yaml(&ast)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_parse, bench_serialize, bench_roundtrip);
criterion_main!(benches);
