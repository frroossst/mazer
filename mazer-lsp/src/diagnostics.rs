//! `Diag` to LSP diagnostics.
//!
//! - Severity maps one to one (`Error`, `Warning`, `Hint`); `code` is
//!   `DiagCode::name()`, `source` is `mazer`, the message ends with the help
//!   text on a line of its own.
//! - Secondary labels in the note become `relatedInformation`. A label in
//!   another file (the embedded `stdlib.zr`, which the editor cannot open) is
//!   folded into the message as `stdlib.zr:line:col: text`.
//! - A diagnostic whose primary span is not in the note (only possible for a
//!   broken embedded prelude) is shown at the start of the note, with its
//!   place in the message.
//! - Problems only `render` finds (dangerous link URLs, `NoEvaluator`) are
//!   added by [`render_problems`]. `NoEvaluator` is downgraded to a hint:
//!   there is never an evaluator in the editor, and that is not the note's
//!   fault.

use std::fmt::Write as _;

use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Position, Range, Uri,
};
use mazer_doc::{Analysis, Diag, DiagCode, Diags, Options, Page, Severity, Span, render};

use crate::convert::{self, NOTE};

/// Renders the note with no evaluator and returns the HTML and the problems
/// only rendering finds.
#[must_use]
pub fn render_problems(analysis: &Analysis, name: &str, page: Page) -> (String, Vec<Diag>) {
    let mut opts = Options::new(name);
    opts.page = page;
    let mut found = Diags::new();
    let html = render(analysis, &mut opts, &mut found);
    (html, found.into_vec())
}

/// Every diagnostic of the note: those `analyze` found, then `extra` (from
/// [`render_problems`]).
#[must_use]
pub fn collect(analysis: &Analysis, uri: &Uri, extra: &[Diag]) -> Vec<Diagnostic> {
    analysis
        .diags
        .iter()
        .chain(extra)
        .map(|diag| to_lsp(analysis, uri, diag))
        .collect()
}

/// One diagnostic in LSP form.
#[must_use]
pub fn to_lsp(analysis: &Analysis, uri: &Uri, diag: &Diag) -> Diagnostic {
    let sources = &analysis.sources;
    let no_evaluator = diag.code == DiagCode::NoEvaluator;
    let severity = match diag.severity {
        _ if no_evaluator => DiagnosticSeverity::HINT,
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Hint => DiagnosticSeverity::HINT,
    };
    let mut message = if no_evaluator {
        "evaluation is not available in the editor: this `eval` is shown as an error box until the note is compiled with an evaluator".to_owned()
    } else {
        diag.message.clone()
    };
    let range = if diag.span.file == NOTE {
        convert::range(sources, diag.span)
    } else {
        let _ = write!(message, "\n(in {})", place(analysis, diag.span));
        Range::new(Position::new(0, 0), Position::new(0, 0))
    };
    if let Some(help) = &diag.help
        && !no_evaluator
    {
        let _ = write!(message, "\nhelp: {help}");
    }
    let mut related = Vec::new();
    for (span, text) in &diag.labels {
        if span.file == NOTE {
            related.push(DiagnosticRelatedInformation {
                location: Location::new(uri.clone(), convert::range(sources, *span)),
                message: text.clone(),
            });
        } else {
            let _ = write!(message, "\nnote: {}: {text}", place(analysis, *span));
        }
    }
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(diag.code.name().to_owned())),
        code_description: None,
        source: Some("mazer".to_owned()),
        message,
        related_information: (!related.is_empty()).then_some(related),
        tags: None,
        data: None,
    }
}

/// `file:line:col` (1-based) of a span outside the note.
fn place(analysis: &Analysis, span: Span) -> String {
    let Some(file) = analysis.sources.file(span.file) else {
        return "built-in definitions".to_owned();
    };
    let (line, col) = analysis.sources.line_col(span.file, span.start);
    format!("{}:{}:{}", file.name, line.0 + 1, col.0 + 1)
}

#[cfg(test)]
mod tests {
    use mazer_doc::{ByteOffset, FileId, analyze};

    use super::*;
    use crate::testing::{text_of, uri};

    #[test]
    fn severity_code_help_and_related() {
        let src = "τ (alias A x)\n\n(alias A y)\n\n(show (frac 1))";
        let analysis = analyze(src, "t.zr");
        let out = collect(&analysis, &uri(), &[]);
        let dup = out
            .iter()
            .find(|d| d.code == Some(NumberOrString::String("duplicate-definition".into())))
            .expect("a duplicate definition");
        assert_eq!(dup.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(dup.source.as_deref(), Some("mazer"));
        assert_eq!(text_of(&analysis, dup.range), "A");
        assert_eq!(dup.range.start, Position::new(2, 7));
        let related = dup.related_information.as_ref().expect("related information");
        assert_eq!(related[0].message, "first defined here");
        // The first definition, after a two-byte τ: UTF-16 column 2.
        assert_eq!(related[0].location.range.start, Position::new(0, 2));
        let arity = out
            .iter()
            .find(|d| d.code == Some(NumberOrString::String("arity".into())))
            .expect("an arity error");
        assert!(arity.message.contains("\nhelp: (frac num den)"), "{}", arity.message);
    }

    #[test]
    fn hints_and_warnings() {
        let analysis = analyze("(show (fracc a b))", "t.zr");
        let out = collect(&analysis, &uri(), &[]);
        assert_eq!(out[0].severity, Some(DiagnosticSeverity::HINT));
        assert!(out[0].message.contains("frac"));
    }

    #[test]
    fn render_only_problems() {
        let src = "(eval (+ 1 2))\n\n[x](javascript:alert(1))";
        let analysis = analyze(src, "t.zr");
        let (html, extra) = render_problems(&analysis, "t.zr", Page::Fragment);
        assert!(html.contains("mazer-error"));
        let out = collect(&analysis, &uri(), &extra);
        let eval = out
            .iter()
            .find(|d| d.code == Some(NumberOrString::String("no-evaluator".into())))
            .expect("no evaluator");
        assert_eq!(eval.severity, Some(DiagnosticSeverity::HINT));
        assert!(eval.message.contains("not available in the editor"));
        assert!(out.iter().any(|d| d.severity == Some(DiagnosticSeverity::WARNING)), "{out:?}");
    }

    #[test]
    fn labels_outside_the_note_are_folded_in() {
        let analysis = analyze("(show x)", "t.zr");
        let stdlib = Span::new(FileId(1), ByteOffset(0), ByteOffset(0));
        let diag = Diag::error(DiagCode::Other, stdlib, "broken prelude").with_label(stdlib, "here");
        let out = to_lsp(&analysis, &uri(), &diag);
        assert_eq!(out.range, Range::default());
        assert!(out.message.contains("(in stdlib.zr:1:1)"), "{}", out.message);
        assert!(out.message.contains("note: stdlib.zr:1:1: here"), "{}", out.message);
        assert!(out.related_information.is_none());
    }
}
