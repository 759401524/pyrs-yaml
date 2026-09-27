//! pyq — a jq/yq-style command-line tool over YAML, JSON, TOML and INI,
//! built directly on `pyrs-yaml-core`. YAML is the hub: everything parses
//! to the shared AST and converts back out; `fmt` is the only operation
//! that preserves comments and layout (it round-trips through the AST).
//!
//! The logic lives in this library (with a thin `main.rs` binary shim) so
//! benchmarks and tests can drive the components without spawning the
//! process.

pub mod json;
pub mod paths;

use clap::{Parser, Subcommand};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::editing::Segment;
use pyrs_yaml_core::editing::plan;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::splice::SpliceState;
use pyrs_yaml_core::{parser, serializer, toml};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    },
    /// Extract a value at a path (`.a.b[0]`, `$` for the whole document).
    Get {
        /// JSONPath-lite selector.
        path: String,
        /// Input file; omit or `-` for stdin.
        file: Option<PathBuf>,
        #[command(flatten)]
        input: InputOpts,
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
        /// Rewrite the input file in place instead of printing.
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Remove a key or sequence element at a path.
    Delete {
        path: String,
        file: Option<PathBuf>,
        #[arg(long, short = 'i')]
        inplace: bool,
    },
    /// Convert to JSON text.
    ToJson {
        file: Option<PathBuf>,
        #[command(flatten)]
        input: InputOpts,
        /// Pretty-print with the given indent (0 = compact).
        #[arg(long, default_value_t = 2)]
        indent: usize,
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
        /// Emit JSON (an identity check / re-serialization of the input).
        #[arg(long)]
        json: bool,
        #[arg(long)]
        raw: bool,
    },
    /// Read TOML text, emit YAML (optionally a sub-extraction).
    FromToml {
        file: Option<PathBuf>,
        #[arg(long, default_value = "$")]
        get: String,
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
        #[arg(long)]
        json: bool,
        #[arg(long)]
        raw: bool,
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
        } => {
            let src = read_input(&file)?;
            let opts = serializer::SerializeOptions {
                explicit_start,
                ..Default::default()
            };
            let ast = parser::parse(&src, Schema::Core)?;
            let out = serializer::to_yaml_with_options(&ast, &opts)?;
            emit_str(&out)?;
        }
        Command::Get {
            path,
            file,
            input,
            json,
            raw,
        } => {
            let node = load(&file, &input)?;
            let sel = paths::parse_path(&path)?;
            emit_matched(&sel.select_all(&node), json, raw, &path)?;
        }
        Command::Set {
            path,
            value,
            file,
            create_missing,
            inplace,
        } => {
            let src = read_input(&file)?;
            let mut node = parser::parse(&src, Schema::Core)?;
            let v = parser::parse(&value, Schema::Core)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = spliced_edit(&mut node, &src, &path, |node, offs| {
                plan::set_path(node, &segs, v.clone(), true, &src, offs, create_missing)
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::Delete {
            path,
            file,
            inplace,
        } => {
            let src = read_input(&file)?;
            let mut node = parser::parse(&src, Schema::Core)?;
            let segs = segments_of(&paths::parse_path(&path)?)?;
            let text = spliced_edit(&mut node, &src, &path, |node, offs| {
                plan::delete_path(node, &segs, &src, offs)
            })?;
            write_text(&text, &file, inplace)?;
        }
        Command::ToJson {
            file,
            input,
            indent,
        } => {
            let node = load(&file, &input)?;
            let value = json::node_to_json(&node)?;
            let text = if indent == 0 {
                serde_json::to_string(&value)?
            } else {
                serde_json::to_string_pretty(&value)?
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
            json,
            raw,
        } => {
            let node = json::json_to_node(&read_input(&file)?)?;
            emit_selected(&node, &get, json, raw)?;
        }
        Command::FromToml {
            file,
            get,
            json,
            raw,
        } => {
            let node = toml::from_toml(&read_input(&file)?)?;
            emit_selected(&node, &get, json, raw)?;
        }
        Command::FromIni {
            file,
            get,
            json,
            raw,
        } => {
            let node = ini_to_node(&read_input(&file)?)?;
            emit_selected(&node, &get, json, raw)?;
        }
    }
    Ok(())
}

/// Shared tail for the `from-*` converters: optional path step, then output.
fn emit_selected(
    node: &CustomNode,
    path: &str,
    json: bool,
    raw: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sel = paths::parse_path(path)?;
    emit_matched(&sel.select_all(node), json, raw, path)
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
                    let v = json::node_to_json(node)?;
                    out.push_str(&serde_json::to_string(&v)?);
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

/// Layout-pinned edit through the shared splice engine (same architecture
/// as the Python CLI's document edits): mutate the AST while the plan's
/// `DirtyUnit` rewrites the original text, so comments and untouched
/// layout never drift. Falls back to full re-serialization when the
/// document layout is ineligible or the splice rejects the unit.
fn spliced_edit(
    node: &mut CustomNode,
    src: &str,
    path: &str,
    edit: impl FnOnce(
        &mut CustomNode,
        Option<&[usize]>,
    ) -> Result<pyrs_yaml_core::editing::DirtyUnit, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut state = parser::check_default_layout(node, src)
        .then(|| SpliceState::new(Arc::from(src.to_string())));
    // Populate the line-offset table once; the plan and splice share it.
    let offsets: Vec<usize> = match state.as_mut() {
        Some(s) => s.line_offsets().to_vec(),
        None => Vec::new(),
    };
    let unit = edit(
        node,
        if state.is_some() {
            Some(&offsets)
        } else {
            None
        },
    )
    .map_err(|e| plan_error(&e, path))?;
    let mut spliced = None;
    if let Some(s) = state.as_mut()
        && s.apply(&unit).is_ok()
    {
        spliced = s.materialize();
    }
    Ok(spliced.unwrap_or_else(|| serializer::to_yaml(node)))
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
        let v = json::node_to_json(node)?;
        println!("{}", serde_json::to_string_pretty(&v)?);
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
