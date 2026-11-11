use super::*;
use crate::node::NodeKind;
use crate::sym::Sym;

fn read(src: &str) -> (Node, Diags) {
    let mut diags = Diags::new();
    let node = read_str(src, &mut diags);
    (node, diags)
}

fn read_ok(src: &str) -> Node {
    let (node, diags) = read(src);
    assert!(diags.is_empty(), "unexpected diagnostics for {src:?}: {diags:?}");
    node
}

fn text_of<'a>(src: &'a str, node: &Node) -> &'a str {
    &src[node.span.range()]
}

fn codes(diags: &Diags) -> Vec<DiagCode> {
    diags.iter().map(|d| d.code).collect()
}

#[test]
fn parens_are_the_only_delimiters() {
    let node = read_ok("(f(x)y)");
    assert_eq!(node.to_string(), "(f (x) y)");
    let items = node.as_list().unwrap();
    assert_eq!(items.len(), 3);
    assert!(matches!(items[1].kind, NodeKind::List(_)));
}

#[test]
fn numbers_are_stored_as_written() {
    for src in ["0", "42", "3.14", "-2", "-0.5", "1e9", "1e-9", "6.02E+23", "007"] {
        let node = read_ok(src);
        assert_eq!(node.kind, NodeKind::Number(src.into()), "{src}");
        assert_eq!(text_of(src, &node), src);
    }
}

#[test]
fn almost_numbers_are_identifiers() {
    for src in ["2x", "1.", "1e", "1.2.3", ".5", "x2", "-2x"] {
        let node = read_ok(src);
        assert_eq!(node.as_ident().map(Sym::as_str), Some(src), "{src}");
    }
}

#[test]
fn minus_glued_to_a_name_is_negation() {
    let src = "(* -s t)";
    let node = read_ok(src);
    assert_eq!(node.to_string(), "(* (- s) t)");
    let neg = &node.args()[0];
    assert_eq!(text_of(src, neg), "-s");
    let items = neg.as_list().unwrap();
    assert_eq!(text_of(src, &items[0]), "-");
    assert_eq!(text_of(src, &items[1]), "s");
    assert!(neg.is_form("-"));

    assert_eq!(read_ok("-alpha").to_string(), "(- alpha)");
    assert_eq!(read_ok("-τ").to_string(), "(- τ)");
    assert_eq!(read_ok("--x").to_string(), "(- (- x))");
    // `-` before a digit is a number, a lone `-` is a name.
    assert_eq!(read_ok("-3").kind, NodeKind::Number("-3".into()));
    assert_eq!(read_ok("(- a b)").to_string(), "(- a b)");
    assert_eq!(read_ok("-").as_ident().map(Sym::as_str), Some("-"));
    // A paren after the minus is a delimiter: `-(x)` is `-` then `(x)`.
    assert_eq!(read_ok("(-(x))").to_string(), "(- (x))");
}

#[test]
fn quote_shorthand() {
    let src = "(= 'x x)";
    let node = read_ok(src);
    assert_eq!(node.to_string(), "(= (quote x) x)");
    let quoted = &node.args()[0];
    assert_eq!(text_of(src, quoted), "'x");
    assert_eq!(text_of(src, &quoted.as_list().unwrap()[0]), "'");

    let node = read_ok("'(a b)");
    assert_eq!(node.to_string(), "(quote (a b))");
    assert_eq!(read_ok("''a").to_string(), "(quote (quote a))");
}

#[test]
fn quote_with_nothing_after_it() {
    let (node, diags) = read("(f ')");
    assert_eq!(node.to_string(), "(f (quote <error>))");
    assert_eq!(codes(&diags), vec![DiagCode::Other]);
}

#[test]
fn strings_and_escapes() {
    let src = r#"(text "a \"b\" c\\d \n")"#;
    let node = read_ok(src);
    let s = &node.args()[0];
    assert_eq!(s.kind, NodeKind::Str(r#"a "b" c\d \n"#.into()));
    assert_eq!(text_of(src, s), r#""a \"b\" c\\d \n""#);
    // Parens and semicolons inside a string are just text.
    let node = read_ok(r#"(text "(;)")"#);
    assert_eq!(node.args()[0].kind, NodeKind::Str("(;)".into()));
}

#[test]
fn unterminated_string_recovers_at_end_of_line() {
    let src = "(text \"abc)\n (b))";
    let (node, diags) = read(src);
    assert_eq!(codes(&diags), vec![DiagCode::UnterminatedString]);
    let items = node.as_list().unwrap();
    assert_eq!(items[1].kind, NodeKind::Str("abc)".into()));
    assert_eq!(text_of(src, &items[1]), "\"abc)");
    // Reading went on after the broken string.
    assert_eq!(items[2].to_string(), "(b)");
    assert_eq!(items.len(), 3);
}

#[test]
fn comments_run_to_end_of_line() {
    let node = read_ok("(+ 1 ; one\n   2) ; trailing");
    assert_eq!(node.to_string(), "(+ 1 2)");
    // `;` inside an atom is part of it.
    assert_eq!(read_ok("a;b").as_ident().map(Sym::as_str), Some("a;b"));
}

#[test]
fn quotes_and_strings_end_atoms_but_primes_do_not() {
    assert_eq!(read_ok("(show'x)").to_string(), "(show (quote x))");
    assert_eq!(read_ok("(text\"a\")").to_string(), "(text \"a\")");
    assert_eq!(read_ok("(prime f')").args()[0].as_ident().map(Sym::as_str), Some("f'"));
    assert_eq!(read_ok("(= f'' g')").to_string(), "(= f'' g')");
    // A paren is a delimiter, so `f'(x)` is `f'` applied to `(x)`.
    assert_eq!(read_ok("(f'(x))").to_string(), "(f' (x))");
    let (node, diags) = read("f'x");
    assert_eq!(node.to_string(), "f");
    assert_eq!(codes(&diags), vec![DiagCode::Other]);
}

#[test]
fn unclosed_paren_points_at_the_opening_paren() {
    let src = "  (show (frac a b)";
    let (node, diags) = read(src);
    assert_eq!(codes(&diags), vec![DiagCode::UnclosedParen]);
    assert_eq!(&src[diags.as_slice()[0].span.range()], "(");
    assert_eq!(diags.as_slice()[0].span.start, ByteOffset(2));
    // The list is closed at the end of the input and holds an Error node.
    assert_eq!(node.to_string(), "(show (frac a b) <error>)");
    assert_eq!(node.span.range(), 2..src.len());
}

#[test]
fn nested_unclosed_parens_each_get_a_diagnostic() {
    let src = "(show (frac a";
    let (node, diags) = read(src);
    assert_eq!(codes(&diags), vec![DiagCode::UnclosedParen, DiagCode::UnclosedParen]);
    let starts: Vec<usize> = diags.iter().map(|d| d.span.start.to_usize()).collect();
    assert_eq!(starts, vec![6, 0]);
    assert_eq!(node.to_string(), "(show (frac a <error>))");
}

#[test]
fn read_form_reads_one_form_and_reports_its_end() {
    let text = "see (show (+ 1 2)) here";
    let mut diags = Diags::new();
    let read = read_form(text, FileId(3), ByteOffset(4), &mut diags);
    assert!(diags.is_empty());
    assert_eq!(read.node.to_string(), "(show (+ 1 2))");
    assert_eq!(read.end, ByteOffset(18));
    assert_eq!(read.node.span.file, FileId(3));
    assert_eq!(&text[read.node.span.range()], "(show (+ 1 2))");
}

#[test]
fn read_form_spans_many_lines() {
    let text = "(show\n    (=\n        a ; note\n        b)\n)\nafter";
    let mut diags = Diags::new();
    let read = read_form(text, FileId(0), ByteOffset(0), &mut diags);
    assert!(diags.is_empty());
    assert_eq!(read.node.to_string(), "(show (= a b))");
    assert_eq!(&text[read.end.to_usize()..], "\nafter");
}

#[test]
fn read_form_with_bad_start_does_not_panic() {
    let text = "τ (show x)";
    let mut diags = Diags::new();
    let read = read_form(text, FileId(0), ByteOffset(1), &mut diags);
    assert_eq!(read.node.kind, NodeKind::Error);
    assert_eq!(diags.len(), 1);
    let read = read_form(text, FileId(0), ByteOffset(999), &mut diags);
    assert_eq!(read.node.kind, NodeKind::Error);
}

#[test]
fn spans_with_multibyte_text() {
    let src = "(= τ 😭 (f αβ) \"π\")";
    let node = read_ok(src);
    let items = node.as_list().unwrap();
    assert_eq!(text_of(src, &items[1]), "τ");
    assert_eq!(text_of(src, &items[2]), "😭");
    assert_eq!(text_of(src, &items[3]), "(f αβ)");
    assert_eq!(text_of(src, &items[3].args()[0]), "αβ");
    assert_eq!(text_of(src, &items[4]), "\"π\"");
    for item in items {
        assert!(src.is_char_boundary(item.span.start.to_usize()));
        assert!(src.is_char_boundary(item.span.end.to_usize()));
    }
}

#[test]
fn read_str_reports_trailing_text_and_empty_input() {
    let (node, diags) = read("(a) b");
    assert_eq!(node.to_string(), "(a)");
    assert_eq!(codes(&diags), vec![DiagCode::Other]);
    let (node, diags) = read("  ; only a comment");
    assert_eq!(node.kind, NodeKind::Error);
    assert_eq!(diags.len(), 1);
    let (node, diags) = read(")");
    assert_eq!(node.kind, NodeKind::Error);
    assert_eq!(diags.len(), 1);
}

#[test]
fn deep_nesting_is_cut_off_with_a_diagnostic() {
    let depth = 100_000;
    let src = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
    let (node, diags) = read(&src);
    assert_eq!(codes(&diags), vec![DiagCode::Other]);
    assert_eq!(node.span.range(), 0..src.len());
    // Unclosed: one diagnostic per open list that was kept, none per skipped one.
    let src = "(".repeat(depth);
    let (_, diags) = read(&src);
    assert_eq!(diags.len(), MAX_NESTING + 1);
    let src = format!("{}x", "'".repeat(depth));
    let (node, _) = read(&src);
    assert!(node.is_form("quote"));
    let src = format!("{}x", "-".repeat(depth));
    let (node, diags) = read(&src);
    assert!(diags.is_empty());
    assert!(node.is_form("-"));
    // Exactly at the limit is fine.
    let src = format!("{}x{}", "(".repeat(MAX_NESTING), ")".repeat(MAX_NESTING));
    let (_, diags) = read(&src);
    assert!(diags.is_empty());
}

#[test]
fn segments_read_as_lines() {
    // `> ` prefixes are cut away by the markdown parser; the reader sees the
    // gap between segments as a newline.
    let text = "> (show (a\n> b))";
    let segs = [Seg::new(2, 10, 11), Seg::new(13, 16, 16)];
    let mut diags = Diags::new();
    let read = read_in(text, FileId(0), 2, &segs, &mut diags);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(read.node.to_string(), "(show (a b))");
    assert_eq!(read.end.to_usize(), text.len());
}

#[test]
fn display_round_trips() {
    for src in ["(show (frac 1 (+ a b)))", "(let x 2)", "(text \"a\\\"b\")", "(quote x)"] {
        assert_eq!(read_ok(src).to_string(), src);
    }
}
