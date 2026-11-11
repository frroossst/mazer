//! Converting between mazer's byte offsets and spans and the LSP's UTF-16
//! positions and ranges, and between document URIs and note names.
//!
//! Every conversion clamps instead of failing: a position past the end of a
//! line is the end of that line, a line past the end of the note is the end
//! of the note.

use lsp_types::{Position, Range, Uri};
use mazer_doc::syntax::{Line, Utf16Col};
use mazer_doc::{ByteOffset, FileId, SourceMap, Span};

/// The note is always file 0 of its analysis.
pub const NOTE: FileId = FileId(0);

/// The byte offset of an LSP position in the note, clamped to the text.
///
/// A position past the end of a line that ends in `\r\n` lands before the
/// `\r`, not between the `\r` and the `\n`.
#[must_use]
pub fn offset_at(sources: &SourceMap, pos: Position) -> ByteOffset {
    let offset = sources.offset_at(NOTE, Line(pos.line), Utf16Col(pos.character));
    let text = sources.file(NOTE).map_or("", |f| f.text.as_str());
    let at = offset.to_usize().min(text.len());
    let bytes = text.as_bytes();
    if at > 0 && bytes.get(at) == Some(&b'\n') && bytes[at - 1] == b'\r' {
        return ByteOffset::from_usize(at - 1);
    }
    ByteOffset::from_usize(at)
}

/// The LSP position of a byte offset in `file`.
#[must_use]
pub fn position(sources: &SourceMap, file: FileId, at: ByteOffset) -> Position {
    let (line, col) = sources.line_col_utf16(file, at);
    Position::new(line.0, col.0)
}

/// The LSP range of a span (in the span's own file).
#[must_use]
pub fn range(sources: &SourceMap, span: Span) -> Range {
    Range::new(position(sources, span.file, span.start), position(sources, span.file, span.end))
}

/// The range from `start` to `end` in the note.
#[must_use]
pub fn note_range(sources: &SourceMap, start: ByteOffset, end: ByteOffset) -> Range {
    range(sources, Span::new(NOTE, start, end))
}

/// The text of the note.
#[must_use]
pub fn note_text(sources: &SourceMap) -> &str {
    sources.file(NOTE).map_or("", |f| f.text.as_str())
}

/// Length of `text` in UTF-16 code units.
#[must_use]
pub fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

/// The note's file name, taken from the last path segment of its URI
/// (percent-decoded), e.g. `dirac-delta.zr`. Falls back to `note.zr`.
#[must_use]
pub fn note_name(uri: &Uri) -> String {
    let text = uri.as_str();
    let text = text.split(['?', '#']).next().unwrap_or(text);
    let last = text.rsplit('/').next().unwrap_or(text);
    let name = percent_decode(last);
    if name.is_empty() { "note.zr".to_owned() } else { name }
}

/// The note's name without its extension, for a page title.
#[must_use]
pub fn note_title(uri: &Uri) -> String {
    let name = note_name(uri);
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        _ => name,
    }
}

/// Decodes `%XX` escapes; anything malformed is kept as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = bytes.get(i + 1..i + 3)
            && let Ok(hex) = std::str::from_utf8(hex)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn map(text: &str) -> SourceMap {
        let mut sources = SourceMap::new();
        sources.add("t.zr".into(), text.into());
        sources
    }

    #[test]
    fn multibyte_round_trip() {
        // τ: 2 bytes, 1 UTF-16 unit. 😭: 4 bytes, 2 UTF-16 units.
        let text = "τ😭x\nαβ (show y)";
        let sources = map(text);
        let x = text.find('x').unwrap();
        let pos = position(&sources, NOTE, ByteOffset::from_usize(x));
        assert_eq!(pos, Position::new(0, 3));
        assert_eq!(offset_at(&sources, pos).to_usize(), x);
        let y = text.find('y').unwrap();
        let pos = position(&sources, NOTE, ByteOffset::from_usize(y));
        assert_eq!(pos, Position::new(1, 9));
        assert_eq!(offset_at(&sources, pos).to_usize(), y);
        assert_eq!(utf16_len("τ😭x"), 4);
    }

    #[test]
    fn positions_clamp() {
        let text = "ab\r\ncd";
        let sources = map(text);
        // Past the end of a line: the end of its content, before the `\r`.
        assert_eq!(offset_at(&sources, Position::new(0, 99)).to_usize(), 2);
        // Past the last line: the end of the note.
        assert_eq!(offset_at(&sources, Position::new(99, 0)).to_usize(), text.len());
        assert_eq!(offset_at(&sources, Position::new(u32::MAX, u32::MAX)).to_usize(), text.len());
        // In the middle of a surrogate pair: the character's start or end, never inside it.
        let sources = map("😭x");
        let at = offset_at(&sources, Position::new(0, 1)).to_usize();
        assert!(at == 0 || at == 4, "{at}");
        // An empty note.
        let sources = map("");
        assert_eq!(offset_at(&sources, Position::new(3, 3)).to_usize(), 0);
    }

    #[test]
    fn names_from_uris() {
        let uri = Uri::from_str("file:///home/a/notes/dirac%20delta.zr").unwrap();
        assert_eq!(note_name(&uri), "dirac delta.zr");
        assert_eq!(note_title(&uri), "dirac delta");
        let uri = Uri::from_str("untitled:Untitled-1").unwrap();
        assert_eq!(note_name(&uri), "untitled:Untitled-1");
        let uri = Uri::from_str("file:///x/").unwrap();
        assert_eq!(note_name(&uri), "note.zr");
    }
}
