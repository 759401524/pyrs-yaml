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

/// A schema resolver built from a list of rules, with an optional fallback
/// schema for scalars that match no rule.
#[derive(Clone)]
pub struct RuleResolver {
    rules: Vec<Rule>,
    fallback: Option<Schema>,
}

impl RuleResolver {
    /// Build a resolver from rules and an optional fallback schema.
    pub fn new(rules: Vec<Rule>, fallback: Option<Schema>) -> Self {
        Self { rules, fallback }
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
            _ => {} // ignore unknown top-level keys (name, version, ...)
        }
    }

    Ok(RuleResolver::new(rules, extends))
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
