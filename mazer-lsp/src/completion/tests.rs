// Snippets are written `${1:name}`, which is not a format string.
#![allow(clippy::literal_string_with_formatting_args)]

use lsp_types::{CompletionItem, CompletionTextEdit, InsertTextFormat};

use super::*;
use crate::testing::{marked, text_of};

const NONE: [&str; 0] = [];

fn items(src: &str) -> Vec<CompletionItem> {
    let (analysis, at) = marked(src);
    complete(&analysis, at, Config { snippets: true })
}

fn labels(src: &str) -> Vec<String> {
    items(src).into_iter().map(|i| i.label).collect()
}

fn find<'a>(items: &'a [CompletionItem], label: &str) -> &'a CompletionItem {
    items
        .iter()
        .find(|i| i.label == label)
        .unwrap_or_else(|| panic!("no {label:?} among {:?}", items.iter().map(|i| &i.label).collect::<Vec<_>>()))
}

fn new_text(item: &CompletionItem) -> &str {
    match item.text_edit.as_ref() {
        Some(CompletionTextEdit::Edit(edit)) => &edit.new_text,
        other => panic!("{other:?}"),
    }
}

/// The text the item's edit replaces.
fn replaced(src: &str, label: &str) -> String {
    let (analysis, at) = marked(src);
    let all = complete(&analysis, at, Config { snippets: true });
    let item = find(&all, label);
    match item.text_edit.as_ref() {
        Some(CompletionTextEdit::Edit(edit)) => text_of(&analysis, edit.range),
        other => panic!("{other:?}"),
    }
}

#[test]
fn head_offers_keywords_and_macros() {
    let src = "(macro (laplace f s) \"Laplace\" (jux L (brace f) (paren s)))\n\n(alias R (bb R))\n\n(show (|";
    let all = items(src);
    let integral = find(&all, "integral");
    assert_eq!(integral.kind, Some(CompletionItemKind::FUNCTION));
    assert!(integral.detail.as_deref().unwrap().contains("∫"), "the sign is in the detail");
    assert!(integral.detail.as_deref().unwrap().contains("(integral body var [lower] [upper])"));
    assert_eq!(new_text(integral), "integral ${1:body} ${2:var}");
    assert_eq!(integral.insert_text_format, Some(InsertTextFormat::SNIPPET));
    // Second names are items of their own.
    find(&all, "add");
    let laplace = find(&all, "laplace");
    assert_eq!(new_text(laplace), "laplace ${1:f} ${2:s}");
    find(&all, "R");
    // Reserved words are not offered inside a form.
    assert!(!all.iter().any(|i| i.label == "show" || i.label == "let"));
}

#[test]
fn head_mid_word_and_without_snippets() {
    assert_eq!(replaced("(show (int|", "integral"), "int");
    assert_eq!(replaced("(show (fr|ac a b))", "frac"), "fr");
    let (analysis, at) = marked("(show (fr|");
    let plain = complete(&analysis, at, Config { snippets: false });
    let frac = find(&plain, "frac");
    assert_eq!(new_text(frac), "frac");
    assert_eq!(frac.insert_text_format, None);
}

#[test]
fn head_of_a_top_level_form_offers_reserved_words() {
    let got = labels("(sh|ow x)");
    assert_eq!(got, ["show", "let", "alias", "macro", "ref", "eval"]);
    // `tag` as the first item of a show.
    assert!(labels("(show (|").contains(&"tag".to_owned()));
    assert!(!labels("(show x (|").contains(&"tag".to_owned()));
}

#[test]
fn arguments() {
    let src = "(let y 2)\n\n(alias R (bb R))\n\n(show (+ x |";
    let all = items(src);
    let tau = find(&all, "tau");
    assert_eq!(tau.detail.as_deref(), Some("τ"));
    assert!(tau.filter_text.as_deref().unwrap().contains("Greek"), "the doc text filters too");
    let y = find(&all, "y");
    assert_eq!(y.detail.as_deref(), Some("2"));
    assert_eq!(y.kind, Some(CompletionItemKind::VARIABLE));
    find(&all, "R");
    // Keywords too, ranked below: a sign keyword bare, a sign-less one as a call.
    let integral = find(&all, "integral");
    assert_eq!(new_text(integral), "integral");
    assert!(integral.sort_text > tau.sort_text);
    assert!(tau.sort_text > y.sort_text);
    assert_eq!(new_text(find(&all, "frac")), "(frac ${1:num} ${2:den})");
}

#[test]
fn unclosed_and_nested() {
    // `(show (int|` mid-edit, unclosed.
    assert!(labels("(show (int|").contains(&"integral".to_owned()));
    // An argument of a nested unclosed call.
    assert!(labels("(show (frac (sqrt al|").contains(&"alpha".to_owned()));
    assert_eq!(replaced("(show (frac (sqrt al|", "alpha"), "al");
    // On a name (Cursor::Name), mid-word.
    assert_eq!(replaced("(show (+ al|pha b))", "alpha"), "al");
    // After a quote mark or a minus, only the name is replaced.
    assert_eq!(replaced("(show (+ 'al|", "alpha"), "al");
    assert_eq!(replaced("(show (+ -al|", "alpha"), "al");
}

#[test]
fn lets_in_scope_only() {
    let src = "(let a 1)\n\n(show (+ |))\n\n(let b 2)";
    let got = labels(src);
    assert!(got.contains(&"a".to_owned()));
    assert!(!got.contains(&"b".to_owned()), "b is bound later");
    // A re-bound name is offered once, with its latest value.
    let src = "(let a 1)\n\n(let a 5)\n\n(show (+ |))";
    let all = items(src);
    assert_eq!(all.iter().filter(|i| i.label == "a").count(), 1);
    assert_eq!(find(&all, "a").detail.as_deref(), Some("5"));
    // Not inside a definition, where variables are not substituted.
    assert!(!labels("(let a 1)\n\n(alias B (+ |))").contains(&"a".to_owned()));
}

#[test]
fn macro_parameters_in_the_body() {
    let got = labels("(macro (sq val) (^ va|");
    assert!(got.contains(&"val".to_owned()));
}

#[test]
fn new_names_get_nothing() {
    assert_eq!(labels("(let |"), NONE);
    assert_eq!(labels("(alias |"), NONE);
    assert_eq!(labels("(macro (|"), NONE);
    assert_eq!(labels("(macro (sq |"), NONE);
    assert_eq!(labels("(show (tag |"), NONE);
}

#[test]
fn labels_in_ref() {
    let src = "(show (tag euler) (= x 1))\n\n(show (tag gauss) y)\n\nSee (ref |";
    let all = items(src);
    assert_eq!(all.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), ["euler", "gauss"]);
    assert_eq!(find(&all, "gauss").detail.as_deref(), Some("(2)"));
    assert_eq!(replaced("(show (tag euler) x)\n\nSee (ref eu|).", "euler"), "eu");
}

#[test]
fn prose() {
    let all = items("So sad :so|");
    let sob = find(&all, ":sob:");
    assert_eq!(sob.detail.as_deref(), Some("😭"));
    assert_eq!(replaced("So sad :so|", ":sob:"), ":so");
    assert_eq!(items("Right after a colon: |").len(), 0);
    assert_eq!(items("Note:|").len(), 0, "a colon right after a word is not a shortcode");
    assert_eq!(items("τ😭 :|").len(), 13);
    // `(` then a word: the reserved words.
    assert_eq!(replaced("Text (sh|", "show"), "sh");
    assert_eq!(new_text(find(&items("Text (|"), "let")), "let ${1:name} ${2:value}");
    assert_eq!(labels("Text \\(sh|"), NONE, "an escaped paren is prose");
    assert_eq!(labels("plain wor|ds"), NONE);
}

#[test]
fn nothing_in_comments_strings_or_code_spans() {
    assert_eq!(labels("(show (+ a ; :so|\n b))"), NONE);
    assert_eq!(labels("(show (+ a ; (sh|\n b))"), NONE);
    assert_eq!(labels("(show \"a :so|\")"), NONE);
    assert_eq!(labels("```\n:so|\n```"), NONE);
    assert_eq!(labels("`(sh|`"), NONE);
}

#[test]
fn every_offset_is_safe() {
    let src = "τ😭 (show (+ α β 'γ -δ (frac a\n\n(let x 2) :sob: (ref e) (show (tag e) x) ; c\n\"s";
    let analysis = mazer_doc::analyze(src, "t.zr");
    for at in (0..=src.len() + 2).filter(|&i| i > src.len() || src.is_char_boundary(i)) {
        let _ = complete(&analysis, ByteOffset::from_usize(at), Config { snippets: true });
    }
}
