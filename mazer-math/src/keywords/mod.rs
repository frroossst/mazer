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
//! Copy the shape of an exemplar: `+` (n-ary infix with a unary form),
//! `frac` (two required arguments, fraction), `paren` (fence) in
//! `algebra.rs`; `integral` (big operator with scripts, required then
//! optional arguments), `sum` (big operator with limits under and over) and
//! `sin` (named function) in `calculus.rs`; `subscript` (script) and `quote`
//! in `notation.rs`. Then check the two tables under "Parentheses" below:
//! they say which level your keyword is at and exactly which helper to call
//! with which arguments.
//!
//! ## Definition
//!
//! One `Keyword::new(name, doc, emitter)` chain per keyword, in the `vec![..]`
//! of `keywords()`:
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
//! - `.sign(..)`: see "Signs" below. Getting it wrong breaks existing notes.
//! - `.example(..)`: **at least two** (a test enforces it), as source text,
//!   each headed by the keyword itself (or one of its aliases; `'x` counts for
//!   `quote`). Show the common case first, then the optional arguments or an
//!   alias. Every example is rendered by a test that requires zero
//!   diagnostics and exactly one top-level element.
//!   **Examples may only use keywords that already exist in the registry.**
//!   An unknown head silently renders as function application (`(cos x)`
//!   before `cos` exists is just "cos(x)" with no diagnostic), so the example
//!   test also fails if an example uses a planned keyword that is not
//!   registered yet. Keywords from the other two files are written in
//!   parallel with yours: do not use them in examples. Plain names like `x`,
//!   `f`, `n` are always fine; do not rely on a name being in the display
//!   table.
//! - The emitter is a plain `fn(&Call, &mut MathMl)` (a closure without
//!   captures for one-liners, a named `fn` otherwise). Arguments are already
//!   checked: `call.req(i)` and `call.opt(i)` use the PARAMETER position `i`,
//!   `call.rest()` is the rest parameter's arguments.
//!
//! ## Signs
//!
//! `.sign(s)` is what the bare keyword (written without parentheses) renders
//! as, always as an `<mo>`. The rule: **every operator, relation, big operator
//! and named function gets a sign equal to the glyph its emitter writes**:
//! `+` "+", `-` "−" (U+2212, not ASCII), `=` "=", `and` "∧", `integral` "∫",
//! `sum` "∑", `sin` "sin". Layouts with no symbol of their own (`frac`,
//! `subscript`, `paren`, `matrix`, `table`, ...) get no sign: a bare use is
//! then a `BareKeyword` diagnostic, which is what the author wants to see.
//!
//! Exceptions, each of which keeps the old output:
//! - **`*` has the sign `*`** (ASCII asterisk), although `(* a b)` renders
//!   `a × b` as the old renderer did. The notes use a bare `*` as a group
//!   operation and the old output showed `*` there.
//! - **Twelve old symbol names are keywords now.** They used to be display
//!   names, and bare use still renders only through the sign, so these
//!   keywords MUST have exactly these signs (a registry test checks every one
//!   that is registered):
//!
//!   | keyword | sign | | keyword | sign | | keyword | sign |
//!   |---|---|---|---|---|---|---|---|
//!   | `partial` | ∂ | | `subset` | ⊂ | | `union` | ∪ |
//!   | `approx` | ≈ | | `in` | ∈ | | `forall` | ∀ |
//!   | `!=` (alias `neq`) | ≠ | | `notin` | ∉ | | `exists` | ∃ |
//!   | `<=` (alias `leq`) | ≤ | | `>=` (alias `geq`) | ≥ | | `arrow` | → |
//!
//!   Note `partial` (∂, although `(partial f x)` writes a fraction) and
//!   `arrow` (→, although `(arrow v)` puts an arrow accent over `v`).
//!   `subset` is ⊂ bare but `(subset a b)` writes ⊆, as the old renderer did;
//!   `superset` follows the general rule (⊇).
//!
//! ## Emitting
//!
//! Only through `MathMl` methods: `mi`/`mn`/`mo`/`mtext` for text (the only
//! places text is escaped), `tag`/`tag_attrs`/`row`/`fence` for structure
//! (closures, so tags always close), the helpers in the recipe table below,
//! and `expr` (or `operand`) for every argument. Never push a raw string. An
//! emitter must write exactly ONE element: wrap several in `out.row(..)`. Use
//! `<mo>` for operators and symbols and `<mi>` for names (`sin`, `d`, `lim`).
//! Report problems with `out.error(diag)` (pushes the diagnostic and writes a
//! visible `<merror>` in place) or `out.report(diag)` (diagnostic only).
//! Never panic and never return early without writing an element.
//!
//! Decide what one operand and zero operands mean for an n-ary keyword and
//! say so in the doc (`(+ a)` is unary plus, `(+)` is the sign itself).
//!
//! ## Parentheses
//!
//! Never parenthesise by looking at a child's head, and never write `(` `)`
//! around an argument by hand. Instead:
//! 1. Put your keyword's primary name in the `precedence` function of your
//!    file with its [`Prec`] level from the first table (it may depend on the
//!    argument count). Anything not listed is `Prec::Atom`.
//! 2. Write it with the helper the second table gives for its kind, with
//!    exactly the arguments shown. The helpers add parentheses where needed.
//! 3. Any argument the tables do not mention sits in a visually delimited
//!    position (inside a fence, a numerator or denominator, a script, a limit
//!    under or over a big operator, a function argument, a radicand, a matrix
//!    cell, an accent's base, a quantifier's variable or body): write it with
//!    plain `out.expr(arg)`.
//!
//! Levels, loosest first, with the keywords at each. "args" is the number of
//! arguments in the call, as passed to `precedence`.
//!
//! | `Prec` | keywords |
//! |---|---|
//! | `Binder` | `forall`, `exists` |
//! | `Implication` | `implies`, `iff` |
//! | `Or` | `or` |
//! | `And` | `and` |
//! | `Not` | `not` |
//! | `Relation` | `=`, `!=`, `<`, `>`, `<=`, `>=`, `approx`, `in`, `notin`, `subset`, `superset` |
//! | `Additive` | `+` and `-` with 2+ args; `union` |
//! | `Prefix` | `+` and `-` with 1 arg. Nothing else may use this level: it means "starts with a sign" |
//! | `BigOp` | `integral`, `sum`, `prod`, `limit` with a body, `derivative`, `partial` (`d/dx f`, `∂/∂x f`) |
//! | `Multiplicative` | `*` and `jux` with 1+ args; `intersect`; `/` with 3+ args (`a ÷ b ÷ c`) |
//! | `Fraction` | `frac`; `/` with 1 or 2 args; `dd` |
//! | `Power` | `^`, `superscript`, `exp` (`eˣ`) |
//! | `Postfix` | `fact`, `prime`, `evalat` |
//! | `Atom` (not listed) | everything else: names, numbers, `subscript`, `sqrt`, `root`, fences (`paren`, `bracket`, `brace`, `abs`, `floor`, `ceil`, `set`, `binom`), named functions (`sin`, `ln`, `log`, `arcsin`, ...), `funcapp`, accents (`bar`, `hat`, `dot`, `ddot`, `arrow`), `box`, `text`, font variants, `matrix`, `vec`, `det`, `table`, `cases`, `ty-judge`, `quote`, `limit` without a body, zero-argument calls that render the sign |
//!
//! Two rules ride on these levels, so keep to them: a keyword at `Additive`
//! or `Multiplicative` level must render as an `infix`/`binary`/`juxtapose`
//! row that starts with its first argument, and one at `Prefix` must render
//! its sign first (the sign rules look through such rows).
//!
//! Recipes by kind. `x`, `a`, `b` are arguments, `xs` is `call.rest()`.
//!
//! | kind | level | emitter |
//! |---|---|---|
//! | associative infix: `+` (2+), `*` (`×`), `union`, `intersect`, `and`, `or` | its level | `out.infix(glyph, level, Assoc::Both, xs)` |
//! | juxtaposition: `jux` | `Multiplicative` | `out.juxtapose(xs)` |
//! | left-grouping infix: binary `-` (`−`), `/` with 3+ args (`÷`) | `Additive` / `Multiplicative` | `out.infix(glyph, level, Assoc::Left, xs)` |
//! | relation, n-ary chain or two parameters | `Relation` | `out.infix(glyph, Prec::Relation, Assoc::Neither, xs)` or `out.binary(glyph, Prec::Relation, Assoc::Neither, a, b)` |
//! | implication: `implies` (`⟹`), `iff` (`⟺`) | `Implication` | `out.binary(glyph, Prec::Implication, Assoc::Neither, a, b)` |
//! | sign: unary `-` (`−`), unary `+` | `Prefix` | `out.prefix(glyph, x, Prec::BigOp)` |
//! | negation: `not` (`¬`) | `Not` | `out.prefix("¬", x, Prec::Not)` |
//! | quantifier: `forall`, `exists` | `Binder` | `out.binder(glyph, var, body)` |
//! | postfix: `fact` | `Postfix` | `out.postfix("!", x)` |
//! | prime | `Postfix` | `out.tag("msup", \|o\| { o.operand(x, Prec::Postfix); o.mo(primes); })` |
//! | evaluated at: `evalat` | `Postfix` | `out.tag("msub", \|o\| { o.row(\|o\| { o.operand(x, Prec::Fraction); o.mo("\|"); }); o.expr(at); })` |
//! | big operator, limits under/over: `sum`, `prod` | `BigOp` | copy `sum`: `out.row(\|o\| { o.under_over(..) or o.under(..); o.operand(body, Prec::BigOp); })` |
//! | big operator, scripts: `integral` | `BigOp` | the `integral` exemplar |
//! | `limit` | `BigOp` | `out.row(\|o\| { o.under(\|o\| o.mi("lim"), \|o\| o.row(..var → to..)); o.operand(body, Prec::BigOp); })` |
//! | derivative operator: `derivative`, `partial` | `BigOp` | `out.row(\|o\| { o.tag("mfrac", ..d.. / ..dx..); o.operand(body, Prec::BigOp); })` |
//! | fraction: `frac`, `/`, `dd` | `Fraction` | `out.frac(num, den)` (or `out.tag("mfrac", ..)` for built-up parts) |
//! | power: `^`, `superscript`, `exp` | `Power` | `out.sup(base, script)` (base parenthesised unless an atom) |
//! | subscript | `Atom` | `out.sub(base, script)` |
//! | root: `sqrt`, `root` | `Atom` | `out.tag("msqrt", ..)`, `out.tag("mroot", ..)` (radicand first, then index), `expr` inside |
//! | fence: `paren`, `bracket`, `brace`, `abs`, `floor`, `ceil`, `set`, `binom` | `Atom` | `out.fence(open, close, \|o\| o.expr(x))`; a list inside uses `o.separated(",", xs)` |
//! | named function: `sin`, `cos`, `ln`, `log`, ... | `Atom` | `out.apply(\|o\| o.mi("cos"), slice::from_ref(x))` |
//! | application: `funcapp` | `Atom` | `out.application(f, args)` |
//! | accent: `bar`, `hat`, `dot`, `ddot`, `arrow` | `Atom` | `out.tag("mover", \|o\| { o.expr(x); o.mo(accent); })` |
//! | other layouts: `text`, `box`, font variants, `matrix`, `vec`, `det`, `table`, `cases`, `ty-judge` | `Atom` | `tag`/`tag_attrs`/`row`/`fence`, every argument with `expr` |
//!
//! What the levels and helpers produce, so you can check your output:
//! `(and (or a b) c)` is `(a ∨ b) ∧ c`; `(implies (implies a b) c)` is
//! `(a ⟹ b) ⟹ c`; `(and (forall x P) Q)` is `(∀x. P) ∧ Q`; `(+ a -2)` is
//! `a + (−2)`; `(- (- x))` is `−(−x)`; `(jux a (- b))` is `a (−b)`;
//! `(* -s t)` is `−s × t`; `(= x -2)` is `x = −2`; `(* (sum a n) b)` is
//! `(∑ a) × b` but `(* b (sum a n))` is `b × ∑ a`; `(sum (+ a b) n)` is
//! `∑ (a + b)`; `(^ (fact n) 2)` is `(n!)²`; `(fact (^ x 2))` is `(x²)!`;
//! `(^ (frac a b) 2)` is `(a/b)²`; `((+ f g) x)` is `(f + g)(x)` but
//! `((prime f) x)` is `f′(x)`. The tests in `mathml.rs` (module
//! `parentheses`) check these against your keywords once they exist: run
//! them.
//!
//! ## Testing
//!
//! The registry tests in this file cover every keyword automatically. In
//! addition, give each keyword a unit test in its file's `tests` module
//! asserting the exact `MathML` of an example with
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
    let declared = algebra::precedence(name, args)
        .or_else(|| calculus::precedence(name, args))
        .or_else(|| notation::precedence(name, args));
    // The parenthesis tests use stand-ins for keywords not written yet.
    #[cfg(test)]
    let declared = declared.or_else(|| crate::testing::standin_precedence(name, args));
    declared.unwrap_or(Prec::Atom)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mazer_syntax::{Diags, Node, Sym};

    use crate::display::display_names;
    use crate::keyword::ParamKind;
    use crate::registry::registry;
    use crate::testing::{PLANNED_KEYWORDS, read, top_level_elements};

    /// The twelve old symbol names that are keywords now, by the name the old
    /// notes use, with the sign the keyword must have (see "Signs" above),
    /// and the other signs that differ from what one might guess.
    const REQUIRED_SIGNS: &[(&str, &str)] = &[
        ("partial", "∂"),
        ("approx", "≈"),
        ("neq", "≠"),
        ("leq", "≤"),
        ("geq", "≥"),
        ("subset", "⊂"),
        ("in", "∈"),
        ("notin", "∉"),
        ("union", "∪"),
        ("forall", "∀"),
        ("exists", "∃"),
        ("arrow", "→"),
        ("*", "*"),
    ];

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
            assert!(
                keyword.examples.len() >= 2,
                "`{}` needs at least two examples, has {}",
                keyword.name,
                keyword.examples.len()
            );
            let mut names = Vec::new();
            for param in &keyword.params {
                assert!(!param.name.is_empty(), "`{}` has an unnamed parameter", keyword.name);
                assert!(!param.doc.trim().is_empty(), "`{}`: parameter `{}` has no doc", keyword.name, param.name);
                assert!(!names.contains(&param.name), "`{}`: parameter `{}` appears twice", keyword.name, param.name);
                names.push(param.name);
            }
        }
    }

    /// Every list head in `node` that names a planned keyword which is not
    /// registered (yet).
    fn unregistered_heads(node: &Node, found: &mut Vec<String>) {
        if let Some(items) = node.as_list() {
            if let Some(head) = node.head()
                && PLANNED_KEYWORDS.contains(&head.as_str())
                && registry().get(head).is_none()
            {
                found.push(head.as_str().to_owned());
            }
            for item in items {
                unregistered_heads(item, found);
            }
        }
    }

    #[test]
    fn every_example_renders_cleanly() {
        for keyword in registry() {
            for src in &keyword.examples {
                let node = read(src);
                let head = node.head().map(Sym::as_str);
                assert!(
                    head.is_some_and(|h| keyword.names().any(|n| n == h)),
                    "`{}`: example {src:?} is not headed by the keyword",
                    keyword.name
                );
                let mut missing = Vec::new();
                unregistered_heads(&node, &mut missing);
                assert!(
                    missing.is_empty(),
                    "`{}`: example {src:?} uses {missing:?}, which is not a keyword yet; it would \
                     silently render as a function application",
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

    /// Checks the required signs of the keywords that exist, and lists the
    /// ones that do not exist yet (without failing).
    #[test]
    fn required_signs() {
        let mut missing = Vec::new();
        for &(name, sign) in REQUIRED_SIGNS {
            match registry().get(Sym::new(name)) {
                Some(keyword) => assert_eq!(
                    keyword.sign,
                    Some(sign),
                    "bare `{name}` (keyword `{}`) must render as {sign}",
                    keyword.name
                ),
                None => missing.push(name),
            }
        }
        if !missing.is_empty() {
            println!("required signs not checked yet, no such keyword: {missing:?}");
        }
    }

    #[test]
    #[ignore = "enable after step 2"]
    fn required_signs_all_present() {
        let missing: Vec<&str> = REQUIRED_SIGNS
            .iter()
            .map(|&(name, _)| name)
            .filter(|&name| registry().get(Sym::new(name)).is_none())
            .collect();
        assert!(missing.is_empty(), "keywords with a required sign are missing: {missing:?}");
        // `!=`, `<=` and `>=` are the primary names; the old names are aliases.
        for (primary, alias) in [("!=", "neq"), ("<=", "leq"), (">=", "geq")] {
            let keyword = registry().get(Sym::new(alias)).expect("checked above");
            assert_eq!(keyword.name, primary, "`{alias}` must be an alias of `{primary}`");
        }
    }
}
