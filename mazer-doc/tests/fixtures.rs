//! Compiles the `.zr` notes in the fixtures directory and the tracked example
//! files, and requires every one to be clean.
//!
//! The notes directory is `$MAZER_FIXTURES`, or `fixtures/notes` at the
//! workspace root. The notes are private and not committed, so when the
//! directory is missing the first test passes without doing anything. The
//! second test reads `examples/*.md`, which are always present.
//!
//! A clean page has no diagnostics of any severity, one non-empty
//! `data-mazer-src` attribute per `<math` element, no element with class
//! `mazer-error`, and at least one `<math` element per `(show` form outside
//! code. Failure messages carry codes, counts and byte offsets only, never
//! the text of a note. With `MAZER_DOC_DUMP=<dir>` each page is also written
//! there, for comparing by eye with the old renderer's output.

use std::path::{Path, PathBuf};

use mazer_doc::{Options, Page, compile};

fn fixture_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("MAZER_FIXTURES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/notes"),
        PathBuf::from,
    );
    dir.is_dir().then_some(dir)
}

/// `text` with everything inside `open ... close` removed.
fn strip_between(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        let after = &rest[start + open.len()..];
        rest = after.find(close).map_or("", |end| &after[end + close.len()..]);
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

/// The source with fenced code blocks and inline code spans removed.
fn without_code(src: &str) -> String {
    let mut out = String::new();
    let mut fenced = false;
    for line in src.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if !fenced {
            out.push_str(line);
            out.push('\n');
        }
    }
    strip_between(&out, "`", "`")
}

/// Number of `(show` forms in prose: the head must be followed by whitespace
/// or a paren and must not be escaped with a backslash.
fn count_shows(src: &str) -> usize {
    let text = without_code(src);
    let bytes = text.as_bytes();
    text.match_indices("(show")
        .filter(|&(at, _)| {
            let escaped = at > 0 && bytes[at - 1] == b'\\';
            let after = bytes.get(at + 5).copied();
            !escaped && after.is_none_or(|b| b.is_ascii_whitespace() || b == b'(' || b == b')')
        })
        .count()
}

/// Compiles one page both ways and returns a list of problems (empty when clean).
fn problems(name: &str, text: &str, dump: Option<&Path>) -> Vec<String> {
    let mut found = Vec::new();
    for page in [Page::Fragment, Page::Full { title: name.to_owned() }] {
        let full = matches!(page, Page::Full { .. });
        let mut opts = Options::new(name.to_owned());
        opts.page = page;
        let out = compile(text, &mut opts);
        let kind = if full { "full" } else { "fragment" };
        if out.html.is_empty() {
            found.push(format!("{name} ({kind}): empty output"));
        }
        if let Some(dir) = dump {
            let target = dir.join(format!("{name}.{kind}.html"));
            std::fs::write(&target, &out.html).expect("writable dump directory");
        }
        if full {
            continue;
        }
        for diag in &out.diags {
            found.push(format!(
                "{name}: {:?} {} at bytes {}..{}",
                diag.severity,
                diag.code.name(),
                diag.span.start.0,
                diag.span.end.0
            ));
        }
        if out.html.contains("mazer-error") {
            found.push(format!("{name}: page contains an element with class mazer-error"));
        }
        let math = out.html.matches("<math").count();
        // Every rendered equation carries its source, and none is empty.
        let sources = out.html.matches("data-mazer-src=").count();
        if sources != math {
            found.push(format!("{name}: {sources} data-mazer-src attributes for {math} <math> elements"));
        }
        if out.html.contains("data-mazer-src=\"\"") {
            found.push(format!("{name}: empty data-mazer-src"));
        }
        let shows = count_shows(text);
        if math < shows {
            found.push(format!("{name}: {math} <math> elements for {shows} `(show` forms"));
        }
        // A backslash-escaped `\(show` is deliberate prose and stays in the page.
        let escaped = without_code(text).matches("\\(show").count();
        if outside_code(&out.html).matches("(show").count() > escaped {
            found.push(format!("{name}: raw `(show` outside code and error boxes"));
        }
    }
    found
}

fn dump_dir() -> Option<PathBuf> {
    std::env::var_os("MAZER_DOC_DUMP").map(PathBuf::from)
}

#[test]
fn every_note_is_clean() {
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
    let dump = dump_dir();
    let mut all = Vec::new();
    for path in paths {
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let text = std::fs::read_to_string(&path).expect("readable note");
        let found = problems(&name, &text, dump.as_deref());
        println!("{name}: {} problems", found.len());
        all.extend(found);
    }
    assert!(all.is_empty(), "problems:\n{}", all.join("\n"));
}

#[test]
fn examples_are_clean() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let dump = dump_dir();
    let mut all = Vec::new();
    for file in ["markdown.md", "docs.md"] {
        let text = std::fs::read_to_string(dir.join(file)).expect("readable example");
        all.extend(problems(file, &text, dump.as_deref()));
    }
    assert!(all.is_empty(), "problems:\n{}", all.join("\n"));
}
