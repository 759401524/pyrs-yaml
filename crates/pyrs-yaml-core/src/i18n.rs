//! 国际化模块：使用 rust-i18n 提供错误消息本地化支持。
//!
//! # 架构
//!
//! 使用 rust-i18n 作为 i18n 后端，翻译文件存储在 `src/i18n/locales/` 目录下（YAML 格式）。
//! 通过 `t!()` 宏实现编译时 key 检查，`format_message()` 提供动态 key 查找接口。
//! 占位符格式为 `%{key}`，例如 `yaml-parse-error: "错误: %{detail}"`。
//!
//! # 自动语言检测
//!
//! 支持从环境变量和 Python locale 自动检测用户偏好语言：
//! - `PYI18N` 环境变量
//! - `LANGUAGE` 环境变量（GNU gettext 风格）
//! - `LC_ALL` / `LC_MESSAGES` 环境变量
//! - `LANG` 环境变量（默认）
//!
//! # 使用方法
//!
//! ```rust
//! use pyrs_yaml_core::i18n;
//!
//! // 设置语言
//! i18n::set_language("zh-CN").unwrap();
//!
//! // 获取当前语言
//! assert_eq!(i18n::get_language(), "zh-CN");
//!
//! // 格式化错误消息
//! let msg = i18n::format_message("yaml-parse-error", &[("detail", "test")]);
//! assert_eq!(msg, "test"); // 占位符 %{detail} 被替换为 "test"
//! ```

use std::cell::RefCell;

/// 支持的语言列表
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh-CN", "ja-JP", "ko-KR"];

// 当前语言状态
thread_local! {
    static CURRENT_LANG: RefCell<String> = RefCell::new("en".to_string());
}

/// 设置当前语言
pub fn set_language(lang: &str) -> Result<(), &'static str> {
    if !SUPPORTED_LANGUAGES.contains(&lang) {
        return Err("Unsupported language");
    }
    CURRENT_LANG.with(|c| {
        *c.borrow_mut() = lang.to_string();
    });
    rust_i18n::set_locale(lang);
    Ok(())
}

/// 获取当前语言
pub fn get_language() -> String {
    CURRENT_LANG.with(|c| c.borrow().clone())
}

/// 获取当前语言的静态引用（用于 Python 绑定）
pub fn get_language_static() -> &'static str {
    let lang = get_language();
    match lang.as_str() {
        "en" => "en",
        "zh-CN" => "zh-CN",
        "ja-JP" => "ja-JP",
        "ko-KR" => "ko-KR",
        _ => "en",
    }
}

/// 列出所有支持的语言
pub fn list_languages() -> Vec<&'static str> {
    SUPPORTED_LANGUAGES.to_vec()
}

/// 格式化错误消息（动态 key 查找 + 运行时参数替换）
///
/// 翻译模板中的占位符格式为 `%{key}`（百分比花括号格式），
/// 例如 YAML 文件中的 `yaml-parse-error: "解析错误: %{detail}"`。
/// 调用时会将 `%{detail}` 替换为实际的参数值。
///
/// # Arguments
/// * `key` - 消息键，对应 `src/i18n/locales/` 目录下 YAML 文件中的条目名称。
/// * `args` - 格式化参数 (键, 值) 对，格式为 `("key", "value")`，
///   其中 `key` 对应模板中的 `%{key}` 占位符名称，`value` 为替换值。
///
/// # Returns
/// 替换占位符后的完整翻译字符串。若模板中无占位符则原样返回。
///
/// # Examples
///
/// ```rust
/// use pyrs_yaml_core::i18n;
///
/// i18n::set_language("en").unwrap();
/// let msg = i18n::format_message("yaml-parse-error", &[("detail", "unexpected token")]);
/// assert_eq!(msg, "unexpected token");
/// ```
pub fn format_message(key: &str, args: &[(&str, &str)]) -> String {
    // 使用 rust_i18n 的 replace_patterns 进行运行时替换
    // 先获取翻译模板（使用 t! 宏）
    let template = rust_i18n::t!(key);

    // 运行时参数替换
    if args.is_empty() {
        template.to_string()
    } else {
        let patterns: Vec<&str> = args.iter().map(|(k, _)| *k).collect();
        let values: Vec<String> = args.iter().map(|(_, v)| v.to_string()).collect();
        rust_i18n::replace_patterns(&template, &patterns, &values)
    }
}

/// BCP 47 语言协商：从用户提供的语言列表中匹配最合适的支持语言
///
/// 实现 RFC 4647 Basic Filtering 算法：
/// 1. 精确匹配
/// 2. 语言前缀匹配（如 "zh" 匹配 "zh-CN"）
/// 3. 回退到默认语言
///
/// # Arguments
/// * `user_locales` - 用户偏好的语言列表（按优先级排序）
/// * `default` - 默认回退语言
///
/// # Examples
///
/// ```
/// use pyrs_yaml_core::i18n;
///
/// // 精确匹配
/// assert_eq!(i18n::negotiate_language(&["zh-CN"], "en"), "zh-CN");
///
/// // 前缀匹配
/// assert_eq!(i18n::negotiate_language(&["zh"], "en"), "zh-CN");
///
/// // 回退到默认
/// assert_eq!(i18n::negotiate_language(&["fr"], "en"), "en");
/// ```
pub fn negotiate_language<'a>(user_locales: &[&'a str], default: &'a str) -> &'a str {
    for user_locale in user_locales {
        // 精确匹配
        if SUPPORTED_LANGUAGES.contains(user_locale) {
            return user_locale;
        }

        // 语言前缀匹配（如 "zh" -> "zh-CN"）
        if let Some(prefix) = user_locale.split('-').next() {
            for supported in SUPPORTED_LANGUAGES {
                if supported.starts_with(prefix) {
                    return supported;
                }
            }
        }
    }

    // 回退到默认语言
    if SUPPORTED_LANGUAGES.contains(&default) {
        default
    } else {
        "en"
    }
}

/// 自动检测用户偏好语言
///
/// 按优先级检查以下环境变量：
/// 1. `PYI18N` — 库专用环境变量（最高优先级）
/// 2. `LANGUAGE` — GNU gettext 风格（支持多个语言，用 `:` 分隔）
/// 3. `LC_ALL` — POSIX locale 覆盖
/// 4. `LC_MESSAGES` — 消息 locale
/// 5. `LANG` — 默认 POSIX locale
///
/// 如果所有环境变量都未设置，返回 `"en"` 作为默认值。
///
/// # Examples
///
/// ```
/// use pyrs_yaml_core::i18n;
///
/// // 注意：实际检测结果取决于环境变量
/// let detected = i18n::detect_language();
/// assert!(detected == "en" || detected == "zh-CN");
/// ```
pub fn detect_language() -> String {
    // 1. 检查 PYI18N（最高优先级）
    if let Ok(val) = std::env::var("PYI18N")
        && !val.is_empty()
    {
        let locales = parse_language_list(&val);
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        return negotiate_language(&refs, "en").to_string();
    }

    // 2. 检查 LANGUAGE（GNU gettext 风格）
    if let Ok(val) = std::env::var("LANGUAGE")
        && !val.is_empty()
    {
        let locales = parse_language_list(&val);
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        let result = negotiate_language(&refs, "en");
        if !result.is_empty() {
            return result.to_string();
        }
    }

    // 3. 检查 LC_ALL
    if let Ok(val) = std::env::var("LC_ALL")
        && !val.is_empty()
        && val != "C"
        && val != "POSIX"
    {
        let locales = parse_language_list(&val);
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        let result = negotiate_language(&refs, "en");
        if !result.is_empty() {
            return result.to_string();
        }
    }

    // 4. 检查 LC_MESSAGES
    if let Ok(val) = std::env::var("LC_MESSAGES")
        && !val.is_empty()
    {
        let locales = parse_language_list(&val);
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        let result = negotiate_language(&refs, "en");
        if !result.is_empty() {
            return result.to_string();
        }
    }

    // 5. 检查 LANG
    if let Ok(val) = std::env::var("LANG")
        && !val.is_empty()
    {
        let locales = parse_language_list(&val);
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        let result = negotiate_language(&refs, "en");
        if !result.is_empty() {
            return result.to_string();
        }
    }

    // 默认英语
    "en".to_string()
}

/// 解析语言列表字符串（支持 `:` 分隔的多语言）
///
/// 例如："zh-CN:zh,en" -> ["zh-cn", "zh", "en"]
fn parse_language_list(input: &str) -> Vec<String> {
    input
        .split([':', ','])
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 验证翻译完整性：确保所有 key 在所有语言文件中都存在
///
/// # Returns
/// * `Ok(())` — 翻译完整
/// * `Err(String)` — 缺失翻译的详细信息
#[cfg(test)]
pub fn validate_translations() -> Result<(), String> {
    // 读取 en.yml 获取所有 key
    let en_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/i18n/locales/en.yml");
    let en_content =
        std::fs::read_to_string(en_path).map_err(|e| format!("Failed to read en.yml: {}", e))?;

    let en_keys: Vec<&str> = en_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| line.split(':').next().unwrap_or("").trim())
        .filter(|key| !key.is_empty())
        .collect();

    // 检查所有语言文件
    let other_files = [
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/i18n/locales/zh-CN.yml"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/i18n/locales/ja-JP.yml"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/i18n/locales/ko-KR.yml"),
    ];

    let mut errors = Vec::new();

    for file_path in &other_files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let file_keys: Vec<&str> = content
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                .map(|line| line.split(':').next().unwrap_or("").trim())
                .filter(|key| !key.is_empty())
                .collect();

            let missing: Vec<&str> = en_keys
                .iter()
                .filter(|key| !file_keys.contains(key))
                .copied()
                .collect();

            if !missing.is_empty() {
                errors.push(format!("Missing keys in {:?}: {:?}", file_path, missing));
            }
        } else {
            errors.push(format!("File not found: {:?}", file_path));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_language_is_english() {
        let _ = set_language("en");
        assert_eq!(get_language(), "en");
    }

    #[test]
    fn test_set_language_zh() {
        let _ = set_language("zh-CN");
        assert_eq!(get_language(), "zh-CN");
        let _ = set_language("en");
    }

    #[test]
    fn test_set_language_unsupported() {
        let result = set_language("fr");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unsupported language");
    }

    #[test]
    fn test_list_languages() {
        let langs = list_languages();
        assert!(langs.contains(&"en"));
        assert!(langs.contains(&"zh-CN"));
        assert!(langs.contains(&"ja-JP"));
        assert!(langs.contains(&"ko-KR"));
        assert_eq!(langs.len(), 4);
    }

    #[test]
    fn test_format_message_english() {
        let _ = set_language("en");
        let msg = format_message("yaml-parse-error", &[("detail", "test error")]);
        assert_eq!(msg, "test error");
    }

    #[test]
    fn test_format_message_chinese() {
        let _ = set_language("zh-CN");
        let msg = format_message("yaml-parse-error", &[("detail", "test error")]);
        assert_eq!(msg, "test error");
        let _ = set_language("en");
    }

    #[test]
    fn test_format_message_missing_key() {
        let msg = format_message("nonexistent-key", &[]);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_format_message_no_args() {
        let msg = format_message("key-not-string", &[]);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_negotiate_language_exact_match() {
        assert_eq!(negotiate_language(&["zh-CN"], "en"), "zh-CN");
        assert_eq!(negotiate_language(&["en"], "zh-CN"), "en");
    }

    #[test]
    fn test_negotiate_language_prefix_match() {
        // "zh" 应该匹配 "zh-CN"
        assert_eq!(negotiate_language(&["zh"], "en"), "zh-CN");
    }

    #[test]
    fn test_negotiate_language_fallback() {
        // "fr" 不在支持列表中，应该回退到默认值
        assert_eq!(negotiate_language(&["fr"], "en"), "en");
        assert_eq!(negotiate_language(&["fr"], "zh-CN"), "zh-CN");
    }

    #[test]
    fn test_negotiate_language_priority() {
        // 第一个匹配的语言优先
        assert_eq!(negotiate_language(&["zh-CN", "en"], "fr"), "zh-CN");
        assert_eq!(negotiate_language(&["en", "zh-CN"], "fr"), "en");
    }

    #[test]
    fn test_parse_language_list() {
        let locales = parse_language_list("zh-CN:zh,en");
        assert_eq!(locales, vec!["zh-cn", "zh", "en"]);

        let locales = parse_language_list("en");
        assert_eq!(locales, vec!["en"]);

        let locales = parse_language_list("");
        assert_eq!(locales, Vec::<String>::new());
    }

    #[test]
    fn test_validate_translations() {
        let result = validate_translations();
        assert!(
            result.is_ok(),
            "Translation validation failed: {:?}",
            result.err()
        );
    }
}
