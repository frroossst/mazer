//! The document tree: what the markdown parser produces.
//!
//! Blocks stack vertically (headings, paragraphs, lists); inlines flow within
//! a line (text, bold, links, inline code). Blocks contain inlines; inlines
//! never contain blocks.

use crate::node::Node;
use crate::span::Span;
use crate::sym::Sym;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Block {
    pub kind: BlockKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BlockKind {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        /// `Some(start)` for a numbered list, `None` for bullets.
        ordered: Option<u32>,
        items: Vec<ListItem>,
    },
    BlockQuote(Vec<Block>),
    CodeBlock {
        lang: Option<Box<str>>,
        code: Box<str>,
    },
    Image {
        alt: Box<str>,
        url: Box<str>,
    },
    /// A reserved-head form (`show`, `eval`, `let`, `alias`, `macro`) standing
    /// alone as its own paragraph. A standalone `show` is a display equation.
    Code(Node),
    /// `---`
    Rule,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ListItem {
    /// `None` = plain bullet, `Some(false)` = `-[ ]`, `Some(true)` = `-[x]`.
    pub checked: Option<bool>,
    /// Blocks, so that lists can nest.
    pub content: Vec<Block>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Inline {
    pub kind: InlineKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InlineKind {
    Text(Box<str>),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strike(Vec<Inline>),
    Spoiler(Vec<Inline>),
    Link {
        content: Vec<Inline>,
        url: Box<str>,
    },
    Image {
        alt: Box<str>,
        url: Box<str>,
    },
    /// Text between backticks. Never interpreted.
    InlineCode(Box<str>),
    /// A known shortcode such as `:sob:`, holding the name `sob`. Unknown
    /// shortcodes stay [`InlineKind::Text`].
    Emoji(Sym),
    /// A reserved-head form inside a sentence, e.g. an inline `(show ...)` or
    /// `(ref name)`.
    Code(Node),
    LineBreak,
}

impl Block {
    #[must_use]
    pub const fn new(kind: BlockKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Inline {
    #[must_use]
    pub const fn new(kind: InlineKind, span: Span) -> Self {
        Self { kind, span }
    }
}
