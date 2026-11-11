//! Browser front end: `.zr` text in, HTML out, diagnostics on the console.

use mazer_doc::{Diag, Options, Severity, SourceMap, compile};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn warn(line: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(line: &str);
}

/// One console line for a diagnostic:
/// `mazer: <name>:<line>:<col>: <severity>[<code>]: <message>`, followed by
/// `; help: <text>` when there is a suggestion. Line and column are 1-based
/// and the column counts characters, as the command-line tool does. (The
/// language server counts UTF-16 units, as its protocol requires, and spans
/// themselves are byte offsets.)
// Only the wasm build prints to the console; host builds just run the tests.
#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
fn format_diag(diag: &Diag, sources: &SourceMap) -> String {
    let (name, line, col) = sources.file(diag.span.file).map_or_else(
        || (String::from("?"), 1, 1),
        |file| {
            let (line, col) = sources.line_col(diag.span.file, diag.span.start);
            // The line's text before the offset; its length in characters is
            // the 0-based character column.
            let at = diag.span.start.to_usize().min(file.text.len());
            let from = at.saturating_sub(col.0 as usize);
            let before = file.text.get(from..at).unwrap_or("");
            (file.name.clone(), line.0 + 1, before.chars().count() + 1)
        },
    );
    let severity = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Hint => "hint",
    };
    let mut out = format!(
        "mazer: {name}:{line}:{col}: {severity}[{}]: {}",
        diag.code.name(),
        diag.message
    );
    if let Some(help) = &diag.help {
        out.push_str("; help: ");
        out.push_str(help);
    }
    out
}

#[cfg(target_arch = "wasm32")]
fn report(diag: &Diag, sources: &SourceMap) {
    let line = format_diag(diag, sources);
    if diag.severity == Severity::Error {
        error(&line);
    } else {
        warn(&line);
    }
}

#[cfg(not(target_arch = "wasm32"))]
const fn report(_diag: &Diag, _sources: &SourceMap) {}

/// Compiles a note to an HTML fragment. The caller sets the result as
/// `innerHTML` (or any other suitable property).
///
/// This function never fails and never panics: a broken form renders as an
/// inline error box and is also described on the browser console. That
/// matters in wasm, where a panic would poison the module for every later
/// call on the page.
#[wasm_bindgen]
#[must_use]
pub fn run_mazer(content: &str, window_name: &str) -> String {
    if content.trim().is_empty() {
        return String::new();
    }
    let mut opts = Options::new(window_name);
    let out = compile(content, &mut opts);
    for diag in &out.diags {
        report(diag, &out.sources);
    }
    out.html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_empty() {
        assert_eq!(run_mazer("  \n\t", "x.zr"), "");
    }

    #[test]
    fn renders_a_fragment() {
        let html = run_mazer("Hello (show (frac 1 2)).", "x.zr");
        assert!(html.contains("<mfrac>"), "{html}");
        assert!(!html.contains("<html"), "{html}");
    }

    #[test]
    fn broken_note_gives_error_box() {
        let html = run_mazer("(show (frac 1))", "x.zr");
        assert!(html.contains("merror") || html.contains("mazer-error"), "{html}");
    }

    #[test]
    fn diagnostic_line_format() {
        let mut opts = Options::new("note.zr");
        let out = compile("ok\n\n  (show (frac 1))\n", &mut opts);
        let diag = out.diags.first().expect("a diagnostic");
        let line = format_diag(diag, &out.sources);
        assert!(line.starts_with("mazer: note.zr:3:"), "{line}");
        assert!(line.contains(": error[arity]: "), "{line}");
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        // Greek letters (2 bytes each) and an emoji (4 bytes) before the error.
        let src = "ok\n\u{3b1}\u{3b2}\u{3b3} \u{1f600} (show (frac 1))\n";
        let mut opts = Options::new("g.zr");
        let out = compile(src, &mut opts);
        let diag = out.diags.first().expect("a diagnostic");
        let line = format_diag(diag, &out.sources);
        // "αβγ 😀 (show " is 12 characters, so the `(frac` form starts at column 13;
        // counting bytes it would be column 19.
        assert!(line.starts_with("mazer: g.zr:2:13: "), "{line}");
        assert_eq!(out.sources.line_col(diag.span.file, diag.span.start).1.0, 18);
    }

    #[test]
    fn help_is_appended() {
        let mut opts = Options::new("n.zr");
        let out = compile("(show (fracc 1 2))", &mut opts);
        let lines: Vec<String> = out
            .diags
            .iter()
            .map(|d| format_diag(d, &out.sources))
            .collect();
        assert!(lines.iter().any(|l| l.contains("; help: ")), "{lines:?}");
    }
}
