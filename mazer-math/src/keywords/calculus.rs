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
#[allow(clippy::missing_const_for_fn, clippy::unnecessary_wraps)] // step 2 adds non-const string matches here
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    let _ = (name, args);
    None
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
        o.operand(body, Prec::Multiplicative);
        o.space(THIN_SPACE);
        o.row(|o| {
            o.mi("d");
            o.operand(var, Prec::Atom);
        });
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
    fn sin() {
        assert_eq!(mathml("(sin x)"), "<mrow><mi>sin</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>");
        assert_eq!(
            mathml("(sin (+ x t))"),
            "<mrow><mi>sin</mi><mrow><mo>(</mo><mrow><mi>x</mi><mo>+</mo><mi>t</mi></mrow><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(mathml("sin"), "<mo>sin</mo>");
    }
}
