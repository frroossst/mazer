//! Arithmetic, comparison, sets, logic, grouping.

use crate::call::Call;
use crate::keyword::Keyword;
use crate::mathml::{Assoc, MathMl, Prec};

#[must_use]
#[allow(clippy::too_many_lines)] // one long declarative table of keywords
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
        Keyword::new(
            "-",
            "Subtraction: the first term minus the others. With one term it is negation (`-x`); \
             with none it is the `−` sign itself.",
            subtract,
        )
        .alias("sub")
        .rest("terms", "The first term, then the terms taken away from it.")
        .sign("\u{2212}")
        .example("(- t a)")
        .example("(- x)")
        .example("(sub a b c)"),
        Keyword::new(
            "*",
            "Multiplication: the factors joined by `×`. With one factor it shows just that \
             factor; with none, the bare `*` sign.",
            |call, out| nary("\u{d7}", "*", Prec::Multiplicative, Assoc::Both, call, out),
        )
        .alias("mul")
        .rest("factors", "The factors to multiply, left to right.")
        .sign("*")
        .example("(* a b)")
        .example("(* 2 (+ a b) c)")
        .example("(mul x y)"),
        Keyword::new(
            "/",
            "Division. Two arguments are written as a fraction, one argument as 1 over it, \
             and three or more are joined by `÷` and group to the left.",
            divide,
        )
        .alias("div")
        .rest("terms", "The dividend, then the divisors.")
        .sign("\u{f7}")
        .example("(/ 1 t)")
        .example("(/ (+ a b) c)")
        .example("(/ a b c)")
        .example("(div x)"),
        Keyword::new("^", "A power: the base with the exponent written as a superscript.", |call, out| {
            out.sup(call.req(0), call.req(1));
        })
        .alias("pow")
        .param("base", "The expression raised to the power.")
        .param("exponent", "The exponent, written smaller and higher.")
        .example("(^ x 2)")
        .example("(^ (+ a b) n)")
        .example("(pow e (- x))"),
        Keyword::new("sqrt", "A square root with a radical sign over its argument.", |call, out| {
            let radicand = call.req(0);
            out.tag("msqrt", |o| o.expr(radicand));
        })
        .param("x", "The expression under the root.")
        .example("(sqrt x)")
        .example("(sqrt (+ a b))"),
        Keyword::new(
            "root",
            "An nth root: `(root 3 x)` shows the cube root of x. Note that the index comes first.",
            |call, out| {
                let (index, radicand) = (call.req(0), call.req(1));
                out.tag("mroot", |o| {
                    o.expr(radicand);
                    o.expr(index);
                });
            },
        )
        .param("n", "The index of the root, written in the notch.")
        .param("x", "The expression under the root.")
        .example("(root 3 x)")
        .example("(root n (+ a b))"),
        Keyword::new(
            "jux",
            "Juxtaposition: the arguments written side by side with a thin space, as in implicit \
             multiplication. With one argument it shows just that argument.",
            |call, out| out.juxtapose(call.rest()),
        )
        .alias("juxtapose")
        .rest("items", "The expressions to write next to each other.")
        .example("(jux a b c)")
        .example("(jux 2 (+ a b))")
        .example("(juxtapose f (paren x))"),
        Keyword::new(
            "=",
            "Equality: the arguments joined by `=`, so `(= a b c)` is a chain. With none it is \
             the `=` sign itself.",
            |call, out| nary("=", "=", Prec::Relation, Assoc::Neither, call, out),
        )
        .alias("eq")
        .rest("terms", "The two or more expressions that are equal.")
        .sign("=")
        .example("(= a b)")
        .example("(= a b c)")
        .example("(eq x (+ 1 y))"),
        Keyword::new("!=", "Inequality: the two expressions joined by `≠`.", |call, out| {
            out.binary("\u{2260}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .alias("neq")
        .param("a", "The left side.")
        .param("b", "The right side.")
        .sign("\u{2260}")
        .example("(!= a b)")
        .example("(neq x 0)"),
        Keyword::new("<", "Less than: the arguments joined by `<`, so `(< a b c)` is a chain.", |call, out| {
            nary("<", "<", Prec::Relation, Assoc::Neither, call, out);
        })
        .alias("lt")
        .rest("terms", "The two or more expressions, smallest first.")
        .sign("<")
        .example("(< a b)")
        .example("(lt 0 x 1)"),
        Keyword::new(">", "Greater than: the arguments joined by `>`, so `(> a b c)` is a chain.", |call, out| {
            nary(">", ">", Prec::Relation, Assoc::Neither, call, out);
        })
        .alias("gt")
        .rest("terms", "The two or more expressions, largest first.")
        .sign(">")
        .example("(> a b)")
        .example("(gt x 0)"),
        Keyword::new(
            "<=",
            "Less than or equal: the arguments joined by `≤`, so `(<= a b c)` is a chain.",
            |call, out| nary("\u{2264}", "\u{2264}", Prec::Relation, Assoc::Neither, call, out),
        )
        .alias("leq")
        .rest("terms", "The two or more expressions, smallest first.")
        .sign("\u{2264}")
        .example("(<= a b)")
        .example("(leq 0 x 1)"),
        Keyword::new(
            ">=",
            "Greater than or equal: the arguments joined by `≥`, so `(>= a b c)` is a chain.",
            |call, out| nary("\u{2265}", "\u{2265}", Prec::Relation, Assoc::Neither, call, out),
        )
        .alias("geq")
        .rest("terms", "The two or more expressions, largest first.")
        .sign("\u{2265}")
        .example("(>= a 0)")
        .example("(geq x y z)"),
        Keyword::new("approx", "Approximate equality: the two expressions joined by `≈`.", |call, out| {
            out.binary("\u{2248}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .alias("\u{2248}")
        .param("a", "The left side.")
        .param("b", "The right side.")
        .sign("\u{2248}")
        .example("(approx a b)")
        .example("(approx x (/ 22 7))"),
        Keyword::new(
            "set",
            "A set written in braces with its elements separated by commas; with no arguments \
             it is the empty set `{}`.",
            |call, out| {
                let elements = call.rest();
                out.fence("{", "}", |o| o.separated(",", elements));
            },
        )
        .rest("elements", "The elements of the set.")
        .example("(set a b c)")
        .example("(set 1 (+ x 1))"),
        Keyword::new("in", "Element of: `x ∈ S`.", |call, out| {
            out.binary("\u{2208}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("x", "The element.")
        .param("set", "The set it belongs to.")
        .sign("\u{2208}")
        .example("(in x S)")
        .example("(in a (set 1 2))"),
        Keyword::new("notin", "Not an element of: `x ∉ S`.", |call, out| {
            out.binary("\u{2209}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("x", "The element.")
        .param("set", "The set it does not belong to.")
        .sign("\u{2209}")
        .example("(notin x S)")
        .example("(notin 0 (set 1 2))"),
        Keyword::new("subset", "Subset: `A ⊆ B`. The bare keyword shows ⊂.", |call, out| {
            out.binary("\u{2286}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("a", "The smaller set.")
        .param("b", "The larger set.")
        .sign("\u{2282}")
        .example("(subset A B)")
        .example("(subset (set a) (set a b))"),
        Keyword::new("superset", "Superset: `A ⊇ B`.", |call, out| {
            out.binary("\u{2287}", Prec::Relation, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("a", "The larger set.")
        .param("b", "The smaller set.")
        .sign("\u{2287}")
        .example("(superset A B)")
        .example("(superset (set a b) (set a))"),
        Keyword::new(
            "union",
            "Union: the sets joined by `∪`. With none it is the `∪` sign itself.",
            |call, out| nary("\u{222a}", "\u{222a}", Prec::Additive, Assoc::Both, call, out),
        )
        .rest("sets", "The sets to unite.")
        .sign("\u{222a}")
        .example("(union A B)")
        .example("(union A B C)"),
        Keyword::new(
            "intersect",
            "Intersection: the sets joined by `∩`. With none it is the `∩` sign itself.",
            |call, out| nary("\u{2229}", "\u{2229}", Prec::Multiplicative, Assoc::Both, call, out),
        )
        .rest("sets", "The sets to intersect.")
        .sign("\u{2229}")
        .example("(intersect A B)")
        .example("(intersect A (union B C))"),
        Keyword::new(
            "and",
            "Logical and: the statements joined by `∧`. With none it is the `∧` sign itself.",
            |call, out| nary("\u{2227}", "\u{2227}", Prec::And, Assoc::Both, call, out),
        )
        .rest("statements", "The statements that all hold.")
        .sign("\u{2227}")
        .example("(and p q)")
        .example("(and p (or q r) s)"),
        Keyword::new(
            "or",
            "Logical or: the statements joined by `∨`. With none it is the `∨` sign itself.",
            |call, out| nary("\u{2228}", "\u{2228}", Prec::Or, Assoc::Both, call, out),
        )
        .rest("statements", "The statements of which at least one holds.")
        .sign("\u{2228}")
        .example("(or p q)")
        .example("(or p q r)"),
        Keyword::new("not", "Logical negation: `¬p`.", |call, out| {
            out.prefix("\u{ac}", call.req(0), Prec::Not);
        })
        .param("p", "The statement being negated.")
        .sign("\u{ac}")
        .example("(not p)")
        .example("(not (and p q))"),
        Keyword::new("implies", "Implication: `p ⟹ q`.", |call, out| {
            out.binary("\u{27f9}", Prec::Implication, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("p", "The hypothesis.")
        .param("q", "The conclusion.")
        .sign("\u{27f9}")
        .example("(implies p q)")
        .example("(implies (and p q) r)"),
        Keyword::new("iff", "Biconditional, if and only if: `p ⟺ q`.", |call, out| {
            out.binary("\u{27fa}", Prec::Implication, Assoc::Neither, call.req(0), call.req(1));
        })
        .param("p", "The left statement.")
        .param("q", "The right statement.")
        .sign("\u{27fa}")
        .example("(iff p q)")
        .example("(iff (= x 0) (= y 0))"),
        Keyword::new(
            "forall",
            "Universal quantifier: `∀x. P`. The body runs as far right as possible; an empty \
             body `()` leaves a placeholder.",
            |call, out| out.binder("\u{2200}", call.req(0), call.req(1)),
        )
        .param("var", "The bound variable.")
        .param("body", "The statement that holds for every value of the variable.")
        .sign("\u{2200}")
        .example("(forall x (> x 0))")
        .example("(forall alpha (jux alpha beta))")
        .example("(forall alpha ())"),
        Keyword::new(
            "exists",
            "Existential quantifier: `∃x. P`. The body runs as far right as possible; an empty \
             body `()` leaves a placeholder.",
            |call, out| out.binder("\u{2203}", call.req(0), call.req(1)),
        )
        .param("var", "The bound variable.")
        .param("body", "The statement that holds for some value of the variable.")
        .sign("\u{2203}")
        .example("(exists x (= (* x x) 2))")
        .example("(exists n ())"),
        Keyword::new("paren", "Parentheses around an expression; they grow with its height.", |call, out| {
            let body = call.req(0);
            out.fence("(", ")", |o| o.expr(body));
        })
        .param("body", "The expression inside the parentheses.")
        .example("(paren x)")
        .example("(paren (frac 1 2))"),
        Keyword::new("bracket", "Square brackets around an expression; they grow with its height.", |call, out| {
            let body = call.req(0);
            out.fence("[", "]", |o| o.expr(body));
        })
        .param("body", "The expression inside the brackets.")
        .example("(bracket x)")
        .example("(bracket (frac a b))"),
        Keyword::new("brace", "Curly braces around an expression; they grow with its height.", |call, out| {
            let body = call.req(0);
            out.fence("{", "}", |o| o.expr(body));
        })
        .param("body", "The expression inside the braces.")
        .example("(brace x)")
        .example("(brace (+ a b))"),
        Keyword::new("frac", "A fraction: the numerator stacked over the denominator.", |call, out| {
            out.frac(call.req(0), call.req(1));
        })
        .param("num", "The numerator, written on top.")
        .param("den", "The denominator, written underneath.")
        .example("(frac 1 2)")
        .example("(frac (+ a b) c)"),
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`.
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    match (name, args) {
        // With no arguments a call renders the sign itself: an atom.
        ("+" | "add" | "-" | "sub", 1) => Some(Prec::Prefix),
        ("+" | "add" | "-" | "sub", 2..) | ("union", 1..) => Some(Prec::Additive),
        ("*" | "mul" | "jux" | "juxtapose" | "intersect", 1..) | ("/" | "div", 3..) => Some(Prec::Multiplicative),
        ("frac", _) | ("/" | "div", 1..=2) => Some(Prec::Fraction),
        ("^" | "pow", _) => Some(Prec::Power),
        ("=" | "eq" | "<" | "lt" | ">" | "gt" | "<=" | "leq" | ">=" | "geq", 1..)
        | ("!=" | "neq" | "approx" | "\u{2248}" | "in" | "notin" | "subset" | "superset", _) => {
            Some(Prec::Relation)
        }
        ("and", 1..) => Some(Prec::And),
        ("or", 1..) => Some(Prec::Or),
        ("not", _) => Some(Prec::Not),
        ("implies" | "iff", _) => Some(Prec::Implication),
        ("forall" | "exists", _) => Some(Prec::Binder),
        _ => None,
    }
}

/// An n-ary infix row of `glyph`; with no operands the bare `bare` sign.
fn nary(glyph: &str, bare: &str, prec: Prec, assoc: Assoc, call: &Call<'_>, out: &mut MathMl<'_>) {
    match call.rest() {
        [] => out.mo(bare),
        terms => out.infix(glyph, prec, assoc, terms),
    }
}

fn add(call: &Call<'_>, out: &mut MathMl<'_>) {
    match call.rest() {
        [] => out.mo("+"),
        [term] => out.prefix("+", term, Prec::BigOp),
        terms => out.infix("+", Prec::Additive, Assoc::Both, terms),
    }
}

fn subtract(call: &Call<'_>, out: &mut MathMl<'_>) {
    match call.rest() {
        [] => out.mo("\u{2212}"),
        [term] => out.prefix("\u{2212}", term, Prec::BigOp),
        terms => out.infix("\u{2212}", Prec::Additive, Assoc::Left, terms),
    }
}

fn divide(call: &Call<'_>, out: &mut MathMl<'_>) {
    match call.rest() {
        [] => out.mo("\u{f7}"),
        [den] => out.tag("mfrac", |o| {
            o.mn("1");
            o.expr(den);
        }),
        [num, den] => out.frac(num, den),
        terms => out.infix("\u{f7}", Prec::Multiplicative, Assoc::Left, terms),
    }
}


#[cfg(test)]
mod tests {
    use crate::testing::{mathml, render, visible};

    /// Asserts what a reader sees for each `(source, text)` pair.
    fn shows(cases: &[(&str, &str)]) {
        for &(src, text) in cases {
            assert_eq!(visible(src), text, "{src}");
        }
    }

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
    fn subtract() {
        assert_eq!(mathml("(- t a)"), "<mrow><mi>t</mi><mo>\u{2212}</mo><mi>a</mi></mrow>");
        assert_eq!(
            mathml("(sub a b c)"),
            "<mrow><mi>a</mi><mo>\u{2212}</mo><mi>b</mi><mo>\u{2212}</mo><mi>c</mi></mrow>"
        );
        // Negation, including what the reader makes of `-s`.
        assert_eq!(mathml("(- x)"), "<mrow><mo>\u{2212}</mo><mi>x</mi></mrow>");
        assert_eq!(mathml("(- infinity)"), "<mrow><mo>\u{2212}</mo><mi>\u{221e}</mi></mrow>");
        assert_eq!(mathml("-s"), mathml("(- s)"));
        // The bare keyword and the empty call are the sign, in U+2212.
        assert_eq!(mathml("-"), "<mo>\u{2212}</mo>");
        assert_eq!(mathml("(-)"), "<mo>\u{2212}</mo>");
        // Subtraction groups to the left.
        shows(&[
            ("(- a (- b c))", "a\u{2212}(b\u{2212}c)"),
            ("(- (- a b) c)", "a\u{2212}b\u{2212}c"),
            ("(- a (+ b c))", "a\u{2212}(b+c)"),
            ("(- (+ a b) c)", "a+b\u{2212}c"),
        ]);
    }

    #[test]
    fn negation_in_every_position() {
        shows(&[
            ("(- (- x))", "\u{2212}(\u{2212}x)"),
            ("(- (+ a b))", "\u{2212}(a+b)"),
            ("(- a -2)", "a\u{2212}(\u{2212}2)"),
            ("(+ a (- b))", "a+(\u{2212}b)"),
            ("(- (- a) b)", "\u{2212}a\u{2212}b"),
            ("(- a (- b))", "a\u{2212}(\u{2212}b)"),
            ("(* -s t)", "\u{2212}s\u{d7}t"),
            ("(* t (- s))", "t\u{d7}(\u{2212}s)"),
            ("(* infinity (/ 1 infinity))", "\u{221e}\u{d7}1\u{221e}"),
            ("(jux a (- b))", "a (\u{2212}b)"),
            ("(jux (- a) b)", "\u{2212}a b"),
            ("(^ (- x) 2)", "(\u{2212}x)2"),
            ("(^ x (- n))", "x\u{2212}n"),
            ("(= x (- y))", "x=\u{2212}y"),
            ("(= (- x) y)", "\u{2212}x=y"),
            ("(< a (- b) c)", "a<\u{2212}b<c"),
            ("(not (- x))", "\u{ac}\u{2212}x"),
            ("(and (= x -1) p)", "x=\u{2212}1\u{2227}p"),
            ("(sqrt (- x))", "\u{2212}x"),
            ("(- (* a b))", "\u{2212}a\u{d7}b"),
            ("(- (/ a b))", "\u{2212}ab"),
            ("(/ (- a) b)", "\u{2212}ab"),
            ("(/ a b (- c))", "a\u{f7}b\u{f7}(\u{2212}c)"),
            ("(union a (- b))", "a\u{222a}(\u{2212}b)"),
        ]);
    }

    #[test]
    fn multiply() {
        assert_eq!(mathml("(* a b)"), "<mrow><mi>a</mi><mo>\u{d7}</mo><mi>b</mi></mrow>");
        assert_eq!(
            mathml("(mul x y z)"),
            "<mrow><mi>x</mi><mo>\u{d7}</mo><mi>y</mi><mo>\u{d7}</mo><mi>z</mi></mrow>"
        );
        // The bare keyword and the empty call keep the old `*`.
        assert_eq!(mathml("*"), "<mo>*</mo>");
        assert_eq!(mathml("(*)"), "<mo>*</mo>");
        // One factor shows itself.
        assert_eq!(mathml("(* x)"), "<mrow><mi>x</mi></mrow>");
        // Sums as factors are parenthesised, products nest freely.
        shows(&[
            ("(* 2 (+ a b) c)", "2\u{d7}(a+b)\u{d7}c"),
            ("(* a (* b c))", "a\u{d7}b\u{d7}c"),
            ("(* (- a b) c)", "(a\u{2212}b)\u{d7}c"),
            ("(* a (/ b c))", "a\u{d7}bc"),
        ]);
    }

    #[test]
    fn divide() {
        assert_eq!(mathml("(/ 1 t)"), "<mfrac><mn>1</mn><mi>t</mi></mfrac>");
        assert_eq!(
            mathml("(/ (+ a b) c)"),
            "<mfrac><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mi>c</mi></mfrac>"
        );
        // One argument is a reciprocal.
        assert_eq!(mathml("(div x)"), "<mfrac><mn>1</mn><mi>x</mi></mfrac>");
        // Three or more are a left-grouping `÷` row.
        assert_eq!(
            mathml("(/ a b c)"),
            "<mrow><mi>a</mi><mo>\u{f7}</mo><mi>b</mi><mo>\u{f7}</mo><mi>c</mi></mrow>"
        );
        assert_eq!(mathml("/"), "<mo>\u{f7}</mo>");
        assert_eq!(mathml("(/)"), "<mo>\u{f7}</mo>");
        shows(&[
            ("(/ a (/ b c) d)", "a\u{f7}bc\u{f7}d"),
            ("(/ a b (* c d))", "a\u{f7}b\u{f7}(c\u{d7}d)"),
            ("(/ (/ a b c) d e)", "a\u{f7}b\u{f7}c\u{f7}d\u{f7}e"),
            ("(/ a (/ b c d) e)", "a\u{f7}(b\u{f7}c\u{f7}d)\u{f7}e"),
        ]);
    }

    #[test]
    fn power() {
        assert_eq!(mathml("(^ x 2)"), "<msup><mi>x</mi><mn>2</mn></msup>");
        assert_eq!(mathml("(pow e (- x))"), mathml("(^ e (- x))"));
        assert_eq!(
            mathml("(^ (+ a b) n)"),
            "<msup><mrow><mo>(</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>)</mo></mrow><mi>n</mi></msup>"
        );
        shows(&[
            ("(^ (* a b) 2)", "(a\u{d7}b)2"),
            ("(^ (^ x 2) 3)", "(x2)3"),
            ("(^ (/ a b) 2)", "(ab)2"),
            ("(^ x (+ n 1))", "xn+1"),
            ("(+ (^ x 2) 1)", "x2+1"),
            ("(* 2 (^ x 2))", "2\u{d7}x2"),
        ]);
        let (_, diags) = render("(^ x)");
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn roots() {
        assert_eq!(mathml("(sqrt x)"), "<msqrt><mi>x</mi></msqrt>");
        assert_eq!(
            mathml("(sqrt (+ a b))"),
            "<msqrt><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow></msqrt>"
        );
        // The radicand comes first in MathML, the index second.
        assert_eq!(mathml("(root 3 x)"), "<mroot><mi>x</mi><mn>3</mn></mroot>");
        assert_eq!(
            mathml("(root n (+ a b))"),
            "<mroot><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mi>n</mi></mroot>"
        );
        // A root is an atom: no parentheses as a power base.
        assert_eq!(
            mathml("(^ (sqrt x) 2)"),
            "<msup><msqrt><mi>x</mi></msqrt><mn>2</mn></msup>"
        );
    }

    #[test]
    fn jux() {
        assert_eq!(
            mathml("(jux a b c)"),
            "<mrow><mi>a</mi><mspace width=\"0.167em\"/><mi>b</mi><mspace width=\"0.167em\"/><mi>c</mi></mrow>"
        );
        assert_eq!(mathml("(juxtapose x y)"), mathml("(jux x y)"));
        // Atoms and lists mix; a sum is parenthesised.
        shows(&[
            ("(jux 2 (+ a b))", "2 (a+b)"),
            ("(jux f (paren x))", "f (x)"),
            ("(jux (frac a b) c)", "ab c"),
            ("(jux a (jux b c))", "a b c"),
            ("(jux (* a b) c)", "a\u{d7}b c"),
        ]);
        assert_eq!(mathml("(jux x)"), "<mrow><mi>x</mi></mrow>");
    }

    #[test]
    fn equality_and_order() {
        assert_eq!(mathml("(= a b)"), "<mrow><mi>a</mi><mo>=</mo><mi>b</mi></mrow>");
        assert_eq!(
            mathml("(= a b c)"),
            "<mrow><mi>a</mi><mo>=</mo><mi>b</mi><mo>=</mo><mi>c</mi></mrow>"
        );
        assert_eq!(mathml("(eq x y)"), mathml("(= x y)"));
        assert_eq!(mathml("="), "<mo>=</mo>");
        assert_eq!(mathml("(=)"), "<mo>=</mo>");
        assert_eq!(mathml("(!= a b)"), "<mrow><mi>a</mi><mo>\u{2260}</mo><mi>b</mi></mrow>");
        assert_eq!(mathml("(neq x 0)"), mathml("(!= x 0)"));
        assert_eq!(mathml("(< a b)"), "<mrow><mi>a</mi><mo>&lt;</mo><mi>b</mi></mrow>");
        assert_eq!(mathml("(lt 0 x 1)"), mathml("(< 0 x 1)"));
        assert_eq!(mathml("(> a b)"), "<mrow><mi>a</mi><mo>&gt;</mo><mi>b</mi></mrow>");
        assert_eq!(mathml("(gt x 0)"), mathml("(> x 0)"));
        assert_eq!(mathml("(<= a b)"), "<mrow><mi>a</mi><mo>\u{2264}</mo><mi>b</mi></mrow>");
        assert_eq!(mathml("(leq 0 x 1)"), mathml("(<= 0 x 1)"));
        assert_eq!(mathml("(>= a 0)"), "<mrow><mi>a</mi><mo>\u{2265}</mo><mn>0</mn></mrow>");
        assert_eq!(mathml("(geq x y z)"), mathml("(>= x y z)"));
        assert_eq!(mathml("(approx a b)"), "<mrow><mi>a</mi><mo>\u{2248}</mo><mi>b</mi></mrow>");
        assert_eq!(mathml("(\u{2248} a b)"), mathml("(approx a b)"));
        // Bare signs.
        assert_eq!(mathml("!="), "<mo>\u{2260}</mo>");
        assert_eq!(mathml("neq"), "<mo>\u{2260}</mo>");
        assert_eq!(mathml("<="), "<mo>\u{2264}</mo>");
        assert_eq!(mathml("leq"), "<mo>\u{2264}</mo>");
        assert_eq!(mathml(">="), "<mo>\u{2265}</mo>");
        assert_eq!(mathml("geq"), "<mo>\u{2265}</mo>");
        assert_eq!(mathml("approx"), "<mo>\u{2248}</mo>");
        assert_eq!(mathml("<"), "<mo>&lt;</mo>");
        assert_eq!(mathml(">"), "<mo>&gt;</mo>");
        // Relations nest only with parentheses; arithmetic needs none.
        shows(&[
            ("(= (= a b) c)", "(a=b)=c"),
            ("(= a (+ b c) (* d e))", "a=b+c=d\u{d7}e"),
            ("(>= a 0)", "a\u{2265}0"),
            ("(!= (< a b) c)", "(a<b)\u{2260}c"),
        ]);
        let (_, diags) = render("(!= a)");
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn sets() {
        assert_eq!(
            mathml("(set a b c)"),
            "<mrow><mo>{</mo><mi>a</mi><mo>,</mo><mi>b</mi><mo>,</mo><mi>c</mi><mo>}</mo></mrow>"
        );
        assert_eq!(mathml("(set)"), "<mrow><mo>{</mo><mo>}</mo></mrow>");
        shows(&[
            ("(set 1 (+ x 1))", "{1,x+1}"),
            ("(in x S)", "x\u{2208}S"),
            ("(in a (set 1 2))", "a\u{2208}{1,2}"),
            ("(notin x S)", "x\u{2209}S"),
            ("(notin 0 (set 1 2))", "0\u{2209}{1,2}"),
            ("(subset A B)", "A\u{2286}B"),
            ("(subset (set a) (set a b))", "{a}\u{2286}{a,b}"),
            ("(superset A B)", "A\u{2287}B"),
            ("(superset (set a b) (set a))", "{a,b}\u{2287}{a}"),
            ("(union A B)", "A\u{222a}B"),
            ("(union A B C)", "A\u{222a}B\u{222a}C"),
            ("(intersect A B)", "A\u{2229}B"),
            ("(intersect A (union B C))", "A\u{2229}(B\u{222a}C)"),
            ("(union A (intersect B C))", "A\u{222a}B\u{2229}C"),
            ("(in x (union A B))", "x\u{2208}A\u{222a}B"),
            ("(= (union A B) (union B A))", "A\u{222a}B=B\u{222a}A"),
            ("(subset (in x A) B)", "(x\u{2208}A)\u{2286}B"),
        ]);
        assert_eq!(mathml("(in x S)"), "<mrow><mi>x</mi><mo>\u{2208}</mo><mi>S</mi></mrow>");
        assert_eq!(mathml("(subset A B)"), "<mrow><mi>A</mi><mo>\u{2286}</mo><mi>B</mi></mrow>");
        // Bare signs: the glyph of the bare keyword can differ from the call.
        assert_eq!(mathml("subset"), "<mo>\u{2282}</mo>");
        assert_eq!(mathml("superset"), "<mo>\u{2287}</mo>");
        assert_eq!(mathml("in"), "<mo>\u{2208}</mo>");
        assert_eq!(mathml("notin"), "<mo>\u{2209}</mo>");
        assert_eq!(mathml("union"), "<mo>\u{222a}</mo>");
        assert_eq!(mathml("intersect"), "<mo>\u{2229}</mo>");
        assert_eq!(mathml("(union)"), "<mo>\u{222a}</mo>");
    }

    #[test]
    fn logic() {
        assert_eq!(mathml("(and p q)"), "<mrow><mi>p</mi><mo>\u{2227}</mo><mi>q</mi></mrow>");
        assert_eq!(mathml("(or p q r)").matches("<mo>\u{2228}</mo>").count(), 2);
        assert_eq!(mathml("(not p)"), "<mrow><mo>\u{ac}</mo><mi>p</mi></mrow>");
        assert_eq!(mathml("(implies p q)"), "<mrow><mi>p</mi><mo>\u{27f9}</mo><mi>q</mi></mrow>");
        assert_eq!(mathml("(iff p q)"), "<mrow><mi>p</mi><mo>\u{27fa}</mo><mi>q</mi></mrow>");
        assert_eq!(mathml("and"), "<mo>\u{2227}</mo>");
        assert_eq!(mathml("or"), "<mo>\u{2228}</mo>");
        assert_eq!(mathml("not"), "<mo>\u{ac}</mo>");
        assert_eq!(mathml("implies"), "<mo>\u{27f9}</mo>");
        assert_eq!(mathml("iff"), "<mo>\u{27fa}</mo>");
        assert_eq!(mathml("(and)"), "<mo>\u{2227}</mo>");
    }

    #[test]
    fn logic_precedence() {
        shows(&[
            ("(and (or a b) c)", "(a\u{2228}b)\u{2227}c"),
            ("(or (and a b) c)", "a\u{2227}b\u{2228}c"),
            ("(and a (and b c))", "a\u{2227}b\u{2227}c"),
            ("(or a (or b c))", "a\u{2228}b\u{2228}c"),
            ("(and p (or q r) s)", "p\u{2227}(q\u{2228}r)\u{2227}s"),
            ("(implies (implies a b) c)", "(a\u{27f9}b)\u{27f9}c"),
            ("(implies a (implies b c))", "a\u{27f9}(b\u{27f9}c)"),
            ("(iff (implies a b) c)", "(a\u{27f9}b)\u{27fa}c"),
            ("(implies (and a b) (or c d))", "a\u{2227}b\u{27f9}c\u{2228}d"),
            ("(implies (or a b) (and c d))", "a\u{2228}b\u{27f9}c\u{2227}d"),
            ("(not (not p))", "\u{ac}\u{ac}p"),
            ("(not (and p q))", "\u{ac}(p\u{2227}q)"),
            ("(not (= x y))", "\u{ac}x=y"),
            ("(= (not a) b)", "(\u{ac}a)=b"),
            ("(and (not a) b)", "\u{ac}a\u{2227}b"),
            ("(and (= x 0) (< y 1))", "x=0\u{2227}y<1"),
            ("(or (in x A) (in x B))", "x\u{2208}A\u{2228}x\u{2208}B"),
            ("(implies (> x 0) (>= (* x x) 0))", "x>0\u{27f9}x\u{d7}x\u{2265}0"),
            ("(iff (= x 0) (= y 0))", "x=0\u{27fa}y=0"),
        ]);
    }

    #[test]
    fn quantifiers() {
        assert_eq!(
            mathml("(forall x (> x 0))"),
            "<mrow><mo>\u{2200}</mo><mi>x</mi><mo>.</mo><mrow><mi>x</mi><mo>&gt;</mo><mn>0</mn></mrow></mrow>"
        );
        assert_eq!(
            mathml("(exists n ())"),
            "<mrow><mo>\u{2203}</mo><mi>n</mi><mo>.</mo><mrow></mrow></mrow>"
        );
        // The notes' placeholder body renders as nothing.
        assert_eq!(
            mathml("(forall alpha ())"),
            "<mrow><mo>\u{2200}</mo><mi>\u{3b1}</mi><mo>.</mo><mrow></mrow></mrow>"
        );
        assert_eq!(visible("(forall alpha (jux alpha beta))"), "\u{2200}\u{3b1}.\u{3b1} \u{3b2}");
        assert_eq!(mathml("forall"), "<mo>\u{2200}</mo>");
        assert_eq!(mathml("exists"), "<mo>\u{2203}</mo>");
        shows(&[
            // The body is never parenthesised, even a quantifier or implication.
            ("(forall x (implies p q))", "\u{2200}x.p\u{27f9}q"),
            ("(forall x (exists y (= x y)))", "\u{2200}x.\u{2203}y.x=y"),
            ("(exists x (and p q))", "\u{2203}x.p\u{2227}q"),
            // A quantifier as an operand is always parenthesised.
            ("(and (forall x P) Q)", "(\u{2200}x.P)\u{2227}Q"),
            ("(and Q (exists x P))", "Q\u{2227}(\u{2203}x.P)"),
            ("(or (exists x P) Q)", "(\u{2203}x.P)\u{2228}Q"),
            ("(implies (forall x P) Q)", "(\u{2200}x.P)\u{27f9}Q"),
            ("(implies P (exists x Q))", "P\u{27f9}(\u{2203}x.Q)"),
            ("(not (forall x P))", "\u{ac}(\u{2200}x.P)"),
            ("(= (forall x P) Q)", "(\u{2200}x.P)=Q"),
        ]);
    }

    #[test]
    fn grouping() {
        assert_eq!(mathml("(paren x)"), "<mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow>");
        assert_eq!(
            mathml("(paren (frac 1 2))"),
            "<mrow><mo>(</mo><mfrac><mn>1</mn><mn>2</mn></mfrac><mo>)</mo></mrow>"
        );
        // The empty placeholder `()` keeps the fence well formed.
        assert_eq!(mathml("(paren ())"), "<mrow><mo>(</mo><mrow></mrow><mo>)</mo></mrow>");
        assert_eq!(mathml("(bracket x)"), "<mrow><mo>[</mo><mi>x</mi><mo>]</mo></mrow>");
        assert_eq!(
            mathml("(bracket (frac tau alpha))"),
            "<mrow><mo>[</mo><mfrac><mi>\u{3c4}</mi><mi>\u{3b1}</mi></mfrac><mo>]</mo></mrow>"
        );
        assert_eq!(mathml("(brace x)"), "<mrow><mo>{</mo><mi>x</mi><mo>}</mo></mrow>");
        assert_eq!(
            mathml("(brace (+ a b))"),
            "<mrow><mo>{</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>}</mo></mrow>"
        );
        assert_eq!(mathml("(brace ())"), "<mrow><mo>{</mo><mrow></mrow><mo>}</mo></mrow>");
        // Fences are atoms.
        shows(&[("(^ (bracket x) 2)", "[x]2"), ("(* (brace a) b)", "{a}\u{d7}b")]);
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
    fn mixed_precedence() {
        shows(&[
            ("(* (+ a b) c)", "(a+b)\u{d7}c"),
            ("(* a (+ b c))", "a\u{d7}(b+c)"),
            ("(+ (* a b) c)", "a\u{d7}b+c"),
            ("(^ (+ a b) 2)", "(a+b)2"),
            ("(^ (* a b) (+ c d))", "(a\u{d7}b)c+d"),
            ("(+ (^ a 2) (^ b 2))", "a2+b2"),
            ("(= (^ c 2) (+ (^ a 2) (^ b 2)))", "c2=a2+b2"),
            ("(jux (+ a b) (- c d))", "(a+b) (c\u{2212}d)"),
            ("(frac (+ a b) (- c d))", "a+bc\u{2212}d"),
            ("(<= 0 (+ x 1) (* 2 y))", "0\u{2264}x+1\u{2264}2\u{d7}y"),
            ("(forall x (in x (union A B)))", "\u{2200}x.x\u{2208}A\u{222a}B"),
        ]);
    }

    #[test]
    fn real_notes() {
        shows(&[
            ("(= a b c)", "a=b=c"),
            ("(>= a 0)", "a\u{2265}0"),
            ("(* infinity (/ 1 infinity))", "\u{221e}\u{d7}1\u{221e}"),
            ("(- t a)", "t\u{2212}a"),
            ("(- infinity)", "\u{2212}\u{221e}"),
            ("(* -s t)", "\u{2212}s\u{d7}t"),
            ("(jux a b (+ c d) (frac e f))", "a b (c+d) ef"),
            ("(/ 1 t)", "1t"),
            ("(brace (+ a b))", "{a+b}"),
            ("(bracket (frac tau alpha))", "[\u{3c4}\u{3b1}]"),
        ]);
        assert!(mathml("(/ 1 t)").starts_with("<mfrac>"));
    }
}
