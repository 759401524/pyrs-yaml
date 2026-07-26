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

/// 将普通标量字符串解析为 YAML 1.2 类型。
///
/// 解析优先级（从高到低）：
/// 1. Null（空字符串、`null`/`Null`/`NULL`/`~`）
/// 2. Bool（`true`/`True`/`TRUE`/`false`/`False`/`FALSE`）
/// 3. Infinity（`.inf`/`inf`/`-inf` 等，大小写不敏感）
/// 4. NaN（`.nan`/`nan`/`NaN` 等）
/// 5. 八进制整数（`0o` 前缀）
/// 6. 十六进制整数（`0x` 前缀）
/// 7. 浮点数（包含 `.` 或 `e`/`E`）
/// 8. 十进制整数
/// 9. 字符串（默认回退）
///
/// # Arguments
/// * `value` - 待解析的标量字符串。
///
/// # Returns
/// 解析后的 `YamlType` 枚举值。
///
/// # Examples
/// ```ignore
/// assert_eq!(resolve_yaml_type("null"), YamlType::Null);
/// assert_eq!(resolve_yaml_type("42"), YamlType::Int(42));
/// assert_eq!(resolve_yaml_type("hello"), YamlType::Str("hello".into()));
/// ```
pub fn resolve_yaml_type(value: &str) -> YamlType {
    let trimmed = value.trim();

    // Null values (YAML 1.2)
    if trimmed.is_empty()
        || trimmed == "null"
        || trimmed == "Null"
        || trimmed == "NULL"
        || trimmed == "~"
    {
        return YamlType::Null;
    }

    // Boolean values (YAML 1.2 - only true/false are valid)
    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return YamlType::Bool(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return YamlType::Bool(false);
    }

    // Infinity
    if trimmed == ".inf"
        || trimmed == ".Inf"
        || trimmed == ".INF"
        || trimmed == "inf"
        || trimmed == "Inf"
        || trimmed == "INF"
    {
        return YamlType::Float(f64::INFINITY);
    }
    if trimmed == "-.inf"
        || trimmed == "-.Inf"
        || trimmed == "-.INF"
        || trimmed == "-inf"
        || trimmed == "-Inf"
        || trimmed == "-INF"
    {
        return YamlType::Float(f64::NEG_INFINITY);
    }

    // NaN
    if trimmed == ".nan"
        || trimmed == ".NaN"
        || trimmed == ".NAN"
        || trimmed == "nan"
        || trimmed == "NaN"
        || trimmed == "NAN"
    {
        return YamlType::Float(f64::NAN);
    }

    // Octal integer (0o prefix)
    if trimmed.starts_with("0o") || trimmed.starts_with("0O") {
        let octal_str = &trimmed[2..];
        if let Ok(val) = i64::from_str_radix(octal_str, 8) {
            return YamlType::Int(val);
        }
    }

    // Hexadecimal integer (0x prefix)
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        let hex_str = &trimmed[2..];
        if let Ok(val) = i64::from_str_radix(hex_str, 16) {
            return YamlType::Int(val);
        }
    }

    // Float with decimal point or exponent
    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        if let Ok(val) = trimmed.parse::<f64>() {
            return YamlType::Float(val);
        }
    }

    // Decimal integer
    if let Ok(val) = trimmed.parse::<i64>() {
        return YamlType::Int(val);
    }

    // Default: string
    YamlType::Str(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(resolve_yaml_type(""), YamlType::Null);
        assert_eq!(resolve_yaml_type("null"), YamlType::Null);
        assert_eq!(resolve_yaml_type("Null"), YamlType::Null);
        assert_eq!(resolve_yaml_type("NULL"), YamlType::Null);
        assert_eq!(resolve_yaml_type("~"), YamlType::Null);
    }

    #[test]
    fn test_resolve_bool() {
        assert_eq!(resolve_yaml_type("true"), YamlType::Bool(true));
        assert_eq!(resolve_yaml_type("True"), YamlType::Bool(true));
        assert_eq!(resolve_yaml_type("TRUE"), YamlType::Bool(true));
        assert_eq!(resolve_yaml_type("false"), YamlType::Bool(false));
        assert_eq!(resolve_yaml_type("False"), YamlType::Bool(false));
        assert_eq!(resolve_yaml_type("FALSE"), YamlType::Bool(false));
    }

    #[test]
    fn test_resolve_int_decimal() {
        assert_eq!(resolve_yaml_type("42"), YamlType::Int(42));
        assert_eq!(resolve_yaml_type("-10"), YamlType::Int(-10));
        assert_eq!(resolve_yaml_type("0"), YamlType::Int(0));
    }

    #[test]
    fn test_resolve_int_octal() {
        assert_eq!(resolve_yaml_type("0o10"), YamlType::Int(8));
        assert_eq!(resolve_yaml_type("0O77"), YamlType::Int(63));
    }

    #[test]
    fn test_resolve_int_hex() {
        assert_eq!(resolve_yaml_type("0xFF"), YamlType::Int(255));
        assert_eq!(resolve_yaml_type("0X0A"), YamlType::Int(10));
    }

    #[test]
    fn test_resolve_float() {
        match resolve_yaml_type("3.14") {
            YamlType::Float(v) => assert!((v - 3.14).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
        match resolve_yaml_type("1e10") {
            YamlType::Float(v) => assert!((v - 1e10).abs() < 1.0),
            other => panic!("expected Float, got {:?}", other),
        }
        match resolve_yaml_type("-1.5E-3") {
            YamlType::Float(v) => assert!((v - (-1.5e-3)).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_infinity_nan() {
        assert_eq!(resolve_yaml_type(".inf"), YamlType::Float(f64::INFINITY));
        assert_eq!(
            resolve_yaml_type("-.inf"),
            YamlType::Float(f64::NEG_INFINITY)
        );
        assert!(resolve_yaml_type(".nan").is_nan_value());
        assert!(resolve_yaml_type("nan").is_nan_value());
    }

    #[test]
    fn test_resolve_string() {
        assert_eq!(
            resolve_yaml_type("hello"),
            YamlType::Str("hello".to_string())
        );
        assert_eq!(
            resolve_yaml_type("12abc"),
            YamlType::Str("12abc".to_string())
        );
    }

    #[test]
    fn test_format_roundtrip() {
        let cases = vec![
            "null", "true", "false", "42", "-10", "3.14", ".inf", "-.inf", ".nan",
        ];
        for input in cases {
            let resolved = resolve_yaml_type(input);
            let formatted = format_yaml_type(&resolved);
            let re_resolved = resolve_yaml_type(&formatted);
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

#[cfg(test)]
impl YamlType {
    /// Check if this is a NaN value (since f64 NaN != NaN)
    fn is_nan_value(&self) -> bool {
        match self {
            YamlType::Float(v) => v.is_nan(),
            _ => false,
        }
    }
}
