//! Calculus and named functions (trig, logs, abs, floor, ...).

use std::slice;

use mazer_syntax::Node;

use crate::call::Call;
use crate::keyword::Keyword;
use crate::mathml::{MathMl, Prec, THIN_SPACE};

#[must_use]
#[allow(clippy::too_many_lines)] // one builder chain per keyword
pub fn keywords() -> Vec<Keyword> {
    vec![
        Keyword::new(
            "integral",
            "An integral of `body` with respect to `var`. With `lower` and `upper` it is a \
             definite integral; with only `lower`, that is written as a subscript on the \
             ∫, as for an integral over a region.",
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
        Keyword::new(
            "prod",
            "A product of `body` over `var`, with the limits written under and over the ∏. With \
             `lower` and `upper` it runs from `var = lower` to `upper`; with only `lower`, that \
             is written underneath on its own; with neither, `var` is.",
            prod,
        )
        .alias("product")
        .param("body", "The factor being multiplied.")
        .param("var", "The index of the product.")
        .optional("lower", "The first value of the index, or the condition or set it ranges over.")
        .optional("upper", "The last value of the index.")
        .sign("∏")
        .example("(prod (subscript a n) n)")
        .example("(prod (frac 1 k) k 1 n)")
        .example("(product f x S)"),
        Keyword::new(
            "lim",
            "A limit: `var` tends to `approach`, written under lim, followed by `body`. Without a \
             `body` only the lim and its subscript are shown. A last argument `side` is written as \
             a superscript on `approach` (a one-sided limit); an empty `()` there shows nothing.",
            lim,
        )
        .alias("limit")
        .param("var", "The variable that tends to the value.")
        .param("approach", "The value it tends to.")
        .optional("body", "The expression whose limit is taken.")
        .optional("side", "The side it is approached from, e.g. `+`; `()` shows nothing.")
        .sign("lim")
        .example("(lim x 0 (frac (sin x) x))")
        .example("(limit n a (subscript a n) ())")
        .example("(lim x 0 f +)"),
        Keyword::new(
            "derivative",
            "The derivative operator d/d`var` applied to `body`. With `order` it is the nth \
             derivative, dⁿ/d`var`ⁿ.",
            derivative,
        )
        .alias("deriv")
        .param("body", "The expression being differentiated.")
        .param("var", "The variable of differentiation.")
        .optional("order", "The order of the derivative.")
        .example("(derivative (sin x) x)")
        .example("(deriv f t 2)"),
        Keyword::new("partial", "The partial derivative operator ∂/∂`var` applied to `body`.", partial)
            .param("body", "The expression being differentiated.")
            .param("var", "The variable of differentiation.")
            .sign("∂")
            .example("(partial f x)")
            .example("(partial (sin x) t)"),
        Keyword::new(
            "dd",
            "A Leibniz differential quotient: `(dd y x)` shows dy/dx, and with `order` dⁿy/dxⁿ.",
            dd,
        )
        .param("y", "The quantity being differentiated.")
        .param("var", "The variable of differentiation.")
        .optional("order", "The order of the derivative.")
        .example("(dd y x)")
        .example("(dd x t 2)"),
        Keyword::new(
            "evalat",
            "Evaluated at: `body` followed by a vertical bar with the point underneath. Written \
             `(evalat body [at] [value])`: with one argument after `body` that is the point; with \
             two the first is a variable and the second its value, shown as `var = value`; with \
             none the bar has nothing under it.",
            evalat,
        )
        .alias("eval-at")
        .param("body", "The expression being evaluated.")
        .optional("at", "With one argument after `body`: the point. With two: the variable.")
        .optional("value", "The value of the variable, when a variable is given.")
        .example("(evalat f x 0)")
        .example("(eval-at (frac 1 x) 2)")
        .example("(evalat f)"),
        Keyword::new("sin", "The sine function: `(sin x)` shows sin(x).", |call, out| {
            out.apply(|o| o.mi("sin"), slice::from_ref(call.req(0)));
        })
        .param("x", "The angle.")
        .sign("sin")
        .example("(sin x)")
        .example("(sin (+ x t))"),
        named("cos", "The cosine function: `(cos x)` shows cos(x).", "The angle.", "(cos x)", "(cos (+ x t))"),
        named("tan", "The tangent function: `(tan x)` shows tan(x).", "The angle.", "(tan x)", "(tan (+ x t))"),
        named("cot", "The cotangent function: `(cot x)` shows cot(x).", "The angle.", "(cot x)", "(cot (+ x t))"),
        named("sec", "The secant function: `(sec x)` shows sec(x).", "The angle.", "(sec x)", "(sec (+ x t))"),
        named("cosec", "The cosecant function: `(cosec x)` shows cosec(x).", "The angle.", "(cosec x)", "(csc (+ x t))")
            .alias("csc"),
        named("arcsin", "The inverse sine: `(arcsin x)` shows sin⁻¹(x); written on its own, `arcsin` shows the name arcsin.", "The value.", "(arcsin x)", "(asin (frac x 2))")
            .alias("asin"),
        named("arccos", "The inverse cosine: `(arccos x)` shows cos⁻¹(x); written on its own, `arccos` shows the name arccos.", "The value.", "(arccos x)", "(acos (frac x 2))")
            .alias("acos"),
        named("arctan", "The inverse tangent: `(arctan x)` shows tan⁻¹(x); written on its own, `arctan` shows the name arctan.", "The value.", "(arctan x)", "(atan (frac x 2))")
            .alias("atan"),
        named("ln", "The natural logarithm: `(ln x)` shows ln(x).", "The argument.", "(ln x)", "(ln (+ x 1))"),
        Keyword::new(
            "log",
            "A logarithm: `(log a)` shows log(a). With two arguments the first is the base, \
             written as a subscript: `(log 2 x)` shows log₂(x).",
            log,
        )
        .param("a", "With one argument: the argument of the logarithm. With two: the base.")
        .optional("b", "The argument of the logarithm, when a base is given.")
        .sign("log")
        .example("(log x)")
        .example("(log 2 x)"),
        Keyword::new("exp", "The exponential function: `(exp x)` shows e raised to x.", |call, out| {
            let x = call.req(0);
            out.tag("msup", |o| {
                o.mi("e");
                o.expr(x);
            });
        })
        .param("x", "The exponent.")
        .sign("exp")
        .example("(exp x)")
        .example("(exp (frac x 2))"),
        Keyword::new("abs", "The absolute value, written between bars: `(abs x)` shows |x|.", |call, out| {
            let x = call.req(0);
            out.fence("|", "|", |o| o.expr(x));
        })
        .param("x", "The value.")
        .example("(abs x)")
        .example("(abs (+ x 1))"),
        Keyword::new("floor", "The floor, written between ⌊ ⌋: `(floor x)` shows ⌊x⌋.", |call, out| {
            let x = call.req(0);
            out.fence("⌊", "⌋", |o| o.expr(x));
        })
        .param("x", "The value.")
        .example("(floor x)")
        .example("(floor (frac n 2))"),
        Keyword::new("ceil", "The ceiling, written between ⌈ ⌉: `(ceil x)` shows ⌈x⌉.", |call, out| {
            let x = call.req(0);
            out.fence("⌈", "⌉", |o| o.expr(x));
        })
        .param("x", "The value.")
        .example("(ceil x)")
        .example("(ceil (frac n 2))"),
        Keyword::new(
            "fact",
            "The factorial: `(fact n)` shows n!. A compound argument is parenthesised, including \
             another factorial: `(fact (fact n))` shows (n!)!, not the double factorial n!!.",
            |call, out| out.postfix("!", call.req(0)),
        )
        .alias("factorial")
        .param("n", "The number.")
        .sign("!")
        .example("(fact n)")
        .example("(factorial (+ n 1))"),
        Keyword::new(
            "binom",
            "A binomial coefficient: `n` stacked over `k` in parentheses, without a fraction bar.",
            |call, out| {
                let (n, k) = (call.req(0), call.req(1));
                out.fence("(", ")", |o| {
                    o.tag_attrs("mfrac", &[("linethickness", "0")], |o| {
                        o.expr(n);
                        o.expr(k);
                    });
                });
            },
        )
        .alias("nCr")
        .param("n", "The size of the set, written on top.")
        .param("k", "The number chosen, written underneath.")
        .example("(binom n k)")
        .example("(nCr (+ n 1) 2)"),
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`.
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    match (name, args) {
        // Without a body `lim` is only the operator and its subscript: an atom.
        ("integral" | "sum" | "prod" | "product" | "derivative" | "deriv" | "partial", _)
        | ("lim" | "limit", 3..) => Some(Prec::BigOp),
        ("dd", _) => Some(Prec::Fraction),
        ("exp", _) => Some(Prec::Power),
        ("fact" | "factorial" | "evalat" | "eval-at", _) => Some(Prec::Postfix),
        _ => None,
    }
}

/// A named function with one argument: `cos`, `tan`, `ln`, ... and the
/// inverses `arcsin`, ... written as sin⁻¹. The function written is the
/// keyword's primary name, so aliases render the same.
fn named(
    name: &'static str,
    doc: &'static str,
    x_doc: &'static str,
    first: &'static str,
    second: &'static str,
) -> Keyword {
    Keyword::new(name, doc, named_function)
        .param("x", x_doc)
        .sign(name)
        .example(first)
        .example(second)
}

fn named_function(call: &Call<'_>, out: &mut MathMl<'_>) {
    let name = call.keyword.name;
    out.apply(
        |o| match name.strip_prefix("arc") {
            Some(base) => o.tag("msup", |o| {
                o.mi(base);
                o.row(|o| {
                    o.mo("\u{2212}");
                    o.mn("1");
                });
            }),
            None => o.mi(name),
        },
        slice::from_ref(call.req(0)),
    );
}

/// `log`, with the base (written first) as a subscript.
fn log(call: &Call<'_>, out: &mut MathMl<'_>) {
    let x = call.req(0);
    match call.opt(1) {
        Some(arg) => out.apply(
            |o| {
                o.tag("msub", |o| {
                    o.mi("log");
                    o.expr(x);
                });
            },
            slice::from_ref(arg),
        ),
        None => out.apply(|o| o.mi("log"), slice::from_ref(x)),
    }
}

/// `∏`, like `sum`.
fn prod(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (body, var) = (call.req(0), call.req(1));
    out.row(|o| {
        match (call.opt(2), call.opt(3)) {
            (Some(lower), Some(upper)) => o.under_over(
                |o| o.mo("∏"),
                |o| {
                    o.row(|o| {
                        o.expr(var);
                        o.mo("=");
                        o.expr(lower);
                    });
                },
                |o| o.expr(upper),
            ),
            (Some(condition), None) => o.under(|o| o.mo("∏"), |o| o.expr(condition)),
            (None, _) => o.under(|o| o.mo("∏"), |o| o.expr(var)),
        }
        o.operand(body, Prec::BigOp);
    });
}

/// `lim` with `var → approach` underneath, then the body.
fn lim(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (var, approach) = (call.req(0), call.req(1));
    // An empty list `()` stands for "no side".
    let side = call.opt(3).filter(|s| !s.as_list().is_some_and(<[Node]>::is_empty));
    let operator = |o: &mut MathMl<'_>| {
        o.under(
            |o| o.mi("lim"),
            |o| {
                o.row(|o| {
                    o.expr(var);
                    o.mo("→");
                    match side {
                        Some(side) => o.sup(approach, side),
                        None => o.expr(approach),
                    }
                });
            },
        );
    };
    match call.opt(2) {
        Some(body) => out.row(|o| {
            operator(o);
            o.operand(body, Prec::BigOp);
        }),
        None => operator(out),
    }
}

/// One differential mark, `d` or `∂`, with the order as a superscript.
fn mark(out: &mut MathMl<'_>, partial: bool, order: Option<&Node>) {
    let glyph = |o: &mut MathMl<'_>| if partial { o.mo("∂") } else { o.mi("d") };
    match order {
        Some(n) => out.tag("msup", |o| {
            glyph(o);
            o.expr(n);
        }),
        None => glyph(out),
    }
}

/// The denominator `d var` or `d varⁿ`.
fn denominator(out: &mut MathMl<'_>, partial: bool, order: Option<&Node>, var: &Node) {
    out.row(|o| {
        mark(o, partial, None);
        match order {
            Some(n) => o.sup(var, n),
            None => o.operand(var, Prec::Atom),
        }
    });
}

/// `d/dx body` (or `∂/∂x`): the operator as a fraction, then the body.
fn operator_fraction(call: &Call<'_>, out: &mut MathMl<'_>, partial: bool) {
    let (body, var) = (call.req(0), call.req(1));
    let order = call.opt(2);
    out.row(|o| {
        o.tag("mfrac", |o| {
            mark(o, partial, order);
            denominator(o, partial, order, var);
        });
        o.operand(body, Prec::BigOp);
    });
}

fn derivative(call: &Call<'_>, out: &mut MathMl<'_>) {
    operator_fraction(call, out, false);
}

fn partial(call: &Call<'_>, out: &mut MathMl<'_>) {
    operator_fraction(call, out, true);
}

/// `dy/dx`: the quantity is inside the numerator.
fn dd(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (y, var) = (call.req(0), call.req(1));
    let order = call.opt(2);
    out.tag("mfrac", |o| {
        o.row(|o| {
            mark(o, false, order);
            o.operand(y, Prec::Multiplicative);
        });
        denominator(o, false, order, var);
    });
}

/// `body|` with the point (or `var = value`) underneath the bar.
fn evalat(call: &Call<'_>, out: &mut MathMl<'_>) {
    let body = call.req(0);
    out.tag("msub", |o| {
        o.row(|o| {
            o.operand(body, Prec::Fraction);
            o.mo("|");
        });
        match (call.opt(1), call.opt(2)) {
            (Some(var), Some(value)) => o.row(|o| {
                o.expr(var);
                o.mo("=");
                o.expr(value);
            }),
            (Some(point), None) => o.expr(point),
            _ => o.row(|_| {}),
        }
    });
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
    use crate::testing::{mathml, render, visible, with_standins};

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
    fn prod() {
        assert_eq!(
            mathml("(prod (subscript a n) n)"),
            "<mrow><munder><mo>∏</mo><mi>n</mi></munder><msub><mi>a</mi><mi>n</mi></msub></mrow>"
        );
        assert_eq!(
            mathml("(prod (frac 1 k) k 1 n)"),
            "<mrow><munderover><mo>∏</mo><mrow><mi>k</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover>\
             <mfrac><mn>1</mn><mi>k</mi></mfrac></mrow>"
        );
        assert_eq!(
            mathml("(product f x S)"),
            "<mrow><munder><mo>∏</mo><mi>S</mi></munder><mi>f</mi></mrow>"
        );
        // A sum as the factor is parenthesised.
        assert_eq!(visible("(prod (+ a b) n)"), "∏n(a+b)");
        assert_eq!(mathml("prod"), "<mo>∏</mo>");
        let (_, diags) = render("(prod a)");
        assert_eq!(diags.as_slice()[0].message, "`prod` is missing its `var` argument");
    }

    #[test]
    fn lim() {
        assert_eq!(
            mathml("(lim x 0 (frac (sin x) x))"),
            "<mrow><munder><mi>lim</mi><mrow><mi>x</mi><mo>→</mo><mn>0</mn></mrow></munder>\
             <mfrac><mrow><mi>sin</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow><mi>x</mi></mfrac></mrow>"
        );
        // The user's notes: an empty list as a fourth argument shows nothing.
        assert_eq!(
            mathml("(limit t a (subscript a t) ())"),
            "<mrow><munder><mi>lim</mi><mrow><mi>t</mi><mo>→</mo><mi>a</mi></mrow></munder>\
             <msub><mi>a</mi><mi>t</mi></msub></mrow>"
        );
        // A side is a superscript on the approached value.
        assert_eq!(visible("(lim x 0 f +)"), "limx→0+f");
        assert_eq!(
            mathml("(lim x 0 f +)"),
            "<mrow><munder><mi>lim</mi><mrow><mi>x</mi><mo>→</mo><msup><mn>0</mn><mo>+</mo></msup></mrow></munder>\
             <mi>f</mi></mrow>"
        );
        // Without a body only the operator is written.
        assert_eq!(
            mathml("(lim x 0)"),
            "<munder><mi>lim</mi><mrow><mi>x</mi><mo>→</mo><mn>0</mn></mrow></munder>"
        );
        // A sum or a signed number as the body is parenthesised.
        assert_eq!(visible("(lim x 0 (+ a b))"), "limx→0(a+b)");
        assert_eq!(visible("(lim x 0 -2)"), "limx→0(−2)");
        assert_eq!(mathml("lim"), "<mo>lim</mo>");
        let (_, diags) = render("(lim x)");
        assert_eq!(diags.as_slice()[0].message, "`lim` is missing its `approach` argument");
    }

    #[test]
    fn derivative() {
        assert_eq!(
            mathml("(derivative (sin x) x)"),
            "<mrow><mfrac><mi>d</mi><mrow><mi>d</mi><mi>x</mi></mrow></mfrac>\
             <mrow><mi>sin</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow></mrow>"
        );
        assert_eq!(
            mathml("(deriv f t 2)"),
            "<mrow><mfrac><msup><mi>d</mi><mn>2</mn></msup>\
             <mrow><mi>d</mi><msup><mi>t</mi><mn>2</mn></msup></mrow></mfrac><mi>f</mi></mrow>"
        );
        // A sum is parenthesised; a fraction is not (nested fractions).
        assert_eq!(visible("(derivative (+ a b) x)"), "ddx(a+b)");
        assert_eq!(
            mathml("(derivative (frac 1 x) x)"),
            "<mrow><mfrac><mi>d</mi><mrow><mi>d</mi><mi>x</mi></mrow></mfrac>\
             <mfrac><mn>1</mn><mi>x</mi></mfrac></mrow>"
        );
        // A compound variable gets parentheses, also under the order.
        assert_eq!(visible("(derivative f (+ a b) 2)"), "d2d(a+b)2f");
        let (_, diags) = render("derivative");
        assert_eq!(diags.as_slice()[0].message, "`derivative` needs arguments");
    }

    #[test]
    fn partial() {
        assert_eq!(
            mathml("(partial f x)"),
            "<mrow><mfrac><mo>∂</mo><mrow><mo>∂</mo><mi>x</mi></mrow></mfrac><mi>f</mi></mrow>"
        );
        assert_eq!(visible("(partial (sin x) t)"), "∂∂tsin(x)");
        assert_eq!(visible("(partial (+ a b) t)"), "∂∂t(a+b)");
        // The bare keyword is the sign.
        assert_eq!(mathml("partial"), "<mo>∂</mo>");
        let (_, diags) = render("(partial f)");
        assert_eq!(diags.as_slice()[0].message, "`partial` is missing its `var` argument");
    }

    #[test]
    fn dd() {
        assert_eq!(
            mathml("(dd y x)"),
            "<mfrac><mrow><mi>d</mi><mi>y</mi></mrow><mrow><mi>d</mi><mi>x</mi></mrow></mfrac>"
        );
        assert_eq!(
            mathml("(dd x t 2)"),
            "<mfrac><mrow><msup><mi>d</mi><mn>2</mn></msup><mi>x</mi></mrow>\
             <mrow><mi>d</mi><msup><mi>t</mi><mn>2</mn></msup></mrow></mfrac>"
        );
        // A sum in the numerator is parenthesised: d(a + b)/dt.
        assert_eq!(visible("(dd (+ a b) t)"), "d(a+b)dt");
        // Nested fractions.
        assert_eq!(
            mathml("(dd (frac a b) t)"),
            "<mfrac><mrow><mi>d</mi><mfrac><mi>a</mi><mi>b</mi></mfrac></mrow>\
             <mrow><mi>d</mi><mi>t</mi></mrow></mfrac>"
        );
        assert_eq!(visible("(dd y (frac a b))"), "dyd(ab)");
        let (_, diags) = render("dd");
        assert_eq!(diags.as_slice()[0].message, "`dd` needs arguments");
    }

    #[test]
    fn evalat() {
        assert_eq!(
            mathml("(evalat f x 0)"),
            "<msub><mrow><mi>f</mi><mo>|</mo></mrow><mrow><mi>x</mi><mo>=</mo><mn>0</mn></mrow></msub>"
        );
        assert_eq!(
            mathml("(eval-at (frac 1 x) 2)"),
            "<msub><mrow><mfrac><mn>1</mn><mi>x</mi></mfrac><mo>|</mo></mrow><mn>2</mn></msub>"
        );
        assert_eq!(mathml("(evalat f)"), "<msub><mrow><mi>f</mi><mo>|</mo></mrow><mrow></mrow></msub>");
        // A sum is parenthesised before the bar.
        assert_eq!(visible("(evalat (+ a b) x 0)"), "(a+b)|x=0");
    }

    #[test]
    fn trigonometry() {
        assert_eq!(mathml("(cos x)"), "<mrow><mi>cos</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>");
        assert_eq!(visible("(tan x)"), "tan(x)");
        assert_eq!(visible("(cot (+ x t))"), "cot(x+t)");
        assert_eq!(visible("(sec x)"), "sec(x)");
        assert_eq!(visible("(cosec x)"), "cosec(x)");
        assert_eq!(visible("(csc x)"), "cosec(x)");
        assert_eq!(mathml("cos"), "<mo>cos</mo>");
        assert_eq!(mathml("cosec"), "<mo>cosec</mo>");
        // No arguments is the bare name; too many is an error.
        assert_eq!(mathml("(cos)"), "<mo>cos</mo>");
        let (_, diags) = render("(cos x y)");
        assert_eq!(diags.as_slice()[0].code, mazer_syntax::DiagCode::Arity);
    }

    #[test]
    fn inverse_trigonometry() {
        assert_eq!(
            mathml("(arcsin x)"),
            "<mrow><msup><mi>sin</mi><mrow><mo>\u{2212}</mo><mn>1</mn></mrow></msup>\
             <mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(visible("(asin (frac x 2))"), "sin−1(x2)");
        assert_eq!(visible("(arccos x)"), "cos−1(x)");
        assert_eq!(visible("(acos x)"), "cos−1(x)");
        assert_eq!(visible("(arctan x)"), "tan−1(x)");
        assert_eq!(visible("(atan x)"), "tan−1(x)");
        assert_eq!(mathml("arctan"), "<mo>arctan</mo>");
    }

    #[test]
    fn logarithms() {
        assert_eq!(mathml("(ln x)"), "<mrow><mi>ln</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>");
        assert_eq!(visible("(ln (+ x 1))"), "ln(x+1)");
        assert_eq!(mathml("(log x)"), "<mrow><mi>log</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>");
        assert_eq!(
            mathml("(log 2 x)"),
            "<mrow><msub><mi>log</mi><mn>2</mn></msub><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(visible("(log (+ a b) x)"), "loga+b(x)");
        assert_eq!(mathml("ln"), "<mo>ln</mo>");
        assert_eq!(mathml("log"), "<mo>log</mo>");
    }

    #[test]
    fn exp() {
        assert_eq!(mathml("(exp x)"), "<msup><mi>e</mi><mi>x</mi></msup>");
        // The compound exponent from the notes.
        with_standins(|| {
            assert_eq!(visible("(exp (* -s t))"), "e−s×t");
            assert_eq!(visible("(exp (* -a s))"), "e−a×s");
        });
        assert_eq!(
            mathml("(exp (+ a b))"),
            "<msup><mi>e</mi><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow></msup>"
        );
        assert_eq!(mathml("exp"), "<mo>exp</mo>");
    }

    #[test]
    fn fences() {
        assert_eq!(mathml("(abs x)"), "<mrow><mo>|</mo><mi>x</mi><mo>|</mo></mrow>");
        assert_eq!(visible("(abs (+ x 1))"), "|x+1|");
        assert_eq!(mathml("(floor x)"), "<mrow><mo>⌊</mo><mi>x</mi><mo>⌋</mo></mrow>");
        assert_eq!(visible("(floor (frac n 2))"), "⌊n2⌋");
        assert_eq!(mathml("(ceil x)"), "<mrow><mo>⌈</mo><mi>x</mi><mo>⌉</mo></mrow>");
        assert_eq!(visible("(ceil (frac n 2))"), "⌈n2⌉");
        // A fence is an atom: no extra parentheses as a script base.
        assert_eq!(visible("(subscript (abs x) 2)"), "|x|2");
    }

    #[test]
    fn fact() {
        assert_eq!(mathml("(fact n)"), "<mrow><mi>n</mi><mo>!</mo></mrow>");
        assert_eq!(mathml("(factorial n)"), "<mrow><mi>n</mi><mo>!</mo></mrow>");
        assert_eq!(visible("(fact (+ n 1))"), "(n+1)!");
        assert_eq!(visible("(fact (fact n))"), "(n!)!");
        assert_eq!(mathml("fact"), "<mo>!</mo>");
        assert_eq!(mathml("factorial"), "<mo>!</mo>");
    }

    #[test]
    fn binom() {
        assert_eq!(
            mathml("(binom n k)"),
            "<mrow><mo>(</mo><mfrac linethickness=\"0\"><mi>n</mi><mi>k</mi></mfrac><mo>)</mo></mrow>"
        );
        assert_eq!(visible("(nCr (+ n 1) 2)"), "(n+12)");
        let (_, diags) = render("(binom n)");
        assert_eq!(diags.as_slice()[0].message, "`binom` is missing its `k` argument");
    }

    #[test]
    fn precedence() {
        with_standins(|| {
            // Big operators are parenthesised as a non-last factor and as a script base ...
            assert_eq!(visible("(* (prod a n) b)"), "(∏na)×b");
            assert_eq!(visible("(* b (prod a n))"), "b×∏na");
            assert_eq!(visible("(* (lim x 0 f) g)"), "(limx→0f)×g");
            assert_eq!(visible("(* g (lim x 0 f))"), "g×limx→0f");
            assert_eq!(visible("(* (derivative f x) g)"), "(ddxf)×g");
            assert_eq!(visible("(* (partial f x) g)"), "(∂∂xf)×g");
            assert_eq!(visible("(subscript (lim x 0 f) 2)"), "(limx→0f)2");
            assert_eq!(visible("(^ (prod a n) 2)"), "(∏na)2");
            assert_eq!(visible("(+ (prod a n) b)"), "∏na+b");
            // ... but a limit without a body is only an operator and an atom.
            assert_eq!(visible("(* (lim x 0) g)"), "limx→0×g");
            // Factorial of a compound, and a power of a factorial.
            assert_eq!(visible("(fact (^ x 2))"), "(x2)!");
            assert_eq!(visible("(fact (* a b))"), "(a×b)!");
            assert_eq!(visible("(^ (fact n) 2)"), "(n!)2");
            assert_eq!(visible("(fact (sin x))"), "sin(x)!");
            // A named function applied to a compound argument keeps its own parentheses.
            assert_eq!(visible("(cos (* a b))"), "cos(a×b)");
            assert_eq!(visible("(ln (- a b))"), "ln(a−b)");
            assert_eq!(visible("(* (cos x) (sin x))"), "cos(x)×sin(x)");
            // `exp` of a sum, and exp as a base or under a postfix.
            assert_eq!(visible("(exp (+ a b))"), "ea+b");
            assert_eq!(visible("(^ (exp x) 2)"), "(ex)2");
            assert_eq!(visible("(fact (exp x))"), "(ex)!");
            assert_eq!(visible("(exp (^ x 2))"), "ex2");
            // `evalat` is postfix: a power of it is parenthesised, and so is its body.
            assert_eq!(visible("(^ (evalat f x 0) 2)"), "(f|x=0)2");
            assert_eq!(visible("(evalat (* a b) x 0)"), "(a×b)|x=0");
            // `dd` is a fraction: a power of it is parenthesised.
            assert_eq!(visible("(^ (dd y x) 2)"), "(dydx)2");
            // A signed body is parenthesised after any big operator.
            assert_eq!(visible("(derivative (- a) x)"), "ddx(−a)");
            assert_eq!(visible("(prod (- a) n)"), "∏n(−a)");
        });
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
