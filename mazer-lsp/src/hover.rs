//! `textDocument/hover`: what the name under the cursor is.
//!
//! Keywords (head or bare, by any name) show their signature, doc, sign,
//! parameters and examples; display names their character; macros and
//! aliases their signature, doc string, expansion and where they are
//! defined; `let` variables the value in scope and where it was bound;
//! `(ref name)` and `(tag name)` the equation number and its source; reserved
//! words what they do.

use std::fmt::Write as _;

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use mazer_doc::math::registry;
use mazer_doc::{Analysis, ByteOffset, Macro};

use crate::convert::{self, NOTE};
use crate::docs::{self, FENCE};
use crate::tree::{self, Role};

/// The hover at `at`, if the cursor is on a name that has something to say.
#[must_use]
pub fn hover(analysis: &Analysis, at: ByteOffset) -> Option<Hover> {
    let found = tree::ident_at(analysis, at)?;
    let sym = found.node.as_ident()?;
    let name = sym.as_str();
    let sources = &analysis.sources;
    let line_of = |span: mazer_doc::Span| sources.line_col(span.file, span.start).0.0 + 1;
    let value = match tree::classify(analysis, found.node, &found.ctx) {
        Role::Mark | Role::Keyword(_) => {
            let keyword = registry().get(sym)?;
            docs::keyword_markdown(keyword, name)
        }
        Role::Reserved(word) => docs::reserved_markdown(word),
        Role::MacroUse(def) | Role::MacroName(Some(def)) => docs::macro_markdown(def, &place(analysis, def)),
        Role::MacroName(None) => format!("`{name}` is not defined: this definition has an error (see its diagnostic)."),
        Role::MacroParam => {
            let owner = found.ctx.macro_form.and_then(tree::definition_name).map_or("", |n| sources.text(n.span));
            format!("Parameter `{name}` of the macro `{owner}`: replaced by the argument as written.")
        }
        Role::LetName(Some(binding)) => format!(
            "`let` variable `{name}`, from here on:\n```{FENCE}\n{}\n```",
            binding.value
        ),
        Role::LetName(None) => format!("`{name}` is not bound: this `let` has an error (see its diagnostic)."),
        Role::LetUse(binding) => {
            let mut out = format!(
                "`let` variable `{name}`, shown as:\n```{FENCE}\n{}\n```\nBound on line {}:\n```{FENCE}\n{}\n```",
                binding.value,
                line_of(binding.span),
                sources.text(binding.span),
            );
            if let Some(display) = mazer_doc::math::display_name(sym) {
                let _ = write!(out, "\nWrite `'{name}` to show the name itself ({}).", display.unicode);
            } else {
                let _ = write!(out, "\nWrite `'{name}` to show the name itself.");
            }
            out
        }
        Role::TagName(_, Some(label)) => format!(
            "Equation ({}). Link to it with `(ref {name})`; its anchor is `#{}`.",
            label.number, label.anchor
        ),
        Role::TagName(_, None) => format!("The tag `{name}` is not registered (see its diagnostic)."),
        Role::RefName(_, Some(label)) => {
            let mut out = format!("Equation ({}), tagged on line {}", label.number, line_of(label.span));
            if let Some(form) = tree::tagged_equation(analysis, label) {
                let _ = write!(out, ":\n```{FENCE}\n{}\n```", sources.text(form.span));
            }
            out
        }
        Role::RefName(_, None) => format!("No equation is tagged `{name}`. Tag one with `(show (tag {name}) ...)`."),
        Role::Display(display) => docs::display_markdown(display),
        Role::Function => format!(
            "`{name}` is not a keyword, so `({name} a b)` is function application: it shows {name}(a, b)."
        ),
        Role::Free => {
            let later = analysis.lets.iter().find(|b| b.name == sym && b.span.file == NOTE);
            match later {
                Some(b) if !found.ctx.quoted => format!(
                    "`{name}` is bound by a `let` on line {}, which only takes effect after it is written; here it is shown as written.",
                    line_of(b.span)
                ),
                _ => return None,
            }
        }
    };
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(convert::range(sources, found.node.span)),
    })
}

/// "on line N" for a definition in the note, or the prelude.
fn place(analysis: &Analysis, def: &Macro) -> String {
    if def.span.file == NOTE {
        let (line, _) = analysis.sources.line_col(def.span.file, def.span.start);
        format!("on line {}", line.0 + 1)
    } else {
        "in the built-in prelude (stdlib.zr)".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::marked;

    fn text(src: &str) -> Option<String> {
        let (analysis, at) = marked(src);
        hover(&analysis, at).map(|h| match h.contents {
            HoverContents::Markup(m) => m.value,
            other => panic!("{other:?}"),
        })
    }

    fn has(src: &str, want: &[&str]) {
        let got = text(src).unwrap_or_else(|| panic!("no hover for {src:?}"));
        for w in want {
            assert!(got.contains(w), "hover for {src:?} lacks {w:?}:\n{got}");
        }
    }

    #[test]
    fn keywords() {
        has("(show (inte|gral (sin x) x))", &["(integral body var [lower] [upper])", "∫", "**Parameters**", "**Examples**"]);
        // Bare, and right after the word.
        has("(show (+ integral| x))", &["(integral body var"]);
        // A second name.
        has("(show (ad|d 1 2))", &["`add` is another name for `+`"]);
    }

    #[test]
    fn display_names() {
        has("(show (+ ta|u 1))", &["# τ", "`tau` shows as **τ**"]);
    }

    #[test]
    fn macros_and_aliases() {
        let src = "(macro (laplace f s) \"Laplace transform\" (jux L (brace f) (paren s)))\n\n(alias R (bb R))\n\n(show (lap|lace g R))";
        has(src, &["(laplace f s)", "Laplace transform", "on line 1", "(jux L (brace f) (paren s))"]);
        let src = "(alias R (bb R))\n\n(show (+ |R 1))";
        has(src, &["Notation (alias)", "(bb R)"]);
        // The parameter in the body.
        has("(macro (sq v) (^ |v 2))", &["Parameter `v` of the macro `sq`"]);
        // The name being defined.
        has("(alias |R (bb R))", &["(bb R)"]);
    }

    #[test]
    fn lets() {
        let src = "(let x 2)\n\n(let x 3)\n\n(show (+ |x 1))";
        has(src, &["shown as:\n```scheme\n3\n```", "Bound on line 3"]);
        let src = "(let x 2)\n\n(show |x)\n\n(let x 3)";
        has(src, &["2", "Bound on line 1"]);
        // Inside its own value, the earlier binding.
        let src = "(let x 2)\n\n(let x (+ |x 1))";
        has(src, &["shown as:\n```scheme\n2\n```"]);
        // Before any let: nothing bound yet.
        has("(show |y)\n\n(let y 1)", &["only takes effect after it is written"]);
        // Quoted: no substitution.
        assert!(text("(let x 2)\n\n(show '|x)").is_none());
    }

    #[test]
    fn refs_and_tags() {
        let src = "(show (tag euler) (= (^ e (* i pi)) -1))\n\nSee (ref eu|ler).";
        has(src, &["Equation (1)", "(show (tag euler) (= (^ e (* i pi)) -1))"]);
        has("(show (tag eu|ler) x)", &["Equation (1)", "(ref euler)"]);
        has("See (ref nope|).", &["No equation is tagged `nope`"]);
    }

    #[test]
    fn reserved_words() {
        has("(sh|ow x)", &["(show [(tag name)] expr...)", "display equation"]);
        has("(l|et x 2)", &["(let name value)"]);
        has("See (r|ef a).", &["(ref name)"]);
        has("(show (t|ag a) x)", &["(tag name)"]);
    }

    #[test]
    fn nothing_in_prose_or_on_numbers() {
        assert!(text("hel|lo (show x)").is_none());
        assert!(text("(show (+ 1|2 x))").is_none());
        assert!(text("(show x ; com|ment\n)").is_none());
    }

    #[test]
    fn multibyte_range() {
        let (analysis, at) = marked("τ😭 (show (+ al|pha γ))");
        let h = hover(&analysis, at).expect("a hover");
        let range = h.range.expect("a range");
        assert_eq!(crate::testing::text_of(&analysis, range), "alpha");
        // τ 1 unit, 😭 2, space 1, "(show (+ " 9.
        assert_eq!(range.start.character, 13);
    }
}
