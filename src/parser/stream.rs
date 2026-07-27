use crate::ast::{Comment, ScalarStyle, Tag};
use crate::parser::yaml::{
    extract_anchors, extract_comments, unescape_double_quoted, RawAnchor, RawComment,
};
use saphyr_parser::{
    Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver,
};
use std::collections::HashMap;

/// Line and column information for a YAML stream event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEvent {
    /// The type of event that occurred.
    pub event_type: StreamEventType,
    /// The line number (0-indexed) where the event starts.
    pub line: usize,
    /// The column number (0-indexed) where the event starts.
    pub column: usize,
}

/// The type of a streaming YAML event, representing a single occurrence
/// in the YAML document stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEventType {
    /// The beginning of the YAML stream.
    StreamStart,
    /// The end of the YAML stream.
    StreamEnd,
    /// The beginning of a YAML document.
    DocumentStart,
    /// The end of a YAML document.
    DocumentEnd,
    /// A scalar value.
    Scalar {
        /// The scalar text value.
        value: String,
        /// The scalar style (plain, quoted, literal, folded).
        style: ScalarStyle,
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The beginning of a mapping node.
    MappingStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a mapping node.
    MappingEnd,
    /// The beginning of a sequence node.
    SequenceStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a sequence node.
    SequenceEnd,
    /// An alias reference.
    Alias {
        /// The alias name.
        name: String,
    },
    /// A comment line or inline comment.
    Comment {
        /// The comment text (without the `#` prefix and leading space).
        text: String,
        /// Whether this is a standalone line comment (`true`) or an inline
        /// comment at the end of a line (`false`).
        standalone: bool,
    },
}

/// Pre-compute the byte offset of the start of each line in the given text.
///
/// The returned vector has one entry per line, where `line_offsets[line]`
/// is the byte offset of the first byte of that line in `yaml`.
///
/// # Arguments
/// * `yaml` - The raw YAML text.
///
/// # Returns
/// A vector of byte offsets, one per line. The length is `number_of_lines + 1`
/// (the extra entry is the offset past the final character, for convenience).
fn compute_line_offsets(yaml: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(64);
    offsets.push(0);
    for (i, byte) in yaml.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Event receiver that collects `StreamEvent`s from saphyr-parser.
///
/// Mirrors the pattern used by `AstReceiver` in `parser::mod.rs` but
/// emits lightweight structural events instead of building an AST.
///
/// Pending standalone comments are emitted as `StreamEventType::Comment`
/// events before the next structural event that carries anchor/tag/alias.
pub struct StreamReceiver<'a> {
    yaml_text: &'a str,
    line_offsets: Vec<usize>,
    raw_comments: Vec<RawComment>,
    raw_anchors: Vec<RawAnchor>,
    comment_idx: usize,
    events: Vec<StreamEvent>,
    pending_standalone_comment: Option<Comment>,
    anchors: HashMap<usize, String>,
    next_anchor_name: usize,
}

impl<'a> StreamReceiver<'a> {
    fn new(yaml_text: &'a str) -> Self {
        let line_offsets = compute_line_offsets(yaml_text);
        let raw_comments = extract_comments(yaml_text);
        let raw_anchors = extract_anchors(yaml_text);
        Self {
            yaml_text,
            line_offsets,
            raw_comments,
            raw_anchors,
            comment_idx: 0,
            events: Vec::new(),
            pending_standalone_comment: None,
            anchors: HashMap::new(),
            next_anchor_name: 0,
        }
    }

    fn find_inline_comment(&mut self, line: usize, after_col: usize) -> Option<Comment> {
        let saved_idx = self.comment_idx;
        while self.comment_idx < self.raw_comments.len() {
            let c = &self.raw_comments[self.comment_idx];
            if c.line > line {
                break;
            }
            if c.line < line {
                self.comment_idx += 1;
                continue;
            }
            if c.col >= after_col && !c.standalone {
                let comment = Comment {
                    text: c.text.clone(),
                    standalone: false,
                };
                return Some(comment);
            }
            self.comment_idx += 1;
        }
        self.comment_idx = saved_idx;
        None
    }

    fn find_standalone_before_line(&mut self, line: usize) -> Option<Comment> {
        let mut result = None;
        while self.comment_idx < self.raw_comments.len() {
            let c = &self.raw_comments[self.comment_idx];
            if c.line >= line {
                break;
            }
            if c.standalone {
                result = Some(Comment {
                    text: c.text.clone(),
                    standalone: true,
                });
            }
            self.comment_idx += 1;
        }
        result
    }

    fn next_anchor_name_from_raw(&mut self) -> Option<String> {
        if self.next_anchor_name < self.raw_anchors.len() {
            let name = self.raw_anchors[self.next_anchor_name].name.clone();
            self.next_anchor_name += 1;
            Some(name)
        } else {
            None
        }
    }

    fn register_anchor(&mut self, anchor_id: usize, name: String) {
        self.anchors.insert(anchor_id, name);
    }

    fn emit_pending_comment(&mut self, line: usize, column: usize) {
        if let Some(comment) = self.pending_standalone_comment.take() {
            self.events.push(StreamEvent {
                event_type: StreamEventType::Comment {
                    text: comment.text,
                    standalone: comment.standalone,
                },
                line,
                column,
            });
        }
    }
}

impl<'a> SpannedEventReceiver<'a> for StreamReceiver<'a> {
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        let line = span.start.line().saturating_sub(1);
        let column = span.start.col();

        match event {
            Event::StreamStart => {
                self.events.push(StreamEvent {
                    event_type: StreamEventType::StreamStart,
                    line,
                    column,
                });
            }
            Event::StreamEnd => {
                self.emit_pending_comment(line, column);
                self.events.push(StreamEvent {
                    event_type: StreamEventType::StreamEnd,
                    line,
                    column,
                });
            }
            Event::DocumentStart(_) => {
                self.emit_pending_comment(line, column);
                self.events.push(StreamEvent {
                    event_type: StreamEventType::DocumentStart,
                    line,
                    column,
                });
            }
            Event::DocumentEnd => {
                self.emit_pending_comment(line, column);
                self.events.push(StreamEvent {
                    event_type: StreamEventType::DocumentEnd,
                    line,
                    column,
                });
            }
            Event::Scalar(value, style, anchor_id, tag) => {
                let standalone = self.find_standalone_before_line(line);
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                    self.emit_pending_comment(line, column);
                }

                let scalar_style = match style {
                    SaphyrScalarStyle::Plain => ScalarStyle::Plain,
                    SaphyrScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                    SaphyrScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                    SaphyrScalarStyle::Literal => ScalarStyle::Literal,
                    SaphyrScalarStyle::Folded => ScalarStyle::Folded,
                };

                let scalar_value = if matches!(style, SaphyrScalarStyle::DoubleQuoted) {
                    unescape_double_quoted(&value)
                } else {
                    value.to_string()
                };

                let value_end_col = if line < self.line_offsets.len() {
                    let start = self.line_offsets[line];
                    let end = if line + 1 < self.line_offsets.len() {
                        self.line_offsets[line + 1].saturating_sub(1)
                    } else {
                        self.yaml_text.len()
                    };
                    let line_text = &self.yaml_text[start..end];
                    if let Some(comment_pos) = line_text.find('#') {
                        comment_pos
                    } else {
                        line_text.len()
                    }
                } else {
                    value.len()
                };

                let inline = self.find_inline_comment(line, value_end_col);

                let anchor = if anchor_id != 0 {
                    self.next_anchor_name_from_raw()
                } else {
                    None
                };
                if let Some(ref name) = anchor {
                    self.register_anchor(anchor_id, name.clone());
                }

                let tag_obj = convert_tag(tag.as_deref());

                self.events.push(StreamEvent {
                    event_type: StreamEventType::Scalar {
                        value: scalar_value,
                        style: scalar_style,
                        anchor,
                        tag: tag_obj,
                    },
                    line,
                    column,
                });

                if let Some(inline_comment) = inline {
                    self.events.push(StreamEvent {
                        event_type: StreamEventType::Comment {
                            text: inline_comment.text,
                            standalone: false,
                        },
                        line,
                        column: value_end_col,
                    });
                }
            }
            Event::MappingStart(anchor_id, tag) => {
                let standalone = self.find_standalone_before_line(line);
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                }

                let anchor = if anchor_id != 0 {
                    self.next_anchor_name_from_raw()
                } else {
                    None
                };
                if let Some(ref name) = anchor {
                    self.register_anchor(anchor_id, name.clone());
                }

                let tag_obj = convert_tag(tag.as_deref());

                self.emit_pending_comment(line, column);

                self.events.push(StreamEvent {
                    event_type: StreamEventType::MappingStart {
                        anchor,
                        tag: tag_obj,
                    },
                    line,
                    column,
                });
            }
            Event::MappingEnd => {
                self.emit_pending_comment(line, column);
                self.events.push(StreamEvent {
                    event_type: StreamEventType::MappingEnd,
                    line,
                    column,
                });
            }
            Event::SequenceStart(anchor_id, tag) => {
                let standalone = self.find_standalone_before_line(line);
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                }

                let anchor = if anchor_id != 0 {
                    self.next_anchor_name_from_raw()
                } else {
                    None
                };
                if let Some(ref name) = anchor {
                    self.register_anchor(anchor_id, name.clone());
                }

                let tag_obj = convert_tag(tag.as_deref());

                self.emit_pending_comment(line, column);

                self.events.push(StreamEvent {
                    event_type: StreamEventType::SequenceStart {
                        anchor,
                        tag: tag_obj,
                    },
                    line,
                    column,
                });
            }
            Event::SequenceEnd => {
                self.emit_pending_comment(line, column);
                self.events.push(StreamEvent {
                    event_type: StreamEventType::SequenceEnd,
                    line,
                    column,
                });
            }
            Event::Alias(anchor_id) => {
                self.emit_pending_comment(line, column);

                let alias_name = self
                    .anchors
                    .get(&anchor_id)
                    .cloned()
                    .unwrap_or_else(|| format!("alias_{}", anchor_id));

                self.events.push(StreamEvent {
                    event_type: StreamEventType::Alias { name: alias_name },
                    line,
                    column,
                });
            }
            Event::Nothing => {}
        }
    }
}

/// Convert a saphyr-parser Tag to our Tag format.
fn convert_tag(tag: Option<&saphyr_parser::Tag>) -> Option<Tag> {
    tag.map(|t| Tag {
        handle: if t.handle == "tag:yaml.org,2002:" {
            "!!".to_string()
        } else if t.handle == "!" {
            "!".to_string()
        } else {
            t.handle.to_string()
        },
        suffix: t.suffix.to_string(),
    })
}

/// Parse a YAML string into a stream of `StreamEvent`s.
///
/// Consumes the YAML string and returns a vector of streaming events
/// that represent every structural element in the document.
///
/// # Arguments
/// * `yaml` - The YAML content string.
/// * `resolve_merges` - Reserved for future use. Currently has no effect
///   on the streaming output.
///
/// # Returns
/// Success: a `Vec<StreamEvent>` representing the complete event stream.
/// Failure: an error message string.
///
/// # Examples
/// ```ignore
/// let events = parse_stream("key: value", true).unwrap();
/// assert!(!events.is_empty());
/// ```
pub fn parse_stream(yaml: &str, resolve_merges: bool) -> Result<Vec<StreamEvent>, String> {
    let _ = resolve_merges;

    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut receiver = StreamReceiver::new(yaml);
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    Ok(receiver.events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_start_and_end() {
        let events = parse_stream("hello", true).unwrap();
        assert!(!events.is_empty());
        assert!(matches!(
            &events[0].event_type,
            StreamEventType::StreamStart
        ));
        assert!(matches!(
            events.last().unwrap().event_type,
            StreamEventType::StreamEnd
        ));
    }

    #[test]
    fn test_stream_simple_scalar() {
        let events = parse_stream("hello", true).unwrap();
        let scalar = events
            .iter()
            .find(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }));
        assert!(scalar.is_some());
        if let StreamEventType::Scalar {
            value,
            style,
            anchor,
            tag,
        } = &scalar.unwrap().event_type
        {
            assert_eq!(value, "hello");
            assert_eq!(style, &ScalarStyle::Plain);
            assert!(anchor.is_none());
            assert!(tag.is_none());
        }
    }

    #[test]
    fn test_stream_mapping() {
        let events = parse_stream("key: value", true).unwrap();
        let mapping_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .collect();
        assert_eq!(mapping_starts.len(), 1);
        let mapping_ends: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingEnd))
            .collect();
        assert_eq!(mapping_ends.len(), 1);
    }

    #[test]
    fn test_stream_sequence() {
        let events = parse_stream("- item1\n- item2", true).unwrap();
        let seq_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceStart { .. }))
            .collect();
        assert_eq!(seq_starts.len(), 1);
        let seq_ends: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceEnd))
            .collect();
        assert_eq!(seq_ends.len(), 1);
    }

    #[test]
    fn test_stream_scalar_with_anchor() {
        let events = parse_stream("key: &anchor value", true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar (with anchor) is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { anchor, .. } = &scalars[1].event_type {
            assert!(anchor.is_some());
            assert_eq!(anchor.as_deref(), Some("anchor"));
        }
    }

    #[test]
    fn test_stream_scalar_with_tag() {
        let events = parse_stream("key: !!str value", true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar (with tag) is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { tag, .. } = &scalars[1].event_type {
            assert!(tag.is_some());
            assert_eq!(tag.as_ref().unwrap().suffix, "str");
        }
    }

    #[test]
    fn test_stream_alias() {
        let events = parse_stream("a: &x 1\nb: *x", true).unwrap();
        let alias_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Alias { .. }))
            .collect();
        assert_eq!(alias_events.len(), 1);
        if let StreamEventType::Alias { name } = &alias_events[0].event_type {
            assert_eq!(name, "x");
        }
    }

    #[test]
    fn test_stream_standalone_comment() {
        let events = parse_stream("# standalone comment\nkey: value", true).unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment { text, standalone } = &comment_events[0].event_type {
            assert_eq!(text, "standalone comment");
            assert!(*standalone);
        }
    }

    #[test]
    fn test_stream_inline_comment() {
        let events = parse_stream("key: value  # inline comment", true).unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment { text, standalone } = &comment_events[0].event_type {
            assert_eq!(text, "inline comment");
            assert!(!*standalone);
        }
    }

    #[test]
    fn test_stream_empty_yaml() {
        let events = parse_stream("", true).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_multiple_documents() {
        let events = parse_stream("---\nkey1: val1\n---\nkey2: val2", true).unwrap();
        let doc_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::DocumentStart))
            .collect();
        assert_eq!(doc_starts.len(), 2);
    }

    #[test]
    fn test_stream_single_quoted_scalar() {
        let events = parse_stream("key: 'value'", true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::SingleQuoted);
        }
    }

    #[test]
    fn test_stream_double_quoted_scalar() {
        let events = parse_stream(r#"key: "value""#, true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { value, style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::DoubleQuoted);
            assert_eq!(value, "value");
        }
    }

    #[test]
    fn test_stream_literal_scalar() {
        let events = parse_stream("key: |\n  line1\n  line2", true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::Literal);
        }
    }

    #[test]
    fn test_stream_folded_scalar() {
        let events = parse_stream("key: >\n  line1\n  line2", true).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::Folded);
        }
    }

    #[test]
    fn test_stream_nested_mapping() {
        let events = parse_stream("a:\n  b: 1\n  c: 2", true).unwrap();
        let mapping_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .collect();
        assert_eq!(mapping_starts.len(), 2);
    }

    #[test]
    fn test_stream_nested_sequence() {
        let events = parse_stream("- - a\n  - b\n- c", true).unwrap();
        let seq_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceStart { .. }))
            .collect();
        assert_eq!(seq_starts.len(), 2);
    }

    #[test]
    fn test_stream_line_column() {
        let events = parse_stream("key: value", true).unwrap();
        for event in &events {
            assert!(event.line < 100);
        }
    }
}
