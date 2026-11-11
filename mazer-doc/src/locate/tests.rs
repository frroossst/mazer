use mazer_syntax::{ByteOffset, Sym};

use super::*;
use crate::pipeline::analyze;

/// What a cursor should be, described by source text.
#[derive(Debug, PartialEq, Eq)]
enum Want {
    Prose,
    /// The text of the list.
    Head(&'static str),
    Arg(&'static str, usize),
    /// The text of the identifier.
    Name(&'static str),
    Ref(&'static str),
}

/// Strips the `|` marking the cursor and locates it.
fn at(marked: &str) -> String {
    let offset = marked.find('|').expect("a | marks the cursor");
    let src = marked.replacen('|', "", 1);
    let analysis = analyze(&src, "t.zr");
    let cursor = locate(&analysis, ByteOffset::from_usize(offset));
    let text = |node: &Node| analysis.sources.text(node.span).to_owned();
    match cursor {
        Cursor::Prose => "Prose".to_owned(),
        Cursor::Head { list } => format!("Head({:?})", text(list)),
        Cursor::Arg { list, index } => format!("Arg({:?}, {index})", text(list)),
        Cursor::Name { ident } => format!("Name({:?})", text(ident)),
        Cursor::Ref { partial } => format!("Ref({partial:?})"),
    }
}

fn want(w: &Want) -> String {
    match w {
        Want::Prose => "Prose".to_owned(),
        Want::Head(t) => format!("Head({t:?})"),
        Want::Arg(t, i) => format!("Arg({t:?}, {i})"),
        Want::Name(t) => format!("Name({t:?})"),
        Want::Ref(t) => format!("Ref({t:?})"),
    }
}

#[test]
fn cursor_positions() {
    use Want::{Arg, Head, Name, Prose, Ref};
    let cases: &[(&str, Want)] = &[
        // Prose and the edges of a form.
        ("hello | world", Prose),
        ("|(show x)", Prose),
        ("(show x)|", Prose),
        ("(show x)  |  more", Prose),
        ("```\n(show |x)\n```", Prose),
        ("`(show |x)`", Prose),
        // Heads, reserved or not.
        ("(|show x)", Head("(show x)")),
        ("(sh|ow x)", Head("(show x)")),
        ("(show| x)", Head("(show x)")),
        ("(show (|frac a b))", Head("(frac a b)")),
        ("(show (fr|ac a b))", Head("(frac a b)")),
        ("(show (frac| a b))", Head("(frac a b)")),
        ("(show (|))", Head("()")),
        ("(show ((f| x) y))", Head("(f x)")),
        ("See (r|ef x).", Head("(ref x)")),
        // Names.
        ("a (show |x) b", Name("x")),
        ("(show (frac a |b))", Name("b")),
        ("(show (+ (frac a b) |c))", Name("c")),
        ("(show (+ |alpha b))", Name("alpha")),
        ("(show (+ al|pha b))", Name("alpha")),
        ("(show '|x)", Name("x")),
        ("(show (+ -|x y))", Name("x")),
        // Arguments.
        ("a (show x|) b", Arg("(show x)", 0)),
        ("(show (frac a| b))", Arg("(frac a b)", 0)),
        ("(show (frac a  | b))", Arg("(frac a   b)", 1)),
        ("(show (frac a b |))", Arg("(frac a b )", 2)),
        ("(show (frac |))", Arg("(frac )", 0)),
        ("(show (frac a b)|)", Arg("(show (frac a b))", 0)),
        ("(show |(frac a b))", Arg("(show (frac a b))", 0)),
        ("(show | x)", Arg("(show  x)", 0)),
        ("(show (+ 1|2 x))", Arg("(+ 12 x)", 0)),
        ("(show (+ |\"st\" x))", Arg("(+ \"st\" x)", 0)),
        ("(show (+ \"st\"| x))", Arg("(+ \"st\" x)", 0)),
        ("(show |'x)", Arg("(show 'x)", 0)),
        ("(show (+ a |; note\n b))", Arg("(+ a ; note\n b)", 1)),
        ("(show (+ a ; note\n |b))", Name("b")),
        ("(show (+ a ; note\n| b))", Arg("(+ a ; note\n b)", 1)),
        // Comments and strings are prose.
        ("(show (+ a ; note|\n b))", Prose),
        ("(show (+ a ;| note\n b))", Prose),
        ("(show (+ a ; (fr|ac\n b))", Prose),
        ("(show (+ a ; note|\r\n b))", Prose),
        ("(show x ; trailing|", Prose),
        ("(show (+ \"s|t\" x))", Prose),
        ("(show (+ \"|st\" x))", Prose),
        ("(show (+ \"st|\" x))", Prose),
        ("(show \"a ; b|\")", Prose),
        ("(show \"a ; b\" |x)", Name("x")),
        ("(show \"a\\\"b|\")", Prose),
        ("(show \"a\\\"b\"| x)", Arg("(show \"a\\\"b\" x)", 0)),
        ("(show \"open|", Prose),
        ("(show \"open|\n\nNext", Prose),
        ("See (ref a ; c|\n).", Prose),
        ("(let x| 2)", Arg("(let x 2)", 0)),
        ("(let |x 2)", Name("x")),
        // References.
        ("See (ref la|b).", Ref("la")),
        ("See (ref lab|).", Ref("lab")),
        ("See (ref |).", Ref("")),
        ("See (ref |lab).", Ref("")),
        ("See (ref a |).", Ref("")),
        // In containers.
        ("- item (show |y)", Name("y")),
        ("# T (show y|)", Arg("(show y)", 0)),
        ("> q (show (+ a |b))", Name("b")),
        ("[link (show |z)](u)", Name("z")),
        // Multibyte text before the cursor.
        ("τ😭 (show (+ α| β))", Arg("(+ α β)", 0)),
        ("τ (show (+ |α β))", Name("α")),
        ("ü (ref é|)", Ref("é")),
        // Unclosed forms, typed mid-edit.
        ("(show (frac a|", Arg("(frac a", 0)),
        ("(show (frac a |", Arg("(frac a ", 1)),
        ("(show (|", Head("(")),
        ("(show |", Arg("(show ", 0)),
        ("(let x|", Arg("(let x", 0)),
        ("(show (frac a|\n\nNext paragraph", Arg("(frac a", 0)),
        ("(show (frac a\n\nNext| paragraph", Prose),
        ("Text (show (+ a (frac b|", Arg("(frac b", 0)),
        ("See (ref ab|", Ref("ab")),
        ("- (show (sin |", Arg("(sin ", 0)),
    ];
    let mut failures = Vec::new();
    for (src, expected) in cases {
        let got = at(src);
        if got != want(expected) {
            failures.push(format!("{src:?}: got {got}, want {}", want(expected)));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn every_offset_is_safe() {
    let inputs = [
        "Text (show (+ a (frac b c))) and (ref x).\n\n(show (tag x) y)",
        "(show (frac a",
        "(show (+ a (frac b\n\nNext (ref ab",
        "τ😭 (show (+ α β 'γ -δ)) ü (ref é) 🍁",
        "(show (+ a ; note\r\n b))\r\n\r\n- (let x ; c\r\n 2)\r\n",
        "(show (+ a ; \"quoted\" (\n b) ; end",
        "(show \"a ; b\" \"c\\\"d\" \"e\\\\\" \"open\n(show x)",
        "> (show (+ a\n> ;c\n> b))\n\n- (show '(-x \"y\"",
        "(let x 2)(let y (+ x x))(show y)(eval y)(alias A b)(macro (f v...) (g v))",
        "",
        "(",
        "\"",
        ";",
    ];
    for src in inputs {
        let analysis = analyze(src, "t.zr");
        for at in (0..=src.len() + 1).filter(|&i| i > src.len() || src.is_char_boundary(i)) {
            let _ = locate(&analysis, ByteOffset::from_usize(at));
        }
    }
}

#[test]
fn offsets_past_the_end_are_prose() {
    let analysis = analyze("(show x)", "t.zr");
    assert_eq!(locate(&analysis, ByteOffset(999)), Cursor::Prose);
    let analysis = analyze("", "t.zr");
    assert_eq!(locate(&analysis, ByteOffset(0)), Cursor::Prose);
}

#[test]
fn a_name_borrows_the_written_tree() {
    // The written tree, not the resolved one: `x` is still `x`.
    let src = "(let x 2)\n\n(show x)";
    let analysis = analyze(src, "t.zr");
    let offset = src.rfind('x').unwrap();
    match locate(&analysis, ByteOffset::from_usize(offset)) {
        Cursor::Name { ident } => assert_eq!(ident.as_ident().map(Sym::as_str), Some("x")),
        other => panic!("{other:?}"),
    }
}
