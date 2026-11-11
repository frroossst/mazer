//! Owns source text and converts byte offsets to positions people and editors
//! understand. Trees never borrow from the source; they hold [`Span`]s.

use crate::span::{ByteOffset, FileId, Span};

/// A 0-based line number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Line(pub u32);

/// A 0-based column counted in bytes. For terminal output.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ByteCol(pub u32);

/// A 0-based column counted in UTF-16 code units. For the LSP.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Utf16Col(pub u32);

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    line_starts: Vec<ByteOffset>,
}

impl SourceFile {
    fn new(name: String, text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Self {
            name,
            text,
            line_starts,
        }
    }

    /// Byte offset of the first byte of each line.
    #[must_use]
    pub fn line_starts(&self) -> &[ByteOffset] {
        &self.line_starts
    }
}

fn compute_line_starts(text: &str) -> Vec<ByteOffset> {
    let mut starts = vec![ByteOffset(0)];
    starts.extend(
        text.bytes()
            .enumerate()
            .filter(|&(_, b)| b == b'\n')
            .map(|(i, _)| ByteOffset::from_usize(i + 1)),
    );
    starts
}

/// All files taking part in one compile, indexed by [`FileId`].
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file and returns its id. Ids are handed out in order from 0.
    pub fn add(&mut self, name: String, text: String) -> FileId {
        let id = FileId(u16::try_from(self.files.len()).unwrap_or(u16::MAX));
        self.files.push(SourceFile::new(name, text));
        id
    }

    /// Replaces the whole text of a file (the LSP does this on every edit).
    ///
    /// # Panics
    /// Panics if `file` was not returned by [`SourceMap::add`] on this map.
    pub fn replace(&mut self, file: FileId, text: String) {
        let slot = &mut self.files[file.0 as usize];
        *slot = SourceFile::new(std::mem::take(&mut slot.name), text);
    }

    #[must_use]
    pub fn file(&self, file: FileId) -> Option<&SourceFile> {
        self.files.get(file.0 as usize)
    }

    /// The text a span covers, or `""` if the span does not fit its file
    /// (for example [`Span::DUMMY`]).
    #[must_use]
    pub fn text(&self, span: Span) -> &str {
        self.file(span.file)
            .and_then(|f| f.text.get(span.range()))
            .unwrap_or("")
    }

    /// Line and byte column of an offset.
    #[must_use]
    pub fn line_col(&self, file: FileId, at: ByteOffset) -> (Line, ByteCol) {
        let Some(f) = self.file(file) else {
            return (Line(0), ByteCol(0));
        };
        let line = f.line_starts.partition_point(|&s| s <= at).saturating_sub(1);
        let start = f.line_starts[line];
        (
            Line(u32::try_from(line).unwrap_or(u32::MAX)),
            ByteCol(at.0.saturating_sub(start.0)),
        )
    }

    /// Line and UTF-16 column of an offset (what the LSP protocol speaks).
    #[must_use]
    pub fn line_col_utf16(&self, file: FileId, at: ByteOffset) -> (Line, Utf16Col) {
        let (line, col) = self.line_col(file, at);
        let Some(f) = self.file(file) else {
            return (line, Utf16Col(0));
        };
        let start = f.line_starts[line.0 as usize].to_usize();
        let end = (start + col.0 as usize).min(f.text.len());
        let units = f
            .text
            .get(start..end)
            .map_or(0, |s| s.encode_utf16().count());
        (line, Utf16Col(u32::try_from(units).unwrap_or(u32::MAX)))
    }

    /// The byte offset of an LSP position. Positions past the end of a line or
    /// file clamp to the end of that line or file.
    #[must_use]
    pub fn offset_at(&self, file: FileId, line: Line, col: Utf16Col) -> ByteOffset {
        let Some(f) = self.file(file) else {
            return ByteOffset(0);
        };
        let Some(&start) = f.line_starts.get(line.0 as usize) else {
            return ByteOffset::from_usize(f.text.len());
        };
        let line_end = f
            .line_starts
            .get(line.0 as usize + 1)
            .map_or(f.text.len(), |next| next.to_usize());
        let line_text = &f.text[start.to_usize()..line_end];
        let mut units = 0u32;
        for (i, ch) in line_text.char_indices() {
            if units >= col.0 || ch == '\n' {
                return ByteOffset::from_usize(start.to_usize() + i);
            }
            units += u32::try_from(ch.len_utf16()).unwrap_or(2);
        }
        ByteOffset::from_usize(line_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_and_back() {
        let mut map = SourceMap::new();
        let f = map.add("t".into(), "ab\nτx\n".into());
        assert_eq!(map.line_col(f, ByteOffset(4)), (Line(1), ByteCol(1)));
        // τ is two bytes but one UTF-16 unit: byte offset 5 is column 1.
        assert_eq!(map.line_col_utf16(f, ByteOffset(5)), (Line(1), Utf16Col(1)));
        assert_eq!(map.offset_at(f, Line(1), Utf16Col(1)), ByteOffset(5));
        assert_eq!(map.offset_at(f, Line(0), Utf16Col(99)), ByteOffset(2));
        assert_eq!(map.offset_at(f, Line(9), Utf16Col(0)), ByteOffset(7));
    }

    #[test]
    fn text_of_span() {
        let mut map = SourceMap::new();
        let f = map.add("t".into(), "hello world".into());
        assert_eq!(map.text(Span::new(f, ByteOffset(6), ByteOffset(11))), "world");
        assert_eq!(map.text(Span::DUMMY), "");
    }
}
