//! Helpers shared by this crate's tests.

use std::cell::Cell;
use std::collections::HashSet;
use std::sync::LazyLock;

use mazer_syntax::{Diags, Node};

use crate::call::Call;
use crate::keyword::Keyword;
use crate::mathml::{Assoc, MathMl, Prec};
use crate::registry::Registry;

/// Reads `src` as one form. Every test that turns source text into a [`Node`]
/// goes through here, so swapping in the real reader is a one-line change.
pub fn read(src: &str) -> Node {
    let mut diags = Diags::new();
    let node = mazer_syntax::reader::read_str(src, &mut diags);
    assert!(diags.is_empty(), "reading {src:?} gave diagnostics: {diags:?}");
    node
}

/// Renders `src`, returning the output and the diagnostics.
pub fn render(src: &str) -> (String, Diags) {
    let node = read(src);
    let mut diags = Diags::new();
    let out = crate::to_mathml(&node, &mut diags);
    (out, diags)
}

/// Renders `src` and asserts it produced no diagnostics.
pub fn mathml(src: &str) -> String {
    let (out, diags) = render(src);
    assert!(diags.is_empty(), "{src:?} gave diagnostics: {diags:?}");
    out
}

/// What a reader sees in rendered `MathML`: the text with all tags removed,
/// a `<mspace>` shown as one space and the five entities decoded. Asserts
/// zero diagnostics, like [`mathml`].
pub fn visible(src: &str) -> String {
    let xml = mathml(src);
    let mut text = String::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        let end = start + rest[start..].find('>').expect("unclosed tag");
        if rest[start..].starts_with("<mspace") {
            text.push(' ');
        }
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// How many elements sit at the top level of `xml`. Text outside any element
/// counts too, so a well-formed single expression gives exactly 1.
pub fn top_level_elements(xml: &str) -> usize {
    let mut depth = 0usize;
    let mut count = 0;
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        if depth == 0 && !rest[..start].trim().is_empty() {
            count += 1;
        }
        let end = start + rest[start..].find('>').expect("unclosed tag");
        let tag = &rest[start..=end];
        if tag.starts_with("</") {
            depth -= 1;
        } else {
            if depth == 0 {
                count += 1;
            }
            if !tag.ends_with("/>") {
                depth += 1;
            }
        }
        rest = &rest[end + 1..];
    }
    if !rest.trim().is_empty() {
        count += 1;
    }
    assert_eq!(depth, 0, "unbalanced tags in {xml}");
    count
}

/// Every name the finished keyword inventory will use (`docs/ARCHITECTURE.md`,
/// "Keyword inventory"), with their second names, plus the reserved heads.
/// Used to check that no display name takes one of them, that examples only
/// use keywords that exist, and to test did-you-mean against the whole
/// inventory before all of it is written.
pub const PLANNED_KEYWORDS: &[&str] = &[
    "+", "add", "-", "sub", "*", "mul", "/", "div", "^", "pow", "frac", "sqrt", "root", "jux",
    "juxtapose", "=", "eq", "!=", "neq", ">", "gt", "<", "lt", "approx", "≈", ">=", "geq", "<=",
    "leq", "integral", "sum", "prod", "product", "lim", "limit", "derivative", "deriv",
    "partial", "dd", "sin", "cos", "tan", "cot", "sec", "cosec", "csc", "arcsin", "asin",
    "arccos", "acos", "arctan", "atan", "ln", "log", "exp", "abs", "floor", "ceil", "fact",
    "factorial", "binom", "nCr", "mat", "matrix", "vec", "vector", "det", "determinant",
    "table", "ty-judge", "ty-judgement", "set", "in", "notin", "subset", "superset", "union",
    "intersect", "and", "or", "not", "implies", "iff", "forall", "exists", "paren", "bracket",
    "brace", "text", "subscript", "superscript", "bar", "overline", "hat", "dot", "ddot",
    "arrow", "box", "prime", "funcapp", "fapp", "evalat", "eval-at", "quote", "bb", "cal",
    "frak", "bold", "rm", "cases", "show", "eval", "let", "alias", "macro", "ref", "tag",
];

// --- Stand-in keywords for the parenthesis tests ----------------------------
//
// The parenthesis rules involve keywords that step 2 writes (`-`, `*`, `and`,
// `fact`, ...). Until they exist, `with_standins` renders with a registry that
// adds minimal stand-ins for the missing ones, written by the recipes in
// `keywords/mod.rs`. A real keyword always wins over its stand-in, so once
// step 2 lands the same tests check the real keywords. These are NOT
// definitions to copy: they have no real docs, aliases, signs or examples.

thread_local! {
    static STANDINS_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// The real keywords plus the stand-ins whose names are still free, and the
/// names of the stand-ins that were added.
static STANDINS: LazyLock<(Registry, HashSet<&'static str>)> = LazyLock::new(|| {
    let mut keywords = crate::keywords::all();
    let taken: HashSet<&'static str> = keywords.iter().flat_map(Keyword::names).collect();
    let added: Vec<Keyword> = standins()
        .into_iter()
        .filter(|k| !k.names().any(|n| taken.contains(n)))
        .collect();
    let names = added.iter().map(|k| k.name).collect();
    keywords.extend(added);
    (Registry::from_keywords(keywords), names)
});

/// Runs `f` with the stand-in registry in place of the real one (on this
/// thread only).
pub fn with_standins<R>(f: impl FnOnce() -> R) -> R {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            STANDINS_ACTIVE.with(|active| active.set(false));
        }
    }
    STANDINS_ACTIVE.with(|active| active.set(true));
    let _reset = Reset;
    f()
}

/// The registry `registry()` returns inside [`with_standins`].
pub fn active_registry() -> Option<&'static Registry> {
    STANDINS_ACTIVE.with(Cell::get).then(|| &STANDINS.0)
}

/// The level of a stand-in keyword, inside [`with_standins`].
pub fn standin_precedence(name: &str, args: usize) -> Option<Prec> {
    if !STANDINS_ACTIVE.with(Cell::get) || !STANDINS.1.contains(name) {
        return None;
    }
    Some(match (name, args) {
        ("-", 1) => Prec::Prefix,
        ("-", _) => Prec::Additive,
        ("*" | "jux", _) => Prec::Multiplicative,
        ("^", _) => Prec::Power,
        ("fact" | "prime", _) => Prec::Postfix,
        ("=", _) => Prec::Relation,
        ("not", _) => Prec::Not,
        ("and", _) => Prec::And,
        ("or", _) => Prec::Or,
        ("implies" | "iff", _) => Prec::Implication,
        ("forall" | "exists", _) => Prec::Binder,
        _ => return None,
    })
}

fn standins() -> Vec<Keyword> {
    fn minus(call: &Call<'_>, out: &mut MathMl<'_>) {
        match call.rest() {
            [x] => out.prefix("−", x, Prec::BigOp),
            terms => out.infix("−", Prec::Additive, Assoc::Left, terms),
        }
    }
    fn prime(call: &Call<'_>, out: &mut MathMl<'_>) {
        let x = call.req(0);
        out.tag("msup", |o| {
            o.operand(x, Prec::Postfix);
            o.mo("′");
        });
    }
    let rest = |name, f| Keyword::new(name, "Stand-in.", f).rest("xs", "Operands.");
    let one = |name, f| Keyword::new(name, "Stand-in.", f).param("x", "Operand.");
    let two = |name, f| {
        Keyword::new(name, "Stand-in.", f)
            .param("a", "First.")
            .param("b", "Second.")
    };
    vec![
        rest("-", minus),
        rest("*", |c, o| o.infix("×", Prec::Multiplicative, Assoc::Both, c.rest())),
        rest("jux", |c, o| o.juxtapose(c.rest())),
        two("^", |c, o| o.sup(c.req(0), c.req(1))),
        one("fact", |c, o| o.postfix("!", c.req(0))),
        one("prime", prime),
        rest("=", |c, o| o.infix("=", Prec::Relation, Assoc::Neither, c.rest())),
        one("not", |c, o| o.prefix("¬", c.req(0), Prec::Not)),
        rest("and", |c, o| o.infix("∧", Prec::And, Assoc::Both, c.rest())),
        rest("or", |c, o| o.infix("∨", Prec::Or, Assoc::Both, c.rest())),
        two("implies", |c, o| o.binary("⟹", Prec::Implication, Assoc::Neither, c.req(0), c.req(1))),
        two("iff", |c, o| o.binary("⟺", Prec::Implication, Assoc::Neither, c.req(0), c.req(1))),
        two("forall", |c, o| o.binder("∀", c.req(0), c.req(1))),
        two("exists", |c, o| o.binder("∃", c.req(0), c.req(1))),
    ]
}
