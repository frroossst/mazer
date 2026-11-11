//! Helpers for the unit tests.

use lsp_types::Uri;
use mazer_doc::{Analysis, ByteOffset, analyze};

use crate::convert;

/// Strips the `|` marking the cursor from `marked` and analyzes the rest.
/// Returns the analysis and the cursor's byte offset.
pub fn marked(marked: &str) -> (Analysis, ByteOffset) {
    let offset = marked.find('|').expect("a | marks the cursor");
    let src = marked.replacen('|', "", 1);
    (analyze(&src, "test.zr"), ByteOffset::from_usize(offset))
}

/// The URI the tests use for their note.
pub fn uri() -> Uri {
    "file:///notes/test.zr".parse().expect("a valid URI")
}

/// The text a range of the note covers.
pub fn text_of(analysis: &Analysis, range: lsp_types::Range) -> String {
    let start = convert::offset_at(&analysis.sources, range.start).to_usize();
    let end = convert::offset_at(&analysis.sources, range.end).to_usize();
    convert::note_text(&analysis.sources)[start..end].to_owned()
}
