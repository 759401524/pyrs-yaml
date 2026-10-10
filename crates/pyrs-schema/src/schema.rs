use crate::is_yaml_noncharacter;
use crate::types::{YamlSchema, YamlType};
use alloc::borrow::Cow;
use alloc::string::String;

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
///
/// The hub's own spellings are accepted too. `pyrs-json` stores a JSON5 infinity as `.inf` /
/// `-.inf` / `.nan` because the projection it emits is YAML text and `Infinity` re-reads there as a
/// string (#312); the resolver that consumes those ASTs has to agree with the parser that produced
/// them, or the in-memory loader and the round-tripped document disagree about the type.
pub fn resolve_json5_type(value: &str) -> YamlType<'_> {
    let trimmed = value.trim();
    match trimmed {
        "Infinity" | "+Infinity" | ".inf" | ".Inf" | ".INF" => {
            return YamlType::Float(f64::INFINITY);
        }
        "-Infinity" | "-.inf" | "-.Inf" | "-.INF" => return YamlType::Float(f64::NEG_INFINITY),
        "NaN" | ".nan" | ".NaN" | ".NAN" => return YamlType::Float(f64::NAN),
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
// Standard tags: the type a document states outright
// ---------------------------------------------------------------------------

/// The scalar types YAML's standard tag vocabulary names, restricted to the ones a
/// cross-format bridge can project without changing their meaning.
///
/// `!!binary` is deliberately absent: JSON and TOML have no byte-string type, so what its
/// base64 text should become is a mapping decision, not a type-resolution one. It stays with
/// the caller that has to make it (issue #340 records that this is still open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// `!!null` - the text must spell a null.
    Null,
    /// `!!bool` - the YAML 1.1 lexeme set, which is what the tag itself defines.
    Bool,
    /// `!!int` - the YAML 1.1 resolvable integer, `0b`/`0x`/`0o`/legacy octal/underscores.
    Int,
    /// `!!float` - including the `.inf`/`.nan` spellings.
    Float,
    /// `!!str` - whatever the text says, read as text.
    Str,
}

/// The [`TagKind`] a *standard* tag states, or `None` when the tag names nothing this crate
/// can act on: a local or application tag (`!myclass`, `!int`), an unrecognised `!!` suffix
/// (`!!weird`), or `!!binary`.
///
/// A tag *replaces* the implicit resolution. YAML 1.2 §6.1 says so, and since #335 the loader
/// obeys it - `!!str 1.20` is the string `"1.20"`, `!!bool yes` is `true`. The bridges kept
/// reading the text alone, so the same AST answered two different questions: `to_json` turned
/// `!!str 1.20` into the number `1.2` (changing its type *and* dropping the trailing zero) and
/// `!!bool yes` into the string `"yes"`. This is the one place that grammar now lives, so a
/// bridge cannot drift from the loader again.
///
/// Both spellings a standard tag reaches the AST in are recognised: the shorthand handle
/// (`!!int`) and the verbatim URI (`!<tag:yaml.org,2002:int>`, stored with an empty handle),
/// which is what `Node.set_tag` writes.
#[must_use]
pub fn standard_tag_kind(handle: &str, suffix: &str) -> Option<TagKind> {
    let name: &str = if handle == "!!" {
        suffix
    } else if handle.is_empty() {
        suffix.strip_prefix("tag:yaml.org,2002:")?
    } else {
        return None;
    };
    match name {
        "null" => Some(TagKind::Null),
        "bool" => Some(TagKind::Bool),
        "int" => Some(TagKind::Int),
        "float" => Some(TagKind::Float),
        "str" => Some(TagKind::Str),
        _ => None,
    }
}

impl TagKind {
    /// Read `text` as the type this tag states, or `None` when the text is not that type.
    ///
    /// `None` is a refusal, not a fall-through. Resolving `!!int hello` back to a string is
    /// how a bridge ends up agreeing with no reference implementation: the loader raises
    /// `YamlTypeError` for the same input (#335), and a writer has
    /// `SerializeError::UnsupportedValue` for the same job (#328 set that precedent for
    /// values a strict format cannot spell).
    #[must_use]
    pub fn resolve<'a>(self, text: &'a str) -> Option<YamlType<'a>> {
        match self {
            // A `!!str` tag means the text is the value, exactly as written - that is the
            // whole content of the tag, so nothing here can fail.
            TagKind::Str => Some(YamlType::Str(Cow::Borrowed(text))),
            // The null spellings the engine already recognises, both schemas: `~`, `null`
            // in any case, and empty. 1.1 adds nothing to that list.
            TagKind::Null => match resolve_core_type(text) {
                YamlType::Null => Some(YamlType::Null),
                _ => None,
            },
            // `!!bool` carries its own lexeme set - YAML 1.1's, which is why `yes` is a
            // boolean under a Core-schema document too: it is the tag making it one.
            TagKind::Bool => match resolve_yaml11_type(text) {
                YamlType::Bool(b) => Some(YamlType::Bool(b)),
                _ => None,
            },
            TagKind::Int => resolvable_int11(text).map(YamlType::Int),
            TagKind::Float => resolvable_float11(text).map(YamlType::Float),
        }
    }
}

/// The YAML 1.1 `resolvable int`: optional sign, then decimal (with `_` separators),
/// `0x`/`0X` hex, `0o`/`0O` octal, `0b`/`0B` binary, or the legacy form - a leading `0`
/// followed by octal digits, so `010` is 8. Underscores are stripped before parsing, and a
/// trailing or doubled underscore fails, as 1.1 requires.
///
/// This is the rule #335 put in the loader's tag reader. It lives here now because the
/// bridges need the same answer, and two copies of a grammar is how the loader and the
/// writers came to disagree about one document.
fn resolvable_int11(text: &str) -> Option<i64> {
    let (body, sign) = match text.strip_prefix('-') {
        Some(rest) => (rest, -1i64),
        None => (text.strip_prefix('+').unwrap_or(text), 1),
    };
    if body.is_empty() || body.starts_with('_') || body.ends_with('_') || body.contains("__") {
        return None;
    }
    let digits: String = body.chars().filter(|c| *c != '_').collect();
    let (radix, rest) = if digits.len() > 2 {
        match &digits[..2] {
            "0x" | "0X" => (16, &digits[2..]),
            "0o" | "0O" => (8, &digits[2..]),
            "0b" | "0B" => (2, &digits[2..]),
            _ => (10, digits.as_str()),
        }
    } else {
        (10, digits.as_str())
    };
    if rest.is_empty() {
        return None;
    }
    // Legacy octal: a leading `0` before other decimal digits (`0o7` took the radix path
    // above, so anything still starting with `0` here is either `"0"` itself or 1.1 octal).
    let value = if radix == 10 && rest.len() > 1 && rest.starts_with('0') {
        i64::from_str_radix(trim_zeros(rest), 8).ok()?
    } else {
        i64::from_str_radix(rest, radix).ok()?
    };
    value.checked_mul(sign)
}

/// The digits of a legacy 1.1 octal, with every leading zero the source stacked (`007` is
/// still 7) - `from_str_radix` accepts them, but only after the first digit was claimed as
/// the octal marker.
fn trim_zeros(rest: &str) -> &str {
    let kept = rest.trim_start_matches('0');
    if kept.is_empty() { "0" } else { kept }
}

#[cfg(test)]
mod standard_tag_tests {
    use super::TagKind::*;
    use super::{YamlType, standard_tag_kind};

    #[test]
    fn a_standard_tag_names_a_kind_in_both_spellings() {
        for (suffix, kind) in [
            ("null", Null),
            ("bool", Bool),
            ("int", Int),
            ("float", Float),
            ("str", Str),
        ] {
            assert_eq!(
                standard_tag_kind("!!", suffix),
                Some(kind),
                "{suffix} as a shorthand tag"
            );
            assert_eq!(
                standard_tag_kind("", &alloc::format!("tag:yaml.org,2002:{suffix}")),
                Some(kind),
                "{suffix} as a verbatim URI"
            );
        }
    }

    /// A local tag is how the plugin system is addressed, so naming a standard type with it
    /// must not be read as a standard type: `!int` is the author's own tag, `!!int` is YAML's.
    #[test]
    fn a_local_or_unknown_tag_names_nothing() {
        for (handle, suffix) in [
            ("!", "int"),
            ("!", "myclass"),
            ("!!", "weird"),
            ("", "tag:example.com,2020:int"),
            // `!!binary` has no JSON or TOML type to project into; deciding that mapping is a
            // separate question (issue #340), so the bridges must not guess at it here.
            ("!!", "binary"),
        ] {
            assert_eq!(
                standard_tag_kind(handle, suffix),
                None,
                "{handle}{suffix} names no standard type"
            );
        }
    }

    #[test]
    fn int_follows_the_yaml_11_lexeme_set() {
        for (text, want) in [
            ("10", 10),
            ("0b101", 5),
            ("0x1F", 31),
            ("0o17", 15),
            ("010", 8),
            ("1_000", 1000),
            ("-7", -7),
            ("+7", 7),
        ] {
            assert_eq!(
                Int.resolve(text),
                Some(YamlType::Int(want)),
                "{text} is not the integer the tag states"
            );
        }
        for text in ["hello", "1_", "__1", "", "0b2", "1.5", "y"] {
            assert_eq!(Int.resolve(text), None, "{text} is not an !!int");
        }
    }

    /// `.inf`/`.nan` are YAML's spellings; `inf`/`NaN` are Python's, and 1.1 never defines
    /// them - accepting both would let one document mean two things again.
    #[test]
    fn float_follows_the_yaml_11_lexeme_set() {
        assert_eq!(Float.resolve("1.20"), Some(YamlType::Float(1.2)));
        assert_eq!(Float.resolve("1e3"), Some(YamlType::Float(1000.0)));
        assert_eq!(Float.resolve("-1E+3"), Some(YamlType::Float(-1000.0)));
        assert_eq!(Float.resolve(".inf"), Some(YamlType::Float(f64::INFINITY)));
        assert_eq!(
            Float.resolve("-.INF"),
            Some(YamlType::Float(f64::NEG_INFINITY))
        );
        assert!(matches!(
            Float.resolve(".nan"),
            Some(YamlType::Float(f)) if f.is_nan()
        ));
        // `1.` is accepted on purpose: YAML 1.2's Core float resolver allows digits with an
        // empty fraction (`[0-9]+ (\. [0-9]* )?`), and both reference libraries read it as
        // 1.0 - measured while #335 was written, where the same question came up.
        for text in ["inf", "NaN", ".", "1e", "hello", "", "~"] {
            assert_eq!(Float.resolve(text), None, "{text} is not a !!float");
        }
    }

    /// `!!bool` carries its own lexeme set - which is why `yes` is a boolean in a
    /// Core-schema document too - and `!!str` takes whatever the text says.
    #[test]
    fn bool_and_str_read_the_way_the_tag_states() {
        for text in ["yes", "Yes", "on", "y", "true", "TRUE"] {
            assert_eq!(Bool.resolve(text), Some(YamlType::Bool(true)), "{text}");
        }
        for text in ["no", "OFF", "n", "false"] {
            assert_eq!(Bool.resolve(text), Some(YamlType::Bool(false)), "{text}");
        }
        assert_eq!(Bool.resolve("1"), None);
        for text in ["1.20", "yes", "", "0x1F"] {
            assert_eq!(
                Str.resolve(text),
                Some(YamlType::Str(text.into())),
                "{text} as !!str"
            );
        }
        for text in ["~", "null", "NULL", ""] {
            assert_eq!(Null.resolve(text), Some(YamlType::Null), "{text}");
        }
        assert_eq!(Null.resolve("0"), None);
    }
}

/// The YAML 1.1 `resolvable float`: the `.inf`/`.nan` spellings, or a decimal number with a
/// mandatory digit on at least one side of the point and an optional exponent. `1e3` and
/// `1E+3` qualify; a bare sign, `"1."` with no fraction, or a word like `inf` does not - the
/// last is Python's spelling, not YAML's, and 1.1 never defines it.
fn resolvable_float11(text: &str) -> Option<f64> {
    let (body, negate) = match text.strip_prefix('-') {
        Some(rest) => (rest, true),
        None => (text.strip_prefix('+').unwrap_or(text), false),
    };
    let value = match body {
        ".inf" | ".Inf" | ".INF" => f64::INFINITY,
        ".nan" | ".NaN" | ".NAN" => f64::NAN,
        other => {
            let digits: String = other.chars().filter(|c| *c != '_').collect();
            if digits.is_empty()
                || !digits
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
                || !digits.chars().any(|c| c.is_ascii_digit())
            {
                return None;
            }
            // Rust accepts `inf`, `NaN` and a leading `+`; YAML 1.1 does not, and a value
            // that only one of the two readers can spell is exactly the drift this function
            // exists to stop.
            let parsed = digits.parse::<f64>().ok()?;
            if !parsed.is_finite() {
                return None;
            }
            parsed
        }
    };
    Some(if negate { -value } else { value })
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

    // ---- cross-format key guard ----

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
