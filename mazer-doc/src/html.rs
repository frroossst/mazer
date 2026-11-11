//! HTML output. Every piece of note text goes through [`escape`]; nothing from
//! the note is ever written raw.
//!
//! Markup (kept from the old renderer so the website's CSS and the syntax
//! highlighter, which looks for `pre code`, keep working):
//!
//! | construct        | HTML |
//! |------------------|------|
//! | heading          | `<h1>`..`<h6>` |
//! | paragraph        | `<p>` (soft newlines inside become `<br/>`) |
//! | bullet list      | `<ul><li>` |
//! | numbered list    | `<ol>`, `<ol start="n">` when it does not start at 1 |
//! | checklist item   | `<li><input type="checkbox" disabled> text</li>` (`checked disabled` when ticked); a list of only checklist items is `<ul class="mazer-checklist">` |
//! | block quote      | `<blockquote>` |
//! | list item / quote content | a leading paragraph that is the only paragraph is written bare (no `<p>`), other blocks normally; with more than one paragraph every paragraph gets `<p>` |
//! | code block       | `<pre><code class="language-x">`, `<pre><code>` without a language |
//! | rule             | `<hr/>` |
//! | image            | `<img alt=".." src=".."/>`, wrapped in `<p>` as a block; with a `javascript:`, `vbscript:` or non-image `data:` URL, just the alt text |
//! | bold / italic / underline / strike | `<strong>` / `<em>` / `<u>` / `<s>` |
//! | spoiler          | the old inline-styled `<span>` |
//! | link             | `<a href=".." target="_blank" rel="noopener noreferrer">`; a `javascript:`, `vbscript:` or `data:` URL is dropped and the text written plain |
//! | inline code      | `<code>` |
//! | emoji            | the character itself |
//! | line break       | `<br/>` |
//! | inline math      | `<math display="inline"><mstyle displaystyle="true">..</mstyle></math>` |
//! | display math     | `<div class="mazer-equation" id="eq-name"><math display="block">..</math><span class="mazer-eqno">(n)</span></div>` (no id or number when untagged) |
//! | reference        | `<a class="mazer-ref" href="#eq-name">(n)</a>`; plain `(n)` inside link text |
//! | error            | `<span class="mazer-error">` inline, `<pre class="mazer-error">` as a block (`<pre class="mazer-error" id="eq-name">` for a tagged display equation) |
//!
//! A heading, list item, block quote or inline style whose content writes
//! nothing (only sound definitions and whitespace) is left out entirely, and
//! so is a list all of whose items are left out.
//!
//! Rendering is linear in the size of the note: the analysis diagnostics are
//! indexed by position once per [`body`] call, and "already reported" is a
//! hash set.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

use mazer_syntax::{Block, BlockKind, Diag, DiagCode, Diags, Inline, InlineKind, Node, Severity, Span, emoji};

use crate::eval::Evaluator;
use crate::labels::{Label, tag_name};
use crate::pipeline::{Analysis, Target, shown};
use crate::walk::{self, Place};

/// Writes the body of a note (no page wrapper).
pub fn body(
    analysis: &Analysis,
    target: Target,
    evaluator: Option<&mut (dyn Evaluator + 'static)>,
    diags: &mut Diags,
) -> String {
    let mut errors: Vec<&Diag> = analysis.diags.iter().filter(|d| d.severity == Severity::Error).collect();
    errors.sort_by_key(|d| position(d.span));
    let reported = diags.iter().cloned().map(Key).collect();
    let mut writer = Writer {
        analysis,
        target,
        evaluator,
        diags,
        out: String::new(),
        errors,
        known: None,
        reported,
        in_link: false,
    };
    writer.blocks(&analysis.resolved.blocks, false);
    writer.out
}

/// `Page::Fragment`: the old `InnerHtml` shape.
pub fn fragment(name: &str, body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 200);
    meta(&mut out, name);
    out.push_str("<div id=\"mazer-wasm-container\">");
    out.push_str(body);
    out.push_str("</div>");
    out
}

/// `Page::Full`: the old full-page template, plus a title and default styles
/// for the classes that are new.
pub fn full_page(title: &str, name: &str, body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 2000);
    out.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><title>");
    escape(&mut out, title);
    out.push_str("</title>");
    out.push_str(HEAD);
    out.push_str("<style>");
    out.push_str(CSS);
    out.push_str("</style></head><body>");
    meta(&mut out, name);
    out.push_str(body);
    out.push_str("</body></html>");
    out
}

/// The old template's icon and syntax highlighter.
///
/// The highlighter is loaded from a CDN without a pinned version and without
/// `integrity`/`crossorigin`, exactly as the old template (and the website's
/// `mazer.js`) loads it. It is NOT pinned here because the version the site
/// uses is not recorded anywhere and no copy of the file was available to
/// compute a Subresource Integrity hash from; a made-up version or hash
/// would break the page. To harden it: choose a release, write
/// `@arborium/arborium@<version>` in the URL, and add
/// `integrity="sha384-<hash of that exact file>" crossorigin="anonymous"`.
const HEAD: &str = "<link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><text y=%2232%22 font-size=%2232%22>🍁</text></svg>\"><script src=\"https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js\" data-theme=\"github-light\" data-selector=\"pre code\"></script>";

/// Default styles for the classes the old renderer did not have.
const CSS: &str = ".mazer-equation{display:flex;justify-content:center;align-items:center;position:relative;margin:1em 0}\
.mazer-eqno{position:absolute;right:0}\
.mazer-ref{text-decoration:none}\
.mazer-checklist{list-style:none;padding-left:0}\
.mazer-error{color:#b00020;background:#fff0f0;border-radius:4px;white-space:pre-wrap;padding:0 0.2em}\
pre.mazer-error{padding:0.5em}";

fn meta(out: &mut String, name: &str) {
    out.push_str("<div id=\"mazer-meta\" style=\"display:none\" data-source-path=\"");
    escape(out, name);
    out.push_str("\" data-version=\"");
    out.push_str(env!("CARGO_PKG_VERSION"));
    out.push_str("\"></div>");
}

const SPOILER_BEGIN: &str = "<span style=\"background:#2f3136;color:transparent;border-radius:3px;padding:0 4px;cursor:pointer\" onmouseover=\"this.style.color='#dcddde'\" onmouseout=\"this.style.color='transparent'\">";
const SPOILER_END: &str = "</span>";

/// Writes `text` with `&`, `<`, `>`, `"` and `'` replaced by entities, so it
/// is safe both as element content and inside a double-quoted attribute.
/// Control characters HTML does not allow (C0 other than tab, newline and
/// carriage return) are dropped.
pub fn escape(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}' => {}
            _ => out.push(ch),
        }
    }
}

struct Writer<'a, 'e> {
    analysis: &'a Analysis,
    target: Target,
    evaluator: Option<&'e mut (dyn Evaluator + 'static)>,
    diags: &'a mut Diags,
    out: String,
    /// The error diagnostics of the analysis, sorted by where they start.
    errors: Vec<&'a Diag>,
    /// The analysis diagnostics as a set, built the first time it is needed.
    known: Option<HashSet<Key>>,
    /// What `diags` holds.
    reported: HashSet<Key>,
    /// Writing the text of a link, where a reference cannot be another link.
    in_link: bool,
}

/// A diagnostic as a hash-set key (every field counts).
#[derive(Clone, PartialEq, Eq)]
struct Key(Diag);

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let diag = &self.0;
        diag.severity.hash(state);
        diag.code.hash(state);
        diag.span.hash(state);
        diag.message.hash(state);
        diag.labels.hash(state);
        diag.help.hash(state);
    }
}

/// Sort key for a span: file, then start.
const fn position(span: Span) -> (u16, u32) {
    (span.file.0, span.start.0)
}

/// [`position`] of the end of a span.
const fn position_of_end(span: Span) -> (u16, u32) {
    (span.file.0, span.end.0)
}

/// The scheme of a URL that must not be written into a page, if it has one:
/// `javascript:`, `vbscript:`, or `data:` (but not `data:image/` for an
/// image). The URL is compared trimmed, lowercased, and with ASCII
/// whitespace and control characters removed, as browsers do.
fn dangerous_scheme(url: &str, image: bool) -> Option<&'static str> {
    let cleaned: String = url
        .trim()
        .chars()
        .filter(|c| !c.is_ascii_control() && !c.is_ascii_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if image && cleaned.starts_with("data:image/") {
        return None;
    }
    ["javascript:", "vbscript:", "data:"]
        .into_iter()
        .find(|scheme| cleaned.starts_with(scheme))
}

impl Writer<'_, '_> {
    fn push(&mut self, html: &str) {
        self.out.push_str(html);
    }

    fn escaped(&mut self, text: &str) {
        escape(&mut self.out, text);
    }

    /// Writes blocks. With `bare_first` (the content of a list item or a
    /// block quote), a leading paragraph is written without `<p>`, as the
    /// old renderer did, but only when it is the one paragraph there (nested
    /// lists and other blocks aside); with several paragraphs, all of them
    /// get `<p>`, so they are spaced alike. Paragraphs that write nothing
    /// (only definitions) do not count.
    fn blocks(&mut self, blocks: &[Block], bare_first: bool) {
        let paragraphs = blocks
            .iter()
            .filter(|b| matches!(&b.kind, BlockKind::Paragraph(content) if !content.iter().all(|i| self.is_silent(i))))
            .count();
        let bare_first = bare_first && paragraphs <= 1;
        for (i, block) in blocks.iter().enumerate() {
            match &block.kind {
                BlockKind::Paragraph(content) if bare_first && i == 0 => self.inlines(content),
                _ => self.block(block),
            }
        }
    }

    /// Writes `open`, then `body`, then `close`; if `body` wrote nothing and
    /// `has_content` (there was something, all of it silent), writes nothing
    /// at all. Returns whether anything was written.
    fn container(&mut self, open: &str, has_content: bool, body: impl FnOnce(&mut Self), close: &str) -> bool {
        let mark = self.out.len();
        self.push(open);
        let inner = self.out.len();
        body(self);
        if has_content && self.out.len() == inner {
            self.out.truncate(mark);
            return false;
        }
        self.push(close);
        true
    }

    fn block(&mut self, block: &Block) {
        match &block.kind {
            BlockKind::Heading { level, content } => {
                let level = (*level).clamp(1, 6);
                self.container(
                    &format!("<h{level}>"),
                    !content.is_empty(),
                    |w| w.inlines(content),
                    &format!("</h{level}>"),
                );
            }
            BlockKind::Paragraph(content) => {
                if !content.iter().all(|i| self.is_silent(i)) {
                    self.push("<p>");
                    self.inlines(content);
                    self.push("</p>");
                }
            }
            BlockKind::List { ordered, items } => {
                let tag = if ordered.is_some() { "ol" } else { "ul" };
                let mut open = format!("<{tag}");
                if let Some(start) = ordered.filter(|&s| s != 1) {
                    let _ = write!(open, " start=\"{start}\"");
                }
                if !items.is_empty() && items.iter().all(|item| item.checked.is_some()) {
                    open.push_str(" class=\"mazer-checklist\"");
                }
                open.push('>');
                self.container(
                    &open,
                    !items.is_empty(),
                    |w| {
                        for item in items {
                            let open = match item.checked {
                                Some(true) => "<li><input type=\"checkbox\" checked disabled> ",
                                Some(false) => "<li><input type=\"checkbox\" disabled> ",
                                None => "<li>",
                            };
                            w.container(open, !item.content.is_empty(), |w| w.blocks(&item.content, true), "</li>");
                        }
                    },
                    &format!("</{tag}>"),
                );
            }
            BlockKind::BlockQuote(inner) => {
                self.container("<blockquote>", !inner.is_empty(), |w| w.blocks(inner, true), "</blockquote>");
            }
            BlockKind::CodeBlock { lang, code } => {
                self.push("<pre><code");
                if let Some(lang) = lang {
                    self.push(" class=\"language-");
                    self.escaped(lang);
                    self.push("\"");
                }
                self.push(">");
                self.escaped(code);
                self.push("</code></pre>");
            }
            BlockKind::Image { alt, url } => {
                self.push("<p>");
                self.image(alt, url, block.span);
                self.push("</p>");
            }
            BlockKind::Code(node) => {
                self.form(node, Place::Block);
            }
            BlockKind::Rule => self.push("<hr/>"),
        }
    }

    /// Writes a run of inlines. A definition on a line of its own does not
    /// leave an empty line behind: the newline after it is dropped, and so is
    /// a trailing newline before definitions that end the run.
    fn inlines(&mut self, inlines: &[Inline]) {
        let visible_end = inlines.iter().rposition(|i| !self.is_silent(i)).map_or(0, |i| i + 1);
        let mut line_start = true;
        let mut swallow = false;
        for (i, inline) in inlines.iter().enumerate() {
            match &inline.kind {
                // Only whitespace and sound definitions follow: write nothing.
                InlineKind::Text(_) | InlineKind::LineBreak if i >= visible_end => {}
                InlineKind::Text(text) => {
                    let mut text: &str = text;
                    if swallow {
                        let rest = text.trim_start_matches([' ', '\t']);
                        if let Some(rest) = rest.strip_prefix('\n') {
                            text = rest;
                        }
                    }
                    if i + 1 == visible_end && inlines.len() > visible_end {
                        text = text.trim_end_matches([' ', '\t', '\n']);
                    }
                    swallow = false;
                    self.text(text);
                    if !text.is_empty() {
                        line_start = text.ends_with('\n');
                    }
                }
                InlineKind::Code(node) => {
                    let silent = self.form(node, Place::Inline);
                    if silent && line_start {
                        swallow = true;
                    } else if !silent {
                        line_start = false;
                        swallow = false;
                    }
                }
                InlineKind::LineBreak => {
                    self.push("<br/>");
                    line_start = true;
                    swallow = false;
                }
                _ => {
                    let before = self.out.len();
                    self.inline(inline);
                    if self.out.len() == before {
                        // A style holding only definitions: like a definition.
                        swallow = swallow || line_start;
                    } else {
                        line_start = false;
                        swallow = false;
                    }
                }
            }
        }
    }

    fn inline(&mut self, inline: &Inline) {
        match &inline.kind {
            InlineKind::Text(text) => self.text(text),
            InlineKind::Bold(inner) => self.wrapped("<strong>", inner, "</strong>"),
            InlineKind::Italic(inner) => self.wrapped("<em>", inner, "</em>"),
            InlineKind::Underline(inner) => self.wrapped("<u>", inner, "</u>"),
            InlineKind::Strike(inner) => self.wrapped("<s>", inner, "</s>"),
            InlineKind::Spoiler(inner) => self.wrapped(SPOILER_BEGIN, inner, SPOILER_END),
            InlineKind::Link { content, url } => {
                if let Some(scheme) = dangerous_scheme(url, false) {
                    self.unsafe_url(scheme, "link", inline.span);
                    self.inlines(content);
                    return;
                }
                self.push("<a href=\"");
                self.escaped(url);
                self.push("\" target=\"_blank\" rel=\"noopener noreferrer\">");
                let outer = std::mem::replace(&mut self.in_link, true);
                self.inlines(content);
                self.in_link = outer;
                self.push("</a>");
            }
            InlineKind::Image { alt, url } => self.image(alt, url, inline.span),
            InlineKind::InlineCode(code) => {
                self.push("<code>");
                self.escaped(code);
                self.push("</code>");
            }
            InlineKind::Emoji(name) => match emoji::lookup(name.as_str()) {
                Some(ch) => self.push(ch),
                None => self.escaped(&format!(":{name}:")),
            },
            InlineKind::Code(node) => {
                self.form(node, Place::Inline);
            }
            InlineKind::LineBreak => self.push("<br/>"),
        }
    }

    /// A style: nothing at all if its content writes nothing.
    fn wrapped(&mut self, open: &str, inner: &[Inline], close: &str) {
        self.container(open, !inner.is_empty(), |w| w.inlines(inner), close);
    }

    /// Prose text; a soft newline becomes `<br/>`.
    fn text(&mut self, text: &str) {
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                self.push("<br/>");
            }
            self.escaped(line);
        }
    }

    fn image(&mut self, alt: &str, url: &str, span: Span) {
        if let Some(scheme) = dangerous_scheme(url, true) {
            self.unsafe_url(scheme, "image", span);
            self.escaped(alt);
            return;
        }
        self.push("<img alt=\"");
        self.escaped(alt);
        self.push("\" src=\"");
        self.escaped(url);
        self.push("\"/>");
    }

    /// Reports a link or image URL that was not written.
    fn unsafe_url(&mut self, scheme: &str, what: &str, span: Span) {
        let diag = Diag::warning(
            DiagCode::Other,
            span,
            format!("this {what} uses a `{scheme}` URL, which is not allowed, so it is shown as plain text"),
        )
        .with_help("use an http(s) or relative URL");
        self.report(diag);
    }

    /// Whether an inline shows nothing: whitespace, a line break, a sound
    /// definition, or a style holding only those.
    fn is_silent(&self, inline: &Inline) -> bool {
        match &inline.kind {
            InlineKind::Text(text) => text.trim().is_empty(),
            InlineKind::LineBreak => true,
            InlineKind::Code(node) => is_definition(node) && self.is_sound(node, &Diags::new()),
            InlineKind::Bold(inner)
            | InlineKind::Italic(inner)
            | InlineKind::Underline(inner)
            | InlineKind::Strike(inner)
            | InlineKind::Spoiler(inner) => !inner.is_empty() && inner.iter().all(|i| self.is_silent(i)),
            _ => false,
        }
    }

    /// Whether a form is free of document-stage errors: no `Error` node in
    /// it, and no error diagnostic inside it other than those in `emitted`
    /// (what rendering its expressions reported).
    fn is_sound(&self, node: &Node, emitted: &Diags) -> bool {
        !walk::contains_error(node) && self.problems(node, emitted).is_empty()
    }

    /// Messages of the error diagnostics inside `node` that rendering it does
    /// not produce itself.
    fn problems(&self, node: &Node, emitted: &Diags) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let first = self.errors.partition_point(|d| position(d.span) < position(node.span));
        for diag in &self.errors[first..] {
            // Sorted by position: everything from here on starts past `node`.
            if position(diag.span) > position_of_end(node.span) {
                break;
            }
            if walk::span_within(diag.span, node.span)
                && !emitted.iter().any(|e| same_problem(e, diag))
                && !out.contains(&diag.message)
            {
                out.push(diag.message.clone());
            }
        }
        out
    }

    /// Writes one code form. Returns `true` if it wrote nothing (a sound
    /// definition).
    fn form(&mut self, node: &Node, place: Place) -> bool {
        if node.is_form("show") {
            self.show(node, place);
        } else if node.is_form("eval") {
            self.eval(node, place);
        } else if node.is_form("ref") {
            self.reference(node);
        } else if is_definition(node) {
            if self.is_sound(node, &Diags::new()) {
                return true;
            }
            let problems = self.problems(node, &Diags::new());
            self.error_box(place, &problems);
        } else {
            let problems = self.problems(node, &Diags::new());
            self.error_box(place, &problems);
        }
        false
    }

    fn show(&mut self, node: &Node, place: Place) {
        let exprs = shown(node);
        let mut emitted = Diags::new();
        let bodies: Vec<String> = exprs.iter().map(|e| self.emit(e, &mut emitted)).collect();
        let checked = self.check_set(exprs, &emitted);
        let label = match node.args().first() {
            Some(tag) if tag.is_form("tag") && place == Place::Block => tag_name(tag)
                .and_then(|name| self.analysis.labels.get(name))
                .filter(|label| label.span == tag.span),
            _ => None,
        };
        if exprs.is_empty() || !self.is_sound(node, &checked) {
            let problems = self.problems(node, &checked);
            self.error_box_with_id(place, &problems, label);
            return;
        }
        self.report_unchecked(emitted, &checked);
        self.math(place, &bodies, label);
    }

    fn eval(&mut self, node: &Node, place: Place) {
        let exprs = shown(node);
        let mut checked = Diags::new();
        for expr in exprs {
            mazer_math::check(expr, &mut checked);
        }
        let [expr] = exprs else {
            let problems = self.problems(node, &checked);
            self.error_box(place, &problems);
            return;
        };
        if !self.is_sound(node, &checked) {
            let problems = self.problems(node, &checked);
            self.error_box(place, &problems);
            return;
        }
        let Some(evaluator) = self.evaluator.as_deref_mut() else {
            let diag = Diag::error(DiagCode::NoEvaluator, node.span, "no evaluator is configured, so `eval` cannot run")
                .with_help("use (show ...) to display an expression without computing it");
            let message = diag.message.clone();
            self.report(diag);
            self.error_box(place, &[message]);
            return;
        };
        let mut own = Diags::new();
        let result = evaluator.eval(expr, &mut own);
        let mut failures: Vec<String> = own
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        for diag in own.into_vec() {
            self.reported.insert(Key(diag.clone()));
            self.diags.push(diag);
        }
        if let Some(value) = result {
            let mut emitted = Diags::new();
            let body = self.emit(&value, &mut emitted);
            self.report_new(emitted);
            self.math(place, &[body], None);
            return;
        }
        if failures.is_empty() {
            let diag = Diag::error(DiagCode::EvalFailed, node.span, "evaluation failed");
            failures.push(diag.message.clone());
            self.report(diag);
        }
        self.error_box(place, &failures);
    }

    fn reference(&mut self, node: &Node) {
        let label = tag_name(node).and_then(|name| self.analysis.labels.get(name));
        match label {
            Some(label) if self.is_sound(node, &Diags::new()) => {
                if self.in_link {
                    // Links do not nest: just the number.
                    self.escaped(&format!("({})", label.number));
                } else {
                    self.ref_link(label);
                }
            }
            _ => {
                let problems = self.problems(node, &Diags::new());
                self.error_box(Place::Inline, &problems);
            }
        }
    }

    fn ref_link(&mut self, label: &Label) {
        match self.target {
            Target::MathMl => {
                self.push("<a class=\"mazer-ref\" href=\"#");
                self.escaped(&label.anchor);
                self.push(&format!("\">({})</a>", label.number));
            }
            #[cfg(feature = "latex")]
            Target::Latex => {
                let latex = format!("\\eqref{{{}}}", latex_label(&label.anchor));
                self.escaped(&latex);
            }
        }
    }

    /// One expression in the configured target.
    fn emit(&self, node: &Node, diags: &mut Diags) -> String {
        match self.target {
            Target::MathMl => mazer_math::to_mathml(node, diags),
            #[cfg(feature = "latex")]
            Target::Latex => mazer_math::to_latex(node, diags),
        }
    }

    /// What `mazer_math::check` reports for `exprs`: the diagnostics
    /// `analyze` already has. For `MathML` that is exactly what rendering
    /// reported.
    fn check_set(&self, exprs: &[Node], emitted: &Diags) -> Diags {
        if self.target == Target::MathMl {
            return emitted.clone();
        }
        let mut checked = Diags::new();
        for expr in exprs {
            mazer_math::check(expr, &mut checked);
        }
        checked
    }

    /// Pushes a diagnostic found while rendering, unless it was already
    /// reported.
    fn report(&mut self, diag: Diag) {
        let key = Key(diag);
        if !self.reported.contains(&key) {
            self.reported.insert(key.clone());
            self.diags.push(key.0);
        }
    }

    /// Passes on the diagnostics rendering produced that `analyze` did not
    /// already report.
    fn report_new(&mut self, emitted: Diags) {
        let analysis = self.analysis;
        let known = self
            .known
            .get_or_insert_with(|| analysis.diags.iter().cloned().map(Key).collect());
        let fresh: Vec<Diag> = emitted.into_vec().into_iter().filter(|d| !known.contains(&Key(d.clone()))).collect();
        for diag in fresh {
            self.report(diag);
        }
    }

    /// Passes on what rendering a `show` reported beyond what checking its
    /// expressions reports (which `analyze` has already dealt with): for
    /// `MathML` nothing, for LaTeX the target-specific problems.
    fn report_unchecked(&mut self, emitted: Diags, checked: &Diags) {
        let checked: HashSet<Key> = checked.iter().cloned().map(Key).collect();
        for diag in emitted.into_vec() {
            if !checked.contains(&Key(diag.clone())) {
                self.report(diag);
            }
        }
    }

    /// Wraps rendered expressions: inline or display, numbered if `label`.
    fn math(&mut self, place: Place, bodies: &[String], label: Option<&Label>) {
        match self.target {
            Target::MathMl => {
                let joined = if let [single] = bodies {
                    single.clone()
                } else {
                    format!("<mrow>{}</mrow>", bodies.concat())
                };
                match place {
                    Place::Inline => {
                        self.push("<math display=\"inline\"><mstyle displaystyle=\"true\">");
                        self.push(&joined);
                        self.push("</mstyle></math>");
                    }
                    Place::Block => {
                        self.equation_open(label);
                        self.push("<math display=\"block\">");
                        self.push(&joined);
                        self.push("</math>");
                        if let Some(label) = label {
                            self.push(&format!("<span class=\"mazer-eqno\">({})</span>", label.number));
                        }
                        self.push("</div>");
                    }
                }
            }
            #[cfg(feature = "latex")]
            Target::Latex => {
                let joined = bodies.join(" ");
                match place {
                    Place::Inline => self.escaped(&format!("\\(\\displaystyle {joined}\\)")),
                    Place::Block => {
                        self.equation_open(label);
                        let tag = label.map_or_else(String::new, |l| {
                            format!(" \\tag{{{}}}\\label{{{}}}", l.number, latex_label(&l.anchor))
                        });
                        self.escaped(&format!("\\[{joined}{tag}\\]"));
                        self.push("</div>");
                    }
                }
            }
        }
    }

    fn equation_open(&mut self, label: Option<&Label>) {
        self.push("<div class=\"mazer-equation\"");
        if let Some(label) = label {
            self.push(" id=\"");
            self.escaped(&label.anchor);
            self.push("\"");
        }
        self.push(">");
    }

    fn error_box(&mut self, place: Place, messages: &[String]) {
        self.error_box_with_id(place, messages, None);
    }

    /// An error box; a block one for a tagged equation keeps its anchor.
    fn error_box_with_id(&mut self, place: Place, messages: &[String], label: Option<&Label>) {
        let fallback = ["this form contains an error".to_owned()];
        let messages = if messages.is_empty() { &fallback[..] } else { messages };
        match place {
            Place::Block => {
                self.push("<pre class=\"mazer-error\"");
                if let Some(label) = label {
                    self.push(" id=\"");
                    self.escaped(&label.anchor);
                    self.push("\"");
                }
                self.push(">");
                self.escaped(&messages.join("\n"));
                self.push("</pre>");
            }
            Place::Inline => {
                self.push("<span class=\"mazer-error\">");
                self.escaped(&messages.join("; "));
                self.push("</span>");
            }
        }
    }
}

fn is_definition(node: &Node) -> bool {
    node.is_form("let") || node.is_form("alias") || node.is_form("macro")
}

/// An equation anchor as a LaTeX `\label` / `\eqref` key: ASCII letters,
/// digits, `:`, `_` and `-` are kept, every other character (`%`, `#`, `{`,
/// a non-ASCII letter ...) becomes `-`. The same text is used for the label
/// and every reference to it, so they still match.
#[cfg(feature = "latex")]
fn latex_label(anchor: &str) -> String {
    anchor
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-') { c } else { '-' })
        .collect()
}

/// Whether two diagnostics report the same problem at the same place. A
/// problem found inside an expansion is reported by `analyze` with extra
/// labels and help pointing at the template; rendering the expression finds
/// it again without them, and it is still the same problem.
fn same_problem(a: &Diag, b: &Diag) -> bool {
    a.code == b.code && a.span == b.span && a.message == b.message
}
