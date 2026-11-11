//! Byte positions and spans.

use std::ops::Range;

/// A byte offset into one source file. A newtype (not an alias) so it cannot be
/// mixed up with a line number or a UTF-16 column.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ByteOffset(pub u32);

impl ByteOffset {
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }

    /// Converts from a `usize` index, saturating at `u32::MAX`.
    #[must_use]
    pub fn from_usize(offset: usize) -> Self {
        Self(u32::try_from(offset).unwrap_or(u32::MAX))
    }

    #[must_use]
    pub const fn to_usize(self) -> usize {
        self.0 as usize
    }
}

/// Identifies one file in a [`crate::SourceMap`]. `0` is the note being
/// compiled; later ids are the embedded `stdlib.zr` and any other files.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct FileId(pub u16);

/// A half-open byte range `[start, end)` in one file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Span {
    pub file: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Span {
    /// A span for nodes that do not come from source text (tests, synthesized nodes).
    pub const DUMMY: Self = Self {
        file: FileId(u16::MAX),
        start: ByteOffset(0),
        end: ByteOffset(0),
    };

    #[must_use]
    pub const fn new(file: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        Self { file, start, end }
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Whether `at` lies inside this span. The end is inclusive here so that a
    /// cursor sitting right after the last character still counts as "in" it.
    #[must_use]
    pub fn contains(self, at: ByteOffset) -> bool {
        self.start <= at && at <= self.end
    }

    /// The smallest span covering both `self` and `other` (same file assumed).
    #[must_use]
    pub fn to(self, other: Self) -> Self {
        Self {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// The byte range, for slicing the file text.
    #[must_use]
    pub const fn range(self) -> Range<usize> {
        self.start.to_usize()..self.end.to_usize()
    }
}
