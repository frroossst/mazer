use mazer_syntax::DiagCode;

use super::*;
use crate::testing::{codes, parse};

fn labels(src: &str) -> (Labels, Vec<DiagCode>, Diags) {
    let (doc, mut diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    let labels = collect_labels(&doc, &mut diags);
    let found = codes(diags.as_slice());
    (labels, found, diags)
}

#[test]
fn numbered_in_document_order() {
    let src = "(show (tag b) x)\n\n(show y)\n\n- not here\n\n(show (tag a) z)\n\n(show (tag c) w)";
    let (labels, found, _) = labels(src);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(labels.len(), 3);
    let b = labels.get(Sym::new("b")).unwrap();
    assert_eq!((b.number, &*b.anchor), (1, "eq-b"));
    assert_eq!(labels.get(Sym::new("a")).unwrap().number, 2);
    assert_eq!(labels.get(Sym::new("c")).unwrap().number, 3);
    assert!(labels.get(Sym::new("y")).is_none());
}

#[test]
fn label_span_is_the_tag() {
    let src = "(show (tag lap) x)";
    let (labels, _, _) = labels(src);
    let span = labels.get(Sym::new("lap")).unwrap().span;
    assert_eq!(&src[span.range()], "(tag lap)");
}

#[test]
fn forward_references_resolve() {
    let (_, found, _) = labels("See (ref later).\n\n(show (tag later) x)");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn duplicate_tags() {
    let (labels, found, diags) = labels("(show (tag a) x)\n\n(show (tag a) y)");
    assert_eq!(found, [DiagCode::DuplicateTag]);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels.get(Sym::new("a")).unwrap().number, 1);
    let first = labels.get(Sym::new("a")).unwrap().span;
    assert_eq!(diags.as_slice()[0].labels[0].0, first);
}

#[test]
fn misplaced_tags() {
    for src in [
        "Inline (show (tag a) x) here.",
        "- (show (tag a) x)",
        "(show x (tag a))",
        "(show (+ (tag a) x))",
        "(let y (tag a))",
        "(eval (tag a))",
        "(show (tag a) (tag b) x)",
    ] {
        let (labels, found, _) = labels(src);
        assert_eq!(found, [DiagCode::MisplacedTag], "{src}");
        assert!(labels.get(Sym::new("b")).is_none());
        let _ = labels;
    }
    // The one that is in place still counts.
    let (labels, _, _) = labels("(show (tag a) (tag b) x)");
    assert_eq!(labels.len(), 1);
}

#[test]
fn malformed_tags() {
    for src in ["(show (tag) x)", "(show (tag 1) x)", "(show (tag a b) x)"] {
        let (labels, found, _) = labels(src);
        assert_eq!(found, [DiagCode::BadDefinition], "{src}");
        assert!(labels.is_empty());
    }
}

#[test]
fn unresolved_references() {
    let (_, found, diags) = labels("See (ref nope).");
    assert_eq!(found, [DiagCode::UnresolvedRef]);
    assert!(diags.as_slice()[0].message.contains("nope"));
    let (_, found, _) = labels("See (ref 1).");
    assert_eq!(found, [DiagCode::UnresolvedRef]);
    let (_, found, _) = labels("See (ref a b).");
    assert_eq!(found, [DiagCode::Arity]);
    let (_, found, _) = labels("See (ref ).");
    assert_eq!(found, [DiagCode::Arity]);
}

#[test]
fn a_misspelt_reference_gets_a_suggestion() {
    let help = |src: &str| {
        let (_, found, diags) = labels(src);
        assert_eq!(found, [DiagCode::UnresolvedRef], "{src}");
        diags.as_slice()[0].help.clone().unwrap()
    };
    let tags = "(show (tag euler) x)\n\n(show (tag gauss) y)\n\n";
    assert_eq!(help(&format!("{tags}See (ref eular).")), "did you mean `euler`? (equation (1))");
    assert_eq!(help(&format!("{tags}See (ref guass).")), "did you mean `gauss`? (equation (2))", "a swap is one edit");
    assert_eq!(help(&format!("{tags}See (ref eu).")), "tag one with (show (tag eu) ...)", "too far");
    assert_eq!(help(&format!("{tags}See (ref fermat).")), "tag one with (show (tag fermat) ...)");
    // Short names allow one edit only.
    assert_eq!(help("(show (tag ab) x)\n\nSee (ref ba)."), "did you mean `ab`? (equation (1))");
    assert_eq!(help("(show (tag abc) x)\n\nSee (ref xyc)."), "tag one with (show (tag xyc) ...)");
    assert_eq!(edit_distance("τau", "tau"), 1, "characters, not bytes");
    assert_eq!(edit_distance("", "abc"), 3);
}
