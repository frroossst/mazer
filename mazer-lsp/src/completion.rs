//! `textDocument/completion` and `completionItem/resolve`, driven by
//! `mazer_doc::locate`.
//!
//! - **Head of a list**: keywords (each primary name and second name), the
//!   note's and the prelude's macros and aliases. At the head of a top-level
//!   form, the reserved words instead; `tag` as the first item of a `show`;
//!   the directives `header`, `row`, `align`, `style` first inside a
//!   `(table ...)`.
//! - **Argument**: `let` variables in scope, macro parameters inside a macro
//!   body, display names (with their character), aliases, then keywords
//!   ranked below (a bare keyword renders its sign; one without a sign is
//!   inserted as a call). Inside `(align ...)` of a table: `left`, `center`,
//!   `right`; as the argument of `(style ...)`: `grid`, `clean`, `plain`.
//! - **Inside `(ref ...)`**: the note's labels.
//! - **Prose**: emoji shortcodes after a `:`, the reserved words after a `(`.
//!   Nothing inside comments, string literals, fenced code or inline code.
//!
//! Ranking (`sortText`): what the note defines first, then display names
//! and aliases, then keywords. Every item carries a `textEdit` over the whole
//! word under the cursor, so names such as `+`, `->` or `f'` replace
//! correctly and completing in the middle of a word does not duplicate its
//! tail. Calls are inserted as snippets (tab stops for the required
//! arguments) only when the client supports them and nothing but a `)`
//! follows on the line, so completing the head of an existing call does not
//! add arguments it already has.
//!
//! # Light items, full docs on resolve
//!
//! An argument position offers a few hundred names, so the list carries only
//! what is needed to show, filter and insert each item: label, kind,
//! `detail`, a one-line plain `documentation` (Neovim 0.11's built-in popup
//! shows only what the list carries; it resolves an item only after it is
//! accepted), sort and filter text, the text edit, and a small `data`
//! payload (`{"what": .., "name": .., "at": ..}`; the server adds the
//! document `uri`). [`resolve`] turns that payload into the full markdown
//! documentation (signature, parameters, examples, values, sources).

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit, Documentation,
    InsertTextFormat, MarkupContent, MarkupKind, Range, TextEdit,
};
use mazer_doc::math::{Keyword, display_name, display_names, registry};
use mazer_doc::syntax::{Node, emoji::EMOJI};
use mazer_doc::{Analysis, ByteOffset, Cursor, Macro, Sym, locate};
use serde_json::{Value, json};

use crate::convert::{self, note_text};
use crate::docs::{self, FENCE};
use crate::tree;

/// What the client can do.
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    /// The client accepts snippets (`${1:name}` tab stops).
    pub snippets: bool,
}

/// The directives of a `(table ...)`, with a one-line description each.
const TABLE_DIRECTIVES: &[(&str, &str)] = &[
    ("header", "A bold header row: (header a b ...)."),
    ("row", "A data row: (row a b ...)."),
    ("align", "Column alignment: (align left center right); the last value repeats."),
    ("style", "The look of the table: (style grid), (style clean) or (style plain)."),
];

const ALIGN_VALUES: &[&str] = &["left", "center", "right"];
const STYLE_VALUES: &[(&str, &str)] = &[
    ("grid", "Lines between every row and column, and a frame (the default)."),
    ("clean", "A frame and one rule under the header."),
    ("plain", "No lines at all."),
];

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
            builder.labels(builder.range(start, builder.word_end()))
        }
        Cursor::Head { list } => builder.head(list),
        Cursor::Arg { list, .. } => builder.arg(Some(list)),
        Cursor::Name { .. } => builder.arg(mazer_doc::enclosing_lists(analysis, at).last().copied()),
    }
}

/// The `textDocument/completion` result for `items` of the note `uri`.
///
/// The `data` of each item the note defines (a macro, alias, `let` variable
/// or label) gets the `uri`, so [`resolve`] can find the note; built-in names
/// need none. When the client supports a default edit range for a
/// completion list (`completionList.itemDefaults` with `editRange`, LSP
/// 3.17; Neovim 0.11 does), the range shared by the items is sent once as
/// `itemDefaults.editRange` and those items carry only `textEditText` (when
/// it differs from the label) instead of a whole `textEdit`. Otherwise the
/// plain array of items.
#[must_use]
pub fn response(items: Vec<CompletionItem>, uri: &str, edit_range_defaults: bool) -> Value {
    let common = edit_range_defaults
        .then(|| {
            items.iter().find_map(|item| match &item.text_edit {
                Some(CompletionTextEdit::Edit(edit)) => Some(edit.range),
                _ => None,
            })
        })
        .flatten();
    let mut out = Vec::with_capacity(items.len());
    for mut item in items {
        if let Some(Value::Object(data)) = item.data.as_mut()
            && matches!(data.get("what").and_then(Value::as_str), Some("macro" | "let" | "label"))
        {
            data.insert("uri".to_owned(), Value::String(uri.to_owned()));
        }
        let shared = match (&item.text_edit, common) {
            (Some(CompletionTextEdit::Edit(edit)), Some(range)) if edit.range == range => Some(edit.new_text.clone()),
            _ => None,
        };
        if shared.is_some() {
            item.text_edit = None;
        }
        let mut value = serde_json::to_value(&item).unwrap_or(Value::Null);
        if let (Some(new_text), Value::Object(fields)) = (shared, &mut value)
            && new_text != item.label
        {
            fields.insert("textEditText".to_owned(), Value::String(new_text));
        }
        out.push(value);
    }
    match common {
        Some(range) => json!({"isIncomplete": false, "itemDefaults": {"editRange": range}, "items": out}),
        None => Value::Array(out),
    }
}

/// Fills in the full markdown documentation of an item from its `data`.
///
/// `analysis` is the note the item came from (`None` if it is no
/// longer open: only built-in names can be resolved then). An item that
/// cannot be resolved is returned unchanged.
#[must_use]
pub fn resolve(analysis: Option<&Analysis>, mut item: CompletionItem) -> CompletionItem {
    let Some(data) = item.data.as_ref() else {
        return item;
    };
    let what = data.get("what").and_then(Value::as_str).unwrap_or("");
    let Some(name) = data.get("name").and_then(Value::as_str) else {
        return item;
    };
    let at = data.get("at").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok());
    let sym = Sym::new(name);
    let doc = match what {
        "keyword" => registry().get(sym).map(|k| docs::keyword_markdown(k, name)),
        "display" => display_name(sym).map(docs::display_markdown),
        "reserved" => Some(docs::reserved_markdown(name)),
        "macro" => analysis.and_then(|a| a.macros.get(sym).map(|def| docs::macro_markdown(def, &place(a, def)))),
        "let" => analysis.and_then(|a| {
            let binding = a.lets.iter().find(|b| b.name == sym && Some(b.span.start.0) == at)?;
            let (line, _) = a.sources.line_col(binding.span.file, binding.span.start);
            Some(format!(
                "`let` variable, bound on line {}:\n```{FENCE}\n{}\n```",
                line.0 + 1,
                a.sources.text(binding.span)
            ))
        }),
        "label" => analysis.and_then(|a| {
            let label = a.labels.get(sym)?;
            let equation = mazer_doc::equation_span(a, label).map_or("", |span| a.sources.text(span));
            Some(format!("Equation ({}):\n```{FENCE}\n{equation}\n```", label.number))
        }),
        _ => None,
    };
    if let Some(doc) = doc {
        item.documentation = Some(markdown(doc));
    }
    item
}

/// "on line N" for a definition in the note, or the prelude.
fn place(analysis: &Analysis, def: &Macro) -> String {
    if def.span.file == convert::NOTE {
        let (line, _) = analysis.sources.line_col(def.span.file, def.span.start);
        format!("on line {}", line.0 + 1)
    } else {
        "in the built-in prelude".to_owned()
    }
}

struct Builder<'a> {
    analysis: &'a Analysis,
    text: &'a str,
    at: ByteOffset,
    config: Config,
}

impl Builder<'_> {
    /// The range from `start` to `end`.
    fn range(&self, start: usize, end: usize) -> Range {
        convert::note_range(&self.analysis.sources, ByteOffset::from_usize(start), ByteOffset::from_usize(end))
    }

    /// The whole word under the cursor (see [`Self::word_start`]).
    fn word_range(&self, floor: usize) -> Range {
        self.range(self.word_start(floor), self.word_end())
    }

    /// The end of the word the cursor is in: forward over atom characters.
    fn word_end(&self) -> usize {
        self.end_while(|ch| !(ch.is_whitespace() || matches!(ch, '(' | ')' | '"' | ';')))
    }

    /// Forward from the cursor while `keep` holds.
    fn end_while(&self, keep: impl Fn(char) -> bool) -> usize {
        let at = self.at.to_usize();
        let after = &self.text[at..];
        at + after.find(|ch: char| !keep(ch)).unwrap_or(after.len())
    }

    /// Whether a snippet may be inserted for a word ending at `end`: the
    /// client supports snippets and the line goes on with nothing but `)`.
    fn snippet_fits(&self, end: usize) -> bool {
        let rest = self.text[end..].split('\n').next().unwrap_or("").trim_start();
        self.config.snippets && (rest.is_empty() || rest.starts_with(')'))
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
        let range = self.word_range(list.span.start.to_usize() + 1);
        let snippet = self.snippet_fits(self.word_end());
        let chain = mazer_doc::enclosing_lists(self.analysis, self.at);
        let parent = chain.len().checked_sub(2).and_then(|i| chain.get(i)).copied();
        let is_root = mazer_doc::code_forms(self.analysis).iter().any(|(form, _)| form.span == list.span);
        if is_root {
            return docs::RESERVED.iter().map(|(name, _)| Self::reserved(name, range, snippet)).collect();
        }
        if parent.is_some_and(|p| p.is_form("macro") && p.args().first().is_some_and(|sig| sig.span == list.span)) {
            return Vec::new(); // naming a new macro
        }
        let mut items = Vec::new();
        if let Some(parent) = parent
            && parent.is_form("show")
            && parent.args().first().is_some_and(|first| first.span == list.span)
        {
            items.push(Self::reserved("tag", range, snippet));
        }
        if parent.is_some_and(|p| p.is_form("table")) {
            for (name, doc) in TABLE_DIRECTIVES {
                items.push(word_item(name, CompletionItemKind::KEYWORD, "table directive", doc, "directive", range, "0"));
            }
        }
        for keyword in registry() {
            for name in keyword.names() {
                items.push(self.keyword(keyword, name, range, Some(snippet)));
            }
        }
        for def in self.analysis.macros.iter() {
            items.push(self.macro_item(def, range, Some(snippet)));
        }
        items
    }

    /// The values of a table directive, if `list` is one and the cursor is
    /// where a value goes.
    fn directive_values(&self, list: &Node, parent: Option<&Node>, index: Option<usize>) -> Option<Vec<CompletionItem>> {
        if !parent.is_some_and(|p| p.is_form("table")) {
            return None;
        }
        let range = self.word_range(list.span.start.to_usize() + 1);
        if list.is_form("align") {
            return Some(
                ALIGN_VALUES
                    .iter()
                    .map(|v| word_item(v, CompletionItemKind::ENUM_MEMBER, "column alignment", "Align this column.", "directive", range, "0"))
                    .collect(),
            );
        }
        if list.is_form("style") {
            if index != Some(0) {
                return Some(Vec::new());
            }
            return Some(
                STYLE_VALUES
                    .iter()
                    .map(|(v, doc)| word_item(v, CompletionItemKind::ENUM_MEMBER, "table style", doc, "directive", range, "0"))
                    .collect(),
            );
        }
        None
    }

    fn arg(&self, list: Option<&Node>) -> Vec<CompletionItem> {
        let Some(list) = list else {
            return Vec::new();
        };
        let index = mazer_doc::arg_index(list, self.at);
        let chain = mazer_doc::enclosing_lists(self.analysis, self.at);
        let root = chain.first().copied().unwrap_or(list);
        let parent = chain.len().checked_sub(2).and_then(|i| chain.get(i)).copied();
        let naming = (index == Some(0) && (list.is_form("let") || list.is_form("alias") || list.is_form("macro")))
            || list.is_form("tag")
            || parent.is_some_and(|p| p.is_form("macro") && p.args().first().is_some_and(|sig| sig.span == list.span));
        if naming {
            return Vec::new(); // a new name is being written
        }
        if let Some(values) = self.directive_values(list, parent, index) {
            return values;
        }
        let range = self.word_range(list.span.start.to_usize() + 1);
        let mut items = Vec::new();
        let in_definition = chain.iter().any(|l| l.is_form("alias") || l.is_form("macro"));
        if !in_definition && (root.is_form("show") || root.is_form("eval") || root.is_form("let")) {
            for binding in mazer_doc::bindings_in_scope(self.analysis, self.at) {
                let (line, _) = self.analysis.sources.line_col(binding.span.file, binding.span.start);
                items.push(CompletionItem {
                    label: binding.name.as_str().to_owned(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(shorten(&binding.value.to_string())),
                    documentation: Some(Documentation::String(format!("`let` variable, bound on line {}", line.0 + 1))),
                    sort_text: Some(format!("0_{}", binding.name)),
                    filter_text: Some(binding.name.as_str().to_owned()),
                    text_edit: Some(edit(range, binding.name.as_str())),
                    data: Some(json!({"what": "let", "name": binding.name.as_str(), "at": binding.span.start.0})),
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
                    filter_text: Some(param.as_str().to_owned()),
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
                    description: None,
                }),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(display.unicode.to_owned()),
                documentation: Some(Documentation::String(one_line(display.doc))),
                sort_text: Some(format!("1_{}", display.name)),
                // Typing words of the description finds the name too.
                filter_text: Some(format!("{} {}", display.name, display.doc)),
                text_edit: Some(edit(range, display.name)),
                data: Some(json!({"what": "display", "name": display.name})),
                ..CompletionItem::default()
            });
        }
        for def in self.analysis.macros.iter().filter(|m| m.is_alias()) {
            items.push(self.macro_item(def, range, None));
        }
        for keyword in registry() {
            for name in keyword.names() {
                items.push(self.keyword(keyword, name, range, None));
            }
        }
        items
    }

    fn labels(&self, range: Range) -> Vec<CompletionItem> {
        let mut labels: Vec<_> = self.analysis.labels.iter().collect();
        labels.sort_by_key(|l| l.number);
        labels
            .into_iter()
            .map(|label| CompletionItem {
                label: label.name.as_str().to_owned(),
                kind: Some(CompletionItemKind::REFERENCE),
                detail: Some(format!("({})", label.number)),
                documentation: Some(Documentation::String(format!("Equation ({})", label.number))),
                sort_text: Some(format!("{:06}", label.number)),
                filter_text: Some(label.name.as_str().to_owned()),
                text_edit: Some(edit(range, label.name.as_str())),
                data: Some(json!({"what": "label", "name": label.name.as_str()})),
                ..CompletionItem::default()
            })
            .collect()
    }

    fn prose(&self) -> Vec<CompletionItem> {
        let at = self.at.to_usize();
        // A comment or string of a code form, or fenced / inline code.
        let in_code = mazer_doc::code_forms(self.analysis)
            .iter()
            .any(|(form, _)| mazer_doc::is_inside(self.text, form, self.at));
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
                let mut end = self.end_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-'));
                if self.text[end..].starts_with(':') {
                    end += 1;
                }
                let range = self.range(line_start + colon, end);
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
            let end = self.end_while(|c| c.is_ascii_lowercase());
            let range = self.range(line_start + open, end);
            let snippet = self.snippet_fits(end);
            return docs::RESERVED.iter().map(|(name, _)| Self::reserved(name, range, snippet)).collect();
        }
        Vec::new()
    }

    /// A reserved word; `snippet` says whether to insert its parameters.
    fn reserved(name: &str, range: Range, snippet: bool) -> CompletionItem {
        let new_text = if snippet { docs::reserved_snippet(name) } else { name.to_owned() };
        let summary = docs::reserved_signature(name).map_or_else(String::new, |s| one_line(&s.doc));
        CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: docs::reserved_signature(name).map(|s| s.label().0),
            documentation: Some(Documentation::String(summary)),
            sort_text: Some(format!("0_{name}")),
            filter_text: Some(name.to_owned()),
            insert_text_format: snippet.then_some(InsertTextFormat::SNIPPET),
            text_edit: Some(edit(range, &new_text)),
            data: Some(json!({"what": "reserved", "name": name})),
            ..CompletionItem::default()
        }
    }

    /// A keyword under one of its names. At a head (`head` is `Some`, saying
    /// whether a snippet fits there) it is inserted with its required
    /// parameters as tab stops; as an argument it is inserted bare when it
    /// has a sign, else as a whole call.
    fn keyword(&self, keyword: &Keyword, name: &str, range: Range, head: Option<bool>) -> CompletionItem {
        let sig = docs::keyword_signature(keyword, name);
        let required = sig.has_required();
        let (new_text, format) = match (head, keyword.sign) {
            (Some(true), _) if required => (sig.snippet(name), Some(InsertTextFormat::SNIPPET)),
            (None, None) if required && self.config.snippets => {
                (format!("({})", sig.snippet(name)), Some(InsertTextFormat::SNIPPET))
            }
            _ => (name.to_owned(), None),
        };
        let head = head.is_some();
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
            documentation: Some(Documentation::String(one_line(keyword.doc))),
            sort_text: Some(format!("{}_{name}", if head { 0 } else { 2 })),
            filter_text: Some(name.to_owned()),
            insert_text_format: format,
            text_edit: Some(edit(range, &new_text)),
            data: Some(json!({"what": "keyword", "name": name})),
            ..CompletionItem::default()
        }
    }

    /// A macro or alias; `head` as for [`Self::keyword`].
    fn macro_item(&self, def: &Macro, range: Range, head: Option<bool>) -> CompletionItem {
        let sig = docs::macro_signature(def);
        let snippet = head == Some(true) && sig.has_required();
        let head = head.is_some();
        let new_text = if snippet { sig.snippet(def.name.as_str()) } else { def.name.as_str().to_owned() };
        let kind = if def.is_alias() { "alias" } else { "macro" };
        let summary = def.doc.as_deref().map_or_else(|| format!("Notation ({kind}) {}", place(self.analysis, def)), one_line);
        CompletionItem {
            label: def.name.as_str().to_owned(),
            kind: Some(if def.is_alias() {
                CompletionItemKind::CONSTANT
            } else {
                CompletionItemKind::FUNCTION
            }),
            detail: Some(format!("{kind} {}", shorten(&format!("{} = {}", def.signature(), def.body)))),
            documentation: Some(Documentation::String(summary)),
            sort_text: Some(format!("{}_{}", u8::from(!head), def.name)),
            filter_text: Some(def.name.as_str().to_owned()),
            insert_text_format: snippet.then_some(InsertTextFormat::SNIPPET),
            text_edit: Some(edit(range, &new_text)),
            data: Some(json!({"what": "macro", "name": def.name.as_str()})),
            ..CompletionItem::default()
        }
    }
}

/// A plain word item (table directives and their values).
fn word_item(
    name: &str,
    kind: CompletionItemKind,
    detail: &str,
    doc: &str,
    what: &str,
    range: Range,
    rank: &str,
) -> CompletionItem {
    CompletionItem {
        label: name.to_owned(),
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        documentation: Some(Documentation::String(doc.to_owned())),
        sort_text: Some(format!("{rank}_{name}")),
        filter_text: Some(name.to_owned()),
        text_edit: Some(edit(range, name)),
        data: Some(json!({"what": what, "name": name})),
        ..CompletionItem::default()
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

/// The first sentence (or line) of a doc text, at most 100 characters: the
/// one-line documentation a list item carries.
fn one_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let sentence = line.find(". ").map_or(line, |i| &line[..=i]);
    match sentence.char_indices().nth(100) {
        Some((cut, _)) => format!("{}…", &sentence[..cut]),
        None => sentence.to_owned(),
    }
}

#[cfg(test)]
mod tests;
