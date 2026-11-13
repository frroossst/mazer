//! Queries for editors: what surrounds a cursor, which `let` is in scope,
//! where the comments are, where a definition's name is written.
//!
//! These answer from the tree as written ([`Analysis::written`]) and follow
//! the same rules as [`crate::locate`] and as `analyze` itself, so a language
//! server built on them agrees with what the renderer does. Offsets are byte
//! offsets into the note (file 0); the cursor sits *before* the byte at `at`.

use mazer_syntax::{ByteOffset, FileId, Node, NodeKind, Span, Sym};

use crate::bindings::Binding;
use crate::labels::Label;
use crate::locate::{is_inside, is_paren_list, real_items, string_spans};
use crate::pipeline::Analysis;
use crate::walk::{self, Place};

/// The note is file 0 of every analysis.
const NOTE: FileId = FileId(0);

/// The text of the note.
fn note_text(analysis: &Analysis) -> &str {
    analysis.sources.file(NOTE).map_or("", |f| f.text.as_str())
}

/// The code forms of the note (file 0) as written, in document order, with
/// their placement.
#[must_use]
pub fn code_forms(analysis: &Analysis) -> Vec<(&Node, Place)> {
    let mut out = Vec::new();
    walk::for_each_code(&analysis.written, &mut |form, place| {
        if form.span.file == NOTE {
            out.push((form, place));
        }
    });
    out
}

/// The paren lists of the written tree that contain `at`, outermost first.
///
/// The rule is that of [`crate::locate`]; the first list is the code form
/// itself and the last the innermost. Lists written with the `'x` / `-x` shorthand are looked
/// through but not included. Empty in prose.
#[must_use]
pub fn enclosing_lists(analysis: &Analysis, at: ByteOffset) -> Vec<&Node> {
    let text = note_text(analysis);
    let mut chain = Vec::new();
    let Some((root, _)) = code_forms(analysis).into_iter().find(|(form, _)| is_inside(text, form, at)) else {
        return chain;
    };
    let mut node = root;
    loop {
        chain.push(node);
        match real_items(node).iter().find_map(|child| descend(text, child, at)) {
            Some(child) => node = child,
            None => return chain,
        }
    }
}

/// The paren list at or below `node` (through `'x` / `-x` shorthand) that
/// contains `at`.
fn descend<'a>(text: &str, node: &'a Node, at: ByteOffset) -> Option<&'a Node> {
    if is_inside(text, node, at) {
        return Some(node);
    }
    if matches!(node.kind, NodeKind::List(_)) && !is_paren_list(text, node) && node.span.start <= at && at <= node.span.end {
        return real_items(node).iter().find_map(|child| descend(text, child, at));
    }
    None
}

/// The argument position of `at` in `list` (0-based, not counting the head).
///
/// `None` when the cursor is on or before the head. The rule is the one
/// [`crate::locate`] uses for [`crate::Cursor::Arg`]: just after an argument
/// (still typing it) or on the whitespace before it is that argument's
/// index; after the last argument is the next index. The reader's `Error`
/// placeholder at the end of an unclosed list is not an argument.
#[must_use]
pub fn arg_index(list: &Node, at: ByteOffset) -> Option<usize> {
    let (head, args) = real_items(list).split_first()?;
    if at <= head.span.end {
        return None;
    }
    Some(args.iter().position(|arg| at <= arg.span.end).unwrap_or(args.len()))
}

/// The spans of the `;` comments inside the code forms of the note (file 0).
///
/// In source order. The reader drops comments, so they are found in the
/// text: a `;` outside a string literal, up to the end of its line (without
/// the line break or a `\r` before it).
#[must_use]
pub fn comment_spans(analysis: &Analysis) -> Vec<Span> {
    let text = note_text(analysis);
    let mut out = Vec::new();
    for (form, _) in code_forms(analysis) {
        form_comments(text, form, &mut out);
    }
    out
}

fn form_comments(text: &str, root: &Node, out: &mut Vec<Span>) {
    let mut strings = Vec::new();
    string_spans(root, &mut strings);
    strings.sort_by_key(|s| s.start);
    let bytes = text.as_bytes();
    let end = root.span.end.to_usize().min(bytes.len());
    let mut pos = root.span.start.to_usize();
    let mut next = strings.iter().peekable();
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
}

/// The `let` binding of `name` in scope at `at`.
///
/// A variable is defined once per note, so this is the one `let` of that
/// name, if its form *ends* at or before `at`. A `let` takes effect after it
/// is written, so inside `(let x (+ x 1))` the `x` of the value is not bound
/// by that `let`. This is exactly what `analyze` substituted there, including
/// a `let` refused by the document's node budget, whose binding is in
/// [`Analysis::lets`] with an `Error` value (its variable is an error from
/// then on). A `let` that was rejected outright (bad shape, a reserved name,
/// a value too large, a second definition) binds nothing.
#[must_use]
pub fn binding_at(analysis: &Analysis, name: Sym, at: ByteOffset) -> Option<&Binding> {
    analysis
        .lets
        .iter()
        .find(|b| b.name == name && b.span.file == NOTE && b.span.end <= at)
}

/// Every binding in scope at `at` (see [`binding_at`]), in document order.
#[must_use]
pub fn bindings_in_scope(analysis: &Analysis, at: ByteOffset) -> Vec<&Binding> {
    analysis
        .lets
        .iter()
        .filter(|b| b.span.file == NOTE && b.span.end <= at)
        .collect()
}

/// The name node of a definition or tag form as written: the `x` of
/// `(let x v)` and `(alias x v)`, the `name` of `(macro (name ...) ...)`,
/// the `name` of `(tag name)`. `None` when the form has no such name.
#[must_use]
pub fn name_node(form: &Node) -> Option<&Node> {
    let first = form.args().first()?;
    let name = if form.is_form("macro") {
        first.as_list()?.first()?
    } else if form.is_form("let") || form.is_form("alias") || form.is_form("tag") {
        first
    } else {
        return None;
    };
    name.as_ident().is_some().then_some(name)
}

/// The span of just the name in a `let`, `alias` or `macro` form, or in a
/// `(tag name)` (see [`name_node`]).
#[must_use]
pub fn name_span(form: &Node) -> Option<Span> {
    name_node(form).map(|n| n.span)
}

/// The written code form of the note whose span is exactly `span` (for
/// example [`crate::Macro::span`] or [`Binding::span`]).
#[must_use]
pub fn form_at(analysis: &Analysis, span: Span) -> Option<&Node> {
    if span.file != NOTE {
        return None;
    }
    code_forms(analysis).into_iter().map(|(form, _)| form).find(|form| form.span == span)
}

/// The span of the defined name in the note's definition form at `form_span`.
///
/// `form_span` is a [`crate::Macro::span`] or a [`Binding::span`]; the name
/// is the `x` of `(let x v)`, the `name` of `(macro (name ...) ...)`. `None` for a
/// definition outside the note (the embedded prelude).
#[must_use]
pub fn definition_name_span(analysis: &Analysis, form_span: Span) -> Option<Span> {
    form_at(analysis, form_span).and_then(name_span)
}

/// The display `show` whose leading `(tag ...)` defined `label`, as written.
#[must_use]
pub fn tagged_equation<'a>(analysis: &'a Analysis, label: &Label) -> Option<&'a Node> {
    code_forms(analysis)
        .into_iter()
        .map(|(form, _)| form)
        .find(|form| form.is_form("show") && form.args().first().is_some_and(|tag| tag.span == label.span))
}

/// The span of the tagged display equation of `label` (the whole `show`).
#[must_use]
pub fn equation_span(analysis: &Analysis, label: &Label) -> Option<Span> {
    tagged_equation(analysis, label).map(|form| form.span)
}

/// The span of just the name in the `(tag name)` that defined `label`.
#[must_use]
pub fn label_name_span(analysis: &Analysis, label: &Label) -> Option<Span> {
    tagged_equation(analysis, label)?.args().first().and_then(name_span)
}

#[cfg(test)]
mod tests;
