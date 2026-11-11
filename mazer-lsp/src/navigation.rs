//! `textDocument/definition` and `textDocument/references`.
//!
//! - A macro or alias use goes to the name in its definition. Definitions in
//!   the embedded prelude have no file the editor could open: nothing.
//! - A `let` variable goes to the name in the `let` in scope there (the last
//!   one written before the use), so a re-bound name goes to the right one.
//! - `(ref name)` goes to the `(tag name)` of the equation.
//! - A macro parameter in the body goes to the parameter in the signature.
//!
//! References are found within the note: every use that resolves to the same
//! definition (the same `let` binding, the same macro, the same label name).

use lsp_types::{Location, Uri};
use mazer_doc::{Analysis, ByteOffset, Span, Sym};

use crate::convert::{self, NOTE};
use crate::tree::{self, Found, Role};

/// What a name refers to, for matching uses against each other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    Macro(Span),
    Let(Span),
    Label(Sym),
    Param(Span, Sym),
}

fn target(analysis: &Analysis, found: &Found<'_>) -> Option<Target> {
    let sym = found.node.as_ident()?;
    Some(match tree::classify(analysis, found.node, &found.ctx) {
        Role::MacroUse(def) | Role::MacroName(Some(def)) => Target::Macro(def.span),
        Role::LetUse(binding) | Role::LetName(Some(binding)) => Target::Let(binding.span),
        Role::RefName(name, _) | Role::TagName(name, _) => Target::Label(name),
        Role::MacroParam => Target::Param(found.ctx.macro_form?.span, sym),
        _ => return None,
    })
}

/// Whether `found` is the defining occurrence of its target.
fn is_declaration(analysis: &Analysis, found: &Found<'_>) -> bool {
    match tree::classify(analysis, found.node, &found.ctx) {
        Role::MacroName(_) | Role::LetName(_) | Role::TagName(..) => true,
        Role::MacroParam => found.ctx.macro_form.and_then(|f| f.args().first()).zip(found.ctx.parent).is_some_and(|(sig, parent)| sig.span == parent.span),
        _ => false,
    }
}

/// Where the name at `at` is defined.
#[must_use]
pub fn definition(analysis: &Analysis, uri: &Uri, at: ByteOffset) -> Option<Location> {
    let found = tree::ident_at(analysis, at)?;
    let span = match tree::classify(analysis, found.node, &found.ctx) {
        Role::MacroUse(def) | Role::MacroName(Some(def)) => name_in(analysis, def.span)?,
        Role::LetUse(binding) | Role::LetName(Some(binding)) => name_in(analysis, binding.span)?,
        Role::RefName(_, Some(label)) | Role::TagName(_, Some(label)) => label.span,
        Role::MacroParam => {
            let sym = found.node.as_ident()?;
            let sig = found.ctx.macro_form?.args().first()?;
            sig.args()
                .iter()
                .find(|p| p.as_ident().is_some_and(|p| p == sym || p.as_str().strip_suffix("...") == Some(sym.as_str())))?
                .span
        }
        _ => return None,
    };
    (span.file == NOTE).then(|| Location::new(uri.clone(), convert::range(&analysis.sources, span)))
}

/// The span of the defined name in the definition form at `form_span`, or
/// the form itself.
fn name_in(analysis: &Analysis, form_span: Span) -> Option<Span> {
    if form_span.file != NOTE {
        return None;
    }
    Some(mazer_doc::definition_name_span(analysis, form_span).unwrap_or(form_span))
}

/// Every occurrence in the note of what the name at `at` refers to, in
/// source order; with `include_declaration`, the definition too.
#[must_use]
pub fn references(analysis: &Analysis, uri: &Uri, at: ByteOffset, include_declaration: bool) -> Vec<Location> {
    let Some(found) = tree::ident_at(analysis, at) else {
        return Vec::new();
    };
    let Some(wanted) = target(analysis, &found) else {
        return Vec::new();
    };
    tree::idents(analysis)
        .into_iter()
        .filter(|f| target(analysis, f) == Some(wanted))
        .filter(|f| include_declaration || !is_declaration(analysis, f))
        .map(|f| Location::new(uri.clone(), convert::range(&analysis.sources, f.node.span)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{marked, text_of, uri};

    /// The line and text of the definition of the name at the cursor.
    fn def(src: &str) -> Option<(u32, String)> {
        let (analysis, at) = marked(src);
        definition(&analysis, &uri(), at).map(|loc| (loc.range.start.line, text_of(&analysis, loc.range)))
    }

    fn refs(src: &str, decl: bool) -> Vec<(u32, String)> {
        let (analysis, at) = marked(src);
        references(&analysis, &uri(), at, decl)
            .into_iter()
            .map(|loc| (loc.range.start.line, text_of(&analysis, loc.range)))
            .collect()
    }

    #[test]
    fn macros_and_aliases() {
        let src = "(macro (sq v) (^ v 2))\n\n(alias R (bb R))\n\n(show (sq| R))";
        assert_eq!(def(src), Some((0, "sq".into())));
        let src = "(macro (sq v) (^ v 2))\n\n(alias R (bb R))\n\n(show (sq |R))";
        assert_eq!(def(src), Some((2, "R".into())));
        // A parameter in the body goes to the signature.
        assert_eq!(def("(macro (sq v) (^ |v 2))"), Some((0, "v".into())));
        let src = "(alias R (bb R))\n\n(show (+ |R R))\n\n(show (R x))";
        assert_eq!(refs(src, true), vec![(0, "R".into()), (2, "R".into()), (2, "R".into()), (4, "R".into())]);
        assert_eq!(refs(src, false).len(), 3);
    }

    #[test]
    fn lets_in_scope() {
        let src = "(let x 2)\n\n(show x)\n\n(let x 3)\n\n(show (+ |x 1))";
        assert_eq!(def(src), Some((4, "x".into())), "the re-bound x");
        let src = "(let x 2)\n\n(show |x)\n\n(let x 3)\n\n(show x)";
        assert_eq!(def(src), Some((0, "x".into())));
        // In its own value: the earlier one.
        let src = "(let x 2)\n\n(let x (+ |x 1))";
        assert_eq!(def(src), Some((0, "x".into())));
        // References of the first binding only.
        let src = "(let |x 2)\n\n(show x)\n\n(let x (+ x 1))\n\n(show x)\n\n(show 'x)";
        assert_eq!(refs(src, true), vec![(0, "x".into()), (2, "x".into()), (4, "x".into())]);
        assert_eq!(refs(src, false), vec![(2, "x".into()), (4, "x".into())]);
    }

    #[test]
    fn labels() {
        let src = "(show (tag a) x)\n\nSee (ref |a) and (ref a).";
        assert_eq!(def(src), Some((0, "(tag a)".into())));
        let src = "(show (tag |a) x)\n\nSee (ref a) and (ref a).";
        assert_eq!(refs(src, true).len(), 3);
        assert_eq!(refs(src, false).len(), 2);
        assert_eq!(def("See (ref |nope)."), None);
    }

    #[test]
    fn nothing_for_keywords_or_prose() {
        assert_eq!(def("(show (fr|ac a b))"), None);
        assert_eq!(def("pro|se"), None);
        assert_eq!(refs("(show (fr|ac a b))", true), []);
    }
}
