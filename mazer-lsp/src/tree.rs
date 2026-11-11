//! Deciding what each name in the tree as written ([`Analysis::written`])
//! *is*: the shared ground of hover, definition, references, semantic
//! tokens and signature help.
//!
//! Walking the document, cursor geometry (enclosing lists, argument index),
//! `let` scope, comments and the spans of definition names come from
//! `mazer_doc` itself ([`mazer_doc::code_forms`], [`mazer_doc::binding_at`]
//! and the rest of its query API), so the editor and the renderer cannot
//! disagree about them.

use mazer_doc::math::{DisplayName, Keyword, display_name, registry};
use mazer_doc::syntax::{Block, BlockKind, Document, Inline, InlineKind, Node, RESERVED_HEADS};
use mazer_doc::{Analysis, Binding, ByteOffset, Label, Macro, Sym, binding_at, code_forms, is_paren_list};

use crate::convert::{NOTE, note_text};

/// Whether `at` lies in a fenced code block or an inline code span, where
/// nothing is ever interpreted.
#[must_use]
pub fn in_verbatim(doc: &Document, at: ByteOffset) -> bool {
    fn in_blocks(list: &[Block], at: ByteOffset) -> bool {
        list.iter().any(|block| match &block.kind {
            BlockKind::CodeBlock { .. } => block.span.start <= at && at <= block.span.end,
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => in_inlines(content, at),
            BlockKind::List { items, .. } => items.iter().any(|item| in_blocks(&item.content, at)),
            BlockKind::BlockQuote(inner) => in_blocks(inner, at),
            BlockKind::Image { .. } | BlockKind::Code(_) | BlockKind::Rule => false,
        })
    }
    fn in_inlines(list: &[Inline], at: ByteOffset) -> bool {
        list.iter().any(|inline| match &inline.kind {
            InlineKind::InlineCode(_) => inline.span.start < at && at < inline.span.end,
            InlineKind::Bold(inner)
            | InlineKind::Italic(inner)
            | InlineKind::Underline(inner)
            | InlineKind::Strike(inner)
            | InlineKind::Spoiler(inner)
            | InlineKind::Link { content: inner, .. } => in_inlines(inner, at),
            _ => false,
        })
    }
    in_blocks(&doc.blocks, at)
}

/// Where a node stands in its code form.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'a> {
    /// The outermost form (`show`, `let`, ...).
    pub root: &'a Node,
    /// The list holding the node, if any (none for the root itself).
    pub parent: Option<&'a Node>,
    /// The node's position in `parent`.
    pub index: usize,
    /// Whether `parent` is written with parentheses (not `'x` / `-x`).
    pub paren: bool,
    /// Inside `(quote ...)` / `'...`: never substituted.
    pub quoted: bool,
    /// Inside the body or the signature of an `alias` / `macro`, where `let`
    /// variables are not substituted.
    pub definition: bool,
    /// The `macro` form whose signature or body holds the node.
    pub macro_form: Option<&'a Node>,
}

/// Calls `f` on every node of the code form `root`, depth first, in source
/// order, with its context. `text` is the note.
pub fn visit<'a>(text: &str, root: &'a Node, f: &mut impl FnMut(&'a Node, &Ctx<'a>)) {
    let ctx = Ctx {
        root,
        parent: None,
        index: 0,
        paren: false,
        quoted: false,
        definition: false,
        macro_form: None,
    };
    visit_node(text, root, &ctx, f);
}

fn visit_node<'a>(text: &str, node: &'a Node, ctx: &Ctx<'a>, f: &mut impl FnMut(&'a Node, &Ctx<'a>)) {
    f(node, ctx);
    let Some(items) = node.as_list() else {
        return;
    };
    let paren = is_paren_list(text, node);
    let quote = node.is_form("quote");
    let alias = node.is_form("alias");
    let mac = node.is_form("macro");
    for (index, child) in items.iter().enumerate() {
        let child_ctx = Ctx {
            root: ctx.root,
            parent: Some(node),
            index,
            paren,
            quoted: ctx.quoted || (quote && index > 0),
            definition: ctx.definition || ((alias || mac) && index > 0),
            macro_form: if mac && index > 0 { Some(node) } else { ctx.macro_form },
        };
        visit_node(text, child, &child_ctx, f);
    }
}

/// What a name in the written tree is.
#[derive(Clone, Copy, Debug)]
pub enum Role<'a> {
    /// The `'` of `'x` or the `-` of `-x`.
    Mark,
    /// A reserved head (`show`, `eval`, `let`, `alias`, `macro`, `ref`) or
    /// `tag`, at the head of a list.
    Reserved(&'static str),
    /// A built-in keyword (or one of its second names), as a head or bare.
    Keyword(&'static Keyword),
    /// A use of an alias, or of a macro at the head of a list.
    MacroUse(&'a Macro),
    /// The name being defined by an `alias` or `macro` (the definition, if
    /// it was accepted).
    MacroName(Option<&'a Macro>),
    /// A parameter of a macro, in its signature or its body.
    MacroParam,
    /// The name bound by a `let` (the binding, if it was accepted).
    LetName(Option<&'a Binding>),
    /// A use of a `let` variable: the binding in scope there.
    LetUse(&'a Binding),
    /// The name in `(tag name)` (the label, if it was registered).
    TagName(Sym, Option<&'a Label>),
    /// The name in `(ref name)` (the label, if there is one).
    RefName(Sym, Option<&'a Label>),
    /// A variable shown as a special character (`tau` as τ).
    Display(&'static DisplayName),
    /// The head of a list that is not a keyword: function application.
    Function,
    /// Any other name.
    Free,
}

/// Decides what the identifier `node` (with context `ctx`) is.
#[must_use]
pub fn classify<'a>(analysis: &'a Analysis, node: &Node, ctx: &Ctx<'_>) -> Role<'a> {
    let Some(sym) = node.as_ident() else {
        return Role::Free;
    };
    let name = sym.as_str();
    let head = ctx.index == 0 && ctx.parent.is_some();
    if head && !ctx.paren {
        return Role::Mark;
    }
    if head && (RESERVED_HEADS.contains(&name) || name == "tag") {
        return Role::Reserved(reserved_name(name));
    }
    if let Some(parent) = ctx.parent {
        if ctx.index == 1 && parent.is_form("let") {
            let binding = analysis.lets.iter().find(|b| b.span == parent.span && b.name == sym);
            return Role::LetName(binding);
        }
        if ctx.index == 1 && parent.is_form("alias") {
            return Role::MacroName(analysis.macros.get(sym).filter(|m| m.span == parent.span));
        }
        if ctx.index == 1 && parent.is_form("tag") {
            return Role::TagName(sym, analysis.labels.get(sym).filter(|l| l.span == parent.span));
        }
        if ctx.index == 1 && parent.is_form("ref") {
            return Role::RefName(sym, analysis.labels.get(sym));
        }
        if let Some(form) = ctx.macro_form
            && form.args().first().is_some_and(|sig| sig.span == parent.span)
        {
            // In the signature `(name params...)`.
            if ctx.index == 0 {
                return Role::MacroName(analysis.macros.get(sym).filter(|m| m.span == form.span));
            }
            return Role::MacroParam;
        }
    }
    if let Some(form) = ctx.macro_form
        && macro_params(form).contains(&sym)
    {
        return Role::MacroParam;
    }
    if let Some(keyword) = registry().get(sym) {
        return Role::Keyword(keyword);
    }
    // Inside its own template a definition's name is not expanded again.
    if let Some(def) = analysis.macros.get(sym)
        && (def.is_alias() || (head && ctx.paren))
        && !(ctx.definition && def.span == ctx.root.span)
    {
        return Role::MacroUse(def);
    }
    if substituted(ctx)
        && let Some(binding) = binding_at(analysis, sym, node.span.start)
    {
        return Role::LetUse(binding);
    }
    if let Some(display) = display_name(sym) {
        return Role::Display(display);
    }
    if head { Role::Function } else { Role::Free }
}

/// Whether `let` variables are substituted at a node with this context: in
/// a `show`, an `eval` or a `let` value, outside `quote`.
fn substituted(ctx: &Ctx<'_>) -> bool {
    !ctx.quoted && !ctx.definition && (ctx.root.is_form("show") || ctx.root.is_form("eval") || ctx.root.is_form("let"))
}

/// The `&'static str` of a reserved word.
fn reserved_name(name: &str) -> &'static str {
    RESERVED_HEADS.iter().copied().find(|r| *r == name).unwrap_or("tag")
}

/// The parameter names (rest parameter without its dots) of a `macro` form.
#[must_use]
pub fn macro_params(form: &Node) -> Vec<Sym> {
    form.args().first().map_or_else(Vec::new, |sig| {
        sig.args()
            .iter()
            .filter_map(Node::as_ident)
            .map(|p| p.as_str().strip_suffix("...").filter(|b| !b.is_empty()).map_or(p, Sym::new))
            .collect()
    })
}


/// An identifier of the written tree, with its context.
#[derive(Clone, Copy, Debug)]
pub struct Found<'a> {
    pub node: &'a Node,
    pub ctx: Ctx<'a>,
}

/// The identifier at `at`: one with `start <= at < end`, or else one that
/// ends exactly at `at` (the cursor just after a word).
#[must_use]
pub fn ident_at(analysis: &Analysis, at: ByteOffset) -> Option<Found<'_>> {
    let text = note_text(&analysis.sources);
    let mut on = None;
    let mut after = None;
    for (form, _) in code_forms(analysis) {
        if at < form.span.start || form.span.end < at {
            continue;
        }
        visit(text, form, &mut |node, ctx| {
            if node.as_ident().is_none() || node.span.file != NOTE || node.span.is_empty() {
                return;
            }
            if node.span.start <= at && at < node.span.end {
                on = Some(Found { node, ctx: *ctx });
            } else if node.span.end == at && after.is_none() {
                after = Some(Found { node, ctx: *ctx });
            }
        });
        if on.is_some() {
            break;
        }
    }
    on.or(after)
}

/// Every identifier of the note, with its context, in source order.
#[must_use]
pub fn idents(analysis: &Analysis) -> Vec<Found<'_>> {
    let text = note_text(&analysis.sources);
    let mut out = Vec::new();
    for (form, _) in code_forms(analysis) {
        visit(text, form, &mut |node, ctx| {
            if node.as_ident().is_some() && node.span.file == NOTE && !node.span.is_empty() {
                out.push(Found { node, ctx: *ctx });
            }
        });
    }
    out
}

