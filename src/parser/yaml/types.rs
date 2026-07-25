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

/// Resolve a plain scalar to its YAML 1.2 type
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

/// Format a YAML 1.2 type back to string for serialization
pub fn format_yaml_type(ty: &YamlType) -> String {
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
                // Format as integer if it's a whole number
                format!("{}", *val as i64)
            } else {
                val.to_string()
            }
        }
        YamlType::Str(s) => s.clone(),
    }
}
