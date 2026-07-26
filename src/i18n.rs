//! 国际化模块：使用 fluent-rs 提供错误消息本地化支持。

use fluent::{FluentBundle, FluentResource};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::LazyLock;

/// 支持的语言列表
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh-CN"];

/// Fluent 资源缓存：语言 → 资源字符串
static RESOURCES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("en", include_str!("i18n/en.ftl"));
    m.insert("zh-CN", include_str!("i18n/zh-CN.ftl"));
    m
});

// 当前语言状态
thread_local! {
    static CURRENT_LANG: RefCell<String> = RefCell::new("en".to_string());
}

/// 设置当前语言
///
/// # Arguments
/// * `lang` - 语言代码，支持 "en" 和 "zh-CN"
///
/// # Errors
/// 返回错误字符串如果语言不受支持
pub fn set_language(lang: &str) -> Result<(), &'static str> {
    if !SUPPORTED_LANGUAGES.contains(&lang) {
        return Err("Unsupported language");
    }
    CURRENT_LANG.with(|c| {
        *c.borrow_mut() = lang.to_string();
    });
    Ok(())
}

/// 获取当前语言
pub fn get_language() -> String {
    CURRENT_LANG.with(|c| c.borrow().clone())
}

/// 获取当前语言的静态引用（用于 Python 绑定）
pub fn get_language_static() -> &'static str {
    let lang = get_language();
    // SAFETY: Supported language strings are &'static
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

/// 格式化错误消息
pub fn format_message(key: &str, args: &[(&str, &str)]) -> String {
    let lang = get_language();
    let resource_str = RESOURCES.get(lang.as_str())
        .or_else(|| RESOURCES.get("en"))
        .unwrap();

    let res = match FluentResource::try_new(resource_str.to_string()) {
        Ok(r) => r,
        Err(_) => return format!("[i18n error: {}]", key),
    };

    let mut bundle: FluentBundle<&FluentResource> = FluentBundle::default();
    let _ = bundle.add_resource(&res);

    let mut fluent_args = fluent::FluentArgs::new();
    for (k, v) in args {
        fluent_args.set(*k, (*v).to_string());
    }

    match bundle.get_message(key) {
        Some(msg) => match msg.value() {
            Some(pattern) => {
                let mut errors = vec![];
                let result = bundle
                    .format_pattern(pattern, Some(&fluent_args), &mut errors)
                    .to_string();
                // Strip Fluent span markers
                result.replace(['\u{2068}', '\u{2069}'], "")
            }
            None => format!("[i18n missing: {}]", key),
        },
        None => format!("[i18n missing: {}]", key),
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
        let msg = format_message("yaml-parse-error", &[("detail", "unexpected token")]);
        assert!(msg.contains("YAML parse error"));
        assert!(msg.contains("unexpected token"));
    }

    #[test]
    fn test_format_message_chinese() {
        let _ = set_language("zh-CN");
        let msg = format_message("yaml-parse-error", &[("detail", "意外的标记")]);
        assert!(msg.contains("解析错误"));
        assert!(msg.contains("意外的标记"));
        let _ = set_language("en");
    }

    #[test]
    fn test_format_message_missing_key() {
        let msg = format_message("nonexistent-key", &[]);
        assert!(msg.contains("[i18n missing: nonexistent-key]"));
    }

    #[test]
    fn test_format_message_no_args() {
        let msg = format_message("key-not-string", &[]);
        assert!(!msg.is_empty());
        assert!(!msg.contains("[i18n"));
    }
}
