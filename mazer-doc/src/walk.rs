//! Visiting every code form of a document in document order, and small tree
//! helpers shared by the passes.
//!
//! # Size limits
//!
//! Macro expansion and `let` substitution copy trees, so a few short lines can
//! ask for an enormous amount of output: fifteen doubling definitions,
//! `(let x1 (+ x0 x0))` ... `(let x15 (+ x14 x14))`, make `x15` about 100 000
//! nodes, and every `(show x15)` would copy all of them. Three limits keep
//! `analyze` (which the editor runs on every keystroke) and the HTML bounded:
//!
//! - **One tree**: an expanded or substituted tree may have at most
//!   [`MAX_NODES`] nodes and [`MAX_TREE_DEPTH`] levels. Going over is a
//!   diagnostic at that use, which becomes an `Error` node.
//! - **The whole document**: every node *created* by instantiating a macro or
//!   alias template, by copying a macro argument into it, or by substituting a
//!   `let` variable is charged to one [`Budget`] of [`MAX_DOC_NODES`] for the
//!   whole `analyze`. Nodes copied unchanged from what was written are free,
//!   and so is storing a value with `let` (that is a move); the value is
//!   charged each time it is substituted at a use, including a use inside a
//!   later `let`. When the budget runs out, the expression being processed
//!   becomes an `Error` node (an error box), exactly one diagnostic is
//!   reported for the whole document (`MacroDepth` if a macro or alias ran it
//!   out, `Other` if a variable did), and from then on every expansion or
//!   substitution that would create nodes is refused the same way, silently.
//!   A note with 10 000 macro uses and 10 000 `let`s creates under 100 000;
//!   the notes in use today create none. At the limit the page holds roughly
//!   2.5 MB of `MathML` from created nodes and `analyze` stays well under a
//!   second, which is what the editor needs on every keystroke.

use mazer_syntax::{Block, BlockKind, Diag, DiagCode, Diags, Document, Inline, InlineKind, Node, NodeKind, Span, Sym};

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

/// Most nodes macro expansion and `let` substitution may create in one whole
/// document (see the module docs).
pub const MAX_DOC_NODES: usize = 250_000;

/// The document-wide node budget: one per `analyze`, shared by macro
/// expansion and `let` substitution.
#[derive(Debug, Default)]
pub struct Budget {
    used: usize,
    exhausted: bool,
}

impl Budget {
    pub const fn new() -> Self {
        Self { used: 0, exhausted: false }
    }

    /// Charges `n` nodes that are being created now. Checked by the next
    /// [`Self::fits`] / [`Self::try_spend`].
    pub const fn spend(&mut self, n: usize) {
        self.used = self.used.saturating_add(n);
    }

    /// Whether more nodes may still be created.
    pub const fn fits(&self) -> bool {
        !self.exhausted && self.used <= MAX_DOC_NODES
    }

    /// Charges `n` nodes about to be created, if they fit.
    pub const fn try_spend(&mut self, n: usize) -> bool {
        let total = self.used.saturating_add(n);
        if self.exhausted || total > MAX_DOC_NODES {
            return false;
        }
        self.used = total;
        true
    }

    /// Marks the budget as run out. The first time, reports the one
    /// diagnostic for the document at `span` (with `def` as a secondary
    /// label); afterwards does nothing.
    pub fn refuse(&mut self, code: DiagCode, span: Span, def: Option<Span>, diags: &mut Diags) {
        if self.exhausted {
            return;
        }
        self.exhausted = true;
        let mut diag = Diag::error(
            code,
            span,
            format!(
                "this note creates more than {MAX_DOC_NODES} nodes through `let` variables and notation; \
                 this and every later expression that would create more are not shown"
            ),
        )
        .with_help("a definition is probably growing exponentially, e.g. `(let x (+ x x))` repeated or an alias used twice in the next one");
        if let Some(def) = def {
            diag = diag.with_label(def, "defined here");
        }
        diags.push(diag);
    }
}

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

/// The diagnostic for a `(tag ...)` that is not the first item of a display
/// `show`. Shared by every pass that finds one, so `analyze` can merge the
/// reports into one.
pub fn misplaced_tag(span: Span) -> Diag {
    Diag::error(DiagCode::MisplacedTag, span, "a tag must be the first item of a display `show`")
        .with_help("write it as (show (tag name) expr) in a paragraph of its own")
}

/// Replaces every list headed by a reserved word (`show`, `eval`, `let`,
/// `alias`, `macro`, `ref`) or by `tag`, anywhere in `node` including `node`
/// itself, with an `Error` node, reporting each once. `outer` is the form the
/// code belongs to (`show`, `eval`, `let`, `alias`, `macro`), for messages.
/// Returns whether anything was replaced.
///
/// A reserved word that is not the head of a list is an ordinary variable and
/// is left alone. Callers skip the one legitimate `(tag name)` (the first item
/// of a `show`) themselves.
pub fn strip_reserved(node: &mut Node, outer: &str, diags: &mut Diags) -> bool {
    if let Some(diag) = reserved_use(node, outer) {
        diags.push(diag);
        *node = Node::error(node.span);
        return true;
    }
    let mut found = false;
    if let NodeKind::List(items) = &mut node.kind {
        for item in items {
            found |= strip_reserved(item, outer, diags);
        }
    }
    found
}

/// The diagnostic for `node` if it is a list headed by a reserved word or `tag`.
fn reserved_use(node: &Node, outer: &str) -> Option<Diag> {
    let head = node.head()?;
    let diag = match head.as_str() {
        "eval" => Diag::error(DiagCode::NestedEval, node.span, "`eval` cannot be used inside another form")
            .with_help("put the (eval ...) in a paragraph or sentence of its own"),
        "tag" => misplaced_tag(node.span),
        name @ ("let" | "alias" | "macro") => Diag::error(
            DiagCode::BadDefinition,
            node.span,
            format!("`{name}` cannot be used inside `{outer}`"),
        )
        .with_help("write the definition as a block or in a sentence of its own"),
        name @ ("show" | "ref") => Diag::error(
            DiagCode::Other,
            node.span,
            format!("`{name}` can only be used in prose, not inside `{outer}`"),
        )
        .with_help(format!("move the ({name} ...) out of the `{outer}`")),
        _ => return None,
    };
    Some(diag)
}

/// Whether `inner` lies within `outer` (same file).
pub fn span_within(inner: Span, outer: Span) -> bool {
    inner.file == outer.file && outer.start <= inner.start && inner.end <= outer.end
}
