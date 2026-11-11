//! The `MathML` emitter. Writes straight into one `String`; there is no
//! intermediate element tree.
//!
//! Text is escaped in exactly four places: [`MathMl::mi`], [`MathMl::mn`],
//! [`MathMl::mo`] and [`MathMl::mtext`]. Emitters never build a tag by hand.

use mazer_syntax::{Diag, Diags, Node};

pub struct MathMl<'a> {
    pub(crate) out: String,
    pub(crate) diags: &'a mut Diags,
}

impl<'a> MathMl<'a> {
    #[must_use]
    pub const fn new(diags: &'a mut Diags) -> Self {
        Self {
            out: String::new(),
            diags,
        }
    }

    /// The `MathML` written so far (no surrounding `<math>` element).
    #[must_use]
    pub fn finish(self) -> String {
        self.out
    }

    /// Renders any node. This is the recursive dispatcher:
    /// - `Number` -> `<mn>`
    /// - `Ident` -> a bare keyword's sign, else the display table, else the name itself
    /// - `Str` -> `<mtext>`
    /// - a list headed by a keyword -> bind the arguments, then that keyword's emitter
    /// - a list headed by anything else -> function application: `(f x y)` renders
    ///   as f(x, y); with no arguments, `(Gamma)` renders just the head. If the
    ///   head is one typo away from a keyword, also push an `UnknownForm` warning
    ///   with a did-you-mean.
    /// - `Error` -> `<merror>` (the reader already reported it)
    pub fn expr(&mut self, node: &Node) {
        let _ = node;
        todo!("step 1, math core agent")
    }

    /// Writes `<name>`, runs `body`, writes `</name>`. The closure guarantees
    /// the closing tag.
    pub fn tag(&mut self, name: &'static str, body: impl FnOnce(&mut Self)) {
        let _ = (name, body);
        todo!("step 1, math core agent")
    }

    /// Like [`MathMl::tag`] with attributes. Attribute values are escaped.
    pub fn tag_attrs(&mut self, name: &'static str, attrs: &[(&str, &str)], body: impl FnOnce(&mut Self)) {
        let _ = (name, attrs, body);
        todo!("step 1, math core agent")
    }

    /// An identifier: `<mi>text</mi>`, escaped.
    pub fn mi(&mut self, text: &str) {
        let _ = text;
        todo!("step 1, math core agent")
    }

    /// A number: `<mn>text</mn>`, escaped.
    pub fn mn(&mut self, text: &str) {
        let _ = text;
        todo!("step 1, math core agent")
    }

    /// An operator: `<mo>text</mo>`, escaped.
    pub fn mo(&mut self, text: &str) {
        let _ = text;
        todo!("step 1, math core agent")
    }

    /// Plain text: `<mtext>text</mtext>`, escaped.
    pub fn mtext(&mut self, text: &str) {
        let _ = text;
        todo!("step 1, math core agent")
    }

    /// `<mrow> ... </mrow>`
    pub fn row(&mut self, body: impl FnOnce(&mut Self)) {
        let _ = body;
        todo!("step 1, math core agent")
    }

    /// `<mfrac>num den</mfrac>`
    pub fn frac(&mut self, num: &Node, den: &Node) {
        let _ = (num, den);
        todo!("step 1, math core agent")
    }

    /// `<msub>base script</msub>`
    pub fn sub(&mut self, base: &Node, script: &Node) {
        let _ = (base, script);
        todo!("step 1, math core agent")
    }

    /// `<msup>base script</msup>`
    pub fn sup(&mut self, base: &Node, script: &Node) {
        let _ = (base, script);
        todo!("step 1, math core agent")
    }

    /// A row wrapped in stretchy delimiters. Either delimiter may be `""`.
    pub fn fence(&mut self, open: &str, close: &str, body: impl FnOnce(&mut Self)) {
        let _ = (open, close, body);
        todo!("step 1, math core agent")
    }

    /// Reports a problem: pushes the diagnostic and writes a visible `<merror>`
    /// in its place so the rest of the expression still renders.
    #[allow(clippy::needless_pass_by_value)] // contract: takes `Diag` by value; stub does not consume it yet
    pub fn error(&mut self, diag: Diag) {
        let _ = diag;
        todo!("step 1, math core agent")
    }
}

/// Renders one expression to `MathML` (no surrounding `<math>` element).
pub fn to_mathml(node: &Node, diags: &mut Diags) -> String {
    let mut out = MathMl::new(diags);
    out.expr(node);
    out.finish()
}

/// Checks one expression (keyword argument counts, bare keywords, typo'd
/// heads) and pushes diagnostics, without keeping any output. Used by
/// `mazer_doc::analyze`, which the LSP calls on every edit.
pub fn check(node: &Node, diags: &mut Diags) {
    let _ = (node, diags);
    todo!("step 1, math core agent")
}
