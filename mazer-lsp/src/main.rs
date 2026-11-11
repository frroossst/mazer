//! The mazer language server. Synchronous, over stdio, built on `lsp-server`.
//!
//! State is one [`mazer_doc::Analysis`] per open note, re-made by
//! `mazer_doc::analyze` on every change (full-text sync). Every request is
//! answered from the stored analysis plus `mazer_doc::locate`.
//!
//! Features, in the agreed order: diagnostics; completion (keywords, display
//! names with their character, macros / aliases / `let` variables, labels
//! inside `(ref ...)`, emoji shortcodes); hover; signature help. Then
//! go-to-definition, semantic-token colouring, live preview on a local port.
//!
//! See `docs/ARCHITECTURE.md`, "Front ends".

fn main() {
    todo!("step 3, LSP agent")
}
