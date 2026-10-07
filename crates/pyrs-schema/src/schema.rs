use crate::is_yaml_noncharacter;
use crate::types::{YamlSchema, YamlType};
use alloc::borrow::Cow;

// YamlSchema is defined in types.rs and re-exported via mod.rs.

/// Resolve a plain scalar with zero implicit resolution.
/// Always returns `YamlType::Str`.
pub fn resolve_failsafe(value: &str) -> YamlType<'_> {
    YamlType::Str(Cow::Borrowed(value))
}

/// Case-insensitive equality against any of the candidate strings.
fn matches_any(value: &str, candidates: &[&str]) -> bool {
    candidates.contains(&value)
}

/// The bytes a typed plain scalar can begin with, across every schema this crate
/// resolves for, plus the blanks [`crate::is_yaml_blank`] skips - a leading blank can
/// hide a starter, so those bytes have to fall through to the full chain.
const POSSIBLE_TYPED_STARTERS: &[u8] = b"~nNtTfFyYoOiI.-+0123456789 \t\r\n\0";

/// Whether the very first byte leaves a typed reading possible at all.
///
/// Safe for the YAML resolvers, whose blank class is exactly the five bytes listed
/// above. The JSON-family resolvers trim with `str::trim` (Unicode), where an NBSP can
/// hide a starter from a byte-only test, so they must not use this - and do not.
#[must_use]
pub fn might_start_typed(text: &str) -> bool {
    match text.as_bytes().first() {
        Some(byte) => POSSIBLE_TYPED_STARTERS.contains(byte),
        None => true,
    }
}

/// Resolve a plain scalar as YAML 1.2 Core.
///
/// Priority: Null → Bool → Infinity → NaN → Octal → Hex → Float → Decimal int → String.
///
/// Edge separation is stripped with [`crate::is_yaml_blank`], not `str::trim`:
/// `str::trim` is Unicode-based and also removes NBSP, U+0085 and U+2028/U+2029,
/// which YAML treats as ordinary plain-scalar content. Trimming them away made
/// `"<NBSP>42"` resolve to the integer `42` and a NBSP-only value resolve to
/// `Null` — silent type corruption (libFuzzer `yaml_roundtrip` crash-b44481b2,
/// 7 bytes: a NBSP-only *multi-line* scalar resolved to `Null`, so the writer
/// skipped quoting it and emitted raw line breaks that collapsed on re-read).
pub fn resolve_core_type(value: &str) -> YamlType<'_> {
    // Byte-only pre-check, ahead of the whole-edge scan the chain otherwise starts
    // with: a scalar whose first byte can neither start a typed lexeme nor hide one
    // behind a blank is a string whatever the trim finds. The full chain keeps its own
    // first-byte whitelist (it runs after the trim, where blanks may have been removed);
    // this only avoids doing the trim at all for the overwhelmingly common case, which
    // matters because both plain values and - since the key-resolution fix - mapping
    // keys come through here, so a document pays for roughly twice as many scalars as
    // before. `fast_bail_agrees_with_the_full_chain` pins the two paths' agreement.
    if !might_start_typed(value) {
        return YamlType::Str(Cow::Borrowed(value));
    }
    resolve_core_type_full(value)
}

/// The Core chain without the byte-only pre-check, kept whole so the whitelist above can
/// be proved against it rather than assumed.
pub(crate) fn resolve_core_type_full(value: &str) -> YamlType<'_> {
    let trimmed = value.trim_matches(crate::is_yaml_blank);

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

    if matches_any(trimmed, &["null", "Null", "NULL"]) {
        return YamlType::Null;
    }

    if let Some(b) = bool_word(trimmed) {
        return YamlType::Bool(b);
    }

    if matches_any(trimmed, &[".inf", ".Inf", ".INF", "inf", "Inf", "INF"]) {
        return YamlType::Float(f64::INFINITY);
    }
    if matches_any(
        trimmed,
        &["-.inf", "-.Inf", "-.INF", "-inf", "-Inf", "-INF"],
    ) {
        return YamlType::Float(f64::NEG_INFINITY);
    }

    if matches_any(trimmed, &[".nan", ".NaN", ".NAN", "nan", "NaN", "NAN"]) {
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

    numeric_tail(trimmed, value)
}

/// 1.2 boolean lexeme check shared by the core and JSON chains.
fn bool_word(trimmed: &str) -> Option<bool> {
    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return Some(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return Some(false);
    }
    None
}

/// Float/decimal-int/string tail shared by the core and JSON chains.
fn numeric_tail<'a>(trimmed: &str, value: &'a str) -> YamlType<'a> {
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

/// Whether the string resolves to a non-Str type under the core schema
/// (int/float/bool/null). Shared by [`needs_quotes`] and the serializer's
/// [`needs_double_quoted`] so the "is this value type-resolvable?" dimension
/// lives in one place.
pub fn core_type_is_non_string(value: &str) -> bool {
    !matches!(resolve_core_type(value), YamlType::Str(_))
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
    if core_type_is_non_string(value) {
        return true;
    }
    if value.starts_with(' ') || value.ends_with(' ') {
        return true;
    }
    if value
        .chars()
        .any(|c| c.is_control() || is_yaml_noncharacter(c))
    {
        return true;
    }
    // NBSP (non-breaking space, U+00A0) is not a control character and is
    // printable, but when emitted as a plain scalar it can cause the parser
    // to fail in layout-check paths (precompute/ensure_splice). Treat it
    // like a control character for quoting purposes.
    if value.contains('\u{00a0}') {
        return true;
    }
    // U+FEFF (BOM) is printable but granit-parser treats a plain scalar
    // starting with it as a document-start BOM and drops it. Quote it so the
    // character survives round-trip.
    if value.contains('\u{feff}') {
        return true;
    }
    false
}

/// Resolve a plain scalar as JSON-compatible YAML.
///
/// Same as Core minus: inf, nan, octal (0o), hex (0x) — those become strings.
///
/// Deliberately keeps `str::trim` (unlike the YAML resolvers): JSON-family
/// callers feed these already-stripped tokens, and JSON5's own whitespace class
/// really does include NBSP and the Zs/LS/PS separators — pinning that here
/// would break `load_json5`, which accepts them as structural whitespace.
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

    if let Some(b) = bool_word(trimmed) {
        return YamlType::Bool(b);
    }

    // inf / nan → strings (not floats)
    if is_inf_or_nan(trimmed) {
        return YamlType::Str(Cow::Borrowed(value));
    }

    // octal / hex → strings (not ints)
    if is_octal(trimmed) || is_hex(trimmed) {
        return YamlType::Str(Cow::Borrowed(value));
    }

    numeric_tail(trimmed, value)
}

/// Resolve a plain scalar produced by the JSON5 parser.
///
/// JSON5's number grammar is a superset of JSON's: hexadecimal integers
/// (`0x1F`), an explicit leading `+` (`+7`), a trailing decimal point
/// (`5.`), and the `Infinity` / `NaN` literals all denote numbers even
/// though JSON has no spelling for them. `resolve_json_type` — used for
/// strict JSON and JSONC — deliberately maps those extra forms to
/// strings; JSON5 load must turn them into real numbers. Everything the
/// two grammars share (decimal ints/floats, leading-dot `.5`, exponent,
/// `true`/`false`/`null`) is delegated so the two can never drift.
pub fn resolve_json5_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim();
    match trimmed {
        "Infinity" | "+Infinity" => return YamlType::Float(f64::INFINITY),
        "-Infinity" => return YamlType::Float(f64::NEG_INFINITY),
        "NaN" => return YamlType::Float(f64::NAN),
        _ => {}
    }
    if let Some(n) = parse_json5_hex(trimmed) {
        return YamlType::Int(n);
    }
    // Trailing-dot float: `5.` / `-5.` are floats in JSON5 (numeric_tail
    // would read the leading digits as an integer or reject the bare dot).
    if trimmed.ends_with('.') {
        let core = trimmed.trim_end_matches('.');
        let core = core.strip_prefix('+').unwrap_or(core);
        if let Ok(f) = core.parse::<f64>() {
            return YamlType::Float(f);
        }
    }
    // Leading-plus on an otherwise-JSON number.
    if let Some(rest) = trimmed.strip_prefix('+') {
        return match resolve_json_type(rest) {
            YamlType::Int(n) => YamlType::Int(n),
            YamlType::Float(f) => YamlType::Float(f),
            _ => YamlType::Str(Cow::Borrowed(value)),
        };
    }
    resolve_json_type(value)
}

/// Parse a JSON5 hexadecimal integer (`[+-]?0x[0-9a-fA-F]+`). No
/// underscore separators (JSON5 only allows them in decimal literals).
fn parse_json5_hex(trimmed: &str) -> Option<i64> {
    let (neg, unsigned) = match trimmed.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let hex = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))?;
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = i64::from_str_radix(hex, 16).ok()?;
    Some(if neg { -v } else { v })
}

/// Resolve a plain scalar as YAML 1.1.
///
/// Same as Core plus legacy boolean lexemes (yes/No/ON/off/y/N/...).
/// The legacy words are matched first; everything else (null spellings,
/// 1.2 bools, inf/nan, octal/hex, numbers) is delegated to the core
/// chain so the two can never drift.
pub fn resolve_yaml11_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim_matches(crate::is_yaml_blank);

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

/// Whether a plain scalar's text resolves to anything other than a string, under
/// *either* YAML schema the engine offers.
///
/// Cross-format bridges need this question, and they need it answered conservatively.
/// A TOML or JSON object key is a string by that format's own grammar, but the shared
/// AST has no marker for "this scalar is a string" except quoting — the same mechanism
/// TOML string *values* already use. So a bridge that writes the key `"1"` as plain
/// `1` produces a YAML document whose key is the integer 1, and `""` becomes a null
/// key: the conversion would change what the document means, which is the one thing a
/// multi-format AST must not do. Checking core *and* 1.1 keeps that answer the same
/// whichever profile the reader picks — `yes` is a string under 1.2 and a bool under
/// 1.1, so it has to be quoted too. Quoting a key that did not need it is harmless;
/// leaving one that did unquoted is a type change.
pub fn plain_text_is_typed(text: &str) -> bool {
    !matches!(resolve_core_type(text), YamlType::Str(_))
        || !matches!(resolve_yaml11_type(text), YamlType::Str(_))
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
    use alloc::format;

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

    // ---- cross-format key guard ----

    /// The byte-only pre-check must never disagree with the chain it skips.
    ///
    /// Exhaustive over the cross product of every typed spelling and near-miss this
    /// crate knows about with the separations that can hide one, rather than over a
    /// handful of examples: the failure mode of a fast path is a single missed
    /// interaction (a blank in front of `~`, an NBSP in front of digits), and both of
    /// those are cases a sample-based test happily passes.
    #[test]
    fn fast_bail_agrees_with_the_full_chain() {
        /// The domain includes NaN, and `NaN != NaN`, so a plain `==` on the resolved
        /// type fails on the very inputs it is comparing - `.nan` disagrees with itself
        /// before any fast path is involved. Same trap as the Python key-parity helper
        /// and for the same reason: the value domain has a non-reflexive member.
        fn agrees(a: &YamlType<'_>, b: &YamlType<'_>) -> bool {
            match (a, b) {
                (YamlType::Float(x), YamlType::Float(y)) => x == y || (x.is_nan() && y.is_nan()),
                _ => a == b,
            }
        }

        let stems = [
            "",
            "~",
            "null",
            "Null",
            "NULL",
            "true",
            "True",
            "TRUE",
            "false",
            "False",
            "FALSE",
            "y",
            "Y",
            "n",
            "N",
            "yes",
            "Yes",
            "YES",
            "no",
            "No",
            "NO",
            "on",
            "On",
            "ON",
            "off",
            "Off",
            "OFF",
            "inf",
            "Inf",
            "INF",
            "-.inf",
            ".nan",
            "NaN",
            "NAN",
            "0o17",
            "0O17",
            "0x1F",
            "0X1f",
            "42",
            "-7",
            "+7",
            "007",
            "3.5",
            ".5",
            "-1.5",
            "1e3",
            "1E3",
            "1.5e+3",
            "1:30",
            "a",
            "host",
            "_x",
            "x1",
            "1x",
            "3.4.5",
            "hello world",
            "\u{a0}42",
            "\u{a0}",
            "~ ",
            " #c",
            "-",
            "+",
            ".",
            "..",
            "-.-",
            "1_000",
            "0b101",
            "\u{5e7b}\u{5e7b}",
        ];
        let decorations = ["", " ", "\t", " \t ", "\r\n", "\0", " ", "  "];
        let mut checked = 0usize;
        for stem in stems {
            for lead in decorations {
                for trail in decorations {
                    let text = format!("{lead}{stem}{trail}");
                    checked += 1;
                    assert!(
                        agrees(&resolve_core_type(&text), &resolve_core_type_full(&text),),
                        "core disagrees for {text:?}: {:?} vs {:?}",
                        resolve_core_type(&text),
                        resolve_core_type_full(&text),
                    );
                    // 1.1 runs its legacy words first, then delegates; the reference
                    // here mirrors that order against the un-pre-checked chain, so the
                    // only thing under test is the bail - not the profile's semantics.
                    let legacy = match text.trim_matches(crate::is_yaml_blank) {
                        "yes" | "Yes" | "YES" | "y" | "Y" | "on" | "On" | "ON" => Some(true),
                        "no" | "No" | "NO" | "n" | "N" | "off" | "Off" | "OFF" => Some(false),
                        _ => None,
                    };
                    let expected =
                        legacy.map_or_else(|| resolve_core_type_full(&text), YamlType::Bool);
                    assert!(
                        agrees(&resolve_yaml11_type(&text), &expected),
                        "1.1 disagrees for {text:?}"
                    );
                }
            }
        }
        // The matrix has to stay big enough to be worth running: a refactor that
        // shrinks `stems` silently shrinks the proof.
        assert!(checked > 3000, "only {checked} combinations checked");
    }

    #[test]
    fn plain_text_is_typed_covers_both_schemas() {
        // Resolved under the 1.2 core profile, so a bridge must quote them as keys.
        for text in [
            "", "~", "null", "Null", "1", "-2", "3.5", "true", "0x1F", ".inf", "inf", "1.5e3",
        ] {
            assert!(plain_text_is_typed(text), "{text:?} resolves under core");
        }
        // Only YAML 1.1 types these, and the engine offers that profile, so they quote
        // too - that is the point of checking both schemas rather than the default.
        for text in ["yes", "no", "on", "off", "y", "n", "0755"] {
            assert!(
                plain_text_is_typed(text),
                "{text:?} resolves under YAML 1.1"
            );
        }
        for text in ["port", "host", "text", "a", "v1", "2024-01-01", "0.1.2"] {
            assert!(
                !plain_text_is_typed(text),
                "{text:?} is a string under both"
            );
        }
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
        match resolve_core_type("3.14") {
            // Tests parsing the string "3.14"; the expected value IS 3.14, not PI
            #[allow(clippy::approx_constant)]
            YamlType::Float(v) => assert!((v - 3.14).abs() < 1e-10),
            other => panic!("expected Float, got {:?}", other),
        }
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
    fn test_json5_numbers_are_values_not_strings() {
        // The JSON5-only forms that resolve_json_type (strict) keeps as
        // strings must resolve to real numbers here.
        assert_eq!(resolve_json5_type("0x1F"), YamlType::Int(31));
        assert_eq!(resolve_json5_type("0X1f"), YamlType::Int(31));
        assert_eq!(resolve_json5_type("-0x10"), YamlType::Int(-16));
        assert_eq!(resolve_json5_type("+7"), YamlType::Int(7));
        assert_eq!(resolve_json5_type("5."), YamlType::Float(5.0));
        assert!(matches!(
            resolve_json5_type("Infinity"),
            YamlType::Float(f) if f == f64::INFINITY
        ));
        assert!(matches!(
            resolve_json5_type("-Infinity"),
            YamlType::Float(f) if f == f64::NEG_INFINITY
        ));
        assert!(matches!(resolve_json5_type("NaN"), YamlType::Float(f) if f.is_nan()));
    }

    #[test]
    fn test_json5_delegates_shared_forms_to_json() {
        // Forms the two grammars share resolve identically to the JSON
        // schema (and NOT to weird places): decimal, leading-dot,
        // exponent, bools, null, and a real JSON string stays a string.
        assert_eq!(resolve_json5_type("42"), YamlType::Int(42));
        assert_eq!(resolve_json5_type("-1.5"), YamlType::Float(-1.5));
        assert_eq!(resolve_json5_type(".5"), YamlType::Float(0.5));
        assert_eq!(resolve_json5_type("true"), YamlType::Bool(true));
        assert_eq!(resolve_json5_type("null"), YamlType::Null);
        assert_eq!(resolve_json5_type("hello"), YamlType::Str("hello".into()));
    }

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
