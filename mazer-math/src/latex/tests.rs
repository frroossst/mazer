//! Tests of the LaTeX emitter.
//!
//! The parenthesis tests mirror the `parentheses` module of `mathml.rs`: the
//! same inputs, with the LaTeX expected.

use std::fmt::Write as _;

use mazer_syntax::{DiagCode, Diags, Node, Severity, Span};

use super::{Latex, signs};
use crate::display::display_names;
use crate::registry::registry;
use crate::testing::{read, with_standins};

/// Renders `src`, returning the output and the diagnostics.
fn render(src: &str) -> (String, Diags) {
    let node = read(src);
    let mut diags = Diags::new();
    let out = crate::to_latex(&node, &mut diags);
    (out, diags)
}

/// Renders `src` and asserts it produced no diagnostics.
#[track_caller]
fn latex(src: &str) -> String {
    let (out, diags) = render(src);
    assert!(diags.is_empty(), "{src:?} gave diagnostics: {diags:?}");
    out
}

#[track_caller]
fn shows(cases: &[(&str, &str)]) {
    for &(src, expected) in cases {
        assert_eq!(latex(src), expected, "{src}");
    }
}

fn with(f: impl FnOnce(&mut Latex<'_>)) -> (String, Diags) {
    let mut diags = Diags::new();
    let mut out = Latex::new(&mut diags);
    f(&mut out);
    let out = out.finish();
    (out, diags)
}

// --- Registry-wide checks ------------------------------------------------------

/// Keywords allowed to have no LaTeX emitter. Empty: every keyword has one.
const WITHOUT_LATEX: &[&str] = &[];

#[test]
fn every_keyword_has_an_emitter() {
    for keyword in registry() {
        assert!(
            keyword.latex.is_some() || WITHOUT_LATEX.contains(&keyword.name),
            "`{}` has no LaTeX emitter",
            keyword.name
        );
    }
    for name in WITHOUT_LATEX {
        assert!(
            registry().iter().any(|k| k.name == *name && k.latex.is_none()),
            "`{name}` is allow-listed but has an emitter (or does not exist)"
        );
    }
}

#[test]
fn every_sign_has_a_spelling() {
    for keyword in registry() {
        if let Some(sign) = keyword.sign {
            assert!(
                signs::sign(sign).is_some(),
                "sign {sign:?} of `{}` has no LaTeX spelling",
                keyword.name
            );
        }
    }
}

#[test]
fn bare_signs() {
    shows(&[
        ("integral", "\\int"),
        ("sum", "\\sum"),
        ("add", "+"),
        ("-", "-"),
        ("*", "*"),
        ("/", "\\div"),
        ("neq", "\\neq"),
        ("leq", "\\leq"),
        ("geq", "\\geq"),
        ("subset", "\\subset"),
        ("arrow", "\\to"),
        ("partial", "\\partial"),
        ("sin", "\\sin"),
        ("cosec", "\\operatorname{cosec}"),
        ("arcsin", "\\arcsin"),
        ("lim", "\\lim"),
        ("fact", "!"),
        ("prime", "'"),
        ("<", "<"),
        ("and", "\\land"),
        ("implies", "\\implies"),
    ]);
}

/// Every control word the emitters can write. A word fused to a letter after
/// it (`\intf`) is not in this list, so it is caught.
const KNOWN_WORDS: &[&str] = &[
    "left", "right", "frac", "dfrac", "sqrt", "binom", "begin", "end", "hline", "int", "sum", "prod", "lim",
    "partial", "mathrm", "mathbb", "mathcal", "mathfrak", "mathbf", "text", "operatorname", "sin", "cos",
    "tan", "cot", "sec", "arcsin", "arccos", "arctan", "ln", "log", "exp", "to", "div", "times", "neq", "leq",
    "geq", "approx", "in", "notin", "subset", "subseteq", "supseteq", "cup", "cap", "land", "lor", "lnot",
    "implies", "iff", "forall", "exists", "lfloor", "rfloor", "lceil", "rceil", "bar", "overline", "hat",
    "widehat", "dot", "ddot", "vec", "overrightarrow", "boxed", "qquad", "textbackslash", "textasciicircum",
    "textasciitilde", "textless", "textgreater", "textbar", "backslash", "wedge", "sim", "cosec",
];

/// The control words in `s`, as the maximal runs a TeX reader would see.
fn control_words(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut words = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                end += 1;
            }
            if end > start {
                words.push(s[start..end].to_owned());
            }
            i = end.max(start + 1);
        } else {
            i += 1;
        }
    }
    words
}

/// `s` with every `\text{...}` group removed.
fn without_text(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find("\\text{") {
        out.push_str(&rest[..at]);
        let mut depth = 0usize;
        let mut end = rest.len();
        let bytes = rest.as_bytes();
        for (i, &b) in bytes.iter().enumerate().skip(at + 5) {
            let escaped = bytes[..i].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 1;
            match b {
                b'{' if !escaped => depth += 1,
                b'}' if !escaped => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Braces balanced, `\left`/`\right` and `\begin`/`\end` paired, ASCII outside
/// `\text{}`, no unknown control word.
#[track_caller]
fn assert_sane(src: &str, out: &str) {
    let mut depth = 0i64;
    let bytes = out.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        let escaped = bytes[..i].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 1;
        match b {
            b'{' if !escaped => depth += 1,
            b'}' if !escaped => depth -= 1,
            _ => {}
        }
        assert!(depth >= 0, "{src}: closing brace without an opening one in {out}");
    }
    assert_eq!(depth, 0, "{src}: unbalanced braces in {out}");

    let mut stack: Vec<String> = Vec::new();
    let mut rest = out;
    while let Some(at) = rest.find('\\') {
        rest = &rest[at + 1..];
        let word_len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
        let (word, after) = rest.split_at(word_len);
        // `\begin{name}` / `\end{name}`: the name is in the braces.
        let environment = || {
            let inner = after.strip_prefix('{').expect("environment name");
            inner[..inner.find('}').expect("unclosed environment name")].to_owned()
        };
        match word {
            "left" => stack.push("left".to_owned()),
            "right" => assert_eq!(stack.pop().as_deref(), Some("left"), "{src}: stray \\right in {out}"),
            "begin" => stack.push(environment()),
            "end" => {
                let name = environment();
                assert_eq!(stack.pop(), Some(name.clone()), "{src}: \\end{{{name}}} does not match in {out}");
            }
            _ => {}
        }
        rest = after;
    }
    assert!(stack.is_empty(), "{src}: unclosed {stack:?} in {out}");

    let visible = without_text(out);
    assert!(visible.is_ascii(), "{src}: non-ASCII output outside \\text: {out}");

    let display_words: Vec<String> = display_names().iter().flat_map(|d| control_words(d.latex)).collect();
    for word in control_words(&visible) {
        assert!(
            KNOWN_WORDS.contains(&word.as_str()) || display_words.contains(&word),
            "{src}: unknown or fused control word \\{word} in {out}"
        );
    }
}

#[test]
fn every_example_renders_cleanly() {
    for keyword in registry() {
        for src in &keyword.examples {
            let (out, diags) = render(src);
            assert!(diags.is_empty(), "`{}`: {src:?} gave diagnostics: {diags:?}", keyword.name);
            assert!(!out.trim().is_empty(), "`{}`: {src:?} rendered nothing", keyword.name);
            assert_sane(src, &out);
        }
    }
}

#[test]
fn every_display_name_emits_its_latex() {
    for display in display_names() {
        assert!(!display.latex.is_empty(), "`{}` has no LaTeX spelling", display.name);
        let out = latex(display.name);
        assert!(out.contains(display.latex), "`{}`: {out} does not contain {}", display.name, display.latex);
        assert_sane(display.name, &out);
        // Used as an operand too.
        let out = latex(&format!("(+ {} a)", display.name));
        assert!(out.contains(display.latex), "`{}`: {out}", display.name);
        assert_sane(display.name, &out);
    }
    shows(&[
        ("tau", "\\tau"),
        ("Gamma", "\\Gamma"),
        ("to", "\\to"),
        ("infinity", "\\infty"),
        ("Reals", "\\mathbb{R}"),
        ("turnstile", "\\vdash"),
        ("epsilon", "\\varepsilon"),
        // A spelling that starts with a script gets an empty base.
        ("degree", "{{}^\\circ}"),
        ("(^ degree 2)", "{{}^\\circ}^{2}"),
    ]);
}

/// One exact output for every keyword and each of its optional forms.
const OUTPUTS: &[(&str, &str)] = &[
    ("(+ a b)", "a + b"),
    ("(+ 1 x (frac y 2))", "1 + x + \\frac{y}{2}"),
    ("(add a b c)", "a + b + c"),
    ("(+ x)", "+x"),
    ("(+)", "+"),
    ("(- t a)", "t - a"),
    ("(- x)", "-x"),
    ("(sub a b c)", "a - b - c"),
    ("(* a b)", "a \\times b"),
    ("(* 2 (+ a b) c)", "2 \\times \\left(a + b\\right) \\times c"),
    ("(mul x y)", "x \\times y"),
    ("(* x)", "x"),
    ("(/ 1 t)", "\\frac{1}{t}"),
    ("(/ (+ a b) c)", "\\frac{a + b}{c}"),
    ("(/ a b c)", "a \\div b \\div c"),
    ("(div x)", "\\frac{1}{x}"),
    ("(^ x 2)", "x^{2}"),
    ("(^ (+ a b) n)", "\\left(a + b\\right)^{n}"),
    ("(pow e (- x))", "e^{-x}"),
    ("(sqrt x)", "\\sqrt{x}"),
    ("(sqrt (+ a b))", "\\sqrt{a + b}"),
    ("(root 3 x)", "\\sqrt[3]{x}"),
    ("(root n (+ a b))", "\\sqrt[n]{a + b}"),
    ("(jux a b c)", "a \\, b \\, c"),
    ("(jux x)", "x"),
    ("(juxtapose f (paren x))", "f \\, \\left(x\\right)"),
    ("(= a b c)", "a = b = c"),
    ("(eq x (+ 1 y))", "x = 1 + y"),
    ("(!= a b)", "a \\neq b"),
    ("(neq x 0)", "x \\neq 0"),
    ("(< a b)", "a < b"),
    ("(lt 0 x 1)", "0 < x < 1"),
    ("(> a b)", "a > b"),
    ("(<= a b)", "a \\leq b"),
    ("(leq 0 x 1)", "0 \\leq x \\leq 1"),
    ("(>= a 0)", "a \\geq 0"),
    ("(geq x y z)", "x \\geq y \\geq z"),
    ("(approx x (/ 22 7))", "x \\approx \\frac{22}{7}"),
    ("(set a b c)", "\\left\\{a, b, c\\right\\}"),
    ("(set)", "\\left\\{\\right\\}"),
    ("(in a (set 1 2))", "a \\in \\left\\{1, 2\\right\\}"),
    ("(notin 0 (set 1 2))", "0 \\notin \\left\\{1, 2\\right\\}"),
    ("(subset A B)", "A \\subseteq B"),
    ("(superset A B)", "A \\supseteq B"),
    ("(union A B C)", "A \\cup B \\cup C"),
    ("(intersect A (union B C))", "A \\cap \\left(B \\cup C\\right)"),
    ("(and p (or q r) s)", "p \\land \\left(q \\lor r\\right) \\land s"),
    ("(or p q r)", "p \\lor q \\lor r"),
    ("(not (and p q))", "\\lnot\\left(p \\land q\\right)"),
    ("(implies (and p q) r)", "p \\land q \\implies r"),
    ("(iff (= x 0) (= y 0))", "x = 0 \\iff y = 0"),
    ("(forall x (> x 0))", "\\forall x.\\, x > 0"),
    ("(forall alpha (jux alpha beta))", "\\forall\\alpha.\\, \\alpha \\, \\beta"),
    ("(forall alpha ())", "\\forall\\alpha.\\, {}"),
    ("(exists x (= (* x x) 2))", "\\exists x.\\, x \\times x = 2"),
    ("(paren (frac 1 2))", "\\left(\\frac{1}{2}\\right)"),
    ("(bracket (frac a b))", "\\left[\\frac{a}{b}\\right]"),
    ("(brace (+ a b))", "\\left\\{a + b\\right\\}"),
    ("(frac (+ a b) c)", "\\frac{a + b}{c}"),
    ("(integral (sin x) x)", "\\int \\sin\\left(x\\right) \\, \\mathrm{d}x"),
    ("(integral (frac 1 x) x 1 b)", "\\int_{1}^{b} \\frac{1}{x} \\, \\mathrm{d}x"),
    ("(integral f V D)", "\\int_{D} f \\, \\mathrm{d}V"),
    ("(sum (subscript a n) n)", "\\sum_{n} a_{n}"),
    ("(sum (frac 1 k) k 1 n)", "\\sum_{k=1}^{n} \\frac{1}{k}"),
    ("(sum f x S)", "\\sum_{S} f"),
    ("(prod (frac 1 k) k 1 n)", "\\prod_{k=1}^{n} \\frac{1}{k}"),
    ("(product f x S)", "\\prod_{S} f"),
    ("(lim x 0 (frac (sin x) x))", "\\lim_{x \\to 0} \\frac{\\sin\\left(x\\right)}{x}"),
    ("(limit n a (subscript a n) ())", "\\lim_{n \\to a} a_{n}"),
    ("(lim x 0 f +)", "\\lim_{x \\to 0^{+}} f"),
    ("(lim x 0)", "\\lim_{x \\to 0}"),
    ("(derivative (sin x) x)", "\\frac{\\mathrm{d}}{\\mathrm{d}x} \\sin\\left(x\\right)"),
    ("(deriv f t 2)", "\\frac{\\mathrm{d}^{2}}{\\mathrm{d}t^{2}} f"),
    ("(partial f x)", "\\frac{\\partial}{\\partial x} f"),
    ("(dd y x)", "\\frac{\\mathrm{d}y}{\\mathrm{d}x}"),
    ("(dd x t 2)", "\\frac{\\mathrm{d}^{2}x}{\\mathrm{d}t^{2}}"),
    ("(evalat f x 0)", "\\left.f\\right|_{x=0}"),
    ("(eval-at (frac 1 x) 2)", "\\left.\\frac{1}{x}\\right|_{2}"),
    ("(evalat f)", "\\left.f\\right|"),
    ("(sin (+ x t))", "\\sin\\left(x + t\\right)"),
    ("(cos x)", "\\cos\\left(x\\right)"),
    ("(tan x)", "\\tan\\left(x\\right)"),
    ("(cot x)", "\\cot\\left(x\\right)"),
    ("(sec x)", "\\sec\\left(x\\right)"),
    ("(cosec x)", "\\operatorname{cosec}\\left(x\\right)"),
    ("(csc (+ x t))", "\\operatorname{cosec}\\left(x + t\\right)"),
    ("(arcsin x)", "\\sin^{-1}\\left(x\\right)"),
    ("(acos (frac x 2))", "\\cos^{-1}\\left(\\frac{x}{2}\\right)"),
    ("(atan x)", "\\tan^{-1}\\left(x\\right)"),
    ("(ln (+ x 1))", "\\ln\\left(x + 1\\right)"),
    ("(log x)", "\\log\\left(x\\right)"),
    ("(log 2 x)", "\\log_{2}\\left(x\\right)"),
    ("(exp (frac x 2))", "e^{\\frac{x}{2}}"),
    ("(abs (+ x 1))", "\\left|x + 1\\right|"),
    ("(floor (frac n 2))", "\\left\\lfloor\\frac{n}{2}\\right\\rfloor"),
    ("(ceil x)", "\\left\\lceil x\\right\\rceil"),
    ("(factorial (+ n 1))", "\\left(n + 1\\right)!"),
    ("(nCr (+ n 1) 2)", "\\binom{n + 1}{2}"),
    ("(subscript a (+ n 1))", "a_{n + 1}"),
    ("(quote x)", "x"),
    ("'integral", "\\mathrm{integral}"),
    ("(text area under the curve)", "\\text{area under the curve}"),
    ("(text \"area under the curve\")", "\\text{area under the curve}"),
    ("(text for n 2)", "\\text{for n 2}"),
    ("(superscript (subscript a n) (+ n 1))", "a_{n}^{n + 1}"),
    ("(bar x)", "\\bar{x}"),
    ("(overline (+ a b))", "\\overline{a + b}"),
    ("(hat x)", "\\hat{x}"),
    ("(hat (subscript v i))", "\\widehat{v_{i}}"),
    ("(dot x)", "\\dot{x}"),
    ("(ddot (subscript q i))", "\\ddot{q_{i}}"),
    ("(arrow v)", "\\vec{v}"),
    ("(arrow (+ a b))", "\\overrightarrow{a + b}"),
    ("(box (frac 1 2))", "\\boxed{\\frac{1}{2}}"),
    ("(prime f)", "f'"),
    ("(prime f 2)", "f''"),
    ("(prime f 0)", "f"),
    ("(prime f n)", "f^{(n)}"),
    ("(prime f 9)", "f^{(9)}"),
    ("(funcapp f x y)", "f\\left(x, y\\right)"),
    ("(fapp (+ f g) x)", "\\left(f + g\\right)\\left(x\\right)"),
    ("(funcapp f)", "f"),
    ("(mat (a b) (c d))", "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}"),
    ("(matrix (1 0) (0 1))", "\\begin{pmatrix} 1 & 0 \\\\ 0 & 1 \\end{pmatrix}"),
    ("(mat (a) b)", "\\begin{pmatrix} a \\\\ b \\end{pmatrix}"),
    ("(mat)", "\\begin{pmatrix}\\end{pmatrix}"),
    ("(vec x y z)", "\\begin{pmatrix} x \\\\ y \\\\ z \\end{pmatrix}"),
    ("(det (a b) (c d))", "\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}"),
    (
        "(table (header Name Age) (row Alice 30) (row Bob 25))",
        "\\begin{array}{|l|l|} \\hline \\mathbf{Name} & \\mathbf{Age} \\\\ \\hline \\mathrm{Alice} & 30 \\\\ \\hline \
         \\mathrm{Bob} & 25 \\\\ \\hline \\end{array}",
    ),
    (
        "(table (style clean) (align left right) (header n f) (row 1 (frac 1 2)))",
        "\\begin{array}{|lr|} \\hline \\mathbf{n} & \\mathbf{f} \\\\ \\hline 1 & \\frac{1}{2} \\\\ \\hline \\end{array}",
    ),
    (
        "(table (style clean) (row a b) (row c d))",
        "\\begin{array}{|ll|} \\hline a & b \\\\ c & d \\\\ \\hline \\end{array}",
    ),
    ("(table (style plain) (a b) (c d))", "\\begin{array}{ll} a & b \\\\ c & d \\end{array}"),
    ("(table (align center right) (a b c))", "\\begin{array}{|c|r|r|} \\hline a & b & c \\\\ \\hline \\end{array}"),
    ("(table)", "\\begin{array}{|l|} \\hline \\end{array}"),
    ("(ty-judge T-Var ((subscript x i)) (+ x y))", "{\\dfrac{x_{i}}{x + y} \\; \\text{T-Var}}"),
    (
        "(ty-judge \"T-App\" ((+ a b) (+ c d)) (frac a b))",
        "{\\dfrac{a + b \\qquad c + d}{\\frac{a}{b}} \\; \\text{T-App}}",
    ),
    ("(ty-judge R () c)", "{\\dfrac{}{c} \\; \\text{R}}"),
    ("(bb R)", "\\mathbb{R}"),
    ("(bb (+ a b))", "\\mathbb{a + b}"),
    ("(cal L)", "\\mathcal{L}"),
    ("(cal (subscript F n))", "\\mathcal{F_{n}}"),
    ("(frak g)", "\\mathfrak{g}"),
    ("(bold v)", "\\mathbf{v}"),
    ("(bold gen)", "\\mathbf{gen}"),
    ("(rm d)", "\\mathrm{d}"),
    (
        "(cases (x (text if x is positive)) (0 (text otherwise)))",
        "\\begin{cases} x & \\text{if x is positive} \\\\ 0 & \\text{otherwise} \\end{cases}",
    ),
    ("(cases (a b) (c))", "\\begin{cases} a & b \\\\ c \\end{cases}"),
];

#[test]
fn keyword_outputs() {
    for &(src, expected) in OUTPUTS {
        let out = latex(src);
        assert_eq!(out, expected, "{src}");
        assert_sane(src, &out);
    }
}

#[test]
fn every_keyword_has_an_exact_output() {
    for keyword in registry() {
        assert!(
            OUTPUTS.iter().any(|(src, _)| {
                let head = read(src).head().map(|h| h.as_str().to_owned());
                head.is_some_and(|h| keyword.names().any(|n| n == h))
            }),
            "`{}` has no exact-output test",
            keyword.name
        );
    }
}

// --- Atoms and dispatch ----------------------------------------------------------

#[test]
fn atoms() {
    shows(&[
        ("3.14", "3.14"),
        ("1e-9", "1e-9"),
        ("-2", "-2"),
        ("x", "x"),
        ("gen", "\\mathrm{gen}"),
        ("x_1", "\\mathrm{x\\_1}"),
        ("<>", "<>"),
        ("|", "|"),
        ("\"hello world\"", "\\text{hello world}"),
        ("()", "{}"),
    ]);
}

#[test]
fn unicode_names_use_the_display_table() {
    shows(&[("\u{3b1}", "\\alpha"), ("(+ \u{3c0} a)", "\\pi + a")]);
}

#[test]
fn reader_error_node() {
    let mut diags = Diags::new();
    let out = crate::to_latex(&Node::error(Span::DUMMY), &mut diags);
    assert_eq!(out, "\\text{[error]}");
    assert!(diags.is_empty(), "the reader already reported it");
}

#[test]
fn function_application() {
    shows(&[
        ("(f x)", "f\\left(x\\right)"),
        ("(gen (G) t)", "\\mathrm{gen}\\left(G, t\\right)"),
        ("(G)", "G"),
        ("((subscript f 1) x)", "f_{1}\\left(x\\right)"),
        ("((+ f g) x)", "\\left(f + g\\right)\\left(x\\right)"),
        ("((+ f g))", "f + g"),
        ("(f ())", "f\\left({}\\right)"),
        ("(f (frac 1 2) y)", "f\\left(\\frac{1}{2}, y\\right)"),
        ("(f x y z)", "f\\left(x, y, z\\right)"),
        ("(Gamma x)", "\\Gamma\\left(x\\right)"),
    ]);
}

#[test]
fn bare_keyword_without_a_sign() {
    let src = "(f frac)";
    let (out, diags) = render(src);
    assert_eq!(out, "f\\left(\\text{[error: 'frac' needs arguments]}\\right)");
    let [diag] = diags.as_slice() else { panic!("{diags:?}") };
    assert_eq!(diag.code, DiagCode::BareKeyword);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.span, read(src).args()[0].span);
    assert_eq!(diag.help.as_deref(), Some("write it as (frac num den)"));

    let (out, diags) = render("(frac)");
    assert!(out.starts_with("\\text{[error:"), "{out}");
    assert_eq!(diags.as_slice()[0].code, DiagCode::BareKeyword);
}

#[test]
fn zero_argument_calls_are_the_bare_keyword() {
    shows(&[
        ("(in)", "\\in"),
        ("(arrow)", "\\to"),
        ("(forall)", "\\forall"),
        ("(partial)", "\\partial"),
        ("(neq)", "\\neq"),
        ("(+)", "+"),
        ("(prime)", "'"),
        ("(^ (neq) 2)", "\\neq^{2}"),
        ("(* a (+))", "a \\times +"),
        ("(set)", "\\left\\{\\right\\}"),
    ]);
}

#[test]
fn did_you_mean() {
    let src = "(intgral f x)";
    let (out, diags) = render(src);
    assert_eq!(out, "\\mathrm{intgral}\\left(f, x\\right)", "still rendered, as a function application");
    let [diag] = diags.as_slice() else { panic!("{diags:?}") };
    assert_eq!(diag.code, DiagCode::UnknownForm);
    assert_eq!(diag.severity, Severity::Hint);
    assert_eq!(diag.span, read(src).as_list().unwrap()[0].span);
    assert_eq!(diag.help.as_deref(), Some("did you mean `integral`?"));

    // The same hint MathML gives, for the same inputs.
    for src in ["(farc 1 2)", "(intgral f x)", "(f x)", "(min a b)", "(sinh x)", "(Laplace f)"] {
        let node = read(src);
        let (mut math, mut tex) = (Diags::new(), Diags::new());
        crate::to_mathml(&node, &mut math);
        crate::to_latex(&node, &mut tex);
        assert_eq!(math, tex, "{src}");
    }
}

#[test]
fn errors_stay_local() {
    let (out, diags) = render("(+ (frac 1) x)");
    assert_eq!(out, "\\text{[error: 'frac' is missing its 'den' argument]} + x");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags.as_slice()[0].code, DiagCode::Arity);

    let (out, diags) = with(|o| o.error(mazer_syntax::Diag::error(DiagCode::Other, Span::DUMMY, "a < b & c")));
    assert_eq!(out, "\\text{[error: a \\textless{} b \\& c]}");
    assert_eq!(diags.as_slice()[0].message, "a < b & c");
}

#[test]
fn same_diagnostics_as_mathml() {
    for src in [
        "(+ (frac 1) (intgral f x) frac (sin 1 2))",
        "(table (style bogus) (row a b))",
        "(cases (a b c))",
        "(frac 1 2 3)",
    ] {
        let node = read(src);
        let (mut math, mut tex) = (Diags::new(), Diags::new());
        crate::to_mathml(&node, &mut math);
        crate::to_latex(&node, &mut tex);
        assert_eq!(math, tex, "{src}");
    }
}

#[test]
fn keyword_without_an_emitter_is_unsupported() {
    // The stand-in registry is built without `latex::attach`, so no keyword
    // has an emitter in it.
    with_standins(|| {
        let src = "(f (ty-judge T () c))";
        let (out, diags) = render(src);
        assert_eq!(out, "f\\left(\\text{[error: 'ty-judge' has no LaTeX output yet]}\\right)");
        let [diag] = diags.as_slice() else { panic!("{diags:?}") };
        assert_eq!(diag.code, DiagCode::UnsupportedTarget);
        assert_eq!(diag.span, read(src).args()[0].span);
        assert_eq!(diag.message, "`ty-judge` has no LaTeX output yet");

        // Arity is checked first.
        let (_, diags) = render("(frac 1)");
        assert_eq!(diags.as_slice()[0].code, DiagCode::Arity);
        // A bare sign needs no emitter.
        assert_eq!(latex("integral"), "\\int");
    });
}

// --- Escaping and spacing -------------------------------------------------------

#[test]
fn text_is_escaped() {
    let (out, _) = with(|o| o.text("\\ { } $ & # _ % ^ ~"));
    assert_eq!(
        out,
        "\\textbackslash{} \\{ \\} \\$ \\& \\# \\_ \\% \\textasciicircum{} \\textasciitilde{}"
    );
    let (out, _) = with(|o| o.text("a < b | c > d\ne"));
    assert_eq!(out, "a \\textless{} b \\textbar{} c \\textgreater{} d e");

    shows(&[
        ("\"100% of $5 #1 {x} a_b\"", "\\text{100\\% of \\$5 \\#1 \\{x\\} a\\_b}"),
        ("(text \"a & b\" c_d)", "\\text{a \\& b c\\_d}"),
        ("\"back\\\\slash ^ ~\"", "\\text{back\\textbackslash{}slash \\textasciicircum{} \\textasciitilde{}}"),
        ("(text)", "\\text{}"),
        // Names with special characters stay valid in math mode.
        ("a%b", "\\mathrm{a\\%b}"),
        ("(bold a#b)", "\\mathbf{a\\#b}"),
    ]);
    assert_sane("text", &latex("\"100% of $5 #1 {x} a_b\""));
}

#[test]
fn control_words_never_fuse() {
    let (out, _) = with(|o| {
        o.raw("\\int");
        o.raw("f");
        o.raw("\\\\");
        o.raw("a");
        o.raw("\\,");
        o.raw("b");
        o.raw("1");
    });
    assert_eq!(out, "\\int f\\\\a\\,b1");
    shows(&[
        ("(integral f x)", "\\int f \\, \\mathrm{d}x"),
        ("(forall x P)", "\\forall x.\\, P"),
        ("(not p)", "\\lnot p"),
        ("(* a (forall x P))", "a \\times \\left(\\forall x.\\, P\\right)"),
        ("(floor x)", "\\left\\lfloor x\\right\\rfloor"),
        ("(+ tau x)", "\\tau + x"),
        ("(jux tau x)", "\\tau \\, x"),
        ("(f tau x)", "f\\left(\\tau, x\\right)"),
        ("(sin tau)", "\\sin\\left(\\tau\\right)"),
    ]);
}

#[test]
fn building_blocks() {
    let (out, _) = with(|o| {
        o.frac(&read("a"), &read("(+ b c)"));
        o.raw(" ");
        o.sub(&read("x"), &read("i"));
        o.raw(" ");
        o.sup(&read("x"), &read("(+ n 1)"));
        o.raw(" ");
        o.group(|o| o.raw("z"));
        o.raw(" ");
        o.fence("[", ")", |o| o.raw("t"));
        o.fence("", "|", |o| o.raw("t"));
    });
    assert_eq!(
        out,
        "\\frac{a}{b + c} x_{i} x^{n + 1} {z} \\left[t\\right)\\left.t\\right|"
    );
    // A base that is not an atom is parenthesised; a script on a script is braced.
    shows(&[
        ("(subscript (subscript x i) j)", "{x_{i}}_{j}"),
        ("(superscript (superscript x i) j)", "\\left(x^{i}\\right)^{j}"),
        ("(subscript (superscript x i) j)", "\\left(x^{i}\\right)_{j}"),
        ("(superscript (subscript x i) j)", "x_{i}^{j}"),
    ]);
}

// --- Parentheses, mirroring `mathml.rs` ------------------------------------------

#[test]
fn font_variants_hold_together_like_their_argument() {
    shows(&[
        ("(- (rm (+ a b)))", "-\\left(\\mathrm{a + b}\\right)"),
        ("(^ (bb (+ a b)) 2)", "\\left(\\mathbb{a + b}\\right)^{2}"),
        ("(* c (bold (+ a b)))", "c \\times \\left(\\mathbf{a + b}\\right)"),
        ("(fact (cal (* a b)))", "\\left(\\mathcal{a \\times b}\\right)!"),
        ("(sum (bb (+ a b)) n)", "\\sum_{n} \\left(\\mathbb{a + b}\\right)"),
        ("(- (bb x))", "-\\mathbb{x}"),
        ("(^ (bold x) 2)", "\\mathbf{x}^{2}"),
        ("(+ a (bb -2))", "a + \\left(\\mathbb{-2}\\right)"),
        ("(* (sum a n) (rm b))", "\\left(\\sum_{n} a\\right) \\times \\mathrm{b}"),
    ]);
}

#[test]
fn zero_argument_application_takes_its_heads_level() {
    shows(&[
        ("(* a ((+ b c)))", "a \\times \\left(b + c\\right)"),
        ("(^ (funcapp (+ f g)) 2)", "\\left(f + g\\right)^{2}"),
        ("(+ a ((* b c)))", "a + b \\times c"),
        ("(^ ((Gamma)) 2)", "\\Gamma^{2}"),
        ("(^ ((+ f g) x) 2)", "\\left(f + g\\right)\\left(x\\right)^{2}"),
    ]);
}

#[test]
fn prime_with_count_zero_is_not_there() {
    shows(&[
        ("(^ (prime (+ a b) 0) 2)", "\\left(a + b\\right)^{2}"),
        ("(- (prime (+ a b) 0))", "-\\left(a + b\\right)"),
        ("(prime (prime f 0))", "f'"),
    ]);
}

#[test]
fn postfix_operands() {
    shows(&[
        ("(fact (fact n))", "\\left(n!\\right)!"),
        ("(prime (fact n))", "n!'"),
        ("(prime (prime f))", "f''"),
        ("(fact (* a b))", "\\left(a \\times b\\right)!"),
    ]);
}

#[test]
fn big_operators_and_products() {
    shows(&[
        ("(* (sum a n) b)", "\\left(\\sum_{n} a\\right) \\times b"),
        ("(* b (sum a n))", "b \\times \\sum_{n} a"),
        ("(* 2 (sum a n) b)", "2 \\times \\left(\\sum_{n} a\\right) \\times b"),
        (
            "(^ (derivative f x) 2)",
            "\\left(\\frac{\\mathrm{d}}{\\mathrm{d}x} f\\right)^{2}",
        ),
        ("(- (frac a b))", "-\\frac{a}{b}"),
    ]);
}

#[test]
fn quantifier_inside_and() {
    shows(&[
        ("(and (forall x (= x y)) p)", "\\left(\\forall x.\\, x = y\\right) \\land p"),
        ("(and p (forall x (= x y)))", "p \\land \\left(\\forall x.\\, x = y\\right)"),
        (
            "(and (exists x (< x y)) (forall z (in z S)))",
            "\\left(\\exists x.\\, x < y\\right) \\land \\left(\\forall z.\\, z \\in S\\right)",
        ),
    ]);
}

#[test]
fn accents_and_primes_as_script_bases() {
    shows(&[
        ("(^ (hat x) 2)", "\\hat{x}^{2}"),
        ("(^ (bar x) 2)", "\\bar{x}^{2}"),
        ("(subscript (hat x) i)", "\\hat{x}_{i}"),
        ("(subscript (bar x) i)", "\\bar{x}_{i}"),
        ("(^ (prime f) 2)", "\\left(f'\\right)^{2}"),
        ("(subscript (prime f) i)", "\\left(f'\\right)_{i}"),
    ]);
}

#[test]
fn text_and_fonts_as_operands() {
    shows(&[
        ("(+ (text hi) a)", "\\text{hi} + a"),
        ("(* (text hi) a)", "\\text{hi} \\times a"),
        ("(^ (text hi) 2)", "\\text{hi}^{2}"),
        ("(+ (bb R) a)", "\\mathbb{R} + a"),
        ("(* (bb R) a)", "\\mathbb{R} \\times a"),
        ("(^ (bb R) 2)", "\\mathbb{R}^{2}"),
    ]);
}

#[test]
fn grids_hold_any_expression_in_a_cell() {
    shows(&[
        (
            "(cases ((+ a b) (text if p)) ((frac a b) q) ((- x) r))",
            "\\begin{cases} a + b & \\text{if p} \\\\ \\frac{a}{b} & q \\\\ -x & r \\end{cases}",
        ),
        (
            "(mat ((+ a b) (frac a b)) ((- x) c))",
            "\\begin{pmatrix} a + b & \\frac{a}{b} \\\\ -x & c \\end{pmatrix}",
        ),
        (
            "(table (row (+ a b) (frac a b) (- x)))",
            "\\begin{array}{|l|l|l|} \\hline a + b & \\frac{a}{b} & -x \\\\ \\hline \\end{array}",
        ),
        (
            "(table (header (+ a b) (- x)) (row (frac a b) c))",
            "\\begin{array}{|l|l|} \\hline \\mathbf{a + b} & \\mathbf{-x} \\\\ \\hline \\frac{a}{b} & c \\\\ \\hline \\end{array}",
        ),
    ]);
}

#[test]
fn logic_levels() {
    shows(&[
        ("(and (or a b) c)", "\\left(a \\lor b\\right) \\land c"),
        ("(and c (or a b))", "c \\land \\left(a \\lor b\\right)"),
        ("(or (and a b) c)", "a \\land b \\lor c"),
        ("(and (and a b) c)", "a \\land b \\land c"),
        ("(implies (implies a b) c)", "\\left(a \\implies b\\right) \\implies c"),
        ("(implies a (implies b c))", "a \\implies \\left(b \\implies c\\right)"),
        ("(iff (implies a b) c)", "\\left(a \\implies b\\right) \\iff c"),
        ("(implies (or a b) (and c d))", "a \\lor b \\implies c \\land d"),
        ("(and (= a b) c)", "a = b \\land c"),
        ("(and (not a) b)", "\\lnot a \\land b"),
        ("(not (and a b))", "\\lnot\\left(a \\land b\\right)"),
        ("(not (not p))", "\\lnot\\lnot p"),
        ("(not (= x y))", "\\lnot x = y"),
        ("(= (not a) b)", "\\left(\\lnot a\\right) = b"),
        ("(= a b c)", "a = b = c"),
        ("(= (= a b) c)", "\\left(a = b\\right) = c"),
    ]);
}

#[test]
fn quantifiers() {
    shows(&[
        ("(forall x (implies P Q))", "\\forall x.\\, P \\implies Q"),
        ("(forall x (exists y (and P Q)))", "\\forall x.\\, \\exists y.\\, P \\land Q"),
        ("(and (forall x P) Q)", "\\left(\\forall x.\\, P\\right) \\land Q"),
        ("(and Q (exists x P))", "Q \\land \\left(\\exists x.\\, P\\right)"),
        ("(implies (exists x P) Q)", "\\left(\\exists x.\\, P\\right) \\implies Q"),
        ("(not (forall x P))", "\\lnot\\left(\\forall x.\\, P\\right)"),
    ]);
}

#[test]
fn signs() {
    shows(&[
        ("(+ a -2)", "a + \\left(-2\\right)"),
        ("(+ -2)", "+\\left(-2\\right)"),
        ("(+ (+ a))", "+\\left(+a\\right)"),
        ("(- a -2)", "a - \\left(-2\\right)"),
        ("(- (- x))", "-\\left(-x\\right)"),
        ("(jux a (- b))", "a \\, \\left(-b\\right)"),
        ("(* a -2)", "a \\times \\left(-2\\right)"),
        ("(+ -2 a)", "-2 + a"),
        ("(- (- a) b)", "-a - b"),
        ("(* -s t)", "-s \\times t"),
        ("(jux -2 x)", "-2 \\, x"),
        ("(= x -2)", "x = -2"),
        ("(= y (- x))", "y = -x"),
        ("(+ b (+ -2 a))", "b + \\left(-2 + a\\right)"),
        ("(+ b (* -2 a))", "b + \\left(-2 \\times a\\right)"),
        ("(- (* -2 a))", "-\\left(-2 \\times a\\right)"),
        ("(- (* a b))", "-a \\times b"),
        ("(- (+ a b))", "-\\left(a + b\\right)"),
        ("(* (- a) b)", "-a \\times b"),
        ("(^ (- a) 2)", "\\left(-a\\right)^{2}"),
        ("(^ -2 2)", "\\left(-2\\right)^{2}"),
        ("(- a (- b c))", "a - \\left(b - c\\right)"),
        ("(- (- a b) c)", "a - b - c"),
        ("(- a (+ b c))", "a - \\left(b + c\\right)"),
    ]);
}

#[test]
fn big_operator_parentheses() {
    shows(&[
        ("(subscript (integral f x) 2)", "\\left(\\int f \\, \\mathrm{d}x\\right)_{2}"),
        ("(jux (sum a n) b)", "\\left(\\sum_{n} a\\right) \\, b"),
        ("(jux b (sum a n))", "b \\, \\sum_{n} a"),
        ("(* (* b (sum a n)) c)", "\\left(b \\times \\sum_{n} a\\right) \\times c"),
        ("(* (- (sum a n)) b)", "\\left(-\\sum_{n} a\\right) \\times b"),
        ("(+ (sum a n) b)", "\\sum_{n} a + b"),
        ("(+ b (sum a n))", "b + \\sum_{n} a"),
        ("(- (sum a n))", "-\\sum_{n} a"),
        ("(sum (+ a b) n)", "\\sum_{n} \\left(a + b\\right)"),
        ("(sum (- a) n)", "\\sum_{n} \\left(-a\\right)"),
        ("(sum (* a b) n)", "\\sum_{n} a \\times b"),
        ("(sum (sum a m) n)", "\\sum_{n} \\sum_{m} a"),
        ("(integral (sum a n) x)", "\\int \\sum_{n} a \\, \\mathrm{d}x"),
        ("(integral (- f) x)", "\\int \\left(-f\\right) \\, \\mathrm{d}x"),
        ("(^ (sum a n) 2)", "\\left(\\sum_{n} a\\right)^{2}"),
        ("(fact (sum a n))", "\\left(\\sum_{n} a\\right)!"),
    ]);
}

#[test]
fn postfix_and_scripts() {
    shows(&[
        ("(^ (fact n) 2)", "\\left(n!\\right)^{2}"),
        ("(fact (^ x 2))", "\\left(x^{2}\\right)!"),
        ("(fact (+ n 1))", "\\left(n + 1\\right)!"),
        ("(fact (- n))", "\\left(-n\\right)!"),
        ("(fact (subscript x i))", "x_{i}!"),
        ("(prime (^ x 2))", "\\left(x^{2}\\right)'"),
        ("(^ (^ x 2) 3)", "\\left(x^{2}\\right)^{3}"),
        ("(^ (subscript x i) 2)", "x_{i}^{2}"),
        ("(subscript (^ x 2) i)", "\\left(x^{2}\\right)_{i}"),
        ("(^ (frac a b) 2)", "\\left(\\frac{a}{b}\\right)^{2}"),
        ("(subscript (frac a b) i)", "\\left(\\frac{a}{b}\\right)_{i}"),
        ("(fact (frac a b))", "\\left(\\frac{a}{b}\\right)!"),
        ("(* (frac a b) c)", "\\frac{a}{b} \\times c"),
    ]);
}

#[test]
fn application_heads() {
    shows(&[
        ("((+ f g) x)", "\\left(f + g\\right)\\left(x\\right)"),
        ("((prime f) x)", "f'\\left(x\\right)"),
        ("((^ f -1) x)", "f^{-1}\\left(x\\right)"),
        ("((frac a b) x)", "\\left(\\frac{a}{b}\\right)\\left(x\\right)"),
        ("((subscript f 1) x)", "f_{1}\\left(x\\right)"),
    ]);
}

#[test]
fn quote_is_transparent() {
    shows(&[
        ("(* (quote (+ a b)) c)", "\\left(a + b\\right) \\times c"),
        ("(+ a '-2)", "a + \\left(-2\\right)"),
    ]);
}

#[test]
fn table_directives_are_never_applications() {
    let (out, diags) = render("(table (align right) (style bogus) (row a b))");
    assert_eq!(out, "\\begin{array}{|r|r|} \\hline a & b \\\\ \\hline \\end{array}");
    let [diag] = diags.as_slice() else { panic!("{diags:?}") };
    assert_eq!(diag.severity, Severity::Warning);
    assert!(!out.contains("align") && !out.contains("style"));
    // The last alignment repeats; unknown words are left-aligned.
    shows(&[
        ("(table (align right left) (a b c))", "\\begin{array}{|r|l|l|} \\hline a & b & c \\\\ \\hline \\end{array}"),
        ("(table (style plain) (align wobble) (a b))", "\\begin{array}{ll} a & b \\end{array}"),
    ]);
}

/// Writes one `.tex` document with every example of every keyword (and every
/// exact output, and every display name) as a display equation, to the path in
/// `MAZER_LATEX_OUT`. Run by hand, then compile the file:
///
/// ```text
/// MAZER_LATEX_OUT=/tmp/all.tex cargo test -p mazer-math --features latex \
///     write_all_examples_tex -- --ignored
/// pdflatex -interaction=nonstopmode -halt-on-error /tmp/all.tex
/// ```
#[test]
#[ignore = "writes a file; run by hand"]
fn write_all_examples_tex() {
    let Ok(path) = std::env::var("MAZER_LATEX_OUT") else {
        panic!("set MAZER_LATEX_OUT to the .tex file to write");
    };
    let mut doc = String::from(
        "\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n\\begin{document}\n\\allowdisplaybreaks\n",
    );
    let mut push = |src: &str| {
        let (out, diags) = render(src);
        assert!(diags.is_empty(), "{src}: {diags:?}");
        let _ = writeln!(doc, "% {}\n\\[\n{out}\n\\]", src.replace('\n', " "));
    };
    for keyword in registry() {
        for src in &keyword.examples {
            push(src);
        }
    }
    for (src, _) in OUTPUTS {
        push(src);
    }
    for display in display_names() {
        push(display.name);
    }
    for src in [
        "(subscript (subscript x i) j)",
        "(superscript (superscript x i) j)",
        "(^ degree 2)",
        "(subscript (lim x 0) 2)",
        "\"100% of $5 #1 {x} a_b < c\"",
        "(* (sum a n) (- (integral f x 0 1)) (forall y p))",
        "(f (ty-judge T ((in x S) (= a b)) c) x_1 a%b)",
    ] {
        push(src);
    }
    doc.push_str("\\end{document}\n");
    std::fs::write(path, doc).expect("write the .tex file");
}

#[test]
fn directive_lists_in_grids() {
    // `mat` rows that look like keyword calls are cells, not calls.
    assert_eq!(latex("(mat (a b))"), "\\begin{pmatrix} a & b \\end{pmatrix}");
    let (out, diags) = render("(cases (a b c))");
    assert_eq!(out, "\\begin{cases} a & b \\end{cases}");
    assert_eq!(diags.as_slice()[0].severity, Severity::Warning);
}
