use crate::parser::yaml::types::{YamlSchema, YamlType};
use std::borrow::Cow;

// YamlSchema is defined in types.rs and re-exported via mod.rs.

/// Resolve a plain scalar with zero implicit resolution.
/// Always returns `YamlType::Str`.
pub fn resolve_failsafe(value: &str) -> YamlType<'_> {
    YamlType::Str(Cow::Borrowed(value))
}

/// Resolve a plain scalar as YAML 1.2 Core.
///
/// Priority: Null → Bool → Infinity → NaN → Octal → Hex → Float → Decimal int → String.
pub fn resolve_core_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim();

    if trimmed.is_empty() || trimmed == "~" {
        return YamlType::Null;
    }

    // Fast path: most string scalars ("hello", "database", "_foo", "你好", …)
    // start with a character that can't be the first character of any resolved
    // lexeme (null/true/false/inf/nan/hex/oct/int/float).  Whitelist inversion:
    // anything not in the keep-set is guaranteed Str, so skip the full chain.
    let first = trimmed.as_bytes().first().copied().unwrap_or(0);
    if !matches!(
        first,
        b'~' | b'n' | b'N' | b't' | b'T' | b'f' | b'F' | b'i' | b'I' | b'.' | b'-' | b'+' | b'0'
            ..=b'9'
    ) {
        return YamlType::Str(Cow::Borrowed(value));
    }

    if trimmed == "null" || trimmed == "Null" || trimmed == "NULL" {
        return YamlType::Null;
    }

    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return YamlType::Bool(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return YamlType::Bool(false);
    }

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

    if trimmed == ".nan"
        || trimmed == ".NaN"
        || trimmed == ".NAN"
        || trimmed == "nan"
        || trimmed == "NaN"
        || trimmed == "NAN"
    {
        return YamlType::Float(f64::NAN);
    }

    if (trimmed.starts_with("0o") || trimmed.starts_with("0O"))
        && let Ok(val) = i64::from_str_radix(&trimmed[2..], 8)
    {
        return YamlType::Int(val);
    }

    if (trimmed.starts_with("0x") || trimmed.starts_with("0X"))
        && let Ok(val) = i64::from_str_radix(&trimmed[2..], 16)
    {
        return YamlType::Int(val);
    }

    if (trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E'))
        && let Ok(val) = trimmed.parse::<f64>()
    {
        return YamlType::Float(val);
    }

    if let Ok(val) = trimmed.parse::<i64>() {
        return YamlType::Int(val);
    }

    YamlType::Str(Cow::Borrowed(value))
}

/// Whether a string would be misread if emitted as an unquoted plain scalar.
///
/// Quoting is required when the value resolves to a non-string type under the
/// core schema (bool/int/float/null — e.g. `"true"`, `"42"`, `"null"`, `"~"`,
/// `""`, `"0x1F"`, `".inf"`), or when raw emission would be ambiguous or
/// invalid YAML (leading/trailing whitespace, control characters).
///
/// This is the conversion-layer guard used by `pyobject_to_node` and the
/// direct writer so that Python strings round-trip losslessly. It intentionally
/// stays bound to [`resolve_core_type`] so the two can never drift.
pub fn needs_quotes(value: &str) -> bool {
    if !matches!(resolve_core_type(value), YamlType::Str(_)) {
        return true;
    }
    if value.starts_with(' ') || value.ends_with(' ') {
        return true;
    }
    if value.chars().any(|c| c.is_control()) {
        return true;
    }
    false
}

/// Resolve a plain scalar as JSON-compatible YAML.
///
/// Same as Core minus: inf, nan, octal (0o), hex (0x) — those become strings.
pub fn resolve_json_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim();

    if trimmed.is_empty()
        || trimmed == "null"
        || trimmed == "Null"
        || trimmed == "NULL"
        || trimmed == "~"
    {
        return YamlType::Null;
    }

    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return YamlType::Bool(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return YamlType::Bool(false);
    }

    // inf / nan → strings (not floats)
    if is_inf_or_nan(trimmed) {
        return YamlType::Str(Cow::Borrowed(value));
    }

    // octal / hex → strings (not ints)
    if is_octal(trimmed) || is_hex(trimmed) {
        return YamlType::Str(Cow::Borrowed(value));
    }

    if (trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E'))
        && let Ok(val) = trimmed.parse::<f64>()
    {
        return YamlType::Float(val);
    }

    if let Ok(val) = trimmed.parse::<i64>() {
        return YamlType::Int(val);
    }

    YamlType::Str(Cow::Borrowed(value))
}

/// Resolve a plain scalar as YAML 1.1.
///
/// Same as Core plus legacy boolean lexemes (yes/No/ON/off/y/N/...).
pub fn resolve_yaml11_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim();

    if trimmed.is_empty()
        || trimmed == "null"
        || trimmed == "Null"
        || trimmed == "NULL"
        || trimmed == "~"
    {
        return YamlType::Null;
    }

    // YAML 1.2 booleans
    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return YamlType::Bool(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return YamlType::Bool(false);
    }

    // YAML 1.1 legacy booleans
    let legacy_bool = match trimmed {
        "yes" | "Yes" | "YES" | "y" | "Y" | "on" | "On" | "ON" => Some(true),
        "no" | "No" | "NO" | "n" | "N" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    };
    if let Some(b) = legacy_bool {
        return YamlType::Bool(b);
    }

    resolve_core_type(value)
}

/// Public dispatcher: resolve a plain scalar according to the given schema.
pub fn resolve_yaml_type(value: &str, schema: YamlSchema) -> YamlType<'_> {
    match schema {
        YamlSchema::Failsafe => resolve_failsafe(value),
        YamlSchema::Json => resolve_json_type(value),
        YamlSchema::Core => resolve_core_type(value),
        YamlSchema::Yaml1_1 => resolve_yaml11_type(value),
    }
}

// ---------------------------------------------------------------------------
// Private helpers (shared between core and json)
// ---------------------------------------------------------------------------

fn is_inf_or_nan(s: &str) -> bool {
    matches!(
        s,
        ".inf"
            | ".Inf"
            | ".INF"
            | "inf"
            | "Inf"
            | "INF"
            | "-.inf"
            | "-.Inf"
            | "-.INF"
            | "-inf"
            | "-Inf"
            | "-INF"
            | ".nan"
            | ".NaN"
            | ".NAN"
            | "nan"
            | "NaN"
            | "NAN"
    )
}

fn is_octal(s: &str) -> bool {
    s.starts_with("0o") || s.starts_with("0O")
}

fn is_hex(s: &str) -> bool {
    s.starts_with("0x") || s.starts_with("0X")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- failsafe ----

    #[test]
    fn test_failsafe_returns_str() {
        assert_eq!(resolve_failsafe("42"), YamlType::Str(Cow::from("42")));
        assert_eq!(resolve_failsafe("null"), YamlType::Str("null".into()));
        assert_eq!(resolve_failsafe("true"), YamlType::Str("true".into()));
        assert_eq!(resolve_failsafe("3.14"), YamlType::Str("3.14".into()));
        assert_eq!(resolve_failsafe(".inf"), YamlType::Str(".inf".into()));
        assert_eq!(resolve_failsafe("0x1F"), YamlType::Str("0x1F".into()));
    }

    // ---- core ----

    #[test]
    fn test_core_null() {
        assert_eq!(resolve_core_type(""), YamlType::Null);
        assert_eq!(resolve_core_type("null"), YamlType::Null);
        assert_eq!(resolve_core_type("~"), YamlType::Null);
    }

    #[test]
    fn test_core_bool() {
        assert_eq!(resolve_core_type("true"), YamlType::Bool(true));
        assert_eq!(resolve_core_type("TRUE"), YamlType::Bool(true));
        assert_eq!(resolve_core_type("false"), YamlType::Bool(false));
    }

    #[test]
    fn test_core_numbers() {
        assert_eq!(resolve_core_type("42"), YamlType::Int(42));
        assert_eq!(resolve_core_type("-10"), YamlType::Int(-10));
        assert_eq!(resolve_core_type("0x1F"), YamlType::Int(31));
        assert_eq!(resolve_core_type("0o17"), YamlType::Int(15));
        match resolve_core_type("3.14") {
            // Tests parsing the string "3.14"; the expected value IS 3.14, not PI
            #[allow(clippy::approx_constant)]
            YamlType::Float(v) => assert!((v - 3.14).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
        assert_eq!(resolve_core_type(".inf"), YamlType::Float(f64::INFINITY));
        assert_eq!(
            resolve_core_type("-.inf"),
            YamlType::Float(f64::NEG_INFINITY)
        );
        assert!(resolve_core_type(".nan").is_nan_value());
    }

    #[test]
    fn test_core_string() {
        assert_eq!(resolve_core_type("hello"), YamlType::Str("hello".into()));
    }

    #[test]
    fn test_core_first_char_fast_path() {
        // Whitelist-inversion fast path: any first char outside the keep-set is
        // guaranteed Str and must NOT fall through the numeric/bool resolution.
        assert_eq!(resolve_core_type("_foo"), YamlType::Str("_foo".into()));
        assert_eq!(resolve_core_type("-foo"), YamlType::Str("-foo".into()));
        assert_eq!(resolve_core_type("123abc"), YamlType::Str("123abc".into()));
        assert_eq!(resolve_core_type("0b101"), YamlType::Str("0b101".into()));
        assert_eq!(resolve_core_type("~x"), YamlType::Str("~x".into()));
        assert_eq!(resolve_core_type("."), YamlType::Str(".".into()));
        assert_eq!(resolve_core_type("-"), YamlType::Str("-".into()));
        assert_eq!(resolve_core_type("你好"), YamlType::Str("你好".into()));
        assert_eq!(
            resolve_core_type("quote\"d"),
            YamlType::Str("quote\"d".into())
        );
        assert_eq!(resolve_core_type("@tag"), YamlType::Str("@tag".into()));
        assert_eq!(resolve_core_type("a1"), YamlType::Str("a1".into()));
        // Leading whitespace trims first, then the fast path applies.
        assert_eq!(resolve_core_type(" hello"), YamlType::Str(" hello".into()));
        assert_eq!(resolve_core_type(" null"), YamlType::Null);
        assert_eq!(resolve_core_type(" 42"), YamlType::Int(42));
    }

    #[test]
    fn test_core_first_char_keep_set() {
        // Every keep-set first char must still resolve through the full chain.
        assert_eq!(resolve_core_type("+5"), YamlType::Int(5));
        assert_eq!(resolve_core_type("05"), YamlType::Int(5));
        assert_eq!(resolve_core_type("~"), YamlType::Null);
        assert_eq!(resolve_core_type("true"), YamlType::Bool(true));
        assert_eq!(resolve_core_type("false"), YamlType::Bool(false));
        assert_eq!(resolve_core_type("null"), YamlType::Null);
        assert_eq!(resolve_core_type("inf"), YamlType::Float(f64::INFINITY));
        assert!(resolve_core_type(".nan").is_nan_value());
        assert_eq!(resolve_core_type("0x1F"), YamlType::Int(31));
        assert_eq!(resolve_core_type("3.14"), YamlType::Float(3.14));
        assert_eq!(resolve_core_type("n"), YamlType::Str("n".into()));
        assert_eq!(resolve_core_type("t"), YamlType::Str("t".into()));
    }

    // ---- json ----

    #[test]
    fn test_json_inf_nan_are_strings() {
        assert_eq!(resolve_json_type(".inf"), YamlType::Str(".inf".into()));
        assert_eq!(resolve_json_type(".INF"), YamlType::Str(".INF".into()));
        assert_eq!(resolve_json_type("-inf"), YamlType::Str("-inf".into()));
        assert_eq!(resolve_json_type(".nan"), YamlType::Str(".nan".into()));
        assert_eq!(resolve_json_type(".NaN"), YamlType::Str(".NaN".into()));
    }

    #[test]
    fn test_json_octal_hex_are_strings() {
        assert_eq!(resolve_json_type("0x1F"), YamlType::Str("0x1F".into()));
        assert_eq!(resolve_json_type("0o17"), YamlType::Str("0o17".into()));
    }

    #[test]
    fn test_json_resolves_normal_types() {
        assert_eq!(resolve_json_type("null"), YamlType::Null);
        assert_eq!(resolve_json_type("true"), YamlType::Bool(true));
        assert_eq!(resolve_json_type("false"), YamlType::Bool(false));
        assert_eq!(resolve_json_type("42"), YamlType::Int(42));
        match resolve_json_type("3.14") {
            // Tests parsing the string "3.14"; the expected value IS 3.14, not PI
            #[allow(clippy::approx_constant)]
            YamlType::Float(v) => assert!((v - 3.14).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
        assert_eq!(resolve_json_type("hello"), YamlType::Str("hello".into()));
    }

    // ---- yaml1.1 ----

    #[test]
    fn test_yaml11_legacy_bool() {
        assert_eq!(resolve_yaml11_type("yes"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("Yes"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("YES"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("y"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("Y"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("on"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("On"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("ON"), YamlType::Bool(true));
        assert_eq!(resolve_yaml11_type("no"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("No"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("NO"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("n"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("N"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("off"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("Off"), YamlType::Bool(false));
        assert_eq!(resolve_yaml11_type("OFF"), YamlType::Bool(false));
    }

    #[test]
    fn test_yaml11_keeps_tilde_as_null() {
        assert_eq!(resolve_yaml11_type("~"), YamlType::Null);
    }

    #[test]
    fn test_yaml11_inherits_core_for_numbers() {
        assert_eq!(resolve_yaml11_type("42"), YamlType::Int(42));
        assert_eq!(resolve_yaml11_type("0x1F"), YamlType::Int(31));
        assert_eq!(resolve_yaml11_type(".inf"), YamlType::Float(f64::INFINITY));
    }

    // ---- dispatcher ----

    #[test]
    fn test_resolve_yaml_type_dispatcher() {
        assert_eq!(
            resolve_yaml_type("42", YamlSchema::Failsafe),
            YamlType::Str("42".into())
        );
        assert_eq!(
            resolve_yaml_type("0x1F", YamlSchema::Json),
            YamlType::Str("0x1F".into())
        );
        assert_eq!(
            resolve_yaml_type("0x1F", YamlSchema::Core),
            YamlType::Int(31)
        );
        assert_eq!(
            resolve_yaml_type("yes", YamlSchema::Yaml1_1),
            YamlType::Bool(true)
        );
        assert_eq!(
            resolve_yaml_type("yes", YamlSchema::Core),
            YamlType::Str("yes".into())
        );
    }

    // is_nan_value() provided by #[cfg(test)] impl YamlType in types.rs
}
