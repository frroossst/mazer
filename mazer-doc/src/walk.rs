//! Visiting every code form of a document in document order, and small tree
//! helpers shared by the passes.

use mazer_syntax::{Block, BlockKind, Document, Inline, InlineKind, Node, NodeKind, Span, Sym};

/// Where a code form stands: alone as a block (a display equation) or inside
/// a sentence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Block,
    Inline,
}

/// Calls `f` on every code form of `doc` (block and inline, at any nesting of
/// lists, quotes, headings, styles and links), in document order.
pub fn for_each_code<'a>(doc: &'a Document, f: &mut impl FnMut(&'a Node, Place)) {
    blocks(&doc.blocks, f);
}

fn blocks<'a>(list: &'a [Block], f: &mut impl FnMut(&'a Node, Place)) {
    for block in list {
        match &block.kind {
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inlines(content, f),
            BlockKind::List { items, .. } => {
                for item in items {
                    blocks(&item.content, f);
                }
            }
            BlockKind::BlockQuote(inner) => blocks(inner, f),
            BlockKind::Code(node) => f(node, Place::Block),
            BlockKind::CodeBlock { .. } | BlockKind::Image { .. } | BlockKind::Rule => {}
        }
    }
}

fn inlines<'a>(list: &'a [Inline], f: &mut impl FnMut(&'a Node, Place)) {
    for inline in list {
        match &inline.kind {
            InlineKind::Bold(inner)
            | InlineKind::Italic(inner)
            | InlineKind::Underline(inner)
            | InlineKind::Strike(inner)
            | InlineKind::Spoiler(inner)
            | InlineKind::Link { content: inner, .. } => inlines(inner, f),
            InlineKind::Code(node) => f(node, Place::Inline),
            InlineKind::Text(_)
            | InlineKind::Image { .. }
            | InlineKind::InlineCode(_)
            | InlineKind::Emoji(_)
            | InlineKind::LineBreak => {}
        }
    }
}

/// Like [`for_each_code`], with mutable access to each form.
pub fn for_each_code_mut(doc: &mut Document, f: &mut impl FnMut(&mut Node, Place)) {
    blocks_mut(&mut doc.blocks, f);
}

fn blocks_mut(list: &mut [Block], f: &mut impl FnMut(&mut Node, Place)) {
    for block in list {
        match &mut block.kind {
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inlines_mut(content, f),
            BlockKind::List { items, .. } => {
                for item in items {
                    blocks_mut(&mut item.content, f);
                }
            }
            BlockKind::BlockQuote(inner) => blocks_mut(inner, f),
            BlockKind::Code(node) => f(node, Place::Block),
            BlockKind::CodeBlock { .. } | BlockKind::Image { .. } | BlockKind::Rule => {}
        }
    }
}

fn inlines_mut(list: &mut [Inline], f: &mut impl FnMut(&mut Node, Place)) {
    for inline in list {
        match &mut inline.kind {
            InlineKind::Bold(inner)
            | InlineKind::Italic(inner)
            | InlineKind::Underline(inner)
            | InlineKind::Strike(inner)
            | InlineKind::Spoiler(inner)
            | InlineKind::Link { content: inner, .. } => inlines_mut(inner, f),
            InlineKind::Code(node) => f(node, Place::Inline),
            InlineKind::Text(_)
            | InlineKind::Image { .. }
            | InlineKind::InlineCode(_)
            | InlineKind::Emoji(_)
            | InlineKind::LineBreak => {}
        }
    }
}

/// Whether the tree holds an `Error` node anywhere (reader recovery, or a
/// failed expansion).
pub fn contains_error(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Error => true,
        NodeKind::List(items) => items.iter().any(contains_error),
        NodeKind::Ident(_) | NodeKind::Number(_) | NodeKind::Str(_) => false,
    }
}

/// Size and depth of a tree, used to keep expansion and substitution from
/// blowing up on hostile input.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Measure {
    /// Number of nodes.
    pub size: usize,
    /// Nesting depth; a leaf has depth 1.
    pub depth: usize,
}

/// Most nodes one expanded or substituted tree may have.
pub const MAX_NODES: usize = 100_000;

/// Deepest one expanded or substituted tree may be. The reader allows 256
/// levels; this leaves room for macros and variables on top while keeping the
/// recursive emitters well inside the stack (also in wasm).
pub const MAX_TREE_DEPTH: usize = 320;

impl Measure {
    pub const LEAF: Self = Self { size: 1, depth: 1 };

    pub fn of(node: &Node) -> Self {
        match &node.kind {
            NodeKind::List(items) => Self::list(items.iter().map(Self::of)),
            _ => Self::LEAF,
        }
    }

    /// The measure of a list whose children measure `children`.
    pub fn list(children: impl Iterator<Item = Self>) -> Self {
        let inner = children.fold(Self::default(), |acc, c| Self {
            size: acc.size.saturating_add(c.size),
            depth: acc.depth.max(c.depth),
        });
        Self {
            size: inner.size.saturating_add(1),
            depth: inner.depth + 1,
        }
    }

    pub const fn too_big(self) -> bool {
        self.size > MAX_NODES || self.depth > MAX_TREE_DEPTH
    }
}

/// A copy of `node` in which every node has the span `span`.
pub fn respan(node: &Node, span: Span) -> Node {
    match &node.kind {
        NodeKind::List(items) => Node::list(items.iter().map(|c| respan(c, span)).collect(), span),
        kind => Node::new(kind.clone(), span),
    }
}

/// Why a definition may not use `name`, if it may not: it is a reserved word,
/// a built-in keyword, or (when `display` is set) a display name. Display
/// names count for `alias`/`macro` but not for `let`.
pub fn builtin_kind(name: Sym, display: bool) -> Option<&'static str> {
    let text = name.as_str();
    if mazer_syntax::RESERVED_HEADS.contains(&text) || text == "tag" {
        Some("a reserved word")
    } else if mazer_math::registry().get(name).is_some() {
        Some("a built-in keyword")
    } else if display && mazer_math::display_name(name).is_some() {
        Some("a built-in display name")
    } else {
        None
    }
}

/// Whether `inner` lies within `outer` (same file).
pub fn span_within(inner: Span, outer: Span) -> bool {
    inner.file == outer.file && outer.start <= inner.start && inner.end <= outer.end
}
