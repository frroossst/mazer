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
