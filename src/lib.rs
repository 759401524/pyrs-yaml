//! # pyyaml-rs
//!
//! A high-performance Python YAML library with perfect round-trip support.

pub mod ast;
pub mod i18n;
pub mod parser;
pub mod py;
pub mod serializer;

#[cfg(test)]
mod integration;

// rust-i18n 初始化
rust_i18n::i18n!("src/i18n/locales");

// 自定义 Python 异常类型
pyo3::create_exception!(pyyaml_rs, YamlParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(
    pyyaml_rs,
    YamlSerializeError,
    pyo3::exceptions::PyValueError
);
pyo3::create_exception!(pyyaml_rs, YamlTypeError, pyo3::exceptions::PyTypeError);
pyo3::create_exception!(pyyaml_rs, YamlValidateError, pyo3::exceptions::PyValueError);
