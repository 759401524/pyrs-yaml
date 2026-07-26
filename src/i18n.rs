//! 国际化模块：使用 rust-i18n 提供错误消息本地化支持。
//!
//! # 架构
//!
//! 使用 rust-i18n 作为 i18n 后端，翻译文件存储在 `locales/` 目录下（YAML 格式）。
//! 通过 `t!()` 宏实现编译时 key 检查，`format_message()` 提供动态 key 查找接口。
//!
//! # 使用方法
//!
//! ```rust
//! use pyyaml_rs::i18n;
//!
//! // 设置语言
//! i18n::set_language("zh-CN").unwrap();
//!
//! // 获取当前语言
//! assert_eq!(i18n::get_language(), "zh-CN");
//!
//! // 格式化错误消息
//! let msg = i18n::format_message("yaml-parse-error", &[("detail", "test")]);
//! assert!(!msg.is_empty());
//! ```

use std::cell::RefCell;

/// 支持的语言列表
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh-CN"];

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
        _ => "en",
    }
}

/// 列出所有支持的语言
pub fn list_languages() -> Vec<&'static str> {
    SUPPORTED_LANGUAGES.to_vec()
}

/// 格式化错误消息（动态 key 查找 + 运行时参数替换）
///
/// # Arguments
/// * `key` - 消息键
/// * `args` - 格式化参数 (键, 值) 对，格式为 `{key}` 占位符
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
        assert_eq!(langs.len(), 2);
    }

    #[test]
    fn test_format_message_english() {
        let _ = set_language("en");
        let msg = format_message("yaml-parse-error", &[("detail", "test error")]);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_format_message_chinese() {
        let _ = set_language("zh-CN");
        let msg = format_message("yaml-parse-error", &[("detail", "test error")]);
        assert!(!msg.is_empty());
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
}
