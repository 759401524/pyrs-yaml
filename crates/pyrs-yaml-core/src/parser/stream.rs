use crate::ast::{Comment, ScalarStyle, Tag};
use crate::parser::yaml::{
    extract_comments_and_anchors, scan_yaml, unescape_double_quoted, CommentAnchorTracker,
};
use saphyr_parser::{
    Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver,
};
use std::collections::HashMap;
use std::sync::Arc;

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
        text: Arc<str>,
        /// Whether this is a standalone line comment (`true`) or an inline
        /// comment at the end of a line (`false`).
        standalone: bool,
    },
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
    comment_anchor_tracker: CommentAnchorTracker,
    events: Vec<StreamEvent>,
    pending_standalone_comment: Option<Comment>,
    anchors: HashMap<usize, String>,
}

impl<'a> StreamReceiver<'a> {
    fn new(yaml_text: &'a str) -> Self {
        let scan = scan_yaml(yaml_text);
        let (comments, anchors) = if !scan.has_hash && !scan.has_amp {
            (Vec::new(), Vec::new())
        } else {
            extract_comments_and_anchors(yaml_text)
        };
        Self {
            yaml_text,
            line_offsets: scan.line_offsets,
            comment_anchor_tracker: CommentAnchorTracker::new(comments, anchors),
            events: Vec::new(),
            pending_standalone_comment: None,
            anchors: HashMap::new(),
        }
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

        // 注释注入逻辑保持逐分支：standalone 查找仅 Scalar/MappingStart/SequenceStart；
        // Scalar 分支在发现 standalone 时立即 emit pending（与现有实现逐字节一致）。
        let mut inline_comment = None;
        let mut value_end_col = 0;
        match &event {
            Event::Scalar(value, ..) => {
                let standalone = self
                    .comment_anchor_tracker
                    .find_standalone_before_line(line);
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                    self.emit_pending_comment(line, column);
                }

                value_end_col = if line < self.line_offsets.len() {
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

                inline_comment = self
                    .comment_anchor_tracker
                    .find_inline_comment(line, value_end_col);
            }
            Event::MappingStart(..) | Event::SequenceStart(..) => {
                let standalone = self
                    .comment_anchor_tracker
                    .find_standalone_before_line(line);
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                }
            }
            _ => {}
        }

        let Some(stream_event) =
            event_to_stream_event(event, span, &mut self.anchors, &mut |_id| {
                self.comment_anchor_tracker.next_anchor_name()
            })
        else {
            return; // Event::Nothing
        };

        // emit_pending_comment 时机：除 StreamStart 与 Scalar 外，其余事件在 push 前无条件 emit。
        // （Scalar 分支仅在发现 standalone 时 emit，见上方 match；StreamStart 不 emit。）
        let needs_emit_pending = !matches!(
            &stream_event.event_type,
            StreamEventType::StreamStart | StreamEventType::Scalar { .. }
        );
        if needs_emit_pending {
            self.emit_pending_comment(line, column);
        }

        self.events.push(stream_event);

        if let Some(inline_comment) = inline_comment {
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
}

/// Convert a saphyr-parser Tag to our Tag format.
fn convert_tag(tag: Option<&saphyr_parser::Tag>) -> Option<Tag> {
    tag.map(|t| super::convert_tag(t).clone())
}

/// 纯函数：saphyr `Event` + `Span` → `StreamEvent`（D4 抽取）。
///
/// 不含注释/`line_offsets`/`value_end_col` 等 O(doc) 依赖逻辑（留在
/// `StreamReceiver::on_event`）；anchor 名经 `resolve_anchor` 回调注入
/// （字符串路径：`comment_anchor_tracker.next_anchor_name()`；流式路径：
/// `format!("anchor_{id}")`），回调返回 `None` 表示无 anchor；解析到的
/// anchor 注册进 `anchor_map` 供 Alias 分支查找。`Event::Nothing` → `None`。
pub fn event_to_stream_event<F>(
    event: Event<'_>,
    span: Span,
    anchor_map: &mut HashMap<usize, String>,
    resolve_anchor: &mut F,
) -> Option<StreamEvent>
where
    F: FnMut(usize) -> Option<String>,
{
    let line = span.start.line().saturating_sub(1);
    let column = span.start.col();

    let event_type = match event {
        Event::Nothing => return None,
        Event::StreamStart => StreamEventType::StreamStart,
        Event::StreamEnd => StreamEventType::StreamEnd,
        Event::DocumentStart(_) => StreamEventType::DocumentStart,
        Event::DocumentEnd => StreamEventType::DocumentEnd,
        Event::Scalar(value, style, anchor_id, tag) => {
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
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::Scalar {
                value: scalar_value,
                style: scalar_style,
                anchor,
                tag: tag_obj,
            }
        }
        Event::MappingStart(anchor_id, tag) => {
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::MappingStart {
                anchor,
                tag: tag_obj,
            }
        }
        Event::MappingEnd => StreamEventType::MappingEnd,
        Event::SequenceStart(anchor_id, tag) => {
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::SequenceStart {
                anchor,
                tag: tag_obj,
            }
        }
        Event::SequenceEnd => StreamEventType::SequenceEnd,
        Event::Alias(anchor_id) => {
            let name = anchor_map
                .get(&anchor_id)
                .cloned()
                .unwrap_or_else(|| format!("alias_{}", anchor_id));
            StreamEventType::Alias { name }
        }
    };

    Some(StreamEvent {
        event_type,
        line,
        column,
    })
}

/// Resolve and register an anchor name for a non-zero anchor id.
///
/// `resolve_anchor` 返回 `None` 时表示无 anchor（名称来源耗尽），此时不注册，
/// 与字符串路径 `next_anchor_name()` 的语义一致。
fn resolve_anchor_name<F>(
    anchor_id: usize,
    anchor_map: &mut HashMap<usize, String>,
    resolve_anchor: &mut F,
) -> Option<String>
where
    F: FnMut(usize) -> Option<String>,
{
    if anchor_id == 0 {
        return None;
    }
    match resolve_anchor(anchor_id) {
        Some(name) => {
            anchor_map.insert(anchor_id, name.clone());
            Some(name)
        }
        None => None,
    }
}

/// Parse a YAML string into a stream of `StreamEvent`s.
///
/// Consumes the YAML string and returns a vector of streaming events
/// that represent every structural element in the document.
///
/// # Arguments
/// * `yaml` - The YAML content string.
///
/// # Returns
/// Success: a `Vec<StreamEvent>` representing the complete event stream.
/// Failure: a `ParseErrorDetail` with line/column information.
///
/// # Examples
/// ```rust
/// use pyrs_yaml_core::parser::stream::parse_stream;
/// let events = parse_stream("key: value").unwrap();
/// assert!(!events.is_empty());
/// ```
pub fn parse_stream(yaml: &str) -> Result<Vec<StreamEvent>, super::ParseErrorDetail> {
    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut receiver = StreamReceiver::new(yaml);
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| super::ParseErrorDetail {
            message: format!("YAML parse error: {}", e),
            line: 0,
            col: 0,
        })?;

    Ok(receiver.events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_start_and_end() {
        let events = parse_stream("hello").unwrap();
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
        let events = parse_stream("hello").unwrap();
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
        let events = parse_stream("key: value").unwrap();
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
        let events = parse_stream("- item1\n- item2").unwrap();
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
        let events = parse_stream("key: &anchor value").unwrap();
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
        let events = parse_stream("key: !!str value").unwrap();
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
        let events = parse_stream("a: &x 1\nb: *x").unwrap();
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
        let events = parse_stream("# standalone comment\nkey: value").unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment { text, standalone } = &comment_events[0].event_type {
            assert_eq!(text.as_ref(), "standalone comment");
            assert!(*standalone);
        }
    }

    #[test]
    fn test_stream_inline_comment() {
        let events = parse_stream("key: value  # inline comment").unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment { text, standalone } = &comment_events[0].event_type {
            assert_eq!(text.as_ref(), "inline comment");
            assert!(!*standalone);
        }
    }

    #[test]
    fn test_stream_empty_yaml() {
        let events = parse_stream("").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_multiple_documents() {
        let events = parse_stream("---\nkey1: val1\n---\nkey2: val2").unwrap();
        let doc_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::DocumentStart))
            .collect();
        assert_eq!(doc_starts.len(), 2);
    }

    #[test]
    fn test_stream_single_quoted_scalar() {
        let events = parse_stream("key: 'value'").unwrap();
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
        let events = parse_stream(r#"key: "value""#).unwrap();
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
        let events = parse_stream("key: |\n  line1\n  line2").unwrap();
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
        let events = parse_stream("key: >\n  line1\n  line2").unwrap();
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
        let events = parse_stream("a:\n  b: 1\n  c: 2").unwrap();
        let mapping_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .collect();
        assert_eq!(mapping_starts.len(), 2);
    }

    #[test]
    fn test_stream_nested_sequence() {
        let events = parse_stream("- - a\n  - b\n- c").unwrap();
        let seq_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceStart { .. }))
            .collect();
        assert_eq!(seq_starts.len(), 2);
    }

    #[test]
    fn test_stream_line_column() {
        let events = parse_stream("key: value").unwrap();
        for event in &events {
            assert!(event.line < 100);
        }
    }

    #[test]
    fn spike_new_from_iter_matches_str() {
        // 项 1：new_from_iter 与 new_from_str 事件序列一致（值/样式/line/col 逐项相等）
        let mut p1 = SaphyrParser::new_from_str("a: 1\n");
        let mut p2 = SaphyrParser::new_from_iter("a: 1\n".chars());
        loop {
            match (p1.next_event(), p2.next_event()) {
                (None, None) => break,
                (Some(Ok(e1)), Some(Ok(e2))) => assert_eq!(e1, e2),
                _ => panic!("spike 1: 序列不一致 (streams diverged)"),
            }
        }
    }

    #[test]
    fn spike_alias_event_carries_only_id() {
        // 项 1 附加：Event::Alias(usize) 只带 id（设计 A'''' 验证）
        let mut p = SaphyrParser::new_from_iter("a: &x 1\nb: *x\n".chars());
        let mut alias_ids = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            if let Event::Alias(id) = event {
                alias_ids.push(id);
            }
        }
        assert_eq!(alias_ids.len(), 1);
        assert_ne!(alias_ids[0], 0);
    }

    #[test]
    fn spike_eof_returns_none() {
        // 项 2：EOF 后 next_event() → None（stream_end_emitted flag）
        let mut p = SaphyrParser::new_from_iter("a: 1\n".chars());
        let mut count = 0;
        while p.next_event().is_some() {
            count += 1;
        }
        assert!(count > 2);
        assert!(p.next_event().is_none());
        assert!(p.next_event().is_none());
    }

    #[test]
    fn spike_multidoc_sequence() {
        // 项 2a：多文档 `--- a\n--- b\n` 事件序列 = StreamStart→DocStart→…→DocEnd→…→StreamEnd
        let mut p = SaphyrParser::new_from_iter("--- a\n--- b\n".chars());
        let mut types = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            types.push(match event {
                Event::StreamStart => "SS",
                Event::StreamEnd => "SE",
                Event::DocumentStart(_) => "DS",
                Event::DocumentEnd => "DE",
                Event::Scalar(..) => "SC",
                _ => "?",
            });
        }
        assert_eq!(types, ["SS", "DS", "SC", "DE", "DS", "SC", "DE", "SE"]);
    }

    #[test]
    fn spike_empty_input_yields_stream_start_end() {
        // 项 2b：空输入 → [StreamStart, StreamEnd]（与字符串 parse_stream 的 [] 不同，架构性差异）
        let mut p = SaphyrParser::new_from_iter("".chars());
        let mut types = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            types.push(match event {
                Event::StreamStart => "SS",
                Event::StreamEnd => "SE",
                other => panic!("unexpected: {:?}", other),
            });
        }
        assert_eq!(types, ["SS", "SE"]);
    }

    #[test]
    fn spike_crlf_line_col() {
        // 项 4：CRLF 输入 line/col 正确（skip_linebreak → 单次 line 增量，scanner.rs:619-625）。
        // 实测结论（spike）：key 与 value 都是 scalar 事件——key 在 col 0，value 在 col 3；
        // line 1-indexed；每条 CRLF 使 line 仅 +1。
        let mut p = SaphyrParser::new_from_iter("a: 1\r\nb: 2\r\n".chars());
        let mut cols = Vec::new();
        while let Some(Ok((event, span))) = p.next_event() {
            if matches!(event, Event::Scalar(..)) {
                cols.push((span.start.line(), span.start.col()));
            }
        }
        assert_eq!(cols, [(1, 0), (1, 3), (2, 0), (2, 3)]);
    }

    fn mk_span(line: usize, col: usize) -> Span {
        Span::new(
            saphyr_parser::Marker::new(0, line, col),
            saphyr_parser::Marker::new(0, line, col),
        )
    }

    #[test]
    fn event_to_stream_event_scalar_double_quoted_unescapes() {
        let mut anchor_map = HashMap::new();
        let event = Event::Scalar("a\\n\\t".into(), SaphyrScalarStyle::DoubleQuoted, 0, None);
        let ev = super::event_to_stream_event(event, mk_span(1, 0), &mut anchor_map, &mut |_| None)
            .expect("scalar maps to Some");
        match ev.event_type {
            super::StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => {
                assert_eq!(value, "a\n\t");
                assert_eq!(style, ScalarStyle::DoubleQuoted);
                assert_eq!(anchor, None);
                assert_eq!(tag, None);
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn event_to_stream_event_plain_scalar_passthrough() {
        let mut anchor_map = HashMap::new();
        let event = Event::Scalar("hello".into(), SaphyrScalarStyle::Plain, 0, None);
        let ev = super::event_to_stream_event(event, mk_span(2, 4), &mut anchor_map, &mut |_| None)
            .unwrap();
        match ev.event_type {
            super::StreamEventType::Scalar { value, style, .. } => {
                assert_eq!(value, "hello");
                assert_eq!(style, ScalarStyle::Plain);
            }
            other => panic!("wrong variant: {:?}", other),
        }
        assert_eq!((ev.line, ev.column), (1, 4)); // line = span.line - 1
    }

    #[test]
    fn event_to_stream_event_anchor_resolves_and_registers() {
        let mut anchor_map = HashMap::new();
        let event = Event::Scalar("v".into(), SaphyrScalarStyle::Plain, 7, None);
        let ev = super::event_to_stream_event(event, mk_span(1, 0), &mut anchor_map, &mut |id| {
            Some(format!("anchor_{}", id))
        })
        .unwrap();
        match ev.event_type {
            super::StreamEventType::Scalar {
                anchor: Some(name), ..
            } => {
                assert_eq!(name, "anchor_7");
                assert_eq!(anchor_map.get(&7), Some(&"anchor_7".to_string()));
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn event_to_stream_event_alias_lookup_and_fallback() {
        let mut anchor_map = HashMap::new();
        let ev = super::event_to_stream_event(
            Event::Alias(3),
            mk_span(1, 0),
            &mut anchor_map,
            &mut |_| None,
        )
        .unwrap();
        assert!(
            matches!(ev.event_type, super::StreamEventType::Alias { name } if name == "alias_3")
        );

        let mut anchor_map = HashMap::new();
        anchor_map.insert(3, "real".to_string());
        let ev = super::event_to_stream_event(
            Event::Alias(3),
            mk_span(1, 0),
            &mut anchor_map,
            &mut |_| None,
        )
        .unwrap();
        assert!(matches!(ev.event_type, super::StreamEventType::Alias { name } if name == "real"));
    }

    #[test]
    fn event_to_stream_event_nothing_is_none() {
        let mut anchor_map = HashMap::new();
        assert!(super::event_to_stream_event(
            Event::Nothing,
            mk_span(1, 0),
            &mut anchor_map,
            &mut |_| None
        )
        .is_none());
    }
}
