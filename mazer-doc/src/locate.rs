//! "What is under the cursor?" for the LSP.
//!
//! # Rules
//!
//! `at` is a byte offset into the note; the cursor sits *before* the byte at
//! `at` (so `at == end` of a word means "just after it", where completion
//! happens, and `start <= at < end` means "on it", where hover happens).
//!
//! - Outside every code form: [`Cursor::Prose`]. A form counts from just
//!   after its `(` to just before its `)`; on the `(` itself or after the `)`
//!   the cursor is outside. An unclosed form (typed mid-edit) runs to the end
//!   of what the reader took, so a cursor at its very end is inside.
//! - Inside a `;` comment of a code form (after the `;`, up to and including
//!   the end of its line) or between the quotes of a string literal (after
//!   the opening quote, up to and including the position before the closing
//!   one, or the end of an unclosed string): [`Cursor::Prose`], so the editor
//!   offers no code completion there.
//! - Inside a list, the *innermost* list containing the cursor decides.
//! - On the head identifier, immediately after it, or anywhere between the
//!   `(` and the head (including an empty `()` or a lone `(`): [`Cursor::Head`].
//!   Reserved heads (`show`, `let`, ...) count as heads.
//! - Inside a `(ref ...)` after its head: [`Cursor::Ref`], with `partial`
//!   the text of the name from its start up to the cursor (empty when the
//!   cursor is not on a name).
//! - On an argument that is an identifier (`start <= at < end`):
//!   [`Cursor::Name`]. This includes the name inside `'x` and the name in
//!   `-x`.
//! - Elsewhere among the arguments: [`Cursor::Arg`] with the 0-based index of
//!   the argument the cursor is in or about to start. Just after an argument
//!   (`at == end`, still typing it) is that argument's index; on whitespace
//!   before an argument is that argument's index; after the last argument is
//!   the next index. Numbers, the edges of a string (just before its opening
//!   quote, just after its closing one) and the `'` of a quote count as "in"
//!   an argument too; so does the position just before a nested list's `(` or
//!   just after its `)`.
//! - The `Error` placeholder the reader leaves at the end of an unclosed list
//!   is not an argument.

use mazer_syntax::{ByteOffset, FileId, Node, NodeKind};

use crate::pipeline::Analysis;
use crate::walk;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cursor<'a> {
    /// Ordinary text. Nothing to offer except emoji shortcodes.
    Prose,
    /// Typing the first word of a list: offer keywords and macros.
    Head { list: &'a Node },
    /// Inside the arguments of a call: signature help and variables. `index`
    /// is the 0-based argument position (not counting the head).
    Arg { list: &'a Node, index: usize },
    /// On an existing name: hover and go-to-definition.
    Name { ident: &'a Node },
    /// Inside `(ref ...)`: offer label names. `partial` is what has been typed.
    Ref { partial: &'a str },
}

/// Finds what is at byte offset `at` of the note (file 0), using the tree
/// exactly as typed ([`Analysis::written`]).
#[must_use]
pub fn locate(analysis: &Analysis, at: ByteOffset) -> Cursor<'_> {
    let text = analysis.sources.file(FileId(0)).map_or("", |f| f.text.as_str());
    let finder = Finder { text, at };
    let mut found = None;
    walk::for_each_code(&analysis.written, &mut |form, _| {
        if found.is_none() && form.span.file == FileId(0) && finder.inside(form) {
            found = Some(form);
        }
    });
    match found {
        Some(form) if !finder.in_comment_or_string(form) => finder.in_list(form),
        _ => Cursor::Prose,
    }
}

struct Finder<'t> {
    text: &'t str,
    at: ByteOffset,
}

impl<'t> Finder<'t> {
    /// Whether `node` is a list written with parentheses (not the `'x` or
    /// `-x` shorthand the reader expands into a list).
    fn is_paren_list(&self, node: &Node) -> bool {
        matches!(node.kind, NodeKind::List(_)) && self.text.as_bytes().get(node.span.start.to_usize()) == Some(&b'(')
    }

    /// Whether the cursor is between the parentheses of a paren list.
    fn inside(&self, node: &Node) -> bool {
        self.is_paren_list(node)
            && node.span.start < self.at
            && (self.at < node.span.end || (self.at == node.span.end && is_unclosed(node)))
    }

    fn in_list<'a>(&self, list: &'a Node) -> Cursor<'a>
    where
        't: 'a,
    {
        let items = real_items(list);
        let Some((head, args)) = items.split_first() else {
            return Cursor::Head { list };
        };
        if self.at <= head.span.end {
            if self.inside(head) {
                return self.in_list(head);
            }
            return Cursor::Head { list };
        }
        if list.is_form("ref") {
            return Cursor::Ref {
                partial: self.ref_partial(args),
            };
        }
        for (index, arg) in args.iter().enumerate() {
            if self.at < arg.span.start {
                return Cursor::Arg { list, index };
            }
            if self.at <= arg.span.end {
                return self.on_arg(list, index, arg);
            }
        }
        Cursor::Arg {
            list,
            index: args.len(),
        }
    }

    /// The cursor is within `arg` (start and end inclusive).
    fn on_arg<'a>(&self, list: &'a Node, index: usize, arg: &'a Node) -> Cursor<'a>
    where
        't: 'a,
    {
        if self.inside(arg) {
            return self.in_list(arg);
        }
        if self.on(arg) && arg.as_ident().is_some() {
            return Cursor::Name { ident: arg };
        }
        if !self.is_paren_list(arg)
            && let Some(items) = arg.as_list()
        {
            // `'x` or `-x`: look at what was written after the mark.
            for item in items {
                if self.inside(item) {
                    return self.in_list(item);
                }
                if self.on(item) && item.as_ident().is_some() && item.span.start != arg.span.start {
                    return Cursor::Name { ident: item };
                }
                if let Some(inner) = item.as_list()
                    && self.on(item)
                {
                    // `'-x`: one more level of shorthand.
                    if let Some(name) = inner.iter().find(|n| self.on(n) && n.as_ident().is_some() && n.span.start != item.span.start) {
                        return Cursor::Name { ident: name };
                    }
                }
            }
        }
        Cursor::Arg { list, index }
    }

    /// Whether the cursor is inside a `;` comment or between the quotes of a
    /// string literal of `form`. The reader drops comments, so they are found
    /// in the source text; strings are the `Str` nodes of the tree, whose
    /// spans are exactly what the reader took (an unclosed string runs to the
    /// end of its line, and a cursor at its very end is inside it).
    fn in_comment_or_string(&self, form: &Node) -> bool {
        let mut strings = Vec::new();
        string_spans(form, &mut strings);
        strings.sort_by_key(|s| s.start);
        for span in &strings {
            let unclosed = !self.is_closed_string(span.range());
            if span.start < self.at && (self.at < span.end || (self.at == span.end && unclosed)) {
                return true;
            }
        }
        let bytes = self.text.as_bytes();
        let at = self.at.to_usize();
        let mut pos = form.span.start.to_usize();
        let mut next = strings.iter().peekable();
        while pos < at && pos < bytes.len() {
            if let Some(span) = next.next_if(|s| s.start.to_usize() <= pos) {
                pos = pos.max(span.end.to_usize());
                continue;
            }
            if bytes[pos] == b';' {
                let line_end = bytes[pos..].iter().position(|&b| b == b'\n').map_or(bytes.len(), |n| pos + n);
                if at <= line_end {
                    return true;
                }
                pos = line_end;
            }
            pos += 1;
        }
        false
    }

    /// Whether the string literal written at `range` ends with its own
    /// closing quote.
    fn is_closed_string(&self, range: std::ops::Range<usize>) -> bool {
        let Some(body) = self.text.as_bytes().get(range).and_then(|b| b.get(1..)) else {
            return false;
        };
        let mut escaped = false;
        for (i, &b) in body.iter().enumerate() {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => return i + 1 == body.len(),
                _ => {}
            }
        }
        false
    }

    /// Whether the cursor is on `node`: `start <= at < end`.
    fn on(&self, node: &Node) -> bool {
        node.span.start <= self.at && self.at < node.span.end
    }

    /// What has been typed of the name in a `(ref ...)` up to the cursor.
    fn ref_partial(&self, args: &[Node]) -> &'t str {
        args.iter()
            .find(|a| a.span.start <= self.at && self.at <= a.span.end)
            .filter(|a| a.as_ident().is_some())
            .and_then(|a| self.text.get(a.span.start.to_usize()..self.at.to_usize()))
            .unwrap_or("")
    }
}

/// Whether a list was left open: its last child reaches its end (the reader's
/// `Error` placeholder, or an unclosed inner list).
fn is_unclosed(node: &Node) -> bool {
    node.as_list()
        .and_then(<[Node]>::last)
        .is_some_and(|last| last.span.end == node.span.end && (matches!(last.kind, NodeKind::Error) || is_unclosed(last)))
}

/// The spans of every string literal in `node`.
fn string_spans(node: &Node, out: &mut Vec<mazer_syntax::Span>) {
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

/// The children of a list without the reader's trailing `Error` placeholder.
fn real_items(list: &Node) -> &[Node] {
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

#[cfg(test)]
mod tests;
