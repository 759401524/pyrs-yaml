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
    assert!(err.contains("no key nope"), "{err}");
}

#[test]
fn to_toml_rejects_null_with_stable_message() {
    let (code, _, err) = run_with_stdin(&["to-toml", "-"], "k: null\n");
    assert_eq!(code, Some(1));
    assert!(err.contains("toml-cannot-represent-null"), "{err}");
}
