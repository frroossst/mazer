//! Documentation generator for Mazer.
//!
//! Extracts function metadata and outputs it as JSON for LSP autocomplete
//! and documentation tooling.

use mazer_types::implfuncs::{FuncInfo, ShowFunc};
use serde::Serialize;
use std::{env, fs, io};

/// Root structure for the documentation JSON output.
#[derive(Serialize)]
struct DocOutput {
    version: &'static str,
    functions: Vec<FuncInfo>,
}

fn main() -> io::Result<()> {
    let output_path = env::args().nth(1).unwrap_or_else(|| "mazer-doc.json".to_string());

    let doc = DocOutput {
        version: env!("CARGO_PKG_VERSION"),
        functions: ShowFunc::all_functions(),
    };

    let json = serde_json::to_string_pretty(&doc).expect("Failed to serialize documentation");

    fs::write(&output_path, &json)?;

    println!("Documentation written to: {output_path}");
    println!("Total functions: {}", doc.functions.len());

    Ok(())
}
