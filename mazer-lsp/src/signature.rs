//! `textDocument/signatureHelp`: the signature of the innermost enclosing
//! keyword, macro or reserved-word call, with the parameter the cursor's
//! argument binds to highlighted.
//!
//! The argument is mapped to a parameter by the rule of `mazer_math::bind`
//! for the number of arguments written so far (required parameters first,
//! spare arguments fill optional ones left to right, the rest parameter takes
//! the remainder and stays active). Lists that are neither (function
//! application, `(f x)`) are skipped in favour of the call around them.

use lsp_types::{
    Documentation, MarkupContent, MarkupKind, ParameterInformation, ParameterLabel, SignatureHelp,
    SignatureInformation,
};
use mazer_doc::math::registry;
use mazer_doc::syntax::{Node, RESERVED_HEADS};
use mazer_doc::{Analysis, ByteOffset, Cursor, locate};

use crate::docs::{self, Signature};
use crate::tree;

/// The signature help at `at`, if the cursor is inside a call that has one.
#[must_use]
pub fn signature_help(analysis: &Analysis, at: ByteOffset) -> Option<SignatureHelp> {
    if matches!(locate(analysis, at), Cursor::Prose) {
        return None;
    }
    let chain = tree::enclosing_lists(analysis, at);
    for list in chain.iter().rev() {
        let Some((sig, show)) = signature_of(analysis, list) else {
            continue;
        };
        let count = tree::real_items(list).len().saturating_sub(1);
        let active = tree::arg_index(list, at).and_then(|index| {
            if show {
                // `(show [(tag name)] expr...)`: the tag only if one is written first.
                let tagged = list.args().first().is_some_and(|a| a.is_form("tag"));
                Some(usize::from(!(tagged && index == 0)))
            } else {
                sig.active(count, index)
            }
        });
        return Some(help(&sig, active));
    }
    None
}

/// The signature of a list's head, and whether it is `show`.
fn signature_of(analysis: &Analysis, list: &Node) -> Option<(Signature, bool)> {
    let head = tree::real_items(list).first()?;
    let sym = head.as_ident()?;
    let name = sym.as_str();
    if RESERVED_HEADS.contains(&name) || name == "tag" {
        return docs::reserved_signature(name).map(|s| (s, name == "show"));
    }
    if let Some(keyword) = registry().get(sym) {
        return Some((docs::keyword_signature(keyword, name), false));
    }
    let def = analysis.macros.get(sym).filter(|m| !m.is_alias())?;
    Some((docs::macro_signature(def), false))
}

fn help(sig: &Signature, active: Option<usize>) -> SignatureHelp {
    let (label, offsets) = sig.label();
    let parameters = sig
        .params
        .iter()
        .zip(offsets)
        .map(|(p, range)| ParameterInformation {
            label: ParameterLabel::LabelOffsets(range),
            documentation: (!p.doc.is_empty()).then(|| Documentation::String(p.doc.clone())),
        })
        .collect();
    let active = active.and_then(|a| u32::try_from(a).ok());
    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: (!sig.doc.is_empty()).then(|| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: sig.doc.clone(),
                })
            }),
            parameters: Some(parameters),
            active_parameter: active,
        }],
        active_signature: Some(0),
        active_parameter: active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::marked;

    /// The signature label and the text of the active parameter.
    fn at(src: &str) -> Option<(String, Option<String>)> {
        let (analysis, at) = marked(src);
        let help = signature_help(&analysis, at)?;
        let sig = &help.signatures[0];
        let active = help.active_parameter.map(|i| {
            let ParameterLabel::LabelOffsets([a, b]) = sig.parameters.as_ref().unwrap()[i as usize].label else {
                panic!("offsets");
            };
            let units: Vec<u16> = sig.label.encode_utf16().collect();
            String::from_utf16(&units[a as usize..b as usize]).unwrap()
        });
        Some((sig.label.clone(), active))
    }

    fn active(src: &str) -> Option<String> {
        at(src).and_then(|(_, a)| a)
    }

    #[test]
    fn required_parameters() {
        assert_eq!(active("(show (frac |"), Some("num".into()));
        assert_eq!(active("(show (frac a |"), Some("den".into()));
        assert_eq!(active("(show (frac a| b))"), Some("num".into()));
        assert_eq!(active("(show (frac a b|))"), Some("den".into()));
        // One too many: nothing active.
        assert_eq!(active("(show (frac a b |))"), None);
    }

    #[test]
    fn integral_optional_bounds() {
        assert_eq!(active("(show (integral |"), Some("body".into()));
        assert_eq!(active("(show (integral f |"), Some("var".into()));
        assert_eq!(active("(show (integral f x |"), Some("[lower]".into()));
        assert_eq!(active("(show (integral f x a |"), Some("[upper]".into()));
        // Editing the body of a call that already has bounds.
        assert_eq!(active("(show (integral f| x a b))"), Some("body".into()));
    }

    #[test]
    fn rest_parameter_stays_active() {
        let (label, a) = at("(show (+ a b c d |").unwrap();
        assert!(label.starts_with("(+ "), "{label}");
        assert!(a.unwrap().ends_with("..."));
    }

    #[test]
    fn innermost_call_wins() {
        let (label, _) = at("(show (frac (sqrt |) b))").unwrap();
        assert!(label.starts_with("(sqrt"), "{label}");
        // Function application is skipped for the call around it.
        let (label, a) = at("(show (frac (f |) b))").unwrap();
        assert_eq!(label, "(frac num den)");
        assert_eq!(a, Some("num".into()));
        // In the head of a nested list not yet typed: the enclosing call.
        let (label, a) = at("(show (frac a (|").unwrap();
        assert_eq!(label, "(frac num den)");
        assert_eq!(a, Some("den".into()));
    }

    #[test]
    fn reserved_words() {
        assert_eq!(active("(show |"), Some("expr...".into()));
        assert_eq!(active("(show (tag a) |x)"), Some("expr...".into()));
        assert_eq!(active("(show (tag a|) x)"), Some("name".into()), "inside the tag: tag's own signature");
        assert_eq!(active("(show (tag a)| x)"), Some("[(tag name)]".into()));
        assert_eq!(active("(let x |"), Some("value".into()));
        assert_eq!(active("(let |"), Some("name".into()));
        assert_eq!(active("(macro (f x) |"), Some("body".into()));
        assert_eq!(active("(macro (f x) \"doc\" |"), Some("body".into()));
        assert_eq!(active("(alias R |"), Some("expansion".into()));
        assert_eq!(active("See (ref |)."), Some("name".into()));
    }

    #[test]
    fn macros() {
        let src = "(macro (laplace f s) (jux L (brace f) (paren s)))\n\n(show (laplace g |";
        let (label, a) = at(src).unwrap();
        assert_eq!(label, "(laplace f s)");
        assert_eq!(a, Some("s".into()));
        let src = "(macro (pair a rest...) (paren a rest))\n\n(show (pair 1 2 3 |";
        assert_eq!(active(src), Some("rest...".into()));
    }

    #[test]
    fn nothing_outside_code() {
        assert!(at("hello |world").is_none());
        assert!(at("(show (frac a ; no|te\n b))").is_none());
    }
}
