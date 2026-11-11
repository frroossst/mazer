use std::fmt::Write as _;

use mazer_syntax::{DiagCode, Diags, NodeKind, Span};

use super::*;
use crate::pipeline::analyze;
use crate::testing::{codes, parse, read, resolved_forms};

/// Collects the definitions in `defs`, asserting there is nothing wrong
/// with them.
fn defs(src: &str) -> Macros {
    let (doc, mut diags) = parse(src);
    let mut macros = Macros::new();
    collect_macros(&doc, &mut macros, &mut diags);
    assert!(diags.is_empty(), "{diags:?}");
    macros
}

/// The diagnostics collecting the definitions in `src` gives.
fn def_codes(src: &str) -> Vec<DiagCode> {
    let (doc, mut diags) = parse(src);
    let mut macros = Macros::new();
    collect_macros(&doc, &mut macros, &mut diags);
    codes(diags.as_slice())
}

/// Expands `expr` with the definitions in `src`; returns the result as text.
fn exp(src: &str, expr: &str) -> (String, Diags) {
    let macros = defs(src);
    let mut diags = Diags::new();
    let out = expand(&read(expr), &macros, &mut diags);
    (out.to_string(), diags)
}

/// Like [`exp`], asserting no diagnostics.
fn ok(src: &str, expr: &str) -> String {
    let (out, diags) = exp(src, expr);
    assert!(diags.is_empty(), "{expr}: {diags:?}");
    out
}

const R: &str = "(alias R (paren R))";
const HALF: &str = "(macro (half x) \"Half of x.\" (frac x 2))";

#[test]
fn alias() {
    assert_eq!(ok(R, "R"), "(paren R)");
    assert_eq!(ok(R, "(+ R 1)"), "(+ (paren R) 1)");
    // Used as a head, an alias is still a bare name.
    assert_eq!(ok(R, "(R x)"), "((paren R) x)");
    assert!(defs(R).get(Sym::new("R")).is_some_and(Macro::is_alias));
}

#[test]
fn macro_with_parameters() {
    assert_eq!(ok(HALF, "(half (+ a b))"), "(frac (+ a b) 2)");
    let macros = defs(HALF);
    let half = macros.get(Sym::new("half")).unwrap();
    assert_eq!(half.params, vec![Sym::new("x")]);
    assert_eq!(half.rest, None);
    assert!(!half.is_alias());
    // A zero-parameter macro is an alias.
    assert!(defs("(macro (two) 2)").get(Sym::new("two")).is_some_and(Macro::is_alias));
    assert_eq!(ok("(macro (two) 2)", "(+ two two)"), "(+ 2 2)");
}

#[test]
fn doc_string() {
    let macros = defs(HALF);
    assert_eq!(macros.get(Sym::new("half")).unwrap().doc.as_deref(), Some("Half of x."));
    // Without a doc string.
    let macros = defs("(macro (half x) (frac x 2))");
    assert_eq!(macros.get(Sym::new("half")).unwrap().doc, None);
    // A signature and a string only: the string is the body.
    let macros = defs("(macro (hello) \"hi\")");
    let hello = macros.get(Sym::new("hello")).unwrap();
    assert_eq!(hello.doc, None);
    assert!(matches!(hello.body.kind, NodeKind::Str(_)));
}

#[test]
fn rest_parameter() {
    let src = "(macro (total first more...) (+ first more))";
    let macros = defs(src);
    let total = macros.get(Sym::new("total")).unwrap();
    assert_eq!(total.params, vec![Sym::new("first")]);
    assert_eq!(total.rest, Some(Sym::new("more")));
    assert_eq!(ok(src, "(total a b c)"), "(+ a b c)");
    assert_eq!(ok(src, "(total a)"), "(+ a)");
    // The rest parameter splices wherever it stands in a list.
    assert_eq!(ok("(macro (wrap xs...) (paren (+ xs 1)))", "(wrap a b)"), "(paren (+ a b 1))");
    let (out, diags) = exp(src, "(total)");
    assert_eq!(out, "<error>");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroArity]);
    assert!(diags.as_slice()[0].message.contains("at least 1"), "{diags:?}");
}

#[test]
fn nested_uses() {
    let src = format!("{R}\n\n{HALF}\n\n(macro (quarter x) (half (half x)))");
    // A macro used in an argument.
    assert_eq!(ok(&src, "(half R)"), "(frac (paren R) 2)");
    assert_eq!(ok(&src, "(half (half a))"), "(frac (frac a 2) 2)");
    // A macro whose template uses another macro.
    assert_eq!(ok(&src, "(quarter a)"), "(frac (frac a 2) 2)");
    // An argument is expanded once, not again inside the template.
    assert_eq!(ok("(macro (same x) x)\n\n(alias S (paren S))", "(same S)"), "(paren S)");
}

#[test]
fn self_reference_is_not_expanded_again() {
    assert_eq!(ok(R, "R"), "(paren R)");
    assert_eq!(ok("(macro (f x) (f (paren x)))", "(f a)"), "(f (paren a))");
    // Another name using it: the inner use expands once.
    assert_eq!(ok(&format!("{R}\n\n(alias S (+ R S))"), "S"), "(+ (paren R) S)");
}

#[test]
fn quote_contents_are_expanded() {
    assert_eq!(ok(R, "'R"), "(quote (paren R))");
}

#[test]
fn bare_use_of_a_macro_with_parameters_is_left_alone() {
    assert_eq!(ok(HALF, "(+ half 1)"), "(+ half 1)");
}

#[test]
fn mutual_recursion_hits_the_depth_cap() {
    let src = "(alias A (paren B))\n\n(alias B (paren A))";
    let (out, diags) = exp(src, "(+ A A)");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroDepth], "one diagnostic for the whole tree");
    assert!(out.contains("<error>"), "{out}");
    let diag = &diags.as_slice()[0];
    assert_eq!(diag.labels.len(), 1, "points at the definition");

    // Fan-out does not make it exponential.
    let src = "(alias A (+ B B B))\n\n(alias B (+ A A A))";
    let (_, diags) = exp(src, "A");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroDepth]);

    let src = "(macro (f x) (g x))\n\n(macro (g x) (f x))";
    let (_, diags) = exp(src, "(f 1)");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroDepth]);
}

#[test]
fn oversized_expansion_is_refused() {
    // Each level multiplies the size by 8: far past the node budget.
    let mut src = String::from("(alias L0 x)");
    for i in 1..10 {
        let p = i - 1;
        write!(src, "\n\n(alias L{i} (+ L{p} L{p} L{p} L{p} L{p} L{p} L{p} L{p}))").unwrap();
    }
    let (out, diags) = exp(&src, "L9");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroDepth]);
    assert!(out.contains("<error>"));
}

#[test]
fn wrong_argument_count() {
    let (out, diags) = exp(HALF, "(+ (half a b) 1)");
    assert_eq!(out, "(+ <error> 1)");
    assert_eq!(codes(diags.as_slice()), [DiagCode::MacroArity]);
    let diag = &diags.as_slice()[0];
    assert!(diag.message.contains("takes 1 argument, found 2"), "{}", diag.message);
    assert_eq!(diag.help.as_deref(), Some("write it as (half x)"));
}

#[test]
fn bad_use_points_at_the_definition() {
    let src = format!("{HALF}\n\n(show (half a b))");
    let analysis = analyze(&src, "t.zr");
    let diag = analysis.diags.iter().find(|d| d.code == DiagCode::MacroArity).expect("macro arity");
    let def = analysis.macros.get(Sym::new("half")).unwrap();
    assert_eq!(diag.labels, vec![(def.span, "defined here".to_owned())]);
    assert_eq!(analysis.sources.text(def.span), HALF);
    assert_eq!(analysis.sources.text(diag.span), "(half a b)");
}

#[test]
fn duplicates() {
    assert_eq!(def_codes("(alias R x)\n\n(alias R y)"), [DiagCode::DuplicateDefinition]);
    assert_eq!(def_codes("(alias R x)\n\n(macro (R a) a)"), [DiagCode::DuplicateDefinition]);
    let (doc, mut diags) = parse("(alias R x)\n\n(alias R y)");
    let mut macros = Macros::new();
    collect_macros(&doc, &mut macros, &mut diags);
    let diag = &diags.as_slice()[0];
    assert_eq!(diag.labels[0].0, Span::new(mazer_syntax::FileId(0), mazer_syntax::ByteOffset(0), mazer_syntax::ByteOffset(11)));
    // The first definition is kept.
    assert_eq!(macros.get(Sym::new("R")).unwrap().body.to_string(), "x");
}

#[test]
fn shadowing_builtins() {
    for src in [
        "(alias sin x)",             // keyword
        "(alias add x)",             // a keyword's second name
        "(macro (frac a b) a)",      // keyword, as a macro
        "(alias tau t)",             // display name
        "(alias show x)",            // reserved head
        "(alias ref x)",             // reserved head
        "(alias tag x)",             // reserved word
        "(alias quote x)",           // keyword
    ] {
        assert_eq!(def_codes(src), [DiagCode::ShadowsBuiltin], "{src}");
    }
}

#[test]
fn malformed_definitions() {
    for src in [
        "(alias R)",
        "(alias R x y)",
        "(alias (R) x)",
        "(macro half x)",
        "(macro (half x))",
        "(macro (half 1) x)",
        "(macro (half x) 3 (frac x 2))",
        "(macro (f x x) x)",
        "(macro (f a... b) a)",
        "(macro (f a...) a)",
        "(macro () a)",
    ] {
        assert_eq!(def_codes(src), [DiagCode::BadDefinition], "{src}");
    }
    // An unclosed definition is the reader's problem only.
    assert_eq!(def_codes("(alias R (paren R)"), [DiagCode::UnclosedParen]);
}

#[test]
fn definitions_anywhere_in_the_document() {
    let src = "- item (alias A a)\n\n> (alias B b)\n\ntext\n(macro (c x) x)\n\n# Heading (alias D d)";
    let macros = defs(src);
    for name in ["A", "B", "c", "D"] {
        assert!(macros.get(Sym::new(name)).is_some(), "{name}");
    }
    assert_eq!(macros.len(), 4);
}

#[test]
fn definition_order_does_not_matter() {
    let analysis = analyze("(show (half a))\n\n(macro (half x) (frac x 2))", "t.zr");
    assert!(analysis.diags.is_empty(), "{:?}", analysis.diags);
    assert_eq!(resolved_forms(&analysis)[0], "(show (frac a 2))");
}

#[test]
fn expanded_nodes_take_the_use_site_span() {
    let src = format!("{HALF}\n\n(show (half (+ a b)))");
    let analysis = analyze(&src, "t.zr");
    assert!(analysis.diags.is_empty(), "{:?}", analysis.diags);
    let mut shows = Vec::new();
    crate::walk::for_each_code(&analysis.resolved, &mut |form, _| {
        if form.is_form("show") {
            shows.push(form.clone());
        }
    });
    let expr = &shows[0].args()[0];
    assert_eq!(analysis.sources.text(expr.span), "(half (+ a b))");
    let items = expr.as_list().unwrap();
    // `frac` and `2` come from the template: they take the use site.
    assert_eq!(items[0].span, expr.span);
    assert_eq!(items[2].span, expr.span);
    // The argument keeps its own span.
    assert_eq!(analysis.sources.text(items[1].span), "(+ a b)");
    assert_eq!(analysis.sources.text(items[1].as_list().unwrap()[1].span), "a");
}

// ------------------------------------------- diagnostics inside expansions

/// A diagnostic as (code, primary text, labels as (text, label), help).
type Report = (DiagCode, String, Vec<(String, String)>, Option<String>);

/// Every diagnostic `analyze` reports for `src`.
fn reports(src: &str) -> Vec<Report> {
    let analysis = analyze(src, "t.zr");
    let text = |span: Span| analysis.sources.text(span).to_owned();
    analysis
        .diags
        .iter()
        .map(|d| {
            let labels = d.labels.iter().map(|(s, l)| (text(*s), l.clone())).collect();
            (d.code, text(d.span), labels, d.help.clone())
        })
        .collect()
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_owned(), b.to_owned())
}

fn assert_real_spans(node: &Node) {
    assert_eq!(node.span.file, mazer_syntax::FileId(0), "{node}");
    for child in node.as_list().unwrap_or(&[]) {
        assert_real_spans(child);
    }
}

#[test]
fn a_problem_in_a_template_points_at_the_template() {
    let src = "(macro (tuple first rest...) (paren first rest))\n\n(show (tuple a b c d))";
    let got = reports(src);
    assert_eq!(got.len(), 1, "{got:?}");
    let (code, primary, labels, help) = &got[0];
    assert_eq!(*code, DiagCode::Arity);
    assert_eq!(primary, "(tuple a b c d)", "the primary span stays at the use");
    assert_eq!(
        labels,
        &[
            pair("b", "unexpected argument"),
            pair("c", "unexpected argument"),
            pair("d", "unexpected argument"),
            pair("(paren first rest)", "in the expansion of `tuple`, defined here"),
        ]
    );
    assert_eq!(help.as_deref(), Some("the template of `tuple` calls `paren` with 4 arguments; (paren body)"));
    // The label is inside the definition, not just anywhere with that text.
    let analysis = analyze(src, "t.zr");
    let def = analysis.macros.get(Sym::new("tuple")).unwrap();
    let template = analysis.diags.as_slice()[0].labels[3].0;
    assert!(def.span.start <= template.start && template.end <= def.span.end);
    // The resolved tree still carries only real use-site spans, and the use
    // renders its `<merror>` without an error box.
    crate::walk::for_each_code(&analysis.resolved, &mut |form, _| assert_real_spans(form));
    let html = crate::testing::body(src);
    assert!(html.contains("<merror>") && !html.contains("mazer-error"), "{html}");
}

#[test]
fn a_closed_alias_is_reported_once_at_its_definition() {
    let src = "(alias H (frac h))\n\n(show H)\n\n(show (+ H H))\n\nText (show H).";
    let got = reports(src);
    assert_eq!(got, [(DiagCode::Arity, "(frac h)".to_owned(), vec![], Some("(frac num den)".to_owned()))]);
    // The uses still render the problem in place; the broken definition is
    // an error box, like a broken `let`.
    let html = crate::testing::body(src);
    assert_eq!(html.matches("<merror>").count(), 4, "{html}");
    assert!(html.starts_with("<pre class=\"mazer-error\">`frac` is missing its `den` argument</pre>"), "{html}");
    // A macro whose body is wrong whatever its arguments: the same.
    let got = reports("(macro (f v) (+ v (frac 1)))\n\n(show (f a))\n\n(show (f b))");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].1, "(frac 1)");
    // A macro use with the wrong count, inside a closed template.
    let got = reports("(macro (half v) (frac v 2))\n\n(alias H (half))\n\n(show H)");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!((got[0].0, got[0].1.as_str()), (DiagCode::MacroArity, "(half)"));
    // A single name is not checked on its own: it may be a head.
    assert_eq!(reports("(alias F frac)\n\n(show (F a b))").len(), 0);
    // An unused closed definition is still reported.
    assert_eq!(reports("(alias U (sqrt))").len(), 1);
}

#[test]
fn nested_expansions_name_every_definition() {
    // The faulty call is in `inner`'s template and depends on the arguments
    // `outer` passes: reported at the use, labelled at both templates.
    let src = "(macro (inner x rest...) (paren x rest))\n\n(macro (outer a) (+ 1 (inner a b c)))\n\n(show (outer q))";
    let got = reports(src);
    assert_eq!(got.len(), 1, "{got:?}");
    let (code, primary, labels, help) = &got[0];
    assert_eq!(*code, DiagCode::Arity);
    assert_eq!(primary, "(outer q)");
    assert_eq!(
        labels,
        &[
            pair("(paren x rest)", "in the expansion of `inner`, defined here"),
            pair("(inner a b c)", "`inner` is used in the expansion of `outer` here"),
        ]
    );
    assert_eq!(help.as_deref(), Some("the template of `inner` calls `paren` with 3 arguments; (paren body)"));
    // A closed problem of an inner definition is reported at that definition
    // only, not again at the definition that uses it, nor at the uses.
    let got = reports("(alias A (paren 1 2))\n\n(macro (b) (+ A x))\n\n(show (b))\n\n(show A)");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].1, "(paren 1 2)");
    // Closed for the outer definition, through the inner one: reported at
    // the outer definition, labelled at the inner template.
    let got = reports("(macro (inner x rest...) (paren x rest))\n\n(alias O (inner 1 2))\n\n(show O)");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].1, "(inner 1 2)");
    assert_eq!(
        got[0].2,
        [
            pair("2", "unexpected argument"),
            pair("(paren x rest)", "in the expansion of `inner`, defined here")
        ]
    );
    assert_eq!(
        got[0].3.as_deref(),
        Some("the template of `inner` calls `paren` with 2 arguments; (paren body)")
    );
}

#[test]
fn sound_definitions_report_nothing() {
    let src = "(macro (tuple first rest...) (paren (jux first rest)))\n\n(alias R (bb R))\n\n\
               (macro (lap f s) \"doc\" (jux L (brace f) (paren s)))\n\n(show (tuple a b c))\n\n(show (lap R x))";
    assert!(reports(src).is_empty(), "{:?}", reports(src));
    // Recursion through each other is reported at each definition, and not
    // again at a use, which is an error box.
    let src = "(alias A (paren B))\n\n(alias B (paren A))\n\n(show A)";
    let got = reports(src);
    assert_eq!(got.iter().map(|r| r.0).collect::<Vec<_>>(), [DiagCode::MacroDepth, DiagCode::MacroDepth]);
    assert!(got.iter().all(|r| r.2.len() <= 6), "a bounded number of labels: {got:?}");
    assert!(crate::testing::body(src).contains("mazer-error"));
}
