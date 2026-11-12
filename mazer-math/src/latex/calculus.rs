//! LaTeX emitters for the keywords of `keywords/calculus.rs`.

use std::slice;

use mazer_syntax::Node;

use super::{Latex, LatexFn};
use crate::call::Call;
use crate::mathml::Prec;

/// The emitters of this file, by the keyword's primary name.
pub(super) fn emitters() -> Vec<(&'static str, LatexFn)> {
    fn e(name: &'static str, f: LatexFn) -> (&'static str, LatexFn) {
        (name, f)
    }
    vec![
        e("integral", integral),
        e("sum", |call, out| big_operator("\\sum", call, out)),
        e("prod", |call, out| big_operator("\\prod", call, out)),
        e("lim", lim),
        e("derivative", |call, out| operator_fraction(call, out, false)),
        e("partial", |call, out| operator_fraction(call, out, true)),
        e("dd", dd),
        e("evalat", evalat),
        e("sin", named_function),
        e("cos", named_function),
        e("tan", named_function),
        e("cot", named_function),
        e("sec", named_function),
        e("cosec", named_function),
        e("arcsin", named_function),
        e("arccos", named_function),
        e("arctan", named_function),
        e("ln", named_function),
        e("log", log),
        e("exp", |call, out| {
            let x = call.req(0);
            out.raw("e");
            out.superscript(x);
        }),
        e("abs", |call, out| {
            let x = call.req(0);
            out.fence("|", "|", |o| o.expr(x));
        }),
        e("floor", |call, out| {
            let x = call.req(0);
            out.fence("\u{230a}", "\u{230b}", |o| o.expr(x));
        }),
        e("ceil", |call, out| {
            let x = call.req(0);
            out.fence("\u{2308}", "\u{2309}", |o| o.expr(x));
        }),
        e("fact", |call, out| out.postfix("!", call.req(0))),
        e("binom", |call, out| {
            let (n, k) = (call.req(0), call.req(1));
            out.raw("\\binom");
            out.group(|o| o.expr(n));
            out.group(|o| o.expr(k));
        }),
    ]
}

/// A named function with one argument. `cosec` has no command of its own, and
/// the inverses `arcsin`, ... are written `\sin^{-1}`, as in `MathML`. The
/// function written is the keyword's primary name, so aliases render the same.
fn named_function(call: &Call<'_>, out: &mut Latex<'_>) {
    let name = call.keyword.name;
    out.apply(
        |o| match name.strip_prefix("arc") {
            Some(base) => {
                o.raw(&command(base));
                o.raw("^{-1}");
            }
            None => o.raw(&command(name)),
        },
        slice::from_ref(call.req(0)),
    );
}

/// `\sin`, `\ln`, `\operatorname{cosec}`.
fn command(name: &str) -> String {
    if name == "cosec" {
        "\\operatorname{cosec}".to_owned()
    } else {
        format!("\\{name}")
    }
}

/// `log`, with the base (written first) as a subscript.
fn log(call: &Call<'_>, out: &mut Latex<'_>) {
    let x = call.req(0);
    match call.opt(1) {
        Some(arg) => out.apply(
            |o| {
                o.raw("\\log");
                o.subscript(x);
            },
            slice::from_ref(arg),
        ),
        None => out.apply(|o| o.raw("\\log"), slice::from_ref(x)),
    }
}

/// `\sum` or `\prod` with its limits, then the body.
fn big_operator(sign: &str, call: &Call<'_>, out: &mut Latex<'_>) {
    let (body, var) = (call.req(0), call.req(1));
    out.raw(sign);
    match (call.opt(2), call.opt(3)) {
        (Some(lower), Some(upper)) => {
            out.raw("_");
            out.group(|o| {
                o.expr(var);
                o.raw("=");
                o.expr(lower);
            });
            out.superscript(upper);
        }
        (Some(condition), None) => out.subscript(condition),
        (None, _) => out.subscript(var),
    }
    out.space();
    out.operand(body, Prec::BigOp);
}

/// `\lim_{var \to approach}`, then the body.
fn lim(call: &Call<'_>, out: &mut Latex<'_>) {
    let (var, approach) = (call.req(0), call.req(1));
    // An empty list `()` stands for "no side".
    let side = call.opt(3).filter(|s| !s.as_list().is_some_and(<[Node]>::is_empty));
    out.raw("\\lim_");
    out.group(|o| {
        o.expr(var);
        o.raw(" \\to ");
        match side {
            Some(side) => o.sup(approach, side),
            None => o.expr(approach),
        }
    });
    if let Some(body) = call.opt(2) {
        out.space();
        out.operand(body, Prec::BigOp);
    }
}

/// One differential mark, `\mathrm{d}` or `\partial`, with the order as a superscript.
fn mark(out: &mut Latex<'_>, partial: bool, order: Option<&Node>) {
    out.raw(if partial { "\\partial" } else { "\\mathrm{d}" });
    if let Some(n) = order {
        out.superscript(n);
    }
}

/// The denominator `d var` or `d var^n`.
fn denominator(out: &mut Latex<'_>, partial: bool, order: Option<&Node>, var: &Node) {
    mark(out, partial, None);
    match order {
        Some(n) => out.sup(var, n),
        None => out.operand(var, Prec::Atom),
    }
}

/// `d/dx body` (or `\partial/\partial x`): the operator as a fraction, then the body.
fn operator_fraction(call: &Call<'_>, out: &mut Latex<'_>, partial: bool) {
    let (body, var) = (call.req(0), call.req(1));
    let order = call.opt(2);
    out.raw("\\frac");
    out.group(|o| mark(o, partial, order));
    out.group(|o| denominator(o, partial, order, var));
    out.space();
    // Same rule as the MathML emitter: no closing mark, so wrap a product.
    out.operand(body, Prec::Fraction);
}

/// `dy/dx`: the quantity is inside the numerator.
fn dd(call: &Call<'_>, out: &mut Latex<'_>) {
    let (y, var) = (call.req(0), call.req(1));
    let order = call.opt(2);
    out.raw("\\frac");
    out.group(|o| {
        mark(o, false, order);
        o.operand(y, Prec::Fraction);
    });
    out.group(|o| denominator(o, false, order, var));
}

/// `\left. body \right|_{...}` with the point (or `var = value`) as the subscript.
fn evalat(call: &Call<'_>, out: &mut Latex<'_>) {
    let body = call.req(0);
    out.fence("", "|", |o| o.operand(body, Prec::Fraction));
    match (call.opt(1), call.opt(2)) {
        (Some(var), Some(value)) => {
            out.raw("_");
            out.group(|o| {
                o.expr(var);
                o.raw("=");
                o.expr(value);
            });
        }
        (Some(point), None) => out.subscript(point),
        _ => {}
    }
}

fn integral(call: &Call<'_>, out: &mut Latex<'_>) {
    let (body, var) = (call.req(0), call.req(1));
    out.raw("\\int");
    match (call.opt(2), call.opt(3)) {
        (Some(lower), Some(upper)) => {
            out.subscript(lower);
            out.superscript(upper);
        }
        (Some(region), None) => out.subscript(region),
        _ => {}
    }
    out.space();
    out.operand(body, Prec::BigOp);
    out.raw(" \\, \\mathrm{d}");
    out.operand(var, Prec::Atom);
}
