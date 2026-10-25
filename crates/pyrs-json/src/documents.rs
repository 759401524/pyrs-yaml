//! Line-delimited JSON: one complete document per line.
//!
//! This is the shape JSON-lines producers write (`NDJSON`, `JSON Lines`, `.jsonl`). The single
//! document walk in [`parse_stream_with_options_to`] already knows how to describe one document;
//! this module runs it once per line and reports every position against the whole input, so an
//! event and the error for the same byte name the same place.
//!
//! The contract, in the order a reader needs it:
//!
//! - Lines break at `\n`. A `\r` before it is whitespace to the JSON grammar, so a CRLF file reads
//!   the same as an LF one.
//! - Each line holds exactly one complete document. A blank line, a line holding only a note, or
//!   a value that needs the next line to finish (an object left open, a JSON5 backslash
//!   continuation inside a string), is refused rather than skipped: a record this format does not
//!   define is not a record to lose silently.
//! - The stream keeps the shape the YAML engine uses for several documents: one `StreamStart`,
//!   then `DocumentStart` … `DocumentEnd` per line, then one `StreamEnd`.
//! - The dialect chosen by [`JsonParseOptions`] applies to every line, so JSONC notes and JSON5
//!   spellings are accepted and reported per line, positions included.
//!
//! An empty input is an empty stream: the two markers and nothing between them.
//!
//! Behind the opt-in `parallel` feature the same lines are divided into consecutive shards, one
//! sink each, and every thread runs the serial walk over its own shard. Nothing is buffered between
//! the producer and its sink: an event is handed over on the thread that built it, with positions
//! against the whole input, so a consumer that wants one order merges the shards by offset.

use alloc::vec::Vec;
use core::ops::ControlFlow;
use pyrs_ast::ast::CustomNode;
use pyrs_ast::error::ParseError;
use pyrs_event::{EventSink, LineTable, StreamEvent, StreamEventType};

use crate::{JsonParseOptions, from_json_with_options, parse_stream_with_options_to};

/// A line's `start..end` in the whole input, the `\n` excluded.
pub type Line = (usize, usize);

/// Every line's `start..end` in the whole input, in order.
///
/// The collecting form of [`Lines`], for the walks that need the index at once: `parse_documents_with_options_to`
/// shifts each line's events by its start, and the shard split in
/// `parse_documents_with_options_to_parallel` ends a shard where the next one starts.
fn lines(text: &str) -> Vec<Line> {
    Lines::new(text).map(|(_, line)| line).collect()
}

/// One line of a line-delimited stream at a time: its 0-indexed number and its `start..end`.
///
/// Offsets rather than the line's text, because a consumer that owns its text and moves its own
/// cursor between items — a `#[pyclass]`, which cannot borrow the string it holds — cannot keep a
/// slice into it. Two integers survive the borrow ending, and the line they name always ends before
/// the break: a final break adds no empty line, so `"{}\n"` is one line, `"{}"` is one line, and a
/// file ending in two breaks has a blank last line for [`document_from_line`] to refuse.
pub struct Lines<'a> {
    text: &'a str,
    /// The first byte of the line not yet returned.
    at: usize,
    /// How many lines were returned before it.
    number: usize,
}

impl<'a> Lines<'a> {
    /// Every line of `text`, numbered from zero.
    pub fn new(text: &'a str) -> Self {
        Self::resume(text, 0, 0)
    }

    /// The lines of `text` from byte `at` on, numbered from `number`.
    ///
    /// A resumed walk is indistinguishable from one that never stopped: the two arguments are the
    /// last line's `end + 1` and its number plus one.
    pub fn resume(text: &'a str, at: usize, number: usize) -> Self {
        Lines { text, at, number }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = (usize, Line);

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.text.len() {
            return None;
        }
        let end = self.text[self.at..]
            .find('\n')
            .map_or(self.text.len(), |index| self.at + index);
        let item = (self.number, (self.at, end));
        // One past the break, which is the end of the input itself when the line had none: the
        // guard above then reads the stream as finished rather than slicing past the last byte.
        self.at = end + 1;
        self.number += 1;
        Some(item)
    }
}

/// Whether a line holds no token at all.
///
/// The set is JSON's own whitespace: a line of JSON5's wider whitespace is content, and the walk
/// refuses it with the diagnostic that names the missing value rather than calling it blank.
fn is_blank(line: &str) -> bool {
    line.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c))
}

/// The sink one line writes through: it drops the line's own stream markers and moves every other
/// event onto the whole input.
///
/// The column needs no correction: a line starts at the beginning of its own line in the whole
/// text, so characters counted from either start are the same characters.
struct LineSink<'s, S: ?Sized> {
    out: &'s mut S,
    /// The line's first byte in the whole input.
    base: usize,
    /// The line's 0-indexed number in the whole input.
    line: usize,
    stopped: bool,
}

impl<'a, S: EventSink<'a> + ?Sized> EventSink<'a> for LineSink<'_, S> {
    fn event(&mut self, mut event: StreamEvent<'a>) -> ControlFlow<()> {
        if matches!(
            &event.event_type,
            StreamEventType::StreamStart | StreamEventType::StreamEnd
        ) {
            return ControlFlow::Continue(());
        }
        event.offset += self.base;
        event.end += self.base;
        event.line += self.line;
        let answer = self.out.event(event);
        self.stopped |= answer.is_break();
        answer
    }
}

/// The sink one shard writes through: it moves every event, the shard's own stream markers
/// included, onto the whole input.
///
/// A shard is a complete document stream over a run of whole lines, so unlike [`LineSink`] it
/// keeps the markers: each sink sees the shape it would see from the serial entry point applied
/// to its own lines. The column needs no correction, for the same reason as [`LineSink`].
#[cfg(feature = "parallel")]
struct ShardSink<'s, S: ?Sized> {
    out: &'s mut S,
    /// The shard's first byte in the whole input.
    base: usize,
    /// The shard's 0-indexed first line in the whole input.
    line: usize,
}

#[cfg(feature = "parallel")]
impl<'a, S: EventSink<'a> + ?Sized> EventSink<'a> for ShardSink<'_, S> {
    fn event(&mut self, mut event: StreamEvent<'a>) -> ControlFlow<()> {
        event.offset += self.base;
        event.end += self.base;
        event.line += self.line;
        self.out.event(event)
    }
}

/// Move an error's coordinates down by `base` lines.
///
/// The position the parser embeds in its message is the line that error was found on, counted
/// from one, so a refusal on a later line rewrites that one fragment as well as the field. Every
/// syntax error this walk can report writes its position in that one form: the parser's `err` and
/// the blank-line refusal below.
fn shift_error(error: ParseError, base: usize) -> ParseError {
    let ParseError::Syntax { message, line, col } = error else {
        return error;
    };
    if base == 0 {
        return ParseError::Syntax { message, line, col };
    }
    ParseError::Syntax {
        message: message.replacen(
            &format!(" at line {} column ", line + 1),
            &format!(" at line {} column ", base + line + 1),
            1,
        ),
        line: base + line,
        col,
    }
}

/// The refusal a line with no token at all earns, named by its place in the stream.
fn blank_refusal(index: usize) -> ParseError {
    ParseError::Syntax {
        message: format!(
            "a blank line carries no document in a line-delimited JSON stream at line {} column 1",
            index + 1
        ),
        line: index,
        col: 0,
    }
}

/// The document one line of a line-delimited stream holds, with positions against the stream.
///
/// The value surface of [`parse_documents_with_options`] and the value sibling of [`parse_line`],
/// so the two cannot disagree about a line: the same blank-line refusal, the same [`shift_error`]
/// on the parser's own error, and underneath it the single-document reader
/// [`from_json_with_options`] — the tree the event walk replays into is the tree this returns,
/// which is what `crates/pyrs-json/tests/events_replay.rs` already proves for every document the
/// scanner accepts.
///
/// A consumer that can resolve a line itself — the binding builds Python objects straight from the
/// bytes of a canonical JSON line — hands it here only when its own route declines, so one line is
/// never read twice into two different values.
pub fn document_from_line(
    text: &str,
    (start, end): Line,
    index: usize,
    opts: JsonParseOptions,
) -> Result<CustomNode, ParseError> {
    let line = &text[start..end];
    if is_blank(line) {
        return Err(blank_refusal(index));
    }
    from_json_with_options(line, opts).map_err(|error| shift_error(error, index))
}

/// Describe one line as the document it must be, reporting through `sink`.
///
/// Returns whether the sink asked to stop. The line's own `StreamStart`/`StreamEnd` never reach
/// the sink: the stream has one of each for the whole input.
fn parse_line<'a, S: EventSink<'a> + ?Sized>(
    text: &'a str,
    (start, end): Line,
    index: usize,
    opts: JsonParseOptions,
    sink: &mut S,
) -> Result<bool, ParseError> {
    let line = &text[start..end];
    if is_blank(line) {
        return Err(blank_refusal(index));
    }
    let mut shifted = LineSink {
        out: sink,
        base: start,
        line: index,
        stopped: false,
    };
    parse_stream_with_options_to(line, opts, &mut shifted)
        .map_err(|error| shift_error(error, index))?;
    Ok(shifted.stopped)
}

fn start_event<'a>() -> StreamEvent<'a> {
    StreamEvent {
        event_type: StreamEventType::StreamStart,
        line: 0,
        column: 0,
        offset: 0,
        end: 0,
    }
}

fn end_event(text: &str) -> StreamEvent<'_> {
    let mut table = LineTable::new(text);
    let (line, column) = table.line_col(text.len());
    StreamEvent {
        event_type: StreamEventType::StreamEnd,
        line,
        column,
        offset: text.len(),
        end: text.len(),
    }
}

/// Describe a line-delimited JSON stream as events, with the strict RFC 8259 dialect.
pub fn parse_documents(text: &str) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    parse_documents_with_options(text, JsonParseOptions::STRICT)
}

/// Describe a line-delimited JSON stream as events, with the dialect chosen by `opts`.
pub fn parse_documents_with_options(
    text: &str,
    opts: JsonParseOptions,
) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    let mut events = Vec::new();
    parse_documents_with_options_to(text, opts, &mut events)?;
    Ok(events)
}

/// Describe a line-delimited JSON stream as events, handing each one to `sink`.
///
/// The walk is one document per line, in source order, and the sink may stop it with `Break` at
/// any event: the remaining lines are never scanned, and neither `DocumentEnd` nor `StreamEnd` is
/// reported after the break. A sink that consumes everything sees exactly the events the
/// collecting entry point returns.
pub fn parse_documents_with_options_to<'a, S: EventSink<'a> + ?Sized>(
    text: &'a str,
    opts: JsonParseOptions,
    sink: &mut S,
) -> Result<(), ParseError> {
    let lines = lines(text);
    if sink.event(start_event()).is_break() {
        return Ok(());
    }
    for (index, &line) in lines.iter().enumerate() {
        if parse_line(text, line, index, opts, sink)? {
            return Ok(());
        }
    }
    let _ = sink.event(end_event(text));
    Ok(())
}

/// Describe a line-delimited JSON stream as events, one sink per shard of consecutive lines.
///
/// The lines are divided into `sinks.len()` consecutive runs of about equal size and each run is
/// handed to one thread with one sink. A sink that owns at least one line receives a complete
/// stream over that run — its own `StreamStart`, the run's documents, its own `StreamEnd` — with
/// every position measured against the whole input, so shards merge by `offset` or `line` without
/// correction. A sink that owns no line is not opened.
///
/// Nothing is buffered between a producer and its sink: each thread runs exactly the one-line walk
/// the serial entry point runs, and every event is handed over on the thread that built it. That
/// is the point of this shape — ordering is per shard, not global, and a consumer that needs one
/// order pays for the merge itself.
///
/// `Break` stops the shard whose sink asked for it and no other. The first refusal in source order
/// is the one returned, whichever shard met it first; every shard still runs, because its sink
/// owns work no other shard can skip. One sink is the serial walk, and an input with no line is
/// handed to the first sink as the empty stream.
///
/// Each sink is written by its own thread. When a sink is no larger than a cache line — a bare
/// counter, say — pad it rather than packing several into one array: neighbouring counters on the
/// same line turn every event into a coherence transfer (measured on the 24-thread probe: 18.9 ms
/// padded against 46.5 ms packed for the same 20.5 MiB corpus).
#[cfg(feature = "parallel")]
pub fn parse_documents_with_options_to_parallel<'a, S: EventSink<'a> + Send>(
    text: &'a str,
    opts: JsonParseOptions,
    sinks: &mut [S],
) -> Result<(), ParseError> {
    let lines = lines(text);
    if sinks.is_empty() {
        return Ok(());
    }
    if sinks.len() == 1 || lines.is_empty() {
        return parse_documents_with_options_to(text, opts, &mut sinks[0]);
    }
    // A shard ends where the next one starts, so the slice holds whole lines and any break they
    // carry; the last shard ends with the input.
    let per = lines.len().div_ceil(sinks.len());
    let mut outcomes = Vec::with_capacity(sinks.len());
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (index, sink) in sinks.iter_mut().enumerate() {
            let first = index * per;
            if first >= lines.len() {
                break;
            }
            let stop = (first + per).min(lines.len());
            let start = lines[first].0;
            let end = lines.get(stop).map_or(text.len(), |&(next, _)| next);
            let shard = &text[start..end];
            handles.push(scope.spawn(move || {
                let mut out = ShardSink {
                    out: sink,
                    base: start,
                    line: first,
                };
                parse_documents_with_options_to(shard, opts, &mut out)
                    .map_err(|error| shift_error(error, first))
            }));
        }
        for handle in handles {
            outcomes.push(match handle.join() {
                Ok(outcome) => outcome,
                Err(payload) => std::panic::resume_unwind(payload),
            });
        }
    });
    for outcome in outcomes {
        outcome?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_events, from_json5, parse_stream, parse_stream_with_options};
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    fn document_starts(events: &[StreamEvent<'_>]) -> Vec<(usize, usize, usize)> {
        events
            .iter()
            .filter(|event| event.event_type == StreamEventType::DocumentStart)
            .map(|event| (event.line, event.column, event.offset))
            .collect()
    }

    fn document_ends(events: &[StreamEvent<'_>]) -> Vec<(usize, usize, usize)> {
        events
            .iter()
            .filter(|event| event.event_type == StreamEventType::DocumentEnd)
            .map(|event| (event.line, event.column, event.offset))
            .collect()
    }

    /// A sink that keeps the first `stop_at` events and then asks the producer to stop.
    struct Prefix<'a> {
        events: Vec<StreamEvent<'a>>,
        stop_at: usize,
    }

    impl<'a> EventSink<'a> for Prefix<'a> {
        fn event(&mut self, event: StreamEvent<'a>) -> ControlFlow<()> {
            if self.events.len() == self.stop_at {
                return ControlFlow::Break(());
            }
            self.events.push(event);
            ControlFlow::Continue(())
        }
    }

    #[test]
    fn one_line_is_the_single_document_stream() {
        let doc = r#"{"a":[1,2]}"#;
        let single = parse_stream(doc).unwrap();
        let multi = parse_documents(doc).unwrap();
        assert_eq!(multi.first(), single.first());
        assert_eq!(multi.last(), single.last());
        assert_eq!(
            &multi[1..multi.len() - 1],
            &single[1..single.len() - 1],
            "one line must describe exactly the document the single-document walk describes"
        );
    }

    #[test]
    fn positions_are_global_and_columns_count_characters() {
        let text = "[1]\n{\"k\":\"值\"}\n";
        let events = parse_documents(text).unwrap();
        assert_eq!(document_starts(&events), vec![(0, 0, 0), (1, 0, 4)]);
        assert_eq!(document_ends(&events), vec![(0, 3, 3), (1, 9, 15)]);
        let last = events.last().unwrap();
        assert_eq!(last.event_type, StreamEventType::StreamEnd);
        assert_eq!((last.line, last.column, last.offset), (2, 0, 16));
        // The scalar event starts at the string's opening quote: character 5, byte 9. The `}` after
        // the multi-byte value is where column and byte offset diverge: character 9, byte 15.
        let scalar = events
            .iter()
            .find(|event| match &event.event_type {
                StreamEventType::Scalar { value, .. } => value.as_ref() == "值",
                _ => false,
            })
            .expect("the value scalar must be reported");
        assert_eq!((scalar.line, scalar.column, scalar.offset), (1, 5, 9));
    }

    #[test]
    fn a_crlf_file_reads_like_an_lf_one() {
        let text = "{\"a\":1}\r\n[2]\r\n";
        let events = parse_documents(text).unwrap();
        assert_eq!(document_starts(&events), vec![(0, 0, 0), (1, 0, 9)]);
        // `DocumentEnd` sits after the line's trailing whitespace, so the `\r` is included.
        assert_eq!(document_ends(&events), vec![(0, 8, 8), (1, 4, 13)]);
        let last = events.last().unwrap();
        assert_eq!((last.line, last.column, last.offset), (2, 0, 14));
    }

    #[test]
    fn a_blank_line_is_refused_and_names_its_line() {
        let error = parse_documents("{\"a\":1}\n \t\r\n{}\n").unwrap_err();
        let ParseError::Syntax { message, line, col } = error else {
            panic!("a blank line must be a syntax error");
        };
        assert_eq!((line, col), (1, 0));
        assert!(
            message.contains("line 2"),
            "the message must name the global line: {message}"
        );
    }

    #[test]
    fn a_document_that_needs_the_next_line_is_refused() {
        // Valid JSON5 as a whole document, but line one leaves its string open.
        let text = "\"a\\\nb\"\n";
        assert!(
            from_json5(text).is_ok(),
            "the whole input is one JSON5 string"
        );
        let error = parse_documents_with_options(text, JsonParseOptions::JSON5).unwrap_err();
        let ParseError::Syntax { line, .. } = error else {
            panic!("a spanning document must be refused line by line");
        };
        assert_eq!(
            line, 0,
            "the refusal belongs to the line that opened the string"
        );
    }

    #[test]
    fn the_dialect_applies_to_every_line() {
        let text = "{a:1}\n[1,2,]\n";
        assert!(parse_documents(text).is_err(), "strict reads neither line");
        let events = parse_documents_with_options(text, JsonParseOptions::JSON5).unwrap();
        assert_eq!(document_starts(&events).len(), 2);
        assert_eq!(document_ends(&events).len(), 2);
    }

    #[test]
    fn jsonc_notes_keep_global_positions() {
        let text = "{\"a\":1} // two\n{\"b\":2} // three\n";
        let events = parse_documents_with_options(text, JsonParseOptions::JSONC).unwrap();
        let notes: Vec<(String, usize, usize)> = events
            .iter()
            .filter_map(|event| match &event.event_type {
                StreamEventType::Comment { text, .. } => {
                    Some((text.to_string(), event.line, event.offset))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            notes,
            vec![("two".to_string(), 0, 8), ("three".to_string(), 1, 23)]
        );
        assert_eq!(document_starts(&events), vec![(0, 0, 0), (1, 0, 15)]);

        // A line holding only a note is not a document, in a dialect that accepts notes: the
        // line-delimited contract is one complete document per line, so it is refused rather than
        // attached to a document it never named.
        assert!(
            parse_documents_with_options("// only a note\n{}\n", JsonParseOptions::JSONC).is_err()
        );
    }

    #[test]
    fn an_empty_input_is_an_empty_stream() {
        let events = parse_documents("").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], start_event());
        assert_eq!(events[1], end_event(""));
    }

    #[test]
    fn a_sink_that_breaks_sees_the_serial_prefix_and_no_stream_end() {
        let text = "{\"a\":1}\n[2]\n{\"b\":3}\n";
        let mut sink = Prefix {
            events: Vec::new(),
            stop_at: 10,
        };
        parse_documents_with_options_to(text, JsonParseOptions::STRICT, &mut sink).unwrap();
        assert_eq!(sink.events.len(), 10);
        assert!(
            !sink
                .events
                .iter()
                .any(|event| event.event_type == StreamEventType::StreamEnd),
            "a stopped walk reports no StreamEnd"
        );
    }

    /// A line-delimited corpus with multi-byte values, so a walk that counts bytes and one that
    /// counts characters cannot both get away with the same number.
    fn corpus(count: usize) -> String {
        (0..count)
            .map(|index| format!("{{\"i\":{index},\"s\":\"名字{index}\",\"v\":[{index}]}}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[cfg(feature = "parallel")]
    fn shards<'a>(text: &'a str, count: usize) -> Vec<Vec<StreamEvent<'a>>> {
        let mut sinks: Vec<Vec<StreamEvent<'a>>> = (0..count).map(|_| Vec::new()).collect();
        parse_documents_with_options_to_parallel(text, JsonParseOptions::STRICT, &mut sinks)
            .unwrap();
        sinks
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_shards_concatenate_to_the_serial_stream() {
        let text = corpus(400);
        let expected = parse_documents(&text).unwrap();
        for count in 1..=8 {
            let sinks = shards(&text, count);
            let mut documents = Vec::new();
            for (index, events) in sinks.iter().enumerate() {
                assert!(
                    !events.is_empty(),
                    "shard {index} of {count} owns lines and must be opened"
                );
                assert_eq!(
                    events.first().unwrap().event_type,
                    StreamEventType::StreamStart
                );
                assert_eq!(
                    events.last().unwrap().event_type,
                    StreamEventType::StreamEnd
                );
                documents.extend_from_slice(&events[1..events.len() - 1]);
            }
            assert_eq!(
                documents.as_slice(),
                &expected[1..expected.len() - 1],
                "{count} shards must describe the serial documents in source order"
            );
        }
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_reports_the_first_error_in_source_order_at_its_global_line() {
        let mut docs: Vec<String> = (0..200).map(|index| format!("[{index}]")).collect();
        docs[50] = "{\"a\":}".to_string();
        docs[150] = "{\"b\":}".to_string();
        let text = docs.join("\n");
        let mut sinks: Vec<Vec<StreamEvent>> = (0..8).map(|_| Vec::new()).collect();
        let error =
            parse_documents_with_options_to_parallel(&text, JsonParseOptions::STRICT, &mut sinks)
                .unwrap_err();
        let ParseError::Syntax { message, line, .. } = error else {
            panic!("the malformed line must be a syntax error");
        };
        assert_eq!(
            line, 50,
            "the earlier refusal wins regardless of shard order"
        );
        assert!(
            message.contains("line 51"),
            "the message must name the global line: {message}"
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_break_stops_only_the_shard_that_asked() {
        let text = corpus(400);
        let expected = parse_documents(&text).unwrap();
        let mut sinks: Vec<Prefix> = (0..4)
            .map(|index| Prefix {
                events: Vec::new(),
                stop_at: if index == 0 { 25 } else { usize::MAX },
            })
            .collect();
        parse_documents_with_options_to_parallel(&text, JsonParseOptions::STRICT, &mut sinks)
            .unwrap();
        assert_eq!(sinks[0].events.as_slice(), &expected[..25]);
        assert!(
            sinks[0]
                .events
                .iter()
                .all(|event| event.event_type != StreamEventType::StreamEnd),
            "the stopped shard reports no StreamEnd"
        );
        for sink in &sinks[1..] {
            assert_eq!(
                sink.events.last().unwrap().event_type,
                StreamEventType::StreamEnd,
                "a shard whose sink did not stop must finish"
            );
        }
        assert_eq!(
            sinks[3].events.last(),
            expected.last(),
            "the last shard ends where the whole input does"
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_refuses_a_blank_line_at_its_global_number() {
        let mut docs: Vec<String> = (0..120).map(|index| format!("[{index}]")).collect();
        docs[77] = String::new();
        let text = docs.join("\n");
        let mut sinks: Vec<Vec<StreamEvent>> = (0..4).map(|_| Vec::new()).collect();
        let error =
            parse_documents_with_options_to_parallel(&text, JsonParseOptions::STRICT, &mut sinks)
                .unwrap_err();
        let ParseError::Syntax { message, line, .. } = error else {
            panic!("the blank line must be a syntax error");
        };
        assert_eq!(line, 77);
        assert!(
            message.contains("line 78"),
            "the message must name the global line: {message}"
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_opens_only_the_shards_that_own_a_line() {
        let text = corpus(2);
        let sinks = shards(&text, 5);
        for opened in &sinks[..2] {
            assert_eq!(
                opened.first().unwrap().event_type,
                StreamEventType::StreamStart
            );
            assert_eq!(
                opened.last().unwrap().event_type,
                StreamEventType::StreamEnd
            );
        }
        assert!(
            sinks[2..].iter().all(Vec::is_empty),
            "a sink with no line is not opened"
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_hands_an_empty_input_to_the_first_sink() {
        let sinks = shards("", 3);
        assert_eq!(sinks[0], parse_documents("").unwrap());
        assert!(sinks[1..].iter().all(Vec::is_empty));
    }

    #[test]
    fn the_value_surface_resolves_each_line_the_event_surface_describes() {
        // The two ways this module answers a line-delimited stream: describe it as events and fold
        // them, or hand each line to the single-document reader. The Python value iterator takes the
        // second route and the registered contract is the first, so a line the stream describes has
        // to resolve to the tree that stream describes — same count, same trees, per line.
        let inputs = [
            ("lf", corpus(40)),
            ("crlf", corpus(40).replace('\n', "\r\n")),
            ("padding", " [1] \n\t{}\n{\"a\":2}".to_string()),
            ("no final break", "{\"a\":[1,2]}".to_string()),
            ("single line", "[1]".to_string()),
            ("empty", String::new()),
        ];
        for (name, text) in inputs {
            let events = parse_documents(&text).unwrap();
            let lines: Vec<(usize, Line)> = Lines::new(&text).collect();
            assert_eq!(
                lines.len(),
                document_starts(&events).len(),
                "{name}: the two surfaces count different documents"
            );
            for (number, line) in lines {
                let folded = from_events(
                    &parse_stream_with_options(&text[line.0..line.1], JsonParseOptions::STRICT)
                        .unwrap(),
                )
                .unwrap_or_else(|error| panic!("{name} line {number}: replays ({error})"));
                let value = document_from_line(&text, line, number, JsonParseOptions::STRICT)
                    .unwrap_or_else(|error| panic!("{name} line {number}: {error}"));
                assert_eq!(folded, value, "{name} line {number}: the surfaces disagree");
            }
        }
    }

    #[test]
    fn a_resumed_line_walk_reads_the_same_lines_as_one_that_never_stopped() {
        // A `#[pyclass]` cannot borrow the text it owns, so the Python iterator rebuilds the walk
        // between every value from the two numbers the last item named: that line's `end + 1` and
        // its own number plus one. Off by one and it re-reads a line or skips one, so values repeat
        // or vanish with no error to say so.
        let text = corpus(60) + "\n";
        let whole: Vec<(usize, Line)> = Lines::new(&text).collect();
        let mut resumed: Vec<(usize, Line)> = Vec::new();
        let (mut at, mut number) = (0, 0);
        while let Some(item) = Lines::resume(&text, at, number).next() {
            let (index, line) = item;
            assert_eq!(
                index, number,
                "a resumed walk must number from where it was pointed"
            );
            resumed.push(item);
            at = line.1 + 1;
            number += 1;
        }
        assert_eq!(resumed, whole);
        assert!(Lines::resume(&text, at, number).next().is_none());
        // An input with no final break leaves the same offset one past the end rather than at it,
        // and the walk reads that as finished instead of slicing past the last byte.
        let open = corpus(3);
        let (_, last) = Lines::new(&open).last().unwrap();
        assert!(Lines::resume(&open, last.1 + 1, 3).next().is_none());
    }

    #[test]
    fn the_value_surface_refuses_the_line_the_event_surface_refuses() {
        // Same refusal, same words, same line — the message is part of the contract because it is
        // what a caller sees when a record is malformed in the middle of a stream.
        let cases = [
            (
                "blank in the middle",
                JsonParseOptions::STRICT,
                "{\"a\":1}\n \t\r\n{}\n",
            ),
            ("trailing blank", JsonParseOptions::STRICT, "[1]\n\n"),
            (
                "parser error on a later line",
                JsonParseOptions::STRICT,
                "[1]\n[2]\n{oops}\n",
            ),
            ("document left open", JsonParseOptions::STRICT, "{\"a\":1\n"),
            (
                "string spanning the break",
                JsonParseOptions::JSON5,
                "\"a\\\nb\"\n",
            ),
            (
                "note and no value",
                JsonParseOptions::JSONC,
                "// only a note\n{}\n",
            ),
        ];
        for (name, opts, text) in cases {
            let described = parse_documents_with_options(text, opts)
                .err()
                .unwrap_or_else(|| panic!("{name}: the event surface must refuse it"));
            let resolved = Lines::new(text)
                .filter_map(|(number, line)| document_from_line(text, line, number, opts).err())
                .next()
                .unwrap_or_else(|| panic!("{name}: the value surface accepted every line"));
            let (
                ParseError::Syntax {
                    message: first,
                    line: first_line,
                    ..
                },
                ParseError::Syntax {
                    message: second,
                    line: second_line,
                    ..
                },
            ) = (described, resolved)
            else {
                panic!("{name}: both refusals must be syntax errors");
            };
            assert_eq!(
                first_line, second_line,
                "{name}: the two surfaces name different lines"
            );
            assert_eq!(
                first, second,
                "{name}: the two surfaces word the refusal differently"
            );
        }
    }
}
