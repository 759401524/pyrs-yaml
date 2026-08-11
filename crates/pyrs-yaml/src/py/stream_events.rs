//! Convert `StreamEvent` to Python dict for stream parsing API.

use crate::ast::ScalarStyle;
use crate::parser::{StreamEvent, StreamEventType};

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Set `key` to `value` if present, otherwise to `None`.
fn set_item_or_none<'a>(
    dict: &Bound<'a, PyDict>,
    py: Python<'a>,
    key: &str,
    value: Option<&str>,
) -> PyResult<()> {
    match value {
        Some(v) => dict.set_item(key, v),
        None => dict.set_item(key, py.None()),
    }
}

/// Helper: set common event fields (type, value, style, anchor, tag) on a dict.
pub(crate) fn fill_event_dict<'a>(
    dict: &Bound<'a, PyDict>,
    py: Python<'a>,
    event_type: &str,
    value: Option<&str>,
    style: Option<&str>,
    anchor: Option<&str>,
    tag: Option<&str>,
) -> PyResult<()> {
    dict.set_item("type", event_type)?;
    set_item_or_none(dict, py, "value", value)?;
    set_item_or_none(dict, py, "style", style)?;
    set_item_or_none(dict, py, "anchor", anchor)?;
    set_item_or_none(dict, py, "tag", tag)?;
    Ok(())
}

/// Convert a `StreamEvent` to a Python dict.
pub(crate) fn stream_event_to_py_dict<'a>(
    py: Python<'a>,
    event: &StreamEvent,
) -> PyResult<Bound<'a, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("line", event.line)?;
    dict.set_item("column", event.column)?;

    match &event.event_type {
        StreamEventType::StreamStart => {
            fill_event_dict(&dict, py, "stream_start", None, None, None, None)?;
        }
        StreamEventType::StreamEnd => {
            fill_event_dict(&dict, py, "stream_end", None, None, None, None)?;
        }
        StreamEventType::DocumentStart => {
            fill_event_dict(&dict, py, "document_start", None, None, None, None)?;
        }
        StreamEventType::DocumentEnd => {
            fill_event_dict(&dict, py, "document_end", None, None, None, None)?;
        }
        StreamEventType::Scalar {
            value,
            style,
            anchor,
            tag,
        } => {
            let style_str = match style {
                ScalarStyle::Plain => "plain",
                ScalarStyle::SingleQuoted => "single_quoted",
                ScalarStyle::DoubleQuoted => "double_quoted",
                ScalarStyle::Literal => "literal",
                ScalarStyle::Folded => "folded",
            };
            let anchor_str = anchor.as_deref();
            let tag_str = tag.as_ref().map(|t| format!("{}{}", t.handle, t.suffix));
            fill_event_dict(
                &dict,
                py,
                "scalar",
                Some(value),
                Some(style_str),
                anchor_str,
                tag_str.as_deref(),
            )?;
        }
        StreamEventType::MappingStart { anchor, tag } => {
            let anchor_str = anchor.as_deref();
            let tag_str = tag.as_ref().map(|t| format!("{}{}", t.handle, t.suffix));
            fill_event_dict(
                &dict,
                py,
                "mapping_start",
                None,
                None,
                anchor_str,
                tag_str.as_deref(),
            )?;
        }
        StreamEventType::MappingEnd => {
            fill_event_dict(&dict, py, "mapping_end", None, None, None, None)?;
        }
        StreamEventType::SequenceStart { anchor, tag } => {
            let anchor_str = anchor.as_deref();
            let tag_str = tag.as_ref().map(|t| format!("{}{}", t.handle, t.suffix));
            fill_event_dict(
                &dict,
                py,
                "sequence_start",
                None,
                None,
                anchor_str,
                tag_str.as_deref(),
            )?;
        }
        StreamEventType::SequenceEnd => {
            fill_event_dict(&dict, py, "sequence_end", None, None, None, None)?;
        }
        StreamEventType::Alias { name } => {
            fill_event_dict(&dict, py, "alias", Some(name), None, None, None)?;
        }
        StreamEventType::Comment { text, standalone } => {
            let style_str = if *standalone { "standalone" } else { "inline" };
            fill_event_dict(
                &dict,
                py,
                "comment",
                Some(text.as_ref()),
                Some(style_str),
                None,
                None,
            )?;
        }
    }

    Ok(dict)
}
