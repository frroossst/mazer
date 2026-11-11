//! Calculus and named functions (trig, logs, abs, floor, ...).

use std::slice;

use crate::call::Call;
use crate::keyword::Keyword;
use crate::mathml::{MathMl, Prec, THIN_SPACE};

#[must_use]
pub fn keywords() -> Vec<Keyword> {
    vec![
        Keyword::new(
            "integral",
            "An integral of `body` with respect to `var`. With `lower` and `upper` it is a \
             definite integral; with only `lower`, that is written underneath, as for an \
             integral over a region.",
            integral,
        )
        .param("body", "The integrand.")
        .param("var", "The variable of integration, written after the `d`.")
        .optional("lower", "The lower bound, or the region integrated over.")
        .optional("upper", "The upper bound.")
        .sign("∫")
        .example("(integral (sin x) x)")
        .example("(integral (frac 1 x) x 1 b)")
        .example("(integral f V D)"),
        Keyword::new(
            "sum",
            "A sum of `body` over `var`, with the limits written under and over the ∑. With \
             `lower` and `upper` it runs from `var = lower` to `upper`; with only `lower`, that \
             is written underneath on its own, as a condition or set; with neither, `var` is.",
            sum,
        )
        .param("body", "The summand.")
        .param("var", "The index of summation.")
        .optional("lower", "The first value of the index, or the condition or set it ranges over.")
        .optional("upper", "The last value of the index.")
        .sign("∑")
        .example("(sum (subscript a n) n)")
        .example("(sum (frac 1 k) k 1 n)")
        .example("(sum f x S)"),
        Keyword::new("sin", "The sine function: `(sin x)` shows sin(x).", |call, out| {
            out.apply(|o| o.mi("sin"), slice::from_ref(call.req(0)));
        })
        .param("x", "The angle.")
        .sign("sin")
        .example("(sin x)")
        .example("(sin (+ x t))"),
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`.
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    match (name, args) {
        ("integral" | "sum", _) => Some(Prec::BigOp),
        _ => None,
    }
}

fn integral(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (body, var) = (call.req(0), call.req(1));
    out.row(|o| {
        match (call.opt(2), call.opt(3)) {
            (Some(lower), Some(upper)) => o.tag("msubsup", |o| {
                o.mo("∫");
                o.expr(lower);
                o.expr(upper);
            }),
            (Some(region), None) => o.tag("msub", |o| {
                o.mo("∫");
                o.expr(region);
            }),
            _ => o.mo("∫"),
        }
        o.operand(body, Prec::BigOp);
        o.space(THIN_SPACE);
        o.row(|o| {
            o.mi("d");
            o.operand(var, Prec::Atom);
        });
    });
}

/// `∑` with its limits under and over, then the body. The integral writes
/// its bounds as scripts instead (`msubsup`), as the old renderer did.
fn sum(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (body, var) = (call.req(0), call.req(1));
    out.row(|o| {
        match (call.opt(2), call.opt(3)) {
            (Some(lower), Some(upper)) => o.under_over(
                |o| o.mo("∑"),
                |o| {
                    o.row(|o| {
                        o.expr(var);
                        o.mo("=");
                        o.expr(lower);
                    });
                },
                |o| o.expr(upper),
            ),
            (Some(condition), None) => o.under(|o| o.mo("∑"), |o| o.expr(condition)),
            (None, _) => o.under(|o| o.mo("∑"), |o| o.expr(var)),
        }
        o.operand(body, Prec::BigOp);
    });
}

#[cfg(test)]
mod tests {
    use crate::testing::{mathml, render};

    #[test]
    fn integral() {
        assert_eq!(
            mathml("(integral (sin x) x)"),
            "<mrow><mo>∫</mo><mrow><mi>sin</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>\
             <mspace width=\"0.167em\"/><mrow><mi>d</mi><mi>x</mi></mrow></mrow>"
        );
        assert_eq!(
            mathml("(integral f x 0 1)"),
            "<mrow><msubsup><mo>∫</mo><mn>0</mn><mn>1</mn></msubsup><mi>f</mi>\
             <mspace width=\"0.167em\"/><mrow><mi>d</mi><mi>x</mi></mrow></mrow>"
        );
        assert_eq!(
            mathml("(integral f V D)"),
            "<mrow><msub><mo>∫</mo><mi>D</mi></msub><mi>f</mi>\
             <mspace width=\"0.167em\"/><mrow><mi>d</mi><mi>V</mi></mrow></mrow>"
        );
        // A sum as the integrand is parenthesised: ∫ (x + 1) dx.
        assert!(
            mathml("(integral (+ x 1) x)")
                .starts_with("<mrow><mo>∫</mo><mrow><mo>(</mo><mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow><mo>)</mo></mrow>")
        );
        // The bare keyword is its sign.
        assert_eq!(mathml("integral"), "<mo>∫</mo>");
        // The variable is required; the bounds are not.
        let (_, diags) = render("(integral f)");
        assert_eq!(diags.as_slice()[0].message, "`integral` is missing its `var` argument");
        let (_, diags) = render("(integral f x 0 1 2)");
        assert_eq!(diags.as_slice()[0].message, "`integral` takes 2 to 4 arguments but was given 5");
    }

    #[test]
    fn sum() {
        assert_eq!(
            mathml("(sum (subscript a n) n)"),
            "<mrow><munder><mo>∑</mo><mi>n</mi></munder><msub><mi>a</mi><mi>n</mi></msub></mrow>"
        );
        assert_eq!(
            mathml("(sum f k 1 n)"),
            "<mrow><munderover><mo>∑</mo><mrow><mi>k</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover>\
             <mi>f</mi></mrow>"
        );
        // Only `lower`: written underneath as it is.
        assert_eq!(
            mathml("(sum f x S)"),
            "<mrow><munder><mo>∑</mo><mi>S</mi></munder><mi>f</mi></mrow>"
        );
        // A sum as the body is parenthesised; a fraction or another sum is not.
        assert_eq!(
            mathml("(sum (+ a b) n)"),
            "<mrow><munder><mo>∑</mo><mi>n</mi></munder>\
             <mrow><mo>(</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(
            mathml("(sum (frac 1 k) k)"),
            "<mrow><munder><mo>∑</mo><mi>k</mi></munder><mfrac><mn>1</mn><mi>k</mi></mfrac></mrow>"
        );
        assert_eq!(
            mathml("(sum (sum a m) n)"),
            "<mrow><munder><mo>∑</mo><mi>n</mi></munder>\
             <mrow><munder><mo>∑</mo><mi>m</mi></munder><mi>a</mi></mrow></mrow>"
        );
        // As a script base it is parenthesised.
        assert_eq!(
            mathml("(subscript (sum a n) 2)"),
            "<msub><mrow><mo>(</mo><mrow><munder><mo>∑</mo><mi>n</mi></munder><mi>a</mi></mrow><mo>)</mo></mrow>\
             <mn>2</mn></msub>"
        );
        assert_eq!(mathml("sum"), "<mo>∑</mo>");
        let (_, diags) = render("(sum a)");
        assert_eq!(diags.as_slice()[0].message, "`sum` is missing its `var` argument");
    }

    #[test]
    fn sin() {
        assert_eq!(mathml("(sin x)"), "<mrow><mi>sin</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>");
        assert_eq!(
            mathml("(sin (+ x t))"),
            "<mrow><mi>sin</mi><mrow><mo>(</mo><mrow><mi>x</mi><mo>+</mo><mi>t</mi></mrow><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(mathml("sin"), "<mo>sin</mo>");
    }
}
