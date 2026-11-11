//! The LaTeX emitter. Only compiled with the `latex` feature.
//!
//! Mirrors [`crate::MathMl`]'s method names where that makes sense, so writing
//! the second emitter for a keyword has the same shape as the first.

use mazer_syntax::{Diag, Diags, Node};

use crate::call::Call;
use crate::keyword::Keyword;

/// A keyword's LaTeX emitter.
pub type LatexFn = fn(&Call<'_>, &mut Latex<'_>);

pub struct Latex<'a> {
    pub(crate) out: String,
    pub(crate) diags: &'a mut Diags,
}

impl<'a> Latex<'a> {
    #[must_use]
    pub const fn new(diags: &'a mut Diags) -> Self {
        Self {
            out: String::new(),
            diags,
        }
    }

    #[must_use]
    pub fn finish(self) -> String {
        self.out
    }

    /// Renders any node. Same dispatch rules as [`crate::MathMl::expr`]. A
    /// keyword with no LaTeX emitter pushes an `UnsupportedTarget` diagnostic.
    pub fn expr(&mut self, node: &Node) {
        let _ = node;
        todo!("step 3, latex agent")
    }

    /// Writes raw LaTeX. The caller is responsible for it being valid.
    pub fn raw(&mut self, latex: &str) {
        let _ = latex;
        todo!("step 3, latex agent")
    }

    /// Writes text with LaTeX special characters escaped.
    pub fn text(&mut self, text: &str) {
        let _ = text;
        todo!("step 3, latex agent")
    }

    /// `{ ... }`
    pub fn group(&mut self, body: impl FnOnce(&mut Self)) {
        let _ = body;
        todo!("step 3, latex agent")
    }

    /// `\frac{num}{den}`
    pub fn frac(&mut self, num: &Node, den: &Node) {
        let _ = (num, den);
        todo!("step 3, latex agent")
    }

    /// `base_{script}`
    pub fn sub(&mut self, base: &Node, script: &Node) {
        let _ = (base, script);
        todo!("step 3, latex agent")
    }

    /// `base^{script}`
    pub fn sup(&mut self, base: &Node, script: &Node) {
        let _ = (base, script);
        todo!("step 3, latex agent")
    }

    /// `\left<open> ... \right<close>`. Either delimiter may be `""` (emitted as `.`).
    pub fn fence(&mut self, open: &str, close: &str, body: impl FnOnce(&mut Self)) {
        let _ = (open, close, body);
        todo!("step 3, latex agent")
    }

    /// Pushes the diagnostic and writes a visible marker in the output.
    #[allow(clippy::needless_pass_by_value)] // contract: takes `Diag` by value; stub does not consume it yet
    pub fn error(&mut self, diag: Diag) {
        let _ = diag;
        todo!("step 3, latex agent")
    }
}

/// Renders one expression to LaTeX math (no surrounding `$` or environment).
pub fn to_latex(node: &Node, diags: &mut Diags) -> String {
    let mut out = Latex::new(diags);
    out.expr(node);
    out.finish()
}

/// Attaches LaTeX emitters to keywords by name. Keyword definitions themselves
/// carry no LaTeX code; it all lives behind this function.
#[allow(clippy::missing_const_for_fn)] // skeleton placeholder: stub body is empty
pub fn attach(keywords: &mut [Keyword]) {
    // step 3, latex agent
    let _ = keywords;
}
