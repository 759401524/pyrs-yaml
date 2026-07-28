//! # pyyaml-rs
//!
//! A high-performance Python YAML library with perfect round-trip support.

pub mod ast;
pub mod i18n;
pub mod parser;
pub mod serializer;

#[cfg(test)]
mod integration;

// rust-i18n 初始化
rust_i18n::i18n!("src/i18n/locales");

use ast::CustomNode;
use indexmap::IndexMap;
use numpy::{
    dtype, Complex32, Complex64, PyArrayDescrMethods, PyArrayDyn, PyArrayMethods, PyUntypedArray,
    PyUntypedArrayMethods,
};
use parser::yaml::YamlSchema;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

// 自定义 Python 异常类型
pyo3::create_exception!(pyyaml_rs, YamlParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(
    pyyaml_rs,
    YamlSerializeError,
    pyo3::exceptions::PyValueError
);
pyo3::create_exception!(pyyaml_rs, YamlTypeError, pyo3::exceptions::PyTypeError);
pyo3::create_exception!(pyyaml_rs, YamlValidateError, pyo3::exceptions::PyValueError);

/// A Python module implemented in Rust.
///
/// pyyaml-rs: high-performance YAML parsing with perfect round-trip support.
#[pymodule]
mod pyyaml_rs {
    use super::*;

    use super::parser::{StreamEvent, StreamEventType};
    #[pymodule_export]
    use super::YamlParseError;
    #[pymodule_export]
    use super::YamlSerializeError;
    #[pymodule_export]
    use super::YamlTypeError;
    #[pymodule_export]
    use super::YamlValidateError;

    /// 格式化 i18n 错误消息，将翻译模板中的占位符替换为实际值。
    fn format_i18n_error(key: &str, args: &[(&str, &str)]) -> String {
        super::i18n::format_message(key, args)
    }

    /// Python wrapper for the parsed YAML document.
    #[pyclass]
    struct YamlDocument {
        ast: CustomNode,
        schema: YamlSchema,
        source: Option<Arc<str>>,
    }

    #[pymethods]
    impl YamlDocument {
        /// 将文档序列化为 YAML 字符串（默认 2 空格缩进）。
        ///
        /// # Returns
        /// 完整的 YAML 文档字符串，末尾包含换行符。
        fn to_yaml(&self) -> String {
            super::serializer::to_yaml(&self.ast)
        }

        /// 使用自定义选项将文档序列化为 YAML 字符串。
        ///
        /// # Arguments
        /// * `indent_size` - 每层缩进的空格数，默认 2。
        /// * `explicit_start` - 是否在文档开头添加 `---`。
        /// * `explicit_end` - 是否在文档末尾添加 `...`。
        /// * `sort_keys` - 是否按键名排序（不保留原始顺序）。
        ///
        /// # Returns
        /// 完整的 YAML 文档字符串。
        #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "str")]
        fn to_yaml_with_options(
            &self,
            indent_size: usize,
            explicit_start: bool,
            explicit_end: bool,
            sort_keys: bool,
        ) -> String {
            let options = super::serializer::SerializeOptions {
                indent_size,
                explicit_start,
                explicit_end,
                sort_keys,
            };
            super::serializer::to_yaml_with_options(&self.ast, &options)
        }

        /// 将文档转换为 Python 字典/列表，自动解析锚点引用。
        ///
        /// 锚点（`&name`）指向的节点会被内联展开，别名（`*name`）会被替换为实际值。
        /// 标量值会根据 YAML 1.2 类型规则自动转换为 Python 原生类型（bool/int/float/str/None）。
        fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
            let mut anchors = HashMap::new();
            collect_anchors(&self.ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&self.ast, py, &anchors, &mut visited, self.schema)
        }

        /// 获取映射中指定键的值。
        ///
        /// # Arguments
        /// * `key` - 要查找的键名（字符串）。
        /// * `default` - 键不存在时返回的默认值，默认为 `None`。
        ///
        /// # Returns
        /// 键对应的值，或 `default`（若未提供则返回 `None`）。
        /// 如果根节点不是映射，也返回 `default`。
        #[pyo3(signature = (key: "str", default: "Any" = None) -> "Any")]
        fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let key_node = CustomNode::plain_scalar(key);
                    if let Some(value) = pairs.get(&key_node) {
                        Ok(node_to_pyobject(value, py, self.schema)?)
                    } else {
                        Ok(default.unwrap_or_else(|| py.None()))
                    }
                }
                _ => Ok(default.unwrap_or_else(|| py.None())),
            }
        }

        /// 返回文档根节点的类型名称。
        ///
        /// # Returns
        /// 可能的返回值：`"scalar"`, `"mapping"`, `"sequence"`, `"null"`, `"alias"`。
        fn root_type(&self) -> String {
            match &self.ast {
                CustomNode::Scalar { .. } => "scalar".to_string(),
                CustomNode::Mapping { .. } => "mapping".to_string(),
                CustomNode::Sequence { .. } => "sequence".to_string(),
                CustomNode::Null { .. } => "null".to_string(),
                CustomNode::Alias { .. } => "alias".to_string(),
            }
        }

        /// 返回调试表示字符串，格式为 `YamlDocument(<yaml>)`。
        fn __repr__(&self) -> String {
            format!("YamlDocument({})", self.to_yaml())
        }

        /// 返回 YAML 字符串表示。
        fn __str__(&self) -> String {
            self.to_yaml()
        }

        /// 检查映射是否包含指定键。
        ///
        /// # Arguments
        /// * `key` - 要检查的键名。
        ///
        /// # Returns
        /// 如果根节点是映射且包含该键返回 `true`，否则返回 `false`。
        fn __contains__(&self, key: &str) -> bool {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let key_node = CustomNode::plain_scalar(key);
                    pairs.contains_key(&key_node)
                }
                _ => false,
            }
        }

        /// 返回映射的键值对数量或序列的元素数量。
        ///
        /// # Returns
        /// 对于映射和序列返回元素数，其他类型返回 0。
        fn __len__(&self) -> usize {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => pairs.len(),
                CustomNode::Sequence { items, .. } => items.len(),
                _ => 0,
            }
        }

        /// 迭代文档内容。映射迭代键，序列迭代值，其他类型返回空列表。
        fn __iter__<'py>(&self, _py: Python<'py>) -> PyResult<Py<PyAny>> {
            let schema = self.schema;
            Python::attach(|py| match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let keys: Vec<Py<PyAny>> = pairs
                        .keys()
                        .map(|k| node_to_pyobject(k, py, schema))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(keys.into_pyobject(py)?.into_any().unbind())
                }
                CustomNode::Sequence { items, .. } => {
                    let values: Vec<Py<PyAny>> = items
                        .iter()
                        .map(|v| node_to_pyobject(v, py, schema))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(values.into_pyobject(py)?.into_any().unbind())
                }
                _ => Ok(Vec::<Py<PyAny>>::new()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()),
            })
        }

        /// 通过下标访问文档内容。
        ///
        /// # Arguments
        /// * `key` - 映射使用字符串键，序列使用整数索引。
        ///
        /// # Errors
        /// 返回 `KeyError`（映射键不存在）、`IndexError`（索引越界）或 `TypeError`（类型不支持下标访问）。
        fn __getitem__<'py>(&self, py: Python<'py>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
            let schema = self.schema;
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    if let Ok(key_str) = key.bind(py).extract::<String>() {
                        let key_node = CustomNode::plain_scalar(key_str.clone());
                        if let Some(value) = pairs.get(&key_node) {
                            Ok(node_to_pyobject(value, py, schema)?)
                        } else {
                            Err(pyo3::exceptions::PyKeyError::new_err(format_i18n_error(
                                "key-not-found",
                                &[("key", key_str.as_str())],
                            )))
                        }
                    } else {
                        Err(YamlTypeError::new_err(format_i18n_error(
                            "key-not-string",
                            &[],
                        )))
                    }
                }
                CustomNode::Sequence { items, .. } => {
                    if let Ok(idx) = key.bind(py).extract::<usize>() {
                        if idx < items.len() {
                            Ok(node_to_pyobject(&items[idx], py, schema)?)
                        } else {
                            Err(pyo3::exceptions::PyIndexError::new_err(format_i18n_error(
                                "index-out-of-range",
                                &[
                                    ("index", &idx.to_string()),
                                    ("len", &items.len().to_string()),
                                ],
                            )))
                        }
                    } else {
                        Err(YamlTypeError::new_err(format_i18n_error(
                            "index-not-integer",
                            &[],
                        )))
                    }
                }
                _ => Err(YamlTypeError::new_err(format_i18n_error(
                    "not-subscriptable",
                    &[],
                ))),
            }
        }

        /// 返回文档的原始 YAML 源文本。
        ///
        /// 当 `YamlDocument` 通过 `parse()` 或 `parse_file()` 创建时，
        /// 原始 YAML 字符串会被保存以便后续使用 `reparse()`。
        ///
        /// # Returns
        /// 原始 YAML 字符串，若不存在则返回 `None`。
        fn source(&self) -> Option<&str> {
            self.source.as_deref()
        }

        /// 根据存储的原始 YAML 源重新解析文档。
        ///
        /// 当文档源文本已被外部修改后，调用此方法可以重新解析，
        /// 将新的 AST 反映到 `YamlDocument` 中。
        ///
        /// # Arguments
        /// * `resolve_merges` - 是否解析 `<<` 合并键，默认 `true`。
        /// * `schema` - 类型解析 Schema，默认为 `"core"`。
        ///
        /// # Returns
        /// 返回 `None`（原地修改 `YamlDocument`）。
        ///
        /// # Errors
        /// 如果未保存源文本（如通过 `parse()` 从字节流创建）则抛出 `YamlTypeError`；
        /// 解析失败时抛出 `YamlParseError`。
        #[pyo3(signature = (resolve_merges: "bool" = true, schema: "str" = "core") -> "None")]
        fn reparse(&mut self, py: Python, resolve_merges: bool, schema: &str) -> PyResult<()> {
            let source = self.source.as_ref().ok_or_else(|| {
                YamlTypeError::new_err(format_i18n_error("no-source-to-reparse", &[]))
            })?;
            let schema_enum = parse_schema(schema)?;
            let new_ast = py.detach(|| {
                super::parser::parse_with_options(source, resolve_merges, schema_enum).map_err(
                    |e| {
                        YamlParseError::new_err(format_i18n_error(
                            "yaml-parse-error",
                            &[("detail", &e.to_string())],
                        ))
                    },
                )
            })?;
            self.ast = new_ast;
            self.schema = schema_enum;
            Ok(())
        }

        /// 将文档内容转换为 JSON 格式。
        ///
        /// 使用 `json.dumps` 序列化后再解析，确保输出为标准 JSON。
        ///
        /// # Arguments
        /// * `indent` - JSON 缩进空格数，默认为 `2`。
        ///
        /// # Returns
        /// JSON 字符串。
        #[pyo3(signature = (indent: "int" = 2) -> "str")]
        fn to_json(&self, py: Python, indent: usize) -> PyResult<String> {
            let obj = self.to_dict(py)?;
            let json_module = py.import("json")?;
            let kw = PyDict::new(py);
            kw.set_item("indent", indent)?;
            let s = json_module
                .call_method("dumps", (obj,), Some(&kw))?
                .extract()?;
            Ok(s)
        }

        /// 验证文档内容是否符合 JSON Schema。
        ///
        /// 调用 Python `jsonschema.validate` 进行验证。如果文档内容
        /// 不符合 Schema，则抛出 `YamlValidateError`。
        ///
        /// # Arguments
        /// * `schema` - JSON Schema 描述，可以是：
        ///   - `str`：Schema 的 JSON 字符串
        ///   - `dict[str, Any]`：Schema 的 Python 字典
        ///
        /// # Returns
        /// 验证成功返回 `None`。
        ///
        /// # Errors
        /// 验证失败时抛出 `YamlValidateError`，错误消息包含 JSON Schema
        /// 库报告的详细错误路径和描述。
        #[pyo3(signature = (schema: "str | dict[str, Any]") -> "None")]
        fn validate(&self, py: Python, schema: &Bound<'_, PyAny>) -> PyResult<()> {
            let instance = self.to_dict(py)?;

            let schema_obj: Bound<'_, PyAny> = if let Ok(schema_str) = schema.extract::<String>() {
                let json_module = py.import("json")?;
                json_module.call_method("loads", (schema_str,), None)?
            } else {
                schema.clone()
            };

            let jsonschema = py.import("jsonschema")?;
            let validate_fn = jsonschema.getattr("validate")?;
            let kw = PyDict::new(py);
            kw.set_item("instance", instance)?;
            kw.set_item("schema", schema_obj)?;
            validate_fn.call((), Some(&kw)).map_err(|e| {
                let msg = e
                    .value(py)
                    .getattr("message")
                    .ok()
                    .and_then(|m| m.extract::<String>().ok())
                    .unwrap_or_else(|| e.to_string());
                YamlValidateError::new_err(msg)
            })?;

            Ok(())
        }
    }

    // ---- helper functions (not exposed to Python) ----

    /// Parse a schema string into YamlSchema.
    ///
    /// Supported values: "core", "yaml.org,2002", "YamlOrg2002" → Core
    /// "json", "yaml.org,2002:json" → Json
    /// "failsafe", "yaml.org,2002:failsafe" → Failsafe
    /// "yaml1.1", "1.1", "yaml.org,2002:yaml1.1" → Yaml11
    fn parse_schema(raw: &str) -> PyResult<YamlSchema> {
        let s = raw.to_lowercase();
        match s.as_str() {
            "core" | "yaml.org,2002" | "yamlorg2002" => Ok(YamlSchema::Core),
            "json" | "yaml.org,2002:json" => Ok(YamlSchema::Json),
            "failsafe" | "yaml.org,2002:failsafe" => Ok(YamlSchema::Failsafe),
            "yaml1.1" | "1.1" | "yaml.org,2002:yaml1.1" => Ok(YamlSchema::Yaml11),
            _ => Err(YamlTypeError::new_err(format!(
                "Unsupported schema '{}'. Supported: core, json, failsafe, yaml1.1",
                raw
            ))),
        }
    }

    /// 递归遍历 AST，收集所有锚点到节点的映射，用于别名解析。
    fn collect_anchors<'a>(node: &'a CustomNode, anchors: &mut HashMap<String, &'a CustomNode>) {
        if let Some(name) = node.anchor() {
            anchors.insert(name.to_string(), node);
        }
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (key, value) in pairs {
                    collect_anchors(key, anchors);
                    collect_anchors(value, anchors);
                }
            }
            CustomNode::Sequence { items, .. } => {
                for item in items {
                    collect_anchors(item, anchors);
                }
            }
            _ => {}
        }
    }

    /// 将 `CustomNode` 转换为 Python 对象，解析别名引用（`*alias`）为实际值。
    fn node_to_pyobject_with_anchors(
        node: &CustomNode,
        py: Python,
        anchors: &HashMap<String, &CustomNode>,
        visited: &mut HashSet<usize>,
        schema: YamlSchema,
    ) -> PyResult<Py<PyAny>> {
        match node {
            CustomNode::Alias { name } => {
                if let Some(target) = anchors.get(name) {
                    let addr = std::ptr::addr_of!(*target) as usize;
                    if visited.contains(&addr) {
                        return Ok(py.None());
                    }
                    visited.insert(addr);
                    node_to_pyobject_with_anchors(target, py, anchors, visited, schema)
                } else {
                    Ok(py.None())
                }
            }
            _ => node_to_pyobject_inner(node, py, anchors, visited, schema),
        }
    }

    /// 将 `CustomNode` 转换为 Python 对象，不解析别名（别名节点返回 `None`）。
    fn node_to_pyobject(node: &CustomNode, py: Python, schema: YamlSchema) -> PyResult<Py<PyAny>> {
        node_to_pyobject_simple(node, py, schema)
    }

    /// 内部转换逻辑，被 `node_to_pyobject` 和 `node_to_pyobject_with_anchors` 共用。
    fn node_to_pyobject_inner(
        node: &CustomNode,
        py: Python,
        anchors: &HashMap<String, &CustomNode>,
        visited: &mut HashSet<usize>,
        schema: YamlSchema,
    ) -> PyResult<Py<PyAny>> {
        match node {
            CustomNode::Scalar { value, style, .. } => {
                // Plain scalars always get type resolution.
                // Quoted scalars (Single/Double) also get type resolution for round-trip correctness
                // — the serializer may quote negative numbers (YAML 1.2 block-sequence restriction),
                // and they should parse back as numbers.
                // Literal/Folded block scalars are always strings.
                if matches!(
                    style,
                    ast::ScalarStyle::Plain
                        | ast::ScalarStyle::SingleQuoted
                        | ast::ScalarStyle::DoubleQuoted
                ) {
                    use parser::yaml::{resolve_yaml_type, YamlType};
                    match resolve_yaml_type(value, schema) {
                        YamlType::Null => Ok(py.None()),
                        YamlType::Bool(b) => Ok(pyo3::types::PyBool::new(py, b)
                            .to_owned()
                            .into_any()
                            .unbind()),
                        YamlType::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Str(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                    }
                } else {
                    Ok(value.clone().into_pyobject(py)?.into_any().unbind())
                }
            }
            CustomNode::Mapping { pairs, .. } => {
                let dict = PyDict::new(py);
                for (key, value) in pairs {
                    let key_str = match key {
                        CustomNode::Scalar { value, .. } => value.clone(),
                        _ => format!("{:?}", key),
                    };
                    let val = node_to_pyobject_with_anchors(value, py, anchors, visited, schema)?;
                    dict.set_item(key_str, val).ok();
                }
                Ok(dict.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let list = pyo3::types::PyList::empty(py);
                for item in items {
                    let val = node_to_pyobject_with_anchors(item, py, anchors, visited, schema)?;
                    list.append(val).ok();
                }
                Ok(list.into_any().unbind())
            }
            CustomNode::Null { .. } => Ok(py.None()),
            CustomNode::Alias { .. } => Ok(py.None()),
        }
    }

    /// Simple conversion path without alias resolution.
    fn node_to_pyobject_simple(
        node: &CustomNode,
        py: Python,
        schema: YamlSchema,
    ) -> PyResult<Py<PyAny>> {
        match node {
            CustomNode::Scalar { value, style, .. } => {
                if matches!(
                    style,
                    ast::ScalarStyle::Plain
                        | ast::ScalarStyle::SingleQuoted
                        | ast::ScalarStyle::DoubleQuoted
                ) {
                    use parser::yaml::{resolve_yaml_type, YamlType};
                    match resolve_yaml_type(value, schema) {
                        YamlType::Null => Ok(py.None()),
                        YamlType::Bool(b) => Ok(pyo3::types::PyBool::new(py, b)
                            .to_owned()
                            .into_any()
                            .unbind()),
                        YamlType::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Str(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                    }
                } else {
                    Ok(value.clone().into_pyobject(py)?.into_any().unbind())
                }
            }
            CustomNode::Mapping { pairs, .. } => {
                let dict = PyDict::new(py);
                for (key, value) in pairs {
                    let key_str = match key {
                        CustomNode::Scalar { value, .. } => value.clone(),
                        _ => format!("{:?}", key),
                    };
                    let val = node_to_pyobject_simple(value, py, schema)?;
                    dict.set_item(key_str, val).ok();
                }
                Ok(dict.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let list = pyo3::types::PyList::empty(py);
                for item in items {
                    let val = node_to_pyobject_simple(item, py, schema)?;
                    list.append(val).ok();
                }
                Ok(list.into_any().unbind())
            }
            CustomNode::Null { .. } => Ok(py.None()),
            CustomNode::Alias { .. } => Ok(py.None()),
        }
    }

    /// 将 NumPy ndarray 序列化为嵌套 YAML 列表（通过 `__array_interface__`）。
    ///
    /// 使用 numpy crate 的 `PyUntypedArray` 读取 shape 和 dtype，
    /// 根据 dtype 分派到对应 Rust 类型的 `PyArray1<T>`，
    /// 通过 `as_slice()` 获取 `&[T]` 切片，释放 GIL 后转换为 `CustomNode`。
    ///
    /// 支持：int8/16/32/64、uint8/16/32/64、float32/64、complex64/128、bool。
    /// 0-D 标量数组直接返回标量节点。
    ///
    /// # Arguments
    /// * `py` — Python GIL 上下文
    /// * `obj` — Python 对象（NumPy ndarray）
    ///
    /// # Returns
    /// 成功时返回嵌套的 `CustomNode`，失败时返回 `None`。
    fn ndarray_to_node(py: Python, obj: &Bound<'_, PyAny>) -> Option<CustomNode> {
        let arr = obj.cast::<PyUntypedArray>().ok()?;

        // 0-D 标量：reshape(1) → 1-D，递归处理
        if arr.ndim() == 0 {
            let reshape = obj.getattr("reshape").ok()?;
            let tuple_cls = py.import("builtins").ok()?.getattr("tuple").ok()?;
            let shape_arg = tuple_cls.call1((1usize,)).ok()?;
            let reshaped = reshape.call1((shape_arg,)).ok()?;
            return ndarray_to_node(py, &reshaped);
        }
        let shape = arr.shape();
        let total = shape.iter().product::<usize>();

        // 空数组
        if total == 0 {
            return Some(CustomNode::plain_sequence(Vec::new()));
        }

        macro_rules! dispatch_dtype {
            ($ty:ty, $to_scalar:expr) => {
                if arr.dtype().is_equiv_to(&dtype::<$ty>(py)) {
                    let typed = arr.cast::<PyArrayDyn<$ty>>().ok()?;
                    let slice = unsafe { typed.as_slice() }.ok()?;
                    let flat =
                        py.detach(|| slice.iter().map($to_scalar).collect::<Vec<CustomNode>>());
                    let mut result = flat;
                    for &dim in shape[1..].iter().rev() {
                        result = nest_ndarray_sequence(result, dim);
                    }
                    // shape[1..] 处理除根维度外的嵌套；plain_sequence 包装根维度
                    return Some(CustomNode::plain_sequence(result));
                }
            };
        }

        dispatch_dtype!(i8, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(i16, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(i32, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(i64, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(u8, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(u16, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(u32, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(u64, |v| CustomNode::plain_scalar(v.to_string()));
        dispatch_dtype!(f32, |v| CustomNode::plain_scalar(if v.is_nan() {
            "NaN".to_string()
        } else {
            v.to_string()
        }));
        dispatch_dtype!(f64, |v| CustomNode::plain_scalar(if v.is_nan() {
            "NaN".to_string()
        } else {
            v.to_string()
        }));
        dispatch_dtype!(bool, |v| CustomNode::plain_scalar(if *v {
            "true"
        } else {
            "false"
        }));
        dispatch_dtype!(Complex64, |c| CustomNode::plain_scalar(format!(
            "({}+{}j)",
            c.re, c.im
        )));
        dispatch_dtype!(Complex32, |c| CustomNode::plain_scalar(format!(
            "({}+{}j)",
            c.re, c.im
        )));

        None
    }

    /// 将展平的 `Vec<CustomNode>` 按 `dim` 嵌套一层。
    ///
    /// # Arguments
    /// * `flat` — 展平的节点列表
    /// * `dim` — 嵌套维度大小
    ///
    /// # Returns
    /// 嵌套后的 `Vec<CustomNode>`，每个元素是 `CustomNode::Sequence`。
    fn nest_ndarray_sequence(flat: Vec<CustomNode>, dim: usize) -> Vec<CustomNode> {
        if dim == 0 {
            return vec![CustomNode::plain_sequence(Vec::new())];
        }
        flat.chunks(dim)
            .map(|chunk| CustomNode::plain_sequence(chunk.to_vec()))
            .collect()
    }

    /// 将 Python 对象递归转换为 `CustomNode` AST 节点，支持 dict/list/str/int/float/bool/None/ndarray。
    fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
        let obj = obj.bind(py);

        if obj.is_none() {
            return Ok(CustomNode::plain_null());
        }

        if let Ok(dict) = obj.cast::<PyDict>() {
            let mut pairs = IndexMap::new();
            for (key, value) in dict.iter() {
                let key_node = if let Ok(key_str) = key.extract::<String>() {
                    CustomNode::plain_scalar(key_str)
                } else {
                    pyobject_to_node(py, &key.into_any().unbind())?
                };
                let value_node = pyobject_to_node(py, &value.into_any().unbind())?;
                pairs.insert(key_node, value_node);
            }
            return Ok(CustomNode::plain_mapping(pairs));
        }

        if let Ok(list) = obj.cast::<pyo3::types::PyList>() {
            let items: Vec<CustomNode> = list
                .iter()
                .map(|item| pyobject_to_node(py, &item.into_any().unbind()))
                .collect::<Result<_, _>>()?;
            return Ok(CustomNode::plain_sequence(items));
        }

        if let Ok(b) = obj.extract::<bool>() {
            return Ok(CustomNode::plain_scalar(if b { "true" } else { "false" }));
        }

        if let Ok(n) = obj.extract::<i64>() {
            return Ok(CustomNode::plain_scalar(n.to_string()));
        }

        if let Ok(f) = obj.extract::<f64>() {
            return Ok(CustomNode::plain_scalar(f.to_string()));
        }

        if let Ok(s) = obj.extract::<String>() {
            return Ok(CustomNode::plain_scalar(s));
        }

        // NumPy ndarray 支持：通过 `__array_interface__` 序列化
        if let Some(node) = ndarray_to_node(py, obj) {
            return Ok(node);
        }

        Err(YamlTypeError::new_err(format_i18n_error(
            "unsupported-type",
            &[],
        )))
    }

    /// 将 `serde_json::Value` 转换为 `CustomNode` AST 节点。
    fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
        match value {
            serde_json::Value::Null => Ok(CustomNode::plain_null()),
            serde_json::Value::Bool(b) => {
                Ok(CustomNode::plain_scalar(if *b { "true" } else { "false" }))
            }
            serde_json::Value::Number(n) => {
                let s = if let Some(i) = n.as_i64() {
                    i.to_string()
                } else if let Some(f) = n.as_f64() {
                    f.to_string()
                } else {
                    n.to_string()
                };
                Ok(CustomNode::plain_scalar(s))
            }
            serde_json::Value::String(s) => Ok(CustomNode::plain_scalar(s.clone())),
            serde_json::Value::Array(arr) => {
                let items: Vec<CustomNode> = arr
                    .iter()
                    .map(json_value_to_node)
                    .collect::<Result<_, _>>()?;
                Ok(CustomNode::plain_sequence(items))
            }
            serde_json::Value::Object(map) => {
                let mut pairs = IndexMap::new();
                for (key, value) in map {
                    let key_node = CustomNode::plain_scalar(key.clone());
                    let value_node = json_value_to_node(value)?;
                    pairs.insert(key_node, value_node);
                }
                Ok(CustomNode::plain_mapping(pairs))
            }
        }
    }

    // ---- Python-facing functions ----

    /// 解析 YAML 字符串或字节流，返回 `YamlDocument` 对象。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
    fn parse(
        py: Python,
        yaml: &Bound<'_, PyAny>,
        resolve_merges: bool,
        schema: &str,
    ) -> PyResult<YamlDocument> {
        let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
            s
        } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
            String::from_utf8(bytes).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "invalid-utf8",
                    &[("detail", &e.to_string())],
                ))
            })?
        } else {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-str-or-bytes",
                &[],
            )));
        };

        let schema_enum = parse_schema(schema)?;
        let ast = py.detach(|| {
            super::parser::parse_with_options(&yaml_str, resolve_merges, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(yaml_str)),
        })
    }

    /// 解析 YAML 文件，返回 `YamlDocument` 对象。
    #[pyfunction]
    #[pyo3(signature = (path: "str", schema: "str" = "core") -> "YamlDocument")]
    fn parse_file(py: Python, path: &str, schema: &str) -> PyResult<YamlDocument> {
        let schema_enum = parse_schema(schema)?;
        let content = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let ast = py.detach(|| {
            super::parser::parse_with_options(&content, true, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(content)),
        })
    }

    /// 解析包含多个 YAML 文档的字符串（以 `---` 分隔）。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core") -> "list[YamlDocument]")]
    fn parse_all_docs(
        py: Python,
        yaml: &str,
        resolve_merges: bool,
        schema: &str,
    ) -> PyResult<Vec<YamlDocument>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            super::parser::parse_all_with_options(yaml, resolve_merges, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(asts
            .into_iter()
            .map(|ast| YamlDocument {
                ast,
                schema: schema_enum,
                source: Some(Arc::from(yaml)),
            })
            .collect())
    }

    /// PyYAML 兼容接口：解析 YAML 字符串并返回原生 Python 对象。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core") -> "dict[str, Any] | list[Any]")]
    fn safe_load(py: Python, yaml: &str, schema: &str) -> PyResult<Py<PyAny>> {
        let schema_enum = parse_schema(schema)?;
        let ast = py.detach(|| {
            super::parser::parse(yaml, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        let mut anchors = HashMap::new();
        collect_anchors(&ast, &mut anchors);
        let mut visited = HashSet::new();
        node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)
    }

    /// PyYAML 兼容接口：解析多个 YAML 文档并返回原生 Python 对象列表。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(py: Python, yaml: &str, schema: &str) -> PyResult<Vec<Py<PyAny>>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            super::parser::parse_all(yaml, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        asts.iter()
            .map(|ast| {
                let mut anchors = HashMap::new();
                collect_anchors(ast, &mut anchors);
                let mut visited = HashSet::new();
                node_to_pyobject_with_anchors(ast, py, &anchors, &mut visited, schema_enum)
            })
            .collect()
    }

    /// Convert a `StreamEvent` to a Python dict.
    fn stream_event_to_py_dict<'a>(
        py: Python<'a>,
        event: &StreamEvent,
    ) -> PyResult<Bound<'a, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("line", event.line)?;
        dict.set_item("column", event.column)?;

        match &event.event_type {
            StreamEventType::StreamStart => {
                dict.set_item("type", "stream_start")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::StreamEnd => {
                dict.set_item("type", "stream_end")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::DocumentStart => {
                dict.set_item("type", "document_start")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::DocumentEnd => {
                dict.set_item("type", "document_end")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => {
                dict.set_item("type", "scalar")?;
                dict.set_item("value", value)?;
                dict.set_item(
                    "style",
                    match style {
                        ast::ScalarStyle::Plain => "plain",
                        ast::ScalarStyle::SingleQuoted => "single_quoted",
                        ast::ScalarStyle::DoubleQuoted => "double_quoted",
                        ast::ScalarStyle::Literal => "literal",
                        ast::ScalarStyle::Folded => "folded",
                    },
                )?;
                if let Some(a) = anchor {
                    dict.set_item("anchor", a)?;
                } else {
                    dict.set_item("anchor", py.None())?;
                }
                if let Some(t) = tag {
                    dict.set_item("tag", format!("{}{}", t.handle, t.suffix))?;
                } else {
                    dict.set_item("tag", py.None())?;
                }
            }
            StreamEventType::MappingStart { anchor, tag } => {
                dict.set_item("type", "mapping_start")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                if let Some(a) = anchor {
                    dict.set_item("anchor", a)?;
                } else {
                    dict.set_item("anchor", py.None())?;
                }
                if let Some(t) = tag {
                    dict.set_item("tag", format!("{}{}", t.handle, t.suffix))?;
                } else {
                    dict.set_item("tag", py.None())?;
                }
            }
            StreamEventType::MappingEnd => {
                dict.set_item("type", "mapping_end")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::SequenceStart { anchor, tag } => {
                dict.set_item("type", "sequence_start")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                if let Some(a) = anchor {
                    dict.set_item("anchor", a)?;
                } else {
                    dict.set_item("anchor", py.None())?;
                }
                if let Some(t) = tag {
                    dict.set_item("tag", format!("{}{}", t.handle, t.suffix))?;
                } else {
                    dict.set_item("tag", py.None())?;
                }
            }
            StreamEventType::SequenceEnd => {
                dict.set_item("type", "sequence_end")?;
                dict.set_item("value", py.None())?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::Alias { name } => {
                dict.set_item("type", "alias")?;
                dict.set_item("value", name)?;
                dict.set_item("style", py.None())?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
            StreamEventType::Comment { text, standalone } => {
                dict.set_item("type", "comment")?;
                dict.set_item("value", text)?;
                dict.set_item("style", if *standalone { "standalone" } else { "inline" })?;
                dict.set_item("anchor", py.None())?;
                dict.set_item("tag", py.None())?;
            }
        }

        Ok(dict)
    }

    /// Stream-parse a YAML string, yielding events one at a time.
    ///
    /// In generator mode (no callback), returns a `StreamIterator` that
    /// yields event dicts. In callback mode, calls `on_event` for each
    /// event and returns `None`.
    ///
    /// # Arguments
    /// * `yaml` - YAML content string or bytes.
    /// * `on_event` - Optional callback called per event. Return `False` to stop iteration.
    ///
    /// # Returns
    /// A `StreamIterator` in generator mode, or `None` in callback mode.
    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", on_event: "Callable[[dict[str, Any]], bool] | None" = None) -> "StreamIterator | None")]
    fn parse_stream(
        py: Python,
        yaml: &Bound<'_, PyAny>,
        on_event: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
            s
        } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
            String::from_utf8(bytes).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "invalid-utf8",
                    &[("detail", &e.to_string())],
                ))
            })?
        } else {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-str-or-bytes",
                &[],
            )));
        };

        if let Some(callback) = on_event {
            let events = py.detach(|| {
                super::parser::parse_stream(&yaml_str).map_err(|e| {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.to_string())],
                    ))
                })
            })?;

            Python::attach(|py| -> PyResult<()> {
                let cb = callback.bind(py);
                for event in &events {
                    let py_event = stream_event_to_py_dict(py, event)?;
                    let should_continue: bool = cb.call1((py_event,))?.extract()?;
                    if !should_continue {
                        break;
                    }
                }
                Ok(())
            })?;
            Ok(py.None())
        } else {
            let events = py.detach(|| {
                super::parser::parse_stream(&yaml_str).map_err(|e| {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.to_string())],
                    ))
                })
            })?;

            let iter = StreamIterator { events, index: 0 };
            Ok(iter.into_pyobject(py)?.into_any().unbind())
        }
    }

    /// Iterator class for stream parsing events.
    #[pyclass]
    struct StreamIterator {
        events: Vec<StreamEvent>,
        index: usize,
    }

    #[pymethods]
    impl StreamIterator {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__<'a>(&mut self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDict>>> {
            if self.index < self.events.len() {
                let event = &self.events[self.index];
                self.index += 1;
                Ok(Some(stream_event_to_py_dict(py, event)?))
            } else {
                Ok(None)
            }
        }
    }

    /// PyYAML 兼容接口：将 Python 对象序列化为 YAML 字符串。
    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(super::serializer::to_yaml(&node))
    }

    /// 将 Python 字典/列表转换为 YAML 字符串。
    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(super::serializer::to_yaml(&node))
    }

    /// 将 JSON 字符串转换为 YAML 字符串。
    #[pyfunction]
    #[pyo3(signature = (json_str: "str") -> "str")]
    fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
        let json_value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            YamlParseError::new_err(format_i18n_error(
                "json-parse-error",
                &[("detail", &e.to_string())],
            ))
        })?;
        let node = json_value_to_node(&json_value)?;
        Ok(super::serializer::to_yaml(&node))
    }

    /// 将 Python 对象序列化为 YAML 并写入文件。
    #[pyfunction]
    #[pyo3(signature = (data: "Any", path: "str") -> "None")]
    fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
        let node = pyobject_to_node(py, &data)?;
        let yaml = super::serializer::to_yaml(&node);
        std::fs::write(path, yaml).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-write-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        Ok(())
    }

    /// 从 Markdown 文件中读取 YAML frontmatter。
    #[pyfunction]
    #[pyo3(signature = (path: "str", schema: "str" = "core") -> "tuple[dict[str, Any] | None, str]")]
    fn read_markdown(
        py: Python,
        path: &str,
        schema: &str,
    ) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = py.detach(|| {
            std::fs::read_to_string(path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                    "file-read-error",
                    &[("detail", &e.to_string()), ("path", path)],
                ))
            })
        })?;
        read_markdown_str(py, &content, schema)
    }

    /// 从 Markdown 字符串中读取 YAML frontmatter。
    #[pyfunction]
    #[pyo3(signature = (content: "str", schema: "str" = "core") -> "tuple[dict[str, Any] | None, str]")]
    fn read_markdown_str(
        _py: Python,
        content: &str,
        schema: &str,
    ) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = content.trim_start();
        let schema_enum = parse_schema(schema)?;

        if let Some(rest) = content.strip_prefix("---") {
            if let Some(end_idx) = rest.find("---") {
                let frontmatter = rest[..end_idx].trim();
                let markdown_content = rest[end_idx + 3..].trim();

                if !frontmatter.is_empty() {
                    return Python::attach(|py| {
                        let ast = super::parser::parse(frontmatter, schema_enum).map_err(|e| {
                            YamlParseError::new_err(format_i18n_error(
                                "yaml-parse-error",
                                &[("detail", &e.to_string())],
                            ))
                        })?;
                        Ok((
                            Some(node_to_pyobject(&ast, py, schema_enum)?),
                            markdown_content.to_string(),
                        ))
                    });
                }
            }
        }

        Ok((None, content.to_string()))
    }

    /// 设置错误消息语言。
    #[pyfunction]
    #[pyo3(signature = (lang: "str") -> "None")]
    fn set_language(lang: &str) -> PyResult<()> {
        i18n::set_language(lang).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format_i18n_error(
                "unsupported-language",
                &[
                    ("lang", lang),
                    ("supported", &format!("{:?}", i18n::SUPPORTED_LANGUAGES)),
                ],
            ))
        })
    }

    /// 获取当前错误消息语言。
    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    fn get_language() -> &'static str {
        i18n::get_language_static()
    }

    /// 列出所有支持的语言。
    #[pyfunction]
    #[pyo3(signature = () -> "list[str]")]
    fn list_languages() -> Vec<&'static str> {
        i18n::list_languages()
    }

    /// 自动检测用户偏好语言（从环境变量）。
    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    fn detect_language() -> String {
        i18n::detect_language()
    }

    /// BCP 47 语言协商：从用户提供的语言列表中匹配最合适的支持语言。
    #[pyfunction]
    #[pyo3(signature = (user_locales: "list[str]", default: "str" = "en") -> "str")]
    fn negotiate_language(user_locales: &Bound<'_, PyAny>, default: &str) -> PyResult<String> {
        let locales: Vec<String> = user_locales.extract()?;
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        Ok(i18n::negotiate_language(&refs, default).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::parser::{parse, yaml::YamlSchema};

    #[test]
    fn test_parse_and_serialize() {
        let yaml = "key: value";
        let ast = parse(yaml, YamlSchema::Core).unwrap();
        let output = super::serializer::to_yaml(&ast);
        assert_eq!(output, "key: value\n");
    }
}
