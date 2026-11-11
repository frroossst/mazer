//! The reader: text to [`Node`].
//!
//! Rules (see `docs/ARCHITECTURE.md`, "S-expression tree and reader"):
//! - `(` and `)` are the only delimiters; a paren always ends an atom.
//! - Number: starts with a digit, or `-` directly followed by a digit; optional
//!   fraction and exponent; stored exactly as written.
//! - `-name` (minus glued to a non-digit) reads as `(- name)`.
//! - `'x` reads as `(quote x)`.
//! - `"..."` is a `Str` node; escapes are `\"` and `\\`.
//! - `;` to end of line is a comment.
//! - An unclosed `(` yields an `Error` node (or a list containing one) plus a
//!   diagnostic pointing at the opening paren. The reader never panics and
//!   always returns a tree.

use crate::diag::Diags;
use crate::node::Node;
use crate::span::{ByteOffset, FileId};

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
    let _ = (text, file, start, diags);
    todo!("step 1, parser agent")
}

/// Reads a whole string as a single form. Convenience for tests and for
/// keyword examples; spans use `FileId(0)`. Text after the first form is
/// reported as a diagnostic.
pub fn read_str(src: &str, diags: &mut Diags) -> Node {
    let _ = (src, diags);
    todo!("step 1, parser agent")
}
