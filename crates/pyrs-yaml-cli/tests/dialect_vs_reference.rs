//! The dialect shell is only worth having if an accepted invocation answers the
//! same question the real tool answers. This test runs the reference binaries when
//! this machine has them (`jq`, mikefarah `yq`) and compares stdout byte for byte;
//! it never compares our build against itself - the reference path is checked to
//! be outside this target directory, because the first version of this probe
//! silently found our own copy of `yq.exe` on `PATH` and reported nine perfect
//! matches that proved nothing.
//!
//! What is refused is asserted too: the shell must not answer at all.
//!
//! A reference has to be the tool it claims to be, because a binary named `yq` is not
//! evidence of mikefarah `yq` - the PyPI one is a jq wrapper with other semantics. So
//! each candidate's version banner is checked first, and `PYQ_DIALECT_YQ` /
//! `PYQ_DIALECT_JQ` bind a reference explicitly: a binding that is missing or answers
//! to another tool fails the leg instead of skipping it. Nothing in CI binds one yet,
//! which is registered as `dialect-tier:reference-unbound` in `.ci/quality-holes.json`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The version banner a reference gives, or the empty string when it will not run.
fn banner(program: &Path) -> String {
    Command::new(program)
        .arg("--version")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

/// Whether this binary is the tool the dialect claims to speak. Naming matters: the
/// `yq` on PyPI is a jq wrapper with other semantics and other exit codes, so
/// comparing against it would judge the wrong authority - and the first version of
/// this probe matched our own `yq.exe` sitting on `PATH`.
fn is_expected(tool: &str, text: &str) -> bool {
    match tool {
        "yq" => text.contains("mikefarah") && text.contains(" v4"),
        _ => text.starts_with("jq-") || text.contains("jq version"),
    }
}

/// An explicitly bound reference is a promise, so a broken promise is a failure
/// rather than a skip: the leg that means to compare must compare.
fn bound(tool: &str) -> Option<PathBuf> {
    let var = format!("PYQ_DIALECT_{}", tool.to_uppercase());
    let path = PathBuf::from(std::env::var_os(&var)?);
    let text = banner(&path);
    assert!(path.is_file(), "{var}={} is not a file", path.display());
    assert!(
        is_expected(tool, &text),
        "{var}={} is not the reference this tier compares against: {text:?} \
         (mikefarah yq v4 prints `... version v4.x`, jq prints `jq-1.x`)",
        path.display()
    );
    Some(path)
}

fn reference(tool: &str) -> Option<PathBuf> {
    if let Some(path) = bound(tool) {
        return Some(path);
    }
    let exe = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    };
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = dir.join(&exe);
        if candidate.is_file() {
            let real = candidate
                .canonicalize()
                .unwrap_or_else(|_| candidate.clone());
            let ours = Path::new(env!("CARGO_MANIFEST_DIR"));
            // `<workspace>/crates/pyrs-yaml-cli` -> the workspace root, which is
            // where every binary this test could mistake for the reference lives.
            let workspace = ours.parent().and_then(|p| p.parent());
            if let Some(root) = workspace
                && real.starts_with(root)
            {
                continue;
            }
            if is_expected(tool, &banner(&real)) {
                return Some(real);
            }
            eprintln!(
                "SKIP: {} looks like a {tool} but is not one: {:?}",
                real.display(),
                banner(&real).trim()
            );
            return None;
        }
    }
    None
}

fn fixture_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pyq-dialect-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    std::fs::write(
        dir.join("doc.yaml"),
        "# a comment the round trip must keep\na:\n  b: hello\n  n: 3\nlist:\n  - x: 1\n  - x: 2\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("doc.json"),
        "{\"a\": {\"b\": \"hello\", \"n\": 3}, \"list\": [{\"x\": 1}, {\"x\": 2}]}\n",
    )
    .unwrap();
    dir
}

fn run(program: &Path, args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("{} {args:?}: {e}", program.display()));
    // jq writes CRLF on Windows while this build writes LF; the comparison is about the
    // document, so a transport difference must not read as a dialect difference. The LF
    // rule for the repository itself is asserted by scripts/check_line_endings.py.
    let fold = |bytes: &[u8]| String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    (
        out.status.code().unwrap_or(-1),
        fold(&out.stdout),
        fold(&out.stderr),
    )
}

const YQ_ACCEPTED: &[&[&str]] = &[
    &[".a.b", "doc.yaml"],
    &[".a", "doc.yaml"],
    &[".", "doc.yaml"],
    &["del(.a.n)", "doc.yaml"],
    &[".a.n = 9", "doc.yaml"],
    &[".list[]", "doc.yaml"],
    &[".list[] | select(.x > 1)", "doc.yaml"],
    &["-N", ".list[]", "doc.yaml"],
    &["-r", ".a.b", "doc.yaml"],
    &["-o", "json", ".a", "doc.yaml"],
    &["-p", "json", ".a.b", "doc.json"],
    &[".list[0]", "doc.yaml"],
    &[".list[].x", "doc.yaml"],
    &[".a.n", "doc.yaml"],
    &["-o", "yaml", ".a", "doc.yaml"],
    &[".missing", "doc.yaml"],
];

const JQ_ACCEPTED: &[&[&str]] = &[
    &[".a", "doc.json"],
    &[".", "doc.json"],
    &["-r", ".a.b", "doc.json"],
    &[".list[0].x", "doc.json"],
    &[".list[]", "doc.json"],
    &["-r", ".list[].x", "doc.json"],
    &[".a[\"b\"]", "doc.json"],
    &[".missing", "doc.json"],
];

/// The forms the shell claims, answered the way the reference answers them.
#[test]
fn accepted_yq_invocations_match_mikefarah_yq() {
    let Some(yq) = reference("yq") else {
        eprintln!("SKIP: no mikefarah yq on PATH");
        return;
    };
    let dir = fixture_dir();
    let ours = Path::new(env!("CARGO_BIN_EXE_pyq"));
    let mut compared = 0usize;
    for args in YQ_ACCEPTED {
        let (mine_rc, mine_out, mine_err) =
            run(ours, &[["--as", "yq"].as_slice(), args].concat(), &dir);
        let (ref_rc, ref_out, ref_err) = run(&yq, args, &dir);
        assert_eq!(
            (ref_rc, ref_out.as_str()),
            (mine_rc, mine_out.as_str()),
            "`yq {args:?}` answered differently (ref stderr {ref_err:?}, ours {mine_err:?})"
        );
        compared += 1;
    }
    assert!(
        compared >= 14,
        "the table below fourteen forms proves nothing: {compared}"
    );
}

#[test]
fn accepted_jq_invocations_match_jq() {
    let Some(jq) = reference("jq") else {
        eprintln!("SKIP: no jq on PATH");
        return;
    };
    let dir = fixture_dir();
    let ours = Path::new(env!("CARGO_BIN_EXE_pyq"));
    for args in JQ_ACCEPTED {
        let (mine_rc, mine_out, mine_err) =
            run(ours, &[["--as", "jq"].as_slice(), args].concat(), &dir);
        let (ref_rc, ref_out, ref_err) = run(&jq, args, &dir);
        assert_eq!(
            (ref_rc, ref_out.as_str()),
            (mine_rc, mine_out.as_str()),
            "`jq {args:?}` answered differently (ref stderr {ref_err:?}, ours {mine_err:?})"
        );
    }
}

/// Everything else must be silent on stdout: a refusal that also printed a
/// document is the failure mode a drop-in shell cannot have.
#[test]
fn refused_invocations_print_nothing_and_name_the_construct() {
    let dir = fixture_dir();
    let ours = Path::new(env!("CARGO_BIN_EXE_pyq"));
    let cases: &[(&str, &[&str], &str)] = &[
        ("yq", &[".a | .b", "doc.yaml"], ".a | .b"),
        ("yq", &["map(.x)", "doc.yaml"], "map"),
        ("yq", &["keys", "doc.yaml"], "keys"),
        ("yq", &["length", "doc.yaml"], "length"),
        ("yq", &[".a.n |= 5", "doc.yaml"], "|="),
        ("yq", &["-e", ".a", "doc.yaml"], "-e"),
        ("yq", &["-P", ".a", "doc.yaml"], "-P"),
        ("yq", &["-n", ".a = 1"], "-n"),
        ("yq", &["-o", "csv", ".a", "doc.yaml"], "csv"),
        ("yq", &[".a", "doc.yaml", "doc.json"], "2 input files"),
        ("jq", &["-S", ".", "doc.json"], "-S"),
        ("jq", &["-c", ".", "doc.json"], "-c"),
        ("jq", &["--arg", "k", "v", ".$k", "doc.json"], "--arg"),
        ("jq", &["-s", ".", "doc.json"], "-s"),
        ("jq", &[".a // 1", "doc.json"], "//"),
        ("jq", &["del(.a.n)", "doc.json"], "an edit expression"),
        ("jq", &[".a.n = 9", "doc.json"], "an edit expression"),
    ];
    for (dialect, args, needle) in cases {
        let (rc, out, err) = run(ours, &[["--as", dialect].as_slice(), args].concat(), &dir);
        assert_eq!(
            rc, 2,
            "{dialect} {args:?} must exit 2, got {rc} with stdout {out:?}"
        );
        assert!(
            out.is_empty(),
            "{dialect} {args:?} printed a document while refusing: {out:?}"
        );
        assert!(
            err.contains(needle),
            "{dialect} {args:?} refusal must name {needle:?}, got {err:?}"
        );
    }
}

/// The override is the shell's own contract: without it the dialect comes from the
/// invoked name, and an unknown dialect is refused rather than guessed.
#[test]
fn the_dialect_override_is_explicit() {
    let dir = fixture_dir();
    let ours = Path::new(env!("CARGO_BIN_EXE_pyq"));
    let (jq_rc, jq_out, _) = run(ours, &["--as", "jq", ".a", "doc.json"], &dir);
    let (yq_rc, yq_out, _) = run(ours, &["--as", "yq", ".a", "doc.json"], &dir);
    assert_eq!((jq_rc, yq_rc), (0, 0));
    assert_ne!(
        jq_out, yq_out,
        "the two dialects must answer differently for one argv"
    );
    let (rc, _, err) = run(ours, &["--as", "xml", ".a", "doc.json"], &dir);
    assert_eq!(rc, 2, "--as xml must be refused");
    assert!(err.contains("--as"), "{err}");
}
