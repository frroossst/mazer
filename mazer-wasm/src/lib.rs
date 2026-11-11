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
/// (the column counts bytes, as in the terminal).
// Only the wasm build prints to the console; host builds just run the tests.
#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
fn format_diag(diag: &Diag, sources: &SourceMap) -> String {
    let (name, line, col) = sources.file(diag.span.file).map_or_else(
        || (String::from("?"), 1, 1),
        |file| {
            let (line, col) = sources.line_col(diag.span.file, diag.span.start);
            (file.name.clone(), line.0 + 1, col.0 + 1)
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
