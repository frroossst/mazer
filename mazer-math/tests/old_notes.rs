//! Guards the `--strict` deploy check: notes written in the old language must
//! produce at least one warning or error. Skips (passes) when the originals are
//! not present. Prints only file names, codes and counts, never note text.

use std::path::PathBuf;

use mazer_syntax::doc::{Block, BlockKind, Inline, InlineKind};
use mazer_syntax::{Diags, FileId, Node, Severity};

fn notes_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("MAZER_OLD_NOTES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/reference/original-notes"),
        PathBuf::from,
    );
    dir.is_dir().then_some(dir)
}

fn inline_forms<'a>(inlines: &'a [Inline], out: &mut Vec<&'a Node>) {
    for inline in inlines {
        match &inline.kind {
            InlineKind::Code(node) => out.push(node),
            InlineKind::Bold(c)
            | InlineKind::Italic(c)
            | InlineKind::Underline(c)
            | InlineKind::Strike(c)
            | InlineKind::Spoiler(c)
            | InlineKind::Link { content: c, .. } => inline_forms(c, out),
            _ => {}
        }
    }
}

fn block_forms<'a>(blocks: &'a [Block], out: &mut Vec<&'a Node>) {
    for block in blocks {
        match &block.kind {
            BlockKind::Code(node) => out.push(node),
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inline_forms(content, out),
            BlockKind::List { items, .. } => items.iter().for_each(|i| block_forms(&i.content, out)),
            BlockKind::BlockQuote(inner) => block_forms(inner, out),
            _ => {}
        }
    }
}

/// Codes of the warnings and errors from checking every `(show ...)` argument.
fn problems(text: &str) -> Vec<String> {
    let mut parse_diags = Diags::new();
    let doc = mazer_syntax::markdown::parse(text, FileId(0), &mut parse_diags);
    let mut forms = Vec::new();
    block_forms(&doc.blocks, &mut forms);
    let mut diags = Diags::new();
    for form in forms {
        let Some((head, args)) = form.as_list().and_then(<[Node]>::split_first) else { continue };
        if head.as_ident().map(mazer_syntax::Sym::as_str) != Some("show") {
            continue;
        }
        for arg in args {
            mazer_math::check(arg, &mut diags);
        }
    }
    diags
        .iter()
        .filter(|d| matches!(d.severity, Severity::Warning | Severity::Error))
        .map(|d| format!("{:?}", d.code))
        .collect()
}

#[test]
fn old_syntax_notes_are_flagged() {
    let Some(dir) = notes_dir() else {
        eprintln!("old notes not found; skipping");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "zr"))
        .collect();
    files.sort();
    let mut missing = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let found = problems(&text);
        let mut codes = found.clone();
        codes.sort();
        codes.dedup();
        eprintln!("{name}: {} warnings/errors {codes:?}", found.len());
        if matches!(name.as_str(), "dirac-delta.zr" | "a-prettier-printer.zr") && found.is_empty() {
            missing.push(name);
        }
    }
    assert!(missing.is_empty(), "old-syntax notes passed unflagged: {missing:?}");
}
