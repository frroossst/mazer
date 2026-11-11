//! The markdown parser: note text to [`Document`].
//!
//! Features (see `docs/ARCHITECTURE.md`, "Document tree"):
//! - Blocks: headings, paragraphs, bullet / numbered / nested lists, checklist
//!   items (`-[ ]`, `-[x]`), block quotes, fenced code blocks, images, `---`.
//! - Inlines: bold, italic, underline, strikethrough, spoiler (`||..||`),
//!   links, images, inline code, emoji shortcodes, line breaks. Styles nest.
//! - Code: a parenthesis is code only if it opens with one of
//!   [`crate::RESERVED_HEADS`]; it is then read with [`crate::reader::read_form`].
//!   Alone in its paragraph it becomes `BlockKind::Code`; inside a sentence it
//!   becomes `InlineKind::Code`. Every other parenthesis is prose.
//! - Nothing inside a fenced code block or inline code is interpreted.
//! - There is no pipe-table syntax.
//!
//! The parser never fails: problems are pushed to `diags` and parsing goes on.

use crate::diag::Diags;
use crate::doc::Document;
use crate::span::FileId;

/// Parses a whole note. Spans are byte offsets into `text`, tagged with `file`.
pub fn parse(text: &str, file: FileId, diags: &mut Diags) -> Document {
    let _ = (text, file, diags);
    todo!("step 1, parser agent")
}
