//! The reader: text to [`Node`].
//!
//! Rules (see `docs/ARCHITECTURE.md`, "S-expression tree and reader"):
//! - `(` and `)` are the only delimiters; a paren always ends an atom.
//! - Number: starts with a digit, or `-` directly followed by a digit; optional
//!   fraction and exponent; stored exactly as written.
//! - `-name` (a minus directly followed by a letter) reads as `(- name)`;
//!   any other atom starting with `-` is an ordinary identifier.
//! - `'x` reads as `(quote x)`.
//! - `"..."` is a `Str` node; escapes are `\"` and `\\`.
//! - `;` to end of line is a comment.
//! - An unclosed `(` yields an `Error` node (or a list containing one) plus a
//!   diagnostic pointing at the opening paren. The reader never panics and
//!   always returns a tree.
//!
//! Details this module settles:
//! - An atom is a maximal run of characters that are not whitespace and not a
//!   paren. It is a `Number` only if the *whole* run matches
//!   `-?digits(.digits)?([eE][+-]?digits)?` (ASCII digits); otherwise it is an
//!   `Ident`. So `2x` and `1.` are identifiers, not a number glued to a name.
//! - `-name`: an atom that starts with `-` followed directly by an alphabetic
//!   character (a Unicode letter) reads as `(- rest)`, where `rest` is the atom
//!   without its minus. Every other atom is an ordinary identifier, so `-`,
//!   `->`, `-->`, `->>`, `--x` and `-_` stay single identifiers. `-3` and
//!   `-3.5e2` are numbers. `-(+ a b)` is the identifier `-` followed by a
//!   list. The synthesized `-` has the span of the minus sign; the list spans
//!   the whole atom.
//! - An atom also ends before a `"` (which starts a string) and before a `'`
//!   that is followed by something to quote, so `(show'x)` is
//!   `(show (quote x))`. A run of `'` at the very end of an atom is part of it,
//!   so primes work: `f'` and `f''` are identifiers. `;` starts a comment only
//!   at the start of a token; inside an atom it is an ordinary character.
//! - `'` with nothing to quote (end of input or a `)`) reads as
//!   `(quote <error>)` with a diagnostic.
//! - In a string, `\"` and `\\` are escapes; any other backslash is kept as
//!   written. A string with no closing quote ends at the end of its line and
//!   gets an `UnterminatedString` diagnostic.
//! - An unclosed `(` reads as the list closed at the end of the input. Every
//!   unclosed `(` gets its own `UnclosedParen` diagnostic spanning that paren,
//!   and the innermost unclosed list gets an `Error` node as its last child.

use crate::diag::{Diag, DiagCode, Diags};
use crate::node::Node;
use crate::span::{ByteOffset, FileId, Span};

/// The result of reading one form out of a larger text.
#[derive(Clone, Debug)]
pub struct Read {
    pub node: Node,
    /// The offset just past the form (past its closing paren).
    pub end: ByteOffset,
}

/// Reads exactly one form starting at byte `start` of `text`, which must be the
/// position of an opening `(`. Spans in the result are offsets into `text`,
/// tagged with `file`.
///
/// This is what the markdown parser calls when it meets a reserved head.
pub fn read_form(text: &str, file: FileId, start: ByteOffset, diags: &mut Diags) -> Read {
    let whole = [Seg::new(0, text.len(), text.len())];
    read_in(text, file, start.to_usize(), &whole, diags)
}

/// Reads a whole string as a single form. Convenience for tests and for
/// keyword examples; spans use `FileId(0)`. Text after the first form is
/// reported as a diagnostic.
pub fn read_str(src: &str, diags: &mut Diags) -> Node {
    let file = FileId(0);
    let whole = [Seg::new(0, src.len(), src.len())];
    let mut reader = Reader::new(src, file, &whole, 0);
    reader.skip_trivia();
    if reader.peek() == Peek::Eof {
        let span = reader.span(src.len(), src.len());
        diags.push(Diag::error(DiagCode::Other, span, "expected a form, found nothing"));
        return Node::error(span);
    }
    let node = reader.read(diags);
    reader.skip_trivia();
    if reader.peek() != Peek::Eof {
        let span = reader.span(reader.cur.pos, src.len());
        diags.push(
            Diag::error(DiagCode::Other, span, "unexpected text after the form")
                .with_help("wrap several forms in one list"),
        );
    }
    node
}

/// One line (or other contiguous piece) of text the reader may look at.
///
/// The markdown parser hands the reader the content of each line with any
/// container prefix (`> ` of a block quote, list indentation) cut away. The
/// bytes between two segments are read as a single newline, and the end of the
/// last segment is the end of the input. Offsets stay absolute offsets into
/// the full text, so spans need no adjusting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Seg {
    /// First byte of the content.
    pub start: usize,
    /// One past the last byte of the content (before any `\r\n`).
    pub end: usize,
    /// One past the line terminator (the start of the next physical line).
    /// Unused by the reader; the markdown parser uses it for newline spans.
    pub eol: usize,
}

impl Seg {
    pub(crate) const fn new(start: usize, end: usize, eol: usize) -> Self {
        Self { start, end, eol }
    }
}

/// Reads one form at `start` from text restricted to `segs`. `segs` must be in
/// increasing order and on char boundaries; `start` should lie in `segs[0]`.
/// A bad `start` yields an `Error` node and a diagnostic, never a panic.
pub(crate) fn read_in(text: &str, file: FileId, start: usize, segs: &[Seg], diags: &mut Diags) -> Read {
    let valid = text.is_char_boundary(start)
        && segs.first().is_some_and(|s| s.start <= start && start <= s.end && s.end <= text.len());
    if !valid {
        let at = floor_boundary(text, start);
        let span = Span::new(file, ByteOffset::from_usize(at), ByteOffset::from_usize(at));
        diags.push(Diag::error(DiagCode::Other, span, "expected a form here"));
        return Read {
            node: Node::error(span),
            end: ByteOffset::from_usize(at),
        };
    }
    let mut reader = Reader::new(text, file, segs, start);
    let node = reader.read(diags);
    Read {
        node,
        end: ByteOffset::from_usize(reader.cur.pos),
    }
}

fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// How deeply lists and quotes may nest. Deeper input becomes an `Error` node,
/// so trees stay shallow enough for the recursive code that walks (and drops)
/// them.
const MAX_NESTING: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Peek {
    Char(char),
    /// Between two segments: reads as a newline.
    Gap,
    Eof,
}

impl Peek {
    /// Whether this ends an atom.
    const fn ends_atom(self) -> bool {
        match self {
            Self::Char(c) => c.is_whitespace() || matches!(c, '(' | ')' | '"'),
            Self::Gap | Self::Eof => true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Cursor {
    /// Index of the current segment.
    seg: usize,
    /// Absolute byte offset.
    pos: usize,
}

enum Frame {
    List { open: usize, items: Vec<Node> },
    Quote { at: usize },
}

struct Reader<'a> {
    text: &'a str,
    file: FileId,
    segs: &'a [Seg],
    cur: Cursor,
}

impl<'a> Reader<'a> {
    const fn new(text: &'a str, file: FileId, segs: &'a [Seg], start: usize) -> Self {
        Self {
            text,
            file,
            segs,
            cur: Cursor { seg: 0, pos: start },
        }
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, ByteOffset::from_usize(start), ByteOffset::from_usize(end))
    }

    fn peek(&self) -> Peek {
        let Some(seg) = self.segs.get(self.cur.seg) else {
            return Peek::Eof;
        };
        if self.cur.pos < seg.end {
            return self
                .text
                .get(self.cur.pos..seg.end)
                .and_then(|s| s.chars().next())
                .map_or(Peek::Eof, Peek::Char);
        }
        if self.cur.seg + 1 < self.segs.len() {
            Peek::Gap
        } else {
            Peek::Eof
        }
    }

    fn bump(&mut self) {
        match self.peek() {
            Peek::Char(c) => self.cur.pos += c.len_utf8(),
            Peek::Gap => {
                self.cur.seg += 1;
                self.cur.pos = self.segs[self.cur.seg].start.max(self.cur.pos);
            }
            Peek::Eof => {}
        }
    }

    /// Skips whitespace, segment gaps and `;` comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Peek::Gap => self.bump(),
                Peek::Char(c) if c.is_whitespace() => self.bump(),
                Peek::Char(';') => {
                    while let Peek::Char(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => return,
            }
        }
    }

    /// Reads one complete form. Uses an explicit stack, so deeply nested input
    /// cannot overflow the call stack.
    fn read(&mut self, diags: &mut Diags) -> Node {
        let mut stack: Vec<Frame> = Vec::new();
        loop {
            self.skip_trivia();
            let at = self.cur.pos;
            let mut node = match self.peek() {
                Peek::Eof | Peek::Gap => return self.unwind(stack, diags),
                Peek::Char('(') if stack.len() >= MAX_NESTING => self.too_deep(diags),
                Peek::Char('(') => {
                    self.bump();
                    stack.push(Frame::List {
                        open: at,
                        items: Vec::new(),
                    });
                    continue;
                }
                Peek::Char('\'') if stack.len() >= MAX_NESTING => {
                    self.bump();
                    let span = self.span(at, at + 1);
                    diags.push(Diag::error(DiagCode::Other, span, "forms are nested too deeply"));
                    continue;
                }
                Peek::Char('\'') => {
                    self.bump();
                    stack.push(Frame::Quote { at });
                    continue;
                }
                Peek::Char(')') => match stack.pop() {
                    Some(Frame::List { open, items }) => {
                        self.bump();
                        Node::list(items, self.span(open, self.cur.pos))
                    }
                    Some(Frame::Quote { at: quote_at }) => {
                        // Leave the `)` for the enclosing list.
                        let missing = self.nothing_to_quote(quote_at, at, diags);
                        quote(self.span(quote_at, quote_at + 1), missing)
                    }
                    None => {
                        self.bump();
                        let span = self.span(at, self.cur.pos);
                        diags.push(Diag::error(DiagCode::Other, span, "unexpected `)`"));
                        Node::error(span)
                    }
                },
                Peek::Char('"') => self.string(diags),
                Peek::Char(_) => self.atom(),
            };
            // Hand the finished node to whatever is waiting for it.
            loop {
                match stack.last_mut() {
                    Some(Frame::List { items, .. }) => {
                        items.push(node);
                        break;
                    }
                    Some(Frame::Quote { at }) => {
                        let at = *at;
                        stack.pop();
                        node = quote(self.span(at, at + 1), node);
                    }
                    None => return node,
                }
            }
        }
    }

    /// Skips a parenthesised group that would nest deeper than
    /// [`MAX_NESTING`], leaving one `Error` node for all of it.
    fn too_deep(&mut self, diags: &mut Diags) -> Node {
        let start = self.cur.pos;
        let mut depth = 0usize;
        loop {
            match self.peek() {
                Peek::Char('(') => depth += 1,
                Peek::Char(')') => {
                    depth -= 1;
                    if depth == 0 {
                        self.bump();
                        break;
                    }
                }
                Peek::Eof => break,
                Peek::Char(_) | Peek::Gap => {}
            }
            self.bump();
        }
        let span = self.span(start, self.cur.pos);
        diags.push(
            Diag::error(DiagCode::Other, span, "forms are nested too deeply")
                .with_help(format!("at most {MAX_NESTING} levels of parentheses are allowed")),
        );
        Node::error(span)
    }

    /// `'` followed by nothing quotable: reports it and returns the `Error`
    /// node that stands in for the missing form.
    fn nothing_to_quote(&self, quote_at: usize, here: usize, diags: &mut Diags) -> Node {
        let mark = self.span(quote_at, quote_at + 1);
        diags.push(Diag::error(DiagCode::Other, mark, "nothing to quote after `'`"));
        Node::error(self.span(here, here))
    }

    /// Closes everything still open at the end of the input.
    fn unwind(&self, stack: Vec<Frame>, diags: &mut Diags) -> Node {
        let eof = self.cur.pos;
        let mut pending: Option<Node> = None;
        let mut innermost = true;
        for frame in stack.into_iter().rev() {
            match frame {
                Frame::Quote { at } => {
                    let inner = pending
                        .take()
                        .unwrap_or_else(|| self.nothing_to_quote(at, eof, diags));
                    pending = Some(quote(self.span(at, at + 1), inner));
                }
                Frame::List { open, mut items } => {
                    items.extend(pending.take());
                    if innermost {
                        items.push(Node::error(self.span(eof, eof)));
                        innermost = false;
                    }
                    let paren = self.span(open, open + 1);
                    diags.push(
                        Diag::error(DiagCode::UnclosedParen, paren, "this `(` is never closed")
                            .with_help("add a matching `)`"),
                    );
                    pending = Some(Node::list(items, self.span(open, eof)));
                }
            }
        }
        pending.unwrap_or_else(|| {
            let span = self.span(eof, eof);
            diags.push(Diag::error(DiagCode::Other, span, "expected a form, found the end of the input"));
            Node::error(span)
        })
    }

    fn atom(&mut self) -> Node {
        let start = self.cur.pos;
        loop {
            match self.peek() {
                Peek::Char('\'') if self.cur.pos > start => {
                    // Trailing primes (`f'`, `f''`) belong to the atom; a `'`
                    // followed by anything else starts a quote.
                    let mut ahead = Reader { cur: self.cur, ..*self };
                    while ahead.peek() == Peek::Char('\'') {
                        ahead.bump();
                    }
                    if ahead.peek().ends_atom() {
                        self.cur = ahead.cur;
                    }
                    break;
                }
                peek if peek.ends_atom() => break,
                _ => self.bump(),
            }
        }
        atom_node(&self.text[start..self.cur.pos], start, self.file)
    }

    fn string(&mut self, diags: &mut Diags) -> Node {
        let open = self.cur.pos;
        let saved = self.cur;
        self.bump();
        if let Some(value) = self.string_body(false) {
            return Node::string(&value, self.span(open, self.cur.pos));
        }
        // Unterminated: take the rest of the line and carry on after it.
        self.cur = saved;
        self.bump();
        let value = self.string_body(true).unwrap_or_default();
        let span = self.span(open, self.cur.pos);
        diags.push(
            Diag::error(DiagCode::UnterminatedString, span, "this string is never closed")
                .with_help("add a closing `\"`"),
        );
        Node::string(&value, span)
    }

    /// Reads string contents after the opening quote. Returns `None` if the
    /// closing quote is missing; with `line_only` it instead stops at the end
    /// of the line and returns what it has.
    fn string_body(&mut self, line_only: bool) -> Option<String> {
        let mut out = String::new();
        loop {
            match self.peek() {
                Peek::Char('"') => {
                    self.bump();
                    return Some(out);
                }
                Peek::Char('\\') => {
                    self.bump();
                    match self.peek() {
                        Peek::Char(c @ ('"' | '\\')) => {
                            out.push(c);
                            self.bump();
                        }
                        _ => out.push('\\'),
                    }
                }
                Peek::Char('\n') | Peek::Gap if line_only => return Some(out),
                Peek::Char(c) => {
                    out.push(c);
                    self.bump();
                }
                Peek::Gap => {
                    out.push('\n');
                    self.bump();
                }
                Peek::Eof => return line_only.then_some(out),
            }
        }
    }
}

fn quote(mark: Span, inner: Node) -> Node {
    let span = Span::new(mark.file, mark.start, inner.span.end.max(mark.end));
    Node::list(vec![Node::ident("quote", mark), inner], span)
}

/// Builds the node for one atom `tok` that starts at byte `start`.
fn atom_node(tok: &str, start: usize, file: FileId) -> Node {
    let span = |a: usize, b: usize| Span::new(file, ByteOffset::from_usize(a), ByteOffset::from_usize(b));
    // `-name` is `(- name)` only when the minus is directly followed by a letter.
    if tok.strip_prefix('-').and_then(|rest| rest.chars().next()).is_some_and(char::is_alphabetic) {
        let name = Node::ident(&tok[1..], span(start + 1, start + tok.len()));
        let sign = Node::ident("-", span(start, start + 1));
        return Node::list(vec![sign, name], span(start, start + tok.len()));
    }
    if is_number(tok) {
        Node::number(tok, span(start, start + tok.len()))
    } else {
        Node::ident(tok, span(start, start + tok.len()))
    }
}

/// `-?digits(.digits)?([eE][+-]?digits)?`, ASCII digits only.
fn is_number(tok: &str) -> bool {
    fn digits(s: &[u8]) -> usize {
        s.iter().take_while(|b| b.is_ascii_digit()).count()
    }
    let b = tok.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'-'));
    let whole = digits(&b[i..]);
    if whole == 0 {
        return false;
    }
    i += whole;
    if b.get(i) == Some(&b'.') {
        let frac = digits(&b[i + 1..]);
        if frac == 0 {
            return false;
        }
        i += 1 + frac;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp = digits(&b[i..]);
        if exp == 0 {
            return false;
        }
        i += exp;
    }
    i == b.len()
}

#[cfg(test)]
mod tests;
