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
    // The whole word is replaced, and an existing call gets no tab stops.
    assert_eq!(replaced("(show (fr|ac a b))", "frac"), "frac");
    assert_eq!(new_text(find(&items("(show (fr|ac a b))"), "frac")), "frac");
    assert_eq!(new_text(find(&items("(show (fr|)"), "frac")), "frac ${1:num} ${2:den}");
    assert_eq!(new_text(find(&items("(sh|ow x)"), "show")), "show");
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
    assert_eq!(replaced("(show (+ al|pha b))", "alpha"), "alpha");
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
    // A second definition binds nothing: the name is offered once, with its
    // first value.
    let src = "(let a 1)\n\n(let a 5)\n\n(show (+ |))";
    let all = items(src);
    assert_eq!(all.iter().filter(|i| i.label == "a").count(), 1);
    assert_eq!(find(&all, "a").detail.as_deref(), Some("1"));
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
    assert_eq!(replaced("Text (sh|ow x)", "show"), "show");
    assert_eq!(new_text(find(&items("Text (sh|ow x)"), "show")), "show");
    assert_eq!(new_text(find(&items("Text (sh|)"), "show")), "show ${1:expr}");
    assert_eq!(replaced("So :so|b: sad", ":sob:"), ":sob:");
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

#[test]
fn table_directives() {
    let src = "(show (table (|";
    let got = labels(src);
    assert_eq!(&got[..4], ["header", "row", "align", "style"], "directives first");
    assert!(got.contains(&"frac".to_owned()), "a bare row may start with anything");
    assert_eq!(new_text(find(&items("(show (table (ro|"), "row")), "row");
    // Values of the directives.
    assert_eq!(labels("(show (table (align |"), ["left", "center", "right"]);
    assert_eq!(labels("(show (table (align left ce|"), ["left", "center", "right"]);
    assert_eq!(replaced("(show (table (align left ce|", "center"), "ce");
    assert_eq!(labels("(show (table (style |"), ["grid", "clean", "plain"]);
    assert_eq!(labels("(show (table (style grid |"), NONE, "style takes one value");
    // Only inside a table: elsewhere `align` is just a function name.
    assert!(!labels("(show (align |").contains(&"left".to_owned()));
    assert!(!labels("(show (frac (|").contains(&"header".to_owned()));
    // Cells of a row are ordinary arguments.
    assert!(labels("(show (table (row al|").contains(&"alpha".to_owned()));
}

#[test]
fn list_items_are_light_and_resolve_to_full_docs() {
    let src = "(macro (sq v) \"Square of v.\" (^ v 2))\n\n(let y 2)\n\n(show (tag e) x)\n\n(show (+ x |";
    let (analysis, at) = marked(src);
    let all = complete(&analysis, at, Config { snippets: true });
    for item in &all {
        assert!(
            !matches!(item.documentation, Some(Documentation::MarkupContent(_))),
            "{} carries full docs in the list",
            item.label
        );
        if let Some(Documentation::String(doc)) = &item.documentation {
            assert!(!doc.contains('\n') && doc.chars().count() <= 101, "{}: {doc:?}", item.label);
        }
    }
    let full = |label: &str, analysis: Option<&Analysis>| {
        let item = find(&all, label).clone();
        assert!(item.data.is_some(), "{label} has a data payload");
        match resolve(analysis, item).documentation {
            Some(Documentation::MarkupContent(m)) => m.value,
            other => panic!("{label}: {other:?}"),
        }
    };
    let integral = full("integral", None);
    assert!(integral.contains("**Parameters**") && integral.contains("**Examples**"), "{integral}");
    assert!(full("tau", None).contains("`tau` shows as **τ**"));
    assert!(full("y", Some(&analysis)).contains("bound on line 3"));
    let one_line = |label: &str| match &find(&all, label).documentation {
        Some(Documentation::String(s)) => s.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(one_line("tau"), mazer_doc::math::display_name(mazer_doc::Sym::new("tau")).unwrap().doc);
    assert!(one_line("integral").len() > 10);
    // A macro, at a head.
    let (analysis, at) = marked("(macro (sq v) \"Square of v.\" (^ v 2))\n\n(show (s|");
    let heads = complete(&analysis, at, Config { snippets: true });
    let sq = find(&heads, "sq").clone();
    assert_eq!(sq.documentation, Some(Documentation::String("Square of v.".to_owned())));
    let Some(Documentation::MarkupContent(m)) = resolve(Some(&analysis), sq).documentation else {
        panic!("no docs for sq");
    };
    assert!(m.value.contains("(^ v 2)") && m.value.contains("on line 1"), "{}", m.value);
    // A label inside (ref ...).
    let (analysis, at) = marked("(show (tag euler) (= x 1))\n\nSee (ref |");
    let refs = complete(&analysis, at, Config { snippets: true });
    let Some(Documentation::MarkupContent(m)) = resolve(Some(&analysis), refs[0].clone()).documentation else {
        panic!("no docs for the label");
    };
    assert!(m.value.contains("(show (tag euler) (= x 1))"), "{}", m.value);
    // Unknown payloads are returned unchanged.
    let plain = CompletionItem::default();
    assert_eq!(resolve(None, plain.clone()), plain);
}

#[test]
fn responses_share_the_edit_range() {
    let src = "(let y 2)\n\n(show (+ x al|";
    let all = items(src);
    let plain = response(all.clone(), "file:///n.zr", false);
    let list = plain.as_array().expect("a plain array");
    assert!(list.iter().all(|i| i.get("textEdit").is_some()));
    let shared = response(all, "file:///n.zr", true);
    let range = &shared["itemDefaults"]["editRange"];
    assert_eq!(range["start"]["character"], 11);
    assert_eq!(range["end"]["character"], 13);
    let items = shared["items"].as_array().unwrap();
    assert!(items.iter().all(|i| i.get("textEdit").is_none()));
    let frac = items.iter().find(|i| i["label"] == "frac").unwrap();
    assert_eq!(frac["textEditText"], "(frac ${1:num} ${2:den})");
    let alpha = items.iter().find(|i| i["label"] == "alpha").unwrap();
    assert!(alpha.get("textEditText").is_none(), "the label is the text");
    // Only what the note defines needs the document to resolve.
    let y = items.iter().find(|i| i["label"] == "y").unwrap();
    assert_eq!(y["data"]["uri"], "file:///n.zr");
    assert!(alpha["data"].get("uri").is_none());
    // The whole response stays small: no markdown documentation in it.
    assert!(!shared.to_string().contains("**Parameters**"));
}
