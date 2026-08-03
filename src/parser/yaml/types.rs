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

/// YAML 1.2 type resolution for plain scalars
#[derive(Debug, Clone, PartialEq)]
pub enum YamlType {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (decimal, octal, hex)
    Int(i64),
    /// Float value (including infinity and NaN)
    Float(f64),
    /// String value (default)
    Str(String),
}

#[cfg(test)]
impl YamlType {
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
    use crate::parser::yaml::schema::resolve_yaml_type;
    use crate::parser::yaml::YamlSchema;

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
            YamlType::Str(s) => s.clone(),
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
            YamlType::Str("hello".to_string())
        );
        assert_eq!(
            resolve_yaml_type("12abc", YamlSchema::Core),
            YamlType::Str("12abc".to_string())
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
