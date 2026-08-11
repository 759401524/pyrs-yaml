use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;

/// YAML schema profile controlling implicit type resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlSchema {
    /// No implicit resolution — every plain scalar is a string.
    Failsafe,
    /// JSON-compatible subset (no inf, nan, 0x, 0o).
    Json,
    /// YAML 1.2 Core — default behavior.
    Core,
    /// YAML 1.1 — adds legacy boolean lexemes (yes/no, on/off, y/n).
    Yaml1_1,
}

/// Extension point for custom schema resolvers.
///
/// Implement this trait to define a custom YAML schema resolution strategy.
/// The resolver is called for each plain scalar to determine its YAML type.
pub trait SchemaResolver: Send + Sync {
    /// Resolve a plain scalar value to its YAML type.
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a>;
}

/// A resolved schema: either a built-in variant (zero-cost dispatch) or a
/// custom resolver registered via [`SchemaRegistry`].
#[derive(Clone)]
pub enum Schema {
    /// No implicit resolution — every plain scalar is a string.
    Failsafe,
    /// JSON-compatible subset (no inf, nan, 0x, 0o).
    Json,
    /// YAML 1.2 Core — default behavior.
    Core,
    /// YAML 1.1 — adds legacy boolean lexemes (yes/no, on/off, y/n).
    Yaml1_1,
    /// Custom resolver registered via the registry.
    Custom(Arc<dyn SchemaResolver>),
}

impl Schema {
    /// Resolve a plain scalar value according to this schema.
    pub fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        match self {
            Schema::Custom(r) => r.resolve(value),
            Schema::Failsafe => resolve_schema_fn::<0>(value),
            Schema::Json => resolve_schema_fn::<1>(value),
            Schema::Core => resolve_schema_fn::<2>(value),
            Schema::Yaml1_1 => resolve_schema_fn::<3>(value),
        }
    }
}

/// Parse a schema name (case-insensitive) into a built-in [`Schema`] variant.
///
/// Returns `None` for `Custom` schemas — those are looked up in the
/// [`SchemaRegistry`] by name instead.
impl FromStr for Schema {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "core" | "yaml.org,2002" | "yamlorg2002" => Ok(Schema::Core),
            "json" | "yaml.org,2002:json" => Ok(Schema::Json),
            "failsafe" | "yaml.org,2002:failsafe" => Ok(Schema::Failsafe),
            "yaml1.1" | "1.1" | "yaml.org,2002:yaml1.1" => Ok(Schema::Yaml1_1),
            _ => Err(()),
        }
    }
}

/// Const-generic dispatch to a built-in resolver function. The return lifetime
/// is tied to the input slice, not to any enclosing borrow.
fn resolve_schema_fn<'a, const IDX: u8>(value: &'a str) -> YamlType<'a> {
    match IDX {
        0 => crate::parser::yaml::schema::resolve_failsafe(value),
        1 => crate::parser::yaml::schema::resolve_json_type(value),
        2 => crate::parser::yaml::schema::resolve_core_type(value),
        _ => crate::parser::yaml::schema::resolve_yaml11_type(value),
    }
}

impl From<YamlSchema> for Schema {
    fn from(s: YamlSchema) -> Self {
        match s {
            YamlSchema::Failsafe => Schema::Failsafe,
            YamlSchema::Json => Schema::Json,
            YamlSchema::Core => Schema::Core,
            YamlSchema::Yaml1_1 => Schema::Yaml1_1,
        }
    }
}

impl PartialEq for Schema {
    /// Built-in variants compare by kind. [`Schema::Custom`] always compares
    /// unequal to any other `Custom` (including itself) because trait objects
    /// are not comparable — two custom schemas with identical rules are still
    /// distinct. Equality is only meaningful for the built-in variants.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Schema::Failsafe, Schema::Failsafe) => true,
            (Schema::Json, Schema::Json) => true,
            (Schema::Core, Schema::Core) => true,
            (Schema::Yaml1_1, Schema::Yaml1_1) => true,
            (Schema::Custom(_), Schema::Custom(_)) => false,
            _ => false,
        }
    }
}

/// YAML 1.2 type resolution for plain scalars
#[derive(Debug, Clone, PartialEq)]
pub enum YamlType<'a> {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (decimal, octal, hex)
    Int(i64),
    /// Float value (including infinity and NaN)
    Float(f64),
    /// String value (default). Borrowed when possible to avoid allocation.
    Str(Cow<'a, str>),
}

#[cfg(test)]
impl YamlType<'_> {
    /// Check if this is a NaN value (since f64 NaN != NaN via PartialEq)
    pub fn is_nan_value(&self) -> bool {
        match self {
            YamlType::Float(v) => v.is_nan(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::yaml::YamlSchema;
    use crate::parser::yaml::schema::resolve_yaml_type;

    fn format_yaml_type(ty: &YamlType) -> String {
        match ty {
            YamlType::Null => "null".to_string(),
            YamlType::Bool(true) => "true".to_string(),
            YamlType::Bool(false) => "false".to_string(),
            YamlType::Int(val) => val.to_string(),
            YamlType::Float(val) => {
                if val.is_infinite() {
                    if *val > 0.0 {
                        ".inf".to_string()
                    } else {
                        "-.inf".to_string()
                    }
                } else if val.is_nan() {
                    ".nan".to_string()
                } else if *val == val.floor() && val.abs() < 1e15 {
                    format!("{}", *val as i64)
                } else {
                    val.to_string()
                }
            }
            YamlType::Str(s) => s.to_string(),
        }
    }

    #[test]
    fn test_resolve_null_variants() {
        assert_eq!(resolve_yaml_type("", YamlSchema::Core), YamlType::Null);
        assert_eq!(resolve_yaml_type("null", YamlSchema::Core), YamlType::Null);
        assert_eq!(resolve_yaml_type("Null", YamlSchema::Core), YamlType::Null);
        assert_eq!(resolve_yaml_type("NULL", YamlSchema::Core), YamlType::Null);
        assert_eq!(resolve_yaml_type("~", YamlSchema::Core), YamlType::Null);
    }

    #[test]
    fn test_resolve_bool() {
        assert_eq!(
            resolve_yaml_type("true", YamlSchema::Core),
            YamlType::Bool(true)
        );
        assert_eq!(
            resolve_yaml_type("True", YamlSchema::Core),
            YamlType::Bool(true)
        );
        assert_eq!(
            resolve_yaml_type("TRUE", YamlSchema::Core),
            YamlType::Bool(true)
        );
        assert_eq!(
            resolve_yaml_type("false", YamlSchema::Core),
            YamlType::Bool(false)
        );
        assert_eq!(
            resolve_yaml_type("False", YamlSchema::Core),
            YamlType::Bool(false)
        );
        assert_eq!(
            resolve_yaml_type("FALSE", YamlSchema::Core),
            YamlType::Bool(false)
        );
    }

    #[test]
    fn test_resolve_int_decimal() {
        assert_eq!(resolve_yaml_type("42", YamlSchema::Core), YamlType::Int(42));
        assert_eq!(
            resolve_yaml_type("-10", YamlSchema::Core),
            YamlType::Int(-10)
        );
        assert_eq!(resolve_yaml_type("0", YamlSchema::Core), YamlType::Int(0));
    }

    #[test]
    fn test_resolve_int_octal() {
        assert_eq!(
            resolve_yaml_type("0o10", YamlSchema::Core),
            YamlType::Int(8)
        );
        assert_eq!(
            resolve_yaml_type("0O77", YamlSchema::Core),
            YamlType::Int(63)
        );
    }

    #[test]
    fn test_resolve_int_hex() {
        assert_eq!(
            resolve_yaml_type("0xFF", YamlSchema::Core),
            YamlType::Int(255)
        );
        assert_eq!(
            resolve_yaml_type("0X0A", YamlSchema::Core),
            YamlType::Int(10)
        );
    }

    #[test]
    fn test_resolve_float() {
        match resolve_yaml_type("3.14", YamlSchema::Core) {
            // Tests parsing the string "3.14"; the expected value IS 3.14, not PI
            #[allow(clippy::approx_constant)]
            YamlType::Float(v) => assert!((v - 3.14).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
        match resolve_yaml_type("1e10", YamlSchema::Core) {
            YamlType::Float(v) => assert!((v - 1e10).abs() < 1.0),
            other => panic!("expected Float, got {:?}", other),
        }
        match resolve_yaml_type("-1.5E-3", YamlSchema::Core) {
            YamlType::Float(v) => assert!((v - (-1.5e-3)).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_infinity_nan() {
        assert_eq!(
            resolve_yaml_type(".inf", YamlSchema::Core),
            YamlType::Float(f64::INFINITY)
        );
        assert_eq!(
            resolve_yaml_type("-.inf", YamlSchema::Core),
            YamlType::Float(f64::NEG_INFINITY)
        );
        assert!(resolve_yaml_type(".nan", YamlSchema::Core).is_nan_value());
        assert!(resolve_yaml_type("nan", YamlSchema::Core).is_nan_value());
    }

    #[test]
    fn test_resolve_string() {
        assert_eq!(
            resolve_yaml_type("hello", YamlSchema::Core),
            YamlType::Str(Cow::from("hello"))
        );
        assert_eq!(
            resolve_yaml_type("12abc", YamlSchema::Core),
            YamlType::Str(Cow::from("12abc"))
        );
    }

    #[test]
    fn test_format_roundtrip() {
        let cases = vec![
            "null", "true", "false", "42", "-10", "3.14", ".inf", "-.inf", ".nan",
        ];
        for input in cases {
            let resolved = resolve_yaml_type(input, YamlSchema::Core);
            let formatted = format_yaml_type(&resolved);
            let re_resolved = resolve_yaml_type(&formatted, YamlSchema::Core);
            assert_eq!(
                format!("{:?}", resolved),
                format!("{:?}", re_resolved),
                "roundtrip failed for {:?}: resolved={:?}, formatted={:?}, re_resolved={:?}",
                input,
                resolved,
                formatted,
                re_resolved
            );
        }
    }
}
