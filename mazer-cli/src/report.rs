//! Renders diagnostics rustc-style with `annotate-snippets`.

use std::ops::Range;

use annotate_snippets::{Annotation, AnnotationKind, Group, Level, Renderer, Snippet};
use mazer_doc::{ByteOffset, Diag, FileId, Severity, SourceMap, Span};

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

/// The byte range of the lines that `r` touches, plus one line of context on
/// each side. Costs the length of those lines, not of the file.
fn window_of(text: &str, r: &Range<usize>) -> Range<usize> {
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |i| i + 1);
    let mut start = line_start(r.start);
    if start > 0 {
        start = line_start(start - 1);
    }
    // A range that ends just after a newline belongs to the line before it.
    let probe = r.end.saturating_sub(1).max(r.start);
    let line_end = |from: usize| text[from..].find('\n').map_or(text.len(), |i| from + i);
    let mut end = line_end(probe);
    if end < text.len() {
        end = line_end(end + 1);
    }
    start..end
}

/// One annotation: where, what kind, and the label text.
type Mark<'a> = (Range<usize>, AnnotationKind, Option<&'a str>);

/// Groups a file's annotations into snippets of nearby lines. Each snippet
/// holds only its window of the source and starts at its real line number.
fn snippets<'a>(sources: &'a SourceMap, file: FileId, marks: Vec<Mark<'a>>) -> Vec<Snippet<'a, Annotation<'a>>> {
    let Some(src) = sources.file(file) else {
        return Vec::new();
    };
    let text = src.text.as_str();
    let mut marks: Vec<(Range<usize>, Mark<'a>)> = marks
        .into_iter()
        .map(|m| (window_of(text, &m.0), m))
        .collect();
    marks.sort_by_key(|(w, _)| w.start);
    let mut windows: Vec<(Range<usize>, Vec<Mark<'a>>)> = Vec::new();
    for (w, m) in marks {
        match windows.last_mut() {
            Some((last, group)) if w.start <= last.end + 1 => {
                last.end = last.end.max(w.end);
                group.push(m);
            }
            _ => windows.push((w, vec![m])),
        }
    }
    windows
        .into_iter()
        .map(|(w, group)| {
            let first_line = sources.line_col(file, ByteOffset::from_usize(w.start)).0.0 as usize + 1;
            let mut snippet = Snippet::source(&text[w.clone()])
                .path(src.name.as_str())
                .line_start(first_line);
            for (r, kind, label) in group {
                let r = (r.start - w.start)..(r.end.min(w.end) - w.start);
                let mut ann = kind.span(r);
                if let Some(l) = label {
                    ann = ann.label(l);
                }
                snippet = snippet.annotation(ann);
            }
            snippet
        })
        .collect()
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

    // Snippets per file the diagnostic touches, the primary file first.
    let mut files: Vec<FileId> = vec![diag.span.file];
    for (span, _) in &diag.labels {
        if !files.contains(&span.file) {
            files.push(span.file);
        }
    }
    let mut group: Group<'_> = Group::with_title(title);
    for file in files {
        let mut marks: Vec<Mark<'_>> = Vec::new();
        if diag.span.file == file
            && let Some(r) = range_of(sources, diag.span)
        {
            marks.push((r, AnnotationKind::Primary, None));
        }
        for (span, text) in &diag.labels {
            if span.file == file
                && let Some(r) = range_of(sources, *span)
            {
                marks.push((r, AnnotationKind::Context, Some(text.as_str())));
            }
        }
        for snippet in snippets(sources, file, marks) {
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

/// `1 error, 2 warnings, 1 hint`: every severity that was reported, or `None`
/// when nothing was. Under `strict`, warnings are said to fail the run.
#[must_use]
pub fn summary(diags: &[Diag], strict: bool) -> Option<String> {
    let count = |s: Severity| diags.iter().filter(|d| d.severity == s).count();
    let plural = |n: usize, word: &str| {
        if n == 1 {
            format!("{n} {word}")
        } else {
            format!("{n} {word}s")
        }
    };
    let parts: Vec<String> = [
        (Severity::Error, "error"),
        (Severity::Warning, "warning"),
        (Severity::Hint, "hint"),
    ]
    .into_iter()
    .filter_map(|(s, word)| {
        let n = count(s);
        (n > 0).then(|| plural(n, word))
    })
    .collect();
    if parts.is_empty() {
        return None;
    }
    let mut line = parts.join(", ");
    if strict && count(Severity::Warning) > 0 {
        line.push_str(" (--strict: warnings fail the run)");
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mazer_doc::{Options, compile};
    use std::fmt::Write as _;

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
        assert_eq!(summary(&[], false), None);
        assert_eq!(summary(std::slice::from_ref(&h), false), Some("1 hint".into()));
        assert_eq!(
            summary(&[e.clone(), w.clone(), w.clone()], false),
            Some("1 error, 2 warnings".into())
        );
        assert_eq!(
            summary(&[e.clone(), e.clone(), h.clone(), w.clone(), h.clone()], false),
            Some("2 errors, 1 warning, 2 hints".into())
        );
        assert_eq!(
            summary(&[w, h], true),
            Some("1 warning, 1 hint (--strict: warnings fail the run)".into())
        );
        assert_eq!(summary(&[e], true), Some("1 error".into()));
    }

    /// A note with `n` filler lines, then a broken form on line `n + 1`.
    fn big_note(n: usize) -> String {
        let mut text = String::new();
        for i in 0..n {
            let _ = writeln!(text, "filler line {i}");
        }
        text.push_str("(show (frac 1))\n");
        text
    }

    #[test]
    fn large_file_shows_the_right_line_number_and_underline() {
        let text = big_note(4999);
        let mut opts = Options::new("big.zr");
        let out = compile(&text, &mut opts);
        let diag = out.diags.first().expect("diag");
        let shown = render_diag(diag, &out.sources, false);
        assert!(shown.contains("--> big.zr:5000:"), "{shown}");
        assert!(shown.contains("5000 | (show (frac 1))"), "{shown}");
        assert!(!shown.contains("filler line 100"), "{shown}");
        // Only the lines around it are in the output.
        assert!(shown.lines().count() < 12, "{shown}");
        // The underline starts under the offending form.
        let code_row = shown.lines().find(|l| l.contains("5000 |")).expect("code row");
        let under = shown.lines().find(|l| l.contains('^')).expect("underline row");
        assert_eq!(under.find('^'), code_row.find("(frac"), "{shown}");
    }

    #[test]
    fn labels_in_another_file_still_render() {
        let mut opts = Options::new("t.zr");
        let out = compile("(macro (m a) (frac a))\n\n(show (m 1 2))\n", &mut opts);
        let diag = out
            .diags
            .iter()
            .find(|d| !d.labels.is_empty())
            .or_else(|| out.diags.first())
            .expect("diag");
        let shown = render_diag(diag, &out.sources, false);
        assert!(shown.contains("t.zr"), "{shown}");
    }

    #[test]
    fn printing_many_diagnostics_in_a_big_file_is_fast() {
        let mut text = String::new();
        for i in 0..2000 {
            let _ = write!(text, "(show (frac {i}))\n\n");
        }
        while text.len() < 2_000_000 {
            text.push_str("filler filler filler filler filler filler filler filler\n");
        }
        let mut opts = Options::new("many.zr");
        let out = compile(&text, &mut opts);
        assert!(out.diags.len() >= 2000, "{}", out.diags.len());
        let start = std::time::Instant::now();
        let mut total = 0;
        for d in &out.diags {
            total += render_diag(d, &out.sources, false).len();
        }
        let took = start.elapsed();
        assert!(total > 0);
        assert!(took.as_secs_f64() < 1.0, "printing took {took:?}");
    }
}
