//! `textDocument/completion`, driven by `mazer_doc::locate`.
//!
//! - **Head of a list**: keywords (each primary name and second name), the
//!   note's and the prelude's macros and aliases. At the head of a top-level
//!   form, the reserved words instead; `tag` as the first item of a `show`.
//! - **Argument**: `let` variables in scope, macro parameters inside a macro
//!   body, display names (with their character), aliases, then keywords
//!   ranked below (a bare keyword renders its sign; one without a sign is
//!   inserted as a call).
//! - **Inside `(ref ...)`**: the note's labels.
//! - **Prose**: emoji shortcodes after a `:`, the reserved words after a `(`.
//!   Nothing inside comments, string literals, fenced code or inline code.
//!
//! Ranking (`sortText`): what the note defines first, then display names
//! and aliases, then keywords. Every item carries a `textEdit` over the word
//! being typed, so names such as `+`, `->` or `f'` replace correctly.

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit, Documentation,
    InsertTextFormat, MarkupContent, MarkupKind, Range, TextEdit,
};
use mazer_doc::math::{Keyword, display_names, registry};
use mazer_doc::syntax::{Node, emoji::EMOJI};
use mazer_doc::{Analysis, ByteOffset, Cursor, Macro, locate};

use crate::convert::{self, note_text};
use crate::docs::{self, FENCE};
use crate::tree::{self, forms};

/// What the client can do.
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    /// The client accepts snippets (`${1:name}` tab stops).
    pub snippets: bool,
}

/// The completion items at `at`.
#[must_use]
pub fn complete(analysis: &Analysis, at: ByteOffset, config: Config) -> Vec<CompletionItem> {
    let text = note_text(&analysis.sources);
    let mut at = at.to_usize().min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let at = ByteOffset::from_usize(at);
    let builder = Builder { analysis, text, at, config };
    match locate(analysis, at) {
        Cursor::Prose => builder.prose(),
        Cursor::Ref { partial } => {
            let start = at.to_usize().saturating_sub(partial.len());
            builder.labels(builder.range(start))
        }
        Cursor::Head { list } => builder.head(list),
        Cursor::Arg { list, .. } => builder.arg(Some(list)),
        Cursor::Name { .. } => builder.arg(tree::enclosing_lists(analysis, at).last().copied()),
    }
}

struct Builder<'a> {
    analysis: &'a Analysis,
    text: &'a str,
    at: ByteOffset,
    config: Config,
}

impl Builder<'_> {
    /// The range from `start` to the cursor.
    fn range(&self, start: usize) -> Range {
        convert::note_range(&self.analysis.sources, ByteOffset::from_usize(start), self.at)
    }

    /// The start of the word being typed: back over atom characters, then
    /// forward past a quote mark `'` or a negating `-` (`-x` is `(- x)`).
    fn word_start(&self, floor: usize) -> usize {
        let at = self.at.to_usize();
        let before = &self.text[floor.min(at)..at];
        let mut start = at;
        for (i, ch) in before.char_indices().rev() {
            if ch.is_whitespace() || matches!(ch, '(' | ')' | '"' | ';') {
                break;
            }
            start = floor + i;
        }
        let mut word = &self.text[start..at];
        while let Some(rest) = word.strip_prefix('\'') {
            word = rest;
        }
        if let Some(rest) = word.strip_prefix('-')
            && rest.chars().next().is_some_and(char::is_alphabetic)
        {
            word = rest;
        }
        at - word.len()
    }

    fn head(&self, list: &Node) -> Vec<CompletionItem> {
        let range = self.range(self.word_start(list.span.start.to_usize() + 1));
        let chain = tree::enclosing_lists(self.analysis, self.at);
        let parent = chain.len().checked_sub(2).and_then(|i| chain.get(i)).copied();
        let is_root = forms(self.analysis).iter().any(|(form, _)| form.span == list.span);
        if is_root {
            return docs::RESERVED.iter().map(|(name, _)| self.reserved(name, range)).collect();
        }
        if parent.is_some_and(|p| p.is_form("macro") && p.args().first().is_some_and(|sig| sig.span == list.span)) {
            return Vec::new(); // naming a new macro
        }
        let mut items = Vec::new();
        if let Some(parent) = parent
            && parent.is_form("show")
            && parent.args().first().is_some_and(|first| first.span == list.span)
        {
            items.push(self.reserved("tag", range));
        }
        for keyword in registry() {
            for name in keyword.names() {
                items.push(self.keyword(keyword, name, range, true));
            }
        }
        for def in self.analysis.macros.iter() {
            items.push(self.macro_item(def, range, true));
        }
        items
    }

    fn arg(&self, list: Option<&Node>) -> Vec<CompletionItem> {
        let Some(list) = list else {
            return Vec::new();
        };
        let index = tree::arg_index(list, self.at);
        let chain = tree::enclosing_lists(self.analysis, self.at);
        let root = chain.first().copied().unwrap_or(list);
        let parent = chain.len().checked_sub(2).and_then(|i| chain.get(i)).copied();
        let naming = (index == Some(0) && (list.is_form("let") || list.is_form("alias") || list.is_form("macro")))
            || list.is_form("tag")
            || parent.is_some_and(|p| p.is_form("macro") && p.args().first().is_some_and(|sig| sig.span == list.span));
        if naming {
            return Vec::new(); // a new name is being written
        }
        let range = self.range(self.word_start(list.span.start.to_usize() + 1));
        let mut items = Vec::new();
        let in_definition = chain.iter().any(|l| l.is_form("alias") || l.is_form("macro"));
        if !in_definition && (root.is_form("show") || root.is_form("eval") || root.is_form("let")) {
            for binding in tree::bindings_in_scope(self.analysis, self.at) {
                let (line, _) = self.analysis.sources.line_col(binding.span.file, binding.span.start);
                items.push(CompletionItem {
                    label: binding.name.as_str().to_owned(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(shorten(&binding.value.to_string())),
                    documentation: Some(markdown(format!(
                        "`let` variable, bound on line {}:\n```{FENCE}\n{}\n```",
                        line.0 + 1,
                        self.analysis.sources.text(binding.span)
                    ))),
                    sort_text: Some(format!("0_{}", binding.name)),
                    text_edit: Some(edit(range, binding.name.as_str())),
                    ..CompletionItem::default()
                });
            }
        }
        if let Some(form) = chain.iter().rev().find(|l| l.is_form("macro")) {
            for param in tree::macro_params(form) {
                items.push(CompletionItem {
                    label: param.as_str().to_owned(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some("macro parameter".to_owned()),
                    sort_text: Some(format!("0_{param}")),
                    text_edit: Some(edit(range, param.as_str())),
                    ..CompletionItem::default()
                });
            }
        }
        for display in display_names() {
            items.push(CompletionItem {
                label: display.name.to_owned(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(format!(" {}", display.unicode)),
                    description: Some(display.doc.to_owned()),
                }),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(display.unicode.to_owned()),
                documentation: Some(markdown(docs::display_markdown(display))),
                sort_text: Some(format!("1_{}", display.name)),
                // Typing words of the description finds the name too.
                filter_text: Some(format!("{} {}", display.name, display.doc)),
                text_edit: Some(edit(range, display.name)),
                ..CompletionItem::default()
            });
        }
        for def in self.analysis.macros.iter().filter(|m| m.is_alias()) {
            items.push(self.macro_item(def, range, false));
        }
        for keyword in registry() {
            for name in keyword.names() {
                items.push(self.keyword(keyword, name, range, false));
            }
        }
        items
    }

    fn labels(&self, range: Range) -> Vec<CompletionItem> {
        let mut labels: Vec<_> = self.analysis.labels.iter().collect();
        labels.sort_by_key(|l| l.number);
        labels
            .into_iter()
            .map(|label| {
                let equation = tree::tagged_equation(self.analysis, label)
                    .map_or_else(String::new, |form| self.analysis.sources.text(form.span).to_owned());
                CompletionItem {
                    label: label.name.as_str().to_owned(),
                    kind: Some(CompletionItemKind::REFERENCE),
                    detail: Some(format!("({})", label.number)),
                    documentation: Some(markdown(format!("Equation ({}):\n```{FENCE}\n{equation}\n```", label.number))),
                    sort_text: Some(format!("{:06}", label.number)),
                    text_edit: Some(edit(range, label.name.as_str())),
                    ..CompletionItem::default()
                }
            })
            .collect()
    }

    fn prose(&self) -> Vec<CompletionItem> {
        let at = self.at.to_usize();
        // A comment or string of a code form, or fenced / inline code.
        let in_code = forms(self.analysis).iter().any(|(form, _)| tree::inside(self.text, form, self.at));
        if in_code || tree::in_verbatim(&self.analysis.written, self.at) {
            return Vec::new();
        }
        let line_start = self.text[..at].rfind('\n').map_or(0, |i| i + 1);
        let line = &self.text[line_start..at];
        // `:` then the start of a shortcode: emoji.
        if let Some(colon) = line.rfind(':') {
            let tail = &line[colon + 1..];
            let before = line[..colon].chars().next_back();
            let shortcode = tail.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-'));
            if shortcode && tail.len() <= 16 && !before.is_some_and(char::is_alphanumeric) {
                let range = self.range(line_start + colon);
                return EMOJI
                    .iter()
                    .map(|(name, ch)| {
                        let code = format!(":{name}:");
                        CompletionItem {
                            label: code.clone(),
                            kind: Some(CompletionItemKind::TEXT),
                            detail: Some((*ch).to_owned()),
                            label_details: Some(CompletionItemLabelDetails {
                                detail: Some(format!(" {ch}")),
                                description: None,
                            }),
                            filter_text: Some(code.clone()),
                            text_edit: Some(edit(range, &code)),
                            ..CompletionItem::default()
                        }
                    })
                    .collect();
            }
        }
        // `(` then the start of a word: the reserved words.
        let open = line.trim_end_matches(|c: char| c.is_ascii_lowercase()).len();
        let bytes = line.as_bytes();
        if open > 0 && bytes[open - 1] == b'(' && (open < 2 || bytes[open - 2] != b'\\') {
            let range = self.range(line_start + open);
            return docs::RESERVED.iter().map(|(name, _)| self.reserved(name, range)).collect();
        }
        Vec::new()
    }

    fn reserved(&self, name: &str, range: Range) -> CompletionItem {
        let snippet = self.config.snippets;
        let new_text = if snippet { docs::reserved_snippet(name) } else { name.to_owned() };
        CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: docs::reserved_signature(name).map(|s| s.label().0),
            documentation: Some(markdown(docs::reserved_markdown(name))),
            sort_text: Some(format!("0_{name}")),
            insert_text_format: snippet.then_some(InsertTextFormat::SNIPPET),
            text_edit: Some(edit(range, &new_text)),
            ..CompletionItem::default()
        }
    }

    /// A keyword under one of its names. At a head it is inserted with its
    /// required parameters as tab stops; as an argument it is inserted bare
    /// when it has a sign, else as a whole call.
    fn keyword(&self, keyword: &Keyword, name: &str, range: Range, head: bool) -> CompletionItem {
        let sig = docs::keyword_signature(keyword, name);
        let snippet = self.config.snippets && sig.has_required();
        let (new_text, format) = match (head, keyword.sign) {
            (true, _) if snippet => (sig.snippet(name), Some(InsertTextFormat::SNIPPET)),
            (false, None) if snippet => (format!("({})", sig.snippet(name)), Some(InsertTextFormat::SNIPPET)),
            _ => (name.to_owned(), None),
        };
        let label = sig.label().0;
        let detail = match keyword.sign {
            Some(sign) => format!("{sign}  {label}"),
            None => label,
        };
        CompletionItem {
            label: name.to_owned(),
            kind: Some(if keyword.params.is_empty() {
                CompletionItemKind::KEYWORD
            } else {
                CompletionItemKind::FUNCTION
            }),
            label_details: keyword.sign.map(|sign| CompletionItemLabelDetails {
                detail: Some(format!(" {sign}")),
                description: None,
            }),
            detail: Some(detail),
            documentation: Some(markdown(docs::keyword_markdown(keyword, name))),
            sort_text: Some(format!("{}_{name}", if head { 0 } else { 2 })),
            filter_text: Some(name.to_owned()),
            insert_text_format: format,
            text_edit: Some(edit(range, &new_text)),
            ..CompletionItem::default()
        }
    }

    fn macro_item(&self, def: &Macro, range: Range, head: bool) -> CompletionItem {
        let sig = docs::macro_signature(def);
        let snippet = head && self.config.snippets && sig.has_required();
        let new_text = if snippet { sig.snippet(def.name.as_str()) } else { def.name.as_str().to_owned() };
        let place = if def.span.file == convert::NOTE {
            let (line, _) = self.analysis.sources.line_col(def.span.file, def.span.start);
            format!("on line {}", line.0 + 1)
        } else {
            "in the built-in prelude".to_owned()
        };
        let kind = if def.is_alias() { "alias" } else { "macro" };
        CompletionItem {
            label: def.name.as_str().to_owned(),
            kind: Some(if def.is_alias() {
                CompletionItemKind::CONSTANT
            } else {
                CompletionItemKind::FUNCTION
            }),
            detail: Some(format!("{kind} {}", shorten(&format!("{} = {}", docs::macro_call_text(def), def.body)))),
            documentation: Some(markdown(docs::macro_markdown(def, &place))),
            sort_text: Some(format!("{}_{}", u8::from(!head), def.name)),
            insert_text_format: snippet.then_some(InsertTextFormat::SNIPPET),
            text_edit: Some(edit(range, &new_text)),
            ..CompletionItem::default()
        }
    }
}

fn edit(range: Range, new_text: &str) -> CompletionTextEdit {
    CompletionTextEdit::Edit(TextEdit::new(range, new_text.to_owned()))
}

const fn markdown(value: String) -> Documentation {
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    })
}

/// At most 80 characters of `text`, with an ellipsis if cut.
fn shorten(text: &str) -> String {
    match text.char_indices().nth(80) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests;
