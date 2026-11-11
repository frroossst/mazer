//! "What is under the cursor?" for the LSP.

use mazer_syntax::{ByteOffset, Node};

use crate::pipeline::Analysis;

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
    let _ = (analysis, at);
    todo!("step 2, document agent")
}
