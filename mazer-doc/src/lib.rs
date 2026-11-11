//! The document stage: everything between "a tree of blocks" and "HTML".
//!
//! This is the only crate the front ends (CLI, wasm, LSP) depend on. It owns
//! notation macros, variable bindings, equation labels, the pipeline entry
//! points and HTML output.
//!
//! See `docs/ARCHITECTURE.md`.

pub mod bindings;
pub mod eval;
pub mod labels;
pub mod locate;
pub mod macros;
pub mod pipeline;

mod html;
mod walk;

#[cfg(test)]
mod testing;

pub use bindings::{Binding, Bindings};
pub use eval::Evaluator;
pub use labels::{Label, Labels, collect_labels};
pub use locate::{Cursor, locate};
pub use macros::{Macro, Macros, expand};
pub use pipeline::{Analysis, Options, Output, Page, Target, analyze, compile, render};

// Front ends depend only on this crate, so re-export what they need from below.
pub use mazer_math as math;
pub use mazer_syntax as syntax;
pub use mazer_syntax::{ByteOffset, Diag, DiagCode, Diags, FileId, Severity, SourceMap, Span, Sym};

/// The embedded prelude: `alias` and `macro` definitions available in every note.
///
/// It is read once per process as file 1 of every [`Analysis`]. Only its
/// definitions are used; it is never rendered. It must parse with no
/// diagnostics, and since `;` comments exist only inside code forms, it holds
/// no top-level comment lines. It is empty for now: the built-in keywords
/// and display names cover everything the notes need, and definitions are
/// added here only when one is clearly useful to every note.
pub const STDLIB: &str = include_str!("stdlib.zr");
