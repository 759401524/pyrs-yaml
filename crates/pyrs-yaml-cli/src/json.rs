//! JSON ⇄ `CustomNode` projection via the core native engine.
//!
//! The CLI's JSON surface now delegates entirely to `pyrs_yaml_core::json`
//! (RFC 8259 grammar, exact line/col errors, verbatim number spelling). The
//! historical `serde_json::Value` intermediate is gone.

use pyrs_yaml_core::ast::CustomNode;

/// AST -> compact JSON text.
pub fn node_to_json(node: &CustomNode) -> Result<String, String> {
    pyrs_yaml_core::json::to_json_text(node).map_err(|e| e.to_string())
}

/// AST -> pretty JSON text with `indent` spaces per level.
pub fn node_to_json_pretty(node: &CustomNode, indent: usize) -> Result<String, String> {
    pyrs_yaml_core::json::to_json_text_pretty(node, indent).map_err(|e| e.to_string())
}

/// Parse JSON text into the AST with the CLI's minimum-quoting policy:
/// strings that would re-resolve (numbers, bool words, null words) are
/// emitted as DoubleQuoted scalars by the core parser; everything else is
/// Plain.
pub fn json_to_node(text: &str) -> Result<CustomNode, String> {
    pyrs_yaml_core::json::from_json(text).map_err(|e| format!("JSON parse error: {e}"))
}
