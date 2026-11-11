//! LaTeX emitters for the keywords of `keywords/algebra.rs`.

use super::{Latex, LatexFn};
use crate::call::Call;
use crate::mathml::{Assoc, Prec};

/// The emitters of this file, by the keyword's primary name.
pub(super) fn emitters() -> Vec<(&'static str, LatexFn)> {
    fn e(name: &'static str, f: LatexFn) -> (&'static str, LatexFn) {
        (name, f)
    }
    vec![
        e("+", add),
        e("-", subtract),
        e("*", |call, out| nary("\\times", "*", Prec::Multiplicative, Assoc::Both, call, out)),
        e("/", divide),
        e("^", |call, out| out.sup(call.req(0), call.req(1))),
        e("sqrt", |call, out| {
            let radicand = call.req(0);
            out.raw("\\sqrt");
            out.group(|o| o.expr(radicand));
        }),
        e("root", |call, out| {
            let (index, radicand) = (call.req(0), call.req(1));
            out.raw("\\sqrt[");
            out.expr(index);
            out.raw("]");
            out.group(|o| o.expr(radicand));
        }),
        e("jux", |call, out| out.juxtapose(call.rest())),
        e("=", |call, out| nary("=", "=", Prec::Relation, Assoc::Neither, call, out)),
        e("!=", |call, out| relation("\\neq", call, out)),
        e("<", |call, out| nary("<", "<", Prec::Relation, Assoc::Neither, call, out)),
        e(">", |call, out| nary(">", ">", Prec::Relation, Assoc::Neither, call, out)),
        e("<=", |call, out| nary("\\leq", "\\leq", Prec::Relation, Assoc::Neither, call, out)),
        e(">=", |call, out| nary("\\geq", "\\geq", Prec::Relation, Assoc::Neither, call, out)),
        e("approx", |call, out| relation("\\approx", call, out)),
        e("set", |call, out| {
            let elements = call.rest();
            out.fence("{", "}", |o| o.separated(",", elements));
        }),
        e("in", |call, out| relation("\\in", call, out)),
        e("notin", |call, out| relation("\\notin", call, out)),
        e("subset", |call, out| relation("\\subseteq", call, out)),
        e("superset", |call, out| relation("\\supseteq", call, out)),
        e("union", |call, out| nary("\\cup", "\\cup", Prec::Additive, Assoc::Both, call, out)),
        e("intersect", |call, out| nary("\\cap", "\\cap", Prec::Multiplicative, Assoc::Both, call, out)),
        e("and", |call, out| nary("\\land", "\\land", Prec::And, Assoc::Both, call, out)),
        e("or", |call, out| nary("\\lor", "\\lor", Prec::Or, Assoc::Both, call, out)),
        e("not", |call, out| out.prefix("\\lnot", call.req(0), Prec::Not)),
        e("implies", |call, out| {
            out.binary("\\implies", Prec::Implication, Assoc::Neither, call.req(0), call.req(1));
        }),
        e("iff", |call, out| {
            out.binary("\\iff", Prec::Implication, Assoc::Neither, call.req(0), call.req(1));
        }),
        e("forall", |call, out| out.binder("\\forall", call.req(0), call.req(1))),
        e("exists", |call, out| out.binder("\\exists", call.req(0), call.req(1))),
        e("paren", |call, out| {
            let body = call.req(0);
            out.fence("(", ")", |o| o.expr(body));
        }),
        e("bracket", |call, out| {
            let body = call.req(0);
            out.fence("[", "]", |o| o.expr(body));
        }),
        e("brace", |call, out| {
            let body = call.req(0);
            out.fence("{", "}", |o| o.expr(body));
        }),
        e("frac", |call, out| out.frac(call.req(0), call.req(1))),
    ]
}

/// A two-argument relation.
fn relation(op: &str, call: &Call<'_>, out: &mut Latex<'_>) {
    out.binary(op, Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
}

/// An n-ary infix row of `op`; with no operands the bare `bare` sign.
fn nary(op: &str, bare: &str, prec: Prec, assoc: Assoc, call: &Call<'_>, out: &mut Latex<'_>) {
    match call.rest() {
        [] => out.raw(bare),
        terms => out.infix(op, prec, assoc, terms),
    }
}

fn add(call: &Call<'_>, out: &mut Latex<'_>) {
    match call.rest() {
        [] => out.raw("+"),
        [term] => out.prefix("+", term, Prec::BigOp),
        terms => out.infix("+", Prec::Additive, Assoc::Both, terms),
    }
}

fn subtract(call: &Call<'_>, out: &mut Latex<'_>) {
    match call.rest() {
        [] => out.raw("-"),
        [term] => out.prefix("-", term, Prec::BigOp),
        terms => out.infix("-", Prec::Additive, Assoc::Left, terms),
    }
}

fn divide(call: &Call<'_>, out: &mut Latex<'_>) {
    match call.rest() {
        [] => out.raw("\\div"),
        [den] => {
            out.raw("\\frac{1}");
            out.group(|o| o.expr(den));
        }
        [num, den] => out.frac(num, den),
        terms => out.infix("\\div", Prec::Multiplicative, Assoc::Left, terms),
    }
}
