//! pyq — a jq/yq-style command-line tool over YAML, JSON, TOML and INI,
//! built directly on `pyrs-yaml-core`. YAML is the hub: everything parses
//! to the shared AST and converts back out; `fmt` is the only operation
//! that preserves comments and layout (it round-trips through the AST).
//!
//! The logic lives in this library (with a thin `main.rs` binary shim) so
//! benchmarks and tests can drive the components without spawning the
//! process.

pub mod compare;
pub mod json;
pub mod multidoc;
pub mod paths;
pub mod verbs;

use clap::{Parser, Subcommand};
use pyrs_toml as toml;
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::editing::Segment;
use pyrs_yaml_core::editing::plan;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::{parser, serializer};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "pyq",
    version,
    about = "jq/yq-style processor for YAML, JSON, TOML and INI (pyrs-yaml core)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Round-trip a YAML document preserving comments, anchors and order.
    Fmt {
        /// Input file; omit or `-` for stdin.
        file: Option<PathBuf>,
        /// Wrap the document with an explicit `---` marker.
        #[arg(long)]
        explicit_start: bool,
        /// Process every document of a multi-document stream.
        #[arg(long, short = 'A')]
        all_docs: bool,
        /// Block-indent width per nesting level (default 2, like
        /// `pyrs-yaml fmt --indent`).
        #[arg(long, default_value_t = 2, value_name = "N")]
        indent: usize,
        /// Soft wrap column for plain scalars (0 disables wrapping).
        #[arg(long, default_value_t = 80, value_name = "N")]
        width: usize,
        /// Sort every mapping by key (whole document, serializer-level).
        #[arg(long)]
        sort_keys: bool,
        /// Rewrite the input file in place instead of printing.
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Extract a value at a path (`.a.b[0]`, `$` for the whole document).
    Get {
        /// JSONPath-lite selector.
        path: String,
        /// Input file; omit or `-` for stdin.
        file: Option<PathBuf>,
        #[command(flatten)]
        input: InputOpts,
        /// Process every document; docs without a match are skipped.
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[command(flatten)]
        verbs: verbs::Verbs,
        #[arg(long)]
        /// Emit JSON instead of YAML.
        json: bool,
        #[arg(long)]
        /// Print bare scalars without document markers.
        raw: bool,
    },
    /// Assign a value at a path (creates missing mappings with
    /// `--create-missing`; `$` replaces the whole document). Input is
    /// YAML (round-trip semantics, like `fmt`).
    Set {
        path: String,
        /// New value in YAML syntax; JSON works, being a YAML subset.
        value: String,
        file: Option<PathBuf>,
        #[arg(long)]
        create_missing: bool,
        /// Apply to every document where the path resolves; others keep
        /// their bytes.
        #[arg(long, short = 'A')]
        all_docs: bool,
        /// Rewrite the input file in place instead of printing.
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Remove a key or sequence element at a path.
    Delete {
        path: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Rename the key addressed by PATH (keeps value, comments and
    /// layout through the splice engine).
    Rename {
        path: String,
        /// New key text (written as a plain scalar).
        new_key: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Move the subtree at FROM to the existing destination path TO.
    Move {
        from: String,
        to: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Append VALUE to the sequence at PATH.
    Append {
        path: String,
        /// New value in YAML syntax; JSON works, being a YAML subset.
        value: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Insert VALUE into the sequence at PATH before index N (negative
    /// counts from the end).
    Insert {
        path: String,
        index: i64,
        value: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Check that a file parses; with `--schema`, validate every
    /// document against schema-language rules. Prints one violation per
    /// line, exit 1 on any.
    Validate {
        file: Option<PathBuf>,
        /// Input format; `auto` keys off the file extension. Without this
        /// a `pyproject.toml` would be fed to the YAML parser and rejected.
        #[arg(long, value_enum, default_value_t = Format::Auto)]
        input: Format,
        /// Schema rules file (YAML, core schema language).
        #[arg(long)]
        schema: Option<PathBuf>,
    },
    /// Extract Markdown front matter as YAML (`--body-out` splits the body).
    Frontmatter {
        file: Option<PathBuf>,
        #[arg(long)]
        body_out: Option<PathBuf>,
    },
    /// Sort the keys of the mapping at a path (one level; `$` sorts the
    /// document root).
    SortKeys {
        /// Target mapping path (`$` for the root).
        path: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'A')]
        all_docs: bool,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Compare two documents semantically: resolved values, structure and
    /// tags - comments, quoting and layout never appear. Lines are
    /// `- path: left`, `+ path: right`, `~ path: left -> right`; exit 0
    /// when identical, exit 1 when differences (engine errors also exit
    /// 1, but with a `pyq:` message on stderr and empty stdout).
    Diff {
        /// Base (left) document.
        base: PathBuf,
        /// Head (right) document.
        head: PathBuf,
        #[command(flatten)]
        input: InputOpts,
    },
    /// Deep-merge two documents, right-biased (yq `*+` shape): mappings
    /// recurse, sequences append (or replace with `--replace-arrays`),
    /// every other conflict lets the right node win with its style and
    /// comments intact. Emits YAML.
    Merge {
        /// Base (left) document.
        base: PathBuf,
        /// Overlay (right) document.
        head: PathBuf,
        /// Replace sequences instead of appending right items.
        #[arg(long)]
        replace_arrays: bool,
        #[command(flatten)]
        input: InputOpts,
    },
    /// Convert to JSON text.
    ToJson {
        file: Option<PathBuf>,
        #[command(flatten)]
        input: InputOpts,
        /// Emit a JSON array of all stream documents.
        #[arg(long, short = 'A')]
        all_docs: bool,
        /// Pretty-print with the given indent (0 = compact).
        #[arg(long, default_value_t = 2)]
        indent: usize,
        /// Emit JSONC: preserved leading/inline comments re-appear as
        /// `//` / `/* */` notes (RFC 8259 otherwise).
        #[arg(long, conflicts_with = "json5")]
        jsonc: bool,
        /// Emit JSON5: single-quoted strings where safer, `0x`/`.5`/`+7`/
        /// `Infinity`/`NaN` spellings, plus JSONC comments.
        #[arg(long)]
        json5: bool,
    },
    /// Convert to TOML text.
    ToToml {
        file: Option<PathBuf>,
        #[command(flatten)]
        input: InputOpts,
    },
    /// Read JSON text, emit YAML (optionally a sub-extraction).
    FromJson {
        file: Option<PathBuf>,
        #[arg(long, default_value = "$")]
        get: String,
        #[command(flatten)]
        verbs: verbs::Verbs,
        /// Emit JSON (an identity check / re-serialization of the input).
        #[arg(long)]
        json: bool,
        #[arg(long)]
        raw: bool,
        /// Accept `// line` and `/* block */` comments (JSONC dialect).
        #[arg(long)]
        jsonc: bool,
        /// Accept JSON5 extensions: trailing commas, single-quoted strings,
        /// unquoted identifier keys, and line/block comments.
        #[arg(long)]
        json5: bool,
    },
    /// Read TOML text, emit YAML (optionally a sub-extraction).
    FromToml {
        file: Option<PathBuf>,
        #[arg(long, default_value = "$")]
        get: String,
        #[command(flatten)]
        verbs: verbs::Verbs,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        raw: bool,
    },
    /// Read INI text, emit YAML (values stay strings, as in `load_ini`).
    FromIni {
        file: Option<PathBuf>,
        #[arg(long, default_value = "$")]
        get: String,
        #[command(flatten)]
        verbs: verbs::Verbs,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        raw: bool,
    },
    /// Print a shell completion script (bash, zsh, fish, ...).
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(clap::Args)]
pub struct InputOpts {
    /// Input format; `auto` keys off the file extension (content is
    /// always parsed as YAML, which is a JSON superset, and falls back
    /// from TOML on parse failure).
    #[arg(long, value_enum, default_value_t = Format::Auto)]
    pub input: Format,
    /// Scalar resolution schema for YAML input.
    #[arg(long, value_enum, default_value_t = SchemaKind::Core)]
    pub schema: SchemaKind,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Format {
    Auto,
    Yaml,
    Json,
    /// JSON with `// line` and `/* block */` comments (TypeScript/VS Code
    /// dialects); parsed by the native `pyrs-json` engine so comments are
    /// carried on the AST.
    Jsonc,
    /// JSON5: trailing commas, single-quoted strings, unquoted identifier
    /// keys, comments, `.5`/`+7`/`Infinity`/`NaN` numbers.
    Json5,
    Toml,
    Ini,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum SchemaKind {
    Core,
    Json,
    Failsafe,
    Yaml11,
}

impl From<SchemaKind> for Schema {
    fn from(k: SchemaKind) -> Self {
        match k {
            SchemaKind::Core => Schema::Core,
            SchemaKind::Json => Schema::Json,
            SchemaKind::Failsafe => Schema::Failsafe,
            SchemaKind::Yaml11 => Schema::Yaml1_1,
        }
    }
}

/// Process entry: parse argv, dispatch, map errors to `pyq: <msg>` + exit 1.
pub fn run(cli: Cli) -> ! {
    match run_command(cli.command) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "pyq: {e}");
            std::process::exit(1);
        }
    }
}

/// Dispatch one parsed command (exposed for benches and harnesses).
pub fn run_command(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Command::Fmt {
            file,
            explicit_start,
            all_docs,
            indent,
            width,
            sort_keys,
            inplace,
        } => {
            let src = read_input(&file)?;
            let opts = serializer::SerializeOptions {
                explicit_start,
                indent_size: indent,
                indent_mapping: indent,
                indent_sequence: indent,
                width,
                sort_keys,
                ..Default::default()
            };
            if all_docs {
                let docs = parser::parse_all(&src, Schema::Core)?;
                let mut out = String::new();
                for doc in &docs {
                    out.push_str("---\n");
                    out.push_str(&serializer::to_yaml_with_options(doc, &opts)?);
                }
                write_text(&out, &file, inplace)?;
            } else {
                let ast = parser::parse(&src, Schema::Core)?;
                let out = serializer::to_yaml_with_options(&ast, &opts)?;
                write_text(&out, &file, inplace)?;
            }
        }
        Command::Get {
            path,
            file,
            input,
            all_docs,
            verbs,
            json,
            raw,
        } => {
            let src = read_input(&file)?;
            let docs = load_docs(&src, file.as_deref(), &input, all_docs)?;
            let sel = paths::parse_path(&path)?;
            let stream: Vec<&CustomNode> = docs.iter().flat_map(|d| sel.select_all(d)).collect();
            emit_with_verbs(stream, &verbs, json, raw, &path)?;
        }
        Command::Set {
            path,
            value,
            file,
            create_missing,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let v = parser::parse(&value, Schema::Core)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::set_path(node, &segs, v.clone(), true, &src, offs, create_missing)
                    .map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Delete {
            path,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::delete_path(node, &segs, &src, offs).map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Rename {
            path,
            new_key,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::rename_path(node, &segs, &new_key, &src, offs).map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Move {
            from,
            to,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let from_segs = segments_of(&paths::parse_path(&from)?)?;
            let to_segs = segments_of(&paths::parse_path(&to)?)?;
            let text = stream_edit(&src, all_docs, &from, |node, offs| {
                plan::move_path(node, &from_segs, &to_segs, &src, offs)
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Append {
            path,
            value,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let v = parser::parse(&value, Schema::Core)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::append_path(node, &segs, v.clone(), &src, offs).map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Insert {
            path,
            index,
            value,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let v = parser::parse(&value, Schema::Core)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::insert_path(node, &segs, index, v.clone(), &src, offs).map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Validate {
            file,
            input,
            schema,
        } => {
            let src = read_input(&file)?;
            let input_opts = InputOpts {
                input,
                schema: SchemaKind::Core,
            };
            let resolver = match &schema {
                Some(p) => Some(parser::yaml::parse_schema_yaml(&std::fs::read_to_string(
                    p,
                )?)?),
                None => None,
            };
            // Route through the shared loader so a .toml/.json/.ini file is
            // checked with its own parser; `validate` used to hardcode YAML
            // and rejected every real config file in the wild. Only an
            // explicitly named YAML/JSON format has a document stream to walk;
            // `auto` must resolve the extension first, so it goes through
            // `load_source` instead of the stream splitter.
            let streamable = matches!(input, Format::Yaml | Format::Json);
            let docs = load_docs(&src, file.as_deref(), &input_opts, streamable)?;
            let mut failures = Vec::new();
            if let Some(rules) = &resolver {
                for doc in &docs {
                    if let Err(errs) =
                        parser::yaml::schema_language::validate_node(doc, rules, &src)
                    {
                        failures.extend(errs);
                    }
                }
            }
            if !failures.is_empty() {
                let mut out = String::new();
                for e in &failures {
                    out.push_str(&format!("{}: {}\n", e.path, e.message));
                }
                emit_str(&out)?;
                return Err("validation failed".into());
            }
            emit_str("ok\n")?;
        }
        Command::Frontmatter { file, body_out } => {
            let src = read_input(&file)?;
            let (fm, body) = split_front_matter(&src)?;
            let node = parser::parse(&fm, Schema::Core)?;
            emit_str(&serializer::to_yaml(&node))?;
            if let Some(p) = body_out {
                std::fs::write(p, body)?;
            }
        }
        Command::SortKeys {
            path,
            file,
            all_docs,
            inplace,
        } => {
            let src = read_input(&file)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = stream_edit(&src, all_docs, &path, |node, offs| {
                plan::sort_keys_path(node, &segs, &src, offs).map(|u| vec![u])
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Diff { base, head, input } => {
            let a = load(&Some(base), &input)?;
            let b = load(&Some(head), &input)?;
            let lines = compare::diff(&a, &b, Schema::from(input.schema));
            if !lines.is_empty() {
                for line in &lines {
                    println!("{line}");
                }
                std::process::exit(1);
            }
        }
        Command::Merge {
            base,
            head,
            replace_arrays,
            input,
        } => {
            let a = load(&Some(base), &input)?;
            let b = load(&Some(head), &input)?;
            let mode = if replace_arrays {
                compare::ArrayMode::Replace
            } else {
                compare::ArrayMode::Append
            };
            let merged = compare::merge(&a, &b, mode);
            emit_str(&serializer::to_yaml(&merged))?;
        }
        Command::ToJson {
            file,
            input,
            all_docs,
            indent,
            jsonc,
            json5,
        } => {
            let src = read_input(&file)?;
            let docs = load_docs(&src, file.as_deref(), &input, all_docs)?;
            // Dialect dispatch: `--json5` > `--jsonc` > plain JSON. The
            // comment-preserving JSONC/JSON5 writers come straight from
            // `pyrs-json`; leading/inline comments carried on the AST
            // re-appear as `//` / `/* */` notes.
            let render = |d: &CustomNode| -> Result<String, String> {
                Ok(if json5 {
                    if indent == 0 {
                        json::node_to_json5(d)?
                    } else {
                        json::node_to_json5_pretty(d, indent)?
                    }
                } else if jsonc {
                    if indent == 0 {
                        json::node_to_jsonc(d)?
                    } else {
                        json::node_to_jsonc_pretty(d, indent)?
                    }
                } else if indent == 0 {
                    json::node_to_json(d)?
                } else {
                    json::node_to_json_pretty(d, indent)?
                })
            };
            // -A emits a JSON array of documents (Python CLI parity);
            // otherwise the single document renders as before. The #107
            // native engine replaces the historical
            // `serde_json::Value::Array` intermediate: each doc is
            // serialised independently and joined textually so
            // indentation stays identical.
            let text = if all_docs || docs.len() > 1 {
                let mut parts = Vec::with_capacity(docs.len());
                for d in &docs {
                    parts.push(render(d)?);
                }
                if indent == 0 {
                    format!("[{}]", parts.join(","))
                } else {
                    format!("[\n{}\n]", parts.join(",\n"))
                }
            } else {
                render(&docs[0])?
            };
            let mut out = text;
            out.push('\n');
            emit_str(&out)?;
        }
        Command::ToToml { file, input } => {
            let node = load(&file, &input)?;
            emit_str(&toml::to_toml(&node)?)?;
        }
        Command::FromJson {
            file,
            get,
            verbs,
            json,
            raw,
            jsonc,
            json5,
        } => {
            let src = read_input(&file)?;
            // `--json5` subsumes `--jsonc`: JSON5 enables comments plus
            // the other three extensions, so a caller passing both flags
            // gets JSON5 without an error.
            let node = if json5 {
                json::json_to_node_json5(&src)?
            } else if jsonc {
                json::json_to_node_jsonc(&src)?
            } else {
                json::json_to_node(&src)?
            };
            emit_selected(&node, &get, &verbs, json, raw)?;
        }
        Command::FromToml {
            file,
            get,
            verbs,
            json,
            raw,
        } => {
            let node = toml::from_toml(&read_input(&file)?)?;
            emit_selected(&node, &get, &verbs, json, raw)?;
        }
        Command::FromIni {
            file,
            get,
            verbs,
            json,
            raw,
        } => {
            let node = ini_to_node(&read_input(&file)?)?;
            emit_selected(&node, &get, &verbs, json, raw)?;
        }
        Command::Completion { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            clap_complete::generate(shell, &mut cmd, "pyq", &mut std::io::stdout());
        }
    }
    Ok(())
}

/// Shared tail for the `from-*` converters: optional path step, then output.
fn emit_selected(
    node: &CustomNode,
    path: &str,
    verbs: &verbs::Verbs,
    json: bool,
    raw: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sel = paths::parse_path(path)?;
    emit_with_verbs(sel.select_all(node), verbs, json, raw, path)
}

/// Verb pipeline over a match stream, then output; `--join` collapses the
/// stream into one scalar first.
fn emit_with_verbs(
    stream: Vec<&CustomNode>,
    verbs: &verbs::Verbs,
    json: bool,
    raw: bool,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = verbs.apply(stream, path)?;
    if let Some(text) = verbs.render_join(&stream)? {
        return emit_value(&CustomNode::plain_scalar(text), json, raw);
    }
    emit_matched(&stream, json, raw, path)
}

/// jq-stream output: zero matches is an error, one match prints plainly,
/// many print as a multi-document YAML stream (or one JSON value per line,
/// matching `jq` output for piping).
fn emit_matched(
    found: &[&CustomNode],
    json: bool,
    raw: bool,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match found.len() {
        0 => Err(format!("path not found: {path}").into()),
        1 => {
            emit_value(found[0], json, raw)?;
            Ok(())
        }
        _ => {
            let mut out = String::new();
            if json {
                for node in found {
                    out.push_str(&json::node_to_json(node)?);
                    out.push('\n');
                }
            } else {
                for node in found {
                    out.push_str("---\n");
                    out.push_str(&serializer::to_yaml(node));
                }
            }
            emit_str(&out)
        }
    }
}

/// Translate a parsed selector into core edit segments; wildcards are a
/// query-only feature and never address a single editable node.
fn segments_of(sel: &paths::Selector) -> Result<Vec<Segment<'static>>, String> {
    sel.segments()
        .iter()
        .map(|s| match s {
            paths::Seg::Key(k) => Ok(Segment::Key(Cow::Owned(k.clone()))),
            paths::Seg::Index(i) => Ok(Segment::Index(*i)),
            paths::Seg::Wildcard => Err("cannot edit through a wildcard".to_string()),
        })
        .collect()
}

/// The shared plan engine reports stable i18n keys (the bindings resolve
/// them through a catalog); the CLI renders them as path-aware sentences.
fn plan_error(e: &str, path: &str) -> String {
    match e {
        "missing-path" => format!("path not found: {path}"),
        "cannot-edit-alias" => format!("cannot edit through an alias: {path}"),
        "create-needs-mapping" => format!("cannot create {path}: parent is not a mapping"),
        "index-out-of-range-edit" => format!("index out of range: {path}"),
        "not-a-sequence" => format!("not a sequence: {path}"),
        "edit-error" => "edit failed".to_string(),
        other => other.to_string(),
    }
}

/// Layout-pinned edit over one document or, with `--all-docs`, over a
/// stream through `MultiDocEditor`: per-document splice state isolates
/// failures - a plan error skips that doc (its original segment bytes
/// are emitted verbatim), and only an all-miss command errors.
fn stream_edit(
    src: &str,
    all_docs: bool,
    path: &str,
    plan: impl Fn(
        &mut CustomNode,
        Option<&[usize]>,
    ) -> Result<Vec<pyrs_yaml_core::editing::DirtyUnit>, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut docs = if all_docs {
        parser::parse_all(src, Schema::Core)?
    } else {
        vec![parser::parse(src, Schema::Core)?]
    };
    let mut ed = multidoc::MultiDocEditor::new(src, &docs);
    let mut applied = 0usize;
    let mut last_err: Option<String> = None;
    for (i, doc) in docs.iter_mut().enumerate() {
        if let Err(e) = ed.edit(i, doc, |node, offs| plan(node, offs)) {
            if !all_docs {
                return Err(plan_error(&e, path).into());
            }
            last_err = Some(e);
            continue;
        }
        applied += 1;
    }
    if applied == 0 {
        let e = last_err.unwrap_or_else(|| "missing-path".to_string());
        return Err(plan_error(&e, path).into());
    }
    Ok(ed.finalize(&docs))
}

/// Load one document, or every document with `--all-docs` (YAML/JSON
/// streams only; explicit TOML/INI input has no stream concept).
pub fn load_docs(
    src: &str,
    path_hint: Option<&Path>,
    input: &InputOpts,
    all_docs: bool,
) -> Result<Vec<CustomNode>, Box<dyn std::error::Error>> {
    if !all_docs {
        return Ok(vec![load_source(src, path_hint, input)?]);
    }
    if matches!(
        input.input,
        Format::Toml | Format::Ini | Format::Jsonc | Format::Json5
    ) {
        return Err("--all-docs only applies to YAML or JSON input".into());
    }
    Ok(parser::parse_all(src, Schema::from(input.schema))?)
}

/// Split `---\n<yaml>\n---\n<body>`; mirrors the Python CLI's frontmatter
/// fences (`---` or `...` closing the block).
fn split_front_matter(src: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rest = src
        .strip_prefix("---\n")
        .or_else(|| src.strip_prefix("---\r\n"))
        .ok_or("no front matter")?;
    let mut consumed = 0usize;
    for line in rest.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        if t == "---" || t == "..." {
            return Ok((
                rest[..consumed].to_string(),
                rest[consumed + line.len()..].to_string(),
            ));
        }
        consumed += line.len();
    }
    Err("unterminated front matter".into())
}

/// Load according to --input (auto: extension, else YAML/JSON superset,
/// with a TOML retry on failure).
pub fn load(
    file: &Option<PathBuf>,
    input: &InputOpts,
) -> Result<CustomNode, Box<dyn std::error::Error>> {
    let src = read_input(file)?;
    load_source(&src, file.as_deref(), input)
}

/// Format dispatch over an in-memory source; `path_hint` drives the
/// extension-based auto-detection (None for stdin).
pub fn load_source(
    src: &str,
    path_hint: Option<&Path>,
    input: &InputOpts,
) -> Result<CustomNode, Box<dyn std::error::Error>> {
    let format = match input.input {
        Format::Auto => match path_hint
            .and_then(|f| f.extension())
            .and_then(|e| e.to_str())
        {
            Some("json") => Format::Json,
            Some("jsonc") => Format::Jsonc,
            Some("json5") => Format::Json5,
            Some("toml") => Format::Toml,
            Some("ini") => Format::Ini,
            _ => Format::Yaml,
        },
        f => f,
    };
    Ok(match format {
        Format::Yaml => parser::parse(src, Schema::from(input.schema))?,
        // YAML 1.2 is a JSON superset: JSON input parses unchanged.
        Format::Json => parser::parse(src, Schema::Json)?,
        // JSONC/JSON5 go through the native pyrs-json engine: comment-only
        // syntax is not YAML-representable, and the dialect parsers carry
        // comments onto the AST slots for `to-json --jsonc` round-trips.
        Format::Jsonc => json::json_to_node_jsonc(src)?,
        Format::Json5 => json::json_to_node_json5(src)?,
        Format::Toml => match toml::from_toml(src) {
            Ok(node) => node,
            // auto mode guesses wrong without an extension; retry as YAML.
            Err(toml_err) if matches!(input.input, Format::Auto) => {
                parser::parse(src, Schema::from(input.schema)).map_err(|_| toml_err)?
            }
            Err(e) => return Err(e.into()),
        },
        Format::Ini => ini_to_node(src)?,
        Format::Auto => unreachable!("resolved above"),
    })
}

/// `{section: {key: str}}` as an all-quoted AST, mirroring `load_ini`
/// output (strings must never re-resolve).
pub fn ini_to_node(src: &str) -> Result<CustomNode, Box<dyn std::error::Error>> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut current: Option<usize> = None;
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if let Some(rest) = t.strip_prefix('[') {
            let name = rest.trim_end_matches(']').trim();
            sections.push((name.to_string(), Vec::new()));
            current = Some(sections.len() - 1);
            continue;
        }
        let Some(idx) = current else {
            return Err(format!("INI key before any section: {t}").into());
        };
        let Some((k, v)) = t.split_once(['=', ':']) else {
            return Err(format!("malformed INI line: {t}").into());
        };
        sections[idx]
            .1
            .push((k.trim().to_string(), v.trim().to_string()));
    }
    let mut map: indexmap::IndexMap<CustomNode, CustomNode> = indexmap::IndexMap::new();
    for (name, pairs) in sections {
        let mut vals: indexmap::IndexMap<CustomNode, CustomNode> = indexmap::IndexMap::new();
        for (k, v) in pairs {
            vals.insert(
                CustomNode::double_quoted_scalar(k),
                CustomNode::double_quoted_scalar(v),
            );
        }
        map.insert(
            CustomNode::plain_scalar(name),
            CustomNode::plain_mapping(vals),
        );
    }
    Ok(CustomNode::plain_mapping(map))
}

/// Shared tail for edit commands: in-place rewrite or stdout.
fn write_text(
    text: &str,
    file: &Option<PathBuf>,
    inplace: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if inplace {
        let f = file.as_ref().ok_or("--inplace requires a file argument")?;
        if f.as_os_str() == "-" {
            return Err("--inplace cannot write to stdin".into());
        }
        std::fs::write(f, text)?;
    } else {
        emit_str(text)?;
    }
    Ok(())
}

fn read_input(file: &Option<PathBuf>) -> Result<String, Box<dyn std::error::Error>> {
    match file {
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(f) if f.as_os_str() == "-" => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(f) => Ok(std::fs::read_to_string(f)?),
    }
}

fn emit_value(node: &CustomNode, json: bool, raw: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        let text = json::node_to_json_pretty(node, 2)?;
        println!("{text}");
        return Ok(());
    }
    if raw && let CustomNode::Scalar { value, .. } = node {
        println!("{value}");
        return Ok(());
    }
    print!("{}", serializer::to_yaml(node));
    Ok(())
}

fn emit_str(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut so = std::io::stdout();
    so.write_all(text.as_bytes())?;
    so.flush()?;
    Ok(())
}
