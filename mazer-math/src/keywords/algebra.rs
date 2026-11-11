//! Arithmetic, comparison, sets, logic, grouping.

use crate::call::Call;
use crate::keyword::Keyword;
use crate::mathml::{Assoc, MathMl, Prec};

#[must_use]
pub fn keywords() -> Vec<Keyword> {
    vec![
        Keyword::new(
            "+",
            "Addition: the terms joined by `+`. With one term it is a unary plus (`+x`); \
             with none it is the `+` sign itself.",
            add,
        )
        .alias("add")
        .rest("terms", "The terms to add, left to right.")
        .sign("+")
        .example("(+ a b)")
        .example("(+ 1 x (frac y 2))")
        .example("(add a b c)"),
        Keyword::new("frac", "A fraction: the numerator stacked over the denominator.", |call, out| {
            out.frac(call.req(0), call.req(1));
        })
        .param("num", "The numerator, written on top.")
        .param("den", "The denominator, written underneath.")
        .example("(frac 1 2)")
        .example("(frac (+ a b) c)"),
        Keyword::new("paren", "Parentheses around an expression; they grow with its height.", |call, out| {
            let body = call.req(0);
            out.fence("(", ")", |o| o.expr(body));
        })
        .param("body", "The expression inside the parentheses.")
        .example("(paren x)")
        .example("(paren (frac 1 2))"),
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`.
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    match (name, args) {
        // `(+)` is just the sign: an atom.
        ("+", 1) => Some(Prec::Prefix),
        ("+", 2..) => Some(Prec::Additive),
        ("frac", _) => Some(Prec::Fraction),
        _ => None,
    }
}

fn add(call: &Call<'_>, out: &mut MathMl<'_>) {
    match call.rest() {
        [] => out.mo("+"),
        [term] => out.prefix("+", term, Prec::BigOp),
        terms => out.infix("+", Prec::Additive, Assoc::Both, terms),
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::{mathml, render};

    #[test]
    fn add() {
        assert_eq!(mathml("(+ a b)"), "<mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow>");
        assert_eq!(
            mathml("(add 1 x 2)"),
            "<mrow><mn>1</mn><mo>+</mo><mi>x</mi><mo>+</mo><mn>2</mn></mrow>"
        );
        // Unary plus and the bare sign.
        assert_eq!(mathml("(+ x)"), "<mrow><mo>+</mo><mi>x</mi></mrow>");
        assert_eq!(mathml("(+)"), "<mo>+</mo>");
        assert_eq!(mathml("+"), "<mo>+</mo>");
        // Sums nest without parentheses (addition is associative) ...
        assert_eq!(
            mathml("(+ a (+ b c))"),
            "<mrow><mi>a</mi><mo>+</mo><mrow><mi>b</mi><mo>+</mo><mi>c</mi></mrow></mrow>"
        );
        // ... but a sum under a unary plus needs them.
        assert_eq!(
            mathml("(+ (+ a b))"),
            "<mrow><mo>+</mo><mrow><mo>(</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>)</mo></mrow></mrow>"
        );
        // A negative number after `+` is parenthesised so that two signs
        // never touch; at the start it needs none.
        assert_eq!(
            mathml("(+ a -2)"),
            "<mrow><mi>a</mi><mo>+</mo><mrow><mo>(</mo><mrow><mo>\u{2212}</mo><mn>2</mn></mrow><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(
            mathml("(+ -2 a)"),
            "<mrow><mrow><mo>\u{2212}</mo><mn>2</mn></mrow><mo>+</mo><mi>a</mi></mrow>"
        );
        // Unary plus of something signed.
        assert_eq!(
            mathml("(+ -2)"),
            "<mrow><mo>+</mo><mrow><mo>(</mo><mrow><mo>\u{2212}</mo><mn>2</mn></mrow><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(
            mathml("(+ (+ a))"),
            "<mrow><mo>+</mo><mrow><mo>(</mo><mrow><mo>+</mo><mi>a</mi></mrow><mo>)</mo></mrow></mrow>"
        );
    }

    #[test]
    fn frac() {
        assert_eq!(mathml("(frac 1 2)"), "<mfrac><mn>1</mn><mn>2</mn></mfrac>");
        assert_eq!(
            mathml("(frac (+ a b) c)"),
            "<mfrac><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mi>c</mi></mfrac>"
        );
        let (out, diags) = render("(frac 1)");
        assert_eq!(out, "<merror><mtext>`frac` is missing its `den` argument</mtext></merror>");
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn paren() {
        assert_eq!(mathml("(paren x)"), "<mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow>");
        assert_eq!(
            mathml("(paren (frac 1 2))"),
            "<mrow><mo>(</mo><mfrac><mn>1</mn><mn>2</mn></mfrac><mo>)</mo></mrow>"
        );
        // The empty placeholder `()` keeps the fence well formed.
        assert_eq!(mathml("(paren ())"), "<mrow><mo>(</mo><mrow></mrow><mo>)</mo></mrow>");
    }
}
