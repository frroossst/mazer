use std::fmt::Write as _;

use mazer_syntax::{ByteOffset, NodeKind, Sym};

use super::*;
use crate::pipeline::analyze;

/// Strips the `|` marking the cursor; returns the analysis and the offset.
fn marked(marked: &str) -> (Analysis, ByteOffset) {
    let offset = marked.find('|').expect("a | marks the cursor");
    let src = marked.replacen('|', "", 1);
    (analyze(&src, "t.zr"), ByteOffset::from_usize(offset))
}

fn text(analysis: &Analysis, span: Span) -> &str {
    analysis.sources.text(span)
}

#[test]
fn code_forms_in_order_with_place() {
    let analysis = analyze("(let x 2)\n\nText (show x) and\n\n- (alias R (bb R))\n\n(show (tag a) y)", "t.zr");
    let forms: Vec<(String, Place)> =
        code_forms(&analysis).into_iter().map(|(f, p)| (f.to_string(), p)).collect();
    assert_eq!(
        forms,
        [
            ("(let x 2)".to_owned(), Place::Block),
            ("(show x)".to_owned(), Place::Inline),
            ("(alias R (bb R))".to_owned(), Place::Inline),
            ("(show (tag a) y)".to_owned(), Place::Block),
        ]
    );
    // The public walker visits the same forms.
    let mut count = 0;
    crate::for_each_code(&analysis.written, &mut |_, _| count += 1);
    assert_eq!(count, 4);
}

#[test]
fn enclosing_lists_outermost_first() {
    let (analysis, at) = marked("(show (frac (sqrt a|) b))");
    let chain: Vec<&str> = enclosing_lists(&analysis, at).iter().map(|n| text(&analysis, n.span)).collect();
    assert_eq!(chain, ["(show (frac (sqrt a) b))", "(frac (sqrt a) b)", "(sqrt a)"]);
    // Through the `'x` shorthand, which is not itself a paren list.
    let (analysis, at) = marked("(show '(f |x))");
    let chain: Vec<&str> = enclosing_lists(&analysis, at).iter().map(|n| text(&analysis, n.span)).collect();
    assert_eq!(chain, ["(show '(f x))", "(f x)"]);
    // Unclosed, mid-edit: the end counts as inside.
    let (analysis, at) = marked("(show (frac a|");
    assert_eq!(enclosing_lists(&analysis, at).len(), 2);
    // Prose and the `(` itself are outside.
    let (analysis, at) = marked("|(show x) text");
    assert_eq!(enclosing_lists(&analysis, at).len(), 0);
    let (analysis, at) = marked("(show x) te|xt");
    assert_eq!(enclosing_lists(&analysis, at).len(), 0);
}

#[test]
fn argument_index() {
    let index = |src: &str| {
        let (analysis, at) = marked(src);
        let chain = enclosing_lists(&analysis, at);
        arg_index(chain.last().expect("inside a list"), at)
    };
    assert_eq!(index("(show (fr|ac a b))"), None);
    assert_eq!(index("(show (frac| a b))"), None);
    assert_eq!(index("(show (frac |a b))"), Some(0));
    assert_eq!(index("(show (frac a| b))"), Some(0));
    assert_eq!(index("(show (frac a |b))"), Some(1));
    assert_eq!(index("(show (frac a b |))"), Some(2));
    // The reader's placeholder of an unclosed list is not an argument.
    assert_eq!(index("(show (frac a |"), Some(1));
}

#[test]
fn comments_in_code_only() {
    let src = "; prose\n(show (+ a ; one\r\n b \"; not\") ; two\n)\n\nText (show x ; three\n)";
    let analysis = analyze(src, "t.zr");
    let spans: Vec<&str> = comment_spans(&analysis).into_iter().map(|s| text(&analysis, s)).collect();
    assert_eq!(spans, ["; one", "; two", "; three"]);
}

#[test]
fn binding_in_scope_starts_after_the_let() {
    let src = "(let x 2)\n\n(show x)\n\n(let y (+ x 1))\n\n(show y)\n\n(let z (+ z 1))";
    let analysis = analyze(src, "t.zr");
    assert!(analysis.diags.is_empty(), "{:?}", analysis.diags);
    let value_at = |name: &str, needle: &str, from: usize| {
        let at = src[from..].find(needle).unwrap() + from;
        binding_at(&analysis, Sym::new(name), ByteOffset::from_usize(at)).map(|b| b.value.to_string())
    };
    assert_eq!(value_at("x", "(let x 2)", 0), None, "nothing before the first let");
    assert_eq!(value_at("x", "x)", 12), Some("2".to_owned()));
    // Inside a let's own value, its name is not bound yet.
    assert_eq!(value_at("y", "x 1)", 0), None);
    assert_eq!(value_at("z", "z 1)", 0), None);
    // After it: the value, captured with x substituted.
    assert_eq!(value_at("y", "y)", src.rfind("(show").unwrap()), Some("(+ 2 1)".to_owned()));
    let all = bindings_in_scope(&analysis, ByteOffset::from_usize(src.len()));
    let names: Vec<String> = all.iter().map(|b| b.name.to_string()).collect();
    assert_eq!(names, ["x", "y", "z"]);
    // A rejected let binds nothing: the earlier binding stays in scope,
    // whether it was malformed or a second definition.
    for src in ["(let y 1)\n\n(let y)\n\n(show y)", "(let y 1)\n\n(let y 5)\n\n(show y)"] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(analysis.lets.len(), 1, "{src}");
        let b = binding_at(&analysis, Sym::new("y"), ByteOffset::from_usize(src.len())).unwrap();
        assert_eq!(b.value.to_string(), "1", "{src}");
        assert_eq!(bindings_in_scope(&analysis, ByteOffset::from_usize(src.len())).len(), 1);
    }
}

#[test]
fn binding_refused_by_the_budget_is_an_error() {
    // Copying a 1 000-node value 300 times runs the document's node budget
    // out; each `let` refused by it binds its variable to an error, exactly
    // as `analyze` substitutes it afterwards.
    let mut src = format!("(let big (+{}))", " a".repeat(999));
    for i in 0..300 {
        let _ = write!(src, "\n\n(let c{i} big)");
    }
    src.push_str("\n\n(show c299)");
    let analysis = analyze(&src, "t.zr");
    let refused = analysis
        .lets
        .iter()
        .find(|b| matches!(b.value.kind, NodeKind::Error))
        .expect("a let refused by the budget");
    let end = ByteOffset::from_usize(src.len());
    let found = binding_at(&analysis, refused.name, end).unwrap();
    assert!(matches!(found.value.kind, NodeKind::Error));
    let last = binding_at(&analysis, Sym::new("c299"), end).unwrap();
    assert!(matches!(last.value.kind, NodeKind::Error));
    // And what analyze substituted at the end is an error too.
    let last = crate::query::code_forms(&analysis).len() - 1;
    let mut shown = Vec::new();
    crate::for_each_code(&analysis.resolved, &mut |form, _| shown.push(form));
    assert!(crate::walk::contains_error(shown[last]));
}

#[test]
fn name_spans() {
    let src = "(let x 2)\n\n(alias R (bb R))\n\n(macro (sq v) (^ v 2))\n\n(show (tag euler) (= x 1))\n\nSee (ref euler).";
    let analysis = analyze(src, "t.zr");
    let names: Vec<&str> = code_forms(&analysis)
        .into_iter()
        .filter_map(|(form, _)| name_span(form))
        .map(|s| text(&analysis, s))
        .collect();
    assert_eq!(names, ["x", "R", "sq"]);
    let def = analysis.macros.get(Sym::new("sq")).unwrap();
    assert_eq!(definition_name_span(&analysis, def.span).map(|s| text(&analysis, s)), Some("sq"));
    let binding = &analysis.lets[0];
    assert_eq!(definition_name_span(&analysis, binding.span).map(|s| text(&analysis, s)), Some("x"));
    let label = analysis.labels.get(Sym::new("euler")).unwrap();
    assert_eq!(label_name_span(&analysis, label).map(|s| text(&analysis, s)), Some("euler"));
    assert_eq!(
        equation_span(&analysis, label).map(|s| text(&analysis, s)),
        Some("(show (tag euler) (= x 1))")
    );
    // A tag form directly, and forms without a name.
    let tag = tagged_equation(&analysis, label).unwrap().args()[0].clone();
    assert_eq!(name_node(&tag).map(ToString::to_string), Some("euler".to_owned()));
    assert_eq!(name_span(&crate::testing::read("(show x)")), None);
    assert_eq!(name_span(&crate::testing::read("(let 3 x)")), None);
    // A prelude definition has no form in the note.
    assert_eq!(form_at(&analysis, Span::new(FileId(1), ByteOffset(0), ByteOffset(1))), None);
}

#[test]
fn locate_helpers() {
    let src = "(show '(f x) (g y";
    let analysis = analyze(src, "t.zr");
    let form = code_forms(&analysis)[0].0;
    assert!(crate::is_paren_list(src, form));
    let quoted = &form.args()[0];
    assert!(!crate::is_paren_list(src, quoted), "the 'x shorthand");
    assert!(crate::is_unclosed(form));
    assert!(!crate::is_unclosed(&quoted.args()[0]));
    assert!(crate::is_inside(src, form, ByteOffset::from_usize(src.len())), "an unclosed end is inside");
    assert!(!crate::is_inside(src, form, ByteOffset(0)), "the `(` itself is outside");
    let g = &form.args()[1];
    assert_eq!(crate::real_items(g).len(), 2, "without the reader's placeholder");
    let mut strings = Vec::new();
    crate::string_spans(&crate::testing::read("(f \"a\" (g \"b\"))"), &mut strings);
    assert_eq!(strings.len(), 2);
}
