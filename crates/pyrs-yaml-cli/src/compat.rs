//! The jq / yq calling-surface shell.
//!
//! `pyq` keeps its own subcommand grammar (`pyq get .a file`) and additionally
//! accepts the two grammars users already have in their scripts: `jq` 1.8 and
//! mikefarah `yq` v4. This module turns such an argv into the equivalent
//! subcommand argv, which the binary then hands to clap exactly as if it had been
//! typed natively.
//!
//! What the shell is *not*: a jq/yq expression engine. pyq's selection power is
//! [`crate::paths::parse_path`] plus [`crate::verbs`], so an expression is
//! translated only when its meaning is already expressible there - a path, an
//! assignment of a literal, `del(<path>)`, or a single `select(<predicate>)`.
//! Anything else is refused by name before anything runs, because the alternative
//! is a command that quietly answers a different question.
//!
//! Flags work the same way: an allowlist per dialect, and every other flag is a
//! refusal. A shell that ignored `--arg` or `--split-exp` would print a document
//! the real tool would not, and that is the one failure mode a drop-in must never
//! have.
//!
//! Dialect comes from the name the binary was invoked as, so a `jq` or `yq`
//! symlink to this build behaves like the tool it replaces. `pyq` with a
//! shell-shaped argv takes the `yq` reading, since that is the YAML case and its
//! default (`--unwrapScalar` on) is also what pyq users expect from `get`.

use std::ffi::{OsStr, OsString};
use std::path::Path;

/// Subcommands that are pyq's own grammar: such an argv is never translated.
const NATIVE_COMMANDS: &[&str] = &[
    "fmt",
    "get",
    "set",
    "delete",
    "rename",
    "move",
    "append",
    "insert",
    "validate",
    "frontmatter",
    "sort-keys",
    "diff",
    "merge",
    "to-json",
    "to-toml",
    "from-json",
    "from-toml",
    "from-ini",
    "completion",
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dialect {
    /// `jq`: output is JSON unless `-r` asks for a bare scalar.
    Jq,
    /// mikefarah `yq` v4: output is YAML with scalars unwrapped.
    Yq,
}

/// Decide the dialect from the invoked name. `file_stem` drops the extension on
/// every platform, so `jq.exe` and `/usr/bin/jq` both read as jq; it splits only
/// at the separators this platform understands, which is what argv[0] is written
/// with.
pub fn dialect_of(argv0: &OsStr) -> Option<Dialect> {
    let stem = Path::new(argv0)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match stem.as_str() {
        "jq" => Some(Dialect::Jq),
        "yq" => Some(Dialect::Yq),
        _ => None,
    }
}

/// yq and jq spell every-element as `[]`; pyq's selector writes the same thing as
/// `[*]`. This is the one spelling difference the shell bridges, and it is a
/// textual one: the two forms select the same nodes in both grammars.
fn wildcard_form(text: &str) -> String {
    text.replace("[]", "[*]")
}

/// The single composition this shell can express: `<path> | select(<predicate>)`,
/// which is exactly pyq's `get <path> --select <predicate>`. Any other pipe chain
/// needs the real engine's evaluation order.
fn select_pipe(text: &str) -> Option<(String, String)> {
    let mut parts = text.trim().splitn(2, " | ");
    let (left, right) = (parts.next()?, parts.next()?);
    if right.trim().contains('|') {
        return None;
    }
    let path = path_expression(left)?;
    let pred = selection(right)?;
    Some((path, pred))
}

/// Characters that mean "this is a program, not a path".
fn looks_like_program(text: &str) -> bool {
    text.contains(['|', '(', ')', '{', '}', '?']) || text.contains("//") || text.contains("..")
}

/// A path expression the selector grammar accepts verbatim.
fn path_expression(text: &str) -> Option<String> {
    let trimmed = text.trim();
    // A jq/yq term that addresses the document starts with `.`, `$` or `[`; a bare
    // word is a function or a variable name (`keys`, `length`, `env`), which this
    // shell does not evaluate.
    if trimmed.is_empty() || trimmed.contains('=') || !trimmed.starts_with(['.', '$', '[']) {
        return None;
    }
    let trimmed = wildcard_form(trimmed);
    if looks_like_program(&trimmed) {
        return None;
    }
    crate::paths::parse_path(&trimmed)
        .ok()
        .map(|_| trimmed.to_string())
}

/// The parts of a value that would have to be *evaluated*, which this shell does
/// not do. `[1, 2]` and `"x"` are literals and travel to `set` untouched.
fn value_is_literal(value: &str) -> bool {
    !value.contains("env(")
        && !value.contains("load(")
        && !value.contains("input")
        && !value.contains('|')
        && !value.contains(';')
}

/// Split `.a.b = <literal>`, refusing the update operators (`|=`, `+=`, …) whose
/// right side is a program.
fn assignment(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    if ["|=", "+=", "-=", "*=", "/=", "//="]
        .iter()
        .any(|op| trimmed.contains(op))
    {
        return None;
    }
    let (left, right) = trimmed.split_once('=')?;
    let path = path_expression(left)?;
    let value = right.trim();
    if value.is_empty() || !value_is_literal(value) {
        return None;
    }
    Some((path, value.to_string()))
}

/// `del(<path>)` - one path, because two would need two edits and the shell does
/// not compose them.
fn deletion(text: &str) -> Option<String> {
    let inner = text.trim().strip_prefix("del(")?.strip_suffix(')')?.trim();
    if inner.contains(',') {
        return None;
    }
    path_expression(inner)
}

/// `select(<predicate>)` maps onto `--select`, whose grammar is deliberately the
/// same micro one (`relpath OP literal`).
fn selection(text: &str) -> Option<String> {
    let inner = text
        .trim()
        .strip_prefix("select(")?
        .strip_suffix(')')?
        .trim()
        .to_string();
    if inner.contains(['(', ')']) || inner.contains(" and ") || inner.contains(" or ") {
        return None;
    }
    // `--select` takes exactly one `relpath OP literal`; a second comparison is a
    // conjunction the verb does not speak.
    if ["==", "!=", ">=", "<=", ">", "<"]
        .iter()
        .filter(|op| inner.contains(*op))
        .count()
        != 1
    {
        return None;
    }
    Some(inner)
}

/// Refuse with the construct named and the dialect that owns it.
fn refuse(what: &str, dialect: Dialect, because: &str) -> String {
    let tool = match dialect {
        Dialect::Jq => "jq",
        Dialect::Yq => "yq",
    };
    format!("the {tool} compatibility shell does not implement {what}: {because}")
}

/// Flags the shell understood, each one already decided to be safe to honour.
#[derive(Default, Debug)]
struct Choices {
    /// Bare-scalar output (`jq -r`, `yq --unwrapScalar`, which v4 defaults on).
    raw: bool,
    /// JSON output (`jq` by nature, `yq -o json`).
    json: bool,
    inplace: bool,
    /// Print a multi-match stream without the `---` separators, as yq v4 does.
    no_doc: bool,
    input: Option<String>,
    /// A filter read from a file (`jq -f PROG`, `yq --from-file F`).
    filter_file: Option<String>,
}

/// Split a short flag into what the allowlist can name: `-oyaml` is `-o yaml`,
/// `-ri` is `-r` then `-i`. Returns None when the token needs no splitting.
fn expand_short(token: &str) -> Option<Vec<String>> {
    let rest_of = token.strip_prefix('-')?;
    if rest_of.is_empty() || rest_of.starts_with('-') || token.len() < 3 {
        return None;
    }
    let first = rest_of.chars().next()?;
    if matches!(first, 'o' | 'p' | 'f') {
        return Some(vec![format!("-{first}"), rest_of[1..].to_string()]);
    }
    if rest_of.chars().all(|c| {
        matches!(
            c,
            'r' | 'i' | 'e' | 'n' | 'N' | 'P' | 'C' | 'M' | 'c' | 's' | 'j' | 'a' | 'R' | '0' | 'v'
        )
    }) {
        return Some(rest_of.chars().map(|c| format!("-{c}")).collect());
    }
    None
}

/// Read the shell flags. `Choices` in `Dialect::Yq` starts with unwrapped
/// scalars, which is what yq v4 prints for YAML.
fn read_flags(args: &[OsString], dialect: Dialect) -> Result<(Vec<String>, Choices), String> {
    let mut positional: Vec<String> = Vec::new();
    let mut c = Choices {
        // yq v4 unwraps scalars by default; jq prints JSON by nature.
        raw: dialect == Dialect::Yq,
        json: dialect == Dialect::Jq,
        no_doc: dialect == Dialect::Yq,
        ..Choices::default()
    };
    // Work the argv as a queue: a short flag may carry a glued value (`-oy json`
    // spelling `-oyaml`) or be a cluster of boolean shorts (`-ri`), and both have
    // to be split before the allowlist is asked.
    let mut rest: Vec<String> = args[1..]
        .iter()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    let mut i = 0usize;
    while i < rest.len() {
        let token = rest[i].clone();
        if let Some(expanded) = expand_short(&token) {
            rest.splice(i..i + 1, expanded);
            continue;
        }
        let (name, inline) = match token.split_once('=') {
            Some((k, v)) if k.starts_with("--") => (k.to_string(), Some(v.to_string())),
            _ => (token.clone(), None),
        };
        let mut read_value = || -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            if i + 1 >= rest.len() {
                return Err(format!("{name} expects a value"));
            }
            i += 1;
            Ok(rest[i].clone())
        };
        match name.as_str() {
            "-r" | "--raw-output" | "--unwrapScalar" => c.raw = true,
            "-i" | "--inplace" => c.inplace = true,
            "-N" | "--no-doc" => c.no_doc = true,
            "-o" | "--output-format" => {
                let v = read_value()?;
                match v.as_str() {
                    "json" | "j" => {
                        c.json = true;
                        c.raw = false;
                    }
                    "yaml" | "y" => {
                        c.json = false;
                        if dialect == Dialect::Yq {
                            c.raw = true;
                        }
                    }
                    other => {
                        return Err(refuse(
                            &format!("--output-format {other}"),
                            dialect,
                            "this build writes YAML and JSON",
                        ));
                    }
                }
            }
            "-p" | "--input-format" => {
                let v = read_value()?;
                let mapped = match v.as_str() {
                    "json" | "j" => "json",
                    "yaml" | "y" | "kyaml" | "ky" => "yaml",
                    "toml" | "t" => "toml",
                    "ini" | "i" => "ini",
                    other => {
                        return Err(refuse(
                            &format!("--input-format {other}"),
                            dialect,
                            "this build reads YAML, JSON, JSONC, JSON5, TOML and INI",
                        ));
                    }
                };
                c.input = Some(mapped.to_string());
            }
            "-f" if dialect == Dialect::Jq => c.filter_file = Some(read_value()?),
            "--from-file" => c.filter_file = Some(read_value()?),
            // Every flag below changes what the *document* means or how it is
            // printed, in a way pyq's serializer does not have a knob for.
            "-e"
            | "--exit-status"
            | "-n"
            | "--null-input"
            | "-P"
            | "--prettyPrint"
            | "-C"
            | "--colors"
            | "-M"
            | "--no-colors"
            | "-c"
            | "--yaml-compact-seq-indent"
            | "--yaml-fix-merge-anchor-to-spec"
            | "-I"
            | "--indent"
            | "--tab"
            | "-j"
            | "--join-output"
            | "-0"
            | "--nul-output"
            | "--raw-output0"
            | "-a"
            | "--ascii-output"
            | "-R"
            | "--raw-input"
            | "-s"
            | "--slurp"
            | "--stream"
            | "--stream-errors"
            | "--seq"
            | "--unbuffered"
            | "-L"
            | "--library-path"
            | "--arg"
            | "--raw-arg"
            | "--argjson"
            | "--args"
            | "--jsonargs"
            | "--slurpfile"
            | "--rawfile"
            | "--jsonarg"
            | "-v"
            | "--verbose"
            | "--expression"
            | "-f"
            | "--front-matter"
            | "--split-exp"
            | "--split-exp-file"
            | "--csv-auto-parse"
            | "--csv-separator"
            | "--tsv-auto-parse"
            | "--lua-globals"
            | "--lua-prefix"
            | "--lua-suffix"
            | "--lua-unquoted"
            | "--properties-array-brackets"
            | "--properties-separator"
            | "--shell-key-separator"
            | "--string-interpolation"
            | "--header-preprocess"
            | "--debug-node-info"
            | "--ini-preserve-quotes"
            | "--security-disable-env-ops"
            | "--security-disable-file-ops"
            | "--security-enable-system-operator"
            | "--xml-attribute-prefix"
            | "--xml-content-name"
            | "--xml-directive-name"
            | "--xml-keep-namespace"
            | "--xml-proc-inst-prefix"
            | "--xml-raw-token"
            | "--xml-skip-directives"
            | "--xml-skip-proc-inst"
            | "--xml-strict-mode" => {
                return Err(refuse(
                    &name,
                    dialect,
                    "it has no pyq equivalent, and ignoring it would print a different \
                     document than the real tool - use pyq's own subcommands instead",
                ));
            }
            other if other.starts_with('-') => {
                return Err(refuse(other, dialect, "it is not on the allowlist"));
            }
            _ => positional.push(token),
        }
        i += 1;
    }
    Ok((positional, c))
}

/// Translate a shell-shaped argv into pyq's own argv. `Err` is the refusal the
/// user sees, and nothing runs in that case.
pub fn translate(argv: &[OsString], dialect: Dialect) -> Result<Vec<OsString>, String> {
    let (mut positional, c) = read_flags(argv, dialect)?;
    let expression = if let Some(path) = &c.filter_file {
        std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read the filter file {path}: {e}"))?
    } else if positional.is_empty() {
        return Err("no expression given".to_string());
    } else {
        positional.remove(0)
    };
    if positional.len() > 1 {
        return Err(refuse(
            &format!("{} input files", positional.len()),
            dialect,
            "pyq reads one document at a time; run it once per file",
        ));
    }
    let file = positional.first().cloned();
    let mut out: Vec<OsString> = vec![argv[0].clone()];
    let mut push = |s: &str| out.push(OsString::from(s));

    // Which command the expression reaches, before the flags are appended.
    enum Kind {
        Get(String),
        Whole,
        Set(String, String),
        Delete(String),
        Select(String),
        /// `<path> | select(<pred>)` - the verb applied to a stream selector.
        SelectOn(String, String),
    }
    let kind = if let Some(pred) = selection(&expression) {
        Kind::Select(pred)
    } else if let Some((path, value)) = assignment(&expression) {
        Kind::Set(path, value)
    } else if let Some(path) = deletion(&expression) {
        Kind::Delete(path)
    } else if let Some((path, pred)) = select_pipe(&expression) {
        Kind::SelectOn(path, pred)
    } else if let Some(path) = path_expression(&expression) {
        if path == "." {
            Kind::Whole
        } else {
            Kind::Get(path)
        }
    } else {
        return Err(refuse(
            &format!("the expression {expression:?}"),
            dialect,
            "this shell selects with a path, assigns a literal, deletes one path or passes one \
             select() predicate - pipes, functions and arithmetic need the real engine \
             (see `pyq get --help` for the verbs it can reach)",
        ));
    };

    // Only `get` carries the input-format and output-form knobs; on any other
    // command they would be a clap usage error rather than a decision.
    let on_get = matches!(kind, Kind::Get(_) | Kind::Select(_) | Kind::SelectOn(_, _));
    // An edit in jq mode would have to print JSON, and pyq's edit commands print
    // YAML: the document would be right and the answer wrong. yq mode is where the
    // two agree, so the edit forms are refused as jq rather than re-styled.
    if dialect == Dialect::Jq && matches!(kind, Kind::Set(..) | Kind::Delete(_)) {
        return Err(refuse(
            "an edit expression",
            dialect,
            "pyq's set and delete write YAML, which is not what jq prints - the same edit reads              as `pyq --as yq del(.a.n)` or `pyq delete .a.n`",
        ));
    }

    match kind {
        Kind::Get(path) => {
            push("get");
            push(&path);
        }
        Kind::Whole => {
            if c.json || dialect == Dialect::Jq {
                push("to-json");
                if c.raw {
                    push("--raw");
                }
            } else {
                push("fmt");
            }
        }
        Kind::Set(path, value) => {
            push("set");
            push(&path);
            push(&value);
            // yq creates the intermediate mapping on assignment; pyq needs it said.
            push("--create-missing");
        }
        Kind::Delete(path) => {
            push("delete");
            push(&path);
        }
        Kind::Select(pred) => {
            push("get");
            push(".");
            push("--select");
            push(&pred);
        }
        Kind::SelectOn(path, pred) => {
            push("get");
            push(&path);
            push("--select");
            push(&pred);
        }
    }

    if !on_get && (c.input.is_some() || (dialect == Dialect::Yq && c.json)) {
        return Err(refuse(
            "--input-format or --output-format here",
            dialect,
            "only the path form reaches pyq's get, which is where those flags live",
        ));
    }
    if on_get {
        if c.json && !c.raw {
            push("--json");
        }
        if c.raw {
            push("--raw");
        }
        if c.no_doc {
            push("--no-doc");
        }
        if let Some(input) = &c.input {
            push("--input");
            push(input);
        }
    }
    if c.inplace {
        push("--inplace");
    }
    if let Some(f) = file {
        push(&f);
    }
    Ok(out)
}

/// `--as jq|yq`: say which grammar the argv is written in, regardless of the name
/// the binary was invoked as. The shell needs this to be testable (copying the
/// binary under another name is not portable on Windows) and users need it when a
/// script was written for the other tool.
const DIALECT_FLAG: &str = "--as";

/// What the binary asks for: pass a native argv through, translate a shell-shaped
/// one. `Err` is the refusal, and nothing runs in that case.
pub fn plan(argv: &[OsString]) -> Result<Vec<OsString>, String> {
    let mut rest = argv.to_vec();
    let mut forced: Option<Dialect> = None;
    if let Some(pos) = rest
        .iter()
        .position(|a| a.to_string_lossy() == DIALECT_FLAG)
    {
        let value = rest.get(pos + 1).map(|v| v.to_string_lossy().to_string());
        forced = Some(match value.as_deref() {
            Some("jq") => Dialect::Jq,
            Some("yq") => Dialect::Yq,
            Some(other) => return Err(format!("--as expects jq or yq, got {other}")),
            None => return Err("--as expects a dialect: jq or yq".to_string()),
        });
        rest.remove(pos + 1);
        rest.remove(pos);
    }
    let Some(head) = rest.get(1) else {
        return Ok(rest);
    };
    let head = head.to_string_lossy().to_string();
    if forced.is_none()
        && (NATIVE_COMMANDS.contains(&head.as_str())
            || matches!(head.as_str(), "-h" | "--help" | "-V" | "--version"))
    {
        return Ok(rest);
    }
    let dialect = forced.unwrap_or_else(|| dialect_of(&rest[0]).unwrap_or(Dialect::Yq));
    translate(&rest, dialect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn translated(args: &[&str]) -> Vec<String> {
        let argv = os(args);
        plan(&argv)
            .map(|out| {
                out.iter()
                    .map(|a| a.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_else(|e| panic!("{args:?} refused: {e}"))
    }

    fn refused(args: &[&str]) -> String {
        let argv = os(args);
        match plan(&argv) {
            Ok(out) => panic!("{args:?}: expected a refusal, got {out:?}"),
            Err(e) => e,
        }
    }

    #[test]
    fn a_native_subcommand_is_never_touched() {
        let argv = os(&["pyq", "get", ".a", "f.yaml"]);
        assert_eq!(plan(&argv).unwrap(), argv);
    }

    #[test]
    fn help_and_version_reach_clap_unchanged() {
        for arg in ["--help", "-h", "--version", "-V"] {
            let argv = os(&["pyq", arg]);
            assert_eq!(plan(&argv).unwrap(), argv, "{arg}");
        }
    }

    #[test]
    fn the_dialect_comes_from_the_invoked_name() {
        assert_eq!(dialect_of(OsStr::new("/usr/bin/jq")), Some(Dialect::Jq));
        assert_eq!(dialect_of(OsStr::new("yq")), Some(Dialect::Yq));
        assert_eq!(dialect_of(OsStr::new("pyq")), None);
        // argv[0] carries the executable suffix on Windows, and `file_stem` drops
        // it whatever the case, so the suffix is not what splits the two readings.
        assert_eq!(dialect_of(OsStr::new("jq.exe")), Some(Dialect::Jq));
        assert_eq!(dialect_of(OsStr::new("yq.EXE")), Some(Dialect::Yq));
        // A backslash is a separator only on Windows; there the same spelling is a
        // single odd file name, and reading it as neither tool is correct. Asserted
        // per platform because a test that runs one way on both is the bug this one
        // replaced.
        #[cfg(windows)]
        assert_eq!(
            dialect_of(OsStr::new("C:\\tools\\jq.exe")),
            Some(Dialect::Jq)
        );
        #[cfg(unix)]
        assert_eq!(dialect_of(OsStr::new("C:\\tools\\jq.exe")), None);
    }

    #[test]
    fn yq_mode_unwraps_scalars_the_way_v4_defaults() {
        assert_eq!(
            translated(&["yq", ".a.b", "f.yaml"]),
            vec!["yq", "get", ".a.b", "--raw", "--no-doc", "f.yaml"]
        );
        // `pyq` with a shell-shaped argv reads as yq.
        assert_eq!(
            translated(&["pyq", ".a", "f.yaml"]),
            vec!["pyq", "get", ".a", "--raw", "--no-doc", "f.yaml"]
        );
    }

    #[test]
    fn jq_mode_prints_json_and_only_r_prints_bare() {
        assert_eq!(
            translated(&["jq", ".a", "x.json"]),
            vec!["jq", "get", ".a", "--json", "x.json"]
        );
        assert_eq!(
            translated(&["jq", "-r", ".a", "x.json"]),
            vec!["jq", "get", ".a", "--raw", "x.json"]
        );
        assert_eq!(
            translated(&["jq", ".", "x.json"]),
            vec!["jq", "to-json", "x.json"]
        );
        assert_eq!(
            translated(&["yq", ".", "f.yaml"]),
            vec!["yq", "fmt", "f.yaml"]
        );
    }

    #[test]
    fn assignment_creates_the_path_and_deletion_removes_it() {
        assert_eq!(
            translated(&["yq", "-i", ".a.b = 3", "f.yaml"]),
            vec![
                "yq",
                "set",
                ".a.b",
                "3",
                "--create-missing",
                "--inplace",
                "f.yaml"
            ]
        );
        assert_eq!(
            translated(&["yq", "del(.a)", "f.yaml"]),
            vec!["yq", "delete", ".a", "f.yaml"]
        );
    }

    #[test]
    fn select_reaches_the_verb_directly() {
        assert_eq!(
            translated(&["yq", "select(.age > 30)", "f.yaml"]),
            vec![
                "yq",
                "get",
                ".",
                "--select",
                ".age > 30",
                "--raw",
                "--no-doc",
                "f.yaml"
            ]
        );
    }

    #[test]
    fn format_flags_map_and_only_reach_get() {
        assert_eq!(
            translated(&["yq", "-o", "json", ".a", "f.yaml"]),
            vec!["yq", "get", ".a", "--json", "--no-doc", "f.yaml"]
        );
        assert_eq!(
            translated(&["yq", "-p", "json", ".a", "f.json"]),
            vec![
                "yq", "get", ".a", "--raw", "--no-doc", "--input", "json", "f.json"
            ]
        );
        // The glued short form yq users write every day.
        assert_eq!(
            translated(&["yq", "-oy", ".a", "f.yaml"]),
            vec!["yq", "get", ".a", "--raw", "--no-doc", "f.yaml"]
        );
        assert!(refused(&["yq", "-o", "json", "del(.a)", "f.yaml"]).contains("--output-format"));
        assert!(refused(&["yq", "-p", "json", ".", "f.json"]).contains("--input-format"));
    }

    #[test]
    fn a_filter_file_is_read_and_then_classified() {
        let dir = std::env::temp_dir().join(format!("pyq-compat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("path.jq");
        std::fs::write(&good, ".a.b\n").unwrap();
        let bad = dir.join("program.jq");
        std::fs::write(&bad, ".a | map(.b)\n").unwrap();
        assert_eq!(
            translated(&["jq", "-f", good.to_str().unwrap(), "x.json"]),
            vec!["jq", "get", ".a.b", "--json", "x.json"]
        );
        assert!(refused(&["jq", "-f", bad.to_str().unwrap(), "x.json"]).contains("map"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn anything_needing_the_engine_is_refused_by_name() {
        for expr in [
            ".a | .b",
            "map(.a)",
            ".a[] | select(.b)",
            "to_entries",
            ".a // 1",
            "has(\"a\")",
            "keys",
            "env(HOME)",
            ".a |= 1",
            ".a += 1",
            "del(.a, .b)",
            "select(.a > 1 and .b < 2)",
            "..a",
            "length",
            ".a = env(X)",
            ".a = .b | .c",
        ] {
            let why = refused(&["yq", expr, "f.yaml"]);
            assert!(why.contains("does not implement"), "{expr} -> {why}");
        }
    }

    #[test]
    fn flags_off_the_allowlist_are_refused_not_ignored() {
        // A flag the shell cannot honour must never be dropped silently: the
        // output would then be a document the real tool would not print.
        for flag in [
            "-e",
            "--exit-status",
            "-n",
            "--null-input",
            "-P",
            "-C",
            "-M",
            "-c",
            "-I",
            "--indent",
            "--tab",
            "-j",
            "-0",
            "-a",
            "-R",
            "-s",
            "--stream",
            "--seq",
            "-L",
            "--arg",
            "--argjson",
            "--args",
            "--slurpfile",
            "--rawfile",
            "-v",
            "--expression",
            "--split-exp",
            "--front-matter",
            "--string-interpolation",
            "--xml-strict-mode",
            "--lua-globals",
            "--security-disable-file-ops",
            "--csv-separator",
            "--nul-output",
        ] {
            let why = refused(&["pyq", flag, "x", ".a", "f.yaml"]);
            assert!(why.contains(flag), "{flag} -> {why}");
        }
    }

    #[test]
    fn an_unsupported_format_is_named() {
        assert!(refused(&["yq", "-o", "csv", ".a", "f.yaml"]).contains("csv"));
        assert!(refused(&["yq", "-p", "xml", ".a", "f.yaml"]).contains("xml"));
    }

    #[test]
    fn more_than_one_input_file_is_refused() {
        assert!(refused(&["yq", ".a", "one.yaml", "two.yaml"]).contains("2 input files"));
    }

    #[test]
    fn stdin_stays_the_default_input() {
        assert_eq!(
            translated(&["pyq", ".a"]),
            vec!["pyq", "get", ".a", "--raw", "--no-doc"]
        );
        // A bare `pyq` is clap's own usage screen, not a refusal.
        let bare = os(&["pyq"]);
        assert_eq!(plan(&bare).unwrap(), bare);
        assert!(refused(&["pyq", "-r"]).contains("no expression"));
    }
}
