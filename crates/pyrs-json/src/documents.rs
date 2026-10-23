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
use pyrs_ast::error::ParseError;
use pyrs_event::{EventSink, LineTable, StreamEvent, StreamEventType};

use crate::{JsonParseOptions, parse_stream_with_options_to};

/// A line's `start..end` in the whole input, the `\n` excluded.
type Line = (usize, usize);

/// Every line of `text`, in order.
///
/// A final line break adds no empty line: `"{}\n"` is one line, `"{}"` is one line. A file that
/// ends in two breaks has a blank last line, which the walk refuses.
fn lines(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            out.push((start, index));
            start = index + 1;
        }
    }
    if start < text.len() {
        out.push((start, text.len()));
    }
    out
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
        return Err(ParseError::Syntax {
            message: format!(
                "a blank line carries no document in a line-delimited JSON stream at line {} column 1",
                index + 1
            ),
            line: index,
            col: 0,
        });
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
    use crate::{from_json5, parse_stream};
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

    #[cfg(feature = "parallel")]
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
}
