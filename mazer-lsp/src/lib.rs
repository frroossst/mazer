//! The mazer language server. Synchronous, over stdio, built on `lsp-server`.
//!
//! State is one [`mazer_doc::Analysis`] per open note, re-made by
//! [`mazer_doc::analyze`] on every change (full-text sync). Every request is
//! answered from the stored analysis plus [`mazer_doc::locate`] and a walk of
//! the tree as written.
//!
//! Each feature is a module of pure functions from `(&Analysis, position)` to
//! LSP types, so it is testable without a client:
//!
//! - [`diagnostics`]: `Diag` to LSP diagnostics, plus what only `render` finds;
//! - [`completion`]: keywords, display names, macros, aliases, `let`
//!   variables, labels inside `(ref ...)`, emoji shortcodes, reserved heads;
//! - [`hover`]: docs for whatever name is under the cursor;
//! - [`signature`]: the signature of the innermost keyword or macro call;
//! - [`navigation`]: go to definition and find references;
//! - [`symbols`]: the outline (headings with definitions and tagged equations);
//! - [`semantic`]: semantic-token colouring of code;
//! - [`preview`]: a live HTML preview served on a loopback port.
//!
//! The protocol loop is in [`server`]; position conversion in [`convert`];
//! walking and classifying the written tree in [`tree`]; the hand-written docs
//! shared by several features in [`docs`].
//!
//! See `docs/ARCHITECTURE.md`, "Front ends".

pub mod completion;
pub mod convert;
pub mod diagnostics;
pub mod docs;
pub mod hover;
pub mod navigation;
pub mod preview;
pub mod semantic;
pub mod server;
pub mod signature;
pub mod symbols;
pub mod tree;

#[cfg(test)]
mod testing;
