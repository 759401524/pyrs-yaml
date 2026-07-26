//! 国际化模块：使用 fluent-rs 提供错误消息本地化支持。

use fluent::FluentBundle;
use fluent::FluentResource;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// 支持的语言列表
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh-CN"];

/// Fluent 资源缓存：语言 → 资源字符串
static RESOURCES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("en", include_str!("i18n/en.ftl"));
    m.insert("zh-CN", include_str!("i18n/zh-CN.ftl"));
    m
});

/// 当前语言状态
static CURRENT_LANG: RwLock<&'static str> = RwLock::new("en");

/// 设置当前语言
pub fn set_language(lang: &str) -> Result<(), &'static str> {
    if !SUPPORTED_LANGUAGES.contains(&lang) {
        return Err("Unsupported language");
    }
    let mut current = CURRENT_LANG.write().map_err(|_| "Lock poisoned")?;
    *current = unsafe { std::mem::transmute::<&str, &'static str>(lang) };
    Ok(())
}

/// 获取当前语言
pub fn get_language() -> &'static str {
    match CURRENT_LANG.read() {
        Ok(lang) => *lang,
        Err(_) => "en",
    }
}

/// 格式化错误消息
pub fn format_message(key: &str, args: &[(&str, &str)]) -> String {
    let lang = get_language();
    let resource_str = RESOURCES.get(lang).unwrap_or_else(|| RESOURCES.get("en").unwrap());

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
                let result = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors).to_string();
                // Strip Fluent span markers (U+2068, U+2069) used for syntax highlighting
                result.replace(['\u{2068}', '\u{2069}'], "")
            }
            None => format!("[i18n missing: {}]", key),
        },
        None => format!("[i18n missing: {}]", key),
    }
}
