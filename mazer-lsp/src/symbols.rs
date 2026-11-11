//! `textDocument/documentSymbol`: the note's outline.
//!
//! Top-level headings form a hierarchy: a heading's section runs to the next
//! heading of the same or a higher level. Inside each section, in source
//! order: subsections, and the definitions written there (macros, aliases,
//! `let` variables) and tagged equations. Anything before the first heading
//! is at the top level.

use lsp_types::{DocumentSymbol, SymbolKind};
use mazer_doc::syntax::BlockKind;
use mazer_doc::{Analysis, ByteOffset, Span};

use crate::convert::{self, NOTE, note_text};
use mazer_doc::Place;

/// One outline entry before nesting.
struct Entry {
    /// `Some(level)` for a heading.
    level: Option<u8>,
    span: Span,
    symbol: DocumentSymbol,
}

/// The outline of the note.
#[must_use]
pub fn document_symbols(analysis: &Analysis) -> Vec<DocumentSymbol> {
    let mut entries = headings(analysis);
    entries.extend(definitions(analysis));
    entries.sort_by_key(|e| (e.span.start, e.level.is_none()));
    nest(analysis, entries)
}

fn headings(analysis: &Analysis) -> Vec<Entry> {
    let sources = &analysis.sources;
    let mut out = Vec::new();
    for block in &analysis.written.blocks {
        let BlockKind::Heading { level, .. } = block.kind else {
            continue;
        };
        if block.span.file != NOTE {
            continue;
        }
        let line = sources.text(block.span).lines().next().unwrap_or("");
        let title = line.trim_start().trim_start_matches('#').trim();
        let name = if title.is_empty() { "(untitled)".to_owned() } else { title.to_owned() };
        let range = convert::range(sources, block.span);
        out.push(Entry {
            level: Some(level),
            span: block.span,
            symbol: symbol(name, None, SymbolKind::NAMESPACE, range, range),
        });
    }
    out
}

fn definitions(analysis: &Analysis) -> Vec<Entry> {
    let sources = &analysis.sources;
    let mut out = Vec::new();
    for (form, place) in mazer_doc::code_forms(analysis) {
        let range = convert::range(sources, form.span);
        let name_range = |node: Option<&mazer_doc::syntax::Node>| node.map_or(range, |n| convert::range(sources, n.span));
        if form.is_form("let") {
            let Some(binding) = analysis.lets.iter().find(|b| b.span == form.span) else {
                continue;
            };
            out.push(Entry {
                level: None,
                span: form.span,
                symbol: symbol(
                    binding.name.as_str().to_owned(),
                    Some(binding.value.to_string()),
                    SymbolKind::VARIABLE,
                    range,
                    name_range(mazer_doc::name_node(form)),
                ),
            });
        } else if form.is_form("alias") || form.is_form("macro") {
            let Some(def) = mazer_doc::name_node(form)
                .and_then(mazer_doc::syntax::Node::as_ident)
                .and_then(|sym| analysis.macros.get(sym))
                .filter(|m| m.span == form.span)
            else {
                continue;
            };
            let (kind, detail) = if def.is_alias() {
                (SymbolKind::CONSTANT, format!("alias {}", def.body))
            } else {
                (SymbolKind::FUNCTION, def.signature())
            };
            out.push(Entry {
                level: None,
                span: form.span,
                symbol: symbol(def.name.as_str().to_owned(), Some(detail), kind, range, name_range(mazer_doc::name_node(form))),
            });
        } else if form.is_form("show") && place == Place::Block {
            let Some(tag) = form.args().first().filter(|t| t.is_form("tag")) else {
                continue;
            };
            let Some(label) = analysis.labels.iter().find(|l| l.span == tag.span) else {
                continue;
            };
            out.push(Entry {
                level: None,
                span: form.span,
                symbol: symbol(
                    format!("({}) {}", label.number, label.name),
                    Some(sources.text(form.span).to_owned()),
                    SymbolKind::KEY,
                    range,
                    convert::range(sources, tag.span),
                ),
            });
        }
    }
    out
}

/// Nests the entries (sorted by start) under their headings.
fn nest(analysis: &Analysis, entries: Vec<Entry>) -> Vec<DocumentSymbol> {
    let sources = &analysis.sources;
    let end_of_note = ByteOffset::from_usize(note_text(sources).len());
    // Where each heading's section ends: at the next heading of the same or a
    // higher level.
    let heads: Vec<(usize, u8, ByteOffset)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.level.map(|l| (i, l, e.span.start)))
        .collect();
    let section_end = |i: usize, level: u8| {
        heads
            .iter()
            .find(|(j, l, _)| *j > i && *l <= level)
            .map_or(end_of_note, |(_, _, start)| *start)
    };

    let mut root: Vec<DocumentSymbol> = Vec::new();
    // Open sections: (end of section, symbol).
    let mut stack: Vec<(ByteOffset, DocumentSymbol)> = Vec::new();
    let close = |stack: &mut Vec<(ByteOffset, DocumentSymbol)>, root: &mut Vec<DocumentSymbol>| {
        if let Some((_, done)) = stack.pop() {
            match stack.last_mut() {
                Some((_, parent)) => parent.children.get_or_insert_with(Vec::new).push(done),
                None => root.push(done),
            }
        }
    };
    for (i, entry) in entries.into_iter().enumerate() {
        while stack.last().is_some_and(|(end, _)| *end <= entry.span.start) {
            close(&mut stack, &mut root);
        }
        match entry.level {
            Some(level) => {
                let end = section_end(i, level);
                let mut symbol = entry.symbol;
                symbol.range = convert::note_range(sources, entry.span.start, end.max(entry.span.end));
                stack.push((end.max(entry.span.end), symbol));
            }
            None => match stack.last_mut() {
                Some((_, parent)) => parent.children.get_or_insert_with(Vec::new).push(entry.symbol),
                None => root.push(entry.symbol),
            },
        }
    }
    while !stack.is_empty() {
        close(&mut stack, &mut root);
    }
    root
}

#[allow(deprecated)] // `DocumentSymbol::deprecated` must be written, though it is deprecated
const fn symbol(
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    range: lsp_types::Range,
    selection_range: lsp_types::Range,
) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: None,
    }
}

#[cfg(test)]
mod tests {
    use mazer_doc::analyze;

    use super::*;

    /// The outline as indented lines: `name kind`.
    fn outline(src: &str) -> Vec<String> {
        fn walk(list: &[DocumentSymbol], depth: usize, out: &mut Vec<String>) {
            for s in list {
                out.push(format!("{}{}", "  ".repeat(depth), s.name));
                assert!(s.range.start <= s.selection_range.start && s.selection_range.end <= s.range.end, "{s:?}");
                if let Some(children) = &s.children {
                    for c in children {
                        assert!(s.range.start <= c.range.start && c.range.end <= s.range.end, "{c:?} not in {s:?}");
                    }
                    walk(children, depth + 1, out);
                }
            }
        }
        let analysis = analyze(src, "t.zr");
        let mut out = Vec::new();
        walk(&document_symbols(&analysis), 0, &mut out);
        out
    }

    #[test]
    fn headings_nest_with_definitions() {
        let src = "(alias R (bb R))\n\n# Intro\n\n(let x 2)\n\n## Detail\n\n(macro (sq v) (^ v 2))\n\n(show (tag e) (= x 2))\n\n# Next τ\n\nText (let y 3) here.\n";
        assert_eq!(
            outline(src),
            ["R", "Intro", "  x", "  Detail", "    sq", "    (1) e", "Next τ", "  y"]
        );
    }

    #[test]
    fn broken_definitions_are_left_out() {
        assert_eq!(outline("(let frac 2)\n\n(alias 3 x)\n\n# H"), ["H"]);
        assert_eq!(outline(""), [] as [&str; 0]);
    }
}
