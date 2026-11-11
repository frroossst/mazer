//! Walking the tree as written ([`Analysis::written`]) and deciding what each
//! name in it *is*: the shared ground of hover, definition, references,
//! semantic tokens and signature help.
//!
//! `mazer_doc` keeps its own document walker private, so the walk over
//! blocks and inlines is repeated here (it is short and follows the same
//! order: document order, block and inline code at any nesting).

use mazer_doc::math::{DisplayName, Keyword, display_name, registry};
use mazer_doc::syntax::{Block, BlockKind, Document, Inline, InlineKind, Node, NodeKind, RESERVED_HEADS};
use mazer_doc::{Analysis, Binding, ByteOffset, Label, Macro, Span, Sym};

use crate::convert::{NOTE, note_text};

/// Where a code form stands: alone as a block, or inside a sentence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Block,
    Inline,
}

/// Calls `f` on every code form of `doc` in document order.
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

/// The code forms of the note (file 0), in document order.
#[must_use]
pub fn forms(analysis: &Analysis) -> Vec<(&Node, Place)> {
    let mut out = Vec::new();
    for_each_code(&analysis.written, &mut |form, place| {
        if form.span.file == NOTE {
            out.push((form, place));
        }
    });
    out
}

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

/// Whether `node` is a list written with parentheses (not the `'x` or `-x`
/// shorthand the reader expands into a list).
#[must_use]
pub fn is_paren_list(text: &str, node: &Node) -> bool {
    matches!(node.kind, NodeKind::List(_)) && text.as_bytes().get(node.span.start.to_usize()) == Some(&b'(')
}

/// Whether a list was left open (typed mid-edit): its last child reaches its
/// end and is the reader's `Error` placeholder or an unclosed list.
#[must_use]
pub fn is_unclosed(node: &Node) -> bool {
    node.as_list()
        .and_then(<[Node]>::last)
        .is_some_and(|last| last.span.end == node.span.end && (matches!(last.kind, NodeKind::Error) || is_unclosed(last)))
}

/// Whether `at` is between the parentheses of the paren list `node` (the
/// same rule as `mazer_doc::locate`).
#[must_use]
pub fn inside(text: &str, node: &Node, at: ByteOffset) -> bool {
    is_paren_list(text, node)
        && node.span.start < at
        && (at < node.span.end || (at == node.span.end && is_unclosed(node)))
}

/// The children of a list without the reader's trailing `Error` placeholder.
#[must_use]
pub fn real_items(list: &Node) -> &[Node] {
    let items = list.as_list().unwrap_or(&[]);
    match items.split_last() {
        Some((placeholder, before))
            if matches!(placeholder.kind, NodeKind::Error)
                && placeholder.span.is_empty()
                && placeholder.span.end == list.span.end =>
        {
            before
        }
        _ => items,
    }
}

/// The paren lists containing `at`, outermost first. Empty in prose.
#[must_use]
pub fn enclosing_lists(analysis: &Analysis, at: ByteOffset) -> Vec<&Node> {
    let text = note_text(&analysis.sources);
    let mut chain = Vec::new();
    let Some((root, _)) = forms(analysis).into_iter().find(|(form, _)| inside(text, form, at)) else {
        return chain;
    };
    let mut node = root;
    loop {
        chain.push(node);
        let next = real_items(node).iter().find_map(|child| descend(text, child, at));
        match next {
            Some(child) => node = child,
            None => return chain,
        }
    }
}

/// The paren list at or below `node` (through `'x` / `-x` shorthand) that
/// contains `at`.
fn descend<'a>(text: &str, node: &'a Node, at: ByteOffset) -> Option<&'a Node> {
    if inside(text, node, at) {
        return Some(node);
    }
    if matches!(node.kind, NodeKind::List(_))
        && !is_paren_list(text, node)
        && node.span.start <= at
        && at <= node.span.end
    {
        return real_items(node).iter().find_map(|child| descend(text, child, at));
    }
    None
}

/// The argument position of `at` in `list` (0-based, not counting the head),
/// or `None` when the cursor is on or before the head. The rule is the one
/// `mazer_doc::locate` uses for `Cursor::Arg`.
#[must_use]
pub fn arg_index(list: &Node, at: ByteOffset) -> Option<usize> {
    let items = real_items(list);
    let (head, args) = items.split_first()?;
    if at <= head.span.end {
        return None;
    }
    for (index, arg) in args.iter().enumerate() {
        if at <= arg.span.end {
            return Some(index);
        }
    }
    Some(args.len())
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

/// The `let` binding of `name` in scope at `at`: the last one whose `let`
/// form ends at or before `at`. (A `let` takes effect after it is written,
/// so inside `(let x (+ x 1))` the `x` in the value is the earlier one.)
#[must_use]
pub fn binding_at(analysis: &Analysis, name: Sym, at: ByteOffset) -> Option<&Binding> {
    analysis
        .lets
        .iter()
        .rev()
        .find(|b| b.name == name && b.span.file == NOTE && b.span.end <= at)
}

/// Every binding in scope at `at`: for each name, the last `let` that ends
/// at or before `at`. In document order of those bindings.
#[must_use]
pub fn bindings_in_scope(analysis: &Analysis, at: ByteOffset) -> Vec<&Binding> {
    let mut out: Vec<&Binding> = Vec::new();
    for binding in analysis.lets.iter().filter(|b| b.span.file == NOTE && b.span.end <= at) {
        out.retain(|b| b.name != binding.name);
        out.push(binding);
    }
    out
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
    for (form, _) in forms(analysis) {
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
    for (form, _) in forms(analysis) {
        visit(text, form, &mut |node, ctx| {
            if node.as_ident().is_some() && node.span.file == NOTE && !node.span.is_empty() {
                out.push(Found { node, ctx: *ctx });
            }
        });
    }
    out
}

/// The spans of the `;` comments in the code form `root`. The reader drops
/// comments, so they are found in the text: a `;` outside a string literal
/// up to the end of its line (without the line break).
#[must_use]
pub fn comment_spans(text: &str, root: &Node) -> Vec<Span> {
    let mut strings = Vec::new();
    string_spans(root, &mut strings);
    strings.sort_by_key(|s| s.start);
    let bytes = text.as_bytes();
    let end = root.span.end.to_usize().min(bytes.len());
    let mut pos = root.span.start.to_usize();
    let mut next = strings.iter().peekable();
    let mut out = Vec::new();
    while pos < end {
        if let Some(span) = next.next_if(|s| s.start.to_usize() <= pos) {
            pos = pos.max(span.end.to_usize());
            continue;
        }
        if bytes[pos] == b';' {
            let mut stop = bytes[pos..].iter().position(|&b| b == b'\n').map_or(bytes.len(), |n| pos + n);
            if stop > pos && bytes[stop - 1] == b'\r' {
                stop -= 1;
            }
            out.push(Span::new(NOTE, ByteOffset::from_usize(pos), ByteOffset::from_usize(stop)));
            pos = stop.max(pos + 1);
            continue;
        }
        pos += 1;
    }
    out
}

/// The spans of every string literal in `node`.
pub fn string_spans(node: &Node, out: &mut Vec<Span>) {
    match &node.kind {
        NodeKind::Str(_) => out.push(node.span),
        NodeKind::List(items) => {
            for item in items {
                string_spans(item, out);
            }
        }
        NodeKind::Ident(_) | NodeKind::Number(_) | NodeKind::Error => {}
    }
}

/// The display `show` form tagged by `label`, if it is in the note.
#[must_use]
pub fn tagged_equation<'a>(analysis: &'a Analysis, label: &Label) -> Option<&'a Node> {
    forms(analysis)
        .into_iter()
        .map(|(form, _)| form)
        .find(|form| form.is_form("show") && form.args().first().is_some_and(|tag| tag.span == label.span))
}

/// The name node of a definition form: the `x` of `(let x v)` or
/// `(alias x v)`, the `name` of `(macro (name ...) ...)`.
#[must_use]
pub fn definition_name(form: &Node) -> Option<&Node> {
    let first = form.args().first()?;
    if form.is_form("macro") {
        first.as_list()?.first().filter(|n| n.as_ident().is_some())
    } else {
        Some(first).filter(|n| n.as_ident().is_some())
    }
}

/// The written form whose span is `span` (a definition's form).
#[must_use]
pub fn form_at_span(analysis: &Analysis, span: Span) -> Option<&Node> {
    forms(analysis).into_iter().map(|(form, _)| form).find(|form| form.span == span)
}
