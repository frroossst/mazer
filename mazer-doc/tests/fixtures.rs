//! Compiles every `.zr` note in the fixtures directory.
//!
//! The directory is `$MAZER_FIXTURES`, or `fixtures/notes` at the workspace
//! root. The notes are private and not committed, so when the directory is
//! missing this test passes without doing anything.
//!
//! The notes have not been migrated to the new syntax and many keywords are
//! still being written, so diagnostics are expected; this test only checks
//! that compiling never panics and never leaks raw code into the page. It
//! prints a per-note summary of diagnostic codes (codes and counts only).
//! With `MAZER_DOC_DUMP=<dir>` it also writes each page there, for comparing
//! by eye with the old renderer's output.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mazer_doc::{Options, Page, compile};

fn fixture_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("MAZER_FIXTURES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/notes"),
        PathBuf::from,
    );
    dir.is_dir().then_some(dir)
}

/// `html` with everything inside `open ... close` removed.
fn strip_between(html: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        rest = rest[start..].find(close).map_or("", |end| &rest[start + end + close.len()..]);
    }
    out.push_str(rest);
    out
}

/// The page without code blocks, inline code and error boxes: what is left
/// must not contain raw `(show`.
fn outside_code(html: &str) -> String {
    let html = strip_between(html, "<pre", "</pre>");
    let html = strip_between(&html, "<code", "</code>");
    strip_between(&html, "<span class=\"mazer-error\">", "</span>")
}

#[test]
fn every_note_compiles() {
    let Some(dir) = fixture_dir() else {
        eprintln!("no fixtures directory; skipping");
        return;
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("readable fixtures directory")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "zr"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no .zr files in {}", dir.display());
    for path in paths {
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let text = std::fs::read_to_string(&path).expect("readable note");
        for page in [Page::Fragment, Page::Full { title: name.clone() }] {
            let full = matches!(page, Page::Full { .. });
            let mut opts = Options::new(name.clone());
            opts.page = page;
            let out = compile(&text, &mut opts);
            assert!(!out.html.is_empty(), "{name}: empty output");
            assert!(
                !outside_code(&out.html).contains("(show"),
                "{name}: raw `(show` outside code and error boxes"
            );
            if let Some(dump) = std::env::var_os("MAZER_DOC_DUMP") {
                let suffix = if full { "full.html" } else { "fragment.html" };
                let target = PathBuf::from(dump).join(format!("{name}.{suffix}"));
                std::fs::write(&target, &out.html).expect("writable dump directory");
            }
            if !full {
                let mut summary: BTreeMap<&str, usize> = BTreeMap::new();
                for diag in &out.diags {
                    *summary.entry(diag.code.name()).or_default() += 1;
                }
                let line: Vec<String> = summary.iter().map(|(code, n)| format!("{code}:{n}")).collect();
                println!("{name}: {} diagnostics [{}]", out.diags.len(), line.join(" "));
            }
        }
    }
}
