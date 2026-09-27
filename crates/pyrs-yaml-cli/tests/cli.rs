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
    let (_, out, err) = run_with_stdin(&["from-toml", "-"], "s = \"true\"\nn = 42\n");
    assert_eq!(out, "s: \"true\"\nn: 42\n", "{err}");
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
