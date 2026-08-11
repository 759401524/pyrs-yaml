//! YAML schema registry — maps schema names to [`Schema`] instances.
//!
//! The registry is pre-loaded with the four built-in schemas (`failsafe`,
//! `json`, `core`, `yaml1.1`) and allows custom schemas to be registered at
//! runtime. Lookups use a read lock and are cheap enough for per-document use;
//! the hot per-scalar path resolves via [`Schema::resolve`] without touching
//! the registry.

use crate::parser::yaml::schema::{
    resolve_core_type, resolve_failsafe, resolve_json_type, resolve_yaml11_type,
};
use crate::parser::yaml::types::{Schema, SchemaResolver, YamlType};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

/// Built-in schema resolvers. Each is a zero-sized unit struct that implements
/// [`SchemaResolver`] by delegating to the existing free functions.
#[derive(Debug, Clone, Copy)]
pub struct FailsafeResolver;
#[derive(Debug, Clone, Copy)]
pub struct JsonResolver;
#[derive(Debug, Clone, Copy)]
pub struct CoreResolver;
#[derive(Debug, Clone, Copy)]
pub struct Yaml11Resolver;

impl SchemaResolver for FailsafeResolver {
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        fn inner(v: &str) -> YamlType<'_> {
            resolve_failsafe(v)
        }
        inner(value)
    }
}

impl SchemaResolver for JsonResolver {
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        fn inner(v: &str) -> YamlType<'_> {
            resolve_json_type(v)
        }
        inner(value)
    }
}

impl SchemaResolver for CoreResolver {
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        fn inner(v: &str) -> YamlType<'_> {
            resolve_core_type(v)
        }
        inner(value)
    }
}

impl SchemaResolver for Yaml11Resolver {
    fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
        fn inner(v: &str) -> YamlType<'_> {
            resolve_yaml11_type(v)
        }
        inner(value)
    }
}

static REGISTRY: LazyLock<RwLock<SchemaRegistry>> = LazyLock::new(|| {
    let mut reg = SchemaRegistry {
        schemas: HashMap::new(),
    };
    reg.schemas.insert("failsafe".to_string(), Schema::Failsafe);
    reg.schemas.insert("json".to_string(), Schema::Json);
    reg.schemas.insert("core".to_string(), Schema::Core);
    reg.schemas.insert("yaml1.1".to_string(), Schema::Yaml1_1);
    RwLock::new(reg)
});

/// A registry of named YAML schemas.
#[derive(Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    /// Create a fresh registry pre-loaded with the built-in schemas.
    pub fn new() -> Self {
        let mut schemas = HashMap::new();
        schemas.insert("failsafe".to_string(), Schema::Failsafe);
        schemas.insert("json".to_string(), Schema::Json);
        schemas.insert("core".to_string(), Schema::Core);
        schemas.insert("yaml1.1".to_string(), Schema::Yaml1_1);
        Self { schemas }
    }

    /// Register a custom schema under a name. Overwrites any existing entry.
    pub fn register<R: SchemaResolver + 'static>(&mut self, name: &str, resolver: R) {
        self.schemas
            .insert(name.to_string(), Schema::Custom(Arc::new(resolver)));
    }

    /// Register a custom schema from an already-boxed resolver.
    pub fn register_boxed(&mut self, name: &str, resolver: Arc<dyn SchemaResolver>) {
        self.schemas
            .insert(name.to_string(), Schema::Custom(resolver));
    }

    /// Look up a schema by name.
    pub fn get(&self, name: &str) -> Option<Schema> {
        self.schemas.get(name).cloned()
    }
}

/// Global registry accessors. The global registry is pre-loaded with the four
/// built-in schemas.
pub fn get(name: &str) -> Option<Schema> {
    let reg = REGISTRY.read().ok()?;
    reg.get(name)
}

pub fn exists(name: &str) -> bool {
    get(name).is_some()
}

/// Register a custom resolver into the global registry.
pub fn register<R: SchemaResolver + 'static>(name: &str, resolver: R) {
    if let Ok(mut reg) = REGISTRY.write() {
        reg.register(name, resolver);
    }
}

/// Register a custom resolver (already boxed) into the global registry.
pub fn register_boxed(name: &str, resolver: Arc<dyn SchemaResolver>) {
    if let Ok(mut reg) = REGISTRY.write() {
        reg.register_boxed(name, resolver);
    }
}

/// All registered schema names.
pub fn names() -> Vec<String> {
    let Ok(reg) = REGISTRY.read() else {
        return Vec::new();
    };
    reg.schemas.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::yaml::types::YamlType;

    #[test]
    fn test_builtins_registered() {
        assert!(exists("core"));
        assert!(exists("json"));
        assert!(exists("failsafe"));
        assert!(exists("yaml1.1"));
        assert!(!exists("nope"));
    }

    #[test]
    fn test_register_custom() {
        #[derive(Debug, Clone, Copy)]
        struct UpperResolver;
        impl SchemaResolver for UpperResolver {
            fn resolve<'a>(&self, value: &'a str) -> YamlType<'a> {
                fn inner(v: &str) -> YamlType<'_> {
                    YamlType::Str(std::borrow::Cow::Owned(v.to_uppercase()))
                }
                inner(value)
            }
        }
        register("test_upper", UpperResolver);
        let s = get("test_upper").expect("registered");
        match s.resolve("hello") {
            YamlType::Str(v) => assert_eq!(v.as_ref(), "HELLO"),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn test_override_builtin() {
        register("core", CoreResolver); // re-register same — no-op behavior preserved
        assert!(get("core").is_some());
    }
}
