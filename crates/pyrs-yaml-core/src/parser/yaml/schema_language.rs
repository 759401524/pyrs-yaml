//! YAML Schema Language — user-defined schema resolution.
//!
//! A schema file (or inline dict) defines a list of rules mapping scalar
//! patterns to YAML types. Rules are checked in order; the first pattern that
//! matches decides the type. If no rule matches, resolution falls back to the
//! `extends` schema (default `core`).
//!
//! Example:
//! ```yaml
//! name: myapp
//! version: 1
//! extends: core
//! rules:
//!   - pattern: "^0x[0-9a-fA-F]+$"
//!     type: int
//!   - pattern: "^\\d{4}-\\d{2}-\\d{2}$"
//!     type: str
//!   - pattern: "^(yes|no|Yes|No)$"
//!     type: bool
//! ```

use crate::ast::CustomNode;
use crate::error::ParseError;
use crate::parser::yaml::{Schema, SchemaResolver, YamlType};
use regex::Regex;
use std::borrow::Cow;

/// The target YAML type a rule maps a matching scalar to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlTypeKind {
    Null,
    Bool,
    Int,
    Float,
    Str,
}

impl YamlTypeKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "null" => Some(Self::Null),
            "bool" | "boolean" => Some(Self::Bool),
            "int" | "integer" => Some(Self::Int),
            "float" | "double" => Some(Self::Float),
            "str" | "string" => Some(Self::Str),
            _ => None,
        }
    }

    /// The resolver function for this kind. Called with the original scalar
    /// and its trimmed form; the match on kind is made once here at
    /// construction instead of per-scalar at resolve time.
    fn resolver(self) -> KindResolver {
        match self {
            Self::Null => resolve_null_kind,
            Self::Bool => resolve_bool_kind,
            Self::Int => resolve_int_kind,
            Self::Float => resolve_float_kind,
            Self::Str => resolve_str_kind,
        }
    }
}

impl std::fmt::Display for YamlTypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool => write!(f, "bool"),
            Self::Int => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Str => write!(f, "str"),
        }
    }
}

/// Resolves a matched scalar (original + trimmed form) to a `YamlType`.
type KindResolver = for<'a> fn(&'a str, &'a str) -> YamlType<'a>;

/// A single schema rule: a compiled regex pattern and a target type resolver.
#[derive(Clone)]
pub struct Rule {
    pattern: Regex,
    resolver: KindResolver,
}

impl Rule {
    /// Build a rule from a raw pattern string and type name.
    pub fn new(pattern: &str, target: YamlTypeKind) -> Result<Self, ParseError> {
        let regex = Regex::new(pattern).map_err(|e| ParseError::Syntax {
            message: format!("invalid schema pattern '{pattern}': {e}"),
            line: 0,
            col: 0,
        })?;
        Ok(Self {
            pattern: regex,
            resolver: target.resolver(),
        })
    }
}

/// What a validate rule expects a node to be: one of the scalar types a resolver can
/// produce, or the shape of a collection.
///
/// The container shapes are why this enum exists instead of reusing [`YamlTypeKind`].
/// A `rules:` pattern resolves the *text* of a scalar, so it can never produce a
/// mapping or a sequence - but a rule about a *path* plainly can. Before `map` and
/// `seq` the language could not say "this node must be a mapping" at all, and
/// `mapping_of` / `sequence_of` could only describe members, so a node of the wrong
/// shape was skipped rather than reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSpec {
    /// A scalar that resolves to this type.
    Scalar(YamlTypeKind),
    /// A mapping node, whatever it holds.
    Mapping,
    /// A sequence node, whatever it holds.
    Sequence,
}

impl TypeSpec {
    /// Parse a type name the way a hand-written schema spells it. `map` / `seq` are
    /// the short forms; the long names and the common synonyms are accepted because a
    /// schema is typed by a person, not emitted by a tool.
    fn from_name(name: &str) -> Option<Self> {
        if let Some(kind) = YamlTypeKind::from_name(name) {
            return Some(Self::Scalar(kind));
        }
        match name {
            "map" | "mapping" | "object" => Some(Self::Mapping),
            "seq" | "sequence" | "array" | "list" => Some(Self::Sequence),
            _ => None,
        }
    }

    /// A container shape is a property of a node the rule names. A pathless rule has
    /// no such node, so a pathless `type: map` would be a rule that checks nothing.
    fn is_container(self) -> bool {
        matches!(self, Self::Mapping | Self::Sequence)
    }
}

impl std::fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar(kind) => write!(f, "{kind}"),
            Self::Mapping => write!(f, "map"),
            Self::Sequence => write!(f, "seq"),
        }
    }
}

/// The kind of structural validation a [`ValidateRule`] performs.
///
/// Scope decides how much a rule asserts. A rule that names a `path` is about that
/// node, so arriving with the wrong shape is a failure. A pathless rule cannot name a
/// node, so it selects the nodes it can describe (`type` speaks of scalars,
/// `sequence_of` of sequences, `mapping_of` of mappings) and says nothing about the
/// rest. Members are asserted either way: `sequence_of: int` has already said what its
/// elements are, and a nested sequence among them is not an `int`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateKind {
    /// The node must be a scalar of the given type, or have the given container shape.
    Type(TypeSpec),
    /// The node must be a sequence and every element must satisfy the given type.
    SequenceOf(TypeSpec),
    /// The node must be a mapping and every value must satisfy the given type.
    MappingOf(TypeSpec),
    /// Path must exist (non-null).
    Required,
}

/// A structural validation rule: applies to a specific path (or all scalars
/// if `path` is `None`) and checks type/structure.
#[derive(Debug, Clone)]
pub struct ValidateRule {
    /// JSONPath-like path (e.g. `"$.port"`, `"$.tags[*]"`). `None` = all scalars.
    pub path: Option<String>,
    pub kind: ValidateKind,
    /// If `true`, the path must resolve to a non-null value.
    pub required: bool,
}

impl ValidateRule {
    pub fn new(path: Option<&str>, kind: ValidateKind) -> Self {
        Self {
            path: path.map(String::from),
            kind,
            required: false,
        }
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

/// A validation error: the path, the expected constraint, and the actual value.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaValidationError {
    pub path: String,
    pub message: String,
    /// Line number (1-based) in the source document, if available.
    pub line: Option<usize>,
    /// Column number (1-based) in the source document, if available.
    pub column: Option<usize>,
}

impl SchemaValidationError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Set the line/column location from a source byte range.
    pub fn with_location(mut self, source: &str, range: Option<&std::ops::Range<usize>>) -> Self {
        if let Some(r) = range {
            let before = &source[..r.start];
            self.line = Some(before.lines().count().max(1));
            self.column = Some(r.start - before.rfind('\n').map(|i| i + 1).unwrap_or(0) + 1);
        }
        self
    }
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Location and path answer different questions: one points into the source, the
        // other names the target the schema author wrote. Reporting only the location left
        // a reader with "1:9: expected map but got sequence" and no way to tell which key
        // the schema had complained about.
        match self.line {
            Some(line) => write!(
                f,
                "{}:{}: {}: {}",
                line,
                self.column.unwrap_or(1),
                self.path,
                self.message
            ),
            None => write!(f, "{}: {}", self.path, self.message),
        }
    }
}

/// A schema resolver built from a list of rules, with an optional fallback
/// schema for scalars that match no rule.
#[derive(Clone)]
pub struct RuleResolver {
    rules: Vec<Rule>,
    fallback: Option<Schema>,
    validate_rules: Vec<ValidateRule>,
}

impl RuleResolver {
    /// Build a resolver from rules and an optional fallback schema.
    pub fn new(rules: Vec<Rule>, fallback: Option<Schema>) -> Self {
        Self {
            rules,
            fallback,
            validate_rules: Vec::new(),
        }
    }

    /// Build a resolver with both resolve rules and validate rules.
    pub fn with_validate_rules(
        rules: Vec<Rule>,
        fallback: Option<Schema>,
        validate_rules: Vec<ValidateRule>,
    ) -> Self {
        Self {
            rules,
            fallback,
            validate_rules,
        }
    }

    /// Access the validate rules (for `validate_node`).
    pub fn validate_rules(&self) -> &[ValidateRule] {
        &self.validate_rules
    }
}

impl SchemaResolver for RuleResolver {
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        // YAML's own separation set - `str::trim` would also strip NBSP and the
        // Unicode separators, which are plain-scalar content (see
        // `resolve_core_type`).
        let trimmed = value.trim_matches(pyrs_schema::is_yaml_blank);
        for rule in &self.rules {
            if rule.pattern.is_match(trimmed) {
                return (rule.resolver)(value, trimmed);
            }
        }
        match &self.fallback {
            Some(schema) => schema.resolve(value),
            None => YamlType::Str(Cow::Borrowed(value)),
        }
    }
}

fn resolve_null_kind<'a>(_value: &'a str, _trimmed: &'a str) -> YamlType<'a> {
    YamlType::Null
}

fn resolve_str_kind<'a>(value: &'a str, _trimmed: &'a str) -> YamlType<'a> {
    YamlType::Str(Cow::Borrowed(value))
}

fn resolve_bool_kind<'a>(value: &'a str, trimmed: &'a str) -> YamlType<'a> {
    match trimmed {
        "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" | "y" | "Y" => {
            YamlType::Bool(true)
        }
        "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" | "n" | "N" => {
            YamlType::Bool(false)
        }
        _ => YamlType::Str(Cow::Borrowed(value)),
    }
}

fn resolve_int_kind<'a>(value: &'a str, trimmed: &'a str) -> YamlType<'a> {
    parse_int(trimmed)
        .map(YamlType::Int)
        .unwrap_or(YamlType::Str(Cow::Borrowed(value)))
}

fn resolve_float_kind<'a>(value: &'a str, trimmed: &'a str) -> YamlType<'a> {
    match trimmed.parse::<f64>() {
        Ok(f) => YamlType::Float(f),
        Err(_) => YamlType::Str(Cow::Borrowed(value)),
    }
}

/// Parse an integer, handling decimal, hex (`0x`), octal (`0o`), and binary
/// (`0b`) prefixes.
fn parse_int(value: &str) -> Option<i64> {
    if (value.starts_with("0x") || value.starts_with("0X"))
        && let Ok(n) = i64::from_str_radix(&value[2..], 16)
    {
        return Some(n);
    }
    if (value.starts_with("0o") || value.starts_with("0O"))
        && let Ok(n) = i64::from_str_radix(&value[2..], 8)
    {
        return Some(n);
    }
    if (value.starts_with("0b") || value.starts_with("0B"))
        && let Ok(n) = i64::from_str_radix(&value[2..], 2)
    {
        return Some(n);
    }
    value.parse::<i64>().ok()
}

/// Parse a schema YAML document into a [`RuleResolver`].
///
/// Expected structure:
/// ```yaml
/// name: myapp
/// extends: core
/// rules:
///   - pattern: "^0x[0-9a-fA-F]+$"
///     type: int
/// ```
pub fn parse_schema_yaml(yaml: &str) -> Result<RuleResolver, ParseError> {
    let ast = crate::parser::parse(yaml, Schema::Core)?;
    let CustomNode::Mapping { pairs, .. } = &ast else {
        return Err(ParseError::Syntax {
            message: "schema must be a mapping".to_string(),
            line: 0,
            col: 0,
        });
    };

    let mut extends: Option<Schema> = None;
    let mut rules: Vec<Rule> = Vec::new();
    let mut validate_rules: Vec<ValidateRule> = Vec::new();

    for (key, value) in pairs {
        let key_str = scalar_str(key)?;
        match key_str.as_deref() {
            Some("extends") => {
                let ext_name = scalar_str(value)?;
                extends = parse_extends(ext_name.as_deref().unwrap_or("core"));
            }
            Some("rules") => {
                let rule_nodes = match value {
                    CustomNode::Sequence { items, .. } => items,
                    _ => {
                        return Err(ParseError::Syntax {
                            message: "'rules' must be a sequence".to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                };
                for node in rule_nodes {
                    rules.push(rule_from_node(node)?);
                }
            }
            Some("validate") => {
                let vnodes = match value {
                    CustomNode::Sequence { items, .. } => items,
                    _ => {
                        return Err(ParseError::Syntax {
                            message: "'validate' must be a sequence".to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                };
                for node in vnodes {
                    validate_rules.push(validate_rule_from_node(node)?);
                }
            }
            _ => {} // ignore unknown top-level keys (name, version, ...)
        }
    }

    Ok(RuleResolver::with_validate_rules(
        rules,
        extends,
        validate_rules,
    ))
}

/// Parse the `extends` schema name into a Schema.
fn parse_extends(name: &str) -> Option<Schema> {
    match name.to_lowercase().as_str() {
        "failsafe" => Some(Schema::Failsafe),
        "json" => Some(Schema::Json),
        "core" => Some(Schema::Core),
        "yaml1.1" | "yaml11" => Some(Schema::Yaml1_1),
        other => crate::parser::yaml::registry::get(other),
    }
}

/// Extract a scalar string from a node.
fn scalar_str(node: &CustomNode) -> Result<Option<Cow<'_, str>>, ParseError> {
    match node {
        CustomNode::Scalar { value, .. } => Ok(Some(Cow::Borrowed(value.as_ref()))),
        CustomNode::Null { .. } => Ok(None),
        _ => Err(ParseError::Syntax {
            message: "expected scalar".to_string(),
            line: 0,
            col: 0,
        }),
    }
}

/// Build a Rule from a `{pattern, type}` mapping node.
fn rule_from_node(node: &CustomNode) -> Result<Rule, ParseError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(ParseError::Syntax {
            message: "each rule must be a mapping with 'pattern' and 'type'".to_string(),
            line: 0,
            col: 0,
        });
    };
    let mut pattern: Option<String> = None;
    let mut target: Option<YamlTypeKind> = None;
    for (key, value) in pairs {
        let key_str = scalar_str(key)?.unwrap_or(Cow::Borrowed(""));
        match key_str.as_ref() {
            "pattern" => {
                pattern = scalar_str(value)?.map(|s| s.into_owned());
            }
            "type" => {
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                target = YamlTypeKind::from_name(ty.as_ref());
                if target.is_none() {
                    return Err(ParseError::Syntax {
                        message: format!(
                            "invalid schema type '{}'. Valid: null, bool, int, float, str; \
                             map and seq assert a node, so they belong in a validate rule",
                            ty
                        ),
                        line: 0,
                        col: 0,
                    });
                }
            }
            _ => {}
        }
    }
    let pattern = pattern.ok_or_else(|| ParseError::Syntax {
        message: "rule missing 'pattern'".to_string(),
        line: 0,
        col: 0,
    })?;
    let target = target.ok_or_else(|| ParseError::Syntax {
        message: "rule missing 'type'".to_string(),
        line: 0,
        col: 0,
    })?;
    Rule::new(&pattern, target)
}

/// Build a [`ValidateRule`] from a `{path, type|sequence_of|mapping_of|required}` mapping.
fn validate_rule_from_node(node: &CustomNode) -> Result<ValidateRule, ParseError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(ParseError::Syntax {
            message: "each validate rule must be a mapping".to_string(),
            line: 0,
            col: 0,
        });
    };
    let mut path: Option<String> = None;
    let mut kind: Option<ValidateKind> = None;
    let mut required = false;
    // How many checks the rule spelled. Each arm below overwrites `kind`, so a rule with
    // two of them silently kept the last one and dropped the first author's intent.
    let mut asserted = 0u8;
    for (key, value) in pairs {
        let key_str = scalar_str(key)?.unwrap_or(Cow::Borrowed(""));
        match key_str.as_ref() {
            "path" => {
                path = scalar_str(value)?.map(|s| s.into_owned());
            }
            "type" => {
                asserted += 1;
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let spec = TypeSpec::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid validate type '{}'. Valid: null, bool, int, float, str, map, seq",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::Type(spec));
            }
            "sequence_of" => {
                asserted += 1;
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let spec = TypeSpec::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid sequence_of type '{}'. Valid: null, bool, int, float, str, map, seq",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::SequenceOf(spec));
            }
            "mapping_of" => {
                asserted += 1;
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let spec = TypeSpec::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid mapping_of type '{}'. Valid: null, bool, int, float, str, map, seq",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::MappingOf(spec));
            }
            "required" => {
                let is_true = match value {
                    CustomNode::Scalar { value, .. } => {
                        matches!(value.as_ref(), "true" | "True" | "TRUE")
                    }
                    _ => true,
                };
                required = is_true;
            }
            _ => {}
        }
    }
    // `required` is orthogonal - it combines with a check - so only the three checks
    // conflict. Two of them in one rule means the schema said two different things about
    // one node, and only one of them was ever run.
    if asserted > 1 {
        return Err(ParseError::Syntax {
            message: "a validate rule carries one check: choose between type, sequence_of and \
                      mapping_of (`required` may be added to any of them)"
                .to_string(),
            line: 0,
            col: 0,
        });
    }
    let kind = match kind {
        Some(k) => k,
        None if required => ValidateKind::Required,
        None => {
            return Err(ParseError::Syntax {
                message: "validate rule missing one of: type, sequence_of, mapping_of, required"
                    .to_string(),
                line: 0,
                col: 0,
            });
        }
    };
    // Refuse a rule that cannot check anything rather than accept one that looks like
    // it does: with no path there is no node whose shape to assert, and a rule that
    // silently passes every document is the defect this section exists to remove.
    if path.is_none() && matches!(kind, ValidateKind::Type(spec) if spec.is_container()) {
        return Err(ParseError::Syntax {
            message: "a container type needs a path: `type: map` and `type: seq` assert the \
                      shape of a named node, while a pathless rule only selects the nodes \
                      it can check"
                .to_string(),
            line: 0,
            col: 0,
        });
    }
    Ok(ValidateRule::new(path.as_deref(), kind).with_required(required))
}

/// Check if a concrete path (e.g. `"$.tags[0]"`) matches a pattern path
/// (e.g. `"$.tags[*]"`). `None` pattern matches everything.
fn path_matches(pattern: Option<&str>, actual: &str) -> bool {
    let Some(pat) = pattern else {
        return true; // None = all scalars
    };
    if pat == actual {
        return true;
    }
    // Support `[*]` wildcard: split pattern on `[*]`, check prefix/suffix.
    if !pat.contains("[*]") {
        return false;
    }
    let parts: Vec<&str> = pat.split("[*]").collect();
    if parts.len() != 2 {
        return false; // only single [*] supported
    }
    let prefix = parts[0];
    let suffix = parts[1];
    // The span one `[*]` stands for is exactly one index: `$.rows[*]` names an element,
    // not that element's whole subtree. `starts_with` and `ends_with` alone also matched
    // `$.rows[0].a`, which was harmless while every rule ignored the nodes whose shape it
    // could not describe - and became a false positive the moment a rule asserts shape.
    let middle = actual
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix));
    let Some(middle) = middle else { return false };
    let index = middle
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'));
    match index {
        Some(digits) => !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()),
        None => false,
    }
}

/// Recursively validate a `CustomNode` AST against the validate rules in a
/// [`RuleResolver`]. Returns `Ok(())` if all rules pass, or `Err(Vec<...>)`
/// with all collected errors.
pub fn validate_node(
    ast: &CustomNode,
    resolver: &RuleResolver,
    source: &str,
) -> Result<(), Vec<SchemaValidationError>> {
    let mut errors = Vec::new();
    // Required existence checks (paths not present in the AST are skipped by
    // traversal, so check them up front).
    for rule in resolver.validate_rules() {
        if !rule.required {
            continue;
        }
        let Some(path) = rule.path.as_deref() else {
            continue;
        };
        if path_matches(Some(path), path) && !contains_path(ast, path) {
            errors.push(SchemaValidationError::new(path, "required path is missing"));
        }
    }
    validate_recursive(ast, "$", source, resolver, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check whether `path` (a `$.a.b[0]`-style path, or with `[*]` wildcards)
/// resolves to an existing node.
fn contains_path(ast: &CustomNode, path: &str) -> bool {
    let Some(segs) = rule_path_to_segments(path) else {
        // Unparseable path (e.g. wildcard) — conservatively treat as present.
        return true;
    };
    crate::editing::navigate(ast, &segs).is_ok()
}

/// Parse a `$.a.b[0][*]` path into navigate segments. Returns `None` if the
/// path contains a `[*]` wildcard (existence over wildcards is ambiguous).
fn rule_path_to_segments(path: &str) -> Option<Vec<crate::editing::Segment<'static>>> {
    use crate::editing::Segment;
    let rest = path.strip_prefix('$')?;
    // A bare `$` is the root document, and the empty-path branch below was written to handle it - but
    // requiring a separator before testing for emptiness meant `path: $` never reached that branch, so the
    // root could not be a rule target at all. Anything after `$` that is not `.` is still not a path.
    let rest = match rest.chars().next() {
        None => "",              // the bare `$`, which is the whole document
        Some('.') => &rest[1..], // `.` is one byte, so this slice is always on a boundary
        Some(_) => return None,
    };
    if rest.is_empty() {
        return Some(Vec::new()); // path "$"
    }
    let mut segs = Vec::new();
    let mut cur_key = String::new();
    let mut i = 0;
    while i < rest.len() {
        // The cursor is advanced by the decoded character's width, not by one byte. Walking a path
        // byte-wise left it inside a multi-byte key - `$.café`, `$.emoji😀key` - and the next `rest[i..]` slice
        // panicked with "start byte index is not a char boundary". A rule path is user-authored text, and user
        // text is not ASCII, so the panic was reachable from a valid schema rather than from a malformed one.
        let Some(c) = rest[i..].chars().next() else {
            break;
        };
        let width = c.len_utf8();
        match c {
            '.' => {
                if !cur_key.is_empty() {
                    segs.push(Segment::Key(std::borrow::Cow::Owned(std::mem::take(
                        &mut cur_key,
                    ))));
                }
                i += width;
            }
            '[' => {
                if !cur_key.is_empty() {
                    segs.push(Segment::Key(std::borrow::Cow::Owned(std::mem::take(
                        &mut cur_key,
                    ))));
                }
                let close = rest[i + 1..].find(']')? + i + 1;
                let inner = &rest[i + 1..close];
                if inner == "*" {
                    return None;
                }
                let idx: i64 = inner.parse().ok()?;
                segs.push(Segment::Index(idx));
                i = close + 1;
            }
            _ => {
                cur_key.push(c);
                i += width;
            }
        }
    }
    if !cur_key.is_empty() {
        segs.push(Segment::Key(std::borrow::Cow::Owned(cur_key)));
    }
    Some(segs)
}

fn validate_recursive(
    node: &CustomNode,
    path: &str,
    source: &str,
    resolver: &RuleResolver,
    errors: &mut Vec<SchemaValidationError>,
) {
    // Check rules that match this path
    for rule in resolver.validate_rules() {
        if !path_matches(rule.path.as_deref(), path) {
            continue;
        }
        if rule.required && matches!(node, CustomNode::Null { .. }) {
            errors.push(SchemaValidationError::new(
                path,
                "required value is null or missing",
            ));
            continue;
        }
        // Scope decides whether this rule has anything to say about this node at all.
        // Measured first: every shape check below was silently absent, so a scalar at
        // `$.config` under `mapping_of: str`, a mapping at `$.port` under `type: int`
        // and a nested sequence inside `sequence_of: int` all passed.
        if rule.path.is_none() && !shape_selectable(node, &rule.kind) {
            continue;
        }
        match &rule.kind {
            ValidateKind::Required => {
                if matches!(node, CustomNode::Null { .. }) {
                    errors.push(
                        SchemaValidationError::new(path, "required path is null or missing")
                            .with_location(source, node.source_range()),
                    );
                }
            }
            ValidateKind::Type(expected) => {
                if let Some(detail) = type_mismatch(node, *expected, resolver) {
                    errors.push(
                        SchemaValidationError::new(path, format!("expected {detail}"))
                            .with_location(source, node.source_range()),
                    );
                }
            }
            ValidateKind::SequenceOf(expected) => match node {
                CustomNode::Sequence { items, .. } => {
                    for (i, item) in items.iter().enumerate() {
                        let item_path = format!("{}[{}]", path, i);
                        if let Some(detail) = type_mismatch(item, *expected, resolver) {
                            errors.push(
                                SchemaValidationError::new(
                                    item_path,
                                    format!("expected sequence element {detail}"),
                                )
                                .with_location(source, item.source_range()),
                            );
                        }
                    }
                }
                other => {
                    errors.push(
                        SchemaValidationError::new(
                            path,
                            format!(
                                "expected sequence of {expected} but got {}",
                                node_shape(other)
                            ),
                        )
                        .with_location(source, other.source_range()),
                    );
                }
            },
            ValidateKind::MappingOf(expected) => match node {
                CustomNode::Mapping { pairs, .. } => {
                    for (key, val) in pairs.iter() {
                        let key_str = match key {
                            CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                            _ => "(complex)".to_string(),
                        };
                        let val_path = format!("{}.{}", path, key_str);
                        if let Some(detail) = type_mismatch(val, *expected, resolver) {
                            errors.push(
                                SchemaValidationError::new(
                                    val_path,
                                    format!("expected mapping value {detail}"),
                                )
                                .with_location(source, val.source_range()),
                            );
                        }
                    }
                }
                other => {
                    errors.push(
                        SchemaValidationError::new(
                            path,
                            format!(
                                "expected mapping of {expected} but got {}",
                                node_shape(other)
                            ),
                        )
                        .with_location(source, other.source_range()),
                    );
                }
            },
        }
    }

    // Recurse into children
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, val) in pairs.iter() {
                let key_str = match key {
                    CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                    _ => "(complex)".to_string(),
                };
                let child_path = format!("{}.{}", path, key_str);
                validate_recursive(val, &child_path, source, resolver, errors);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                let child_path = format!("{}[{}]", path, i);
                validate_recursive(item, &child_path, source, resolver, errors);
            }
        }
        _ => {}
    }
}

/// Check if a resolved `YamlType` matches an expected `YamlTypeKind`.
fn yaml_type_matches(resolved: &YamlType, expected: YamlTypeKind) -> bool {
    matches!(
        (resolved, expected),
        (YamlType::Null, YamlTypeKind::Null)
            | (YamlType::Bool(_), YamlTypeKind::Bool)
            | (YamlType::Int(_), YamlTypeKind::Int)
            | (YamlType::Float(_), YamlTypeKind::Float)
            | (YamlType::Str(_), YamlTypeKind::Str)
    )
}

/// The shape a node has, named the way the schema language names it.
fn node_shape(node: &CustomNode) -> &'static str {
    match node {
        CustomNode::Scalar { .. } => "scalar",
        CustomNode::Mapping { .. } => "mapping",
        CustomNode::Sequence { .. } => "sequence",
        CustomNode::Null { .. } => "null",
        CustomNode::Alias { .. } => "alias",
    }
}

/// Whether a pathless rule can describe this node. Pathless rules select by shape
/// instead of asserting it - that is what `type: str` with no path has always meant in
/// practice: every scalar is a string, whatever else the document holds. A rule that
/// does name a path never consults this, because then the shape is the claim.
fn shape_selectable(node: &CustomNode, kind: &ValidateKind) -> bool {
    match kind {
        ValidateKind::Required => true,
        ValidateKind::Type(TypeSpec::Scalar(YamlTypeKind::Null)) => {
            matches!(node, CustomNode::Scalar { .. } | CustomNode::Null { .. })
        }
        ValidateKind::Type(_) => matches!(node, CustomNode::Scalar { .. }),
        ValidateKind::SequenceOf(_) => matches!(node, CustomNode::Sequence { .. }),
        ValidateKind::MappingOf(_) => matches!(node, CustomNode::Mapping { .. }),
    }
}

/// How a node falls short of what a rule expects, if it does. The wording is returned
/// rather than the error so that all three positions read the way they always have -
/// `expected int but got Str("x")`, `expected sequence element int but got ...` - with
/// the shape added to the same vocabulary.
fn type_mismatch(node: &CustomNode, expected: TypeSpec, resolver: &RuleResolver) -> Option<String> {
    match (expected, node) {
        // An alias names another node; the value it stands for is not here, and the
        // validator holds no anchor table to look it up in. Reported as a pass rather
        // than a guess, which is a documented boundary, not an oversight.
        (_, CustomNode::Alias { .. }) => None,
        (TypeSpec::Mapping, CustomNode::Mapping { .. })
        | (TypeSpec::Sequence, CustomNode::Sequence { .. }) => None,
        // `null` is both a resolved scalar and a node kind, so either spelling satisfies
        // a rule that wants a null.
        (TypeSpec::Scalar(YamlTypeKind::Null), CustomNode::Null { .. }) => None,
        (TypeSpec::Scalar(kind), CustomNode::Scalar { value, .. }) => {
            let resolved = resolver.resolve(value.as_ref());
            if yaml_type_matches(&resolved, kind) {
                None
            } else {
                Some(format!("{kind} but got {resolved:?}"))
            }
        }
        (expected, other) => Some(format!("{expected} but got {}", node_shape(other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve<'a>(r: &'a RuleResolver, v: &'a str) -> YamlType<'a> {
        r.resolve(v)
    }

    #[test]
    fn a_rule_path_with_a_multibyte_key_is_walked_by_character() {
        // Measured as a panic before the cursor advanced by `len_utf8`: the byte-wise walk stopped inside
        // 'é' and the next slice aborted with "start byte index is not a char boundary". A rule path is
        // user-authored text, so this was reachable from a valid schema, not a malformed one.
        use crate::editing::Segment;

        let segments = rule_path_to_segments("$.café").expect("path parses");
        assert_eq!(segments.len(), 1);
        assert!(
            matches!(&segments[0], Segment::Key(key) if key.as_ref() == "café"),
            "{segments:?}"
        );

        let nested = rule_path_to_segments("$.emoji😀key[2].port").expect("path parses");
        let names: Vec<String> = nested
            .iter()
            .map(|segment| match segment {
                Segment::Key(key) => key.to_string(),
                Segment::Index(index) => format!("[{index}]"),
            })
            .collect();
        assert_eq!(names, ["emoji😀key", "[2]", "port"]);
    }

    #[test]
    fn a_rule_path_at_its_own_end_is_not_a_panic() {
        // The boundary shapes around the walk: a path with nothing after `$`, and one that stops at a
        // separator, must both answer rather than index past the last character.
        assert!(rule_path_to_segments("$").unwrap().is_empty());
        assert!(rule_path_to_segments("$.").unwrap().is_empty());
        assert_eq!(rule_path_to_segments("$.a.").unwrap().len(), 1);
        assert_eq!(rule_path_to_segments("$.é.").unwrap().len(), 1);
        assert_eq!(rule_path_to_segments("$.café.x").unwrap().len(), 2);
        // `$` with no separator and no key is the root; `$x` is not a path at all.
        assert!(rule_path_to_segments("$x").is_none());
    }

    #[test]
    fn test_hex_int_rule() {
        let yaml = r#"
name: hex
extends: core
rules:
  - pattern: "^0x[0-9a-fA-F]+$"
    type: int
"#;
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        assert_eq!(resolve(&resolver, "0x1F"), YamlType::Int(31));
        assert_eq!(resolve(&resolver, "0xFF"), YamlType::Int(255));
        // non-matching falls back to core
        assert_eq!(resolve(&resolver, "42"), YamlType::Int(42));
        assert_eq!(resolve(&resolver, "hello"), YamlType::Str("hello".into()));
    }

    #[test]
    fn test_date_str_rule_overrides_core() {
        let yaml = r#"
name: dates
extends: core
rules:
  - pattern: "^\\d{4}-\\d{2}-\\d{2}$"
    type: str
"#;
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        // 2026-08-11 would be int under core (starts with digit), but matches str rule
        assert_eq!(
            resolve(&resolver, "2026-08-11"),
            YamlType::Str("2026-08-11".into())
        );
    }

    #[test]
    fn test_bool_lexemes() {
        let yaml = r#"
name: bools
extends: failsafe
rules:
  - pattern: "^(yes|no|Yes|No|YES|NO)$"
    type: bool
"#;
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        assert_eq!(resolve(&resolver, "yes"), YamlType::Bool(true));
        assert_eq!(resolve(&resolver, "no"), YamlType::Bool(false));
        assert_eq!(resolve(&resolver, "YES"), YamlType::Bool(true));
        // non-matching under failsafe stays a string
        assert_eq!(resolve(&resolver, "42"), YamlType::Str("42".into()));
    }

    #[test]
    fn test_first_rule_wins() {
        let yaml = r#"
name: order
extends: core
rules:
  - pattern: "^0x[0-9a-fA-F]+$"
    type: int
  - pattern: "^0x.*$"
    type: str
"#;
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        // First rule matches 0x1F -> int
        assert_eq!(resolve(&resolver, "0x1F"), YamlType::Int(31));
    }

    #[test]
    fn test_invalid_type_rejected() {
        let yaml = r#"
name: bad
rules:
  - pattern: "^x$"
    type: datetime
"#;
        assert!(parse_schema_yaml(yaml).is_err());
    }

    #[test]
    fn test_invalid_regex_rejected() {
        let yaml = r#"
name: bad
rules:
  - pattern: "[unclosed"
    type: int
"#;
        assert!(parse_schema_yaml(yaml).is_err());
    }

    #[test]
    fn test_no_rules_falls_through() {
        let yaml = "name: empty\nextends: core\nrules: []\n";
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        assert_eq!(resolve(&resolver, "42"), YamlType::Int(42));
    }

    #[test]
    fn test_invalid_yaml_syntax() {
        assert!(parse_schema_yaml("not: valid: yaml: [[[}").is_err());
        assert!(parse_schema_yaml("rules: [broken").is_err());
    }

    #[test]
    fn test_invalid_extends() {
        let yaml = "name: bad\nrules:\n  - pattern: ^x$\n    type: int\n";
        let resolver = parse_schema_yaml(yaml).expect("parse schema");
        // No extends specified — defaults to no fallback
        assert_eq!(
            resolve(&resolver, "hello"),
            YamlType::Str(Cow::Borrowed("hello"))
        );
    }

    // --- shape assertions -------------------------------------------------------
    //
    // The ruling that closed the ROADMAP's open item. Its baseline was measured before
    // the engine changed: a document that disagrees with the shape a rule names used to
    // produce no complaint at all, because each check sat inside an `if let` for the one
    // node kind it could describe.

    /// A document parsed the way the public `validate_against_schema` parses it.
    fn document(src: &str) -> CustomNode {
        crate::parser::parse_with_options(src, true, Schema::Core, 1000, false)
            .expect("document parses")
    }

    /// A schema whose only content is these validate rules.
    fn validate_schema(rules: &str) -> String {
        format!("name: shape\nextends: core\nvalidate:\n{rules}")
    }

    /// Every complaint the schema makes about the document, as `path: message`.
    fn complaints(schema_yaml: &str, src: &str) -> Vec<String> {
        let resolver = parse_schema_yaml(schema_yaml).expect("schema parses");
        match validate_node(&document(src), &resolver, src) {
            Ok(()) => Vec::new(),
            Err(errors) => errors
                .into_iter()
                .map(|error| format!("{}: {}", error.path, error.message))
                .collect(),
        }
    }

    /// The message of a schema the parser refused. Answering `Ok` is the test failure, and
    /// spelling it as a `match` is also what keeps the assertion out of the shape CI's clippy
    /// rejects: `.err().expect()` on a `Result` is its own lint.
    fn refused(result: Result<RuleResolver, ParseError>) -> String {
        match result {
            Ok(_) => panic!("the schema should be refused"),
            Err(error) => format!("{error}"),
        }
    }

    #[test]
    fn a_path_qualified_container_rule_asserts_the_container() {
        let mapping_rule = "  - path: $.config\n    mapping_of: str\n";
        // The shapes `mapping_of` used to walk past, one assertion each.
        assert_eq!(
            complaints(&validate_schema(mapping_rule), "config: hello\n"),
            ["$.config: expected mapping of str but got scalar".to_string()]
        );
        assert_eq!(
            complaints(&validate_schema(mapping_rule), "config: [a, b]\n"),
            ["$.config: expected mapping of str but got sequence".to_string()]
        );

        let sequence_rule = "  - path: $.numbers\n    sequence_of: int\n";
        assert_eq!(
            complaints(&validate_schema(sequence_rule), "numbers: 5\n"),
            ["$.numbers: expected sequence of int but got scalar".to_string()]
        );
        assert_eq!(
            complaints(&validate_schema(sequence_rule), "numbers: {a: 1}\n"),
            ["$.numbers: expected sequence of int but got mapping".to_string()]
        );

        // The documents these rules were written for still pass, which is what makes
        // the four assertions above about shape rather than about member checks.
        assert!(complaints(&validate_schema(mapping_rule), "config: {a: x, b: y}\n").is_empty());
        assert!(complaints(&validate_schema(sequence_rule), "numbers: [1, 2]\n").is_empty());
    }

    #[test]
    fn a_path_qualified_type_rule_asserts_that_the_node_is_a_scalar() {
        let rule = "  - path: $.port\n    type: int\n";
        assert_eq!(
            complaints(&validate_schema(rule), "port: {a: 1}\n"),
            ["$.port: expected int but got mapping".to_string()]
        );
        assert_eq!(
            complaints(&validate_schema(rule), "port: [1, 2]\n"),
            ["$.port: expected int but got sequence".to_string()]
        );
        // The member-style wording is unchanged, so a reader who knows one form knows
        // the other.
        assert_eq!(
            complaints(&validate_schema(rule), "port: eight\n"),
            ["$.port: expected int but got Str(\"eight\")".to_string()]
        );
    }

    #[test]
    fn container_kinds_are_names_the_language_accepts() {
        // `type: map` says the shape and nothing inside it - the only way to make that
        // claim before these kinds existed was to also constrain every member.
        let map = "  - path: $.config\n    type: map\n";
        assert!(complaints(&validate_schema(map), "config: {a: 1}\n").is_empty());
        // An empty mapping is still a mapping: the assertion is about the node.
        assert!(complaints(&validate_schema(map), "config: {}\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(map), "config: [1]\n"),
            ["$.config: expected map but got sequence".to_string()]
        );
        assert_eq!(
            complaints(&validate_schema(map), "config: 1\n"),
            ["$.config: expected map but got scalar".to_string()]
        );

        let seq = "  - path: $.items\n    type: seq\n";
        assert!(complaints(&validate_schema(seq), "items: [1, 2]\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(seq), "items: {a: 1}\n"),
            ["$.items: expected seq but got mapping".to_string()]
        );

        // A schema is hand-written text, so the long names and the ordinary synonyms
        // parse to the same claim.
        for (spelling, doc, want_sequence) in [
            ("mapping", "x: {a: 1}\n", false),
            ("object", "x: {a: 1}\n", false),
            ("sequence", "x: [1]\n", true),
            ("array", "x: [1]\n", true),
            ("list", "x: [1]\n", true),
        ] {
            let rule = format!("  - path: $.x\n    type: {spelling}\n");
            let against = if want_sequence {
                "x: {a: 1}\n"
            } else {
                "x: [1]\n"
            };
            assert!(
                complaints(&validate_schema(&rule), doc).is_empty(),
                "{spelling} accepted {doc}"
            );
            let found = complaints(&validate_schema(&rule), against);
            assert_eq!(found.len(), 1, "{spelling} vs {against}: {found:?}");
        }
    }

    #[test]
    fn a_member_can_be_asked_for_its_shape() {
        // "every value of config must itself be a sequence" had no spelling at all: the
        // wildcard `[*]` addresses sequence indices, not mapping keys.
        let rule = "  - path: $.outer\n    mapping_of: seq\n";
        assert!(complaints(&validate_schema(rule), "outer:\n  a: [1]\n  b: [2]\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(rule), "outer:\n  a: 1\n"),
            ["$.outer.a: expected mapping value seq but got scalar".to_string()]
        );

        let of_map = "  - path: $.rows\n    sequence_of: map\n";
        assert!(complaints(&validate_schema(of_map), "rows:\n  - a: 1\n  - b: 2\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(of_map), "rows:\n  - 1\n"),
            ["$.rows[0]: expected sequence element map but got scalar".to_string()]
        );
    }

    #[test]
    fn a_container_member_is_not_an_element_of_a_scalar_type() {
        // The same false negative one level down: `sequence_of: int` had already said
        // what its elements are, and a nested sequence is not one of them.
        assert_eq!(
            complaints(
                &validate_schema("  - path: $.n\n    sequence_of: int\n"),
                "n:\n  - 1\n  - [2, 3]\n"
            ),
            ["$.n[1]: expected sequence element int but got sequence".to_string()]
        );
        assert_eq!(
            complaints(
                &validate_schema("  - path: $.c\n    mapping_of: str\n"),
                "c:\n  a: x\n  b:\n    deep: y\n"
            ),
            ["$.c.b: expected mapping value str but got mapping".to_string()]
        );
        // A wildcard path asserts per matched node, which is how a list of records is
        // described: every element a mapping, every value an int.
        assert!(
            complaints(
                &validate_schema("  - path: $.rows[*]\n    mapping_of: int\n"),
                "rows:\n  - a: 1\n  - b: 2\n"
            )
            .is_empty()
        );
        assert_eq!(
            complaints(
                &validate_schema("  - path: $.rows[*]\n    mapping_of: int\n"),
                "rows:\n  - a: 1\n  - scal\n"
            ),
            ["$.rows[1]: expected mapping of int but got scalar".to_string()]
        );
    }

    #[test]
    fn a_rule_cannot_say_two_things_about_one_node() {
        // Every arm of the parser used to overwrite `kind`, so such a rule ran only the last
        // check and stayed silent about the one written first.
        for body in [
            "  - path: $.x\n    type: int\n    mapping_of: str\n",
            "  - path: $.x\n    sequence_of: int\n    type: seq\n",
            "  - path: $.x\n    mapping_of: str\n    sequence_of: str\n",
        ] {
            let message = refused(parse_schema_yaml(&validate_schema(body)));
            assert!(message.contains("one check"), "{body}: {message}");
        }
        // `required` is not a second check - it combines with one, as the guides document.
        assert!(
            parse_schema_yaml(&validate_schema(
                "  - path: $.x\n    type: int\n    required: true\n"
            ))
            .is_ok()
        );
    }

    #[test]
    fn a_located_complaint_still_names_the_path_it_matched() {
        // The location points into the source, the path names the target the schema author
        // wrote; only one of them can be matched against the schema text.
        let schema = validate_schema("  - path: $.rows[*]\n    type: map\n");
        let resolver = parse_schema_yaml(&schema).expect("schema parses");
        let src = "rows:\n  - 1\n";
        let errors = match validate_node(&document(src), &resolver, src) {
            Ok(()) => panic!("the rule reports the element the wildcard names"),
            Err(errors) => errors,
        };
        assert_eq!(errors.len(), 1);
        let shown = errors[0].to_string();
        assert!(
            shown.contains("$.rows[0]: expected map but got scalar"),
            "{shown}"
        );
        let location = shown.split("$.rows").next().unwrap_or_default();
        assert!(location.contains(':'), "{shown}");
        assert!(
            location
                .chars()
                .all(|c| c.is_ascii_digit() || c == ':' || c == ' '),
            "{shown}"
        );
    }

    #[test]
    fn a_wildcard_names_one_element_not_a_whole_subtree() {
        // `$.rows[*]` is a claim about each element. The old matcher let the same pattern
        // hold for `$.rows[0].a` as well, which was invisible while rules ignored shapes
        // and became a false positive the moment they assert one.
        let rule = "  - path: $.rows[*]\n    type: map\n";
        assert!(complaints(&validate_schema(rule), "rows:\n  - a: 1\n  - b: x\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(rule), "rows:\n  - 1\n  - {a: 2}\n"),
            ["$.rows[0]: expected map but got scalar".to_string()]
        );
        // One index only: a sequence nested inside an element is not that element.
        assert_eq!(
            complaints(&validate_schema(rule), "rows:\n  - [{a: 1}]\n"),
            ["$.rows[0]: expected map but got sequence".to_string()]
        );
        // A suffix after the wildcard still reaches the member it names.
        let named = "  - path: $.rows[*].a\n    type: int\n";
        assert!(complaints(&validate_schema(named), "rows:\n  - a: 1\n  - a: 2\n").is_empty());
        assert_eq!(
            complaints(&validate_schema(named), "rows:\n  - a: one\n"),
            ["$.rows[0].a: expected int but got Str(\"one\")".to_string()]
        );
    }

    #[test]
    fn a_pathless_rule_selects_by_shape_instead_of_asserting_it() {
        // Pathless `type:` has always meant "every scalar resolves to this", which a
        // document with nested structures has to be allowed to satisfy.
        let scalars = "  - type: str\n";
        assert!(
            complaints(
                &validate_schema(scalars),
                "top: scalar\nnested:\n  deep: also-scalar\n"
            )
            .is_empty()
        );
        // Pathless container rules still check members - that is a claim about members,
        // not about the shape of a node they never named.
        let found = complaints(
            &validate_schema("  - mapping_of: str\n"),
            "plain: 5\nmapped: {k: v}\nnested: {m: {deep: x}}\n",
        );
        assert!(
            found.contains(&"$.plain: expected mapping value str but got Int(5)".to_string()),
            "{found:?}"
        );
        assert!(
            found.contains(&"$.mapped: expected mapping value str but got mapping".to_string()),
            "{found:?}"
        );
        assert!(
            found.contains(&"$.nested.m: expected mapping value str but got mapping".to_string()),
            "{found:?}"
        );
    }

    #[test]
    fn a_pathless_container_shape_is_refused_where_it_is_written() {
        // A pathless `type: map` would have to hold for every node in the document,
        // which no document can be. Accepting it would add a rule that checks nothing -
        // the very shape of bug this ruling removes - so the schema fails to parse.
        for spelling in ["map", "seq", "mapping", "array"] {
            let yaml = format!("name: shape\nextends: core\nvalidate:\n  - type: {spelling}\n");
            let message = refused(parse_schema_yaml(&yaml));
            assert!(message.contains("needs a path"), "{spelling}: {message}");
        }
        // With a path it is a claim about a node, and parses.
        assert!(
            parse_schema_yaml(
                "name: shape\nextends: core\nvalidate:\n  - path: $.x\n    type: map\n"
            )
            .is_ok()
        );
    }

    #[test]
    fn a_pattern_still_resolves_only_scalar_text() {
        // `rules:` patterns match the text of a scalar, so the container names are not
        // offered there even though validate rules accept them.
        let yaml = "name: shape\nrules:\n  - pattern: ^x$\n    type: map\n";
        let message = refused(parse_schema_yaml(yaml));
        assert!(
            message.contains("Valid: null, bool, int, float, str"),
            "{message}"
        );
        assert!(message.contains("validate rule"), "{message}");
    }

    #[test]
    fn an_alias_is_passed_rather_than_guessed() {
        // An alias node does not carry the value it names and the validator holds no
        // anchor table, so a shape rule about an aliased node cannot be decided. The
        // rules below therefore pass, deliberately: this test is the record of that
        // boundary, and a future alias-aware validator has to break it on purpose.
        let rule = "  - path: $.b\n    type: int\n";
        assert!(complaints(&validate_schema(rule), "a: &x 5\nb: *x\n").is_empty());
        assert!(complaints(&validate_schema(rule), "a: &x hi\nb: *x\n").is_empty());
        assert!(
            complaints(
                &validate_schema("  - path: $.b\n    type: map\n"),
                "a: &x hi\nb: *x\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_null_is_satisfied_by_either_spelling_of_null() {
        // `null` is both a scalar that resolves to Null and a node kind, and a rule
        // written for one should not fail on the other.
        let rule = "  - path: $.port\n    type: null\n";
        assert!(complaints(&validate_schema(rule), "port:\n").is_empty());
        assert!(complaints(&validate_schema(rule), "port: null\n").is_empty());
        assert!(
            complaints(&validate_schema(rule), "port: {a: 1}\n")
                .iter()
                .any(|c| c.contains("expected null but got mapping"))
        );
        // A rule wanting any other type still reports the empty value rather than
        // skipping it, which is the behaviour the ruling had to preserve.
        assert!(
            complaints(
                &validate_schema("  - path: $.port\n    type: int\n"),
                "port:\n"
            )
            .iter()
            .any(|c| c.contains("expected int but got Null"))
        );
    }
}
