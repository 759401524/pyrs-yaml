//! End-to-end tests for `pyq diff` / `pyq merge` (semantic compare and
//! right-biased deep merge) over the built binary.

use std::path::PathBuf;
use std::process::Command;

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_pyq"))
        .args(args)
        .output()
        .expect("pyq failed to spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Scratch dir with `a.yaml`/`b.yaml` written; the caller removes it.
fn pair(tag: &str, left: &str, right: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pyq-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.yaml"), left).unwrap();
    std::fs::write(dir.join("b.yaml"), right).unwrap();
    dir
}

fn diff(dir: &PathBuf, extra: &[&str]) -> (Option<i32>, String, String) {
    let a = dir.join("a.yaml");
    let b = dir.join("b.yaml");
    let mut args = vec!["diff"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&[a.to_str().unwrap(), b.to_str().unwrap()]);
    let r = run(&args);
    let _ = std::fs::remove_dir_all(dir);
    r
}

fn merge(dir: &PathBuf, extra: &[&str]) -> (Option<i32>, String, String) {
    let a = dir.join("a.yaml");
    let b = dir.join("b.yaml");
    let mut args = vec!["merge"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&[a.to_str().unwrap(), b.to_str().unwrap()]);
    let r = run(&args);
    let _ = std::fs::remove_dir_all(dir);
    r
}

#[test]
fn diff_ignores_comments_and_layout() {
    let dir = pair(
        "diff-eq",
        "# note\nserver:\n  port: 8080   # inline\n",
        "server:\n    port: 8080\n",
    );
    let (code, out, err) = diff(&dir, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "", "{out:?}");
}

#[test]
fn diff_reports_changed_added_removed_paths() {
    let dir = pair("diff-3", "a: 1\nb: 2\nkeep: x\n", "a: 1\nb: 3\nc: 4\n");
    let (code, out, _) = diff(&dir, &[]);
    assert_eq!(code, Some(1));
    assert_eq!(out, "~ .b: 2 -> 3\n- .keep: x\n+ .c: 4\n", "{out:?}");
}

#[test]
fn diff_is_semantic_not_textual() {
    // quoted "1" is a string, plain 1 an int: different under the Core
    // schema even though both render as `1`.
    let dir = pair("diff-sem", "a: \"1\"\n", "a: 1\n");
    let (code, out, _) = diff(&dir, &[]);
    assert_eq!(code, Some(1));
    assert!(out.starts_with("~ .a: "), "{out:?}");
}

#[test]
fn diff_walks_sequences_by_index() {
    let dir = pair(
        "diff-seq",
        "l:\n  - 1\n  - 2\n  - 3\n",
        "l:\n  - 1\n  - 9\n",
    );
    let (code, out, _) = diff(&dir, &[]);
    assert_eq!(code, Some(1));
    assert_eq!(out, "~ .l[1]: 2 -> 9\n- .l[2]: 3\n", "{out:?}");
}

#[test]
fn merge_deep_overlays_and_appends_arrays() {
    let dir = pair(
        "merge-app",
        "db:\n  host: local\n  port: 5432\ntags:\n  - a\n",
        "db:\n  port: 5433\n  ssl: true\ntags:\n  - b\nextra: 1\n",
    );
    let (code, out, err) = merge(&dir, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("host: local"), "{out:?}");
    assert!(out.contains("port: 5433"), "{out:?}");
    assert!(out.contains("ssl: true"), "{out:?}");
    assert!(out.contains("extra: 1"), "{out:?}");
    // sequences append (yq `*+` shape): a then b
    let tags: Vec<&str> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- "))
        .collect();
    assert_eq!(tags, vec!["a", "b"], "{out:?}");
}

#[test]
fn merge_replace_arrays_swaps_wholesale() {
    let dir = pair("merge-rep", "tags:\n  - a\n  - z\n", "tags:\n  - b\n");
    let (code, out, err) = merge(&dir, &["--replace-arrays"]);
    assert_eq!(code, Some(0), "{err}");
    let tags: Vec<&str> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- "))
        .collect();
    assert_eq!(tags, vec!["b"], "{out:?}");
}

#[test]
fn merge_overlays_scalar_conflict_right_wins() {
    // both files on the YAML path; right document wins the conflict and
    // brings a new key along.
    let dir = std::env::temp_dir().join(format!("pyq-merge-x-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.yaml"), "port: 80\n").unwrap();
    std::fs::write(dir.join("b.yaml"), "port: 443\ntls: on\n").unwrap();
    let a = dir.join("a.yaml");
    let b = dir.join("b.yaml");
    let (code, out, err) = run(&["merge", a.to_str().unwrap(), b.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("port: 443"), "{out:?}");
    assert!(out.contains("tls: on"), "{out:?}");
}
