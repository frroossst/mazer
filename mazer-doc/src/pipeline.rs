//! The pipeline entry points: [`analyze`], [`render`] and [`compile`].
//!
//! - The CLI and the website call [`compile`]: one function in, HTML and
//!   diagnostics out.
//! - The LSP calls only [`analyze`], on every edit.
//! - A live preview calls [`render`] on an existing [`Analysis`].

use mazer_syntax::{Diag, Diags, Document, SourceMap};

use crate::bindings::Binding;
use crate::eval::Evaluator;
use crate::labels::Labels;
use crate::macros::Macros;

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
#[must_use]
pub fn analyze(src: &str, name: &str) -> Analysis {
    let _ = (src, name);
    todo!("step 2, document agent")
}

/// Produces output from an analysis. Evaluation (if an evaluator is
/// configured) happens here, in document order.
///
/// A broken spot renders as a visible inline error box
/// (`<span class="mazer-error">` / `<pre class="mazer-error">`); the rest of
/// the note renders normally. Problems found while rendering (for example
/// `NoEvaluator`) are pushed to `diags`.
pub fn render(analysis: &Analysis, opts: &mut Options, diags: &mut Diags) -> String {
    let _ = (analysis, opts, diags);
    todo!("step 2, document agent")
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
    let _ = (src, opts);
    todo!("step 2, document agent")
}
