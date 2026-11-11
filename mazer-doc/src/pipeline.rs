//! The pipeline entry points: [`analyze`], [`render`] and [`compile`].
//!
//! - The CLI and the website call [`compile`]: one function in, HTML and
//!   diagnostics out.
//! - The LSP calls only [`analyze`], on every edit.
//! - A live preview calls [`render`] on an existing [`Analysis`].
//!
//! # What each reserved form does
//!
//! - `(show expr)` shows `expr`; a single atom `(show G)` shows that atom.
//!   `(show a b c ...)` with more than one expression (after an optional
//!   `(tag name)`) is exactly `(show (jux a b c ...))`: `analyze` builds that
//!   `jux` node in the resolved tree (spanning the expressions), so spacing
//!   and parentheses come from the `jux` keyword, `(show (+ a b) c)` reads
//!   `(a+b) c`, and checking and rendering see the same tree.
//!   `(show (tag name) expr ...)` as a block is a numbered display equation.
//!   `(show)`, or a tag with nothing after it, is an `Arity` error.
//! - A `show` alone in a top-level paragraph is a display equation
//!   (`<div class="mazer-equation"><math display="block">`); anywhere else it
//!   is inline math in the old wrapper
//!   (`<math display="inline"><mstyle displaystyle="true">`).
//! - `(eval expr)` takes exactly one expression (`Arity` otherwise).
//!   The evaluator's result is rendered like a `show`: as a display equation
//!   for a block `eval`, inline otherwise.
//! - Code is not nested: a list headed by a reserved word (`show`, `eval`,
//!   `let`, `alias`, `macro`, `ref`) or by `tag` anywhere inside a `show`, an
//!   `eval` or a `let` value (other than the one `(tag name)` that may start a
//!   `show`) is reported once, at that inner list, and replaced by an `Error`
//!   node, so the enclosing form is an error box. Codes: `NestedEval` for
//!   `eval`, `MisplacedTag` for `tag`, `BadDefinition` for `let`, `alias`
//!   and `macro`, `Other` for `show` and `ref`. This holds inside `quote`
//!   too. A reserved word that is not the head of a list, as in
//!   `(show (jux show me))`, is an ordinary variable.
//! - `(let ...)`, `(alias ...)` and `(macro ...)` produce no output, as a
//!   block or inside a sentence, and a definition on a line of its own leaves
//!   no empty line. A paragraph, heading, list item, block quote or inline
//!   style (bold, italic, underline, strike, spoiler) whose content is only
//!   sound definitions and whitespace produces nothing at all, not even an
//!   empty element; a list whose items all vanish produces no list.
//! - `(ref name)` is a link `<a class="mazer-ref" href="#eq-name">(n)</a>`.
//!   Inside the text of a link it is just the text `(n)`, since links do not
//!   nest.
//! - A link or image whose URL (trimmed, without ASCII whitespace and control
//!   characters, lowercased) starts with `javascript:`, `vbscript:` or
//!   `data:` (other than `data:image/` for an image) is written as its plain
//!   text / alt text, with no `href` / `src`, and `render` reports one
//!   `Other` warning for it.
//!
//! # Error boxes
//!
//! A code form renders as an error box (`<pre class="mazer-error">` for a
//! block, `<span class="mazer-error">` inside a sentence) holding the escaped
//! messages, instead of its normal output, when:
//! 1. its resolved tree contains an `Error` node (reader recovery, a failed
//!    macro use, a nested reserved form, an oversized expansion or
//!    substitution, the document's node budget running out), or
//! 2. [`Analysis::diags`] holds an *error*-severity diagnostic whose primary
//!    span lies inside the form and which rendering the form's expressions
//!    does not produce itself. Problems the math emitter reports (a keyword
//!    with the wrong number of arguments, a bare keyword without a sign)
//!    already render in place as `<merror>`, so they do not box the form;
//!    everything the document stage reports (bad definitions, macro arity
//!    and depth, misplaced or duplicate tags, unresolved references, an empty
//!    `show`, reader errors) does, or
//! 3. it is an `eval` and there is no evaluator (`NoEvaluator`) or the
//!    evaluator returned `None`.
//!
//! This applies to definitions too: a broken `let`, `alias` or `macro` shows
//! a box where a good one shows nothing. A `let` renders no expression, so a
//! keyword problem in its value (reported at the `let`) boxes it; each use of
//! the variable renders the `<merror>` without reporting it again. Warnings
//! and hints never box. An `eval` that boxes for reason 1 or 2 is not
//! evaluated. The box of a tagged display equation keeps the equation's
//! `id`, so references to it still land on it.
//!
//! # Diagnostics
//!
//! `analyze` reports each problem once, including keyword-call problems (via
//! `mazer_math::check`): a problem in a `let` value is reported at the `let`,
//! not at each use, and diagnostics with the same code, span and message are
//! merged into the first. `render` therefore drops diagnostics the emitter
//! pushes again while rendering and reports only new ones: `NoEvaluator`,
//! the evaluator's own, problems in an evaluator's result, dangerous link
//! URLs, and target-specific ones such as `UnsupportedTarget` for LaTeX.

use std::collections::HashSet;
use std::sync::LazyLock;

use mazer_syntax::{Diag, DiagCode, Diags, Document, FileId, Node, NodeKind, SourceMap, Span, markdown};

use crate::STDLIB;
use crate::bindings::{Binding, LetPass};
use crate::eval::Evaluator;
use crate::html;
use crate::labels::{Labels, collect_labels};
use crate::macros::{Macros, collect_macros, expand_with};
use crate::walk::{self, Budget};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Target {
    #[default]
    MathMl,
    #[cfg(feature = "latex")]
    Latex,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum Page {
    /// A complete HTML document.
    Full { title: String },
    /// Just the inner HTML, to be placed inside an existing page.
    #[default]
    Fragment,
}

pub struct Options {
    /// The note's name, e.g. `dirac-delta.zr` or `<inline>`. Used in diagnostics.
    pub name: String,
    pub target: Target,
    pub page: Page,
    /// `None` in the website build.
    pub evaluator: Option<Box<dyn Evaluator>>,
}

impl Options {
    /// `MathML`, fragment output, no evaluator.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target: Target::default(),
            page: Page::default(),
            evaluator: None,
        }
    }
}

/// Everything known about a note after reading it, before producing output.
#[derive(Clone, Debug)]
pub struct Analysis {
    /// File 0 is the note; file 1 is the embedded `stdlib.zr`.
    pub sources: SourceMap,
    /// Exactly as typed. The LSP answers cursor queries from this tree.
    pub written: Document,
    /// Macros expanded and `let` variables substituted. `render` uses this tree.
    pub resolved: Document,
    pub macros: Macros,
    pub labels: Labels,
    /// Every `let` in the note, in document order (a name may appear more than
    /// once). The LSP uses this for completion and go-to-definition: the
    /// binding in scope at a cursor is the last one that starts before it.
    pub lets: Vec<Binding>,
    pub diags: Diags,
}

/// Reads and checks a note without producing output. Never fails.
///
/// Order: parse the markdown and read each code form -> collect `alias` /
/// `macro` (embedded `stdlib.zr`, then the note) -> expand -> substitute `let`
/// variables in document order -> collect labels -> check every keyword call.
///
/// `resolved` differs from `written` only inside code forms: the expressions
/// of `show` and `eval` and the value of `let` are expanded and substituted.
/// Definitions (`alias`, `macro`), `(tag name)` and `(ref name)` are left as
/// written.
#[must_use]
pub fn analyze(src: &str, name: &str) -> Analysis {
    analyze_with(src, name, &PRELUDE)
}

/// [`analyze`] with a given prelude (tests use their own).
fn analyze_with(src: &str, name: &str, prelude: &Prelude) -> Analysis {
    let mut sources = SourceMap::new();
    let file = sources.add(name.to_owned(), src.to_owned());
    sources.add("stdlib.zr".to_owned(), prelude.text.to_owned());

    let mut diags = prelude.diags.clone();
    let written = markdown::parse(src, file, &mut diags);

    let mut macros = prelude.macros.clone();
    collect_macros(&written, &mut macros, &mut diags);

    let mut resolved = written.clone();
    let mut stage = Resolver {
        macros: &macros,
        pass: LetPass::default(),
        budget: Budget::new(),
        diags: &mut diags,
    };
    walk::for_each_code_mut(&mut resolved, &mut |form, _| stage.form(form));
    let pass = stage.pass;

    let labels = collect_labels(&written, &mut diags);

    walk::for_each_code(&resolved, &mut |form, _| {
        for expr in shown(form) {
            let mut found = Diags::new();
            mazer_math::check(expr, &mut found);
            for diag in found.into_vec() {
                if !pass.already_reported(&diag) {
                    diags.push(diag);
                }
            }
        }
    });

    Analysis {
        sources,
        written,
        resolved,
        macros,
        labels,
        lets: pass.lets,
        diags: dedup(diags),
    }
}

/// A prelude, read as file 1: its definitions, and any problems with it.
struct Prelude {
    text: &'static str,
    macros: Macros,
    diags: Diags,
}

impl Prelude {
    fn read(text: &'static str) -> Self {
        let mut diags = Diags::new();
        let doc = markdown::parse(text, FileId(1), &mut diags);
        let mut macros = Macros::new();
        collect_macros(&doc, &mut macros, &mut diags);
        Self { text, macros, diags }
    }
}

/// The embedded prelude, read once per process.
static PRELUDE: LazyLock<Prelude> = LazyLock::new(|| Prelude::read(STDLIB));

/// The expressions a form puts on the page (or hands to the evaluator): the
/// items of a `show` after an optional leading tag, the arguments of an
/// `eval`.
pub(crate) fn shown(form: &Node) -> &[Node] {
    if form.is_form("show") {
        match form.args() {
            [tag, rest @ ..] if tag.is_form("tag") => rest,
            all => all,
        }
    } else if form.is_form("eval") {
        form.args()
    } else {
        &[]
    }
}

/// Diagnostics without repeats: of several with the same code, span and
/// message, only the first is kept.
fn dedup(diags: Diags) -> Diags {
    let mut seen: HashSet<(mazer_syntax::DiagCode, Span, String)> = HashSet::new();
    let mut out = Diags::new();
    for diag in diags.into_vec() {
        if seen.insert((diag.code, diag.span, diag.message.clone())) {
            out.push(diag);
        }
    }
    out
}

/// The state of the resolve pass: notation, the `let` variables so far, the
/// document-wide node budget and the diagnostics.
struct Resolver<'a> {
    macros: &'a Macros,
    pass: LetPass,
    budget: Budget,
    diags: &'a mut Diags,
}

impl Resolver<'_> {
    /// Expands and substitutes one code form in place, in document order.
    fn form(&mut self, form: &mut Node) {
        if form.is_form("show") || form.is_form("eval") {
            self.show_or_eval(form);
        } else if form.is_form("let") {
            if let NodeKind::List(items) = &mut form.kind
                && let [_, _, value] = items.as_mut_slice()
            {
                *value = self.expression(value, "let");
            }
            *form = self.pass.bind(form, self.macros, &mut self.budget, self.diags);
        }
    }

    fn show_or_eval(&mut self, form: &mut Node) {
        let is_show = form.is_form("show");
        let outer = if is_show { "show" } else { "eval" };
        let count = shown(form).len();
        if !walk::contains_error(form) {
            if is_show && count == 0 {
                self.diags.push(
                    Diag::error(DiagCode::Arity, form.span, "nothing to show").with_help("write it as (show expr)"),
                );
            } else if !is_show && count != 1 {
                self.diags.push(
                    Diag::error(DiagCode::Arity, form.span, format!("`eval` takes one expression, found {count}"))
                        .with_help("write it as (eval expr)"),
                );
            }
        }
        let NodeKind::List(items) = &mut form.kind else {
            return;
        };
        let first = if is_show && items.get(1).is_some_and(|tag| tag.is_form("tag")) { 2 } else { 1 };
        for item in items.iter_mut().skip(first) {
            let mut expr = item.clone();
            walk::strip_reserved(&mut expr, outer, self.diags);
            let expr = expand_with(&expr, self.macros, &mut self.budget, self.diags);
            *item = self.pass.substitute(&expr, &mut self.budget, self.diags);
        }
        // `(show a b c)` is `(show (jux a b c))`.
        if is_show && items.len() > first + 1 {
            let exprs: Vec<Node> = items.drain(first..).collect();
            let span = match (exprs.first(), exprs.last()) {
                (Some(a), Some(b)) => a.span.to(b.span),
                _ => form.span,
            };
            let mut jux = Vec::with_capacity(exprs.len() + 1);
            jux.push(Node::ident("jux", span));
            jux.extend(exprs);
            items.push(Node::list(jux, span));
        }
    }

    /// A `let` value: nested reserved forms stripped, notation expanded.
    /// Variables are substituted by [`LetPass::bind`].
    fn expression(&mut self, value: &Node, outer: &str) -> Node {
        let mut expr = value.clone();
        walk::strip_reserved(&mut expr, outer, self.diags);
        expand_with(&expr, self.macros, &mut self.budget, self.diags)
    }
}

/// Produces output from an analysis. Evaluation (if an evaluator is
/// configured) happens here, in document order.
///
/// A broken spot renders as a visible inline error box
/// (`<span class="mazer-error">` / `<pre class="mazer-error">`); the rest of
/// the note renders normally. Problems found while rendering (for example
/// `NoEvaluator`) are pushed to `diags`.
pub fn render(analysis: &Analysis, opts: &mut Options, diags: &mut Diags) -> String {
    let body = html::body(analysis, opts.target, opts.evaluator.as_deref_mut(), diags);
    match &opts.page {
        Page::Fragment => html::fragment(&opts.name, &body),
        Page::Full { title } => html::full_page(title, &opts.name, &body),
    }
}

pub struct Output {
    pub html: String,
    pub diags: Vec<Diag>,
    /// Needed to turn a diagnostic's span into a line and column.
    pub sources: SourceMap,
}

/// [`analyze`] then [`render`]. Never fails: a broken note still returns HTML
/// with inline error boxes, plus the diagnostics.
pub fn compile(src: &str, opts: &mut Options) -> Output {
    let analysis = analyze(src, &opts.name);
    let mut extra = Diags::new();
    let html = render(&analysis, opts, &mut extra);
    let mut diags = analysis.diags.into_vec();
    diags.extend(extra.into_vec());
    Output {
        html,
        diags,
        sources: analysis.sources,
    }
}

#[cfg(test)]
mod tests;
