use std::fmt::Write as _;

use mazer_syntax::{DiagCode, Span};

use super::*;
use crate::pipeline::analyze;
use crate::testing::{clean, codes, read, resolved_forms};

#[test]
fn the_architecture_example() {
    let src = "(let x 2)\n\n(show x)\n\n(show (+ x 1))\n\n(eval (+ x 1))\n\n(let y (+ x 1))\n\n(show (+ 'x x))\n\n(show y)";
    let analysis = clean(src);
    assert_eq!(
        resolved_forms(&analysis),
        [
            "(let x 2)",
            "(show 2)",
            "(show (+ 2 1))",
            "(eval (+ 2 1))",
            "(let y (+ 2 1))",
            "(show (+ (quote x) 2))",
            "(show (+ 2 1))",
        ]
    );
    let lets: Vec<String> = analysis.lets.iter().map(|b| format!("{} = {}", b.name, b.value)).collect();
    assert_eq!(lets, ["x = 2", "y = (+ 2 1)"]);
    assert_eq!(analysis.sources.text(analysis.lets[1].span), "(let y (+ x 1))");
}

#[test]
fn a_variable_is_defined_once() {
    let src = "(let x 2)\n\n(let x 3)\n\n(show x)";
    let analysis = analyze(src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::DuplicateDefinition]);
    let diag = &analysis.diags.as_slice()[0];
    assert_eq!(diag.message, "`x` is already defined");
    assert_eq!(diag.span.start.to_usize(), src.rfind("x 3").unwrap());
    assert_eq!(analysis.sources.text(diag.span), "x");
    assert_eq!(diag.labels.len(), 1);
    assert_eq!(diag.labels[0].0.start.to_usize(), 5);
    assert_eq!(analysis.sources.text(diag.labels[0].0), "x");
    assert_eq!(diag.labels[0].1, "first defined here");
    assert_eq!(diag.help.as_deref(), Some("a variable can be defined only once; choose another name"));
    // The second let binds nothing: the first value stays in effect.
    assert_eq!(resolved_forms(&analysis)[2], "(show 2)");
    let lets: Vec<String> = analysis.lets.iter().map(|b| format!("{} = {}", b.name, b.value)).collect();
    assert_eq!(lets, ["x = 2"]);
    // Inline forms count too.
    let analysis = analyze("A (let x 2) then (show x).\n\n- (let x 3) and (show x)", "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::DuplicateDefinition]);
    assert_eq!(resolved_forms(&analysis)[3], "(show 2)");
}

#[test]
fn every_further_definition_points_at_the_first() {
    let src = "(let x 1)\n\n(let x 2)\n\n(let x 3)\n\n(show x)";
    let analysis = analyze(src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::DuplicateDefinition, DiagCode::DuplicateDefinition]);
    for diag in analysis.diags.as_slice() {
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.labels[0].0.start.to_usize(), 5);
    }
    assert_eq!(resolved_forms(&analysis)[3], "(show 1)");
    assert_eq!(analysis.lets.len(), 1);
}

#[test]
fn a_rejected_let_defines_nothing() {
    // Malformed, then valid.
    let analysis = analyze("(let x)\n\n(let x 2)\n\n(show x)", "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::BadDefinition]);
    assert_eq!(resolved_forms(&analysis)[2], "(show 2)");
    assert_eq!(analysis.lets.len(), 1);
    // A value too large, then valid.
    let mut src = String::from("(let x0 a)");
    for i in 1..=20 {
        write!(src, "\n\n(let x{i} (+ x{} x{}))", i - 1, i - 1).unwrap();
    }
    let analysis = analyze(&src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::BadDefinition]);
    // The refused let is the first one whose name is not bound.
    let refused = (0..=20).find(|i| !analysis.lets.iter().any(|b| b.name == Sym::new(&format!("x{i}")))).unwrap();
    write!(src, "\n\n(let x{refused} b)\n\n(show x{refused})").unwrap();
    let analysis = analyze(&src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::BadDefinition]);
    assert_eq!(resolved_forms(&analysis).last().unwrap(), "(show b)");
}

#[test]
fn a_let_refused_by_the_budget_counts_as_defined() {
    let value = format!("(+{})", " a".repeat(999));
    let mut src = format!("(let big {value})");
    for i in 0..300 {
        write!(src, "\n\n(let c{i} big)").unwrap();
    }
    src.push_str("\n\n(let c299 2)");
    let analysis = analyze(&src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::Other, DiagCode::DuplicateDefinition]);
    let last = analysis.lets.iter().rfind(|b| b.name == Sym::new("c299")).unwrap();
    assert!(matches!(last.value.kind, NodeKind::Error));
}

#[test]
fn a_rejected_let_is_an_error_box() {
    let html = crate::testing::body("(let x 2)\n\n(let x 3)\n\nInline (let x 4) too.");
    assert_eq!(html.matches("<pre class=\"mazer-error\">").count(), 1, "{html}");
    assert_eq!(html.matches("<span class=\"mazer-error\">").count(), 1, "{html}");
    assert!(html.contains("already defined"), "{html}");
}

#[test]
fn a_first_let_may_use_its_own_name() {
    // With nothing bound yet, the name stays free.
    let analysis = clean("(let x (+ x 1))\n\n(show x)");
    assert_eq!(resolved_forms(&analysis)[1], "(show (+ x 1))");
}

#[test]
fn takes_effect_from_where_it_is_written() {
    let analysis = clean("(show x)\n\n(let x 2)\n\n(show x)");
    assert_eq!(resolved_forms(&analysis), ["(show x)", "(let x 2)", "(show 2)"]);
    // Inline forms count too, in document order.
    let analysis = clean("A (let x 2) then (show x).\n\n- (let y 3) and (show (+ x y))");
    assert_eq!(resolved_forms(&analysis), ["(let x 2)", "(show 2)", "(let y 3)", "(show (+ 2 3))"]);
}

#[test]
fn quote_is_never_substituted() {
    let analysis = clean("(let x 2)\n\n(show (+ (quote x) 'x x))\n\n(show '(+ x 1))");
    assert_eq!(resolved_forms(&analysis)[1], "(show (+ (quote x) (quote x) 2))");
    assert_eq!(resolved_forms(&analysis)[2], "(show (quote (+ x 1)))");
}

#[test]
fn the_head_of_a_list_is_substituted() {
    let analysis = clean("(let f g)\n\n(show (f x))");
    assert_eq!(resolved_forms(&analysis)[1], "(show (g x))");
    // A keyword as the value is checked where it is used, not at the let.
    let analysis = clean("(let f frac)\n\n(show (f a b))");
    assert_eq!(resolved_forms(&analysis)[1], "(show (frac a b))");
}

#[test]
fn storing_a_value_costs_nothing_using_it_does() {
    // 300 values of 1 000 nodes each: more than the document budget, but
    // nothing is substituted, so nothing is created.
    let value = format!("(+{})", " a".repeat(999));
    let mut src = String::new();
    for i in 0..300 {
        write!(src, "(let x{i} {value})\n\n").unwrap();
    }
    clean(&src);
    // Copying the value 300 times is over the budget: one diagnostic, at the
    // use that crossed it.
    let mut src = format!("(let big {value})");
    for i in 0..300 {
        write!(src, "\n\n(let c{i} big)").unwrap();
    }
    let analysis = analyze(&src, "t.zr");
    assert_eq!(codes_of(&analysis), [DiagCode::Other]);
    assert_eq!(analysis.sources.text(analysis.diags.as_slice()[0].span), "big");
    let bound = analysis.lets.iter().filter(|b| !matches!(b.value.kind, NodeKind::Error)).count();
    assert!((250..300).contains(&bound), "{bound} lets bound before the budget ran out");
    // The refused ones are recorded too, as errors, so editors see them.
    assert_eq!(analysis.lets.len(), 301);
}

#[test]
fn display_names_may_be_bound() {
    let analysis = clean("(let tau 2)\n\n(show tau)");
    assert_eq!(resolved_forms(&analysis)[1], "(show 2)");
}

#[test]
fn builtin_names_may_not_be_bound() {
    for src in ["(let sin 2)", "(let add 2)", "(let quote 2)", "(let show 2)", "(let ref 2)", "(let tag 2)"] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::ShadowsBuiltin], "{src}");
        assert_eq!(analysis.lets, []);
    }
}

#[test]
fn notation_names_may_not_be_bound() {
    let analysis = analyze("(alias R (paren R))\n\n(let R 2)", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::DuplicateDefinition]);
    assert_eq!(analysis.diags.as_slice()[0].labels.len(), 1);
}

#[test]
fn malformed_lets() {
    for src in ["(let x)", "(let)", "(let 1 2)", "(let x 1 2)", "(let (x) 2)"] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::BadDefinition], "{src}");
        assert_eq!(analysis.lets, [], "{src}");
    }
    // Unclosed: only the reader's diagnostic.
    let analysis = analyze("(let x (+ 1 2)", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::UnclosedParen]);
}

#[test]
fn macros_are_expanded_before_substitution() {
    let analysis = clean("(macro (half v) (frac v 2))\n\n(let x (half a))\n\n(show (half x))");
    assert_eq!(resolved_forms(&analysis)[2], "(show (frac (frac a 2) 2))");
}

#[test]
fn substituted_nodes_take_the_use_site_span() {
    let src = "(let x (+ a 1))\n\n(show (frac x 2))";
    let analysis = clean(src);
    let mut shows = Vec::new();
    crate::walk::for_each_code(&analysis.resolved, &mut |form, _| {
        if form.is_form("show") {
            shows.push(form.clone());
        }
    });
    let frac = &shows[0].args()[0];
    let x = &frac.as_list().unwrap()[1];
    assert_eq!(x.to_string(), "(+ a 1)");
    assert_eq!(analysis.sources.text(x.span), "x");
    for node in x.as_list().unwrap() {
        assert_eq!(node.span, x.span);
    }
}

#[test]
fn substitute_directly() {
    let mut env = Bindings::new();
    assert!(env.is_empty());
    env.insert(Binding {
        name: Sym::new("x"),
        value: read("(+ a 1)"),
        span: Span::DUMMY,
    });
    let out = substitute(&read("(frac x 'x)"), &env);
    assert_eq!(out.to_string(), "(frac (+ a 1) (quote x))");
    // Unbound names are left alone.
    assert_eq!(substitute(&read("(f y)"), &env).to_string(), "(f y)");
    assert_eq!(env.iter().count(), 1);
}

#[test]
fn oversized_substitution_is_refused() {
    // Each let doubles the value: 2^40 nodes if nothing stopped it.
    let mut src = String::from("(let x0 a)");
    for i in 1..=40 {
        write!(src, "\n\n(let x{i} (+ x{} x{}))", i - 1, i - 1).unwrap();
    }
    src.push_str("\n\n(show x40)");
    let analysis = analyze(&src, "t.zr");
    let codes = codes(analysis.diags.as_slice());
    assert!(codes.contains(&DiagCode::BadDefinition), "{codes:?}");
    // Deep chains are refused too.
    let mut src = String::from("(let x0 a)");
    for i in 1..=400 {
        write!(src, "\n\n(let x{i} (paren x{}))", i - 1).unwrap();
    }
    let analysis = analyze(&src, "t.zr");
    assert!(codes_of(&analysis).contains(&DiagCode::BadDefinition));
}

fn codes_of(analysis: &crate::Analysis) -> Vec<DiagCode> {
    codes(analysis.diags.as_slice())
}
