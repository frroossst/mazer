//! Parses every `.zr` note in the fixtures directory.
//!
//! The directory is `$MAZER_FIXTURES`, or `fixtures/notes` at the workspace
//! root. The notes are private and not committed, so when the directory is
//! missing this test passes without doing anything. Every note must parse
//! with no diagnostics at all.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mazer_syntax::markdown::parse;
use mazer_syntax::{Block, BlockKind, Diags, FileId, Inline, InlineKind, Span};

fn fixture_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("MAZER_FIXTURES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/notes"),
        PathBuf::from,
    );
    dir.is_dir().then_some(dir)
}

/// Spans of every code form in the document, block or inline.
fn code_spans(blocks: &[Block], out: &mut Vec<Span>) {
    fn inlines(content: &[Inline], out: &mut Vec<Span>) {
        for inline in content {
            match &inline.kind {
                InlineKind::Code(node) => out.push(node.span),
                InlineKind::Bold(c)
                | InlineKind::Italic(c)
                | InlineKind::Underline(c)
                | InlineKind::Strike(c)
                | InlineKind::Spoiler(c)
                | InlineKind::Link { content: c, .. } => inlines(c, out),
                _ => {}
            }
        }
    }
    for block in blocks {
        match &block.kind {
            BlockKind::Code(node) => out.push(node.span),
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inlines(content, out),
            BlockKind::List { items, .. } => items.iter().for_each(|item| code_spans(&item.content, out)),
            BlockKind::BlockQuote(inner) => code_spans(inner, out),
            BlockKind::CodeBlock { .. } | BlockKind::Image { .. } | BlockKind::Rule => {}
        }
    }
}

/// Offsets of every unescaped `(show` that is outside fenced code and inline code,
/// found with a deliberately simple scan independent of the parser.
fn show_offsets(text: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut in_fence = false;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches([' ', '\t', '>']).trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence {
            for (at, _) in line.match_indices("(show") {
                let after = line[at + 5..].chars().next();
                let is_form = after.is_some_and(|c| c.is_ascii_whitespace() || "()".contains(c));
                let in_inline_code = line[..at].matches('`').count() % 2 == 1;
                let escaped = line[..at].ends_with('\\');
                if is_form && !in_inline_code && !escaped {
                    found.push(line_start + at);
                }
            }
        }
        line_start += line.len();
    }
    found
}

#[test]
fn every_fixture_note_parses_and_its_shows_are_code() {
    let Some(dir) = fixture_dir() else {
        eprintln!("fixture notes not found; skipping");
        return;
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("readable fixtures directory")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "zr"))
        .collect();
    paths.sort();

    let mut total_code = 0;
    let mut total_shows = 0;
    let mut codes: BTreeMap<&str, usize> = BTreeMap::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).expect("readable note");
        let mut diags = Diags::new();
        let doc = parse(&text, FileId(0), &mut diags);

        let mut spans = Vec::new();
        code_spans(&doc.blocks, &mut spans);
        for at in show_offsets(&text) {
            assert!(
                spans.iter().any(|s| s.start.to_usize() <= at && at < s.end.to_usize()),
                "{}: `(show` at byte {at} did not become code",
                path.display()
            );
            total_shows += 1;
        }
        total_code += spans.len();
        for diag in &diags {
            *codes.entry(diag.code.name()).or_default() += 1;
        }
        // Print where, not what: the notes are private.
        let at: Vec<(&str, usize)> = diags.iter().map(|d| (d.code.name(), d.span.start.to_usize())).collect();
        assert!(diags.is_empty(), "{}: diagnostics at {at:?}", path.display());
    }
    eprintln!(
        "fixtures: {} notes, {total_code} code nodes, {total_shows} `(show` forms checked, diagnostics: {codes:?}",
        paths.len()
    );
}

#[test]
fn repository_examples_parse_cleanly() {
    for text in [
        include_str!("../../examples/markdown.md"),
        include_str!("../../examples/docs.md"),
        include_str!("../../examples/showcase.zr"),
    ] {
        let mut diags = Diags::new();
        let doc = parse(text, FileId(0), &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(doc.blocks.iter().any(|b| matches!(b.kind, BlockKind::Heading { .. })), "no heading");
        let mut spans = Vec::new();
        code_spans(&doc.blocks, &mut spans);
        for at in show_offsets(text) {
            assert!(spans.iter().any(|s| s.start.to_usize() <= at && at < s.end.to_usize()));
        }
    }
}
