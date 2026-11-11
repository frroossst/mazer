use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mazer_syntax::{BlockKind, DiagCode, FileId, Node, NodeKind, Severity, Sym};

use super::*;
use crate::testing::{body, clean, codes, parse, resolved_forms, unwrap_fragment};

// ---------------------------------------------------------------- analyze

#[test]
fn written_is_the_plain_parse() {
    let src = "(alias R (paren R))\n\n(let x 2)\n\n# Title (show (+ R x))\n\n- (show (frac x R))\n\n(show (tag e) x)";
    let analysis = clean(src);
    assert_eq!(analysis.written, parse(src).0);
    assert_ne!(analysis.resolved, analysis.written);
    assert_eq!(
        resolved_forms(&analysis),
        [
            "(alias R (paren R))",
            "(let x 2)",
            "(show (+ (paren R) 2))",
            "(show (frac 2 (paren R)))",
            "(show (tag e) 2)",
        ]
    );
}

#[test]
fn tags_and_refs_are_left_as_written() {
    let analysis = clean("(alias e (paren e))\n\n(show (tag e) e)\n\nSee (ref e).");
    assert_eq!(resolved_forms(&analysis), ["(alias e (paren e))", "(show (tag e) (paren e))", "(ref e)"]);
    assert_eq!(analysis.labels.get(Sym::new("e")).map(|l| l.number), Some(1));
}

#[test]
fn stdlib_is_clean() {
    let mut diags = Diags::new();
    let doc = markdown::parse(STDLIB, FileId(1), &mut diags);
    assert!(diags.is_empty(), "{diags:?}");
    for block in &doc.blocks {
        let ok = matches!(&block.kind, BlockKind::Code(n) if n.is_form("alias") || n.is_form("macro"));
        assert!(ok, "stdlib.zr may only hold definitions: {block:?}");
    }
    let analysis = clean("");
    assert_eq!(analysis.sources.file(FileId(1)).map(|f| f.name.as_str()), Some("stdlib.zr"));
    assert_eq!(analysis.sources.file(FileId(0)).map(|f| f.name.as_str()), Some("test.zr"));
    // Every prelude definition is visible in a note.
    let mut prelude = Macros::new();
    collect_macros(&doc, &mut prelude, &mut diags);
    for def in prelude.iter() {
        assert!(analysis.macros.get(def.name).is_some());
    }
    assert_eq!(analysis.macros.len(), prelude.len());
}

#[test]
fn prelude_definitions_are_visible_in_every_note() {
    let prelude = Prelude::read("(alias RR (paren RR))\n\n(macro (half x) \"Half.\" (frac x 2))");
    assert!(prelude.diags.is_empty(), "{:?}", prelude.diags);
    let analysis = analyze_with("(show (half RR))", "t.zr", &prelude);
    assert!(analysis.diags.is_empty(), "{:?}", analysis.diags);
    assert_eq!(resolved_forms(&analysis), ["(show (frac (paren RR) 2))"]);
    let half = analysis.macros.get(Sym::new("half")).unwrap();
    assert_eq!(half.span.file, FileId(1));
    assert_eq!(analysis.sources.text(half.span), "(macro (half x) \"Half.\" (frac x 2))");
    // A note may not redefine a prelude name; the diagnostic points at both.
    let analysis = analyze_with("(alias RR x)", "t.zr", &prelude);
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::DuplicateDefinition]);
    let diag = &analysis.diags.as_slice()[0];
    assert_eq!(diag.span.file, FileId(0));
    assert_eq!(diag.labels[0].0.file, FileId(1));
    // A broken prelude reports on file 1, once per note.
    let broken = Prelude::read("(alias sin x)");
    let analysis = analyze_with("hi", "t.zr", &broken);
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::ShadowsBuiltin]);
    assert_eq!(analysis.diags.as_slice()[0].span.file, FileId(1));
}

#[test]
fn diagnostics_are_reported_once() {
    let out = compile("(show (frac a))\n\nInline (show (frac b)) too.", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity, DiagCode::Arity]);
    // Keyword problems render in place, not as a box.
    assert!(out.html.contains("<merror>"));
    assert!(!out.html.contains("mazer-error"));

    let out = compile("(show (half a b))\n\n(macro (half x) (frac x 2))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::MacroArity]);
    // A bad notation use runs through a let into a show: still one report.
    let out = compile(
        "(macro (half x) (frac x 2))\n\n(let y (half a b))\n\n(show y)\n\n(show y)",
        &mut Options::new("t.zr"),
    );
    assert_eq!(codes(&out.diags), [DiagCode::MacroArity]);
    let out = compile("(let y (+ 1 (eval 2)))\n\n(show y)", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::NestedEval]);
}

#[test]
fn an_unclosed_form_reports_only_the_paren() {
    // The lists cut off by the missing parens are short of arguments only
    // because they are cut off.
    let analysis = analyze("(show (frac (frac (frac 1", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::UnclosedParen]);
    let analysis = analyze("(show (+ (frac 1 2) (half", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::UnclosedParen]);
    let analysis = analyze("(macro (half x) (frac x 2))\n\n(show (half (sqrt", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::UnclosedParen]);
    // A complete sibling inside the unclosed form is still checked, and so
    // is another form.
    let src = "(show (+ (frac 1) (sqrt x\n\n(show (frac 2))";
    let analysis = analyze(src, "t.zr");
    let found: Vec<(DiagCode, &str)> =
        analysis.diags.iter().map(|d| (d.code, analysis.sources.text(d.span))).collect();
    assert_eq!(
        found,
        [(DiagCode::UnclosedParen, "("), (DiagCode::Arity, "(frac 1)"), (DiagCode::Arity, "(frac 2)")]
    );
}

#[test]
fn empty_show_and_eval_arity() {
    let analysis = analyze("(show )", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
    let analysis = analyze("(show (tag a))", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
    let analysis = analyze("(eval )", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
    let analysis = analyze("(eval a b)", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
}

#[test]
fn nested_eval() {
    for src in ["(show (+ 1 (eval x)))", "(let y (eval x))", "(eval (eval x))", "(show (eval x))"] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::NestedEval], "{src}");
    }
    assert_eq!(body("(show (+ 1 (eval x)))"), "<pre class=\"mazer-error\">`eval` cannot be used inside another form</pre>");
}

#[test]
fn analyze_does_not_evaluate() {
    let analysis = analyze("(eval x)", "t.zr");
    assert!(analysis.diags.is_empty(), "NoEvaluator is a render diagnostic");
}

#[test]
fn compile_never_panics() {
    let inputs = [
        "",
        "(",
        ")",
        "(show",
        "(show ",
        "(show (",
        "(show (frac a",
        "(show )",
        "(show (tag))",
        "(show (tag a) (tag a) x)\n\n(show (tag a) y)",
        "(eval)",
        "(eval (eval (eval x)))",
        "(ref)",
        "(ref nope)",
        "(ref (ref x))",
        "(let)",
        "(let x",
        "(alias)",
        "(macro)",
        "(macro (",
        "(macro (f x...) x)\n\n(show (f))",
        "(alias A B)\n\n(alias B A)\n\n(show A)",
        "'''''",
        "(show '''')",
        "(show \"unterminated",
        "(show -)",
        "(show (-x))",
        "- (show (tag a) x)\n  - (ref a",
        "> (let x\n> (show x)",
        "# (show\n\n## (ref",
        "[(show x](u)",
        "**(show x**)",
        "\u{0}\u{1}(show \u{7}x)",
        "(show ())",
        "(show (()))",
        "((((((((((",
        "(show ((((((((((((((((((((((((((((((((x))))))))))))))))))))))))))))))))",
        "😭 (show 😭) (ref 😭)",
        "(show (sum (integral x) i))",
        "```\n(show x)\n",
        "(let x 1)(let x (+ x x))(show x)",
    ];
    for src in inputs {
        for page in [Page::Fragment, Page::Full { title: "t".into() }] {
            let mut opts = Options::new("t.zr");
            opts.page = page;
            let out = compile(src, &mut opts);
            assert!(!out.html.is_empty(), "{src:?}");
        }
    }
}

// ---------------------------------------------------------------- eval

/// An evaluator that records what it is given and answers `42`, fails on
/// `boom` (with a diagnostic) and on `quiet` (without one).
struct Stub {
    seen: Rc<RefCell<Vec<String>>>,
}

impl Evaluator for Stub {
    fn eval(&mut self, expr: &Node, diags: &mut Diags) -> Option<Node> {
        self.seen.borrow_mut().push(expr.to_string());
        match expr.as_ident().map(Sym::as_str) {
            Some("boom") => {
                diags.push(Diag::error(DiagCode::EvalFailed, expr.span, "cannot <evaluate> boom"));
                None
            }
            Some("quiet") => None,
            _ => Some(Node::new(NodeKind::Number("42".into()), expr.span)),
        }
    }
}

fn with_stub(src: &str) -> (Output, Vec<String>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut opts = Options::new("t.zr");
    opts.evaluator = Some(Box::new(Stub { seen: Rc::clone(&seen) }));
    let out = compile(src, &mut opts);
    let seen = seen.borrow().clone();
    (out, seen)
}

#[test]
fn eval_runs_in_document_order_on_resolved_trees() {
    let src = "(let x 2)\n\n(eval (+ x 1))\n\n- (eval a)\n\n(macro (half v) (frac v 2))\n\nText (eval (half x)) end.";
    let (out, seen) = with_stub(src);
    assert_eq!(seen, ["(+ 2 1)", "a", "(frac 2 2)"]);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let html = unwrap_fragment(&out.html);
    assert_eq!(
        html,
        "<div class=\"mazer-equation\"><math display=\"block\"><mn>42</mn></math></div>\
         <ul><li><math display=\"inline\"><mstyle displaystyle=\"true\"><mn>42</mn></mstyle></math></li></ul>\
         <p>Text <math display=\"inline\"><mstyle displaystyle=\"true\"><mn>42</mn></mstyle></math> end.</p>"
    );
}

#[test]
fn eval_failure_is_an_error_box() {
    let (out, _) = with_stub("A (eval boom) b.\n\n(eval quiet)");
    assert_eq!(codes(&out.diags), [DiagCode::EvalFailed, DiagCode::EvalFailed]);
    assert_eq!(
        unwrap_fragment(&out.html),
        "<p>A <span class=\"mazer-error\">cannot &lt;evaluate&gt; boom</span> b.</p>\
         <pre class=\"mazer-error\">evaluation failed</pre>"
    );
}

#[test]
fn broken_eval_is_not_evaluated() {
    let (out, seen) = with_stub("(eval a b)\n\n(eval (ref x))\n\n(macro (f x) x)\n\n(eval (f))");
    assert!(seen.is_empty(), "{seen:?}");
    assert_eq!(codes(&out.diags), [DiagCode::Arity, DiagCode::Other, DiagCode::MacroArity]);
}

#[test]
fn eval_without_an_evaluator() {
    let out = compile("(eval x)\n\nInline (eval y).", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::NoEvaluator, DiagCode::NoEvaluator]);
    assert_eq!(out.diags[0].severity, Severity::Error);
    assert_eq!(
        unwrap_fragment(&out.html),
        "<pre class=\"mazer-error\">no evaluator is configured, so `eval` cannot run</pre>\
         <p>Inline <span class=\"mazer-error\">no evaluator is configured, so `eval` cannot run</span>.</p>"
    );
}

#[test]
fn render_diagnostics_come_after_analysis_diagnostics() {
    let out = compile("(eval x)\n\n(show (frac a))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity, DiagCode::NoEvaluator]);
    let analysis = analyze("(eval x)", "t.zr");
    let mut diags = Diags::new();
    let _ = render(&analysis, &mut Options::new("t.zr"), &mut diags);
    assert_eq!(codes(diags.as_slice()), [DiagCode::NoEvaluator]);
    assert!(analysis.diags.is_empty());
}

// ---------------------------------------------------------------- pages

#[test]
fn fragment_page() {
    let out = compile("hi", &mut Options::new("notes/a\"b.zr"));
    assert_eq!(
        out.html,
        format!(
            "<div id=\"mazer-meta\" style=\"display:none\" data-source-path=\"notes/a&quot;b.zr\" data-version=\"{}\"></div>\
             <div id=\"mazer-wasm-container\"><p>hi</p></div>",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn full_page() {
    let mut opts = Options::new("a.zr");
    opts.page = Page::Full {
        title: "<Notes> & more".into(),
    };
    let html = compile("hi", &mut opts).html;
    assert!(html.starts_with("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><title>&lt;Notes&gt; &amp; more</title>"));
    assert!(html.contains("data-selector=\"pre code\""), "keeps the highlighter");
    assert!(html.contains(".mazer-equation{"));
    assert!(html.contains("<body><div id=\"mazer-meta\""));
    assert!(html.ends_with("<p>hi</p></body></html>"));
    assert!(!html.contains("mazer-wasm-container"));
}

// ---------------------------------------------------------------- escaping

#[test]
fn everything_from_the_note_is_escaped() {
    assert_eq!(
        body("<script>alert('x')</script> & \"q\""),
        "<p>&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;q&quot;</p>"
    );
    assert_eq!(
        body("[a<b](http://x/?a=1&b=\"><script>)"),
        "<p><a href=\"http://x/?a=1&amp;b=&quot;&gt;&lt;script&gt;\" target=\"_blank\" rel=\"noopener noreferrer\">a&lt;b</a></p>"
    );
    assert_eq!(
        body("```html\"><x\n<b>&amp;</b>\n```"),
        "<pre><code class=\"language-html&quot;&gt;&lt;x\">&lt;b&gt;&amp;amp;&lt;/b&gt;\n</code></pre>"
    );
    assert_eq!(body("`<i>`"), "<p><code>&lt;i&gt;</code></p>");
    assert_eq!(body("![a\"b](x\"y.png)"), "<p><img alt=\"a&quot;b\" src=\"x&quot;y.png\"/></p>");
    assert_eq!(body("# <h1>"), "<h1>&lt;h1&gt;</h1>");
    // Error messages quote the note.
    assert_eq!(body("See (ref <x>)."), "<p>See <span class=\"mazer-error\">no equation is tagged `&lt;x&gt;`</span>.</p>");
    // Tag names end up in attributes.
    assert_eq!(
        body("(show (tag a<b&) x)\n\n(ref a<b&)"),
        "<div class=\"mazer-equation\" id=\"eq-a&lt;b&amp;\"><math display=\"block\"><mi>x</mi></math><span class=\"mazer-eqno\">(1)</span></div>\
         <p><a class=\"mazer-ref\" href=\"#eq-a&lt;b&amp;\">(1)</a></p>"
    );
    // Control characters are dropped.
    assert_eq!(body("a\u{1}b"), "<p>ab</p>");
}

// ---------------------------------------------------------------- markdown constructs

#[test]
fn headings() {
    assert_eq!(body("# Hi *there*"), "<h1>Hi <em>there</em></h1>");
    assert_eq!(body("###### six"), "<h6>six</h6>");
    assert_eq!(body("## a (show x)"), "<h2>a <math display=\"inline\"><mstyle displaystyle=\"true\"><mi>x</mi></mstyle></math></h2>");
}

#[test]
fn paragraphs_and_breaks() {
    assert_eq!(body("a\nb"), "<p>a<br/>b</p>", "a soft newline is <br/>");
    assert_eq!(body("a  \nb"), "<p>a<br/>b</p>");
    assert_eq!(body("a\n\nb"), "<p>a</p><p>b</p>");
}

#[test]
fn lists() {
    assert_eq!(body("- a\n- b"), "<ul><li>a</li><li>b</li></ul>");
    assert_eq!(body("1. a\n2. b"), "<ol><li>a</li><li>b</li></ol>");
    assert_eq!(body("3. a\n4. b"), "<ol start=\"3\"><li>a</li><li>b</li></ol>");
    assert_eq!(body("- a\n  - b\n- c"), "<ul><li>a<ul><li>b</li></ul></li><li>c</li></ul>");
    // One paragraph is written bare; several all get <p>.
    assert_eq!(body("- a\n\n  more"), "<ul><li><p>a</p><p>more</p></li></ul>");
    assert_eq!(body("- a\n\n  more\n\n  - x"), "<ul><li><p>a</p><p>more</p><ul><li>x</li></ul></li></ul>");
    assert_eq!(body("- a\n  - x\n- b"), "<ul><li>a<ul><li>x</li></ul></li><li>b</li></ul>");
    // A paragraph of only definitions does not count.
    assert_eq!(body("- (let x 2)\n\n  more"), "<ul><li><p>more</p></li></ul>");
    assert_eq!(
        body("- a (show x)"),
        "<ul><li>a <math display=\"inline\"><mstyle displaystyle=\"true\"><mi>x</mi></mstyle></math></li></ul>"
    );
}

#[test]
fn checklists() {
    assert_eq!(
        body("-[ ] a\n-[x] b"),
        "<ul class=\"mazer-checklist\"><li><input type=\"checkbox\" disabled> a</li><li><input type=\"checkbox\" checked disabled> b</li></ul>"
    );
    assert_eq!(body("- a\n- [x] b"), "<ul><li>a</li><li><input type=\"checkbox\" checked disabled> b</li></ul>");
}

#[test]
fn block_quotes() {
    assert_eq!(body("> a\n> b"), "<blockquote>a<br/>b</blockquote>");
    assert_eq!(body("> a\n>\n> b"), "<blockquote><p>a</p><p>b</p></blockquote>");
    assert_eq!(body("> a\n>\n> - x"), "<blockquote>a<ul><li>x</li></ul></blockquote>");
    assert_eq!(body("> - x"), "<blockquote><ul><li>x</li></ul></blockquote>");
}

#[test]
fn code_blocks() {
    assert_eq!(body("```rust\nfn x() {}\n```"), "<pre><code class=\"language-rust\">fn x() {}\n</code></pre>");
    assert_eq!(body("```\n(show x)\n```"), "<pre><code>(show x)\n</code></pre>");
}

#[test]
fn rule_and_images() {
    assert_eq!(body("a\n\n---\n\nb"), "<p>a</p><hr/><p>b</p>");
    assert_eq!(body("![alt](a.png)"), "<p><img alt=\"alt\" src=\"a.png\"/></p>");
    assert_eq!(body("see ![i](b.png) here"), "<p>see <img alt=\"i\" src=\"b.png\"/> here</p>");
}

#[test]
fn inline_styles() {
    assert_eq!(body("**a _b ~c~_**"), "<p><strong>a <u>b <s>c</s></u></strong></p>");
    assert_eq!(body("*i*"), "<p><em>i</em></p>");
    // The old spoiler markup, unchanged.
    assert_eq!(
        body("||sp *x*||"),
        "<p><span style=\"background:#2f3136;color:transparent;border-radius:3px;padding:0 4px;cursor:pointer\" \
         onmouseover=\"this.style.color='#dcddde'\" onmouseout=\"this.style.color='transparent'\">sp <em>x</em></span></p>"
    );
    assert_eq!(
        body("[a **b**](https://x.org/(y))"),
        "<p><a href=\"https://x.org/(y)\" target=\"_blank\" rel=\"noopener noreferrer\">a <strong>b</strong></a></p>"
    );
    assert_eq!(body("`code`"), "<p><code>code</code></p>");
}

#[test]
fn emoji() {
    assert_eq!(body("hi :sob: :maple: :nope:"), "<p>hi 😭 🍁 :nope:</p>");
}

// ---------------------------------------------------------------- math

const INLINE_X: &str = "<math display=\"inline\"><mstyle displaystyle=\"true\"><mi>x</mi></mstyle></math>";

#[test]
fn inline_and_display_math() {
    assert_eq!(body("a (show x) b"), format!("<p>a {INLINE_X} b</p>"));
    assert_eq!(body("(show x)"), "<div class=\"mazer-equation\"><math display=\"block\"><mi>x</mi></math></div>");
    // A show alone inside a list item is inline.
    assert_eq!(body("- (show x)"), format!("<ul><li>{INLINE_X}</li></ul>"));
}

#[test]
fn several_expressions_are_juxtaposed() {
    // Exactly what `jux` gives: spacing and parentheses come from it.
    for (several, jux) in [
        ("(show a (+ b c))", "(show (jux a (+ b c)))"),
        ("(show (+ a b) c)", "(show (jux (+ a b) c))"),
        ("Inline (show x y z).", "Inline (show (jux x y z))."),
        ("(show (tag e) (+ a b) c)", "(show (tag e) (jux (+ a b) c))"),
    ] {
        assert_eq!(body(several), body(jux), "{several}");
    }
    assert!(body("(show (+ a b) c)").contains("<mo>(</mo>"), "(a+b) c is parenthesised");
    // The resolved tree holds the `jux` node, spanning the expressions, so
    // checking sees what is rendered.
    let analysis = clean("(let y 2)\n\n(show (tag e) (+ a y) c)");
    assert_eq!(resolved_forms(&analysis)[1], "(show (tag e) (jux (+ a 2) c))");
    let show = first_show(&analysis);
    let jux = &shown(&show)[0];
    assert_eq!(analysis.sources.text(jux.span), "(+ a y) c");
    let out = compile("(show (frac a) b)", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity]);
    // A single atom shows just that atom.
    assert_eq!(body("Group (show G)."), "<p>Group <math display=\"inline\"><mstyle displaystyle=\"true\"><mi>G</mi></mstyle></math>.</p>");
    assert_eq!(body("(show tau)"), "<div class=\"mazer-equation\"><math display=\"block\"><mi>τ</mi></math></div>");
}

#[test]
fn tagged_equations_and_references() {
    assert_eq!(
        body("See (ref b) and (ref a).\n\n(show (tag a) x)\n\n(show (tag b) y)"),
        "<p>See <a class=\"mazer-ref\" href=\"#eq-b\">(2)</a> and <a class=\"mazer-ref\" href=\"#eq-a\">(1)</a>.</p>\
         <div class=\"mazer-equation\" id=\"eq-a\"><math display=\"block\"><mi>x</mi></math><span class=\"mazer-eqno\">(1)</span></div>\
         <div class=\"mazer-equation\" id=\"eq-b\"><math display=\"block\"><mi>y</mi></math><span class=\"mazer-eqno\">(2)</span></div>"
    );
    assert_eq!(body("See (ref nope)."), "<p>See <span class=\"mazer-error\">no equation is tagged `nope`</span>.</p>");
    assert_eq!(
        body("Inline (show (tag a) x)."),
        "<p>Inline <span class=\"mazer-error\">only a display equation can be tagged</span>.</p>"
    );
    // The duplicate is boxed; the first keeps its number.
    assert_eq!(
        body("(show (tag a) x)\n\n(show (tag a) y)"),
        "<div class=\"mazer-equation\" id=\"eq-a\"><math display=\"block\"><mi>x</mi></math><span class=\"mazer-eqno\">(1)</span></div>\
         <pre class=\"mazer-error\">the tag `a` is already used</pre>"
    );
}

#[test]
fn definitions_produce_nothing() {
    assert_eq!(body("(let x 2)"), "");
    assert_eq!(body("(alias R (paren R))\n\n(macro (f x) x)"), "");
    assert_eq!(body("a\n(let x 2)\nb"), "<p>a<br/>b</p>");
    assert_eq!(body("(let x 2)\nfoo"), "<p>foo</p>");
    assert_eq!(body("foo\n(let x 2)"), "<p>foo</p>");
    assert_eq!(body("foo (let x 2) bar"), "<p>foo  bar</p>");
    assert_eq!(body("(let x 2) (let y 3)"), "");
    assert_eq!(body("- (let x 2)\n- b"), "<ul><li>b</li></ul>");
}

#[test]
fn containers_of_only_definitions_vanish() {
    assert_eq!(body("# (let x 2)"), "");
    assert_eq!(body("# (let x 2)\n\nafter"), "<p>after</p>");
    assert_eq!(body("- (let x 2)"), "");
    assert_eq!(body("- (let x 2)\n- (alias R x)"), "");
    assert_eq!(body("-[ ] (let x 2)\n-[x] done"), "<ul class=\"mazer-checklist\"><li><input type=\"checkbox\" checked disabled> done</li></ul>");
    assert_eq!(body("1. (let x 2)\n2. b"), "<ol><li>b</li></ol>");
    assert_eq!(body("- a\n  - (let x 2)"), "<ul><li>a</li></ul>");
    assert_eq!(body("> (let x 2)"), "");
    assert_eq!(body("> (let x 2)\n>\n> (macro (f a) a)"), "");
    assert_eq!(body("**(let x 2)**"), "");
    assert_eq!(body("a **(let x 2)** b"), "<p>a  b</p>");
    assert_eq!(body("*_(let x 2)_* (let y 3)"), "");
    assert_eq!(body("**(let x 2)**\nfoo"), "<p>foo</p>");
    // The definitions still take effect.
    assert_eq!(body("# (let x 2)\n\n(show x)"), body("(show 2)"));
    // A broken definition is not silent: its box keeps the container.
    assert_eq!(body("# (let sin 2)"), "<h1><span class=\"mazer-error\">`sin` is a built-in keyword</span></h1>");
    assert_eq!(body("**(let 1 2)**"), "<p><strong><span class=\"mazer-error\">expected the name of the variable</span></strong></p>");
}

#[test]
fn error_boxes() {
    // The form itself is unusable: a box with the message.
    assert_eq!(body("(show )"), "<pre class=\"mazer-error\">nothing to show</pre>");
    assert_eq!(body("(show (+ a b)\n\nmore"), "<pre class=\"mazer-error\">this `(` is never closed</pre><p>more</p>");
    assert_eq!(
        body("(show (half a b))\n\n(macro (half x) (frac x 2))"),
        "<pre class=\"mazer-error\">`half` takes 1 argument, found 2</pre>"
    );
    assert_eq!(body("(let sin 2)"), "<pre class=\"mazer-error\">`sin` is a built-in keyword</pre>");
    assert_eq!(body("a (let 1 2) b"), "<p>a <span class=\"mazer-error\">expected the name of the variable</span> b</p>");
    assert_eq!(body("(alias show x)"), "<pre class=\"mazer-error\">`show` is a reserved word</pre>");
    // A keyword problem inside an otherwise fine expression: <merror>, no box.
    assert_eq!(
        body("(show (+ (frac a) b))"),
        "<div class=\"mazer-equation\"><math display=\"block\"><mrow><merror><mtext>`frac` is missing its `den` argument</mtext></merror><mo>+</mo><mi>b</mi></mrow></math></div>"
    );
    // Hints never box: a typo'd head is a hint and a function application.
    let out = compile("(show (fracc a b))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::UnknownForm]);
    assert!(!out.html.contains("mazer-error"));
}

#[test]
#[cfg(feature = "latex")]
fn latex_target() {
    let mut opts = Options::new("t.zr");
    opts.target = Target::Latex;
    let out = compile("a (show x) b\n\n(show (tag e) y)\n\nSee (ref e).", &mut opts);
    let html = unwrap_fragment(&out.html);
    assert_eq!(
        html,
        "<p>a \\(\\displaystyle x\\) b</p><div class=\"mazer-equation\" id=\"eq-e\">\\[y \\tag{1}\\label{eq-e}\\]</div><p>See \\eqref{eq-e}.</p>"
    );
}

#[test]
#[cfg(feature = "latex")]
fn latex_labels_are_sanitised() {
    // Inline math is display style in both targets, so limits stay above
    // and below.
    let mut opts = Options::new("t.zr");
    opts.target = Target::Latex;
    let out = compile("a (show (sum k k 1 n)) b", &mut opts);
    assert!(unwrap_fragment(&out.html).starts_with("<p>a \\(\\displaystyle "), "{}", out.html);
    // A tag name with characters LaTeX would choke on: the label and the
    // reference get the same safe key; the HTML anchor keeps the name.
    let mut opts = Options::new("t.zr");
    opts.target = Target::Latex;
    let out = compile("(show (tag a%b#c{_é) y)\n\nSee (ref a%b#c{_é).", &mut opts);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let html = unwrap_fragment(&out.html);
    assert!(html.contains("\\label{eq-a-b-c-_-}"), "{html}");
    assert!(html.contains("\\eqref{eq-a-b-c-_-}"), "{html}");
    assert!(html.contains("id=\"eq-a%b#c{_é\""), "{html}");
}

#[test]
#[cfg(feature = "latex")]
fn latex_target_without_math() {
    // Nothing here reaches the LaTeX emitter.
    let mut opts = Options::new("t.zr");
    opts.target = Target::Latex;
    let out = compile("# Hi\n\n(let x 2)\n\nSee (ref nope).", &mut opts);
    assert_eq!(
        unwrap_fragment(&out.html),
        "<h1>Hi</h1><p>See <span class=\"mazer-error\">no equation is tagged `nope`</span>.</p>"
    );
}

// ---------------------------------------------------------------- limits and scaling

/// The first `show` of an analysis, resolved.
fn first_show(analysis: &Analysis) -> Node {
    let mut found = None;
    crate::walk::for_each_code(&analysis.resolved, &mut |form, _| {
        if found.is_none() && form.is_form("show") {
            found = Some(form.clone());
        }
    });
    found.expect("a show")
}

/// Fifteen definitions that each double the previous one (`x15` is about
/// 100 000 nodes, just under the one-tree limit), then `uses` shows of the
/// last.
fn doubling(kind: &str, uses: usize) -> String {
    let mut src = format!("({kind} x0 a)");
    for i in 1..=15 {
        let p = i - 1;
        write!(src, "\n\n({kind} x{i} (+ x{p} x{p}))").unwrap();
    }
    for _ in 0..uses {
        src.push_str("\n\n(show x15)");
    }
    src
}

#[test]
fn one_node_budget_for_the_whole_document() {
    for (kind, code) in [("let", DiagCode::Other), ("alias", DiagCode::MacroDepth)] {
        let src = doubling(kind, 800);
        assert!(src.len() < 11_000);
        let start = Instant::now();
        let analysis = analyze(&src, "t.zr");
        let analyzed = start.elapsed();
        let out = compile(&src, &mut Options::new("t.zr"));
        let elapsed = start.elapsed();
        // Exactly one diagnostic for the whole note, naming the limit.
        assert_eq!(codes(&out.diags), [code], "{kind}");
        let diag = &out.diags[0];
        assert!(diag.message.contains(&crate::walk::MAX_DOC_NODES.to_string()), "{}", diag.message);
        assert!(diag.help.as_deref().is_some_and(|h| h.contains("exponentially")));
        assert_eq!(analysis.sources.text(diag.span), "x15", "{kind}: at the expression that crossed the limit");
        // Every show is either rendered or an error box; the output is bounded.
        let html = unwrap_fragment(&out.html);
        let shown = html.matches("<div class=\"mazer-equation\">").count();
        let boxed = html.matches("<pre class=\"mazer-error\">").count();
        assert_eq!(shown + boxed, 800, "{kind}");
        assert!(boxed >= 790, "{kind}: {boxed} boxes");
        assert!(html.len() < 4_000_000, "{kind}: {} bytes", html.len());
        // The resolved tree, which the editor keeps on every keystroke, is
        // bounded too.
        let mut nodes = 0;
        crate::walk::for_each_code(&analysis.resolved, &mut |form, _| nodes += crate::walk::Measure::of(form).size);
        assert!(nodes < crate::walk::MAX_DOC_NODES + crate::walk::MAX_NODES + 20_000, "{kind}: {nodes} nodes");
        assert!(analyzed < Duration::from_secs(10), "{kind}: analyze took {analyzed:?}");
        assert!(elapsed < Duration::from_secs(20), "{kind}: took {elapsed:?}");
    }
}

#[test]
fn ordinary_notes_stay_within_the_budget() {
    let mut src = String::from("(macro (half v) (frac v 2))\n\n(alias R (bb R))");
    for i in 0..10_000 {
        write!(src, "\n\n(let x{i} (+ a {i}))\n\n(show (half x{i}) R)").unwrap();
    }
    let out = compile(&src, &mut Options::new("t.zr"));
    assert!(out.diags.is_empty(), "{:?}", &out.diags[..out.diags.len().min(3)]);
    assert!(!out.html.contains("mazer-error"));
    assert_eq!(out.html.matches("mazer-equation").count(), 10_000);
}

#[test]
fn rendering_is_linear_in_forms_and_diagnostics() {
    for src in ["(show (frac 1))\n\n".repeat(20_000), "a (show (frac 1)) ".repeat(20_000)] {
        let analysis = analyze(&src, "t.zr");
        assert_eq!(analysis.diags.len(), 20_000);
        let start = Instant::now();
        let mut extra = Diags::new();
        let html = render(&analysis, &mut Options::new("t.zr"), &mut extra);
        let elapsed = start.elapsed();
        assert!(extra.is_empty(), "nothing is reported twice");
        assert_eq!(html.matches("<merror>").count(), 20_000);
        assert!(elapsed < Duration::from_secs(1), "render took {elapsed:?}");
    }
}

// ---------------------------------------------------------------- nesting

#[test]
fn reserved_heads_cannot_be_nested_in_code() {
    use DiagCode::{BadDefinition, MisplacedTag, NestedEval, Other};
    for (src, code, inner) in [
        ("(show (ref a))", Other, "(ref a)"),
        ("(show (+ 1 (show x)))", Other, "(show x)"),
        ("(show (let y 2))", BadDefinition, "(let y 2)"),
        ("(show (alias y 2))", BadDefinition, "(alias y 2)"),
        ("(show (macro (f) 2))", BadDefinition, "(macro (f) 2)"),
        ("(show x (tag a))", MisplacedTag, "(tag a)"),
        ("(show (+ (tag a) x))", MisplacedTag, "(tag a)"),
        ("(show (tag a) (tag b) x)", MisplacedTag, "(tag b)"),
        ("(show (eval x))", NestedEval, "(eval x)"),
        ("(eval (ref a))", Other, "(ref a)"),
        ("(eval (tag a))", MisplacedTag, "(tag a)"),
        ("(eval (+ 1 (let y 2)))", BadDefinition, "(let y 2)"),
        ("(let y (ref a))", Other, "(ref a)"),
        ("(let y (show a))", Other, "(show a)"),
        ("(let y (let z 2))", BadDefinition, "(let z 2)"),
        ("(let y (tag a))", MisplacedTag, "(tag a)"),
        ("(show '(ref a))", Other, "(ref a)"),
        ("(show (quote (+ 1 (eval x))))", NestedEval, "(eval x)"),
        ("Inline (show (+ 1 (ref a))) here.", Other, "(ref a)"),
    ] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(codes(analysis.diags.as_slice()), [code], "{src}");
        assert_eq!(analysis.sources.text(analysis.diags.as_slice()[0].span), inner, "{src}");
        assert!(body(src).contains("class=\"mazer-error\""), "{src}: the enclosing form is an error box");
    }
    let analysis = analyze("(show (ref a))", "t.zr");
    assert_eq!(analysis.diags.as_slice()[0].message, "`ref` can only be used in prose, not inside `show`");
    let analysis = analyze("(let y (show a))", "t.zr");
    assert_eq!(analysis.diags.as_slice()[0].message, "`show` can only be used in prose, not inside `let`");
    assert!(body("(show (ref a))").starts_with("<pre class=\"mazer-error\">`ref` can only be used in prose"));
    // A reserved word that is not a list head is an ordinary variable.
    clean("(show (jux show me))");
    clean("(show (+ ref tag let eval))\n\n(let y (+ alias macro))\n\n(show y)");
    // Reported once, even when the expression is copied.
    let out = compile("(let y (+ 1 (ref a)))\n\n(show y)\n\n(show y)", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Other]);
    let out = compile("(macro (d v) (+ v v))\n\n(show (d (ref a)))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Other]);
}

#[test]
fn reserved_heads_in_notation_are_rejected_where_defined() {
    let src = "(alias T (tag a))\n\n(show T x)";
    let analysis = analyze(src, "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::MisplacedTag]);
    assert_eq!(analysis.sources.text(analysis.diags.as_slice()[0].span), "(tag a)");
    assert!(analysis.macros.get(Sym::new("T")).is_none(), "the name is not defined");
    assert!(analysis.labels.is_empty());
    assert_eq!(
        body(src),
        format!(
            "<pre class=\"mazer-error\">a tag must be the first item of a display `show`</pre>{}",
            body("(show (jux T x))")
        )
    );
    for (src, code) in [
        ("(macro (f v) (+ v (tag a)))", DiagCode::MisplacedTag),
        ("(alias E (eval x))", DiagCode::NestedEval),
        ("(alias S (paren (show x)))", DiagCode::Other),
        ("(macro (f) (let y 2))", DiagCode::BadDefinition),
        ("(alias R '(ref a))", DiagCode::Other),
    ] {
        let analysis = analyze(src, "t.zr");
        assert_eq!(codes(analysis.diags.as_slice()), [code], "{src}");
        assert!(analysis.macros.is_empty(), "{src}");
    }
    // A reserved word as a plain name in a template is fine.
    let analysis = clean("(alias S (jux show tag))\n\n(show S)");
    assert_eq!(resolved_forms(&analysis)[1], "(show (jux show tag))");
}

// ---------------------------------------------------------------- reporting

#[test]
fn a_boxed_tagged_equation_keeps_its_anchor() {
    assert_eq!(
        body("(macro (half v) (frac v 2))\n\n(show (tag a) (half 1 2))\n\nSee (ref a)."),
        "<pre class=\"mazer-error\" id=\"eq-a\">`half` takes 1 argument, found 2</pre>\
         <p>See <a class=\"mazer-ref\" href=\"#eq-a\">(1)</a>.</p>"
    );
    // An untagged box has no id.
    assert!(!body("(macro (half v) (frac v 2))\n\n(show (half 1 2))").contains(" id="));
}

#[test]
fn a_let_value_is_checked_once_where_it_is_bound() {
    let src = "(let y (frac 1))\n\n(show y)\n\n(show (+ y 1))";
    let analysis = analyze(src, "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
    assert_eq!(analysis.sources.text(analysis.diags.as_slice()[0].span), "(frac 1)", "reported at the let");
    let out = compile(src, &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity], "render does not report it again");
    let html = unwrap_fragment(&out.html);
    // The let shows the problem; each use still renders its <merror>.
    assert!(html.starts_with("<pre class=\"mazer-error\">`frac` is missing its `den` argument</pre>"), "{html}");
    assert_eq!(html.matches("<merror>").count(), 2, "{html}");
    // Through a later let, too.
    let out = compile("(let y (frac 1))\n\n(let z (+ y 2))\n\n(show z)\n\n(show z)", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity]);
    // A new problem at a use is still reported there.
    let analysis = analyze("(let y 2)\n\n(show (frac y))", "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::Arity]);
    assert_eq!(analysis.sources.text(analysis.diags.as_slice()[0].span), "(frac y)");
}

#[test]
fn identical_diagnostics_are_merged() {
    // A closed template is checked at its definition: two faulty forms are
    // two reports there, and the uses report nothing more.
    let src = "(macro (f) (+ (frac 1) (frac 1)))\n\n(show (f))";
    let out = compile(src, &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity, DiagCode::Arity]);
    assert!(out.diags.iter().all(|d| out.sources.text(d.span) == "(frac 1)" && d.span.start.0 < 34));
    let out = compile("(macro (f) (frac 1))\n\n(show (f))\n\n(show (f))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity], "one report, at the definition");
    // A problem that depends on the arguments: one report per use.
    let out = compile("(macro (f v) (frac v))\n\n(show (f a))\n\n(show (f a))", &mut Options::new("t.zr"));
    assert_eq!(codes(&out.diags), [DiagCode::Arity, DiagCode::Arity], "different places are different reports");
    let out = compile("(macro (f v) (+ (frac v) (frac v)))\n\n(show (f a))", &mut Options::new("t.zr"));
    // The same problem twice in the template: one report per use (same code,
    // span and message are merged), labelled at the first template form.
    assert_eq!(codes(&out.diags), [DiagCode::Arity]);
    assert_eq!(out.diags[0].labels.len(), 1);
    assert_eq!(out.diags[0].labels[0].0.start.0, 16);
    // A macro problem at the use keeps its label to the definition.
    let src = "(macro (half v) (frac v 2))\n\n(let y (half a b))\n\n(show y)";
    let analysis = analyze(src, "t.zr");
    assert_eq!(codes(analysis.diags.as_slice()), [DiagCode::MacroArity]);
    let def = analysis.macros.get(Sym::new("half")).unwrap();
    assert_eq!(analysis.diags.as_slice()[0].labels, vec![(def.span, "defined here".to_owned())]);
}

// ---------------------------------------------------------------- links

#[test]
fn a_reference_inside_a_link_is_plain_text() {
    assert_eq!(
        body("(show (tag a) x)\n\n[see (ref a)](u)"),
        "<div class=\"mazer-equation\" id=\"eq-a\"><math display=\"block\"><mi>x</mi></math><span class=\"mazer-eqno\">(1)</span></div>\
         <p><a href=\"u\" target=\"_blank\" rel=\"noopener noreferrer\">see (1)</a></p>"
    );
    // Outside the link it is a link again.
    assert!(body("(show (tag a) x)\n\n[l](u) (ref a)").contains("<a class=\"mazer-ref\" href=\"#eq-a\">(1)</a>"));
}

#[test]
fn dangerous_urls_are_not_written() {
    for (src, want) in [
        ("[click](javascript:alert(1))", "<p>click</p>"),
        ("[click *me*](JavaScript:alert(1))", "<p>click <em>me</em></p>"),
        ("[c](java\u{1}script:x)", "<p>c</p>"),
        ("[c](vbscript:x)", "<p>c</p>"),
        ("[c](data:text/html,x)", "<p>c</p>"),
        ("[c](data:image/png,x)", "<p>c</p>"),
        ("![alt](javascript:x)", "<p>alt</p>"),
        ("![a<b](data:text/html,x)", "<p>a&lt;b</p>"),
        ("see ![a](DATA:text/html,x) here", "<p>see a here</p>"),
    ] {
        let analysis = analyze(src, "t.zr");
        assert!(analysis.diags.is_empty(), "{src}: a render-time check");
        let out = compile(src, &mut Options::new("t.zr"));
        assert_eq!(unwrap_fragment(&out.html), want, "{src}");
        assert_eq!(codes(&out.diags), [DiagCode::Other], "{src}");
        assert_eq!(out.diags[0].severity, Severity::Warning);
        let text = analysis.sources.text(out.diags[0].span);
        assert!(text.starts_with('[') || text.starts_with("!["), "{src}: at the link, got {text:?}");
    }
    for (src, want) in [
        ("![a](data:image/png;base64,AAAA)", "<p><img alt=\"a\" src=\"data:image/png;base64,AAAA\"/></p>"),
        ("[a](https://x.org/javascript:y)", "<p><a href=\"https://x.org/javascript:y\" target=\"_blank\" rel=\"noopener noreferrer\">a</a></p>"),
        ("[a](data-sheet.html)", "<p><a href=\"data-sheet.html\" target=\"_blank\" rel=\"noopener noreferrer\">a</a></p>"),
    ] {
        let out = compile(src, &mut Options::new("t.zr"));
        assert_eq!(unwrap_fragment(&out.html), want, "{src}");
        assert!(out.diags.is_empty(), "{src}");
    }
}
