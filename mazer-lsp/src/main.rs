//! The `mazer-lsp` binary: the mazer language server over stdio.
//!
//! Everything lives in the library (`src/lib.rs`), so the feature logic can
//! be unit-tested and the integration tests can use it directly.

use std::process::ExitCode;

fn main() -> ExitCode {
    match mazer_lsp::server::run_stdio() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(err) => {
            eprintln!("mazer-lsp: {err}");
            ExitCode::from(1)
        }
    }
}
