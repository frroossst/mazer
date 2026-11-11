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
pub const STDLIB: &str = include_str!("stdlib.zr");
