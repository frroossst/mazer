//! Renders diagnostics rustc-style with `annotate-snippets`.

use std::ops::Range;

use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};
use mazer_doc::{Diag, FileId, Severity, SourceMap, Span};

/// The byte range of `span` inside its file, clamped to the text and to
/// character boundaries. An empty span is widened to one character so it is
/// visible.
fn range_of(sources: &SourceMap, span: Span) -> Option<Range<usize>> {
    let text = &sources.file(span.file)?.text;
    let mut start = span.start.to_usize().min(text.len());
    let mut end = span.end.to_usize().clamp(start, text.len());
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    while !text.is_char_boundary(end) {
        end += 1;
    }
    if start == end {
        if let Some(c) = text[end..].chars().next() {
            end += c.len_utf8();
        } else if let Some(c) = text[..start].chars().next_back() {
            start -= c.len_utf8();
        }
    }
    Some(start..end)
}

const fn level(severity: Severity) -> Level<'static> {
    match severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Hint => Level::NOTE,
    }
}

/// One diagnostic as text, without a trailing newline. `color` selects ANSI
/// styling.
#[must_use]
pub fn render_diag(diag: &Diag, sources: &SourceMap, color: bool) -> String {
    let level = match diag.severity {
        Severity::Hint => level(diag.severity).with_name("hint"),
        s => level(s),
    };
    let title = level.primary_title(diag.message.as_str()).id(diag.code.name());

    // One snippet per file the diagnostic touches, the primary file first.
    let mut files: Vec<FileId> = vec![diag.span.file];
    for (span, _) in &diag.labels {
        if !files.contains(&span.file) {
            files.push(span.file);
        }
    }
    let mut group: Group<'_> = Group::with_title(title);
    for file in files {
        let Some(src) = sources.file(file) else {
            continue;
        };
        let mut snippet = Snippet::source(src.text.as_str()).path(src.name.as_str());
        let primary = if diag.span.file == file {
            range_of(sources, diag.span)
        } else {
            None
        };
        let mut any = primary.is_some();
        if let Some(r) = primary {
            snippet = snippet.annotation(AnnotationKind::Primary.span(r));
        }
        for (span, text) in &diag.labels {
            if span.file != file {
                continue;
            }
            if let Some(r) = range_of(sources, *span) {
                snippet = snippet
                    .annotation(AnnotationKind::Context.span(r).label(text.as_str()));
                any = true;
            }
        }
        if any {
            group = group.element(snippet);
        }
    }
    let mut report = vec![group];
    if let Some(help) = &diag.help {
        report.push(Group::with_title(Level::HELP.secondary_title(help.as_str())));
    }

    let renderer = if color { Renderer::styled() } else { Renderer::plain() };
    renderer.render(&report)
}

/// `N errors, M warnings`, or `None` when there are neither.
#[must_use]
pub fn summary(diags: &[Diag]) -> Option<String> {
    let count = |s: Severity| diags.iter().filter(|d| d.severity == s).count();
    let (e, w) = (count(Severity::Error), count(Severity::Warning));
    if e + w == 0 {
        return None;
    }
    let plural = |n: usize, word: &str| {
        if n == 1 {
            format!("{n} {word}")
        } else {
            format!("{n} {word}s")
        }
    };
    Some(format!("{}, {}", plural(e, "error"), plural(w, "warning")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mazer_doc::{Options, compile};

    #[test]
    fn renders_location_and_code() {
        let mut opts = Options::new("t.zr");
        let out = compile("hello\n\n(show (frac 1))\n", &mut opts);
        let diag = out.diags.first().expect("diag");
        let text = render_diag(diag, &out.sources, false);
        assert!(text.contains("error[arity]"), "{text}");
        assert!(text.contains("--> t.zr:3:"), "{text}");
        assert!(text.contains("(show (frac 1))"), "{text}");
        assert!(!text.contains('\u{1b}'), "plain output has no escapes");
    }

    #[test]
    fn colour_adds_escapes() {
        let mut opts = Options::new("t.zr");
        let out = compile("(show (frac 1))\n", &mut opts);
        let diag = out.diags.first().expect("diag");
        assert!(render_diag(diag, &out.sources, true).contains('\u{1b}'));
    }

    #[test]
    fn dummy_span_does_not_panic() {
        let sources = SourceMap::new();
        let d = Diag::error(mazer_doc::DiagCode::Other, Span::DUMMY, "odd");
        assert!(render_diag(&d, &sources, false).contains("odd"));
    }

    #[test]
    fn summaries() {
        let e = Diag::error(mazer_doc::DiagCode::Other, Span::DUMMY, "e");
        let w = Diag::warning(mazer_doc::DiagCode::Other, Span::DUMMY, "w");
        let h = Diag::hint(mazer_doc::DiagCode::Other, Span::DUMMY, "h");
        assert_eq!(summary(&[]), None);
        assert_eq!(summary(std::slice::from_ref(&h)), None);
        assert_eq!(summary(&[e.clone(), w.clone(), w]), Some("1 error, 2 warnings".into()));
        assert_eq!(summary(&[e.clone(), e, h]), Some("2 errors, 0 warnings".into()));
    }
}
