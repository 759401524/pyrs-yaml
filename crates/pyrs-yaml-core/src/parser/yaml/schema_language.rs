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

/// The kind of structural validation a [`ValidateRule`] performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateKind {
    /// Scalar value at this path must resolve to the given type.
    Type(YamlTypeKind),
    /// Sequence at this path must contain only elements of the given type.
    SequenceOf(YamlTypeKind),
    /// Mapping at this path must have values of the given type.
    MappingOf(YamlTypeKind),
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
        if let Some(ref line) = self.line {
            write!(f, "{}:{}: {}", line, self.column.unwrap_or(1), self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
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
        let trimmed = value.trim();
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
                            "invalid schema type '{}'. Valid: null, bool, int, float, str",
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
    for (key, value) in pairs {
        let key_str = scalar_str(key)?.unwrap_or(Cow::Borrowed(""));
        match key_str.as_ref() {
            "path" => {
                path = scalar_str(value)?.map(|s| s.into_owned());
            }
            "type" => {
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let k = YamlTypeKind::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid validate type '{}'. Valid: null, bool, int, float, str",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::Type(k));
            }
            "sequence_of" => {
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let k = YamlTypeKind::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid sequence_of type '{}'. Valid: null, bool, int, float, str",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::SequenceOf(k));
            }
            "mapping_of" => {
                let ty = scalar_str(value)?.unwrap_or(Cow::Borrowed(""));
                let k = YamlTypeKind::from_name(ty.as_ref()).ok_or_else(|| ParseError::Syntax {
                    message: format!(
                        "invalid mapping_of type '{}'. Valid: null, bool, int, float, str",
                        ty
                    ),
                    line: 0,
                    col: 0,
                })?;
                kind = Some(ValidateKind::MappingOf(k));
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
    actual.starts_with(prefix)
        && actual.ends_with(suffix)
        && actual.len() > prefix.len() + suffix.len()
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
    let rest = rest.strip_prefix('.')?;
    if rest.is_empty() {
        return Some(Vec::new()); // path "$"
    }
    let mut segs = Vec::new();
    let mut cur_key = String::new();
    let mut i = 0;
    while i < rest.len() {
        let c = rest[i..].chars().next().unwrap();
        match c {
            '.' => {
                if !cur_key.is_empty() {
                    segs.push(Segment::Key(std::borrow::Cow::Owned(std::mem::take(
                        &mut cur_key,
                    ))));
                }
                i += 1;
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
                i += 1;
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
                if let CustomNode::Scalar { value, .. } = node {
                    let resolved = resolver.resolve(value.as_ref());
                    if !yaml_type_matches(&resolved, *expected) {
                        errors.push(
                            SchemaValidationError::new(
                                path,
                                format!("expected {} but got {:?}", expected, resolved),
                            )
                            .with_location(source, node.source_range()),
                        );
                    }
                }
            }
            ValidateKind::SequenceOf(expected) => {
                if let CustomNode::Sequence { items, .. } = node {
                    for (i, item) in items.iter().enumerate() {
                        let item_path = format!("{}[{}]", path, i);
                        if let CustomNode::Scalar { value, .. } = item {
                            let resolved = resolver.resolve(value.as_ref());
                            if !yaml_type_matches(&resolved, *expected) {
                                errors.push(
                                    SchemaValidationError::new(
                                        item_path,
                                        format!(
                                            "expected sequence element {} but got {:?}",
                                            expected, resolved
                                        ),
                                    )
                                    .with_location(source, item.source_range()),
                                );
                            }
                        }
                    }
                }
            }
            ValidateKind::MappingOf(expected) => {
                if let CustomNode::Mapping { pairs, .. } = node {
                    for (key, val) in pairs.iter() {
                        let key_str = match key {
                            CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                            _ => "(complex)".to_string(),
                        };
                        let val_path = format!("{}.{}", path, key_str);
                        if let CustomNode::Scalar { value, .. } = val {
                            let resolved = resolver.resolve(value.as_ref());
                            if !yaml_type_matches(&resolved, *expected) {
                                errors.push(
                                    SchemaValidationError::new(
                                        val_path,
                                        format!(
                                            "expected mapping value {} but got {:?}",
                                            expected, resolved
                                        ),
                                    )
                                    .with_location(source, val.source_range()),
                                );
                            }
                        }
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve<'a>(r: &'a RuleResolver, v: &'a str) -> YamlType<'a> {
        r.resolve(v)
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
}
