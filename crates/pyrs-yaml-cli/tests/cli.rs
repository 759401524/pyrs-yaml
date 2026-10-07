//! End-to-end tests over the built `pyq` binary (CARGO_BIN_EXE_ harness).
//! Pins the behaviors smoke-tested at introduction: comment preservation,
//! JSON key order, scalar resolution parity with to_dict, INI/TOML import
//! and process exit codes.

use std::io::Write;
use std::process::{Command, Stdio};

fn pyq() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pyq"))
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = pyq().args(args).output().expect("pyq failed to spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_with_stdin(args: &[&str], input: &str) -> (Option<i32>, String, String) {
    let mut child = pyq()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pyq failed to spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("pyq failed");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn fmt_preserves_comments_and_order() {
    let (code, out, err) = run_with_stdin(&["fmt", "-"], "a: 1  # keep\nb:\n  - x\n  - y\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "a: 1  # keep\nb:\n  - x\n  - y\n");
}

#[test]
fn get_resolves_paths_with_negative_index() {
    let yaml = "servers:\n  - host: a\n  - host: b\n";
    let (_, out, _) = run_with_stdin(&["get", ".servers[-1].host", "-"], yaml);
    assert_eq!(out, "b\n");
}

#[test]
fn single_dot_is_the_whole_document() {
    let (code, out, err) = run_with_stdin(&["get", "."], "a: 1\nb: two\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "a: 1\nb: two\n");
}

#[test]
fn wildcard_streams_all_matches_as_yaml_docs() {
    let (code, out, err) = run_with_stdin(&["get", ".a.*"], "a:\n  x: 1\n  y: two\n");
    assert_eq!(code, Some(0), "{err}");
    // one document per match, mapping key order preserved
    assert_eq!(out, "---\n1\n---\ntwo\n");
}

#[test]
fn verbs_pipeline_select_sort_unique_slice() {
    let doc =
        "s:\n  - {name: c, port: 1800}\n  - {name: a, port: 900}\n  - {name: b, port: 1500}\n";
    // select gates the stream, sort orders it, --json streams one value/line
    let (code, out, err) = run_with_stdin(
        &[
            "get",
            ".s[*]",
            "--select",
            "port >= 1000",
            "--sort-by",
            "name",
            "--json",
            "-",
        ],
        doc,
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        "{\"name\":\"b\",\"port\":1500}\n{\"name\":\"c\",\"port\":1800}\n"
    );
    // desc reverses, take slices
    let (_, out, _) = run_with_stdin(
        &[
            "get",
            ".s[*]",
            "--sort-by",
            "name",
            "--desc",
            "--take",
            "1",
            "--json",
            "-",
        ],
        doc,
    );
    assert_eq!(out, "{\n  \"name\": \"c\",\n  \"port\": 1800\n}\n");
}

#[test]
fn verbs_join_and_errors() {
    let (_, out, _) = run_with_stdin(
        &["get", ".s[*]", "--join", ",", "--raw", "-"],
        "s:\n  - one\n  - two\n",
    );
    assert_eq!(out, "one,two\n");
    // join over mappings is rejected
    let (code, _, err) = run_with_stdin(&["get", ".s[*]", "--join", ",", "-"], "s:\n  - {a: 1}\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("--join needs an all-scalar stream"), "{err}");
    // unparseable predicate
    let (code, _, err) = run_with_stdin(&["get", ".s[*]", "--select", "bogus", "-"], "s: []\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("invalid predicate: bogus"), "{err}");
    // filters emptied the stream: distinct from a plain path miss
    let (code, _, err) = run_with_stdin(
        &["get", ".s[*]", "--select", "port > 1000", "-"],
        "s:\n  - {port: 1}\n",
    );
    assert_eq!(code, Some(1));
    assert!(err.contains("no matches after filters: .s[*]"), "{err}");
}

#[test]
fn wildcard_json_streams_one_value_per_line() {
    let (code, out, err) = run_with_stdin(
        &["get", ".servers[*].port", "--json"],
        "servers:\n  - port: 1\n  - port: 2\n",
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "1\n2\n");
}

#[test]
fn to_json_preserves_key_order_and_types() {
    // Input via YAML (superset): order and typed values must survive.
    let yaml = "z: 1\na: \"two\"\nm: {k: [1, null, true]}\n";
    let (_, out, _) = run_with_stdin(&["to-json", "-"], yaml);
    let compact: String = out
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\n')
        .collect();
    assert_eq!(
        compact, r#"{"z":1,"a":"two","m":{"k":[1,null,true]}}"#,
        "{out}"
    );
}

#[test]
fn json_input_roundtrips_unchanged() {
    let doc = r#"{"k": "true", "n": 42, "deep": {"arr": [1, null]}}"#;
    let (_, out, _) = run_with_stdin(&["to-json", "--input", "json", "-"], doc);
    let compact: String = out.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(compact, doc.replace(' ', ""));
}

#[test]
fn from_toml_emits_yaml_with_minimal_quoting() {
    let (_, out, err) = run_with_stdin(&["from-toml", "-"], "s = \"true\"\nport = 42\nn = 7\n");
    // Minimal, not blanket: `port` needs nothing and is plain. Two things do need
    // quotes, because an unquoted spelling would be a *different value* to a YAML
    // reader - `s`'s TOML string `"true"`, and the key `n`, which YAML 1.1 types as a
    // bool/null. A conversion may not change what a document means, so a key is quoted
    // by exactly the rule a value is.
    assert_eq!(out, "s: \"true\"\nport: 42\n\"n\": 7\n", "{err}");
}

#[test]
fn from_ini_keeps_values_as_strings() {
    let (_, out, _) = run_with_stdin(&["from-ini", "-"], "[srv]\nHost = 127.0.0.1\nPort = 8080\n");
    assert_eq!(
        out,
        "srv:\n  \"Host\": \"127.0.0.1\"\n  \"Port\": \"8080\"\n"
    );
}

#[test]
fn raw_flag_prints_bare_scalar() {
    let (_, out, _) = run_with_stdin(&["get", "--raw", ".a", "-"], "a: hello\n");
    assert_eq!(out, "hello\n");
}

#[test]
fn file_extension_drives_auto_input_format() {
    // A .toml path must be ingested as TOML without --input; also covers
    // the file-argument path of the harness (run()).
    let dir = std::env::temp_dir().join(format!("pyq-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("cfg.toml");
    std::fs::write(&f, "x = 1\n").unwrap();
    let (code, out, err) = run(&["to-json", f.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "{err}");
    let compact: String = out.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(compact, r#"{"x":1}"#);
}

#[test]
fn missing_path_exits_nonzero() {
    let (code, _, err) = run_with_stdin(&["get", ".nope", "-"], "a: 1\n");
    assert_eq!(code, Some(1));
    // get reports the jq-style unified wording with the full path
    assert!(err.contains("path not found: .nope"), "{err}");
}

#[test]
fn to_toml_rejects_null_with_stable_message() {
    let (code, _, err) = run_with_stdin(&["to-toml", "-"], "k: null\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("toml-cannot-represent-null"), "{err}");
}

#[test]
fn set_overwrites_value_and_keeps_comments() {
    let input = "a: 1  # keep\nb:\n  c: 2\n";
    let (_, out, err) = run_with_stdin(&["set", ".b.c", "99", "-"], input);
    assert_eq!(out, "a: 1  # keep\nb:\n  c: 99\n", "{err}");
}

#[test]
fn set_parses_typed_and_structured_values() {
    // Round-trip style fidelity: the value's own source style is kept,
    // exactly like `fmt` preserves layout.
    let (_, out, _) = run_with_stdin(&["set", ".x", "[1, two]", "-"], "x: 0\n");
    assert_eq!(out, "x: [1, two]\n");
    // quoted YAML value stays a string through the round-trip
    let (_, out, _) = run_with_stdin(&["set", ".x", "\"true\"", "-"], "x: 0\n");
    assert_eq!(out, "x: \"true\"\n");
}

#[test]
fn create_missing_grows_intermediate_mappings() {
    let (_, out, err) =
        run_with_stdin(&["set", "--create-missing", ".a.b.c", "1", "-"], "top: 0\n");
    assert_eq!(out, "top: 0\na:\n  b:\n    c: 1\n", "{err}");
    // without the flag the missing intermediate errors
    let (code, _, err) = run_with_stdin(&["set", ".a.b.c", "1", "-"], "top: 0\n");
    assert_eq!(code, Some(1), "{err}");
}

#[test]
fn set_replaces_whole_document_at_root() {
    // Flow input keeps flow style on output (round-trip fidelity).
    let (_, out, _) = run_with_stdin(&["set", "$", "{a: 1}", "-"], "old: stuff\n");
    assert_eq!(out, "{a: 1}\n");
    // block-style replacement value emits block
    let (_, out, _) = run_with_stdin(&["set", "$", "a: 1", "-"], "old: stuff\n");
    assert_eq!(out, "a: 1\n");
}

#[test]
fn delete_key_and_negative_index() {
    let input = "a: 1\nb: 2\nlist:\n  - x\n  - y\n";
    let (_, out, _) = run_with_stdin(&["delete", ".a", "-"], input);
    assert_eq!(out, "b: 2\nlist:\n  - x\n  - y\n");
    let (_, out, _) = run_with_stdin(&["delete", ".list[-2]", "-"], input);
    assert_eq!(out, "a: 1\nb: 2\nlist:\n  - y\n");
}

#[test]
fn delete_missing_key_exits_nonzero() {
    let (code, _, err) = run_with_stdin(&["delete", ".nope", "-"], "a: 1\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("path not found: .nope"), "{err}");
}

#[test]
fn sort_keys_root_and_path() {
    let (code, out, err) = run_with_stdin(&["sort-keys", "$", "-"], "b: 2\na:\n  z: 1\n  y: 2\n");
    assert_eq!(code, Some(0), "{err}");
    // one level: root sorted, nested mapping keeps its order
    assert_eq!(out, "a:\n  z: 1\n  y: 2\nb: 2\n");
    let (_, out, _) = run_with_stdin(&["sort-keys", ".a", "-"], "b: 2\na:\n  z: 1\n  y: 2\n");
    assert_eq!(out, "b: 2\na:\n  y: 2\n  z: 1\n");
}

#[test]
fn sort_keys_preserves_comments() {
    let (code, out, err) = run_with_stdin(&["sort-keys", "$", "-"], "# note\nz: 1\na: 2\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "# note\na: 2\nz: 1\n");
}

#[test]
fn completion_emits_usable_scripts() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let (code, out, err) = run(&["completion", shell]);
        assert_eq!(code, Some(0), "{shell}: {err}");
        assert!(out.contains("pyq"), "{shell} script should name the binary");
    }
    let (code, _, err) = run(&["completion", "ksh"]);
    assert_eq!(code, Some(2));
    assert!(err.contains("invalid value"), "{err}");
}

#[test]
fn rename_keeps_value_and_comments() {
    let (code, out, err) = run_with_stdin(&["rename", ".a", "b", "-"], "a: 1  # keep\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "b: 1  # keep\n");
}

#[test]
fn move_relocates_subtree_to_existing_destination() {
    let doc = "a:\n  x: 1\nb:\n  y: 2\n";
    let (code, out, err) = run_with_stdin(&["move", ".a.x", ".b.x", "-"], doc);
    assert_eq!(code, Some(0), "{err}");
    // both splice units land: destination gains the node, the source line
    // disappears leaving `a:` (batch fix in move_path)
    assert_eq!(out, "a:\nb:\n  y: 2\n  x: 1\n");
}

#[test]
fn append_and_insert_sequences() {
    let doc = "s:\n  - one\n  - two\n";
    let (_, out, _) = run_with_stdin(&["append", ".s", "three", "-"], doc);
    assert_eq!(out, "s:\n  - one\n  - two\n  - three\n");
    let (_, out, _) = run_with_stdin(&["insert", ".s", "1", "zero", "-"], doc);
    assert_eq!(out, "s:\n  - one\n  - zero\n  - two\n");
}

#[test]
fn validate_reports_parse_and_schema_failures() {
    let (code, out, _) = run_with_stdin(&["validate", "-"], "a: 1\n");
    assert_eq!(code, Some(0));
    assert_eq!(out, "ok\n");
    let (code, _, err) = run_with_stdin(&["validate", "-"], "b: [1,\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("unclosed bracket"), "{err}");

    let dir = std::env::temp_dir().join(format!("pyq-val-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("rules.yaml");
    std::fs::write(
        &rules,
        "validate:\n  - path: $.name\n    type: str\n    required: true\n",
    )
    .unwrap();
    let (code, out, _) = run_with_stdin(
        &["validate", "--schema", rules.to_str().unwrap(), "-"],
        "name: hello\n",
    );
    assert_eq!(code, Some(0));
    assert_eq!(out, "ok\n");
    let (code, stdout, stderr) = run_with_stdin(
        &["validate", "--schema", rules.to_str().unwrap(), "-"],
        "other: 1\n",
    );
    assert_eq!(code, Some(1));
    assert!(
        stdout.contains("$.name: required path is missing"),
        "{stdout}"
    );
    assert!(stderr.contains("validation failed"), "{stderr}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn frontmatter_extracts_yaml_and_splits_body() {
    let dir = std::env::temp_dir().join(format!("pyq-fm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let page = dir.join("page.md");
    let body = dir.join("body.md");
    std::fs::write(&page, "---\ntitle: Hi\ntags: [a, b]\n---\n# Body here\n").unwrap();
    let (code, out, err) = run(&[
        "frontmatter",
        page.to_str().unwrap(),
        "--body-out",
        body.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "title: Hi\ntags: [a, b]\n");
    assert_eq!(std::fs::read_to_string(&body).unwrap(), "# Body here\n");
    // a plain YAML file has no front matter
    let (code, _, err) = run_with_stdin(&["frontmatter", "-"], "a: 1\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("no front matter"), "{err}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn all_docs_get_streams_matches_and_skips_misses() {
    let stream = "---\nname: doc1\n---\nname: doc2\n---\nother: x\n";
    let (code, out, err) = run_with_stdin(&["get", ".name", "-A", "-"], stream);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "---\ndoc1\n---\ndoc2\n");
    // fmt -A normalizes every document
    let (_, out, _) = run_with_stdin(&["fmt", "-A", "-"], "---\nb: 1\n---\na: 2\n");
    assert_eq!(out, "---\nb: 1\n---\na: 2\n");
}

#[test]
fn all_docs_set_edits_every_hit_and_pins_the_rest() {
    let stream = "---\nname: one\nweird:   1\n---\nname: two\n";
    let (code, out, err) = run_with_stdin(&["set", "-A", ".tag", "9", "-"], stream);
    assert_eq!(code, Some(0), "{err}");
    // doc 0 keeps its odd spacing verbatim around the spliced line
    assert!(out.contains("name: one\nweird:   1\ntag: 9"), "{out:?}");
    assert!(out.contains("name: two\ntag: 9"), "{out:?}");
    // the stream still round-trips as two documents with both edits
    let (_, check, _) = run_with_stdin(&["get", ".tag", "-A", "--json", "-"], &out);
    assert_eq!(check, "9\n9\n");
}

#[test]
fn all_docs_delete_skips_docs_without_the_path() {
    let stream = "---\na: 1\nkeep: y\n---\nb: 2\n";
    let (code, out, err) = run_with_stdin(&["delete", "-A", ".a", "-"], stream);
    assert_eq!(code, Some(0), "{err}");
    // doc 1 edited (last-key removal renders {} like single-doc), doc 2
    // byte-identical including its separator prelude
    assert!(out.contains("keep: y\n"), "{out:?}");
    assert!(out.contains("---\nb: 2\n"), "{out:?}");
    // all-miss is still an error
    let (code, _, err) = run_with_stdin(&["delete", "-A", ".zzz", "-"], stream);
    assert_eq!(code, Some(1));
    assert!(err.contains("path not found: .zzz"), "{err}");
}

#[test]
fn all_docs_create_missing_grows_every_document() {
    let (code, out, err) = run_with_stdin(
        &["set", "-A", "--create-missing", ".m.n", "7", "-"],
        "---\nx: 1\n---\ny: 2\n",
    );
    assert_eq!(code, Some(0), "{err}");
    let (_, check, _) = run_with_stdin(&["get", ".m.n", "-A", "--json", "-"], &out);
    assert_eq!(check, "7\n7\n");
}

#[test]
fn to_json_all_docs_emits_a_json_array() {
    let (code, out, err) = run_with_stdin(&["to-json", "-A", "-"], "---\na: 1\n---\nb: two\n");
    assert_eq!(code, Some(0), "{err}");
    let compact: String = out.chars().filter(|c| !matches!(c, ' ' | '\n')).collect();
    assert_eq!(compact, "[{\"a\":1},{\"b\":\"two\"}]");
}

#[test]
fn edit_pins_layout_of_untouched_lines() {
    // The splice engine rewrites only the edited region: standalone notes
    // and odd inline spacing survive verbatim - the serializer alone
    // would normalize `demo   # inline` to two spaces. This is the
    // Python CLI's layout-pinning guarantee, now shared through the
    // core plan engine.
    let input = "# header\napp:\n  name: demo   # inline\n# trailing note\nport: 1\n";
    let (code, out, err) = run_with_stdin(&["set", ".port", "2", "-"], input);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        "# header\napp:\n  name: demo   # inline\n# trailing note\nport: 2\n"
    );
}

#[test]
fn inplace_rewrites_the_file() {
    let dir = std::env::temp_dir().join(format!("pyq-ip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("cfg.yaml");
    std::fs::write(&f, "a: 1  # keep\n").unwrap();
    let (code, _, err) = run(&["set", "--inplace", ".b", "2", f.to_str().unwrap()]);
    let content = std::fs::read_to_string(&f).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(content, "a: 1  # keep\nb: 2\n");
}

#[test]
fn fmt_indent_flag_controls_block_indent() {
    let (code, out, err) = run_with_stdin(&["fmt", "--indent", "4", "-"], "a:\n  b:\n    c: 1\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "a:\n    b:\n        c: 1\n", "{out:?}");
}

#[test]
fn fmt_sort_keys_orders_every_mapping() {
    let (code, out, err) =
        run_with_stdin(&["fmt", "--sort-keys", "-"], "b: 1\na:\n  d: 2\n  c: 3\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "a:\n  c: 3\n  d: 2\nb: 1\n", "{out:?}");
}

#[test]
fn fmt_inplace_rewrites_the_file() {
    let dir = std::env::temp_dir().join(format!("pyq-fmtip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("cfg.yaml");
    std::fs::write(&f, "a:\n  b: 1\n").unwrap();
    let (code, _, err) = run(&["fmt", "--indent", "4", "--inplace", f.to_str().unwrap()]);
    let content = std::fs::read_to_string(&f).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(content, "a:\n    b: 1\n", "{content:?}");
}

#[test]
fn to_json_jsonc_emits_carried_comments() {
    // YAML leading comment rides the AST `leading_comment` slot and the
    // JSONC writer re-emits it as a `//` note (#122 round-trip).
    let (code, out, err) = run_with_stdin(&["to-json", "--jsonc", "-"], "# note\na: 1\n");
    assert_eq!(code, Some(0), "{err}");
    assert!(out.starts_with("// note\n"), "{out:?}");
    assert!(out.contains("\"a\": 1"), "{out:?}");
    // plain JSON output still drops it (strict RFC 8259)
    let (_, plain, _) = run_with_stdin(&["to-json", "-"], "# note\na: 1\n");
    assert!(!plain.contains("note"), "{plain:?}");
}

#[test]
fn to_json_json5_restores_single_quotes() {
    // YAML 'x' parses to a SingleQuoted scalar; the JSON5 writer restores
    // the spelling (PR #121 dialect fidelity). Keys stay double-quoted here:
    // unquoted identifier keys are only restored for JSON5-sourced spellings.
    let (code, out, err) =
        run_with_stdin(&["to-json", "--json5", "--indent", "0", "-"], "b: 'x'\n");
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.trim(), r#"{"b":'x'}"#, "{out:?}");
}

#[test]
fn to_json_dialect_flags_conflict() {
    let (code, _, err) = run_with_stdin(&["to-json", "--jsonc", "--json5", "-"], "a: 1\n");
    assert_eq!(code, Some(2));
    assert!(err.contains("cannot be used with"), "{err}");
}

#[test]
fn input_jsonc_extracts_values_past_comments() {
    // `// note` is not YAML-representable; only the native JSONC dialect
    // path can read this input at all.
    let (code, out, err) = run_with_stdin(
        &["get", "--input", "jsonc", ".a.b", "-"],
        "// note\n{\"a\": {\"b\": 7}}\n",
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "7\n", "{out:?}");
}

#[test]
fn input_jsonc_round_trips_comments_to_jsonc() {
    // comments ride the AST slots from the JSONC parser straight into the
    // JSONC writer (#122 fidelity, now reachable from one command).
    let (code, out, err) = run_with_stdin(
        &["to-json", "--input", "jsonc", "--jsonc", "-"],
        "// keep me\n{\"a\": 1}\n",
    );
    assert_eq!(code, Some(0), "{err}");
    assert!(out.starts_with("// keep me\n"), "{out:?}");
    assert!(out.contains("\"a\": 1"), "{out:?}");
}

#[test]
fn input_json5_restores_spellings() {
    // JSON5 source spellings (single-quoted key + value, trailing comma)
    // survive parse -> re-emit through the dialect writers.
    let (code, out, err) = run_with_stdin(
        &[
            "to-json", "--input", "json5", "--json5", "--indent", "0", "-",
        ],
        "{name: 'x', v: .5,}\n",
    );
    assert_eq!(code, Some(0), "{err}");
    let body = out.trim();
    assert!(body.contains("'x'"), "{body:?}");
    assert!(body.contains(".5"), "{body:?}");
}

#[test]
fn auto_detects_jsonc_and_json5_extensions() {
    let dir = std::env::temp_dir().join(format!("pyq-dialect-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let c = dir.join("cfg.jsonc");
    let five = dir.join("old.json5");
    std::fs::write(&c, "// hi\n{\"a\": 1}\n").unwrap();
    std::fs::write(&five, "{a: 'z',}\n").unwrap();
    let (code1, out1, err1) = run(&["get", ".a", c.to_str().unwrap()]);
    let (code2, out2, err2) = run(&["get", ".a", five.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code1, Some(0), "{err1}");
    assert_eq!(out1, "1\n", "{out1:?}");
    assert_eq!(code2, Some(0), "{err2}");
    // the JSON5 single-quote style rides the AST into the YAML emission
    assert_eq!(out2, "'z'\n", "{out2:?}");
}

#[test]
fn all_docs_rejects_dialect_inputs() {
    let (code, _, err) = run_with_stdin(&["to-json", "-A", "--input", "jsonc", "-"], "{\"a\": 1}");
    assert_eq!(code, Some(1));
    assert!(
        err.contains("--all-docs only applies to YAML or JSON"),
        "{err}"
    );
}
