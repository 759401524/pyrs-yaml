#[cfg(test)]
#[allow(unused_doc_comments)]
mod tests {
    use crate::ast::proptest_strategies::*;
    use crate::parser::parse_with_options;
    use crate::parser::yaml::Schema;
    use crate::serializer::{SerializeOptions, to_yaml, to_yaml_with_options};
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;
    use proptest::prelude::*;

    /// When a round-trip comparison fails, print a unified diff of the two
    /// YAML serializations so the developer can see exactly what changed.
    fn yaml_diff(a: &str, b: &str) -> String {
        let diff = similar::TextDiff::from_lines(a, b);
        diff.unified_diff()
            .context_radius(3)
            .header("original.yaml", "re-parsed.yaml")
            .to_string()
    }

    /// The normalizations a serialize → parse cycle is entitled to perform, each named.
    ///
    /// Seven properties used to assert only inside `if let Some(parsed) = try_roundtrip(..)`, a
    /// helper that *declined* 3433 of 4000 drawn trees. Declining is not tolerating: the filter
    /// states nothing about the majority it drops, and it lets a new difference join the accepted
    /// set quietly. The list below is closed and every entry has a reason; anything the cycle
    /// changes that the list cannot name is reported by [`roundtrip_residuals`] as a failure, which
    /// is what caps the whitelist (`property-tier:conditional-assert`'s exit criterion).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Normalization {
        /// A `Null` leaf has no spelling in granit's AST: `null`, `~` and an empty token all come
        /// back as a Plain scalar, so the variant itself cannot survive the cycle.
        NullToken,
        /// Plain text that is not legal as a plain scalar (a leading `-`, a `#` mid-token, edge
        /// spaces) is quoted by the writer. The content is identical; only the spelling moved.
        PlainRequoted,
        /// A block scalar re-reads in another style, or with the chomping / indentation indicator
        /// corrected: `effective_chomping` rewrites only claims no header can spell, and the body
        /// is what survives - which is the whole claim of ledger (cz) and (da).
        BlockRestyled,
        /// An empty block container has no block spelling, so `{}` / `[]` is what reads back.
        EmptyContainerFlow,
        /// A flow container whose subtree carries a note is written on lines, because a note ends
        /// its line: the container then re-reads as block. The layout is forced by the engine's own
        /// documented rule (`two_notes_on_one_flow_member_send_the_container_to_block`), and every
        /// entry inside survives - which is what the precondition below checks.
        NoteForcesBlockLayout,
        /// A block container that sits inside a flow collection has no block spelling there: `{a:
        /// {b: c}}` cannot carry a body, so the writer inlines it and the reader reports flow. The
        /// style bit is unavoidable at that position - the same reason (and the same kind of claim)
        /// as the empty container's `{}` above - while every entry inside keeps its value and order.
        InlineContainerFlow,
        /// A comment's padding around the `#` marker, and which node the reader files a standalone
        /// note on, are the reader's normalisation: no reference library recovers a padded text,
        /// and a line above a mapping's first key belongs to the mapping (measured in (db)).
        NoteRehomed,
    }

    /// Whether a Plain token reads as null under the schema, as the reader does.
    fn null_token(value: &str) -> bool {
        value.is_empty() || value == "~" || value.eq_ignore_ascii_case("null")
    }

    fn variant_name(node: &crate::ast::CustomNode) -> &'static str {
        use crate::ast::CustomNode as N;
        match node {
            N::Scalar { .. } => "Scalar",
            N::Mapping { .. } => "Mapping",
            N::Sequence { .. } => "Sequence",
            N::Null { .. } => "Null",
            N::Alias { .. } => "Alias",
        }
    }

    fn excerpt(value: &str) -> String {
        let cut: String = value.chars().take(40).collect();
        format!("{cut:?}")
    }

    /// Whether any node in this subtree carries a note: the precondition that makes a
    /// flow-to-block relayout the engine's documented rule rather than an unexplained change.
    fn carries_note(node: &crate::ast::CustomNode) -> bool {
        use crate::ast::CustomNode as N;
        if node.has_notes() {
            return true;
        }
        match node {
            N::Mapping { pairs, .. } => pairs
                .iter()
                .any(|(k, v)| carries_note(k) || carries_note(v)),
            N::Sequence { items, .. } => items.iter().any(carries_note),
            _ => false,
        }
    }

    /// Pair two trees by position, naming every difference the list explains and reporting the
    /// rest. Note slots are not compared here - they are [`note_reach`]'s business, because a note
    /// that survives on a different node is not data loss while a note that never reaches the
    /// document is.
    fn diff_trees(
        a: &crate::ast::CustomNode,
        b: &crate::ast::CustomNode,
        path: &str,
        in_flow: bool,
        labels: &mut std::collections::BTreeSet<Normalization>,
        residual: &mut Vec<String>,
    ) {
        use crate::ast::{Chomping, CustomNode as N, ScalarStyle};
        use Normalization::*;
        if a.anchor() != b.anchor() {
            residual.push(format!(
                "anchor at {path}: {:?} -> {:?}",
                a.anchor(),
                b.anchor()
            ));
        }
        if a.tag() != b.tag() {
            residual.push(format!("tag at {path}: {:?} -> {:?}", a.tag(), b.tag()));
        }
        if a.comment() != b.comment() {
            labels.insert(NoteRehomed);
        }
        match (a, b) {
            (
                N::Scalar {
                    value: va,
                    style: sa,
                    chomping: ca,
                    block_indent: ia,
                    ..
                },
                N::Scalar {
                    value: vb,
                    style: sb,
                    chomping: cb,
                    block_indent: ib,
                    ..
                },
            ) => {
                if va != vb {
                    residual.push(format!(
                        "value at {path}: {} -> {}",
                        excerpt(va),
                        excerpt(vb)
                    ));
                    return;
                }
                if sa != sb {
                    if *sa == ScalarStyle::Plain
                        && matches!(sb, ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted)
                    {
                        labels.insert(PlainRequoted);
                    } else {
                        labels.insert(BlockRestyled);
                    }
                }
                if ca != cb || ia != ib {
                    labels.insert(BlockRestyled);
                }
            }
            (N::Null { .. }, N::Null { .. }) => {}
            (
                N::Null { .. },
                N::Scalar {
                    value,
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    block_indent: None,
                    ..
                },
            )
            | (
                N::Scalar {
                    value,
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    block_indent: None,
                    ..
                },
                N::Null { .. },
            ) if null_token(value) => {
                labels.insert(NullToken);
            }
            (
                N::Mapping {
                    pairs: pa,
                    flow_style: fa,
                    ..
                },
                N::Mapping {
                    pairs: pb,
                    flow_style: fb,
                    ..
                },
            ) => {
                if fa != fb {
                    if pa.is_empty() && pb.is_empty() {
                        labels.insert(EmptyContainerFlow);
                    } else if *fa && carries_note(a) {
                        labels.insert(NoteForcesBlockLayout);
                    } else if *fb && !*fa && in_flow {
                        labels.insert(InlineContainerFlow);
                    } else {
                        residual.push(format!("container style at {path}: flow {fa} -> {fb}"));
                    }
                }
                if pa.len() != pb.len() {
                    residual.push(format!(
                        "entry count at {path}: {} -> {}",
                        pa.len(),
                        pb.len()
                    ));
                    return;
                }
                for (i, ((ka, va), (kb, vb))) in pa.iter().zip(pb.iter()).enumerate() {
                    let name = match ka {
                        N::Scalar { value, .. } => excerpt(value),
                        other => format!("<{}>", variant_name(other)),
                    };
                    let here = format!("{path}.{name}#{i}");
                    let child_flow = *fa || in_flow;
                    diff_trees(
                        ka,
                        kb,
                        &format!("{here}[key]"),
                        child_flow,
                        labels,
                        residual,
                    );
                    diff_trees(va, vb, &here, child_flow, labels, residual);
                }
            }
            (
                N::Sequence {
                    items: ia,
                    flow_style: fa,
                    ..
                },
                N::Sequence {
                    items: ib,
                    flow_style: fb,
                    ..
                },
            ) => {
                if fa != fb {
                    if ia.is_empty() && ib.is_empty() {
                        labels.insert(EmptyContainerFlow);
                    } else if *fa && carries_note(a) {
                        labels.insert(NoteForcesBlockLayout);
                    } else if *fb && !*fa && in_flow {
                        labels.insert(InlineContainerFlow);
                    } else {
                        residual.push(format!("container style at {path}: flow {fa} -> {fb}"));
                    }
                }
                if ia.len() != ib.len() {
                    residual.push(format!(
                        "entry count at {path}: {} -> {}",
                        ia.len(),
                        ib.len()
                    ));
                    return;
                }
                for (i, (xa, xb)) in ia.iter().zip(ib.iter()).enumerate() {
                    diff_trees(
                        xa,
                        xb,
                        &format!("{path}[{i}]"),
                        *fa || in_flow,
                        labels,
                        residual,
                    );
                }
            }
            (x, y) => residual.push(format!(
                "shape at {path}: {} -> {}",
                variant_name(x),
                variant_name(y)
            )),
        }
    }

    /// Every note the tree carries has to reach the document.
    ///
    /// Compared over trimmed text, because padding after the marker is not content any reader
    /// recovers; a note that is blank once trimmed has nothing a comment can carry, so it is exempt
    /// rather than tolerated silently.
    fn note_reach(node: &crate::ast::CustomNode, emitted: &str) -> Vec<String> {
        let mut missing = Vec::new();
        collect_notes(node, emitted, &mut missing);
        missing
    }

    fn collect_notes(node: &crate::ast::CustomNode, emitted: &str, missing: &mut Vec<String>) {
        use crate::ast::CustomNode as N;
        for text in node
            .comment()
            .map(|c| vec![c.text.to_string()])
            .unwrap_or_default()
            .into_iter()
            .chain(node.leading_comments().iter().map(|c| c.text.to_string()))
        {
            let trimmed = text.trim();
            // Restated here rather than borrowed from the serializer, so a writer that stopped
            // flattening reddens this check instead of agreeing with it: a note line has no spelling
            // for a break in any of these formats, so what must reach the document is the one-line
            // form. The fixed-point properties are what prove that form is re-readable and stable.
            let expected = trimmed.replace(['\n', '\r'], " ");
            if !trimmed.is_empty() && !emitted.contains(&expected) {
                missing.push(format!("note never reached the document: {text:?}"));
            }
        }
        match node {
            N::Mapping { pairs, .. } => {
                for (k, v) in pairs {
                    collect_notes(k, emitted, missing);
                    collect_notes(v, emitted, missing);
                }
            }
            N::Sequence { items, .. } => {
                for i in items {
                    collect_notes(i, emitted, missing);
                }
            }
            _ => {}
        }
    }

    /// Classify one serialize → parse cycle: which named normalizations it used, and what it
    /// changed that none of them explains.
    fn roundtrip_residuals(
        before: &crate::ast::CustomNode,
        after: &crate::ast::CustomNode,
        emitted: &str,
    ) -> (std::collections::BTreeSet<Normalization>, Vec<String>) {
        let mut labels = std::collections::BTreeSet::new();
        let mut residual = note_reach(before, emitted);
        diff_trees(before, after, "$", false, &mut labels, &mut residual);
        (labels, residual)
    }

    /// The same cycle, reported the way the old helper did it - now as an assertion the property
    /// makes rather than a filter it hides behind.
    fn roundtrip(node: &crate::ast::CustomNode) -> (String, crate::ast::CustomNode) {
        let yaml = to_yaml(node);
        let parsed =
            parse_with_options(&yaml, true, Schema::Core, 1000, false).expect("output re-parses");
        (yaml, parsed)
    }

    /// The tree as the tier's resolving reader would report it. A plain untagged
    /// `<<` pair is a parse-time merge directive, not a key that survives the
    /// cycle: the reader consumes it and folds the source mapping in, so the
    /// source's own tag/anchor belongs to the anchor, not to the synthesized
    /// result. Comparing against the raw sample made that reader behaviour look
    /// like writer data loss (CI seed `8e067f67…`, `prop_tag_preserved`), and the
    /// writer must keep spelling `<<` plainly because a tree parsed with
    /// `resolve_merges=false` carries a real directive there.
    fn as_the_reader_sees(node: &crate::ast::CustomNode) -> crate::ast::CustomNode {
        let mut composed = node.clone();
        crate::parser::yaml::resolve_merge_keys(&mut composed);
        composed
    }

    /// Key *content* bytes for order-property comparisons, deliberately
    /// ignoring serialized spelling: a flow-unsafe plain token (`0[`) re-reads
    /// as the same string under a quoted style, `null` normalizes between
    /// the Null variant and a resolved scalar, and a container key's flow/block
    /// bit is the reader's choice at that position - `InlineContainerFlow` below.
    /// Round-trip-equivalent trees must not differ in *order* on those shifts, so
    /// this encodes the entries themselves, kind-tagged and in order, rather than
    /// the text the writer happens to emit for them.
    fn key_content(k: &crate::ast::CustomNode) -> Vec<u8> {
        fn into(node: &crate::ast::CustomNode, out: &mut Vec<u8>) {
            use crate::ast::CustomNode as N;
            match node {
                N::Scalar { value, .. } if null_token(value) => out.push(b'n'),
                N::Scalar { value, .. } => {
                    out.push(b's');
                    out.extend(value.as_bytes());
                }
                N::Null { .. } => out.push(b'n'),
                N::Mapping { pairs, .. } => {
                    out.push(b'M');
                    for (key, value) in pairs {
                        into(key, out);
                        into(value, out);
                    }
                    out.push(b'}');
                }
                N::Sequence { items, .. } => {
                    out.push(b'L');
                    for item in items {
                        into(item, out);
                    }
                    out.push(b']');
                }
                N::Alias { name } => {
                    out.push(b'a');
                    out.extend(name.as_bytes());
                }
            }
        }
        let mut out = Vec::new();
        into(k, &mut out);
        out
    }

    /// Recursively apply random scalar style / chomping / flow style to every
    /// node in the tree, exercising the style setters on real data.
    fn deep_apply_styles(
        node: &mut crate::ast::CustomNode,
        style: crate::ast::ScalarStyle,
        chomp: crate::ast::Chomping,
        flow: bool,
    ) {
        use crate::ast::CustomNode;
        match node {
            CustomNode::Scalar { .. } => {
                node.set_scalar_style(style);
                node.set_chomping(chomp);
            }
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. } => {
                node.set_flow_style(flow);
            }
            _ => {}
        }
        if let CustomNode::Mapping { pairs, .. } = node {
            for (_, v) in pairs.iter_mut() {
                deep_apply_styles(v, style, chomp, flow);
            }
        } else if let CustomNode::Sequence { items, .. } = node {
            for item in items.iter_mut() {
                deep_apply_styles(item, style, chomp, flow);
            }
        }
    }

    /// Assemble a syntactically-valid YAML document that exercises the
    /// anchor / alias / merge-key structure class — the shape behind the
    /// #163 (repeated alias) and #166 (self-referential merge stack overflow)
    /// crashes. `arb_custom_node()` emits `meta.anchor` but never an `Alias`
    /// node, so those resolve/merge paths are otherwise never fuzzed.
    ///
    /// The builder always produces well-formed source: anchors are defined
    /// before their aliases, and every merge value is one of the YAML 1.1
    /// forms (single alias, alias sequence, sequence with an inline map, bare
    /// inline map) plus the two *invalid* sources (scalar / null) the #166 fix
    /// now rejects instead of leaving a stray `<<`. A self-referential anchor
    /// (`a0: &a0` whose body merges `*a0`) and a forward chain (`a1` merging
    /// `*a0`) are toggled to drive the resolver's path-scoped cycle guard.
    fn arb_merge_alias_source() -> impl Strategy<Value = String> {
        (
            any::<bool>(), // base0 self-merge (name-based cycle)
            any::<bool>(), // base1 merges *a0 (forward chain)
            any::<bool>(), // base2 merges *a1 (chain depth 2)
            0u8..6,        // consumer merge-value shape
            1usize..4,     // repeated-alias count in a plain sequence
        )
            .prop_map(|(self_merge, chain1, chain2, shape, dup)| {
                let mut s = String::new();
                // base0 — optionally a `<<: *a0` cycle, the #166 crasher.
                if self_merge {
                    s.push_str("base0: &a0\n  <<: *a0\n  p0: q0\n");
                } else {
                    s.push_str("base0: &a0\n  p0: q0\n");
                }
                // base1 — merges the already-defined *a0 (acyclic forward ref).
                if chain1 {
                    s.push_str("base1: &a1\n  <<: *a0\n  p1: q1\n");
                } else {
                    s.push_str("base1: &a1\n  p1: q1\n");
                }
                // base2 — merges *a1, giving depth-2 chain expansion.
                if chain2 {
                    s.push_str("base2: &a2\n  <<: *a1\n  p2: q2\n");
                } else {
                    s.push_str("base2: &a2\n  p2: q2\n");
                }
                let consumer_merge = match shape {
                    0 => "<<: *a0",
                    1 => "<<: [*a0, *a1, *a2]",
                    2 => "<<: [*a0, {inline: 1}]",
                    3 => "<<: {inline: 2}",
                    4 => "<<: scalar", // invalid merge source
                    _ => "<<: null",   // invalid merge source
                };
                s.push_str(&format!("consumer:\n  {consumer_merge}\n  r: s\n"));
                // Repeated alias references to the same anchor (#163 class).
                let aliases = vec!["*a0"; dup];
                s.push_str(&format!("dup: [{}]\n", aliases.join(", ")));
                s
            })
    }

    /// Documents that put an alias in key position, in both container styles, with the anchor
    /// always defined before the reference so every source here is legal YAML. `extra` adds an
    /// unrelated pair, so a key that is resolved away shows up as a shorter mapping rather than
    /// hiding behind a duplicate. Nothing else in this tier reaches the shape: `arb_custom_node`
    /// never builds a mapping keyed by an `Alias`, and `try_roundtrip` *drops* a mismatching tree
    /// instead of failing it.
    fn arb_alias_key_source() -> impl Strategy<Value = String> {
        (0u8..3, any::<bool>(), any::<bool>()).prop_map(|(body, flow, extra)| {
            let filler_block = if extra { "k1: v1\n" } else { "" };
            let filler_flow = if extra { "k1: v1, " } else { "" };
            match body {
                0 if flow => format!("{{a: &x [1, 2], {}? *x : v}}\n", filler_flow),
                0 => format!("a: &x [1, 2]\n{}? *x\n: v\n", filler_block),
                1 if flow => format!("{{a: &x {{p: q}}, {}? *x : v}}\n", filler_flow),
                1 => format!("a: &x {{p: q}}\n{}? *x\n: v\n", filler_block),
                _ => format!("a: &x plain\n{}? *x\n: v\n", filler_block),
            }
        })
    }

    /// Every note text the tree carries, in document order (a mapping's key before its value). Both
    /// sides of the flow-note comparison come from parsing text, so what each holds is already what
    /// the reader can keep — the assertion is between two readings, not between a tree and a hope.
    fn note_texts(node: &crate::ast::CustomNode) -> Vec<String> {
        let mut out = Vec::new();
        collect_note_texts(node, &mut out);
        out
    }

    fn collect_note_texts(node: &crate::ast::CustomNode, out: &mut Vec<String>) {
        use crate::ast::CustomNode as N;
        if let Some(inline) = node.comment().filter(|c| !c.standalone) {
            out.push(inline.text.to_string());
        }
        out.extend(node.leading_comments().iter().map(|c| c.text.to_string()));
        match node {
            N::Mapping { pairs, .. } => {
                for (k, v) in pairs.iter() {
                    collect_note_texts(k, out);
                    collect_note_texts(v, out);
                }
            }
            N::Sequence { items, .. } => {
                for item in items.iter() {
                    collect_note_texts(item, out);
                }
            }
            _ => {}
        }
    }

    /// Sources that put a note on a member of a flow collection — every spelling the writer has to
    /// answer: a value's note, a key's, a sequence item's, the second pair of three, a flow
    /// container under a block key, under a block dash, nested in another flow container, carrying
    /// an anchor, and one pair holding both slots.
    ///
    /// The note text is drawn from the forms that have broken comment handling before: a `#` and a
    /// quote inside the text, the block terminator `*/`, blanks at each end (which the reader
    /// drops), a no-break space (which YAML does not), and a leading `-`.
    fn arb_flow_note_source() -> impl Strategy<Value = String> {
        let text = prop::sample::select(vec![
            "A",
            "note 7",
            "two words",
            "  padded  ",
            "#hash",
            "*/term",
            "q\"uote",
            "\u{a0}nbsp",
            "- dash",
            "x: y",
            "a # b",
        ]);
        (0usize..9, text).prop_map(|(shape, note)| match shape {
            0 => format!("{{A: x  # {note}\n}}\n"),
            1 => format!("{{A: # {note}\n  x}}\n"),
            2 => format!("[x  # {note}\n]\n"),
            3 => format!("k: {{A: x  # {note}\n}}\n"),
            4 => format!("- {{A: x  # {note}\n}}\n"),
            5 => format!("{{A: [x  # {note}\n]}}\n"),
            6 => format!("{{A: x  # {note}\n, B: 2}}\n"),
            7 => format!("&anc {{A: x  # {note}\n}}\n"),
            _ => format!("{{A: # {note}\n  x  # {note}V\n}}\n"),
        })
    }

    /// The `? ` marker asked the style flag instead of the demotion, so a flow-marked complex key
    /// whose note stack the braces cannot hold wrote those notes *below* the marker - one round to
    /// move them back up, and a body stranded inside the marker's own lines. Lifting the spine on
    /// the block-body question has a second effect worth pinning: once the overflowing note is off
    /// the spine the key's braces fit again, so the flow spelling the source asked for survives
    /// instead of being demoted a second time.
    ///
    /// Both halves are rows here. A key that never claimed flow has to keep its block body (the two
    /// control rows), and a flow-marked key has to keep its braces rather than be sent to block by a
    /// lift that no longer has anything to lift.
    #[test]
    fn an_explicit_key_lift_follows_the_block_body_not_the_style_flag() {
        use crate::ast::{Comment, CustomNode, NodeMap, Tag};

        let note = |text: &str| Comment {
            text: text.into(),
            standalone: true,
        };
        let scalar = |value: &str| CustomNode::plain_scalar(value);
        let mapping = |pairs: Vec<(CustomNode, CustomNode)>| {
            let mut map = NodeMap::default();
            for (k, v) in pairs {
                map.insert(k, v);
            }
            CustomNode::plain_mapping(map)
        };
        let flow = |mut node: CustomNode| {
            node.set_flow_style(true);
            node
        };
        let with_notes = |value: &str, texts: &[&str]| {
            let mut node = scalar(value);
            for text in texts {
                node.push_leading_comment(note(text));
            }
            node
        };

        let demoted_key = flow(mapping(vec![(
            with_notes("k", &["first", "second"]),
            scalar("v"),
        )]));
        let anchored = {
            let mut node = demoted_key.clone();
            node.set_anchor("A1");
            node
        };
        let tagged = {
            let mut node = demoted_key.clone();
            node.set_tag(Tag {
                handle: "!!".into(),
                suffix: "Tag1".into(),
            });
            node
        };
        let demoted_seq_key = flow(CustomNode::plain_sequence(vec![with_notes(
            "x",
            &["first", "second"],
        )]));
        let with_anchor = |mut node: CustomNode| {
            node.set_anchor("ROOT");
            node
        };

        /// What the line after the marker is required to look like once the notes are lifted.
        enum MarkerBody {
            KeepsBraces,
            WritesBlockBody,
        }
        use MarkerBody::{KeepsBraces, WritesBlockBody};

        let cases: Vec<(&str, CustomNode, MarkerBody)> = vec![
            (
                "control: plain block complex key, inner key with two notes",
                mapping(vec![(
                    mapping(vec![(with_notes("k", &["first", "second"]), scalar("v"))]),
                    scalar("x"),
                )]),
                WritesBlockBody,
            ),
            (
                "control: plain block complex sequence key, item with two notes",
                mapping(vec![(
                    CustomNode::plain_sequence(vec![with_notes("x", &["first", "second"])]),
                    scalar("y"),
                )]),
                WritesBlockBody,
            ),
            (
                "demoting flow mapping as the complex key",
                mapping(vec![(demoted_key.clone(), scalar("x"))]),
                KeepsBraces,
            ),
            (
                "anchored demoting flow mapping as the complex key",
                mapping(vec![(anchored, scalar("x"))]),
                KeepsBraces,
            ),
            (
                "tagged demoting flow mapping as the complex key",
                mapping(vec![(tagged, scalar("x"))]),
                KeepsBraces,
            ),
            (
                "demoting flow sequence as the complex key",
                mapping(vec![(demoted_seq_key.clone(), scalar("x"))]),
                KeepsBraces,
            ),
            (
                "demoting flow complex key inside a block sequence item",
                CustomNode::plain_sequence(vec![mapping(vec![(demoted_key.clone(), scalar("x"))])]),
                KeepsBraces,
            ),
            (
                "anchored block root whose first key demotes - the header waits for the spine",
                with_anchor(mapping(vec![(demoted_key.clone(), scalar("x"))])),
                KeepsBraces,
            ),
            (
                "anchored block root whose first key is a demoting flow sequence",
                with_anchor(mapping(vec![(demoted_seq_key.clone(), scalar("x"))])),
                KeepsBraces,
            ),
            (
                "anchored mapping as a block sequence item, its first key demoting",
                CustomNode::plain_sequence(vec![with_anchor(mapping(vec![(
                    demoted_key.clone(),
                    scalar("x"),
                )]))]),
                KeepsBraces,
            ),
            (
                "demoting flow complex key nested inside a value mapping",
                mapping(vec![(
                    scalar("outer"),
                    mapping(vec![(demoted_key, scalar("x"))]),
                )]),
                KeepsBraces,
            ),
        ];

        for (label, node, body) in cases {
            let yaml = to_yaml(&node);
            let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false).unwrap_or_else(
                |error| panic!("({label}) emitted {yaml:?}, which does not re-parse: {error:?}"),
            );
            let lines: Vec<&str> = yaml.lines().collect();
            let marker_at = lines
                .iter()
                .position(|line| line.trim_start().starts_with('?'))
                .unwrap_or_else(|| panic!("({label}) wrote no explicit key marker: {yaml:?}"));
            for text in ["first", "second"] {
                let note_at = lines
                    .iter()
                    .position(|line| line.trim() == format!("# {text}"))
                    .unwrap_or_else(|| panic!("({label}) lost its {text:?} note: {yaml:?}"));
                assert!(
                    note_at < marker_at,
                    "({label}) left {text:?} under the marker, where the next round lifts it: {yaml:?}"
                );
            }
            assert_eq!(
                lines
                    .iter()
                    .filter(|line| line.trim_start().starts_with('#'))
                    .count(),
                2,
                "({label}) invented a note line: {yaml:?}"
            );
            let body_line = lines[marker_at + 1];
            let spelled_in_braces = body_line.contains('{') || body_line.contains('[');
            match body {
                KeepsBraces => assert!(
                    spelled_in_braces,
                    "({label}) lost the flow spelling its source asked for: {yaml:?}"
                ),
                WritesBlockBody => assert!(
                    !spelled_in_braces,
                    "({label}) claimed braces for a body it writes across lines: {yaml:?}"
                ),
            }
            // Canonical on the first round, the same demand the sequence and value rows make.
            let once = to_yaml(&parsed);
            assert_eq!(
                yaml, once,
                "({label}) settles only on its second emission: {yaml:?} then {once:?}"
            );
        }
    }

    /// A member note the braces cannot hold demotes a flow-marked container to block style, and the
    /// container writers were the only ones who asked that question. Every parent deciding where a
    /// child's line starts read the `flow_style` flag alone, so it opened `- ` or `key:` for a child
    /// that then wrote a block body at the parent's column - bytes no reader takes back. The
    /// property tier found it through `prop_style_setting_roundtrip` (CI 20k-case shard), where
    /// merge resolution moved a second standalone note onto one key; the class is wider than that
    /// seed and reaches both the sequence item and the mapping value, so the guard asks about the
    /// class.
    ///
    /// Both directions are pinned. A demoting container has to be written as the block document it
    /// really became, with its note stack above the indicator, and a container that stays in flow
    /// has to keep sharing that indicator line - a fix that sent every container to its own line
    /// would pass the first half and quietly cost the second. Every row must also settle on its
    /// first emission, which is what chose the compact route over hanging the body under the dash.
    #[test]
    fn a_demoted_flow_container_is_written_as_the_block_document_it_became() {
        use crate::ast::{Chomping, Comment, CustomNode, NodeMap, ScalarStyle};

        let note = |text: &str| Comment {
            text: text.into(),
            standalone: true,
        };
        let scalar = |value: &str| CustomNode::plain_scalar(value);
        let mapping = |pairs: Vec<(CustomNode, CustomNode)>| {
            let mut map = NodeMap::default();
            for (k, v) in pairs {
                map.insert(k, v);
            }
            CustomNode::plain_mapping(map)
        };
        let flow = |mut node: CustomNode| {
            node.set_flow_style(true);
            node
        };
        let with_notes = |value: &str, texts: &[&str]| {
            let mut node = scalar(value);
            for text in texts {
                node.push_leading_comment(note(text));
            }
            node
        };

        /// How the row is required to be spelled. `StaysInFlow` is the over-demotion control: a
        /// container whose notes the braces do hold must keep its brackets. `DemotesToBlock` is the
        /// note-loss control from the other side: a container that fell back to block must not be
        /// spelled with braces any more, so a "fix" that dropped the note to stay in flow reddens
        /// here. Both kinds also hold the universal rules - no dash opens a note-only line, no note
        /// or scalar text is lost, and the first emission is already the fixed point.
        enum Placement {
            StaysInFlow,
            DemotesToBlock,
        }

        let cases: Vec<(&str, CustomNode, Placement)> = vec![
            (
                "flow item whose key carries two notes has no brace for the second",
                CustomNode::plain_sequence(vec![flow(mapping(vec![(
                    with_notes("", &["first", "second"]),
                    scalar("v"),
                )]))]),
                Placement::DemotesToBlock,
            ),
            (
                "flow item whose key carries one note still fits in the braces",
                CustomNode::plain_sequence(vec![flow(mapping(vec![(
                    with_notes("k", &["only"]),
                    scalar("v"),
                )]))]),
                Placement::StaysInFlow,
            ),
            (
                "empty flow item has no block spelling to take",
                CustomNode::plain_sequence(vec![flow(mapping(vec![]))]),
                Placement::StaysInFlow,
            ),
            (
                "flow sequence item demoted by a note stack inside it",
                CustomNode::plain_sequence(vec![flow(CustomNode::plain_sequence(vec![
                    with_notes("x", &["first", "second"]),
                ]))]),
                Placement::DemotesToBlock,
            ),
            (
                "flow value mapping demoted by a note stack on its key",
                mapping(vec![(
                    scalar("outer"),
                    flow(mapping(vec![(
                        with_notes("k", &["first", "second"]),
                        scalar("v"),
                    )])),
                )]),
                Placement::DemotesToBlock,
            ),
            (
                "flow value sequence demoted by a note stack inside it",
                mapping(vec![(
                    scalar("outer"),
                    flow(CustomNode::plain_sequence(vec![with_notes(
                        "x",
                        &["first", "second"],
                    )])),
                )]),
                Placement::DemotesToBlock,
            ),
            (
                "a sequence item whose value mapping demotes",
                CustomNode::plain_sequence(vec![mapping(vec![(
                    scalar("outer"),
                    flow(mapping(vec![(
                        with_notes("k", &["first", "second"]),
                        scalar("v"),
                    )])),
                )])]),
                Placement::DemotesToBlock,
            ),
            (
                "a flow value mapping whose notes do fit keeps the key line",
                mapping(vec![(
                    scalar("outer"),
                    flow(mapping(vec![(with_notes("k", &["only"]), scalar("v"))])),
                )]),
                Placement::StaysInFlow,
            ),
        ];

        fn collect_note_texts(node: &CustomNode, out: &mut Vec<String>) {
            if let Some(c) = node.comment().filter(|c| !c.standalone) {
                out.push(c.text.to_string());
            }
            for c in node.leading_comments() {
                out.push(c.text.to_string());
            }
            match node {
                CustomNode::Mapping { pairs, .. } => {
                    for (k, v) in pairs {
                        collect_note_texts(k, out);
                        collect_note_texts(v, out);
                    }
                }
                CustomNode::Sequence { items, .. } => {
                    for item in items {
                        collect_note_texts(item, out);
                    }
                }
                _ => {}
            }
        }

        fn collect_scalar_texts(node: &CustomNode, out: &mut Vec<String>) {
            if let CustomNode::Scalar { value, .. } = node {
                out.push(value.to_string());
            }
            match node {
                CustomNode::Mapping { pairs, .. } => {
                    for (k, v) in pairs {
                        collect_scalar_texts(k, out);
                        collect_scalar_texts(v, out);
                    }
                }
                CustomNode::Sequence { items, .. } => {
                    for item in items {
                        collect_scalar_texts(item, out);
                    }
                }
                _ => {}
            }
        }

        fn sorted(mut texts: Vec<String>) -> Vec<String> {
            texts.sort();
            texts
        }

        fn of(node: &CustomNode, collect: fn(&CustomNode, &mut Vec<String>)) -> Vec<String> {
            let mut out = Vec::new();
            collect(node, &mut out);
            sorted(out)
        }

        for (label, node, placement) in cases {
            let yaml = to_yaml(&node);
            let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false);
            let parsed = match parsed {
                Ok(parsed) => parsed,
                Err(error) => {
                    panic!("({label}) emitted {yaml:?} which does not re-parse: {error:?}")
                }
            };
            let (notes, parsed_notes) = (
                of(&node, collect_note_texts),
                of(&parsed, collect_note_texts),
            );
            let (texts, parsed_texts) = (
                of(&node, collect_scalar_texts),
                of(&parsed, collect_scalar_texts),
            );
            assert_eq!(
                notes, parsed_notes,
                "({label}) a note was lost or invented by {yaml:?}"
            );
            assert_eq!(
                texts, parsed_texts,
                "({label}) a scalar was lost or changed by {yaml:?}"
            );
            assert!(
                !yaml.starts_with("- #"),
                "({label}) opened its dash with a note, stranding the item: {yaml:?}"
            );
            let first_line = yaml.split('\n').next().unwrap_or_default();
            let spelled_in_braces = first_line.contains('[') || first_line.contains('{');
            match placement {
                Placement::StaysInFlow => assert!(
                    spelled_in_braces,
                    "({label}) was expected to keep its braces, got {yaml:?}"
                ),
                Placement::DemotesToBlock => assert!(
                    !spelled_in_braces,
                    "({label}) stayed in braces while its notes needed a block, got {yaml:?}"
                ),
            }
            // Canonical on the first round: re-emitting what the reader handed back must not move
            // the text, or the shape needs two emissions to settle and every consumer of an
            // already-parsed document pays for the second one.
            let once = to_yaml(&parsed);
            assert_eq!(
                yaml, once,
                "({label}) settles only on its second emission: {yaml:?} then {once:?}"
            );
        }

        // The seed the property tier actually reported: a `<<` pair whose notes only meet once
        // the reader resolves the merge, styled block-folded all the way down.
        let mut seed = CustomNode::plain_sequence(vec![mapping(vec![(
            with_notes("<<", &["note on the merge key"]),
            mapping(vec![(
                with_notes("", &["note on the null key"]),
                scalar("qD.qrA}-/6U=d%"),
            )]),
        )])]);
        deep_apply_styles(&mut seed, ScalarStyle::Folded, Chomping::Clip, true);
        let seed = as_the_reader_sees(&seed);
        let yaml = to_yaml(&seed);
        let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false)
            .unwrap_or_else(|error| panic!("the CI seed emitted {yaml:?}: {error:?}"));
        let (notes, reparsed) = (
            of(&seed, collect_note_texts),
            of(&parsed, collect_note_texts),
        );
        assert_eq!(notes, reparsed, "the CI seed lost a note through {yaml:?}");
    }

    /// The deepest note stack any single node in the tree carries.
    fn deepest_note_stack(node: &crate::ast::CustomNode) -> usize {
        use crate::ast::CustomNode as N;
        let own = usize::from(node.comment().is_some_and(|c| !c.standalone))
            + node.leading_comments().len();
        match node {
            N::Mapping { pairs, .. } => pairs.iter().fold(own, |best, (k, v)| {
                best.max(deepest_note_stack(k)).max(deepest_note_stack(v))
            }),
            N::Sequence { items, .. } => items
                .iter()
                .fold(own, |best, item| best.max(deepest_note_stack(item))),
            _ => own,
        }
    }

    /// Every scalar value in the tree, in document order.
    fn scalar_texts(node: &crate::ast::CustomNode, out: &mut Vec<String>) {
        use crate::ast::CustomNode as N;
        if let N::Scalar { value, .. } = node {
            out.push(value.to_string());
        }
        match node {
            N::Mapping { pairs, .. } => {
                for (k, v) in pairs.iter() {
                    scalar_texts(k, out);
                    scalar_texts(v, out);
                }
            }
            N::Sequence { items, .. } => {
                for item in items.iter() {
                    scalar_texts(item, out);
                }
            }
            _ => {}
        }
    }

    /// A note stack the braces cannot hold, built by construction in every role the class has.
    ///
    /// `arb_node_meta` carries at most one note per node, so no draw of `arb_custom_node` can put the
    /// two standalone notes on one member that send a flow-marked container to block - which is why
    /// ledger (gn) had to be pinned by a table: the tier could not reach the class at all, and a
    /// regression in it would pass 20,000 cases. This strategy hands the writer the shape in each
    /// position a parent decides about - a sequence item, a mapping value, a complex key, and a
    /// complex key under a root that carries an `&anchor` header of its own - as a mapping body or a
    /// sequence body, optionally anchored or tagged.
    fn arb_note_stack_overflow() -> impl Strategy<Value = crate::ast::CustomNode> {
        use crate::ast::{Comment, CustomNode, NodeMap, Tag};

        /// The container that has to hold the overflowing member: as a mapping key or a sequence item,
        /// and either flow-marked (so the demotion is what decides its spelling) or block already.
        fn body(member: CustomNode, as_sequence: bool, flow: bool) -> CustomNode {
            let mut node = if as_sequence {
                CustomNode::plain_sequence(vec![member])
            } else {
                let mut pairs = NodeMap::default();
                pairs.insert(member, CustomNode::plain_scalar("v"));
                CustomNode::plain_mapping(pairs)
            };
            node.set_flow_style(flow);
            node
        }

        (
            0u8..8,
            prop::collection::vec(
                proptest::string::string_regex("[A-Za-z0-9._-]{1,6}").expect("regex"),
                2..4,
            ),
            proptest::string::string_regex("[A-Za-z0-9._-]{1,6}").expect("regex"),
            prop::option::of("[a-zA-Z_][a-zA-Z0-9_-]{0,4}"),
            prop::option::of("[a-zA-Z_][a-zA-Z0-9_-]{0,4}"),
        )
            .prop_map(|(role, notes, value, anchor, tag)| {
                let mut member = CustomNode::plain_scalar(value.as_str());
                for note in notes {
                    member.push_leading_comment(Comment {
                        text: note.into(),
                        standalone: true,
                    });
                }
                // Every other role pairs a mapping body (the stack sits on a key) with a sequence
                // body (the stack sits on an item), so both arms of each writer are drawn.
                let child = body(member, role % 2 == 1, true);
                match role {
                    0 | 1 => CustomNode::plain_sequence(vec![child]),
                    2 | 3 => {
                        let mut pairs = NodeMap::default();
                        pairs.insert(CustomNode::plain_scalar("outer"), child);
                        CustomNode::plain_mapping(pairs)
                    }
                    4 | 5 => {
                        let child = {
                            let mut child = child;
                            if let Some(name) = anchor {
                                child.set_anchor(name);
                            }
                            if let Some(suffix) = tag {
                                child.set_tag(Tag {
                                    handle: "!!".into(),
                                    suffix,
                                });
                            }
                            child
                        };
                        let mut pairs = NodeMap::default();
                        pairs.insert(child, CustomNode::plain_scalar("x"));
                        CustomNode::plain_mapping(pairs)
                    }
                    _ => {
                        let mut root = {
                            let mut pairs = NodeMap::default();
                            pairs.insert(child, CustomNode::plain_scalar("x"));
                            CustomNode::plain_mapping(pairs)
                        };
                        root.set_anchor("ROOT");
                        root
                    }
                }
            })
    }

    proptest! {
        /// The note-stack class must be written once and settle once, in every role it can take:
        /// the same four demands the tables of ledger (gn) make, drawn rather than listed.
        #[test]
        fn prop_note_stack_overflow_settles_on_the_first_emission(
            node in arb_note_stack_overflow(),
        ) {
            let yaml = to_yaml(&node);
            let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false).unwrap_or_else(
                |error| panic!("the draw emitted {yaml:?}, which does not re-parse: {error:?}"),
            );
            prop_assert!(
                deepest_note_stack(&node) >= 2,
                "the strategy drew no overflowing member: {yaml:?}"
            );
            let (mut mine, mut theirs) = (note_texts(&node), note_texts(&parsed));
            mine.sort();
            theirs.sort();
            prop_assert!(
                mine == theirs,
                "a note was lost or invented by {yaml:?}: {mine:?} vs {theirs:?}"
            );
            let (mut my_scalars, mut their_scalars) = (Vec::new(), Vec::new());
            scalar_texts(&node, &mut my_scalars);
            scalar_texts(&parsed, &mut their_scalars);
            my_scalars.sort();
            their_scalars.sort();
            prop_assert!(
                my_scalars == their_scalars,
                "a scalar was lost or changed by {yaml:?}: {my_scalars:?} vs {their_scalars:?}"
            );
            let once = to_yaml(&parsed);
            prop_assert!(
                yaml == once,
                "settles only on its second emission: {yaml:?} then {once:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn prop_roundtrip(node in arb_custom_node()) {
            let node = as_the_reader_sees(&node);
            let (yaml, parsed) = roundtrip(&node);
            let (labels, residual) = roundtrip_residuals(&node, &parsed, &yaml);
            prop_assert!(
                residual.is_empty(),
                "unexplained round-trip change: {residual:?}\nnormalizations used: {labels:?}\n{}",
                yaml_diff(&yaml, &to_yaml(&parsed))
            );
        }

        /// Text-level re-parseability: every AST the builder can produce must
        /// serialize to YAML that PARSES AT ALL. The AST-vs-AST round-trip
        /// above cannot catch invalid *text* — when nested block scalars lost
        /// their body indent, `parse(to_yaml(x))` failed outright and the
        /// mismatch was only visible through this gate (serializer base-indent
        /// regression, found via the TOML hot-spot benchmark sample).
        #[test]
        fn prop_output_always_parses(node in arb_custom_node()) {
            let yaml = to_yaml(&node);
            let reparsed = parse_with_options(&yaml, true, Schema::Core, 1000, false);
            prop_assert!(
                reparsed.is_ok(),
                "serialized output failed to re-parse: {yaml:?} -> {:?}",
                reparsed.err()
            );
        }

        /// An alias written in key position must come back as that alias. Unconditional on
        /// purpose: the tier's `try_roundtrip` filters a mismatching tree away rather than
        /// reporting it, which is precisely how the writer dropping `? *x` to `null: v` stayed
        /// invisible — the tree changed shape, so the property never asserted anything about it.
        #[test]
        fn prop_alias_key_survives_the_round_trip(src in arb_alias_key_source()) {
            let before = parse_with_options(&src, true, Schema::Core, 1000, false)
                .unwrap_or_else(|e| panic!("{src:?} should parse: {e}"));
            let emitted = to_yaml(&before);
            prop_assert!(
                emitted.contains("*x"),
                "the alias key was resolved away: {src:?} -> {emitted:?}\n{}",
                yaml_diff(&src, &emitted)
            );
            let after = parse_with_options(&emitted, true, Schema::Core, 1000, false)
                .unwrap_or_else(|e| panic!("{emitted:?} should re-parse: {e}"));
            prop_assert!(
                nodes_equal_ignore_meta(&before, &after),
                "alias key changed the tree: {src:?} -> {emitted:?}\n{}",
                yaml_diff(&src, &emitted)
            );
            prop_assert_eq!(emitted, to_yaml(&after), "not a fixed point");
        }

        /// The flow writer used to answer a member's note with no note at all: `{A: x  # V}` reads
        /// into a tree that keeps the note and wrote `{A: x}`, so the note existed only in memory.
        /// Every shape here is one this engine's reader is measured to hand back to a member, so the
        /// assertion needs no filter — each note in the source must reach the text, come back on
        /// re-read, and the emission must equal its own second round. Written as a source strategy
        /// rather than over `arb_custom_node()` because the generated trees scatter notes across the
        /// block path too, whose separate gap `docs/dev/quality-ledger.md` records: a property that
        /// rejects most of its samples to stay green is the blind spot PR #412 registered.
        #[test]
        fn prop_flow_member_note_survives_the_round_trip(src in arb_flow_note_source()) {
            let before = parse_with_options(&src, true, Schema::Core, 1000, false)
                .unwrap_or_else(|e| panic!("{:?} should parse: {e}", src));
            let emitted = to_yaml(&before);
            let after = parse_with_options(&emitted, true, Schema::Core, 1000, false)
                .unwrap_or_else(|e| panic!("{:?} -> {:?} should re-parse: {e}", src, emitted));
            prop_assert_eq!(
                note_texts(&before),
                note_texts(&after),
                "a flow member note was dropped or moved: {:?} -> {:?}",
                src,
                emitted
            );
            prop_assert_eq!(&emitted, &to_yaml(&after), "not a fixed point: {:?}", emitted);
        }

        #[test]
        fn prop_no_crash_invalid_utf8(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
            if let Ok(s) = std::str::from_utf8(&bytes) {
                let _ = parse_with_options(s, true, Schema::Core, 1000, false);
            }
        }

        #[test]
        fn prop_structure_depth_limit(node in arb_custom_node()) {
            let opts = SerializeOptions {
                max_depth: 3,
                ..Default::default()
            };
            let result = to_yaml_with_options(&node, &opts);
            if let Err(ref e) = result {
                let msg = e.to_string();
                prop_assert!(msg.contains("depth") || msg.contains("max"),
                    "unexpected error: {}", msg);
            }
        }

        // --- 0.14+ new features ---

        /// Applying arbitrary scalar style / chomping / flow style to a tree
        /// never produces invalid YAML (round-trip parse still succeeds and is
        /// structurally equivalent up to the tolerated normalizations).
        #[test]
        fn prop_style_setting_roundtrip(
            (mut node, style, chomp, flow) in (
                arb_custom_node(),
                arb_scalar_style(),
                arb_chomping(),
                any::<bool>(),
            )
        ) {
            deep_apply_styles(&mut node, style, chomp, flow);
            let node = as_the_reader_sees(&node);
            let (yaml, parsed) = roundtrip(&node);
            let (labels, residual) = roundtrip_residuals(&node, &parsed, &yaml);
            prop_assert!(
                residual.is_empty(),
                "a styled tree changed beyond the named list: {residual:?} (used {labels:?})"
            );
        }

        /// Serializing the result of a round-trip produces an AST-equivalent (not
        /// byte-identical) document: an empty block container normalizes to its
        /// flow spelling on the first re-serialize, and stays stable after.
        #[test]
        fn prop_idempotent(node in arb_custom_node()) {
            let node = as_the_reader_sees(&node);
            let (yaml, parsed) = roundtrip(&node);
            let (_, first) = roundtrip_residuals(&node, &parsed, &yaml);
            prop_assert!(first.is_empty(), "first cycle unexplained: {first:?}");
            {
                let once = to_yaml(&parsed);
                let twice_parsed = parse_with_options(&once, true, Schema::Core, 1000, false)
                    .expect("re-parse of normalized output must succeed");
                let (_, second) = roundtrip_residuals(&parsed, &twice_parsed, &once);
                prop_assert!(
                    second.is_empty(),
                    "output not stable after first normalization: {second:?}"
                );
            }
        }

        /// Mapping key order survives the serialize → parse cycle.
        #[test]
        fn prop_mapping_order_preserved(node in arb_custom_node()) {
            // Asserted on, never filtered: the cycle below runs for every sample.
            let node = as_the_reader_sees(&node);
            let parsed = roundtrip(&node).1;
            {
                fn key_order(n: &crate::ast::CustomNode) -> Vec<Vec<u8>> {
                    match n {
                        crate::ast::CustomNode::Mapping { pairs, .. } => pairs
                            .keys()
                            .map(key_content)
                            .collect(),
                        _ => vec![],
                    }
                }
                let a = key_order(&node);
                let b = key_order(&parsed);
                prop_assert_eq!(a, b, "mapping key order changed");
            }
        }

        /// Mappings nested anywhere in the tree keep their entry order too.
        /// Collected by position, not by style bit: whether a container reads back
        /// as flow is the reader's normalization (`InlineContainerFlow`, named in
        /// `(da)`), so counting the flow mappings would make a style shift look
        /// like an order change. Each mapping's key order is compared at its own
        /// path, and an empty container still contributes nothing.
        #[test]
        fn prop_flow_order_preserved(node in arb_custom_node()) {
            // Asserted on, never filtered: the cycle below runs for every sample.
            let node = as_the_reader_sees(&node);
            let parsed = roundtrip(&node).1;
            {
                fn mapping_key_orders(
                    n: &crate::ast::CustomNode,
                    path: &str,
                    out: &mut Vec<(String, Vec<u8>)>,
                ) {
                    use crate::ast::CustomNode as N;
                    match n {
                        N::Mapping { pairs, .. } => {
                            if !pairs.is_empty() {
                                let mut order = Vec::new();
                                for key in pairs.keys() {
                                    order.extend(key_content(key));
                                    order.push(0);
                                }
                                out.push((path.to_string(), order));
                            }
                            for (i, (key, value)) in pairs.iter().enumerate() {
                                mapping_key_orders(key, &format!("{path}[{i}]k"), out);
                                mapping_key_orders(value, &format!("{path}[{i}]v"), out);
                            }
                        }
                        N::Sequence { items, .. } => {
                            for (i, item) in items.iter().enumerate() {
                                mapping_key_orders(item, &format!("{path}[{i}]"), out);
                            }
                        }
                        _ => {}
                    }
                }
                let mut a = Vec::new();
                let mut b = Vec::new();
                mapping_key_orders(&node, "$", &mut a);
                mapping_key_orders(&parsed, "$", &mut b);
                prop_assert_eq!(a, b, "mapping key order changed");
            }
        }

        /// A comment on a node is emitted by the serializer (the node may be
        /// on a block path that round-trips, in which case the comment text
        /// must survive).
        #[test]
        fn prop_comment_preserved(node in arb_custom_node()) {
            // Which node owns a note is the reader's normalisation (measured in ledger (db)); that
            // a note reaches the document at all is not. Positional equality would charge the
            // engine for a thing the reference libraries do too, so this asserts the claim the
            // tier can honestly make about every sample, and the classifier owns the rest.
            let yaml = to_yaml(&node);
            let missing = note_reach(&node, &yaml);
            prop_assert!(missing.is_empty(), "notes lost: {missing:?}
    in {yaml:?}");
        }

        /// Tag preservation on round-trip.
        #[test]
        fn prop_tag_preserved(node in arb_custom_node()) {
            fn tag_texts(n: &crate::ast::CustomNode) -> Vec<String> {
                match n {
                    crate::ast::CustomNode::Scalar { meta, .. }
                    | crate::ast::CustomNode::Mapping { meta, .. }
                    | crate::ast::CustomNode::Sequence { meta, .. }
                    | crate::ast::CustomNode::Null { meta, .. } => {
                        let mut v = meta
                            .tag()
                            .map(|t| vec![t.to_string()])
                            .unwrap_or_default();
                        match n {
                            crate::ast::CustomNode::Mapping { pairs, .. } => {
                                for (k, vv) in pairs {
                                    v.extend(tag_texts(k));
                                    v.extend(tag_texts(vv));
                                }
                            }
                            crate::ast::CustomNode::Sequence { items, .. } => {
                                for i in items {
                                    v.extend(tag_texts(i));
                                }
                            }
                            _ => {}
                        }
                        v
                    }
                    _ => vec![],
                }
            }
            // Asserted on, never filtered: the cycle below runs for every sample.
            let node = as_the_reader_sees(&node);
            let parsed = roundtrip(&node).1;
            {
                let a = tag_texts(&node);
                let b = tag_texts(&parsed);
                prop_assert_eq!(a, b, "tags changed after round-trip");
            }
        }
    }

    proptest! {
        // Exercises `parse_schema_yaml` on random schema YAML — never panics.
        // Use arbitrary string in the "pattern" value position via a compact
        // char class that includes quotes and YAML punctuation.
        #[cfg(feature = "std")]
        #[test]
        fn prop_schema_parse_no_panic(schema_yaml in "[!#$%&()*+,-./:;<=>?@\\[\\]^_`{}~a-zA-Z0-9 \"']{0,200}\\n*") {
            let _ = crate::parser::yaml::schema_language::parse_schema_yaml(&schema_yaml);
        }

        /// Anchors + aliases + merge keys (the #163/#166 structure class):
        /// parsing with merge resolution must never panic or blow the native
        /// stack, and any tree that parses must re-serialize and re-parse
        /// cleanly. Before the #166 path-scoped cycle guard, the self-merge
        /// case overflowed the stack and aborted the process; proptest cannot
        /// catch an abort, so a green run here is the regression proof.
        #[test]
        fn prop_merge_alias_never_panics(src in arb_merge_alias_source()) {
            let parsed = parse_with_options(&src, true, Schema::Core, 1000, false);
            if let Ok(node) = parsed {
                let out = to_yaml(&node);
                let _ = parse_with_options(&out, true, Schema::Core, 1000, false);
            }
        }
    }

    /// The generator's key rule, this crate's writer and this crate's reader have to
    /// describe the same mapping, and nothing but a table like this one proves it.
    /// `keys_collide_in_text` lives in `pyrs-ast` and is a hand-maintained model of
    /// the two components here; every gap between the model and the reality has shown
    /// up first as a random property-tier failure. The gap that wrote this guard
    /// reported `DuplicateKey("null")` for `!E null: A  # 0` beside `!E null: a` — a
    /// pair of `Null` nodes the rule did not look at at all.
    ///
    /// Both directions are pinned against real text. A pair the rule calls
    /// distinguishable must still hold two entries once its serialization is re-read
    /// strictly; a pair it calls identical must be refused or folded by the reader.
    /// A rule degraded to "everything collides" would leave the generator emitting
    /// single-key mappings and the tier green, so the table asserts the count of
    /// classes on each side as well.
    #[test]
    fn the_key_rule_agrees_with_the_writer_and_the_reader() {
        use crate::ast::{Comment, NodeMap, ScalarStyle, Tag};
        use pyrs_ast::ast::proptest_strategies::{keys_collide_in_flow, keys_collide_in_text};

        let null = |tag: Option<(&str, &str)>, anchor: Option<&str>, note: Option<&str>| {
            let mut node = crate::ast::CustomNode::Null {
                meta: Default::default(),
            };
            if let Some((handle, suffix)) = tag {
                node.set_tag(Tag {
                    handle: handle.to_string(),
                    suffix: suffix.to_string(),
                });
            }
            if let Some(name) = anchor {
                node.set_anchor(name);
            }
            if let Some(text) = note {
                node.set_comment(Comment {
                    text: text.into(),
                    standalone: false,
                });
            }
            node
        };
        let scalar = |value: &str, style: ScalarStyle, tag: Option<(&str, &str)>| {
            let mut node = crate::ast::CustomNode::plain_scalar(value);
            node.set_scalar_style(style);
            if let Some((handle, suffix)) = tag {
                node.set_tag(Tag {
                    handle: handle.to_string(),
                    suffix: suffix.to_string(),
                });
            }
            node
        };
        let plain = |value: &str| scalar(value, ScalarStyle::Plain, None);
        let mapping_key = |keys: &[(&str, &str)]| {
            let mut pairs = NodeMap::default();
            for (k, v) in keys {
                pairs.insert(plain(k), plain(v));
            }
            crate::ast::CustomNode::plain_mapping(pairs)
        };
        let sequence_key = |items: &[&str]| {
            crate::ast::CustomNode::plain_sequence(items.iter().map(|item| plain(item)).collect())
        };

        // (label, key a, key b, whether the rule claims the text tells them apart)
        let cases: Vec<(&str, crate::ast::CustomNode, crate::ast::CustomNode, bool)> = vec![
            // --- distinguishable: two entries must survive a strict re-read ---
            (
                "two plain scalars, different text",
                plain("k"),
                plain("v"),
                true,
            ),
            (
                "a null and a string",
                null(None, None, None),
                plain("hello"),
                true,
            ),
            (
                "two nulls under different tags",
                null(Some(("!", "E")), None, None),
                null(Some(("!", "F")), None, None),
                true,
            ),
            (
                "a tagged null and a bare null",
                null(Some(("!", "E")), None, None),
                null(None, None, None),
                true,
            ),
            (
                "two tagged nulls under different anchors",
                null(Some(("!", "E")), Some("a"), None),
                null(Some(("!", "E")), Some("b"), None),
                true,
            ),
            (
                "two nested mappings, different content",
                mapping_key(&[("x", "1")]),
                mapping_key(&[("x", "2")]),
                true,
            ),
            (
                "an empty mapping and an empty sequence",
                mapping_key(&[]),
                sequence_key(&[]),
                true,
            ),
            // The writer quotes an empty plain key (`"": v`) instead of emitting the
            // null spelling the reader would fold, so a tilde key keeps its own entry.
            ("a tilde and an empty key", plain("~"), plain(""), true),
            // A tag is part of the emitted line, so `!a k:` and `!b k:` re-read as two
            // entries: the reader's value-collision track deliberately skips tagged keys.
            (
                "two scalars, same text, different tags",
                scalar("k", ScalarStyle::Plain, Some(("!", "a"))),
                scalar("k", ScalarStyle::Plain, Some(("!", "b"))),
                true,
            ),
            (
                "two scalars, same text, same tag",
                scalar("k", ScalarStyle::Plain, Some(("!", "a"))),
                scalar("k", ScalarStyle::Plain, Some(("!", "a"))),
                false,
            ),
            // Which note slot separates a complex key is measured, not assumed. An *inline* note on
            // an item inside a sequence key rides the key's own line (`? - null  # A`) and comes
            // back attached to that item, so the pair re-reads as two entries. A *standalone* note
            // on the same item is hoisted above the pair (`# A` between the two `?` blocks), never
            // reaches the key, and the reader then reports the second key as a duplicate - the
            // shape the property tier drew at seed `de78e195…`.
            (
                "two sequence keys, one item with an inline note",
                crate::ast::CustomNode::plain_sequence(vec![crate::ast::CustomNode::Null {
                    meta: Default::default(),
                }]),
                crate::ast::CustomNode::plain_sequence(vec![null(None, None, Some("A"))]),
                true,
            ),
            // A tagged key is outside the reader's text track (`parser/mod.rs` skips tagged
            // scalars there), so two spellings of one tagged text re-read as two entries: measured,
            // `!a k: one` + `!a 'k': two` comes back with both.
            (
                "a tagged scalar, plain and quoted",
                scalar("k", ScalarStyle::Plain, Some(("!", "a"))),
                scalar("k", ScalarStyle::SingleQuoted, Some(("!", "a"))),
                true,
            ),
            // `null:` reads as a null key and `'null':` as the string, so the reader keeps
            // both; `true:` and `'true':` both reach the reader's text track and collide.
            (
                "a plain null spelling and its quoted twin",
                plain("null"),
                scalar("null", ScalarStyle::SingleQuoted, None),
                true,
            ),
            (
                "a plain boolean spelling and its quoted twin",
                plain("true"),
                scalar("true", ScalarStyle::SingleQuoted, None),
                false,
            ),
            // --- identical: the reader must refuse or fold, never keep both ---
            (
                "two tagged nulls, one carrying a note",
                null(Some(("!", "E")), None, Some("0")),
                null(Some(("!", "E")), None, None),
                false,
            ),
            (
                "two bare nulls under different anchors",
                null(None, Some("a"), None),
                null(None, Some("b"), None),
                false,
            ),
            (
                "a null and a plain scalar spelled null",
                null(None, None, None),
                plain("null"),
                false,
            ),
            (
                "the NULL and null spellings",
                plain("NULL"),
                plain("null"),
                false,
            ),
            (
                "a null and a tilde",
                null(None, None, None),
                plain("~"),
                false,
            ),
            (
                "the same text, plain and quoted",
                plain("k"),
                scalar("k", ScalarStyle::SingleQuoted, None),
                false,
            ),
            (
                "two empty mappings",
                mapping_key(&[]),
                mapping_key(&[]),
                false,
            ),
            (
                "two empty sequences",
                sequence_key(&[]),
                sequence_key(&[]),
                false,
            ),
            // --- the containers and tagged scalars the node map tells apart ---
            (
                "a flow sequence and its block twin",
                crate::ast::CustomNode::Sequence {
                    items: vec![plain("a")],
                    flow_style: true,
                    meta: crate::ast::NodeMeta::default(),
                },
                sequence_key(&["a"]),
                true,
            ),
            (
                "two mappings whose note rides an inner key",
                mapping_key(&[("z", "1")]),
                {
                    let mut pairs = NodeMap::default();
                    let mut key = plain("z");
                    key.set_comment(crate::ast::Comment {
                        text: "deep".into(),
                        standalone: false,
                    });
                    pairs.insert(key, plain("1"));
                    crate::ast::CustomNode::plain_mapping(pairs)
                },
                true,
            ),
            (
                "a sequence of one null and an empty sequence",
                crate::ast::CustomNode::Sequence {
                    items: vec![null(None, None, None)],
                    flow_style: false,
                    meta: crate::ast::NodeMeta::default(),
                },
                sequence_key(&[]),
                true,
            ),
            (
                "a tagged scalar and its noted twin",
                scalar("k", ScalarStyle::Plain, Some(("!", "a"))),
                {
                    let mut noted = scalar("k", ScalarStyle::Plain, Some(("!", "a")));
                    noted.set_comment(crate::ast::Comment {
                        text: "n".into(),
                        standalone: false,
                    });
                    noted
                },
                false,
            ),
            (
                "two sequences of one null, one carrying a note",
                crate::ast::CustomNode::Sequence {
                    items: vec![null(None, None, None)],
                    flow_style: false,
                    meta: crate::ast::NodeMeta::default(),
                },
                crate::ast::CustomNode::Sequence {
                    items: vec![null(None, None, None)],
                    flow_style: false,
                    meta: {
                        let mut meta = crate::ast::NodeMeta::default();
                        meta.set_comment(crate::ast::Comment {
                            text: "n".into(),
                            standalone: true,
                        });
                        meta
                    },
                },
                false,
            ),
        ];
        let distinguishable = cases.iter().filter(|(_, _, _, keepable)| *keepable).count();
        assert_eq!(
            distinguishable, 15,
            "the table must keep a real share of its classes on each side"
        );

        // The same pairs under a FLOW parent, judged by asking the writer and the reader rather
        // than by restating a table: a flow parent cannot spell a block container at all, and
        // that is the shape that took the property tier down at 20k cases (CI job
        // 117575499211) - two keys the block parent tells apart came out as
        // `{? [null]: A, ? [null]: B}`.
        for (label, a, b) in cases.iter().map(|(l, a, b, _)| (l, a, b)) {
            let mut pairs = NodeMap::default();
            pairs.insert(a.clone(), plain("one"));
            if pairs.insert(b.clone(), plain("two")).is_some() {
                // The node map folds the pair before it can be emitted; the block pass below
                // records that, and there is no text left to ask the reader about.
                continue;
            }
            let node = crate::ast::CustomNode::Mapping {
                pairs,
                flow_style: true,
                meta: crate::ast::NodeMeta::default(),
            };
            let yaml = to_yaml(&node);
            let folded = keys_collide_in_flow(a, b);
            // Only one direction has to be exact, because only one direction is loaded: a pair
            // the rule lets through has to survive the reader. Over-folding costs a lost sample,
            // never a broken tree - measured, `{!E null: # 0 one, !E null: two}` keeps two
            // entries because a note landing in the flow text separates them, while the same
            // note separates nothing in a block mapping, so the fold drops that sample here.
            if folded {
                continue;
            }
            match parse_with_options(&yaml, true, Schema::Core, 1000, false) {
                Ok(crate::ast::CustomNode::Mapping { pairs: read, .. }) => assert_eq!(
                    read.len(),
                    2,
                    "{label}: a flow parent emitted {yaml:?} and the rule says the keys are told \
                     apart, but it re-read {} entries",
                    read.len()
                ),
                Ok(other) => panic!("{label}: a flow parent re-read as {other:?}"),
                Err(err) => panic!(
                    "{label}: a flow parent refused {yaml:?} ({err}) while the fold rule says \
                     the two are told apart"
                ),
            }
        }

        for (label, a, b, keepable) in cases {
            assert_eq!(
                keys_collide_in_text(&a, &b),
                !keepable,
                "{label}: the rule disagrees with the table"
            );
            // Values that differ, so a fold is visible as a lost entry rather than
            // an overwritten one.
            let mut pairs = NodeMap::default();
            pairs.insert(a.clone(), plain("one"));
            // `IndexMap::insert` hands back the value it replaced: nothing replaced
            // means both keys are now in the map.
            let held_both = pairs.insert(b.clone(), plain("two")).is_none();
            if !held_both {
                // The node map itself cannot hold the pair — the same conclusion the
                // rule reaches, and a reason the text oracle below cannot run.
                assert!(!keepable, "{label}: the node map folded a distinct pair");
                continue;
            }
            let yaml = to_yaml(&crate::ast::CustomNode::plain_mapping(pairs));
            match parse_with_options(&yaml, true, Schema::Core, 1000, false) {
                Ok(crate::ast::CustomNode::Mapping { pairs: read, .. }) => {
                    assert_eq!(
                        read.len(),
                        if keepable { 2 } else { 1 },
                        "{label}: {yaml:?} re-read as {read:?}"
                    );
                }
                Ok(other) => panic!("{label}: {yaml:?} re-read as a {other:?}"),
                Err(err) => assert!(
                    !keepable,
                    "{label}: a distinct pair {yaml:?} was refused: {err}"
                ),
            }
        }
    }

    #[test]
    fn a_flow_parent_loses_the_block_twin_and_the_reader_is_right_to_refuse_it() {
        // The tree behind a random property-tier failure (CI job 117575499211, 2026-10-18),
        // kept as the shape the generator must stop building: a mapping holding two sequence
        // keys that each wrap one `Null` variant, one block-styled and one flow-styled. Under a
        // block parent the pair re-reads as two entries; under a flow parent there is no
        // spelling for a block container, so both keys emit as `[null]` and the duplicate the
        // reader refuses is not its mistake.
        let null = crate::ast::CustomNode::Null {
            meta: Default::default(),
        };
        let key = |flow: bool| crate::ast::CustomNode::Sequence {
            items: vec![null.clone()],
            flow_style: flow,
            meta: Default::default(),
        };
        let parent = |flow: bool| {
            let mut pairs = crate::ast::NodeMap::default();
            pairs.insert(key(false), crate::ast::CustomNode::plain_scalar("A"));
            pairs.insert(key(true), crate::ast::CustomNode::plain_scalar("B"));
            crate::ast::CustomNode::Mapping {
                pairs,
                flow_style: flow,
                meta: Default::default(),
            }
        };

        let block = to_yaml(&parent(false));
        match parse_with_options(&block, true, Schema::Core, 1000, false) {
            Ok(crate::ast::CustomNode::Mapping { pairs: read, .. }) => {
                assert_eq!(read.len(), 2, "{block:?} re-read as {read:?}");
            }
            other => panic!("{block:?} re-read as {other:?}"),
        }

        let flow = to_yaml(&parent(true));
        assert_eq!(flow, "{? [null]: A, ? [null]: B}\n");
        assert!(
            parse_with_options(&flow, true, Schema::Core, 1000, false).is_err(),
            "{flow:?} was read as one document"
        );
    }
}
