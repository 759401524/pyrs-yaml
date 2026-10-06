use crate::ast::{Chomping, Comment, CustomNode, NodeMeta, ScalarStyle, Tag};
use crate::error::{DepthError, SerializeError};
use crate::parser::yaml::schema::core_type_is_non_string;
use indexmap::IndexMap;

/// 块标量头部的两项指示符，作为一组传递，避免在 `write_scalar` 及其下游
/// 逐层加参数。
#[derive(Debug, Clone, Copy)]
struct BlockScalarHeader<'a> {
    /// Chomping 指示符（`|` / `|-` / `|+`）。
    chomping: &'a Chomping,
    /// 显式缩进指示器（`|2` 中的 `2`）；`None` 表示重新探测。
    indent: Option<u8>,
    /// Inline (`# text`) tail comment for the block header line. granit only
    /// reads a block-scalar header comment on the header line itself — a
    /// comment written on the line *after* `|` is absorbed as block content
    /// (libFuzzer `yaml_roundtrip` crash-cfb3fa83), so it rides here and
    /// `write_block_header` places it before the newline.
    comment: Option<&'a str>,
}

/// Serialization options
#[derive(Debug, Clone, PartialEq)]
pub struct SerializeOptions {
    pub indent_size: usize,
    pub explicit_start: bool,
    pub explicit_end: bool,
    pub sort_keys: bool,
    pub max_depth: usize,
    pub width: usize,
    pub indent_mapping: usize,
    pub indent_sequence: usize,
    pub indent_offset: usize,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            explicit_start: false,
            explicit_end: false,
            sort_keys: false,
            max_depth: 1000,
            width: 80,
            indent_mapping: 2,
            indent_sequence: 2,
            indent_offset: 0,
        }
    }
}

/// Serialize a CustomNode AST back to YAML string
///
/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::serializer::to_yaml;
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let node = CustomNode::Mapping {
///     pairs,
///     flow_style: false,
///     meta: Default::default(),
/// };
/// let output = to_yaml(&node);
/// assert_eq!(output, "a: 1\n");
/// ```
pub fn to_yaml(node: &CustomNode) -> String {
    to_yaml_with_options(node, &SerializeOptions::default()).expect("serialization failed")
}

/// Serialize with custom options
///
/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::serializer::{to_yaml_with_options, SerializeOptions};
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let node = CustomNode::Mapping {
///     pairs,
///     flow_style: false,
///     meta: Default::default(),
/// };
/// let opts = SerializeOptions {
///     indent_size: 4,
///     explicit_start: true,
///     explicit_end: false,
///     sort_keys: false,
///     ..Default::default()
/// };
/// let output = to_yaml_with_options(&node, &opts).unwrap();
/// assert!(output.starts_with("---\n"));
/// ```
pub fn to_yaml_with_options(
    node: &CustomNode,
    options: &SerializeOptions,
) -> Result<String, SerializeError> {
    let mut serializer = Serializer::new(options);
    if options.explicit_start {
        serializer.output.push_str("---\n");
    }
    serializer.serialize_node_internal(
        node,
        options.indent_offset,
        options.indent_offset,
        false,
        0,
    )?;
    if options.explicit_end {
        serializer.output.push_str("...\n");
    }
    Ok(serializer.output)
}

struct Serializer {
    output: String,
    indent_size: usize,
    sort_keys: bool,
    /// Memoized indent strings by width (avoids `repeat()` on every call)
    indent_cache: Vec<String>,
    /// Indent step per block-mapping nesting level
    indent_mapping: usize,
    /// Indent step per block-sequence nesting level
    indent_sequence: usize,
    /// Maximum recursion depth before serialization fails
    max_depth: usize,
    /// Line width for wrapping (0 = no wrapping)
    width: usize,
    /// True while the line just finished can still take a trailing note.
    ///
    /// A YAML line has exactly one trailing slot, so a note may be appended to a
    /// line only when nothing has claimed it and the line is a value line rather
    /// than a note line, a block-scalar body line, or a wrapped continuation.
    /// Set where the line is written - the text alone cannot tell a block body
    /// line from a nested mapping line - and consulted only by the writer that
    /// needs the slot (a container's own inline note).
    trailing_note_slot_open: bool,
    /// Offset just after a tag that was written with no scalar text after it — the
    /// end of a "pending" line. A note line written next is reported by the reader as
    /// that value's leading note, so the note's owner would silently change and the
    /// document needs a second round to settle (libFuzzer `yaml_roundtrip`
    /// crash-c5b367d3, 22 bytes). `close_pending_tag_line` inserts the value's own null
    /// text at this offset before such a line is written, which keeps both the owner and
    /// the one-round fixed point. `None` unless the last-written scalar was tag-only.
    pending_tag_insert_at: Option<usize>,
}

/// Chomping actually written for a block scalar. A Clip-chomped value whose
/// content carries trailing blank lines cannot round-trip as Clip: granit's
/// Clip read strips every trailing line that ends with a newline, so the only
/// header form that re-reads to the same value at any position is Keep
/// (`+`) — libFuzzer `yaml_roundtrip` crash-c18cb1fd. The promotion is a
/// pure emit-side normalization: the AST keeps its parsed `Clip`.
///
/// `no_content_line` is the caller's already-computed `first_text_line.is_empty()`: the
/// body holds no line with text in it, so it is either empty or made of nothing but
/// line breaks. Both spellings lose the indentation indicator on re-read — measured,
/// `>+8\n\n` comes back as `Folded`/`Clip`/`block_indent: None` — and for an all-break
/// body the drift is worse than cosmetic, because the re-read `Clip` then strips the
/// only break the value consists of, so the *value* decays
/// (`">+8\r\r#" -> ">+8\n\n" -> ">\n\n" -> ""`). Sharing one computation of that flag
/// between `effective_chomping` and the indicator branch is a gate requirement, not
/// micro-optimisation: a standalone scan measured `+0.65%` on
/// `serialize_block_scalars` (deterministic, three runs identical) which, with the
/// ~1.45% WSL-to-runner drift the 2% tolerance was calibrated on, landed at `+2.11%`
/// and failed the committed instruction-count gate for duplicating work both writers
/// already do.
///
/// An **empty** body writes as Clip whatever the AST claims, for the same reason
/// the indentation indicator is dropped there: there is no content for the
/// indicator to act on, so granit reports the default chomping when it re-reads
/// the header. Writing `|+`/`>+` for an empty scalar therefore drifts to `|`/`>`
/// on the next round — libFuzzer `yaml_roundtrip` crash-89d81d99 (`>+8\r#`, where
/// the explicit indicator is exactly what makes the reader report `Keep` this once)
/// and crash-b5dcc38f (`ancho: |+`). Nothing is lost: an empty body has no trailing
/// break to keep or strip, so the three spellings denote the same value, and the
/// AST keeps whatever was parsed.
fn effective_chomping(value: &str, chomping: &Chomping, no_content_line: bool) -> Chomping {
    if value.is_empty() {
        return Chomping::Clip;
    }
    if matches!(chomping, Chomping::Clip) && (value.ends_with("\n\n") || no_content_line) {
        Chomping::Keep
    } else {
        *chomping
    }
}

/// Whether a node can be serialized inline on the same line as a mapping
/// key's colon (compact sequence item form): scalars, nulls, aliases, and
/// flow-style containers all end with a newline of their own.
fn inlineable_value(v: &CustomNode) -> bool {
    matches!(
        v,
        CustomNode::Scalar { .. }
            | CustomNode::Null { .. }
            | CustomNode::Alias { .. }
            | CustomNode::Mapping {
                flow_style: true,
                ..
            }
            | CustomNode::Sequence {
                flow_style: true,
                ..
            }
    )
}

/// Shared shape parameters for the mapping/sequence container skeleton
/// consumed by [`Serializer::write_container_node`].
#[derive(Clone, Copy)]
struct ContainerSkeleton<'a> {
    empty: bool,
    meta: &'a NodeMeta,
    flow_style: bool,
    indent_width: usize,
    in_value_context: bool,
    open: char,
    close: char,
    /// A stack of leading notes belonging to the container's first entry, lifted
    /// above the anchor/tag header line this skeleton writes — see
    /// [`first_entry_note`].
    preamble_note: Option<&'a [Comment]>,
}

/// Whether a written line already holds a comment marker.
///
/// A `#` opens a comment only outside quoted scalars and only when preceded by
/// the start of the line or whitespace; everywhere else it is ordinary text
/// (`"+#"`, `!-#`). Walking the line with quote state is what separates the two,
/// and the escape rules differ per quote style: `''` inside single quotes, a
/// backslash inside double quotes.
pub(crate) fn line_has_comment_marker(line: &[u8]) -> bool {
    let (mut single, mut double, mut after_space) = (false, false, true);
    let mut i = 0;
    while i < line.len() {
        let b = line[i];
        if single {
            if b == b'\'' {
                if line.get(i + 1) == Some(&b'\'') {
                    // `''` is an escaped quote, so it does not close the scalar.
                    i += 2;
                    continue;
                }
                single = false;
            }
        } else if double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                double = false;
            }
        } else {
            match b {
                b'\'' => single = true,
                b'"' => double = true,
                b'#' if after_space => return true,
                _ => {}
            }
        }
        after_space = b == b' ' || b == b'\t';
        i += 1;
    }
    false
}

/// Whether a block mapping can be emitted in the compact `- key: value`
/// sequence-item form: no metadata, non-empty, all keys simple scalars and
/// all values inlineable. Mirrors the guard used by `write_sequence_item`;
/// shared so the splice layer and the serializer can never drift.
pub fn is_compact_item(node: &CustomNode) -> bool {
    matches!(
        node,
        CustomNode::Mapping {
            pairs,
            meta: NodeMeta {
                comment: None,
                anchor: None,
                tag: None,
                ..
            },
            flow_style: false,
            ..
        } if !pairs.is_empty()
            && pairs.iter().all(|(k, v)| {
                !matches!(k, CustomNode::Mapping { .. } | CustomNode::Sequence { .. })
                    // A standalone note cannot share the `- key:` line: it
                    // would emit `# …` mid-line and strand the value.
                    && v.leading_comment().is_none()
                    && inlineable_value(v)
            })
    )
}

/// Serialize a single block-mapping pair via [`Serializer::write_mapping_pair`],
/// producing exactly the bytes a splice regeneration splices in.
pub fn pair_to_string(
    key: &CustomNode,
    value: &CustomNode,
    indent_width: usize,
    depth: usize,
) -> Result<String, SerializeError> {
    let mut s = Serializer::new(&SerializeOptions::default());
    s.write_mapping_pair(key, value, indent_width, depth)?;
    Ok(s.output)
}

/// Serialize a single block-sequence item via [`Serializer::write_sequence_item`].
pub fn item_to_string(
    item: &CustomNode,
    indent_width: usize,
    depth: usize,
) -> Result<String, SerializeError> {
    let mut s = Serializer::new(&SerializeOptions::default());
    s.write_sequence_item(item, indent_width, depth)?;
    Ok(s.output)
}

impl Serializer {
    fn new(options: &SerializeOptions) -> Self {
        let mut cache = Vec::with_capacity(128);
        cache.push(String::new()); // width 0 = empty
        Self {
            output: String::new(),
            indent_size: options.indent_size,
            sort_keys: options.sort_keys,
            indent_cache: cache,
            indent_mapping: options.indent_mapping,
            indent_sequence: options.indent_sequence,
            max_depth: options.max_depth,
            width: options.width,
            trailing_note_slot_open: false,
            pending_tag_insert_at: None,
        }
    }

    /// Ensure indent_cache has an entry for the given width, then write it to output.
    /// Does not return a reference to avoid overlapping borrows.
    fn write_indent(&mut self, width: usize) {
        if self.indent_cache.len() <= width {
            self.indent_cache.resize(width + 1, String::new());
        }
        if self.indent_cache[width].is_empty() {
            self.indent_cache[width] = " ".repeat(width);
        }
        self.output.push_str(&self.indent_cache[width]);
    }

    /// 写入锚点（`&name`）和标签（`!!type`）前缀。
    fn write_anchor_tag(&mut self, anchor: &Option<String>, tag: &Option<Tag>) {
        if anchor.is_none() && tag.is_none() {
            return;
        }
        if let Some(anchor_name) = anchor {
            // Emit the bare `&name ` token. Under the granit-aligned scanner
            // (`scan_anchor_name` = maximal `is_anchor_char` run) any name the
            // AST can hold re-scans to itself after the trailing space, so no
            // quoting is needed — the old quoted-emit branch was compensating
            // for the scanner's invented value-indicator-colon stripping.
            self.output.push('&');
            self.output.push_str(anchor_name);
            self.output.push(' ');
        }
        if let Some(t) = tag {
            self.output.push_str(&Self::yaml_tag_text(t));
            self.output.push(' ');
        }
    }

    /// Render a tag for YAML emission, percent-encoding characters that are not
    /// valid tag URI characters.
    ///
    /// granit hands the reader the *decoded* suffix, so the source tag `!y5%7c`
    /// arrives as the suffix `y5|`. Emitting that decoded text is not merely
    /// cosmetic: `|` is not a permitted tag character, so our own reader then
    /// rejects the output ("while scanning a tag, did not find expected
    /// whitespace or line break") and the round trip is not idempotent (libFuzzer
    /// `yaml_roundtrip` crash-b91536ce, 7 bytes `!y5%7c `). Re-encoding on write
    /// restores a readable spelling, and the encoding is deterministic
    /// so the second round emits the same bytes.
    ///
    /// The safe set depends on the *form* being emitted, because the reader uses
    /// two different predicates; see [`Self::TAG_SHORTHAND_SAFE`] and
    /// [`Self::TAG_VERBATIM_SAFE`].
    fn yaml_tag_text(tag: &Tag) -> String {
        // granit spells the bare `!` tag as an empty handle with `!` as its
        // suffix. Encoding that sentinel would yield `%21` — no leading `!`, so
        // not a tag token at all — hence it keeps its own spelling.
        if tag.suffix == "!" && tag.handle.is_empty() {
            return String::from("!");
        }
        // A verbatim `!<uri>` is scanned with `is_uri_char`, which tolerates flow
        // indicators inside the URI, so they are re-emitted as they were read:
        // `!<tag:yaml.org,2002:str>` must not degrade to a percent-spelled URI.
        if tag.handle.is_empty() {
            return format!(
                "!<{}>",
                Self::encode_tag_uri(&tag.suffix, Self::TAG_VERBATIM_SAFE)
            );
        }
        if tag.suffix.is_empty() && tag.handle == "!" {
            return String::from("!");
        }
        let suffix = Self::encode_tag_uri(&tag.suffix, Self::TAG_SHORTHAND_SAFE);
        format!("{}{suffix}", tag.handle)
    }

    /// The characters granit's shorthand suffix scan accepts: `is_tag_char` is
    /// `is_uri_char && !is_flow && c != '!'`. `,`, `[`, `]` and `!` *end* the scan,
    /// and at flow level 0 the scanner then requires a blank, a line break or EOF,
    /// so spelling them literally makes our own output unparseable
    /// (`InvalidTagTerminator`). That is reachable without a hand-built AST: the
    /// escape `!a%2cb` decodes to a suffix carrying a literal `,`, which the writer
    /// used to emit raw (libFuzzer `yaml_roundtrip` crash-e92ce66f, 43 bytes
    /// `!5yrrrrrrrrrrrrrrrrrr'rrrrrrrrrrrrrrrrr%2c `; the same root cause as
    /// crash-a1516147 `!5%2cy7 `). `%` is absent because it introduces an escape,
    /// `>`/`{`/`}` because they are not URI characters at all.
    ///
    /// Stored as a 128-entry membership table rather than a byte string: the scan
    /// is per byte of every tag on every serialize, and a table lookup beats a
    /// linear `slice::contains` (and folds the alphanumeric test into the same one
    /// lookup).
    const TAG_SHORTHAND_SAFE: &'static [bool; 128] = &safe_table(b"#$&'()*+/:;=?@-._~");

    /// The characters a verbatim `!<uri>` accepts: granit's `is_uri_char` — the
    /// shorthand set plus the flow indicators and `!`, all of which are legal
    /// inside `<...>` and decode to themselves.
    const TAG_VERBATIM_SAFE: &'static [bool; 128] = &safe_table(b"!#$&'()*+,;=:@/?[]-._~");

    /// Percent-encode everything outside `safe`, the character set the reader
    /// accepts for this tag form. `%` is always encoded, so a suffix carrying a
    /// literal percent re-reads as the same text instead of starting a fresh
    /// escape.
    fn encode_tag_uri(s: &str, safe: &[bool; 128]) -> String {
        if s.bytes().all(|b| lookup(safe, b)) {
            return s.to_string();
        }
        let mut out = String::with_capacity(s.len() + 8);
        for b in s.bytes() {
            if lookup(safe, b) {
                out.push(b as char);
            } else {
                // Spelled out rather than `format!("%{b:02x}")`: that allocated a
                // fresh String for *every* escaped byte, and tag suffixes are the
                // one place where escapes are the common case, not the exception.
                out.push('%');
                out.push(ASCII_HEX[(b >> 4) as usize]);
                out.push(ASCII_HEX[(b & 0x0f) as usize]);
            }
        }
        out
    }

    /// An empty container: a `Mapping`/`Sequence` with no entries/items,
    /// regardless of flow style (a block-style empty collection is serialized
    /// by its flow token `{}`/`[]`). Such a node cannot host a standalone
    /// comment on its own line in value position — YAML would interpret the
    /// bare `{}`/`[]` as a node needing a key or item — so standalone comments
    /// on empty containers are demoted to inline.
    fn is_empty_container(node: &CustomNode) -> bool {
        match node {
            CustomNode::Mapping { pairs, .. } => pairs.is_empty(),
            CustomNode::Sequence { items, .. } => items.is_empty(),
            _ => false,
        }
    }

    /// 写入一个空集合的行内注释（`  # text`）。
    ///
    /// Standalone notes on an empty container demote to inline (a bare
    /// `{}` / `[]` on its own line is invalid YAML), so a note's home is that
    /// single slot here. PR #117 keeps the semantics identical across both
    /// conventions: `standalone_slot()` normalises so a TOML-origin node's
    /// leading note lands on the same line as its `{}`.
    fn output_empty_node_comment(&mut self, meta: &NodeMeta) {
        if let Some(c) = empty_slot_notes(meta).0 {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
            self.trailing_note_slot_open = false;
        } else {
            self.trailing_note_slot_open = true;
        }
    }

    /// Close the last tag-only pending line by writing its value's own null text, so a
    /// note line about to follow lands where the AST has it. Only acts while that line
    /// is still the last one written: if more than its own terminator has been appended
    /// since, the pending line is no longer adjacent to this note and must be left alone.
    fn close_pending_tag_line(&mut self) {
        let Some(at) = self.pending_tag_insert_at.take() else {
            return;
        };
        if self.output[at..].matches('\n').count() > 1 {
            return;
        }
        // `write_anchor_tag` already leaves one space after the tag, so asking for a
        // second one here produced `!-  ~`, which a re-read normalises to `!- ~` — and a
        // fixed point has to be spelled the way the reader spells it (caught by the
        // note-survival gate, not by any of the placement tests).
        let text = if self.output[..at].ends_with(' ') {
            "~"
        } else {
            " ~"
        };
        self.output.insert_str(at, text);
    }

    /// Write a note as a line of its own. This is the single place that knows a note line
    /// is being emitted, so it closes a pending tag-only line first: `k: !-` followed by
    /// `# n` re-reads with the note inside the value (the scanner still has no scalar
    /// text for that value), which both changes the note's owner and costs a second
    /// round; writing the value's own null text first — `k: !- ~` — keeps the note where
    /// the AST has it and settles at once (crash-c5b367d3).
    fn write_note_line(&mut self, indent_width: usize, text: &str) {
        self.close_pending_tag_line();
        self.write_indent(indent_width);
        self.output.push_str("# ");
        self.output.push_str(text);
        self.output.push('\n');
        // A note line has used its trailing slot, and a following note cannot
        // share it.
        self.trailing_note_slot_open = false;
    }

    /// True when the line just finished already carries a comment marker.
    ///
    /// The writer tracks slot ownership itself, so this only catches the shape it
    /// cannot distinguish by state: a `#` that sits on the line as *text*. Raw
    /// byte scanning read every `#` as a marker, so a quoted scalar holding one
    /// (`"+#": !-`) looked already commented; the refusal then demoted the note to
    /// a line of its own, and the reader hands such a line to the following node,
    /// so the emission settled only on its second round (libFuzzer
    /// `yaml_roundtrip` crash-22cb5f67, 15 bytes). Quoting decides what is text,
    /// so quoting is what this checks — see [`line_has_comment_marker`].
    fn tail_line_has_note(&self) -> bool {
        let bytes = self.output.as_bytes();
        let end = bytes.len().saturating_sub(1);
        let start = bytes[..end]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        line_has_comment_marker(&bytes[start..end])
    }

    /// Write the notes that cannot fit the single inline slot of an empty
    /// container above its line, so a stack of them is never dropped.
    ///
    /// Only possible when the container genuinely starts a line: a value written
    /// straight after `key: ` has no line left to preface, and text cannot produce
    /// that shape with a note stack anyway (one inline slot per line). Measured
    /// before this existed: a comment-only TOML document emitted `{}  # d1` and
    /// lost `# d2` outright.
    fn write_empty_slot_notes(&mut self, sk: &ContainerSkeleton<'_>) {
        let (_, above) = empty_slot_notes(sk.meta);
        if above.is_empty() || !(self.output.ends_with('\n') || self.output.is_empty()) {
            return;
        }
        for note in above {
            self.write_note_line(sk.indent_width, &note.text);
        }
    }

    /// 核心递归序列化方法，处理所有节点类型的缩进和格式化。
    /// `block_base` is the indentation column governing a block scalar's
    /// parent line: the `|`/`>` body must sit deeper than that line, so it is
    /// the *pair/item* indent — not the scalar's own write_indent — whenever a
    /// header shares a line with its parent (`key: |`, `- |`, compact items).
    fn serialize_node_internal(
        &mut self,
        node: &CustomNode,
        indent_width: usize,
        block_base: usize,
        in_value_context: bool,
        depth: usize,
    ) -> Result<(), SerializeError> {
        if depth >= self.max_depth {
            return Err(SerializeError::MaxDepthExceeded(DepthError(self.max_depth)));
        }

        // Handle standalone comments first (all of them, in source order — a
        // stack of note lines above one key is ordinary, and printing only the
        // first or only the last silently drops the rest), but not on empty
        // containers, where a bare `{}`/`[]` on its own line would be invalid
        // YAML — those comments are demoted to inline by the writer below.
        // PR #117: `leading_comments()` normalises across the new
        // `decor.leading_comments` slot (used by the native TOML and
        // JSON engines) and the older `comment(standalone = true)`
        // slot the YAML receiver still writes today, so a document
        // converted from TOML / JSON keeps its leading notes when
        // re-serialised as YAML.
        if !Self::is_empty_container(node) && node.has_notes() {
            for comment in node.leading_comments() {
                self.write_note_line(indent_width, &comment.text);
            }
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                meta,
                chomping,
                block_indent,
                ..
            } => self.write_scalar_node(
                value,
                style,
                meta,
                BlockScalarHeader {
                    chomping,
                    indent: *block_indent,
                    comment: meta
                        .comment
                        .as_ref()
                        .filter(|c| !c.standalone)
                        .map(|c| &c.text[..]),
                },
                indent_width,
                block_base,
            )?,
            CustomNode::Mapping {
                pairs,
                meta,
                flow_style,
                ..
            } => self.write_mapping_node(
                pairs,
                meta,
                *flow_style,
                indent_width,
                depth,
                in_value_context,
            )?,
            CustomNode::Sequence {
                items,
                meta,
                flow_style,
                ..
            } => self.write_sequence_node(
                items,
                meta,
                *flow_style,
                indent_width,
                depth,
                in_value_context,
            )?,
            CustomNode::Null { meta, .. } => self.write_null_node(meta, indent_width)?,
            CustomNode::Alias { name } => self.write_alias_node(name, indent_width)?,
        }
        Ok(())
    }

    /// Serialize a scalar node (plain / quoted / block) with its anchor, tag and
    /// trailing comment. Extracted from `serialize_node_internal`.
    fn write_scalar_node(
        &mut self,
        value: &str,
        style: &ScalarStyle,
        meta: &NodeMeta,
        block: BlockScalarHeader<'_>,
        indent_width: usize,
        block_base: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        if meta.anchor.is_some() || meta.tag.is_some() {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        // A value that renders as nothing but its tag leaves the line "pending": the
        // scanner has seen no scalar text, so a note line written next is reported as
        // this value's leading note whichever node the note belonged to. Recorded here
        // so the next note line can close the line first (`… !- ~`) — the only fix that
        // keeps the note's owner; see `close_pending_tag_line`.
        self.pending_tag_insert_at =
            if value.is_empty() && matches!(style, ScalarStyle::Plain) && meta.tag.is_some() {
                Some(self.output.len())
            } else {
                None
            };
        let line_start = self.output.len();
        self.write_scalar(value, style, block, self.width, block_base);
        let block_body = matches!(style, ScalarStyle::Literal | ScalarStyle::Folded);
        let mut noted = false;
        if let Some(c) = &meta.comment
            && !c.standalone
            && !block_body
        {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
            noted = true;
        }
        // The slot is used up by this scalar's own note, and neither a block
        // body's last content line nor a wrapped continuation can host one.
        self.trailing_note_slot_open =
            !noted && !block_body && !self.output[line_start..].contains('\n');
        self.output.push('\n');
        Ok(())
    }

    /// Serialize a mapping node in either flow (`{ ... }`) or block style,
    /// including anchor/tag and trailing comment. The container skeleton is
    /// shared with sequences by [`write_container_node`]; only the element
    /// renderer, pair separator, and block iteration differ.
    fn write_mapping_node(
        &mut self,
        pairs: &IndexMap<CustomNode, CustomNode>,
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        // A note stack riding the first entry cannot stay *below* this node's own
        // anchor/tag header line: granit reports a note in that position as the
        // tagged node's leading comment, so the next round moves it above the
        // header and a single emission step never reaches a fixed point (libFuzzer
        // `yaml_roundtrip` crash-77a8039b, 28 bytes; crash-e6551c75 and
        // crash-8f7085b0 for the marker-spine half). See [`lift_first_entry_notes`].
        let (stripped_map, preamble_notes) =
            lift_first_entry_notes(pairs, meta, flow_style, in_value_context);
        let pairs: &IndexMap<CustomNode, CustomNode> = stripped_map.as_ref().unwrap_or(pairs);
        let preamble_notes = preamble_notes.as_deref();
        let sk = ContainerSkeleton {
            empty: pairs.is_empty(),
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open: '{',
            close: '}',
            preamble_note: preamble_notes,
        };
        self.write_container_node(
            &sk,
            |s| {
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        s.output.push_str(", ");
                    }
                    s.write_scalar_for_key(key, true);
                    s.output.push_str(": ");
                    s.serialize_flow_value(value, depth + 1)?;
                }
                Ok(())
            },
            |s| {
                let mut pairs_vec: Vec<(&CustomNode, &CustomNode)> = pairs.iter().collect();
                pairs_vec.sort_by(|a, b| {
                    let ka = match a.0 {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => "",
                    };
                    let kb = match b.0 {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => "",
                    };
                    ka.cmp(kb)
                });
                for (key, value) in pairs_vec.iter().copied() {
                    s.write_mapping_pair(key, value, indent_width, depth)?;
                }
                Ok(())
            },
            |s| {
                for (key, value) in pairs.iter() {
                    s.write_mapping_pair(key, value, indent_width, depth)?;
                }
                Ok(())
            },
        )
    }

    /// Serialize a sequence node in either flow (`[ ... ]`) or block style,
    /// including anchor/tag and trailing comment. See [`write_mapping_node`]
    /// for the shared skeleton.
    fn write_sequence_node(
        &mut self,
        items: &[CustomNode],
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        // Same lift as a block mapping's first key: a note under a `&a` / `!tag`
        // header line re-reads as the sequence's own leading comment.
        let (stripped_items, preamble_notes) =
            lift_first_item_notes(items, meta, flow_style, in_value_context);
        let items: &[CustomNode] = stripped_items.as_deref().unwrap_or(items);
        let sk = ContainerSkeleton {
            empty: items.is_empty(),
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open: '[',
            close: ']',
            preamble_note: preamble_notes,
        };
        self.write_container_node(
            &sk,
            |s| {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        s.output.push_str(", ");
                    }
                    s.serialize_flow_value(item, depth + 1)?;
                }
                Ok(())
            },
            // Sequences never sort their items: the sorted branch is the
            // ordinary block pass (mirrors the pre-refactor behavior).
            move |s: &mut Self| {
                for item in items.iter() {
                    s.write_sequence_item(item, indent_width, depth)?;
                }
                Ok(())
            },
            |s| {
                for item in items.iter() {
                    s.write_sequence_item(item, indent_width, depth)?;
                }
                Ok(())
            },
        )
    }

    /// Shared mapping/sequence skeleton: flow form (bracketed, comma-separated
    /// inline elements), the block preamble (anchor/tag line outside a value
    /// context), the empty-container fallback with its node comment, and the
    /// trailing inline comment after block elements.
    fn write_container_node(
        &mut self,
        sk: &ContainerSkeleton<'_>,
        flow: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
        block_sorted: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
        block: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
    ) -> Result<(), SerializeError> {
        let ContainerSkeleton {
            empty,
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open,
            close,
            preamble_note,
        } = *sk;
        if flow_style {
            // An empty flow token written at a line start gets the same preface as
            // its block counterpart below; mid-line (`key: []`) there is no line to
            // preface, which `write_empty_slot_notes` checks.
            if empty {
                self.write_empty_slot_notes(sk);
            }
            // When a flow container starts its own line (a block-context
            // value under a `key:` whose standalone comment forced the newline
            // branch), it still needs the line indent; the inline callers
            // pass indent_width = 0, so this is a no-op for `key: [..]`.
            self.write_indent(indent_width);
            if meta.anchor.is_some() || meta.tag.is_some() {
                self.write_anchor_tag(&meta.anchor, &meta.tag);
            }
            self.output.push(open);
            if !empty {
                flow(self)?;
            }
            self.output.push(close);
            if empty {
                self.output_empty_node_comment(meta);
            } else if let Some(c) = meta.inline_slot() {
                self.output.push_str("  # ");
                self.output.push_str(&c.text);
                self.trailing_note_slot_open = false;
            } else {
                self.trailing_note_slot_open = true;
            }
            self.output.push('\n');
        } else {
            if empty {
                // Spell as `{}` / `[]` and keep the anchor/tag on the same line: a
                // standalone tag line followed by the braces one indent shallow
                // (under an explicit-key `?`) is ambiguous and cannot re-parse.
                // An empty container has no block form, so a mapping/sequence
                // *value* reaches here inline (`key: &a {}`) — emit its own
                // anchor/tag even in value context, or it would vanish.
                self.write_empty_slot_notes(sk);
                self.write_indent(indent_width);
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push(open);
                self.output.push(close);
                self.output_empty_node_comment(meta);
                self.output.push('\n');
                return Ok(());
            }

            if !in_value_context && (meta.anchor.is_some() || meta.tag.is_some()) {
                for note in preamble_note.unwrap_or_default() {
                    self.write_note_line(indent_width, &note.text);
                }
                self.write_indent(indent_width);
                self.write_anchor_tag(&meta.anchor, &meta.tag);
                self.output.push('\n');
            }

            if self.sort_keys {
                block_sorted(self)?
            } else {
                block(self)?
            }

            if let Some(c) = &meta.comment
                && !c.standalone
            {
                // A note in a container's *inline* slot has to land on a line the
                // reader reports inline notes from. Writing it as a line of its own
                // does not: a bare note below a block re-reads as the leading note of
                // the node that ended the block, so the first emission was never a
                // fixed point - `:\t!-<CR>... #-o` gave `~: !- \n# -o\n`, and the
                // re-read moved that note inside the value block as
                // `~:\n  # -o\n  !- \n` (libFuzzer `yaml_roundtrip` crash-11ced252).
                // Append it to the line just finished whenever that line can hold a
                // trailing note; fall back to a note line when it cannot, because a
                // note that is merely mis-indented still beats one that corrupts a
                // block scalar's body or duplicates a line's trailing slot.
                //
                // A pending tag-only line is closed first so the value is no longer
                // unfinished, and the note then rides that line: `~: !- ~  # -o`. Keeping
                // the container's ownership instead (closing and writing a note line,
                // `~: !- ~\n# -o`) was tried and measured: it re-reads with the note on the
                // value, so the next emission goes inline and the text oscillates — owner
                // preservation and a one-round fixed point are mutually exclusive for this
                // shape, and the round-trip contract needs the fixed point. Where the note
                // belongs to a *following* node, `write_note_line` closes the line and the
                // note keeps its owner and settles (crash-c5b367d3).
                if self.trailing_note_slot_open
                    && self.output.ends_with('\n')
                    && !self.tail_line_has_note()
                {
                    self.output.pop();
                    self.output.push_str("  # ");
                    self.output.push_str(&c.text);
                    self.output.push('\n');
                    self.trailing_note_slot_open = false;
                } else {
                    self.write_note_line(indent_width, &c.text);
                }
            }
        }
        Ok(())
    }

    /// Serialize a null node with its anchor, tag and trailing comment.
    /// Extracted from `serialize_node_internal`.
    fn write_null_node(
        &mut self,
        meta: &NodeMeta,
        indent_width: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        if meta.anchor.is_some() || meta.tag.is_some() {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        self.output.push_str("null");
        if let Some(c) = &meta.comment
            && !c.standalone
        {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
            self.trailing_note_slot_open = false;
        } else {
            self.trailing_note_slot_open = true;
        }
        self.output.push('\n');
        Ok(())
    }

    /// Serialize an alias node (`*name`). Extracted from `serialize_node_internal`.
    fn write_alias_node(&mut self, name: &str, indent_width: usize) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        self.output.push('*');
        self.output.push_str(name);
        self.trailing_note_slot_open = true;
        self.output.push('\n');
        Ok(())
    }

    /// Write a scalar value directly to output based on style and chomping.
    /// `remaining` is the remaining width on the current line (0 = don't wrap).
    /// `block_base` is the parent-line indent governing a block scalar's body.
    fn write_scalar(
        &mut self,
        value: &str,
        style: &ScalarStyle,
        block: BlockScalarHeader<'_>,
        remaining: usize,
        block_base: usize,
    ) {
        match style {
            ScalarStyle::Plain => self.write_plain_scalar(value, remaining),
            // Single-quoted scalars cannot represent control characters, Unicode
            // noncharacters, or (losslessly) newlines — a raw `\0` inside `'…'`
            // is not re-parseable. When a value carries such a char (e.g. a
            // JSON5 single-quoted string holding a control char, whose
            // SingleQuoted style the parser preserves for fidelity), downgrade
            // to double-quoted, which can escape them. Found by the dialect
            // no-panic / re-parse fuzz: from_json5 emitted `'\0'` that
            // re-parse rejected.
            ScalarStyle::SingleQuoted
                if value.contains('\n')
                    || value.chars().any(char::is_control)
                    || value.chars().any(is_yaml_noncharacter) =>
            {
                self.write_double_quoted_scalar(value)
            }
            ScalarStyle::SingleQuoted => self.write_single_quoted_scalar(value),
            ScalarStyle::DoubleQuoted => self.write_double_quoted_scalar(value),
            ScalarStyle::Literal => self.write_literal_scalar(value, block, block_base),
            ScalarStyle::Folded => self.write_folded_scalar(value, block, block_base),
        }
    }

    /// Write one `key: value` pair of a block mapping, including indentation,
    /// the `?` marker for complex keys, and the value emission (block value on
    /// a new line, or inline).
    pub(crate) fn write_mapping_pair(
        &mut self,
        key: &CustomNode,
        value: &CustomNode,
        indent_width: usize,
        depth: usize,
    ) -> Result<(), SerializeError> {
        // Check if key is a complex key (mapping or sequence)
        let is_complex_key = matches!(
            key,
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. }
        );

        if is_complex_key {
            // Complex key: use ? indicator. A standalone note on the key
            // must precede the `?` marker — serializing the key body
            // mid-line would emit the comment *after* `? ` (spelling
            // `? # a` + an orphaned key node that cannot re-parse). Emit
            // the note ourselves, then strip it from the cloned body.
            //
            // The two lifts compose rather than compete. A marker chain can carry a
            // note on the key node *and* leave a second one riding the first entry
            // of the body, and the reader reports that one at the marker's level too;
            // taking only the key's own note wrote the body note one indent deeper,
            // where the next re-read hoisted it — the emission then settled only on
            // its second round (libFuzzer `yaml_roundtrip` crash-456176be,
            // crash-f8525a9e, crash-c9031de4: `?` + ` ### standab:` + ` ?` +
            // `  # ! y%% yam2:#l: tr` + `  ~: ~`, where the source tree holds the
            // first note on the key mapping and the second on its inner `~` key).
            let mut stripped;
            let hoisted;
            let key_notes = key.leading_comments();
            let key = if !key_notes.is_empty() {
                for comment in key_notes {
                    self.write_note_line(indent_width, &comment.text);
                }
                stripped = strip_leading_comment(key);
                hoisted = hoist_marker_note(&mut stripped);
                &stripped
            } else if matches!(
                key,
                CustomNode::Mapping {
                    flow_style: false,
                    ..
                }
            ) {
                // The key has no note of its own, but a chain of markers on one line
                // leaves one riding a nested key inside its body, where the reader
                // will not put it back. Bring it up to this marker's line instead.
                stripped = key.clone();
                hoisted = hoist_marker_note(&mut stripped);
                &stripped
            } else {
                hoisted = Vec::new();
                key
            };
            for note in &hoisted {
                self.write_note_line(indent_width, note.text.as_ref());
            }
            self.write_indent(indent_width);
            self.output.push('?');
            self.output.push('\n');
            // The key body lives on its own lines one step deeper than the
            // `?` marker. Starting it mid-line (`? A: …`) pinned the node's
            // first column to the `? ` width while every following line used
            // indent_width, splitting multi-pair / nested keys across two
            // incompatible indents that could not re-parse.
            let body_indent = indent_width + self.indent_mapping;
            self.serialize_node_internal(key, body_indent, body_indent, false, depth + 1)?;
        } else {
            // Simple key
            // Handle standalone comments before the key (every note in the stack,
            // in source order). `has_notes` keeps the note-free case off the
            // normalised view: it is every simple key of every document.
            if key.has_notes() {
                for comment in key.leading_comments() {
                    // Through `write_note_line`, not a hand-rolled `# ` write: that is
                    // where a pending tag-only line gets closed, and a key note printed
                    // straight after `bg: !:` is otherwise swallowed by that value
                    // (libFuzzer `yaml_roundtrip` crash-c5b367d3).
                    self.write_note_line(indent_width, &comment.text);
                }
            }
            self.write_indent(indent_width);
            self.write_scalar_for_key(key, false);
        }

        if is_complex_key {
            // The key body always ended its line (`serialize_node_internal`
            // terminates with `\n`), so the value marker needs its own line
            // aligned with the `?` indicator — pushing `:` bare would land it
            // at column 0 and close any enclosing collection.
            self.write_indent(indent_width);
        }
        self.output.push(':');

        // Where a simple key's trailing note may still be written: the end of the
        // `key:` line once the value has moved down to a line of its own, or (by
        // default) the end of whatever line the pair finishes on. Without the first
        // case the note lands on the value's line, where a reader reports it as the
        // *value's* leading note for a tag-only scalar — the note then changes owner
        // every round and the emission never settles.
        let mut key_note_at: Option<usize> = None;

        // Check if value needs to be on next line
        if (matches!(
            value,
            CustomNode::Mapping {
                flow_style: false,
                ..
            } | CustomNode::Sequence {
                flow_style: false,
                ..
            }
        ) && !Self::is_empty_container(value))
            || is_complex_key
            || (value.leading_comment().is_some() && !Self::is_empty_container(value))
        {
            // Write a block container's anchor/tag after the colon: only
            // block-style Mapping / Sequence suppress their own header in
            // value context (`write_container_node`'s non-flow branch), so
            // only those need the parent to pre-emit it. A flow container
            // (`!tag [..]`) always writes its own header on its line even in
            // value context — pre-emitting for it duplicated the tag
            // (`a: !a` … `!a [0]`) and the text could not re-parse. Scalar /
            // Null nodes likewise still write their own anchor+tag on the
            // node line — pre-emitting for those duplicated the header
            // (`A: !a` … `!a null`) and a standalone comment landed
            // between the two halves. An empty block container reaches the
            // next-line branch only under a complex key, and now writes its
            // own anchor/tag in `write_container_node`, so pre-emitting would
            // duplicate it (`: !a` … `!a {}`).
            if matches!(
                value,
                CustomNode::Mapping {
                    flow_style: false,
                    ..
                } | CustomNode::Sequence {
                    flow_style: false,
                    ..
                }
            ) && !Self::is_empty_container(value)
            {
                if let Some(anchor_name) = value.anchor() {
                    self.output.push_str(" &");
                    self.output.push_str(anchor_name);
                }
                if let Some(t) = value.tag() {
                    self.output.push(' ');
                    self.output.push_str(&Self::yaml_tag_text(t));
                }
            }
            self.output.push('\n');
            key_note_at = Some(self.output.len() - 1);
            let child_indent = indent_width + self.indent_mapping;
            self.serialize_node_internal(value, child_indent, child_indent, true, depth + 1)?;
        } else {
            self.output.push(' ');
            // Inline value: the header shares the `key:` line, so a block
            // body must be measured from the pair indent, not from zero.
            self.serialize_node_internal(value, 0, indent_width, true, depth + 1)?;
        }

        // granit attaches a note that trails a simple key to *that key* node, but
        // once the pair is written as `key: value` there is no spelling for a
        // comment between the key and its `:` — YAML puts a trailing note after the
        // value, and re-reading hands it to the value node. The note used to be
        // dropped outright (`? a # note` + `: b` emitted `a: b`); it now rides the
        // only slot a reader can report it from, so it survives the round trip and
        // the line is stable because the next parse agrees on that position. The
        // value keeps priority when it carries a note of its own — one line has one
        // trailing slot — and complex keys are left alone, their notes travel with
        // the `?` body.
        if !is_complex_key && value.comment().is_none() && self.output.ends_with('\n') {
            let key_note = match key {
                CustomNode::Scalar { meta, .. } | CustomNode::Null { meta, .. } => meta
                    .comment
                    .as_ref()
                    .filter(|c| !c.standalone)
                    .map(|c| c.text.clone()),
                _ => None,
            };
            if let Some(text) = key_note {
                let at = key_note_at.unwrap_or(self.output.len() - 1);
                self.output.insert_str(at, &format!("  # {text}"));
            }
        }

        Ok(())
    }

    /// Write one `- ` item of a block sequence, keeping the compact inline
    /// form for plain mappings on the dash line.
    pub(crate) fn write_sequence_item(
        &mut self,
        item: &CustomNode,
        indent_width: usize,
        depth: usize,
    ) -> Result<(), SerializeError> {
        // Three note slots that `write_mapping_pair` respects are unreachable from this
        // hand-rolled line, and all three have to be written here:
        //
        // * the item's own leading stack, which only a line above the dash can hold — by the
        //   time the loop below runs, `- ` is already open on the key's line;
        // * a *later* pair's key leading stack, which goes at the pair's own indent (a first
        //   pair's key never owns one: measured across twelve crafted shapes, the reader
        //   hands such a note to the item or to the enclosing sequence);
        // * a key's inline note, which rides its pair line.
        //
        // Measured once: `is_compact_item` walks every pair, and this is a hot path, so the
        // branch below reuses one answer rather than asking twice.
        let compact = is_compact_item(item);
        // `has_notes` first, as every writer here does: it is two discriminant tests and
        // never dereferences the boxed `NodeDecor`, while `leading_comments` builds the
        // normalised view — and a note-free item is the common case.
        if compact && item.has_notes() {
            for comment in item.leading_comments() {
                self.write_note_line(indent_width, &comment.text);
            }
        }
        self.write_indent(indent_width);
        self.output.push_str("- ");

        if compact {
            // Compact form: `- key: value` with subsequent keys
            // indented to align under the first key. Only when the
            // mapping carries no metadata and every key/value can
            // share the dash line.
            let CustomNode::Mapping { pairs, .. } = item else {
                return Err(SerializeError::Internal("is_compact_item on non-mapping"));
            };
            for (pi, (key, value)) in pairs.iter().enumerate() {
                // Compact key column is dash indent + `- ` (== indent_sequence
                // for the default 2-step); a block body hangs off that line.
                let key_base = indent_width + self.indent_sequence;
                if pi > 0 {
                    // Same slot rule as `write_mapping_pair`: every note in the stack, in
                    // source order, on a line of its own at the pair's indent.
                    if key.has_notes() {
                        for comment in key.leading_comments() {
                            self.write_note_line(key_base, &comment.text);
                        }
                    }
                    self.write_indent(key_base);
                }
                self.write_scalar_for_key(key, false);
                self.output.push(':');
                self.output.push(' ');
                self.serialize_node_internal(value, 0, key_base, true, depth + 1)?;

                // The loop hand-rolls the `key: value` line, and until now it hand-rolled
                // only the text: a note granit attaches to a simple key had no slot here,
                // so `- a: !   # n` emitted `- a: ! ` and the note was gone from the first
                // round — silent data loss rather than a relocation. Same rule as
                // `write_mapping_pair`: the note rides the line it belongs to, and the
                // value keeps the slot when it carries a note of its own.
                //
                // Ordered for the hot path: a plain field test on the key decides whether
                // to look any further, so a note-free pair costs one `Option` check rather
                // than two method calls plus a tail scan.
                let key_note = match key {
                    CustomNode::Scalar { meta, .. } | CustomNode::Null { meta, .. } => {
                        meta.comment.as_ref().filter(|c| !c.standalone)
                    }
                    _ => None,
                };
                if let Some(note) = key_note
                    && value.comment().is_none()
                    && self.output.ends_with('\n')
                {
                    let at = self.output.len() - 1;
                    self.output.insert_str(at, &format!("  # {}", note.text));
                }
            }
        } else if matches!(
            item,
            CustomNode::Mapping {
                flow_style: false,
                ..
            } | CustomNode::Sequence {
                flow_style: false,
                ..
            }
        ) || item.leading_comment().is_some()
        {
            self.output.push('\n');
            let child_indent = indent_width + self.indent_sequence;
            self.serialize_node_internal(item, child_indent, child_indent, false, depth + 1)?;
        } else {
            // For simple items (including flow-style containers),
            // they go on the same line as the dash. Don't pass
            // indent_width to avoid extra indentation; a block body still
            // hangs off the dash line, so the base is the dash indent.
            self.serialize_node_internal(item, 0, indent_width, false, depth + 1)?;
        }

        Ok(())
    }

    /// Write a scalar formatted as a mapping key.
    fn write_scalar_for_key(&mut self, node: &CustomNode, flow: bool) {
        // A mapping key carries the same node properties as a value: granit
        // attaches an anchor/tag that precede a simple key to that key node, so
        // the emitter must write them too. Dropping them lost the metadata on
        // re-parse (libFuzzer `yaml_roundtrip` crash-62bcff6f: `&f& !&&f&&&  :`,
        // whose empty key held both an anchor and a tag). Complex keys already
        // route through `serialize_node_internal`, which emits properties, so
        // this covers only the scalar / null paths here.
        if let Some(meta) = match node {
            CustomNode::Scalar { meta, .. } | CustomNode::Null { meta, .. } => Some(meta),
            _ => None,
        } {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        match node {
            CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } => {
                if value.is_empty() {
                    // An unquoted empty key re-reads as the null `~` scalar, not
                    // an empty string — quote it to keep the value faithful.
                    self.output.push_str("\"\"");
                } else if flow && flow_plain_unsafe(value) {
                    // `,`/`[`,`]`,`{`,`}` end a plain token inside a flow
                    // collection; quoting is the only lossless escape.
                    self.write_double_quoted_scalar(value);
                } else if is_short_alphanumeric(value) {
                    self.output.push_str(value);
                } else {
                    self.write_plain_scalar(value, 0);
                }
            }
            CustomNode::Scalar {
                value,
                style,
                chomping,
                block_indent,
                ..
            } => {
                // A key line cannot host a block scalar header either: the
                // body would collide with the `:` and the mapping structure.
                // Quote it, same normalization as flow values.
                let style = match style {
                    ScalarStyle::Literal | ScalarStyle::Folded => &ScalarStyle::DoubleQuoted,
                    other => other,
                };
                // `style` is no longer a block style, so the indentation
                // indicator never reaches the writer here.
                self.write_scalar(
                    value,
                    style,
                    BlockScalarHeader {
                        chomping,
                        indent: *block_indent,
                        comment: None,
                    },
                    0,
                    0,
                )
            }
            _ => {
                self.output.push_str("null");
            }
        }
    }

    /// Write a plain scalar, quoting if necessary.
    /// `remaining` is the remaining width on the current line (0 = don't wrap).
    fn write_plain_scalar(&mut self, value: &str, remaining: usize) {
        write_plain_scalar(&mut self.output, value, remaining, self.width);
    }

    /// Write a single-quoted scalar (single quotes escaped by doubling).
    fn write_single_quoted_scalar(&mut self, value: &str) {
        write_single_quoted_scalar(&mut self.output, value);
    }

    /// Write a double-quoted scalar with escape sequences.
    fn write_double_quoted_scalar(&mut self, value: &str) {
        write_double_quoted_scalar(&mut self.output, value);
    }

    /// Emit a block scalar header: the `|`/`>` sigil, the chomping indicator
    /// and — when the source pinned one — the explicit indentation indicator.
    /// Order follows the spec (`c-b-block-header`): chomping precedes
    /// indentation, so a stripped, explicitly indented scalar is `|-2`.
    fn write_block_header(
        &mut self,
        sigil: char,
        block: BlockScalarHeader<'_>,
        chomping: &Chomping,
    ) {
        self.output.push(sigil);
        self.output.push_str(match chomping {
            Chomping::Strip => "-",
            Chomping::Clip => "",
            Chomping::Keep => "+",
        });
        if let Some(indent) = block.indent.filter(|n| (1..=9).contains(n)) {
            self.output.push((b'0' + indent) as char);
        }
        if let Some(text) = block.comment {
            // The comment rides the header line: a comment on the line after
            // `|`/`>` would be absorbed into the block content on re-read
            // (libFuzzer `yaml_roundtrip` crash-cfb3fa83).
            self.output.push_str("  # ");
            self.output.push_str(text);
        }
    }

    /// Write a literal block scalar (`|`) with its block header.
    fn write_literal_scalar(
        &mut self,
        value: &str,
        block: BlockScalarHeader<'_>,
        block_base: usize,
    ) {
        let auto_width = self.block_width(block_base, block.indent);
        let first_text_line = value.lines().find(|l| !l.is_empty()).unwrap_or("");
        let no_content_line = first_text_line.is_empty();
        let chomping = effective_chomping(value, block.chomping, no_content_line);
        // Mirror the folded writer: when the first content line itself begins
        // with a blank, granit's auto-indent detection would take that deeper
        // column as the block indent and read the shallower following lines as
        // a dedent (ending the block / erroring on re-parse) — libFuzzer
        // `yaml_roundtrip` crash-e432d4b8 (`|1` whose explicit indicator the AST
        // dropped, leaving a value like ` 1|l\n:t\n`). Force the indicator so
        // detection is skipped and the leading blanks stay content. The same
        // resolved indent feeds `write_base_indent` so header and body agree.
        let force_indicator = block.indent.is_none() && first_text_line.starts_with([' ', '\t']);
        let indent = if no_content_line {
            // Neither an empty nor an all-break body can carry a recoverable
            // indentation indicator: on re-parse granit drops it (there is no content
            // line to measure the indent against), so emitting `|N` drifts to `|` the
            // next round (libFuzzer `yaml_roundtrip` crash-d4ea8a23, and the `\r\r#`
            // shape that also decayed the value). Emit the bare sigil so the shape is
            // idempotent.
            None
        } else if force_indicator {
            Some((auto_width - block_base) as u8)
        } else {
            block.indent
        };
        let header = BlockScalarHeader {
            chomping: &chomping,
            indent,
            comment: block.comment,
        };
        self.write_block_header('|', header, &chomping);
        self.output.push('\n');
        self.write_base_indent(value, block_base, indent);
    }

    fn write_folded_scalar(
        &mut self,
        value: &str,
        block: BlockScalarHeader<'_>,
        block_base: usize,
    ) {
        let auto_width = self.block_width(block_base, block.indent);
        let first_text_line = value.lines().find(|l| !l.is_empty()).unwrap_or("");
        let no_content_line = first_text_line.is_empty();
        let chomping = effective_chomping(value, block.chomping, no_content_line);
        // granit's folded read map (measured against its scanner):
        //   - k blank lines before a NORMAL continuation line re-read as k '\n'
        //     (the break before them folds away);
        //   - a MORE-INDENTED continuation line keeps its own leading break, so
        //     k blanks there re-read as k + 1 - one blank less is needed for a
        //     given run;
        //   - k blanks right after the header are k leading newlines.
        // A first content line that itself begins with a blank would also be
        // eaten by granit's auto-indent detection, so force an explicit
        // indentation indicator in that case (with it set, detection is skipped
        // and the leading blanks stay content). Run-consumed segments mean an
        // empty segment can only ever be the leading one.
        let force_indicator = block.indent.is_none() && first_text_line.starts_with([' ', '\t']);
        let width = auto_width;
        let header = BlockScalarHeader {
            chomping: &chomping,
            indent: if no_content_line {
                // Neither an empty nor an all-break folded body can carry the
                // indentation indicator on re-parse, so emitting `>N` drifts to `>`
                // next round (mirror of the literal writer; libFuzzer `yaml_roundtrip`
                // crash-d4ea8a23 family).
                None
            } else if force_indicator {
                Some((width - block_base) as u8)
            } else {
                block.indent
            },
            comment: block.comment,
        };
        self.write_block_header('>', header, &chomping);
        self.output.push('\n');
        let mut rest = value;
        let mut started = false;
        loop {
            let nl = match rest.find('\n') {
                Some(k) => k,
                None => {
                    if !rest.is_empty() {
                        self.write_indent(width);
                        self.output.push_str(rest);
                    }
                    break;
                }
            };
            let line = &rest[..nl];
            let mut r = 1usize;
            while rest[nl + r..].starts_with('\n') {
                r += 1;
            }
            let after = &rest[nl + r..];
            let this_more_indented = line.starts_with([' ', '\t']);
            if !line.is_empty() {
                self.write_indent(width);
                self.output.push_str(line);
            }
            // granit's fold rule keys on `leading_blank`/`trailing_blank`: a
            // more-indented line (starting with a blank) both keeps its own
            // leading break un-folded AND sets `leading_blank`, so the break on
            // the line AFTER it is likewise kept. The break we are about to
            // emit sits between the line just written (`this_more_indented`, a
            // more-indented line keeps the break after it) and the next content
            // line (`next_more_indented`, whose leading break is kept). A run of
            // r newlines therefore needs no blank padding (just r physical
            // newlines) when either neighbour is more-indented; only a fold
            // between two plain text lines costs the extra break (r + 1).
            // Leading runs have no prior text line and trailing runs defer to
            // `write_scalar_node`.
            let had_text = started || !line.is_empty();
            if !line.is_empty() {
                started = true;
            }
            let next_more_indented = after.starts_with([' ', '\t']);
            let newlines = if after.is_empty() {
                r - 1
            } else if !had_text || this_more_indented || next_more_indented {
                r
            } else {
                r + 1
            };
            for _ in 0..newlines {
                self.output.push('\n');
            }
            if after.is_empty() {
                break;
            }
            rest = after;
        }
    }

    /// Content column of a block body: the explicit indicator wins, else one
    /// indent step below the header line - the same rule `write_base_indent`
    /// uses for literal blocks (parsed block content is stored de-indented,
    /// so sniffing the value for leading spaces would scan the whole body for
    /// nothing). Keep the two writers in lockstep.
    fn block_width(&self, block_base: usize, explicit_indent: Option<u8>) -> usize {
        match explicit_indent.filter(|n| (1..=9).contains(n)) {
            Some(n) => block_base + n as usize,
            None => block_base + self.indent_size,
        }
    }

    /// Write each line of the block scalar content with base indentation appended.
    /// Writes directly to output (no intermediate Vec or join); indentation goes
    /// through the memoized `write_indent` cache instead of a fresh `repeat()`
    /// allocation per block scalar. The body sits one step deeper than the line
    /// carrying the `|`/`>` header, wherever that line lives (top level, a
    /// nested pair, after a dash).
    ///
    /// `explicit_indent` is the source's indentation indicator, which is
    /// relative to the *parent* line, not to `self.indent_size`. It has to win:
    /// the indicator is already on the wire, and if the body did not land at
    /// exactly `block_base + n` the reader would derive a different content
    /// indent and silently change the value. A body whose first line is deeper
    /// than the rest (`|2` over `[1, 2, 3]` / `  `) is the case that needs it —
    /// auto-detection alone would see the deeper first line and misread the
    /// shallower one as a dedent. Values carrying an out-of-range indicator
    /// (only reachable from hand-built ASTs) fall back to `indent_size`, whose
    /// number the header above did not print, so the two stay consistent.
    fn write_base_indent(&mut self, value: &str, block_base: usize, explicit_indent: Option<u8>) {
        let width = match explicit_indent.filter(|n| (1..=9).contains(n)) {
            Some(n) => block_base + n as usize,
            None => block_base + self.indent_size,
        };
        let mut first = true;
        for line in value.lines() {
            if !first {
                self.output.push('\n');
            }
            if !line.is_empty() {
                self.write_indent(width);
                self.output.push_str(line);
            }
            first = false;
        }
    }

    /// 在 flow 上下文中序列化值（不追加换行符）。
    fn serialize_flow_value(
        &mut self,
        node: &CustomNode,
        depth: usize,
    ) -> Result<(), SerializeError> {
        if depth >= self.max_depth {
            return Err(SerializeError::MaxDepthExceeded(DepthError(self.max_depth)));
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                meta,
                chomping,
                block_indent,
                ..
            } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                // A block scalar (`|` / `>`) cannot live inside a flow
                // collection — its body needs a dedicated, deeper-indented
                // region that flow syntax has no room for. Downgrade to a
                // double-quoted scalar so the emitted text always re-parses
                // (the value survives; only the lost style is normalized,
                // exactly like ruamel/PyYAML do for block scalars in flow).
                let style = match style {
                    ScalarStyle::Literal | ScalarStyle::Folded => &ScalarStyle::DoubleQuoted,
                    // Flow indicators end a plain token mid-scalar, not just
                    // at its start — `needs_double_quoted` only guards the
                    // block-context spelling.
                    ScalarStyle::Plain if flow_plain_unsafe(value) => &ScalarStyle::DoubleQuoted,
                    other => other,
                };
                self.write_scalar(
                    value,
                    style,
                    BlockScalarHeader {
                        chomping,
                        indent: *block_indent,
                        comment: None,
                    },
                    self.width,
                    0,
                );
            }
            CustomNode::Null { meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push_str("null");
            }
            CustomNode::Mapping { pairs, meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.write_scalar_for_key(key, true);
                    self.output.push_str(": ");
                    self.serialize_flow_value(value, depth + 1)?;
                }
                self.output.push('}');
            }
            CustomNode::Sequence { items, meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_flow_value(item, depth + 1)?;
                }
                self.output.push(']');
            }
            CustomNode::Alias { name } => {
                self.output.push('*');
                self.output.push_str(name);
            }
        }
        Ok(())
    }
}

// Shared scalar-formatting helpers used by both [`Serializer`] (the AST path)
// and the Python-side `DirectWriter` fast path. Operating on a caller-owned
// `String` keeps them crate-agnostic and stops the two mirror implementations
// from drifting. Output is byte-identical to the previous serializer methods.
//
// `remaining` is the remaining width on the current line (0 disables the
// first-line wrap); `width` is the full wrap width used for continuations.

/// Append `value` to `out` as a single-quoted YAML scalar (quotes doubled).
pub fn write_single_quoted_scalar(out: &mut String, value: &str) {
    out.push('\'');
    for c in value.chars() {
        if c == '\'' {
            out.push_str("''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
}

/// Whether `c` is a Unicode noncharacter (U+FFFE/U+FFFF and the plane-end
/// twins U+1FFFE…U+10FFFF). granit-parser rejects these even inside quoted
/// scalars, so they must always be escaped.
// `is_yaml_noncharacter` now lives in the shared `pyrs-schema` crate (used by
// both the scalar resolvers and the YAML emitter); re-exported here so
// `crate::serializer::is_yaml_noncharacter` callers keep resolving.
pub use pyrs_schema::is_yaml_noncharacter;

/// Lower-case hex digits, matching the `%{b:02x}` spelling the round-trip tests
/// and the committed fuzz seeds pin.
const ASCII_HEX: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// Build a byte-membership table from `extra`, with every ASCII alphanumeric
/// already set — so a tag suffix scan is one indexed load per byte instead of an
/// alphanumeric test plus a linear slice search.
const fn safe_table(extra: &[u8]) -> [bool; 128] {
    let mut table = [false; 128];
    let mut i = 0usize;
    while i < 128 {
        table[i] = (i as u8).is_ascii_alphanumeric();
        i += 1;
    }
    let mut j = 0usize;
    while j < extra.len() {
        table[extra[j] as usize] = true;
        j += 1;
    }
    table
}

/// Membership test that keeps non-ASCII (and anything past the table) out.
#[inline]
fn lookup(table: &[bool; 128], b: u8) -> bool {
    match table.get(usize::from(b)) {
        Some(&allowed) => allowed,
        None => false,
    }
}

/// Append `value` to `out` as a double-quoted YAML scalar, escaping control
/// and special characters.
pub fn write_double_quoted_scalar(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\x1B' => out.push_str("\\e"),
            '/' => out.push_str("\\/"),
            c if c.is_control() || is_yaml_noncharacter(c) || c == '\u{FEFF}' => {
                // `\u` escapes are 4 hex digits (BMP only); characters above
                // U+FFFF must use the 8-digit `\U` form.
                //
                // U+FEFF needs its own test: YAML treats it as *restricted* (legal
                // only as a byte-order mark at the very start of a stream), and it
                // is a `Cf` format character - neither `is_control()` (which covers
                // `Cc`) nor a Unicode noncharacter - so both existing checks miss it.
                // granit hands the writer the decoded text, so a scalar carrying a
                // BOM was emitted raw even inside double quotes, and our own parser
                // then rejected our output outright: "a BOM must not appear inside a
                // document" (libFuzzer `yaml_roundtrip` crash-2d14c6f6).
                // `needs_double_quoted` already forces the quoting (it lists
                // U+FEFF); this arm supplies the escape that quoting alone cannot
                // provide, since single- and plain styles have no escape mechanism.
                let u = c as u32;
                if u > 0xFFFF {
                    out.push_str(&format!("\\U{:08x}", u));
                } else {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Whether a plain scalar must be rendered double-quoted because it would be
/// ambiguous or invalid as an unquoted token.
fn needs_double_quoted(value: &str) -> bool {
    // Empty scalars must always be quoted: an empty plain token parses back as
    // null, not as an empty string.
    if value.is_empty() {
        return true;
    }
    // A token with edge whitespace can never survive plain emission: the
    // parser strips it (`" "` reparses as an empty — or in flow, broken —
    // token), so quote it even when the raw text would schema-resolve to a
    // non-string (e.g. `" "` → null). The schema promise is moot once
    // re-parsing cannot recover the token at all.
    if value.starts_with([' ', '\t']) || value.ends_with([' ', '\t']) {
        return true;
    }
    // A plain scalar that BEGINS with a document indicator is read back as the
    // marker, not the string, when it lands at column 0: bare `...` re-reads as a
    // document-end, `---` as a document-start, and `... k` / `--- k` as a marker
    // followed by invalid trailing content. Quote the exact indicators AND the
    // `<marker> …` prefixed forms. `---`/`-…` is partly caught by the leading-`-`
    // rule below, but `...` starts with `.` - not a YAML indicator - so it slips
    // past every other clause (libFuzzer `yaml_roundtrip`: crash-41acfbbe ` ...`
    // -> null; crash-08f05e25 ` ...\nk` -> `... k` -> "invalid content after
    // document end marker").
    if value == "..." || value == "---" || value.starts_with("... ") || value.starts_with("--- ") {
        return true;
    }
    // A plain scalar that resolves to a non-string type (int/float/bool/null)
    // is emitted unquoted: its loaded type under the core schema equals
    // `resolve_core_type(text)`, so plain emission always reproduces that type
    // on re-parse. Quoting a value like `-1` would instead load back as the
    // string "-1", because quoted scalars are never schema-resolved (YAML 1.2).
    if core_type_is_non_string(value) {
        return false;
    }
    // Genuine strings: quote only when raw emission would be ambiguous or
    // invalid YAML (YAML indicator characters at the token start, an embedded
    // colon or hash, or a newline).
    value.contains(':')
        || value.contains('#')
        || value.contains('\n')
        || value.contains('\u{00a0}')
        || value.contains('\u{feff}')
        || value.chars().any(char::is_control)
        || value.chars().any(is_yaml_noncharacter)
        || value.starts_with('-')
        || value.starts_with('{')
        || value.starts_with('}')
        || value.starts_with('[')
        || value.starts_with(']')
        || value.starts_with('*')
        || value.starts_with('&')
        || value.starts_with('!')
        || value.starts_with('?')
        || value.starts_with('%')
        || value.starts_with('@')
        || value.starts_with('`')
        || value.starts_with('\'')
        || value.starts_with('"')
        || value.starts_with('|')
        || value.starts_with('>')
        || value.starts_with(',')
}

/// Take every standalone leading note off a node, leaving no empty decoration
/// behind. Ingest order is preserved, so a stack of comment lines above a key
/// comes back as the same stack.
///
/// Read through `CustomNode::leading_comments()` rather than the `decor` field: a
/// note lives in one of two conventions — the receiver fills the legacy
/// `comment { standalone: true }` slot, hand-built fixtures the `decor` list — and
/// peeking at only one of them is the same fork the `standalone_slice()` normaliser
/// exists to prevent. Notably the list can hold a note whose `standalone` flag is
/// `false` (the merge and null-key re-homing moves notes there), which a
/// convention-by-convention read would either miss or duplicate. `remove_leading_comment`
/// clears both conventions, so what is taken here cannot also be emitted there.
fn take_leading_notes(node: &mut CustomNode) -> Vec<crate::ast::Comment> {
    let notes = node.leading_comments().to_vec();
    node.remove_leading_comment();
    notes
}

/// Pull up the notes that a nested explicit key would otherwise emit inside its
/// own body: walk the first-pair-key spine of a block mapping (the chain of `?`
/// markers a single line opens) and take **every** stack sitting on it, outer
/// before inner.
///
/// granit reports a note that trails a marker line one level shallower than the node
/// our receiver attached it to, so writing it at the body indent cannot survive a
/// round trip — the note climbs a level every serialization and never settles
/// (libFuzzer `yaml_roundtrip` crash-ac5d9043, minimised to `? ? ? #~`). The hoisted
/// spelling above the outermost `?` is the fixed point at every marker depth, and is
/// where a single-marker line already puts the note, so this keeps both spellings
/// agreeing. crash-0e1c4378 needed this hoist as one of two fixes — its other half is
/// the note-binding side, `line_break_between` in the parser.
///
/// Taking only the first stack was not enough, and the tree says why: in
/// crash-f8525a9e the spine is three markers deep and carries two separate stacks —
/// `#` on the middle mapping and `!!"#~` on the innermost `~` key — while the fixed
/// point puts both above the outer `?`, in that order. Stopping at the first stack
/// hoisted one note and left the other a level deeper, so the emission settled only
/// on its second round.
fn hoist_marker_note(key: &mut CustomNode) -> Vec<crate::ast::Comment> {
    let CustomNode::Mapping {
        pairs, flow_style, ..
    } = key
    else {
        return Vec::new();
    };
    if *flow_style || pairs.is_empty() {
        return Vec::new();
    }
    // `IndexMap` never hands out `&mut K` (it would break the hash invariant), so
    // take the entry out, work on the owned key, and put it back where it was.
    let Some((mut first_key, first_value)) = pairs.shift_remove_index(0) else {
        return Vec::new();
    };
    let mut notes = take_leading_notes(&mut first_key);
    // Keep walking: a deeper marker can carry its own stack, and the reader
    // reports that one at this level too.
    notes.extend(hoist_marker_note(&mut first_key));
    pairs.shift_insert(0, first_key, first_value);
    notes
}

/// Split an empty container's notes between the one inline slot the reader
/// reports them back from, and the lines above the token.
///
/// Returns `(demoted, above)`. An inline `comment` already owns the slot, so then
/// every standalone note goes above; otherwise the LAST note takes the slot — the
/// position a re-read reports it from, which is what makes the shape a fixed point
/// after one emission.
fn empty_slot_notes(meta: &NodeMeta) -> (Option<&Comment>, &[Comment]) {
    let notes = meta.standalone_slice();
    if let Some(inline) = meta.inline_slot() {
        return (Some(inline), notes);
    }
    match notes.split_last() {
        Some((last, above)) => (Some(last), above),
        None => (None, notes),
    }
}

/// Whether a block container writes its own anchor/tag header line above the
/// body: the only shape where a note printed *under* that line cannot stay where
/// it was written, because the reader hands it to the tagged node itself. Flow
/// containers keep the header on their own line, and a container in value context
/// has the parent pre-emit the header, so neither is affected.
fn writes_block_header(meta: &NodeMeta, flow_style: bool, in_value_context: bool) -> bool {
    !flow_style && !in_value_context && (meta.anchor.is_some() || meta.tag.is_some())
}

/// Lift a block mapping's first-entry leading-note stack above the anchor/tag
/// header line [`Serializer::write_container_node`] writes for the container, and
/// return the body copy with those notes removed so they are not written twice.
///
/// `None` (and the untouched body) when there is nothing to lift or when the
/// container writes no header line. A container that carries notes of its own is
/// lifted too: `serialize_node_internal` has already written its stack above the
/// header, so the lifted one lands right after it, and that order is the measured
/// fixed point — `# a` + `# b` + `!tag` + body. The guard that used to refuse this
/// (`meta.standalone_slot().is_some()`) existed because a node held one leading
/// slot, so a second stack there would have overwritten the first and traded drift
/// for lost text; `decor.leading_comments` is a `Vec` now, so the premise is gone
/// and the lift is unconditional. The lift costs a map clone and only happens for a
/// tagged/anchored block container whose first entry carries notes.
///
/// The stack is taken off the whole marker spine, not just the first key: a header
/// line competes with every note the reader would report at this level, and leaving
/// one in the body puts it *below* the header, where the next round hoists it and
/// the emission only settles on its second pass. Measured on libFuzzer
/// `yaml_roundtrip` crash-e6551c75 (60 bytes) and crash-8f7085b0 (43 bytes,
/// `!3b55?b55?` + ` ? ?` + five note lines + `?` + `  ~: ~` + `:` + `  ~`), whose
/// first round writes the header then the five notes and whose fixed point is the
/// notes above the header.
///
/// The deferment recorded in `ROADMAP.md` was that this walk would drag notes out
/// of *nested* markers and break the pinned `- ?` / `# c` / `? a` / `: b` shape.
/// Withdrawn and re-measured: with the spine walk removed only the test above goes
/// red, and every other shape in the family (`notes_stack_above_a_tagged_containers_own_note`,
/// `a_note_under_a_tag_header_is_lifted_above_it`, `notes_above_an_empty_container_all_survive`,
/// `seq_item_value_with_standalone_comment_is_not_compacted`) passes both ways, so
/// the blast radius was inferred rather than observed and the fear was unfounded.
fn lift_first_entry_notes(
    pairs: &IndexMap<CustomNode, CustomNode>,
    meta: &NodeMeta,
    flow_style: bool,
    in_value_context: bool,
) -> (
    Option<IndexMap<CustomNode, CustomNode>>,
    Option<Vec<Comment>>,
) {
    if !writes_block_header(meta, flow_style, in_value_context) {
        return (None, None);
    }
    let Some((first_key, first_value)) = pairs.get_index(0) else {
        return (None, None);
    };
    // Borrow before allocating. Taking the spine needs an owned key and a copy of
    // the map, and every anchored/tagged block container would pay both — even the
    // overwhelming majority that carry no note anywhere on the spine. Measured:
    // cloning unconditionally before this check cost `serialize_small` 16.7%
    // (7.7 -> 9.3 µs), `serialize_block` 12.6% and `serialize_medium` 10.5% on the
    // CodSpeed gate, which is what caught it.
    if first_key.leading_comments().is_empty() && !spine_has_notes(first_key) {
        return (None, None);
    }
    let mut notes = first_key.leading_comments().to_vec();
    // `IndexMap` never hands out `&mut K`, so take the entry out, strip its
    // notes, and put it back where it was.
    let mut stripped = pairs.clone();
    stripped.shift_remove_index(0);
    // `strip_leading_comment` also drops the `blank_before` decoration that the
    // lifted lines now carry, so the body copy goes through it rather than
    // through a bare `remove_leading_comment`.
    let mut worked = strip_leading_comment(first_key);
    notes.extend(hoist_marker_note(&mut worked));
    if notes.is_empty() {
        return (None, None);
    }
    stripped.shift_insert(0, worked, first_value.clone());
    (Some(stripped), Some(notes))
}

/// Whether any note stack sits on the marker spine of `key`, without taking the
/// spine apart to find out — the read-only twin of [`hoist_marker_note`], used to
/// keep the caller's clone and allocation off the hot path when there is nothing
/// to lift.
fn spine_has_notes(key: &CustomNode) -> bool {
    let CustomNode::Mapping {
        pairs, flow_style, ..
    } = key
    else {
        return false;
    };
    if *flow_style || pairs.is_empty() {
        return false;
    }
    let (first_key, _) = match pairs.get_index(0) {
        Some(entry) => entry,
        None => return false,
    };
    !first_key.leading_comments().is_empty() || spine_has_notes(first_key)
}

/// The [`lift_first_entry_notes`] equivalent for a block sequence's first item.
fn lift_first_item_notes<'a>(
    items: &'a [CustomNode],
    meta: &NodeMeta,
    flow_style: bool,
    in_value_context: bool,
) -> (Option<Vec<CustomNode>>, Option<&'a [Comment]>) {
    if !writes_block_header(meta, flow_style, in_value_context) {
        return (None, None);
    }
    let Some(first_item) = items.first() else {
        return (None, None);
    };
    let notes = first_item.leading_comments();
    if notes.is_empty() {
        return (None, None);
    }
    let mut stripped = items.to_vec();
    stripped[0] = strip_leading_comment(first_item);
    (Some(stripped), Some(notes))
}

/// Clone of a node with every effective leading (standalone) comment removed
/// from *both* storage conventions (`decor.leading_comments` and a standalone
/// `comment`), so `serialize_node_internal` will not re-emit notes the
/// caller has already written above a `?` marker.
fn strip_leading_comment(node: &CustomNode) -> CustomNode {
    let mut stripped = node.clone();
    stripped.remove_leading_comment();
    match &mut stripped {
        CustomNode::Scalar { meta, .. }
        | CustomNode::Mapping { meta, .. }
        | CustomNode::Sequence { meta, .. }
        | CustomNode::Null { meta, .. } => {
            if let Some(decor) = &mut meta.decor {
                decor.leading_comments.clear();
            }
            if meta.comment.as_ref().is_some_and(|c| c.standalone) {
                meta.comment = None;
            }
            if meta.decor.as_ref().is_some_and(|d| !d.blank_before) {
                meta.decor = None;
            }
        }
        CustomNode::Alias { .. } => {}
    }
    stripped
}

/// Whether a plain scalar would break when emitted inside a flow
/// collection. `needs_double_quoted` already rejects leading indicators and
/// embedded `:`/`#`; the extra flow-context danger is an *embedded* `,` `[`
/// `]` `{` `}`, which terminates the plain token per YAML 12.3.3 and corrupts
/// the surrounding flow structure.
fn flow_plain_unsafe(value: &str) -> bool {
    value.contains(',')
        || value.contains('[')
        || value.contains(']')
        || value.contains('{')
        || value.contains('}')
}

/// Append `value` to `out` as a plain scalar, double-quoting it if required
/// and wrapping to `width` when it overflows `remaining`.
pub fn write_plain_scalar(out: &mut String, value: &str, remaining: usize, width: usize) {
    if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
        out.push_str(value);
        return;
    }
    if needs_double_quoted(value) {
        if value.contains('\\')
            && !value.contains('\n')
            && !value.chars().any(char::is_control)
            && !value.chars().any(is_yaml_noncharacter)
        {
            // granit-parser mishandles `\\<escape-letter>` inside double-quoted
            // scalars (e.g. `\\0` collapses to NUL). Single-quoted scalars keep
            // backslashes literal, so prefer them whenever the value contains a
            // backslash (and no newline, which single-quoted cannot represent,
            // and no control/noncharacter, which single-quoted cannot escape).
            write_single_quoted_scalar(out, value);
        } else {
            write_double_quoted_scalar(out, value);
        }
    } else if remaining > 0
        && value.len() > remaining
        // Folding a plain scalar across a line break is lossless only when every
        // break is a *single* space (a break folds back to exactly one space). If
        // the value contains a run of 2+ spaces or a tab, the wrap can break
        // beside it and leave trailing spaces that re-parse to a different number
        // of spaces, so the value is no longer idempotent (libFuzzer
        // `yaml_roundtrip` crash-9ee754bf: `…999  y|` folded to `…999 \n y|`).
        // Emit such values unwrapped (one long, lossless line) instead.
        && !value.contains("  ")
        && !value.contains('\t')
    {
        let safe_remaining = value.floor_char_boundary(remaining);
        match value[..safe_remaining].rfind(' ') {
            Some(split) => {
                out.push_str(&value[..split]);
                // Strip with YAML's separation set: the fold supplies exactly the
                // one space we drop here, but a Unicode `trim` would also eat an
                // NBSP at the edge — and NBSP is content, so the value would come
                // back shorter than it went in.
                let rest = value[split..].trim_matches(pyrs_schema::is_yaml_blank);
                if !rest.is_empty() {
                    // Continuation lines must be indented past the block
                    // indentation; aligning to the value's own column is always
                    // safe (and matches the value start column for inline
                    // contexts like `- item` or `key: value`).
                    let line_start = out.rfind('\n').map_or(0, |i| i + 1);
                    let cont_indent = (out.len() - line_start).max(2);
                    out.push('\n');
                    wrap_plain_scalar(out, rest, width, cont_indent);
                }
            }
            None => {
                // No whitespace to fold across: a mid-token line break would
                // re-parse as an inserted space and change the value. Emit the
                // whole value as one (potentially long) lossless line.
                out.push_str(value);
            }
        }
    } else {
        out.push_str(value);
    }
}

/// Wrap `value` to `width` with continuation lines indented `cont_indent`
/// columns. Used by [`write_plain_scalar`] when a value overflows the current
/// line.
pub fn wrap_plain_scalar(out: &mut String, value: &str, width: usize, cont_indent: usize) {
    let wrap_indent_str = " ".repeat(cont_indent);
    let mut remaining_rest = value;
    while !remaining_rest.is_empty() {
        out.push_str(&wrap_indent_str);
        if remaining_rest.len() <= width.saturating_sub(wrap_indent_str.len()) {
            out.push_str(remaining_rest);
            break;
        }
        let avail = width.saturating_sub(wrap_indent_str.len());
        if avail == 0 {
            out.push_str(remaining_rest);
            break;
        }
        let safe_avail = remaining_rest.floor_char_boundary(avail);
        match remaining_rest[..safe_avail].rfind(' ') {
            Some(split) => {
                out.push_str(&remaining_rest[..split]);
                out.push('\n');
                remaining_rest = remaining_rest[split..].trim_matches(pyrs_schema::is_yaml_blank);
            }
            None => {
                // No whitespace to fold across; another folded line would
                // re-parse as an inserted space and change the value. Emit the
                // remainder as one (potentially long) lossless line.
                out.push_str(remaining_rest);
                break;
            }
        }
    }
}

/// Whether a key or scalar value is a short, purely alphanumeric token that
/// can be emitted without any quoting or wrapping.
pub fn is_short_alphanumeric(value: &str) -> bool {
    value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Tag;
    use indexmap::IndexMap;
    use std::sync::Arc;

    #[test]
    fn a_block_value_that_is_only_line_breaks_stays_itself() {
        // libFuzzer `yaml_roundtrip`, 6-byte input `>+8\r\r#`. The reader hands back a
        // block scalar whose value is a single line break with `Keep` and an explicit
        // indent of 8; the writer emitted `>+8\n\n`, which re-reads as the same value
        // but as `Clip` with no indicator (measured), and the next round then wrote
        // `>\n\n` — a value of `""`. So it was the VALUE that drifted, not just the
        // header: `">+8\r\r#" -> ">+8\n\n" -> ">\n\n" -> ""`.
        //
        // Clip strips trailing breaks, so a body made of nothing but breaks needs
        // `Keep` to survive, and an indentation indicator has no content line to act
        // on, so it must be dropped — the two rules the empty body already applies,
        // which an all-break body walked straight past.
        for src in [">+8\r\r#", "|+8\r\r#"] {
            let node = crate::parser::parse(src, crate::parser::yaml::YamlSchema::Core).unwrap();
            let before = block_value(&node).to_string();
            let one = crate::serializer::to_yaml(&node);
            let again = crate::parser::parse(&one, crate::parser::yaml::YamlSchema::Core).unwrap();
            let two = crate::serializer::to_yaml(&again);
            assert_eq!(one, two, "not idempotent for {src:?}: {one:?} vs {two:?}");
            assert_eq!(
                block_value(&again),
                before,
                "value changed across a round for {src:?}: {one:?}"
            );
        }
    }

    #[test]
    fn clip_cannot_carry_an_all_break_block_body() {
        // The predicate itself, pinned: one trailing break is the case the existing
        // `ends_with("\n\n")` promotion misses, and a value with real content plus one
        // final break must keep Clip exactly as before (that is what `|` means).
        //
        // The writers pass the cheap `first_text_line.is_empty()` flag rather than
        // scanning, so the equivalence of the two definitions is asserted here for
        // every shape that matters - a future refactor that decouples them would fail
        // this test rather than silently re-opening the drift.
        let semantic = |value: &str| value.is_empty() || value.bytes().all(|b| b == b'\n');
        let cheap = |value: &str| {
            value
                .lines()
                .find(|l| !l.is_empty())
                .unwrap_or("")
                .is_empty()
        };
        for value in [
            "", "\n", "\n\n", "x", "x\n", "x\n\n", "\nx", "a\nb", "\n \n",
        ] {
            assert_eq!(
                semantic(value),
                cheap(value),
                "flag disagrees for {value:?}"
            );
        }

        assert_eq!(
            effective_chomping("\n", &Chomping::Clip, true),
            Chomping::Keep
        );
        assert_eq!(
            effective_chomping("\n\n", &Chomping::Clip, true),
            Chomping::Keep
        );
        assert_eq!(
            effective_chomping("x\n", &Chomping::Clip, false),
            Chomping::Clip
        );
        assert_eq!(
            effective_chomping("x\n\n", &Chomping::Clip, false),
            Chomping::Keep
        );
        assert_eq!(
            effective_chomping("", &Chomping::Keep, true),
            Chomping::Clip
        );
        assert_eq!(
            effective_chomping("x", &Chomping::Strip, false),
            Chomping::Strip
        );
    }

    fn block_value(node: &CustomNode) -> &str {
        match node {
            CustomNode::Scalar { value, .. } => value,
            other => panic!("expected a scalar document root, got {other:?}"),
        }
    }

    #[test]
    fn a_note_after_a_tag_only_value_settles_in_one_round() {
        // libFuzzer `yaml_roundtrip`, crash-cf49fe85: a block mapping's own inline note
        // was appended after the line its last pair ends on, and when that value is a
        // property-only scalar (a bare `!`) a reader reports the note as the *value's*
        // leading note. The container then loses it on re-read and the document settles
        // one round late — the idempotence assertion's exact shape:
        // `~: ! # -\n# -\n` -> `~:  # -\n  # -\n  ! \n` -> stable.
        //
        // The note is written where a reader will report it from, so the first emission
        // is the fixed point. Read from the committed seed so the test cannot drift from
        // the bytes that crashed; the extra shapes keep it from over-fitting to the
        // carriage returns.
        let artifact =
            include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-cf49fe85.seed");
        let seed = String::from_utf8(artifact.to_vec()).expect("seed is valid utf-8");
        for src in [
            seed.as_str(),
            "a: 1\nb: !\n# tail\n",
            "p: ! # own\n",
            "q: !x\n# z\n",
        ] {
            let node = crate::parser::parse(src, crate::parser::yaml::YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{src:?} does not parse: {e}"));
            let one = crate::serializer::to_yaml(&node);
            let re = crate::parser::parse(&one, crate::parser::yaml::YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{src:?} emitted unparseable {one:?}: {e}"));
            let again = crate::serializer::to_yaml(&re);
            assert_eq!(
                again, one,
                "{src:?} did not settle in one round: {one:?} -> {again:?}"
            );
        }

        // Both `-` notes survive the re-homing (note survival as a class is owned by
        // `tests/note_survival.rs`, which replays this seed too; this pins the shape).
        let node = crate::parser::parse(":\t! #-\r... #-", crate::parser::yaml::YamlSchema::Core)
            .expect("seed parses");
        let one = crate::serializer::to_yaml(&node);
        assert_eq!(
            one.matches("# -").count(),
            2,
            "a note was dropped or duplicated by the re-homing: {one:?}"
        );

        // The rule is about the line, not the tag spelling. Measured over the shape
        // `crash-cf49fe85` came from, `!-`, `!:`, `!x` and `!!str` all drifted while
        // only `!` settled — which is what the tag whitelist got wrong.
        for tag in ["!", "!-", "!:", "!x", "!!str"] {
            let src = format!(":\t{tag} #-\r... #-\n");
            let node = crate::parser::parse(&src, crate::parser::yaml::YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{src:?} does not parse: {e}"));
            let one = crate::serializer::to_yaml(&node);
            let again = crate::serializer::to_yaml(
                &crate::parser::parse(&one, crate::parser::yaml::YamlSchema::Core)
                    .unwrap_or_else(|e| panic!("{src:?} emitted unparseable {one:?}: {e}")),
            );
            assert_eq!(
                again, one,
                "tag {tag:?} did not settle in one round: {one:?} -> {again:?}"
            );
        }

        // The sorted writer emits in a different order than the source, so "the pair
        // that ends the body" is not "the last pair in insertion order": with the
        // tag-only value written first and `sort_keys` on, only the sorted view puts the
        // note where a reader will report it from. Taking the insertion-order last pair
        // here drifts.
        let src = "b: !\na: 1\n# tail\n";
        let opts = SerializeOptions {
            sort_keys: true,
            ..Default::default()
        };
        let node = crate::parser::parse(src, crate::parser::yaml::YamlSchema::Core)
            .unwrap_or_else(|e| panic!("{src:?} does not parse: {e}"));
        let one = crate::serializer::to_yaml_with_options(&node, &opts).expect("serialize");
        let re = crate::parser::parse(&one, crate::parser::yaml::YamlSchema::Core)
            .unwrap_or_else(|e| panic!("{src:?} emitted unparseable {one:?}: {e}"));
        let again = crate::serializer::to_yaml_with_options(&re, &opts).expect("serialize");
        assert_eq!(
            one.matches("# tail").count(),
            1,
            "the container note did not survive the sorted writer: {one:?}"
        );
        assert_eq!(
            again, one,
            "sorted emission did not settle: {one:?} -> {again:?}"
        );
    }

    /// An empty block body cannot carry a chomping indicator: granit reports the
    /// default chomping when it re-reads a header with no content, so `|+` written
    /// for an empty scalar drifts to `|` on the next round (libFuzzer
    /// `yaml_roundtrip` crash-89d81d99, crash-b5dcc38f). `effective_chomping`
    /// therefore pins Clip for that shape whatever the AST claims — the same
    /// "emit the re-readable form" rule both writers already apply to the
    /// indentation indicator.
    #[test]
    fn empty_block_body_writes_the_default_chomping() {
        for claimed in [Chomping::Keep, Chomping::Strip, Chomping::Clip] {
            assert_eq!(
                effective_chomping("", &claimed, true),
                Chomping::Clip,
                "an empty body must not advertise a chomping indicator"
            );
        }
        // Nothing else about the rule moves.
        assert_eq!(
            effective_chomping("x\n", &Chomping::Keep, false),
            Chomping::Keep
        );
        assert_eq!(
            effective_chomping("x\n\n", &Chomping::Clip, false),
            Chomping::Keep
        );
        assert_eq!(
            effective_chomping("x", &Chomping::Strip, false),
            Chomping::Strip
        );
    }

    /// The escaper's catch-all arm tested `is_control() || is_yaml_noncharacter()`.
    /// U+FEFF satisfies neither: it is a `Cf` format character, and the noncharacter
    /// mask `(c as u32) & 0xFFFE == 0xFFFE` is false for FEFF. It therefore fell
    /// through to the push-verbatim arm and a BOM was written raw even inside
    /// double quotes, where YAML does have an escape. The parser rejects a
    /// mid-document BOM on input, so this is reached through the edit API.
    #[test]
    fn double_quoted_scalar_escapes_a_bom() {
        let mut out = String::new();
        write_double_quoted_scalar(&mut out, "a\u{FEFF}b");
        assert_eq!(out, "\"a\\ufeffb\"", "BOM must be escaped, not written raw");
        assert!(
            !out.contains('\u{FEFF}'),
            "a raw BOM survived escaping: {out:?}"
        );

        // The escaped form must read back and re-emit byte-identically.
        let emitted = format!("key: {out}\n");
        let node = crate::parser::parse(&emitted, crate::parser::yaml::YamlSchema::Core)
            .expect("escaped BOM scalar must re-parse");
        assert_eq!(to_yaml(&node), emitted, "re-emit must be byte-identical");
    }

    /// libFuzzer `yaml_roundtrip` find: granit delivers double-quoted scalars
    /// already decoded, and the receiver's own unescape used to run a *second*
    /// decode over them — `a: "\\n"` (two literal chars `\` `n`) collapsed to
    /// LF, and the fuzz repro `!-# \\f"\t0:!` lost a backslash per round-trip.
    #[test]
    fn double_quoted_escapes_decode_exactly_once() {
        let node = crate::parser::parse(r#"a: "\\n""#, pyrs_schema::types::Schema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &node else {
            panic!()
        };
        let value = pairs.values().next().unwrap();
        assert!(
            matches!(value, CustomNode::Scalar { value, .. } if value.as_ref() == r"\n"),
            "double-decode regressed: {value:?}"
        );
        // And the full serialize loop is stable on the original fuzz input.
        let input = "!-# \\f\"\t0:!";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap();
        assert_eq!(once, to_yaml(&again), "first output: {once:?}");
    }

    /// libFuzzer `yaml_roundtrip` (12 bytes `&&&&:<LF>#&&&:&`): the raw anchor
    /// scanner took the `:` (followed by end-of-line, i.e. the value indicator)
    /// as name material and re-harvested phantom anchors from the `#&&&:&`
    /// comment, so the emitted `&&&&: v` re-parsed as anchor `&&&` plus an
    /// indicator — drifting one character per serialize round. Pinned here at
    /// the full parse -> to_yaml -> re-parse -> to_yaml loop the fuzz target
    /// exercises, not just the `anchor_name_before` unit.
    #[test]
    fn anchor_value_indicator_roundtrip_is_stable() {
        let input = "&&&&:\n#&&&:&";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("fmt output failed to re-parse: {e}\n---\n{once}\n---"));
        assert_eq!(
            once,
            to_yaml(&again),
            "serialization not idempotent: {once:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (crash-62bcff6f, 14 bytes `&f& !&&f&&&  :`): a
    /// mapping key carried an anchor and a tag and its scalar was the empty
    /// string. `write_scalar_for_key` emitted neither property and left the empty
    /// key bare, so ONCE was `: ~` (the empty key re-read as the null `~` scalar,
    /// dropping anchor + tag) and TWICE drifted to `~: ~`. The key emitter now
    /// writes the node's anchor/tag like a value and quotes an empty key, pinning
    /// idempotence for the crash shape plus adjacent simple-key / null-key /
    /// complex-key property shapes.
    #[test]
    fn key_anchor_tag_and_empty_key_roundtrip() {
        for input in [
            "&f& !&&f&&&  :",
            "&a !t k: v",
            "a: &k !t {}",
            "&q : val",
            "? &s !t k\n: v",
            "&a !tag anchored-key: value\nkey2: &b !!str anchored-value\n\"\": empty-string-key\n&c !type anchor-and-tag-key: v\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                to_yaml(&again),
                "key-metadata drift for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-d4ea8a23, 58 bytes): an empty block
    /// scalar carried an explicit indentation indicator (`|2`), which re-parse
    /// drops (there is no body to measure the indent against), so the emit
    /// drifted `|2` -> `|` each round. The block writers now omit the indicator
    /// for an empty body, making the empty shape idempotent. Pinned on the crash
    /// input plus bare empty literal / folded indicators followed by a dedent.
    /// (The raw BOM in the crash body is a separate concern: granit tolerates a
    /// BOM inside a block body, so it round-trips identically — only the
    /// indicator drifted.)
    #[test]
    fn empty_block_scalar_drops_indent_indicator_roundtrip() {
        for input in [
            "yaml: |2\nml: |2\n  >|2\n    MRRRRR\u{feff}st\n\n  \u{feff}st\n  lines\n \n\n",
            "k: |2\nz: 1\n",
            "k: >2\nz: 1\n",
            "a: |1\nb: 2\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                to_yaml(&again),
                "empty-block drift for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (42 bytes `&"X-::::…:<CR>ba`): an anchor name
    /// ending in `:` was emitted as a plain `&name ` token, so the trailing
    /// colon merged with the following space into a value indicator and each
    /// serialize round dropped exactly one colon (drift). The serializer now
    /// wraps unsafe names in quotes; the emitted form must re-parse and stay
    /// byte-identical across rounds.
    #[test]
    fn anchor_name_with_trailing_colon_roundtrips() {
        let input = format!("&\"X-{}{}ba", ":".repeat(35), "\r");
        let node = crate::parser::parse(&input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let twice =
            to_yaml(&crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap());
        let thrice =
            to_yaml(&crate::parser::parse(&twice, pyrs_schema::types::Schema::Core).unwrap());
        assert_eq!(
            once, twice,
            "not idempotent (round1->2): {once:?} vs {twice:?}"
        );
        assert_eq!(twice, thrice, "not idempotent (round2->3): {twice:?}");
        // And the anchor name is genuinely preserved, not silently emptied.
        assert!(once.contains("X-"), "anchor lost: {once:?}");
    }

    /// libFuzzer `yaml_roundtrip` (49 bytes, crash-c18cb1fd; the 43-byte
    /// crash-cfb3fa83 rides the same writer rules): a Clip-chomped block whose
    /// value carries a trailing blank line round-trips only with the Keep
    /// indicator — granit's Clip read strips trailing newlines — and a block
    /// scalar's inline comment must ride the header line, never a line of its
    /// own (it would be absorbed as block content). Both closures hold by
    /// construction now; assert the exact crash inputs re-serialize stable.
    #[test]
    fn block_scalar_emission_is_closed_under_reparse() {
        for input in [
            "yaml: |2\n  filineam  first\n  lineaml: |-ml: |2\n  ",
            "base: &b\n  x: 2  f,1\n:&hcild:\n  <<: *b\n  y: |  # inlEEE\n",
            "y: |2\n  a\n  b\n  ",
            "y: |\n  a\n  ",
            "y: |1\n a\n ",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            let twice = to_yaml(&again);
            assert_eq!(
                once, twice,
                "not idempotent for {input:?}: {once:?} vs {twice:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (14 bytes, crash-490c4beb): a folded scalar
    /// whose value carries a run of newlines between text lines. granit's
    /// folded read turns k blank lines into exactly k newlines, but the old
    /// line-splitting writer emitted one blank too few, so every run shrank a
    /// newline per round (4 -> 3 -> 2 -> ...). The fold-aware writer now closes
    /// every run length by construction; pin the crash input plus runs 1..=5.
    #[test]
    fn folded_newline_runs_roundtrip() {
        let input = ">\r,2C\u{f6bf}\r\r\r\r\r,";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
        assert_eq!(once, to_yaml(&again), "crash input drift: {once:?}");
        for m in 1..=5usize {
            for value in [
                format!("a{}b\n", "\n".repeat(m)),
                format!("{}b\n", "\n".repeat(m)),
                format!("a{} b\n", "\n".repeat(m)),
                format!("{} b\n", "\n".repeat(m)),
            ] {
                let node = CustomNode::Scalar {
                    value: Arc::from(value.as_str()),
                    style: ScalarStyle::Folded,
                    chomping: Chomping::Clip,
                    block_indent: None,
                    meta: Default::default(),
                };
                let once = to_yaml(&node);
                let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                    .unwrap_or_else(|e| panic!("{value:?}: must re-parse: {e}\n{once:?}"));
                assert_eq!(
                    once,
                    to_yaml(&again),
                    "folded run drifts for {value:?}: {once:?}"
                );
            }
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-b7a2285e): a folded scalar whose value
    /// has a MORE-INDENTED continuation line (leading tab/space) flanked by
    /// plain text lines. granit's fold rule keys on `leading_blank`: a
    /// more-indented line keeps the break *before* it and also suppresses the
    /// fold of the break *after* it, so an r-newline run touching a
    /// more-indented neighbour needs exactly r physical newlines — not r + 1.
    /// The writer previously padded every run to r + 1, so each round-trip
    /// gained a blank line (the ONCE/TWICE drift). Pin the crash input plus
    /// synthesized normal↔more-indented shapes and assert full value fidelity
    /// (re-parse preserves the scalar value), not just byte idempotency.
    #[test]
    fn folded_more_indented_continuation_roundtrip() {
        // crash-b7a2285e exact input.
        let crash = ">\n'\"\"\n\t<\t\t\t(\n(\t\t(\n([";
        for input in [crash, ">\na\n\tb\n", ">\n \tq\n r\n"] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(once, to_yaml(&again), "crash input drift: {once:?}");
            let scalar_value = |n: &CustomNode| match n {
                CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                other => panic!("expected scalar, got {other:?}"),
            };
            assert_eq!(
                scalar_value(&node),
                scalar_value(&again),
                "value not preserved for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-e432d4b8): a literal (`|`) block
    /// scalar whose first content line begins with a blank but a later line is
    /// shallower. The AST drops the source's explicit `|1` indicator, so the
    /// de-indented value (e.g. ` 1|l\n:t\n`) re-emitted with auto-detect lets
    /// granit read the deeper first line as the block indent and treat the
    /// shallower line as a dedent — the output no longer re-parses. The literal
    /// writer now forces an indentation indicator (mirroring the folded writer)
    /// so detection is skipped. Pin the crash input plus blank-first-line shapes
    /// with full value fidelity.
    #[test]
    fn literal_leading_blank_line_roundtrip() {
        for input in [
            "yaml: |1\n  1|l\n :t  ts  lins  lines\n\n",
            "k: |1\n   x\n  y\n",
            "k: |1\n  a\n  b\n c\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                to_yaml(&again),
                "literal drift for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-d0e84310): a block-style *empty*
    /// mapping/sequence used as a mapping value. An empty collection has no
    /// block form, so emitting `key:` then `  {}` on the next line re-reads as
    /// a FLOW mapping (flow_style flips), and the second round inlined it —
    /// ONCE `key:\n  {}` vs TWICE `key: {}` drift. Empty containers must always
    /// serialize inline. Pin the crash input plus empty-value shapes with tag.
    #[test]
    fn empty_container_value_is_inlined() {
        for input in [
            "base: &b\n  \n: 1xchi\n  \n!: 1xchild:\n  <<: *b\n: y: 2  # inline\n",
            "k:\n  {}\n",
            "k: !a {}\n",
            "? {}\n: !a {}\n",
            "a:\n  b: {}\n  c: []\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(once, to_yaml(&again), "drift for {input:?}: {once:?}");
        }
    }

    /// Print a unified diff when two YAML strings differ, then panic.
    macro_rules! assert_yaml_eq {
        ($expected:expr, $actual:expr) => {{
            let expected = $expected;
            let actual = $actual;
            let expected_str: &str = expected.as_ref();
            let actual_str: &str = actual.as_ref();
            if expected_str != actual_str {
                let diff = similar::TextDiff::from_lines(expected_str, actual_str);
                let out = diff
                    .unified_diff()
                    .context_radius(3)
                    .header("expected", "actual")
                    .to_string();
                panic!(
                    "YAML mismatch:\n\
                     expected ({} bytes):\n{expected_str:?}\n\
                     actual   ({} bytes):\n{actual_str:?}\n\n\
                     diff:\n{out}",
                    expected_str.len(),
                    actual_str.len()
                );
            }
        }};
    }

    /// Test fixture: core-schema parse with the standard options.
    fn parse_core(yaml: &str) -> CustomNode {
        crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap()
    }

    /// libFuzzer `yaml_roundtrip` crash-c5b367d3 (68 bytes, minimised to 23):
    /// `bg: !:` TAB / `<<: #*b` / `  ::` TAB `!)`. The merge key is consumed and its
    /// note re-homed onto the pair it contributes, so the writer emits that note as a
    /// line right after `bg: !:` — a tag-only line with no scalar text, which the scanner
    /// still considers unfinished. It therefore handed the note to `bg`'s value, and the
    /// document settled only on the second round. Closing the pending line (writing the
    /// value's own `~`) makes the first emission the fixed point, and unlike re-routing
    /// the note it keeps the note on the node the AST says owns it.
    #[test]
    fn a_note_after_a_tag_only_line_keeps_its_owner_and_settles() {
        let raw = include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-c5b367d3.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let node = parse_core(src);
        let one = to_yaml(&node);
        assert_eq!(
            one.matches("# *").count(),
            1,
            "the re-homed note appears exactly once: {one:?}"
        );
        assert!(
            one.contains("bg: !: ~"),
            "the tag-only line is closed before the note line: {one:?}"
        );
        let again = to_yaml(&parse_core(&one));
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }

    /// A note that granit attaches to a simple key rides that pair's line, and
    /// `write_mapping_pair` has a slot for it. The compact dash loop in
    /// `write_sequence_item` hand-rolls the same `key: value` line without that slot, so
    /// under a `- ` the note vanished on the *first* emission — silent data loss, not a
    /// relocation. Measured by holding everything else still: `a: !   # n` at the document
    /// root keeps its note, `- a: !   # n` dropped it; the tag-only value and the quoted
    /// key were both innocent.
    #[test]
    fn a_note_on_a_compact_dash_key_survives_the_first_emission() {
        for src in [
            "- a: !   # n\n",
            "- \"a\": !   # n\n",
            "- a: !   # ! - *:\n",
        ] {
            let node = parse_core(src);
            let one = to_yaml(&node);
            assert!(
                one.contains("# "),
                "{src:?} lost the key's note on the first emission: {one:?}"
            );
            let again = to_yaml(&parse_core(&one));
            assert_eq!(
                again, one,
                "not a fixed point in one round: {one:?} -> {again:?}"
            );
        }
    }

    /// Each pair of a multi-pair compact item owns its own line, so each key note has to
    /// land on its own line — and a value that carries a note of its own keeps the slot,
    /// because one line has exactly one trailing note.
    #[test]
    fn each_compact_dash_line_keeps_its_own_key_note() {
        let src = "- a: !   # one\n  b: !   # two\n";
        let one = to_yaml(&parse_core(src));
        assert_eq!(one.matches("# one").count(), 1, "first note: {one:?}");
        assert_eq!(one.matches("# two").count(), 1, "second note: {one:?}");
        let again = to_yaml(&parse_core(&one));
        assert_eq!(again, one, "not a fixed point: {one:?} -> {again:?}");
    }

    /// The same loop also ignored the item's own leading stack: `  -` / `# z` / `a: 1`
    /// records the note on the item mapping, and the compact branch wrote only the pair,
    /// emitting `- a: 1` and dropping the note. Only the dash line can hold it, so it is
    /// hoisted above the dash — which is also where the sibling spelling (`- # z`) already
    /// puts it, so both inputs land on the same emission.
    #[test]
    fn a_leading_note_on_a_compact_dash_item_survives() {
        for src in [
            "-\n  # z\n  a: 1\n",
            "- # z\n  a: 1\n",
            "-\n  # z1\n  # z2\n  a: 1\n",
        ] {
            let one = to_yaml(&parse_core(src));
            assert!(
                one.contains("# z"),
                "{src:?} dropped the item's note: {one:?}"
            );
            let again = to_yaml(&parse_core(&one));
            assert_eq!(again, one, "not a fixed point: {one:?} -> {again:?}");
        }
    }

    /// A *later* pair can own a leading stack (`- a: 1` / `# z` / `b: 2` measures as
    /// `root[0][1].key.leading`), and its line begins at the pair indent, so the notes go on
    /// their own lines above it. The old loop wrote the indent and the key and skipped the
    /// stack, so the note was gone from the first emission.
    #[test]
    fn a_note_above_a_later_compact_dash_key_survives() {
        for src in [
            "- a: 1\n  # z\n  b: 2\n",
            "- a: 1\n  # z1\n  # z2\n  b: 2\n",
        ] {
            let one = to_yaml(&parse_core(src));
            assert_eq!(
                one, src,
                "the stack was dropped or the shape moved: {one:?}"
            );
            let again = to_yaml(&parse_core(&one));
            assert_eq!(again, one, "not a fixed point: {one:?} -> {again:?}");
        }
    }

    /// libFuzzer `yaml_roundtrip` crash-55c199ef (25 bytes, kept as found: the input no
    /// longer drifts, so `tmin` has nothing to take). `-` TAB `?"."` `:` TAB `!` CR ` #`
    /// CR CR `... #` TAB `! - *:` round-trips into a pair line whose note the reader hands
    /// to the **key**, and the compact dash loop had no slot for that, so the second
    /// emission dropped the note entirely — the class this fix closes.
    #[test]
    fn a_note_on_a_dash_key_from_the_ci_artifact_survives_one_round() {
        let raw = include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-55c199ef.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = to_yaml(&parse_core(src));
        assert!(
            one.contains("# ! - *:"),
            "the key's note vanished on the first emission: {one:?}"
        );
        let again = to_yaml(&parse_core(&one));
        assert_eq!(again, one, "not a fixed point: {one:?} -> {again:?}");
    }

    /// The value's own note has priority for the line's trailing slot; the key's note must
    /// not duplicate it into two notes on one line.
    #[test]
    fn a_value_note_still_owns_the_compact_dash_line_slot() {
        let src = "- a: 1  # own\n";
        let one = to_yaml(&parse_core(src));
        assert_eq!(one, src, "value-owned slot changed shape: {one:?}");
    }

    /// libFuzzer `yaml_roundtrip` crash-22cb5f67 (15 bytes): a `#` that is only *text*
    /// inside a quoted key used to refuse the note slot, stranding the document's own
    /// inline note on a line the reader hands to the next node, so the emission needed two
    /// rounds. The pair line's slot is the right home here — `"+#": !-   # -o` re-reads
    /// with the note on the key and settles at once — and the assertion is pinned exactly
    /// because a routed variant ("move the note under the value") also settles while
    /// quietly changing its owner; see `a_containers_inline_note_after_a_text_less_value_
    /// settles_at_once` in `parser/mod.rs` for the same guard on the unquoted key.
    #[test]
    fn a_quoted_hash_key_settles_the_containers_note_at_once() {
        let raw = include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-22cb5f67.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = to_yaml(&parse_core(src));
        assert_eq!(
            one, "\"+#\": !-   # -o\n",
            "the container's note rides the pair line, which the quoted `#` must not refuse"
        );
        assert_eq!(
            one.matches('#').count(),
            2,
            "the quoted `#` in the key plus exactly one note, no duplication: {one:?}"
        );
        assert_eq!(
            to_yaml(&parse_core(&one)),
            one,
            "one emission is enough to reach the fixed point"
        );
    }

    /// A note that trails a *simple key* rode whatever line had just been
    /// written — which stopped being the pair line as soon as the value moved down
    /// to take a leading note of its own. On a tag-only value, a trailing note on
    /// that line re-reads as the *value's* leading note, so the note changed owner
    /// between rounds and the emission never settled: `b:\n  # ~\n  !   # &` then
    /// `b:\n  # ~\n  # &\n  ! ` (libFuzzer `yaml_roundtrip` crash-1b01ac3f, 93 bytes
    /// minimised to 11). The key's note has to stay on the `key:` line.
    #[test]
    fn a_keys_note_stays_on_the_key_line_when_the_value_moves_down() {
        let raw = include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-1b01ac3f.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = to_yaml(&parse_core(src));
        assert_eq!(
            one, "b:  # &\n  # ~\n  ! \n",
            "the key's note belongs on the `key:` line, the value's own note below it"
        );
        assert_eq!(
            to_yaml(&parse_core(&one)),
            one,
            "one emission must already be the fixed point; first round was {one:?}"
        );
    }

    /// The predicate behind that fix, on its own terms: quoting and the
    /// whitespace rule decide whether a `#` on a written line is a marker.
    #[test]
    fn comment_marker_scan_respects_quoting() {
        assert!(line_has_comment_marker(b"a: 1  # note"));
        assert!(line_has_comment_marker(b"# whole line"));
        assert!(line_has_comment_marker(b"a: \"x#y\"  # real"));
        assert!(!line_has_comment_marker(b"\"+#\": !- "));
        assert!(!line_has_comment_marker(b"a: \"x#y\""));
        assert!(!line_has_comment_marker(b"a: 'x#y'"));
        assert!(!line_has_comment_marker(b"a: \"esc\\\"#still\""));
        assert!(!line_has_comment_marker(b"a: 'doubled''#still'"));
        assert!(
            !line_has_comment_marker(b"!-#tail"),
            "a `#` with no space before it is text, not a marker"
        );
    }

    /// libFuzzer `yaml_roundtrip` crash-e6551c75 (60 bytes) and crash-8f7085b0
    /// (43 bytes): the notes riding the marker spine inside a tagged container have
    /// to be lifted *with* the first key's own stack, or the header line is written
    /// first and the notes land under it — where the reader reports them as the
    /// tagged node's leading comments, so round 2 moves them above the header.
    /// Asserted exactly because the ordering *is* the fix.
    #[test]
    fn every_spine_note_clears_a_tagged_containers_header_line() {
        let cases = [
            (
                include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-e6551c75.seed")
                    as &[u8],
                "# yrrrrrrrr!yrrrr%3c57\n# yrrrrrrrr!yrrrr%3c57%c\n!5j4? \n?\n  ~: ~\n:\n  ~\n",
            ),
            (
                include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-8f7085b0.seed")
                    as &[u8],
                "# ycn-r\n# ?\n# y-r\n# r\n# yrrr\n!3b55?b55? \n?\n  ~: ~\n:\n  ~\n",
            ),
        ];
        for (raw, expected) in cases {
            let src = std::str::from_utf8(raw).expect("seed is utf-8");
            let one = to_yaml(&parse_core(src));
            assert_eq!(one, expected, "notes must sit above the header at once");
            assert_eq!(
                to_yaml(&parse_core(&one)),
                one,
                "one emission is enough to reach the fixed point"
            );
        }
    }

    #[test]
    fn test_pair_helper_matches_full_serialize() {
        let yaml = "a: 1\nb:\n  c: 2\n";
        let ast = parse_core(yaml);
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!()
        };
        for (k, v) in pairs.iter() {
            s.write_mapping_pair(k, v, 0, 0).unwrap();
        }
        assert_yaml_eq!(&full, &s.output);
    }

    #[test]
    fn test_item_helper_matches_full_serialize() {
        // mixed items: simple scalar + compact mapping (multi-key alignment) + block container
        let yaml = "- a\n- b: c\n  d: 1\n- - 1\n  - 2\n";
        let ast = parse_core(yaml);
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        for item in items.iter() {
            s.write_sequence_item(item, 0, 0).unwrap();
        }
        assert_yaml_eq!(&full, &s.output);
    }

    #[test]
    fn test_item_helper_preserves_compact_dash() {
        let yaml = "- host: a\n";
        let ast = parse_core(yaml);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        s.write_sequence_item(&items[0], 0, 0).unwrap();
        assert_yaml_eq!(&s.output, "- host: a\n"); // P2: dash prefix must survive
    }

    #[test]
    fn test_serialize_plain_scalar() {
        let node = CustomNode::plain_scalar("hello");
        assert_yaml_eq!(to_yaml(&node), "hello\n");
    }

    #[test]
    fn test_serialize_scalar_with_comment() {
        let node = CustomNode::Scalar {
            value: Arc::from("value"),
            style: ScalarStyle::Plain,
            meta: NodeMeta {
                comment: Some(crate::ast::Comment {
                    text: Arc::from("a comment"),
                    standalone: false,
                }),
                ..Default::default()
            },
            chomping: Chomping::Clip,
            block_indent: None,
        };
        assert_yaml_eq!(to_yaml(&node), "value  # a comment\n");
    }

    #[test]
    fn test_single_quoted_control_char_downgrades_to_double_quoted() {
        // Regression for the dialect re-parse fuzz (from_json5): a JSON5
        // single-quoted string holding a control char keeps SingleQuoted
        // style for fidelity, but `'\0'` is not re-parseable YAML. The
        // serializer must downgrade such values to double-quoted, which
        // escapes them losslessly.
        for value in ["\0", "a\u{1f}b", "line\nbreak", "\u{fffe}"] {
            let node = CustomNode::Scalar {
                value: Arc::from(value),
                style: ScalarStyle::SingleQuoted,
                meta: NodeMeta::default(),
                chomping: Chomping::Clip,
                block_indent: None,
            };
            let dumped = to_yaml(&node);
            let reparsed = parse_core(&dumped);
            let CustomNode::Scalar { value: got, .. } = reparsed else {
                panic!("re-parse of {dumped:?} did not yield a scalar")
            };
            assert_eq!(&*got, value, "round-trip via {dumped:?}");
        }
    }

    #[test]
    fn test_serialize_scalar_with_tag() {
        let node = CustomNode::Scalar {
            value: Arc::from("42"),
            style: ScalarStyle::Plain,
            meta: NodeMeta {
                tag: Some(Tag::primary("int")),
                ..Default::default()
            },
            chomping: Chomping::Clip,
            block_indent: None,
        };
        assert_yaml_eq!(to_yaml(&node), "!!int 42\n");
    }

    #[test]
    fn test_serialize_mapping() {
        let key = CustomNode::plain_scalar("key");
        let value = CustomNode::plain_scalar("value");

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        assert_yaml_eq!(to_yaml(&node), "key: value\n");
    }

    #[test]
    fn test_serialize_complex_key() {
        let key = CustomNode::Sequence {
            items: vec![
                CustomNode::plain_scalar("key1"),
                CustomNode::plain_scalar("key2"),
            ],
            flow_style: false,
            meta: Default::default(),
        };
        let value = CustomNode::plain_scalar("value");

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        let output = to_yaml(&node);
        assert!(output.contains("?\n"));
        // The explicit-key spelling must re-parse.
        let reparsed = crate::parser::parse_with_options(
            &output,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        );
        assert!(reparsed.is_ok(), "{output:?} -> {reparsed:?}");
    }

    /// Serialize `node` and assert the emitted text re-parses; returns the
    /// text so callers can also pin the exact spelling.
    fn assert_reparses(node: &CustomNode) -> String {
        let out = to_yaml(node);
        let reparsed = crate::parser::parse_with_options(
            &out,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        );
        assert!(
            reparsed.is_ok(),
            "serialized output failed to re-parse: {out:?} -> {reparsed:?}"
        );
        out
    }

    fn literal_scalar(value: &str) -> CustomNode {
        let mut node = CustomNode::quoted_scalar(value);
        node.set_scalar_style(ScalarStyle::Literal);
        node
    }

    /// A literal scalar pinned to an explicit indentation indicator, as
    /// `|2` in the source would produce.
    fn literal_scalar_with_indent(value: &str, indent: u8) -> CustomNode {
        let mut node = literal_scalar(value);
        if let CustomNode::Scalar { block_indent, .. } = &mut node {
            *block_indent = Some(indent);
        }
        node
    }

    /// The `|2` header has to survive a round trip together with the body
    /// indentation it pins. Without it the reader re-detects the content
    /// indent from the *first* body line, so a body whose first line is
    /// deeper than the rest (`4RWC.yaml`) would come back as a dedent.
    ///
    /// The indicator is relative to the line carrying the header, so a
    /// top-level `yaml: |2` puts the body at column 2 — which then leaves the
    /// second line's own 2 leading spaces as content.
    #[test]
    fn explicit_block_indent_indicator_round_trips() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("yaml"),
            literal_scalar_with_indent("[1, 2, 3]  \n  \n", 2),
        );
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("yaml: |2\n  [1, 2, 3]  \n    \n", &out);

        // The indicator is the only thing that makes the shallower second
        // line part of the body rather than a dedent, so the re-parsed value
        // must be byte-identical to the original.
        let reparsed = crate::parser::parse_with_options(
            &out,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        )
        .expect("re-parse");
        let CustomNode::Mapping { pairs, .. } = &reparsed else {
            panic!("unexpected shape: {reparsed:?}");
        };
        let Some(CustomNode::Scalar { value, .. }) = pairs.get(&CustomNode::plain_scalar("yaml"))
        else {
            panic!("missing `yaml` key: {out:?}");
        };
        assert_eq!(value.as_ref(), "[1, 2, 3]  \n  \n");
    }

    /// Chomping precedes indentation in a block header (`c-b-block-header`),
    /// so the two indicators have to be emitted in that order.
    #[test]
    fn block_indent_indicator_follows_chomping_indicator() {
        let mut node = literal_scalar_with_indent("x\ny", 2);
        if let CustomNode::Scalar { chomping, .. } = &mut node {
            *chomping = Chomping::Strip;
        }
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), node);
        let mapping = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&mapping);
        assert_yaml_eq!("a: |-2\n  x\n  y\n", &out);
    }

    /// The pinned number must drive the body indent, not `indent_size`: at
    /// `a:\n  b: |-4` the header sits on the `b:` line (column 2), so the body
    /// lands at column 6. Deriving it from `indent_size` would emit a `|-4`
    /// header over a column-4 body, and the reader would believe the header.
    #[test]
    fn explicit_block_indent_is_relative_to_parent_line() {
        let mut inner = IndexMap::new();
        inner.insert(
            CustomNode::plain_scalar("b"),
            literal_scalar_with_indent("x\ny", 4),
        );
        let mut outer = IndexMap::new();
        outer.insert(
            CustomNode::plain_scalar("a"),
            CustomNode::Mapping {
                pairs: inner,
                flow_style: false,
                meta: Default::default(),
            },
        );
        let node = CustomNode::Mapping {
            pairs: outer,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("a:\n  b: |4\n      x\n      y\n", &out);
    }

    /// No indicator in, no indicator out: the writer must not invent one
    /// (it would pin a value the source never expressed).
    #[test]
    fn block_scalar_without_indicator_keeps_derived_indent() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("b: |\n  x\n  y\n", &out);
    }

    #[test]
    fn nested_literal_block_scalar_body_follows_parent_indent() {
        let mut inner = IndexMap::new();
        inner.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let mut outer = IndexMap::new();
        outer.insert(
            CustomNode::plain_scalar("a"),
            CustomNode::Mapping {
                pairs: inner,
                flow_style: false,
                meta: Default::default(),
            },
        );
        let node = CustomNode::Mapping {
            pairs: outer,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("a:\n  b: |\n    x\n    y\n", &out);
    }

    #[test]
    fn seq_item_literal_block_scalar_body_hangs_off_dash() {
        let seq = CustomNode::Sequence {
            items: vec![literal_scalar("x\ny")],
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_yaml_eq!("- |\n  x\n  y\n", &out);
    }

    #[test]
    fn compact_dash_literal_block_scalar_body_indents_past_key() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::Mapping {
                pairs,
                flow_style: false,
                meta: Default::default(),
            }],
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_yaml_eq!("- b: |\n    x\n    y\n", &out);
    }

    #[test]
    fn literal_block_scalar_in_flow_demotes_to_double_quoted() {
        let seq = CustomNode::Sequence {
            items: vec![literal_scalar("x")],
            flow_style: true,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "[\"x\"]\n");
    }

    #[test]
    fn plain_scalar_with_edge_spaces_is_quoted() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("k"),
            CustomNode::plain_scalar(" %"),
        );
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_eq!(&out, "k: \" %\"\n");
    }

    #[test]
    fn plain_scalar_with_comma_in_flow_is_quoted() {
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::plain_scalar("a,b")],
            flow_style: true,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "[\"a,b\"]\n");
    }

    #[test]
    fn tagged_empty_block_container_puts_header_on_braces_line() {
        let seq = CustomNode::Sequence {
            items: vec![],
            flow_style: false,
            meta: NodeMeta {
                tag: Some(Tag {
                    handle: "!".into(),
                    suffix: "a".into(),
                }),
                ..Default::default()
            },
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "!a []\n");
    }

    #[test]
    fn seq_item_value_with_standalone_comment_is_not_compacted() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("0"),
            CustomNode::Null {
                meta: NodeMeta {
                    comment: Some(crate::ast::Comment {
                        text: "note".into(),
                        standalone: true,
                    }),
                    ..Default::default()
                },
            },
        );
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::Mapping {
                pairs,
                flow_style: false,
                meta: Default::default(),
            }],
            flow_style: false,
            meta: Default::default(),
        };
        assert_reparses(&seq);
    }

    #[test]
    fn test_serialize_max_depth_exceeded() {
        let inner = CustomNode::plain_scalar("leaf");
        let mut current = inner;
        for _ in 0..200 {
            let mut m = IndexMap::<CustomNode, CustomNode>::new();
            m.insert(CustomNode::plain_scalar("a"), current);
            current = CustomNode::plain_mapping(m);
        }
        let options = SerializeOptions {
            indent_size: 2,
            explicit_start: false,
            explicit_end: false,
            sort_keys: false,
            max_depth: 50,
            ..Default::default()
        };
        let result = to_yaml_with_options(&current, &options);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SerializeError::MaxDepthExceeded(DepthError(50)))
        ));
    }

    #[test]
    fn test_serialize_indent_mapping() {
        let mut inner = IndexMap::new();
        inner.insert(CustomNode::plain_scalar("b"), CustomNode::plain_scalar("1"));
        let inner_map = CustomNode::Mapping {
            pairs: inner,
            flow_style: false,
            meta: Default::default(),
        };
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), inner_map);
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_mapping: 4,
            ..Default::default()
        };
        assert_eq!(
            to_yaml_with_options(&node, &options).unwrap(),
            "a:\n    b: 1\n"
        );
    }

    #[test]
    fn test_serialize_indent_sequence() {
        let inner_seq = CustomNode::Sequence {
            items: vec![CustomNode::plain_scalar("1"), CustomNode::plain_scalar("2")],
            flow_style: false,
            meta: Default::default(),
        };
        let node = CustomNode::Sequence {
            items: vec![inner_seq],
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_sequence: 4,
            ..Default::default()
        };
        assert_eq!(
            to_yaml_with_options(&node, &options).unwrap(),
            "- \n    - 1\n    - 2\n"
        );
    }

    #[test]
    fn test_serialize_indent_offset() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_offset: 2,
            ..Default::default()
        };
        assert_yaml_eq!(to_yaml_with_options(&node, &options).unwrap(), "  a: 1\n");
    }

    #[test]
    fn test_standalone_comment_before_simple_key() {
        let yaml = "a: 1\n# c1\nb: 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_yaml_eq!(crate::serializer::to_yaml(&ast), "a: 1\n# c1\nb: 2\n");
    }

    #[test]
    fn test_standalone_comment_before_nested_key() {
        let yaml = "top:\n  x: 1\n  # c2\n  y: 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_yaml_eq!(
            crate::serializer::to_yaml(&ast),
            "top:\n  x: 1\n  # c2\n  y: 2\n"
        );
    }

    /// A note written *under* a container's own anchor/tag header line re-reads as
    /// that container's leading comment, so one emission step never reached a fixed
    /// point: the writer put the tag line first, and the next round moved the note
    /// above it (libFuzzer `yaml_roundtrip` crash-77a8039b, 28 bytes). Such a note is
    /// now lifted above the header — the one line the reader hands it back from.
    #[test]
    fn a_note_under_a_tag_header_is_lifted_above_it() {
        let yaml = "!5b4?\n ? \n#yrrrrrrrrrrrr%3c ";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        let one = crate::serializer::to_yaml(&ast);
        assert!(
            one.starts_with("# yrrrrrrrrrrrr%3c\n!5b4? \n"),
            "the note must be lifted above the tag line: {one:?}"
        );
        let again = crate::parser::parse_with_options(
            &one,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_eq!(
            crate::serializer::to_yaml(&again),
            one,
            "one emission step must reach the fixed point: {one:?}"
        );
    }

    /// Both note stacks go above a block container's tag header, and the first
    /// emission is already the fixed point. This shape used to be pinned as the
    /// deliberate *cost* of a guard, because a node owned one leading slot and a
    /// second stack there would have overwritten the first; the slot is a `Vec`
    /// now, so refusing the lift only made the emission take an extra round to
    /// arrive at the same spelling (measured before the change: round 0 kept
    /// `# b` under the header, round 1 hoisted it, round 2 was the fixed point).
    /// Re-derived rather than quietly kept, and pinned in both directions: the
    /// order the reader agrees on, and that no note is lost or duplicated.
    #[test]
    fn notes_stack_above_a_tagged_containers_own_note() {
        let yaml = "# a\n!5b4?\n?\n# b\nk: v\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        let one = crate::serializer::to_yaml(&ast);
        assert_eq!(
            one.lines().filter(|line| line.starts_with("# ")).count(),
            2,
            "both stacks survive above the header: {one:?}"
        );
        assert_eq!(one, "# a\n# b\n!5b4? \n~: ~\nk: v\n", "and in source order");
        let again = crate::parser::parse_with_options(
            &one,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_eq!(
            crate::serializer::to_yaml(&again),
            one,
            "one emission step must reach the fixed point: {one:?}"
        );
    }

    /// An empty container offers one inline slot, so a stack of notes above it used
    /// to keep only one: the comment-only document `# d1` + `# d2` came back as
    /// `{}  # d1` with the rest gone. The earlier notes are now written above the
    /// token — which is also where a reader reports them from — so a single emission
    /// is already stable.
    #[test]
    fn notes_above_an_empty_container_all_survive() {
        let mut node = CustomNode::Mapping {
            pairs: IndexMap::new(),
            flow_style: false,
            meta: NodeMeta::default(),
        };
        for text in ["d1", "d2", "d3"] {
            node.push_leading_comment(crate::ast::Comment {
                text: text.into(),
                standalone: true,
            });
        }
        let one = to_yaml(&node);
        assert_eq!(one, "# d1\n# d2\n{}  # d3\n", "{one:?}");
        let again = to_yaml(
            &crate::parser::parse(&one, crate::parser::yaml::YamlSchema::Core)
                .expect("emitted document re-parses"),
        );
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
    }

    #[test]
    fn test_wrap_plain_scalar_does_not_split_multibyte() {
        // A value longer than the wrap width with a 4-byte char straddling the
        // continuation slice (byte 78). `wrap_plain_scalar` must floor the
        // slice to a char boundary instead of panicking, and must never split
        // inside the multi-byte character.
        let value =
            format!("x {}y", "y".repeat(75)) + &char::from_u32(0x10a09b).unwrap().to_string();
        let mut out = String::new();
        wrap_plain_scalar(&mut out, &value, 80, 2);
        assert!(out.contains('\u{10a09b}'), "multibyte char lost: {out:?}");
        assert!(out.contains("yyy"), "wrapped output incomplete: {out:?}");
    }
}
