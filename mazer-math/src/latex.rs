//! The LaTeX emitter. Only compiled with the `latex` feature.
//!
//! Mirrors [`crate::MathMl`]'s method names where that makes sense, so writing
//! the second emitter for a keyword has the same shape as the first. The
//! dispatch rules, the bare-keyword rule, the function-application rule, the
//! did-you-mean hint and the parenthesis rules are the `MathML` ones: both
//! targets share the [`Prec`] levels declared in the keyword files.
//!
//! The output is LaTeX math (amsmath and amssymb are assumed), with no
//! surrounding `$` or environment. Conventions:
//!
//! - Operators are written with a space on each side (`a + b`, `x \leq y`).
//! - Sub- and superscripts and the parts of a fraction are always braced
//!   (`x^{2}`, `\frac{a}{b}`).
//! - Every control word written through [`Latex::raw`] is separated from a
//!   following letter, so output never fuses (`\int f`, not `\intf`).
//! - Parentheses are `\left( ... \right)`.
//! - Keyword definitions carry no LaTeX code. The emitters live in the files
//!   below and are attached by name in [`attach`].

mod algebra;
mod calculus;
mod notation;
mod signs;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Span, Sym};

use crate::call::{Call, try_bind};
use crate::display::display_name;
use crate::keyword::Keyword;
use crate::mathml::{Assoc, Prec, chain_needs_parens, leads_with_sign, precedence};
use crate::registry::registry;

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
    ///
    /// - `Number` -> as written (a negative number keeps its `-`)
    /// - `Str` -> `\text{...}`, escaped
    /// - `Ident` -> a bare keyword's sign, else the display table, else the name
    /// - a list headed by a keyword -> bind the arguments, then that keyword's
    ///   LaTeX emitter
    /// - a list headed by anything else -> function application `f(x, y)`
    /// - `Error` -> `\text{[error]}` (the reader already reported it)
    /// - `()` -> `{}`
    pub fn expr(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Number(text) => self.raw(text),
            NodeKind::Str(text) => self.text_run(text),
            NodeKind::Ident(sym) => self.ident(*sym, node.span),
            NodeKind::List(items) => self.list(node, items),
            NodeKind::Error => self.marker("error", None),
        }
    }

    /// Writes raw LaTeX. The caller is responsible for it being valid. A space
    /// is inserted first if the output ends in a control word and `latex`
    /// starts with a letter, so the two never fuse.
    pub fn raw(&mut self, latex: &str) {
        if latex.starts_with(|c: char| c.is_ascii_alphabetic()) && ends_in_control_word(&self.out) {
            self.out.push(' ');
        }
        self.out.push_str(latex);
    }

    /// Writes text with LaTeX special characters escaped (for use inside
    /// `\text{...}`).
    pub fn text(&mut self, text: &str) {
        let escaped = signs::escape_text(text);
        self.raw(&escaped);
    }

    /// `{ ... }`
    pub fn group(&mut self, body: impl FnOnce(&mut Self)) {
        self.out.push('{');
        body(self);
        self.out.push('}');
    }

    /// `\frac{num}{den}`
    pub fn frac(&mut self, num: &Node, den: &Node) {
        self.raw("\\frac");
        self.group(|o| o.expr(num));
        self.group(|o| o.expr(den));
    }

    /// `base_{script}`. A base that is not an atom (`a + b`, `-x`, `x^{2}`,
    /// `n!`, `\int f`) is put in parentheses first; a base that already ends
    /// in a script is braced so the two never stack.
    pub fn sub(&mut self, base: &Node, script: &Node) {
        self.script_base(base, b'_');
        self.out.push('_');
        self.group(|o| o.expr(script));
    }

    /// `base^{script}`; the base is treated as in [`Latex::sub`].
    pub fn sup(&mut self, base: &Node, script: &Node) {
        self.script_base(base, b'^');
        self.out.push('^');
        self.group(|o| o.expr(script));
    }

    /// `\left<open> ... \right<close>`. Either delimiter may be `""` (emitted as `.`).
    /// A delimiter is given as the character (`(`, `{`, `⌊`) or as LaTeX.
    pub fn fence(&mut self, open: &str, close: &str, body: impl FnOnce(&mut Self)) {
        self.raw("\\left");
        self.raw(&signs::delimiter(open));
        body(self);
        self.raw("\\right");
        self.raw(&signs::delimiter(close));
    }

    /// Pushes the diagnostic and writes a visible marker in the output.
    pub fn error(&mut self, diag: Diag) {
        self.marker("error", Some(&diag.message));
        self.diags.push(diag);
    }
}

/// Helpers for keyword emitters. Crate-internal, like the `MathMl` ones.
impl Latex<'_> {
    /// Pushes a diagnostic without writing anything.
    pub(crate) fn report(&mut self, diag: Diag) {
        self.diags.push(diag);
    }

    /// A space, unless the output is empty or already ends in one.
    pub(crate) fn space(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with(' ') {
            self.out.push(' ');
        }
    }

    /// `\text{...}` with the text escaped.
    pub(crate) fn text_run(&mut self, text: &str) {
        self.raw("\\text{");
        self.text(text);
        self.out.push('}');
    }

    /// `\begin{env} ... \end{env}`; an empty body gives no inner spaces.
    pub(crate) fn environment(&mut self, env: &str, body: impl FnOnce(&mut Self)) {
        self.raw("\\begin{");
        self.out.push_str(env);
        self.out.push('}');
        let inner = self.capture(body);
        if !inner.is_empty() {
            self.out.push(' ');
            self.out.push_str(&inner);
            self.out.push(' ');
        }
        self.out.push_str("\\end{");
        self.out.push_str(env);
        self.out.push('}');
    }

    /// Runs `f` and returns what it wrote, leaving the output as it was.
    pub(crate) fn capture(&mut self, f: impl FnOnce(&mut Self)) -> String {
        let saved = std::mem::take(&mut self.out);
        f(self);
        std::mem::replace(&mut self.out, saved)
    }

    /// Renders `node`, in parentheses if it holds together more loosely than
    /// `min` (see [`Prec`]), or if `min` is [`Prec::Prefix`] or tighter and
    /// `node` starts with a sign (`-(-x)`, `\sum (-a)`).
    pub(crate) fn operand(&mut self, node: &Node, min: Prec) {
        let parens = precedence(node) < min || (min >= Prec::Prefix && leads_with_sign(node));
        self.parenthesised_if(parens, node);
    }

    /// An infix row `a op b op c` at level `prec`; `op` is LaTeX
    /// (`\times`, `=`). Parenthesised exactly as [`crate::MathMl`] does.
    pub(crate) fn infix(&mut self, op: &str, prec: Prec, assoc: Assoc, operands: &[Node]) {
        let operands: Vec<&Node> = operands.iter().collect();
        self.chain(prec, assoc, &operands, op);
    }

    /// [`Latex::infix`] with exactly two operands.
    pub(crate) fn binary(&mut self, op: &str, prec: Prec, assoc: Assoc, left: &Node, right: &Node) {
        self.chain(prec, assoc, &[left, right], op);
    }

    /// Juxtaposition: the operands side by side with a thin space between
    /// them, as an associative [`Prec::Multiplicative`] row.
    pub(crate) fn juxtapose(&mut self, operands: &[Node]) {
        let operands: Vec<&Node> = operands.iter().collect();
        self.chain(Prec::Multiplicative, Assoc::Both, &operands, "\\,");
    }

    /// A prefix operator followed by its operand, rendered with
    /// [`Latex::operand`] at `min`.
    pub(crate) fn prefix(&mut self, op: &str, operand: &Node, min: Prec) {
        self.raw(op);
        self.operand(operand, min);
    }

    /// A postfix operator: the operand is parenthesised unless it is an atom.
    pub(crate) fn postfix(&mut self, op: &str, operand: &Node) {
        self.operand(operand, Prec::Atom);
        self.raw(op);
    }

    /// A quantifier: `\forall x.\, body`. The body is never parenthesised.
    pub(crate) fn binder(&mut self, sign: &str, var: &Node, body: &Node) {
        self.raw(sign);
        self.expr(var);
        self.raw(".\\,");
        self.space();
        self.expr(body);
    }

    /// Function application: `head(a, b, c)`. With no arguments only the head
    /// is written.
    pub(crate) fn apply(&mut self, head: impl FnOnce(&mut Self), args: &[Node]) {
        head(self);
        if !args.is_empty() {
            self.fence("(", ")", |o| o.separated(",", args));
        }
    }

    /// Function application with an expression as the function; the head is
    /// parenthesised if it is looser than a power.
    pub(crate) fn application(&mut self, head: &Node, args: &[Node]) {
        if args.is_empty() {
            self.expr(head);
        } else {
            self.apply(|o| o.operand(head, Prec::Power), args);
        }
    }

    /// The items with `sep` and a space between them.
    pub(crate) fn separated(&mut self, sep: &str, items: &[Node]) {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.raw(sep);
                self.out.push(' ');
            }
            self.expr(item);
        }
    }

    /// A variable name: its display form if it has one (`tau` -> `\tau`), a
    /// single letter as it is, a longer name upright. Never treats the name as
    /// a keyword.
    pub(crate) fn name(&mut self, name: Sym) {
        if let Some(display) = display_name(name)
            && !display.latex.is_empty()
        {
            let latex = signs::display_latex(display.latex);
            self.raw(&latex);
            return;
        }
        // A character that is a display-table glyph (typed `α`, `→`, `⊢`) is
        // spelled as its command; any other non-ASCII character is wrapped in
        // `\text`, so nothing outside `\text` is raw Unicode.
        let text = name.as_str();
        let spelled = signs::spell(text);
        if text.chars().count() > 1 && text.chars().any(char::is_alphanumeric) {
            self.raw("\\mathrm{");
            self.out.push_str(&spelled);
            self.out.push('}');
        } else {
            self.raw(&spelled);
        }
    }

    /// `_{script}`, braced.
    pub(crate) fn subscript(&mut self, script: &Node) {
        self.out.push('_');
        self.group(|o| o.expr(script));
    }

    /// `^{script}`, braced.
    pub(crate) fn superscript(&mut self, script: &Node) {
        self.out.push('^');
        self.group(|o| o.expr(script));
    }
}

/// The dispatcher's pieces.
impl Latex<'_> {
    fn ident(&mut self, name: Sym, span: Span) {
        match registry().get(name) {
            Some(keyword) => self.bare(keyword, name, span),
            None => self.name(name),
        }
    }

    /// A keyword with no arguments: its sign if it has one, else the
    /// `BareKeyword` diagnostic.
    fn bare(&mut self, keyword: &Keyword, name: Sym, span: Span) {
        if let Some(sign) = keyword.sign {
            match signs::sign(sign) {
                Some(latex) => self.raw(latex),
                None => self.text_run(sign),
            }
        } else {
            self.error(
                Diag::error(DiagCode::BareKeyword, span, format!("`{name}` needs arguments"))
                    .with_help(format!("write it as {}", keyword.signature())),
            );
        }
    }

    fn list(&mut self, node: &Node, items: &[Node]) {
        let Some((head, args)) = items.split_first() else {
            // `()`: an empty placeholder, e.g. an argument left blank on purpose.
            self.out.push_str("{}");
            return;
        };
        if let Some(name) = head.as_ident() {
            if let Some(keyword) = registry().get(name) {
                // `(neq)` is `neq`; `(set)` and `(mat)` run their emitters.
                if args.is_empty() && (keyword.sign.is_some() || keyword.min_args() > 0) {
                    self.bare(keyword, name, node.span);
                    return;
                }
                match try_bind(keyword, node) {
                    Ok(call) => match keyword.latex {
                        Some(emit) => {
                            crate::lint::call_warnings(&call, self.diags);
                            emit(&call, self);
                        }
                        None => self.error(Diag::error(
                            DiagCode::UnsupportedTarget,
                            node.span,
                            format!("`{}` has no LaTeX output yet", keyword.name),
                        )),
                    },
                    Err(diag) => self.error(diag),
                }
                return;
            }
            crate::lint::unknown_head(name, head, self.diags);
        }
        self.application(head, args);
    }

    /// `\text{[label]}` or `\text{[label: message]}`, without a diagnostic.
    fn marker(&mut self, label: &str, message: Option<&str>) {
        let mut text = format!("[{label}");
        if let Some(message) = message {
            text.push_str(": ");
            text.push_str(&message.replace('`', "'"));
        }
        text.push(']');
        self.text_run(&text);
    }

    /// The operands of [`Latex::infix`] / [`Latex::juxtapose`], with `op`
    /// written between them.
    fn chain(&mut self, prec: Prec, assoc: Assoc, operands: &[&Node], op: &str) {
        if operands.is_empty() {
            self.out.push_str("{}");
        }
        for (i, operand) in operands.iter().enumerate() {
            if i > 0 {
                self.out.push(' ');
                self.raw(op);
                self.out.push(' ');
            }
            let parens = chain_needs_parens(operand, i, operands.len(), prec, assoc);
            self.parenthesised_if(parens, operand);
        }
    }

    fn parenthesised_if(&mut self, parens: bool, node: &Node) {
        if parens {
            self.fence("(", ")", |o| o.expr(node));
        } else {
            self.expr(node);
        }
    }

    /// The base of a script: an atom, braced if it already ends in a script
    /// of the same kind (`mark` is `_` or `^`), which TeX rejects.
    fn script_base(&mut self, base: &Node, mark: u8) {
        self.braced_operand(base, Prec::Atom, mark);
    }

    /// [`Latex::operand`], braced if what the caller writes next would make a
    /// double script. `mark` is what follows: `_` (a subscript), `^` (a
    /// superscript, which also clashes with a trailing prime: `f'^{2}`) or `'`
    /// (a prime, which clashes with a trailing superscript, `f^{2}'`, but not
    /// with another prime: `f''`).
    pub(crate) fn braced_operand(&mut self, node: &Node, min: Prec, mark: u8) {
        let rendered = self.capture(|o| o.operand(node, min));
        let clash = match mark {
            b'\'' => ends_in_script(&rendered, b'^'),
            b'^' => ends_in_script(&rendered, b'^') || rendered.ends_with('\''),
            _ => ends_in_script(&rendered, mark),
        };
        if clash {
            self.out.push('{');
            self.out.push_str(&rendered);
            self.out.push('}');
        } else {
            self.raw(&rendered);
        }
    }
}

/// Whether `s` ends in a control word (a backslash and letters), which would
/// swallow a letter written right after it.
fn ends_in_control_word(s: &str) -> bool {
    let bytes = s.as_bytes();
    let letters = bytes.iter().rev().take_while(|b| b.is_ascii_alphabetic()).count();
    if letters == 0 {
        return false;
    }
    let slashes = bytes[..bytes.len() - letters]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count();
    slashes % 2 == 1
}

/// Whether `s` ends in a `_{...}` group (`mark` is `_`) or a `^{...}` group
/// (`mark` is `^`).
fn ends_in_script(s: &str, mark: u8) -> bool {
    let bytes = s.as_bytes();
    if bytes.last() != Some(&b'}') {
        return false;
    }
    let escaped = |i: usize| bytes[..i].iter().rev().take_while(|&&b| b == b'\\').count() % 2 == 1;
    let mut depth = 0usize;
    for i in (0..bytes.len()).rev() {
        match bytes[i] {
            b'}' if !escaped(i) => depth += 1,
            b'{' if !escaped(i) => {
                let Some(open) = depth.checked_sub(1) else {
                    return false;
                };
                depth = open;
                if depth == 0 {
                    return i > 0 && bytes[i - 1] == mark;
                }
            }
            _ => {}
        }
    }
    false
}

/// Renders one expression to LaTeX math (no surrounding `$` or environment).
pub fn to_latex(node: &Node, diags: &mut Diags) -> String {
    let mut out = Latex::new(diags);
    out.expr(node);
    out.finish()
}

/// Attaches LaTeX emitters to keywords by name. Keyword definitions themselves
/// carry no LaTeX code; it all lives behind this function.
pub fn attach(keywords: &mut [Keyword]) {
    let table: HashMap<&str, LatexFn> = algebra::emitters()
        .into_iter()
        .chain(calculus::emitters())
        .chain(notation::emitters())
        .collect();
    for keyword in keywords {
        if let Some(&emit) = table.get(keyword.name) {
            keyword.latex = Some(emit);
        }
    }
}
