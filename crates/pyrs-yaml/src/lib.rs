//! # pyrs-yaml
//!
//! Python YAML library with perfect round-trip support.
//! Built on top of pyrs-yaml-core.

// Re-export core crate for backward compatibility
pub use pyrs_yaml_core::*;

pub mod py;

// PyO3 exception types
pyo3::create_exception!(pyrs_yaml, YamlParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(
    pyrs_yaml,
    YamlSerializeError,
    pyo3::exceptions::PyValueError
);
pyo3::create_exception!(pyrs_yaml, YamlTypeError, pyo3::exceptions::PyTypeError);
pyo3::create_exception!(pyrs_yaml, YamlValidateError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyrs_yaml, YamlEditError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyrs_yaml, YamlPathError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyrs_yaml, YamlMaxDepthError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(
    pyrs_yaml,
    YamlDuplicateKeyError,
    pyo3::exceptions::PyValueError
);
pyo3::create_exception!(pyrs_yaml, YamlTagError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyrs_yaml, YamlTagSkip, crate::YamlTagError);
