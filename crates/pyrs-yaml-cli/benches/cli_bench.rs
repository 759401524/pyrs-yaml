//! CLI-component benchmarks (CodSpeed-collected alongside yaml_bench).
//!
//! These measure the work `pyq` adds on top of the core engine - format
//! dispatch, the JSON/INI projections, the path selector and in-place
//! mutations - so regressions in the CLI layer show up as CLI lines in
//! the flamegraph, not as noise inside the core parse benches.

use divan::black_box;
use pyrs_yaml_cli::{Format, InputOpts, SchemaKind, ini_to_node, json, load_source, paths};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::editing::plan;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::{parser, serializer, toml};

fn main() {
    divan::main();
}

/// ~10 KB config document mirroring the shape of real-world k8s/compose files.
fn big_yaml() -> String {
    let mut s = String::from("meta:\n  name: bench\n  generated: true\nservices:\n");
    for i in 0..80 {
        s.push_str(&format!(
            "  svc_{i}:\n    image: repo/svc:{i}\n    port: {}\n    args:\n      - --flag={i}\n      - --workers=4\n    enabled: true\n    ratio: 0.{i:0>2}\n",
            8000 + i
        ));
    }
    s
}

fn big_json() -> String {
    let items: Vec<String> = (0..80)
        .map(|i| {
            format!(
                "{{\"id\": \"svc-{i}\", \"port\": {p}, \"tags\": [\"a{i}\", \"b{i}\"], \"replicas\": {r}, \"enabled\": {e}}}",
                p = 8000 + i,
                r = i % 5,
                e = if i % 2 == 0 { "true" } else { "false" }
            )
        })
        .collect();
    format!(
        r#"{{"meta": {{"name": "bench"}}, "services": [{items}]}}"#,
        items = items.join(",")
    )
}

fn big_ini() -> String {
    let mut s = String::from("[global]\nworkers = 4\nkeepalive: yes\n");
    for i in 0..80 {
        s.push_str(&format!(
            "\n[service_{i}]\nhost = 10.0.{i}.1\nport = {p}\ndescription = sample service {i}\nenabled = true\n",
            p = 8000 + i
        ));
    }
    s
}

fn big_toml_doc() -> String {
    let mut s = String::from("[meta]\nname = \"bench\"\ngenerated = true\n");
    for i in 0..60 {
        s.push_str(&format!(
            "\n[[services]]\nname = \"svc-{i}\"\nport = {p}\nreplicas = {r}\nenabled = {e}\n",
            p = 8000 + i,
            r = i % 5,
            e = i % 2 == 0
        ));
    }
    s
}

fn opts(input: Format) -> InputOpts {
    InputOpts {
        input,
        schema: SchemaKind::Core,
    }
}

// ── format dispatch (load_source) ──

#[divan::bench]
fn load_yaml_big() -> CustomNode {
    let src = big_yaml();
    black_box(load_source(black_box(&src), None, &opts(Format::Yaml)).unwrap())
}

#[divan::bench]
fn load_json_via_yaml_superset() -> CustomNode {
    let src = big_json();
    black_box(load_source(black_box(&src), None, &opts(Format::Yaml)).unwrap())
}

#[divan::bench]
fn load_toml_big() -> CustomNode {
    let src = big_toml_doc();
    black_box(load_source(black_box(&src), None, &opts(Format::Toml)).unwrap())
}

#[divan::bench]
fn load_ini_big() -> CustomNode {
    let src = big_ini();
    black_box(load_source(black_box(&src), None, &opts(Format::Ini)).unwrap())
}

// ── projections ──

#[divan::bench]
fn ini_to_node_big() -> CustomNode {
    let src = big_ini();
    black_box(ini_to_node(black_box(&src)).unwrap())
}

#[divan::bench]
fn json_to_node_big() -> CustomNode {
    let src = big_json();
    black_box(json::json_to_node(black_box(&src)).unwrap())
}

#[divan::bench]
fn node_to_json_big() -> String {
    let src = big_json();
    let node = json::json_to_node(&src).unwrap();
    black_box(json::node_to_json(black_box(&node)).unwrap())
}

#[divan::bench]
fn yaml_to_toml_big() -> String {
    let src = big_toml_doc();
    let node = toml::from_toml(&src).unwrap();
    black_box(toml::to_toml(black_box(&node)).unwrap())
}

// ── selector ──

#[divan::bench(args = [".meta.name", ".services.svc_41.args[0]", ".services.svc_77.port"])]
fn select_path(path: &str) -> usize {
    let src = big_yaml();
    let node = parser::parse(&src, Schema::Core).unwrap();
    let sel = paths::parse_path(path).unwrap();
    let found = sel.select(&node).ok().flatten();
    black_box(found.map_or(0, |n| match n {
        CustomNode::Scalar { value, .. } => value.len(),
        _ => 1,
    }))
}

#[divan::bench]
fn path_parse_deep() -> usize {
    let s = ".a.b.c.d.e.f.g.h.i.j[3]";
    black_box(paths::parse_path(black_box(s)).unwrap().segments_len())
}

// ── in-place mutations (set/delete walking) ──

#[divan::bench]
fn set_deep_key() -> usize {
    let src = big_yaml();
    let mut node = parser::parse(&src, Schema::Core).unwrap();
    let value = parser::parse("99", Schema::Core).unwrap();
    paths::parse_path(".services.svc_40.port")
        .unwrap()
        .set_at(&mut node, value, false)
        .unwrap();
    black_box(serializer::to_yaml(&black_box(node)).len())
}

#[divan::bench]
fn delete_middle_key() -> usize {
    let src = big_yaml();
    let mut node = parser::parse(&src, Schema::Core).unwrap();
    paths::parse_path(".services.svc_40")
        .unwrap()
        .delete_at(&mut node)
        .unwrap();
    black_box(serializer::to_yaml(&black_box(node)).len())
}

// ── verbs pipeline (post-selection stream processing) ──

#[divan::bench]
fn verbs_select_80_stream() -> usize {
    let src = big_yaml();
    let node = parser::parse(&src, Schema::Core).unwrap();
    let stream = paths::parse_path(".services.*").unwrap().select_all(&node);
    let verbs = pyrs_yaml_cli::verbs::Verbs {
        select: Some("port > 8040".into()),
        ..Default::default()
    };
    black_box(verbs.apply(stream, ".services.*").unwrap().len())
}

#[divan::bench]
fn verbs_sort_by_80_stream() -> usize {
    let src = big_yaml();
    let node = parser::parse(&src, Schema::Core).unwrap();
    let stream = paths::parse_path(".services.*").unwrap().select_all(&node);
    let verbs = pyrs_yaml_cli::verbs::Verbs {
        sort_by: Some(".port".into()),
        ..Default::default()
    };
    black_box(verbs.apply(stream, ".services.*").unwrap().len())
}

#[divan::bench]
fn verbs_unique_over_dups() -> usize {
    let src = big_yaml();
    let node = parser::parse(&src, Schema::Core).unwrap();
    let mut stream = paths::parse_path(".services.*").unwrap().select_all(&node);
    stream.extend(stream.clone()); // force real dedup work
    let verbs = pyrs_yaml_cli::verbs::Verbs {
        unique: true,
        ..Default::default()
    };
    black_box(verbs.apply(stream, ".services.*").unwrap().len())
}

/// Eight-document stream; set touches every doc through the multi-doc
/// editor (segment derivation + per-doc eligibility + splice units).
#[divan::bench]
fn edit_all_docs_8x() -> usize {
    let mut src = String::new();
    for i in 0..8 {
        src.push_str(&format!("---\nsvc: s{i}\nport: {}\n", 8000 + i));
    }
    let mut docs = parser::parse_all(&src, Schema::Core).unwrap();
    let mut ed = pyrs_yaml_cli::multidoc::MultiDocEditor::new(&src, &docs);
    let seg = [pyrs_yaml_core::editing::Segment::Key(
        std::borrow::Cow::Borrowed("replicas"),
    )];
    let value = parser::parse("3", Schema::Core).unwrap();
    for (i, doc) in docs.iter_mut().enumerate() {
        ed.edit(i, doc, |node, offs| {
            plan::set_path(node, &seg, value.clone(), true, &src, offs, true).map(|u| vec![u])
        })
        .unwrap();
    }
    black_box(ed.finalize(&docs).len())
}
