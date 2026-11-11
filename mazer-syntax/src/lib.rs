//! Text to trees: spans, diagnostics, the S-expression reader and the markdown
//! parser. This crate knows nothing about math or about documents as output.
//!
//! See `docs/ARCHITECTURE.md` for the agreed design. Public signatures in this
//! crate are a contract shared with `mazer-math`, `mazer-doc` and the front ends.

pub mod diag;
pub mod doc;
pub mod emoji;
pub mod markdown;
pub mod node;
pub mod reader;
pub mod source;
pub mod span;
pub mod sym;

pub use diag::{Diag, DiagCode, Diags, Severity};
pub use doc::{Block, BlockKind, Document, Inline, InlineKind, ListItem};
pub use node::{Node, NodeKind};
pub use source::{ByteCol, Line, SourceFile, SourceMap, Utf16Col};
pub use span::{ByteOffset, FileId, Span};
pub use sym::Sym;

/// The only list heads that make a parenthesis in a note "code". Every other
/// parenthesis in prose is prose. `ref` is inline only.
pub const RESERVED_HEADS: &[&str] = &["show", "eval", "let", "alias", "macro", "ref"];
