use mazer_syntax::{Diag, DiagCode, Diags, Severity};

use crate::testing::read;

/// The diagnostics of `src` from every target, asserted identical.
fn diags_of(src: &str) -> Vec<Diag> {
    let node = read(src);
    let mut mathml = Diags::new();
    let _ = crate::to_mathml(&node, &mut mathml);
    let mut checked = Diags::new();
    crate::check(&node, &mut checked);
    assert_eq!(mathml.as_slice(), checked.as_slice(), "check differs from MathMl for {src}");
    #[cfg(feature = "latex")]
    {
        let mut latex = Diags::new();
        let _ = crate::to_latex(&node, &mut latex);
        assert_eq!(mathml.as_slice(), latex.as_slice(), "LaTeX differs from MathMl for {src}");
    }
    mathml.as_slice().to_vec()
}

#[track_caller]
fn assert_silent(src: &str) {
    assert_eq!(diags_of(src), Vec::<Diag>::new(), "{src}");
}

fn only(src: &str) -> Diag {
    let diags = diags_of(src);
    assert_eq!(diags.len(), 1, "{src}: {diags:?}");
    diags.into_iter().next().unwrap()
}

#[test]
fn old_integral_order_warns_on_the_variable() {
    let src = "(integral 0 1 (f x) x)";
    let d = only(src);
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.code, DiagCode::Other);
    assert_eq!(d.message, "the variable of `integral` should be a name, but this is a number");
    assert_eq!(
        d.help.as_deref(),
        Some("(integral body var [lower] [upper]); the argument order changed: body and variable come first, bounds last")
    );
    // The span is the argument bound to `var`: the second one, `1`.
    assert_eq!(&src[d.span.start.0 as usize..d.span.end.0 as usize], "1");
}

#[test]
fn old_sum_and_prod_order() {
    let d = only("(sum i 1 n (f i))");
    assert_eq!(d.message, "the variable of `sum` should be a name, but this is a number");
    assert!(d.help.as_deref().unwrap().starts_with("(sum body var [lower] [upper]); the argument order changed"));
    let d = only("(prod i 1 n (f i))");
    assert_eq!(d.message, "the variable of `prod` should be a name, but this is a number");
    let d = only("(product i 1 n (f i))");
    assert_eq!(d.message, "the variable of `prod` should be a name, but this is a number");
}

#[test]
fn variable_of_other_operators() {
    let d = only("(lim 3 0 x)");
    assert_eq!(d.message, "the variable of `lim` should be a name, but this is a number");
    assert_eq!(d.help.as_deref(), Some("(lim var approach [body] [side])"));
    assert_eq!(only("(limit \"x\" 0 x)").message, "the variable of `lim` should be a name, but this is a string");
    assert_eq!(only("(derivative y 2)").message, "the variable of `derivative` should be a name, but this is a number");
    assert_eq!(
        only("(deriv y (+ x 1))").message,
        "the variable of `derivative` should be a name, but this is an expression"
    );
    assert_eq!(only("(partial f 1)").message, "the variable of `partial` should be a name, but this is a number");
    assert_eq!(only("(dd y 2)").message, "the variable of `dd` should be a name, but this is a number");
    // No order note outside integral, sum and prod.
    assert!(!only("(dd y 2)").help.unwrap().contains("order changed"));
}

#[test]
fn names_and_decorated_names_are_variables() {
    for src in [
        "(integral (f x) x 0 1)",
        "(integral (f x) 'x)",
        "(sum (f i) i 1 n)",
        "(partial f (subscript x i))",
        "(derivative f (subscript x 1))",
        "(dd y (subscript t 0))",
        "(integral f (superscript x 2))",
        "(integral f (bar x))",
        "(integral f (hat x))",
        "(integral f (vec x y))",
        "(integral f (bold x))",
        "(integral f (subscript (hat x) i))",
        "(lim (subscript x n) infty (f x))",
        "(integral f tau)",
    ] {
        assert_silent(src);
    }
}

#[test]
fn other_shapes_are_not_variables() {
    for (src, what) in [
        ("(integral f \"x\")", "a string"),
        ("(integral f (+ x 1))", "an expression"),
        ("(integral f (f x))", "an expression"),
        ("(integral f ())", "an empty list"),
        ("(integral f (subscript 1 i))", "a number"),
        ("(integral f (quote 3))", "an expression"),
        ("(integral f -2)", "a number"),
    ] {
        let d = only(src);
        assert!(d.message.ends_with(what), "{src}: {}", d.message);
    }
}

#[test]
fn each_occurrence_warns_once() {
    let diags = diags_of("(+ (integral 0 1 (f x) x) (sum i 1 n (f i)))");
    assert_eq!(diags.len(), 2);
}

#[test]
fn removed_keywords_get_a_hint() {
    let src = "(string w)";
    let d = only(src);
    assert_eq!(d.severity, Severity::Hint);
    assert_eq!(d.code, DiagCode::UnknownForm);
    assert_eq!(
        d.message,
        "`string` was removed: write the name directly, e.g. `w`, or `(f x)` for f(x), or `'name` to show a keyword's name"
    );
    assert_eq!(&src[d.span.start.0 as usize..d.span.end.0 as usize], "string");
    assert_eq!(only("(define x 1)").message, "`define` was removed: use `(let name value)`");
    assert_eq!(only("(defunc (f x) x)").message, "`defunc` was removed: use `(macro (name params...) body)`");
    // A bare name and similar names are fine.
    assert_silent("string");
    assert_silent("(strings w)");
    // Did-you-mean still works beside it.
    assert_eq!(only("(fracc a b)").help.as_deref(), Some("did you mean `frac`?"));
}

#[test]
fn ty_judge_single_premise_must_be_wrapped() {
    let d = only("(ty-judge R (= a b) c)");
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.code, DiagCode::Other);
    assert_eq!(
        d.message,
        "premises must be a parenthesised list of premises; wrap this single premise: `((= a b))`"
    );
    assert_eq!(only("(ty-judgement R (in x S) c)").severity, Severity::Warning);
    // An alias as the head counts too.
    assert_eq!(only("(ty-judge R (leq a b) c)").severity, Severity::Warning);
    for src in [
        "(ty-judge R ((= a b)) c)",
        "(ty-judge R ((= a b) (= b c)) (= a c))",
        "(ty-judge R (x y) c)",
        "(ty-judge R x c)",
        "(ty-judge R () c)",
    ] {
        assert_silent(src);
    }
}

#[test]
fn table_directive_typos_warn() {
    let d = only("(table (rwo a b))");
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.code, DiagCode::Other);
    assert_eq!(d.message, "`rwo` is not a table directive, so this is a row whose first cell is `rwo`");
    assert_eq!(d.help.as_deref(), Some("did you mean `(row ...)`?"));
    assert_eq!(only("(table (heder a b))").help.as_deref(), Some("did you mean `(header ...)`?"));
    assert_eq!(only("(table (algn left))").help.as_deref(), Some("did you mean `(align ...)`?"));
    assert_eq!(only("(table (stile clean))").help.as_deref(), Some("did you mean `(style ...)`?"));
    assert_eq!(only("(table (foo a b) (rows 1 2))").help.as_deref(), Some("did you mean `(row ...)`?"));
    for src in [
        "(table (foo a b))",
        "(table (a b) (c d))",
        "(table (row a b) (header c d) (align left) (style clean))",
        "(table (+ a b) (1 2))",
        "(table x y)",
    ] {
        assert_silent(src);
    }
    // An unknown style value still warns on its own, and only once.
    assert_eq!(only("(table (style foo))").message, "`style` expects `grid`, `clean` or `plain`");
}
