//! The built-in keywords, split into three files so they can be written in
//! parallel. Each file exposes `keywords() -> Vec<Keyword>` and
//! `precedence(name, args) -> Option<Prec>`.
//!
//! The inventory (names, second names, argument order, what was removed and
//! what is new) is in `docs/ARCHITECTURE.md`, "Keyword inventory". The old
//! names, arities and docs are in `mazer-types/src/implfuncs.rs`; the old
//! `MathML` is in `mazer-render/src/lib.rs`.
//!
//! # Writing a keyword
//!
//! Copy the shape of an exemplar: `+` (n-ary infix), `frac` (two required
//! arguments), `paren` (fence) in `algebra.rs`; `integral` (required then
//! optional arguments, big operator) and `sin` (named function) in
//! `calculus.rs`; `subscript` and `quote` in `notation.rs`.
//!
//! **Definition.** One `Keyword::new(name, doc, emitter)` chain per keyword,
//! in the `vec![..]` of `keywords()`:
//! - `doc`: one or two sentences saying what it shows, written for the person
//!   typing the note (it is the LSP hover text). Mention unusual arities, e.g.
//!   what one argument or zero arguments do.
//! - `.alias(..)`: every second name the old code accepted (`add` for `+`,
//!   `pow` for `^`), unless it now clashes with something; no new aliases
//!   without a reason.
//! - Parameters, in order: `.param` (required), then `.optional`, then at most
//!   one `.rest`, last. A registry test enforces the order. Name parameters by
//!   their role in the notation, short and lowercase: `num`/`den`, `base`/
//!   `script`, `body`/`var`/`lower`/`upper`, `terms`, `x` for "the argument of
//!   a function". Each parameter doc is one short sentence; together the names
//!   form the signature shown in signature help, e.g.
//!   `(integral body var [lower] [upper])`.
//! - `.sign(..)`: set it when the keyword has a symbol of its own that can
//!   sensibly stand alone, so that a bare `integral` renders `∫`, a bare `+`
//!   renders `+` and a bare `sin` renders `sin` (as `<mo>`). Leave it unset for
//!   layouts with no symbol (`frac`, `subscript`, `paren`, `table`): a bare use
//!   is then a `BareKeyword` diagnostic, which is what the author wants to see.
//! - `.example(..)`: at least two, as source text, each headed by the keyword
//!   itself (or one of its aliases; `'x` counts for `quote`). Show the common
//!   case first, then the optional arguments or an alias. Every example is
//!   rendered by a test that requires zero diagnostics and exactly one
//!   top-level element, so examples may only use keywords that exist; plain
//!   names like `x`, `f`, `pi` are always fine. Do not rely on a name being in
//!   the display table.
//! - The emitter is a plain `fn(&Call, &mut MathMl)` (a closure without
//!   captures for one-liners, a named `fn` otherwise). Arguments are already
//!   checked: `call.req(i)` and `call.opt(i)` use the PARAMETER position `i`,
//!   `call.rest()` is the rest parameter's arguments.
//!
//! **Emitting.** Only through `MathMl` methods: `mi`/`mn`/`mo`/`mtext` for
//! text (the only places text is escaped), `tag`/`tag_attrs`/`row`/`fence`
//! for structure (closures, so tags always close), `frac`/`sub`/`sup` as
//! shorthand, `expr` (or `operand`) for every argument. Never push a raw
//! string. An emitter must write exactly ONE element: wrap several in
//! `out.row(..)`. Use `<mo>` for operators and symbols and `<mi>` for names
//! (`sin`, `d`, `lim`); a sign written by hand is the same text as `.sign`.
//! Report problems with `out.error(diag)` (pushes the diagnostic and writes a
//! visible `<merror>` in place) or `out.report(diag)` (diagnostic only). Never
//! panic and never return early without writing an element.
//!
//! **Infix and n-ary operators.** `out.infix(op, prec, operands)` writes
//! `a op b op c` in one `<mrow>`; it treats the operator as associative. A
//! non-associative operator (binary `−`, `÷`) writes its own row: the first
//! operand with `out.operand(x, prec)`, later ones with
//! `out.operand(x, prec.tighter())`, so that `(- a (- b c))` shows
//! `a − (b − c)`. Decide what one operand and zero operands mean and say so
//! in the doc (`(+ a)` is unary plus, `(+)` is the sign itself).
//!
//! **Parentheses.** Never parenthesise by looking at a child's head. Instead:
//! 1. If your keyword renders as an infix or prefix operator, add its primary
//!    name to the `precedence` function of your file with its [`Prec`] level
//!    (it may depend on the argument count, as unary `−` does). Anything not
//!    listed is `Prec::Atom` and never gets parentheses.
//! 2. When writing an argument where a loose expression would be misread,
//!    use `out.operand(arg, min)`: it adds parentheses only if the argument
//!    holds together more loosely than `min`. Operands of an infix operator
//!    use the operator's own level; the integrand of `integral` uses
//!    `Prec::Multiplicative`; bases of scripts use `Prec::Atom` (`sub`/`sup`
//!    already do this). Positions that are visually delimited (a numerator,
//!    a script, inside a fence, a function argument) use plain `out.expr`.
//!
//! **Testing.** The registry tests in this file cover every keyword
//! automatically. In addition, give each keyword a unit test in its file's
//! `tests` module asserting the exact `MathML` of an example with
//! `crate::testing::mathml(src)` (which also asserts zero diagnostics), plus
//! one case per optional argument, and for an operator one case where an
//! operand gets parentheses. Use plain Latin letters (`x`, `f`, `V`) in tests:
//! whether `omega` or `pi` is in the display table is not yours to assume.

use crate::keyword::Keyword;
use crate::mathml::Prec;

/// Arithmetic, comparison, sets, logic, grouping.
pub mod algebra;
/// Calculus and named functions (trig, logs, abs, floor, ...).
pub mod calculus;
/// Decoration, linear algebra, `table`, `ty-judge`, font variants, `cases`, `quote`.
pub mod notation;

/// Every built-in keyword.
#[must_use]
pub fn all() -> Vec<Keyword> {
    let mut all = algebra::keywords();
    all.extend(calculus::keywords());
    all.extend(notation::keywords());
    all
}

/// How tightly a call of the keyword whose primary name is `name`, with `args`
/// arguments, holds together once rendered. Keywords no file lists are atoms.
pub(crate) fn precedence(name: &str, args: usize) -> Prec {
    algebra::precedence(name, args)
        .or_else(|| calculus::precedence(name, args))
        .or_else(|| notation::precedence(name, args))
        .unwrap_or(Prec::Atom)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mazer_syntax::Diags;

    use crate::display::display_names;
    use crate::keyword::ParamKind;
    use crate::registry::registry;
    use crate::testing::{read, top_level_elements};

    #[test]
    fn no_two_keywords_share_a_name() {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for keyword in registry() {
            for name in keyword.names() {
                if let Some(other) = seen.insert(name, keyword.name) {
                    panic!("`{name}` is claimed by both `{other}` and `{}`", keyword.name);
                }
            }
        }
    }

    #[test]
    fn no_keyword_is_a_display_name() {
        for keyword in registry() {
            for name in keyword.names() {
                assert!(
                    !display_names().iter().any(|d| d.name == name),
                    "`{name}` (keyword `{}`) is also a display name",
                    keyword.name
                );
            }
        }
    }

    #[test]
    fn parameter_order() {
        for keyword in registry() {
            let kinds: Vec<ParamKind> = keyword.params.iter().map(|p| p.kind).collect();
            if let Some(i) = kinds.iter().position(|&k| k == ParamKind::Rest) {
                assert_eq!(i, kinds.len() - 1, "`{}`: the rest parameter must be last", keyword.name);
            }
            if let Some(first_optional) = kinds.iter().position(|&k| k == ParamKind::Optional) {
                assert!(
                    !kinds[first_optional..].contains(&ParamKind::Required),
                    "`{}`: a required parameter comes after an optional one",
                    keyword.name
                );
            }
        }
    }

    #[test]
    fn docs_and_params_are_filled_in() {
        for keyword in registry() {
            assert!(!keyword.doc.trim().is_empty(), "`{}` has no doc", keyword.name);
            assert!(!keyword.examples.is_empty(), "`{}` has no example", keyword.name);
            let mut names = Vec::new();
            for param in &keyword.params {
                assert!(!param.name.is_empty(), "`{}` has an unnamed parameter", keyword.name);
                assert!(!param.doc.trim().is_empty(), "`{}`: parameter `{}` has no doc", keyword.name, param.name);
                assert!(!names.contains(&param.name), "`{}`: parameter `{}` appears twice", keyword.name, param.name);
                names.push(param.name);
            }
        }
    }

    #[test]
    fn every_example_renders_cleanly() {
        for keyword in registry() {
            for src in &keyword.examples {
                let node = read(src);
                let head = node.head().map(mazer_syntax::Sym::as_str);
                assert!(
                    head.is_some_and(|h| keyword.names().any(|n| n == h)),
                    "`{}`: example {src:?} is not headed by the keyword",
                    keyword.name
                );
                let mut diags = Diags::new();
                let out = crate::to_mathml(&node, &mut diags);
                assert!(diags.is_empty(), "`{}`: example {src:?} gave diagnostics: {diags:?}", keyword.name);
                assert!(!out.is_empty(), "`{}`: example {src:?} rendered nothing", keyword.name);
                assert_eq!(
                    top_level_elements(&out),
                    1,
                    "`{}`: example {src:?} must render exactly one element, got {out}",
                    keyword.name
                );
            }
        }
    }
}
