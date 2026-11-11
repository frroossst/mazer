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
//!
//! # Decisions
//!
//! Where the design leaves a detail open, the parser follows CommonMark unless
//! a note below says otherwise.
//!
//! **Lines and paragraphs.** Lines end at `\n` (a `\r` before it is dropped).
//! A blank line (only spaces and tabs) ends a paragraph. A paragraph is also
//! ended by a line that starts a heading, a fence, a rule, a block quote, an
//! image line, a bullet item, or a numbered item that starts at `1`; a
//! numbered line like `2003. was a year` inside a paragraph stays text.
//! Leading whitespace of every paragraph line is dropped.
//!
//! **Line breaks.** A newline inside a paragraph is a *soft* break: it stays
//! in the surrounding [`InlineKind::Text`] as `'\n'` and the renderer decides
//! how to show it. A line ending in two or more spaces, or in a backslash, is a
//! *hard* break: [`InlineKind::LineBreak`]. Trailing whitespace on the last
//! line of a paragraph or heading is dropped.
//!
//! **Code forms span lines.** A reserved-head form is read as one unit even
//! across many lines and blank lines; the lines it covers belong to the block
//! where the form starts, whatever they look like. If the form is never
//! closed, the reader is re-run on the text up to the next blank line, so a
//! missing `)` damages one paragraph, not the rest of the note. The
//! `UnclosedParen` diagnostic still points at the opening paren. A form must
//! be followed by whitespace (ASCII space, tab, newline), `(`, `)`, `'` or `"`
//! after its head: `(show` at the very end of the input is prose.
//! `BlockKind::Code` is produced only for top-level paragraphs; inside a list
//! item or block quote a form alone in its paragraph is still
//! `InlineKind::Code` in a one-element paragraph.
//!
//! **Headings.** `#` to `######` followed by a space, a tab or the end of the
//! line. `####### x` (seven) is a paragraph. A closing run of `#`s preceded by
//! a space is removed. There are no setext (underlined) headings: `---` under
//! a line of text is always a rule.
//!
//! **Rules.** Three or more `-`, `*` or `_` (all the same character), with
//! optional spaces between, and nothing else on the line.
//!
//! **Lists.** Bullets are `-`, `*` or `+` followed by a space, a tab or the end
//! of the line; they all make the same kind of list. Numbered items are 1 to 9
//! digits followed by `.` or `)` and a space/tab/end of line; the first item's
//! number is the list's start. Checklist items are `-[ ]`, `-[x]`, `-[X]` (the
//! old syntax, with or without a space after `]`) or a bullet followed by
//! `[ ]`, `[x]` or `[X]` and a space (GitHub style). Checklist and plain bullet
//! items mix in one list. Blank lines between items do not end a list.
//!
//! **List nesting.** An item's *content column* is the column of the first
//! character after the marker and the spaces that follow it (`- a` -> 2,
//! `1. a` -> 3). Following lines indented to at least the content column
//! belong to the item; a list marker on such a line therefore starts a nested
//! list. A tab advances to the next multiple of 4 columns. There is no lazy
//! continuation: a non-blank line indented less than the content column ends
//! the item (and the list, unless it is another item), so
//! `- claim\nanswer` is a list followed by a paragraph.
//!
//! **Block quotes.** Consecutive lines starting with `>` (after any
//! indentation); one space or tab after the `>` is dropped. A line without `>`
//! ends the quote (no lazy continuation), as does a blank line, so two quotes
//! separated by a blank line are two blocks. Quotes contain blocks.
//!
//! **Fences.** Three or more backticks or tildes open a fence; the first word
//! after them is the language. It closes on a line holding at least as many of
//! the same character and nothing else. An unclosed fence runs to the end of
//! its container. Content lines lose up to the fence's own indentation.
//!
//! **Images.** A line holding only `![alt](url)` is a `BlockKind::Image`; it
//! also ends a paragraph. Elsewhere `![alt](url)` is an `InlineKind::Image`.
//! The alt text is plain text.
//!
//! **Emphasis markers** (the old parser's): `**bold**`, `*italic*`,
//! `_underline_`, `~strike~` (`~~strike~~` too), `||spoiler||`. They nest. An
//! opener must be followed by a non-space and needs a matching closer later in
//! the same paragraph (or link text), otherwise it is literal text. A closer
//! must follow a non-space. `*` (single) and `_` directly after a letter or
//! digit are literal (`lambda*I`, `snake_case`); `_` also only closes when not
//! followed by a letter or digit. A run of three or more `~` is literal.
//!
//! **Links.** `[text](url)`; the text is parsed as inlines, the URL is taken
//! verbatim, may contain balanced parentheses (`Magma_(algebra)`) and may not
//! contain whitespace. Anything that does not complete a link is literal text.
//!
//! **Inline code.** A run of N backticks up to the next run of exactly N
//! backticks; may span lines (newlines become spaces); one leading and one
//! trailing space are stripped when both are present. Unmatched backticks are
//! literal.
//!
//! **Escapes.** A backslash before an ASCII punctuation character makes that
//! character literal (`\*`, `\(show`); any other backslash is literal.
//!
//! **Emoji.** `:name:` is an emoji only when [`crate::emoji::lookup`] knows
//! `name`; names are made of ASCII letters, digits, `_`, `+` and `-`.
//!
//! **Limits.** Block containers nest at most 32 deep and inline styles/links
//! at most 32 deep; deeper markers are treated as text. This keeps recursion
//! bounded on hostile input.

mod block;
mod inline;

use crate::diag::{DiagCode, Diags};
use crate::doc::Document;
use crate::reader::{self, Read, Seg};
use crate::span::FileId;

/// Parses a whole note. Spans are byte offsets into `text`, tagged with `file`.
pub fn parse(text: &str, file: FileId, diags: &mut Diags) -> Document {
    let lines = split_lines(text);
    let mut parser = block::BlockParser::new(text, file, diags);
    Document {
        blocks: parser.blocks(&lines, 0),
    }
}

/// Splits `text` into lines. A leading byte-order mark is skipped.
fn split_lines(text: &str) -> Vec<Seg> {
    let mut lines = Vec::new();
    let mut start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let bytes = text.as_bytes();
    while start < text.len() {
        let nl = bytes[start..].iter().position(|&b| b == b'\n').map(|i| start + i);
        let (end, eol) = nl.map_or((text.len(), text.len()), |nl| (nl, nl + 1));
        let end = if end > start && bytes[end - 1] == b'\r' { end - 1 } else { end };
        lines.push(Seg::new(start, end, eol));
        start = eol;
    }
    lines
}

/// Whether the line holds only spaces and tabs.
fn is_blank(text: &str, line: Seg) -> bool {
    text.as_bytes()[line.start..line.end].iter().all(|&b| b == b' ' || b == b'\t')
}

/// Whether a reserved-head form starts at byte `at`: `(`, a head from
/// [`crate::RESERVED_HEADS`], then whitespace, `(`, `)`, `'` or `"`.
fn is_form_start(text: &str, at: usize) -> bool {
    let bytes = text.as_bytes();
    if bytes.get(at) != Some(&b'(') {
        return false;
    }
    let after = &bytes[at + 1..];
    crate::RESERVED_HEADS.iter().any(|head| {
        after.starts_with(head.as_bytes())
            && matches!(
                after.get(head.len()),
                Some(b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')' | b'\'' | b'"')
            )
    })
}

/// Reads the code form at `at` from the lines `segs` (the first of which holds
/// `at`). If the form is never closed, it is re-read from only the lines before
/// the next blank line so that the damage stays inside one paragraph.
fn read_code(text: &str, file: FileId, segs: &[Seg], at: usize) -> (Read, Diags) {
    let mut diags = Diags::new();
    let read = reader::read_in(text, file, at, segs, &mut diags);
    if diags.iter().any(|d| d.code == DiagCode::UnclosedParen) {
        let cut = segs
            .iter()
            .skip(1)
            .position(|&s| is_blank(text, s))
            .map_or(segs.len(), |i| i + 1);
        if cut < segs.len() {
            let mut retry = Diags::new();
            let read = reader::read_in(text, file, at, &segs[..cut], &mut retry);
            return (read, retry);
        }
    }
    (read, diags)
}

#[cfg(test)]
mod tests;
