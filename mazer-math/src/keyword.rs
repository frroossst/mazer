//! How a keyword such as `integral` or `frac` is defined.
//!
//! A keyword is one value built with a builder. The same value drives
//! rendering, argument checking, LSP completion / hover / signature help and
//! `mazer query`.
//!
//! ```ignore
//! Keyword::new("frac", "A fraction: numerator over denominator.", |call, out| {
//!         out.frac(call.req(0), call.req(1));
//!     })
//!     .param("num", "Numerator")
//!     .param("den", "Denominator")
//!     .example("(frac 1 2)")
//! ```

use crate::call::Call;
use crate::mathml::MathMl;

/// A keyword's `MathML` emitter. It writes into `MathMl` and reports problems
/// through it; it never returns an error.
pub type MathMlFn = fn(&Call<'_>, &mut MathMl<'_>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParamKind {
    Required,
    /// May be omitted. Optional parameters come after all required ones.
    Optional,
    /// Zero or more trailing arguments. Must be the last parameter.
    Rest,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Param {
    pub name: &'static str,
    pub doc: &'static str,
    pub kind: ParamKind,
}

#[derive(Clone, Debug)]
pub struct Keyword {
    pub name: &'static str,
    /// Second names, e.g. `add` for `+`, `limit` for `lim`.
    pub aliases: Vec<&'static str>,
    /// One or two sentences. Shown on LSP hover and by `mazer query`.
    pub doc: &'static str,
    pub params: Vec<Param>,
    /// Source snippets such as `"(frac 1 2)"`. Every one is rendered in a test.
    pub examples: Vec<&'static str>,
    /// What the bare keyword (no parentheses, no arguments) renders as, e.g.
    /// `∫` for `integral`. `None` means bare use is a diagnostic.
    pub sign: Option<&'static str>,
    pub mathml: MathMlFn,
    #[cfg(feature = "latex")]
    pub latex: Option<crate::latex::LatexFn>,
}

impl Keyword {
    /// The three things every keyword must have: a name, a doc line and a
    /// `MathML` emitter.
    #[must_use]
    pub const fn new(name: &'static str, doc: &'static str, mathml: MathMlFn) -> Self {
        Self {
            name,
            aliases: Vec::new(),
            doc,
            params: Vec::new(),
            examples: Vec::new(),
            sign: None,
            mathml,
            #[cfg(feature = "latex")]
            latex: None,
        }
    }

    /// Adds a second name the keyword can be called by.
    #[must_use]
    pub fn alias(mut self, name: &'static str) -> Self {
        self.aliases.push(name);
        self
    }

    /// Adds a required parameter.
    #[must_use]
    pub fn param(mut self, name: &'static str, doc: &'static str) -> Self {
        self.params.push(Param {
            name,
            doc,
            kind: ParamKind::Required,
        });
        self
    }

    /// Adds an optional parameter. Must come after all required parameters.
    #[must_use]
    pub fn optional(mut self, name: &'static str, doc: &'static str) -> Self {
        self.params.push(Param {
            name,
            doc,
            kind: ParamKind::Optional,
        });
        self
    }

    /// Adds a parameter collecting zero or more trailing arguments. Must be last.
    #[must_use]
    pub fn rest(mut self, name: &'static str, doc: &'static str) -> Self {
        self.params.push(Param {
            name,
            doc,
            kind: ParamKind::Rest,
        });
        self
    }

    #[must_use]
    pub fn example(mut self, src: &'static str) -> Self {
        self.examples.push(src);
        self
    }

    /// Sets what the bare keyword renders as.
    #[must_use]
    pub const fn sign(mut self, sign: &'static str) -> Self {
        self.sign = Some(sign);
        self
    }

    /// Every name this keyword answers to: its primary name, then its aliases.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.name).chain(self.aliases.iter().copied())
    }

    /// The signature as shown to the user, e.g. `(integral body var [lower] [upper])`.
    #[must_use]
    pub fn signature(&self) -> String {
        let mut out = format!("({}", self.name);
        for p in &self.params {
            out.push(' ');
            match p.kind {
                ParamKind::Required => out.push_str(p.name),
                ParamKind::Optional => {
                    out.push('[');
                    out.push_str(p.name);
                    out.push(']');
                }
                ParamKind::Rest => {
                    out.push_str(p.name);
                    out.push_str("...");
                }
            }
        }
        out.push(')');
        out
    }

    /// Fewest arguments a call may have.
    #[must_use]
    pub fn min_args(&self) -> usize {
        self.params
            .iter()
            .filter(|p| p.kind == ParamKind::Required)
            .count()
    }

    /// Most arguments a call may have, or `None` if there is a `Rest` parameter.
    #[must_use]
    pub fn max_args(&self) -> Option<usize> {
        if self.params.iter().any(|p| p.kind == ParamKind::Rest) {
            None
        } else {
            Some(self.params.len())
        }
    }
}

#[cfg(feature = "latex")]
impl Keyword {
    /// Attaches a LaTeX emitter. Only exists with the `latex` feature; LaTeX
    /// emitters are attached in `latex.rs`, not in the keyword definitions.
    #[must_use]
    pub fn latex(mut self, f: crate::latex::LatexFn) -> Self {
        self.latex = Some(f);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_: &Call<'_>, _: &mut MathMl<'_>) {}

    #[test]
    fn builder_and_signature() {
        let kw = Keyword::new("integral", "An integral.", noop)
            .param("body", "The integrand")
            .param("var", "Variable of integration")
            .optional("lower", "Lower bound")
            .optional("upper", "Upper bound")
            .sign("∫")
            .example("(integral (pow x 2) x)");
        assert_eq!(kw.signature(), "(integral body var [lower] [upper])");
        assert_eq!(kw.min_args(), 2);
        assert_eq!(kw.max_args(), Some(4));
        assert_eq!(kw.sign, Some("∫"));

        let add = Keyword::new("+", "Addition.", noop).alias("add").rest("terms", "Terms");
        assert_eq!(add.names().collect::<Vec<_>>(), ["+", "add"]);
        assert_eq!(add.max_args(), None);
    }
}
