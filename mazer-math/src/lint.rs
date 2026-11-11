//! Checks shared by the `MathML` and `LaTeX` targets for calls that bind
//! cleanly but are very likely written in an old or wrong shape.
//!
//! Both dispatchers call [`call_warnings`] after binding the arguments of a
//! keyword call and [`unknown_head`] for a list whose head is not a keyword, so
//! the two targets (and `check`) report exactly the same thing, once per
//! occurrence. None of these change what is rendered.

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Sym};

use crate::call::Call;
use crate::display::display_name;
use crate::registry::{edit_distance, registry};

/// Keywords the language used to have, with what to write instead. A call
/// headed by one of these renders as a function application, which is almost
/// never what the author meant.
const REMOVED: &[(&str, &str)] = &[
    (
        "string",
        "`string` was removed: write the name directly, e.g. `w`, or `(f x)` for f(x), or `'name` to show a keyword's name",
    ),
    ("define", "`define` was removed: use `(let name value)`"),
    ("defunc", "`defunc` was removed: use `(macro (name params...) body)`"),
];

/// Names of constants and fixed sets. None of them is something one integrates,
/// sums or differentiates over, so in the variable slot of a big operator they
/// mean the arguments are in the old order (`(integral 0 infinity body t)`).
/// Deliberately small: Greek letters, `hbar`, `ell`, `nabla` and the like are
/// plausible variables and are not listed.
const CONSTANTS: &[&str] = &[
    "infinity",
    "emptyset",
    "Reals",
    "reals",
    "Naturals",
    "naturals",
    "Integers",
    "integers",
    "Rationals",
    "rationals",
    "Complex",
    "complex",
    "Complexes",
    "complexes",
];

/// The diagnostic for a list whose head `name` is not a keyword. A removed
/// keyword is a warning: the list renders as a meaningless function
/// application. A did-you-mean is only a hint: the name may be deliberate.
pub fn unknown_head(name: Sym, head: &Node, diags: &mut Diags) {
    if let Some((_, message)) = REMOVED.iter().find(|(removed, _)| *removed == name.as_str()) {
        diags.push(Diag::warning(DiagCode::UnknownForm, head.span, *message));
    } else if display_name(name).is_none()
        && let Some(suggestion) = registry().suggest(name.as_str())
    {
        diags.push(
            Diag::hint(
                DiagCode::UnknownForm,
                head.span,
                format!("`{name}` is not a keyword, so this is shown as a function application"),
            )
            .with_help(format!("did you mean `{suggestion}`?")),
        );
    }
}

/// Warnings about the arguments of a successfully bound keyword call.
pub fn call_warnings(call: &Call<'_>, diags: &mut Diags) {
    let keyword = call.keyword;
    if matches!(keyword.name, "integral" | "sum" | "prod" | "lim" | "derivative" | "partial" | "dd")
        && let Some(index) = keyword.params.iter().position(|p| p.name == "var")
        && let Some(var) = call.opt(index)
        && let Some(what) = not_a_variable(var)
    {
        let mut help = keyword.signature();
        if matches!(keyword.name, "integral" | "sum" | "prod") {
            help.push_str("; the argument order changed: body and variable come first, bounds last");
        }
        diags.push(
            Diag::warning(
                DiagCode::Other,
                var.span,
                format!("the variable of `{}` should be a name, but {what}", keyword.name),
            )
            .with_help(help),
        );
    }
    match keyword.name {
        "ty-judge" => ty_judge_premises(call.req(1), diags),
        "table" => table_children(call.rest(), diags),
        _ => {}
    }
}

/// Why `node` cannot serve, as a clause ("this is a number", "`infinity` is
/// a constant"), if it cannot serve as the variable of a big operator or
/// derivative. A variable is a name, a quoted name, or a name decorated or
/// indexed: `x`, `'x`, `(subscript x i)`, `(superscript x 2)`, `(bar x)`,
/// `(hat x)`, `(dot x)`, `(ddot x)`, `(arrow x)`, `(bold x)`, `(rm x)`,
/// `(vec x y)` and `(prime x)`. Everything else (numbers, strings, other
/// expressions) is almost certainly an argument in the wrong place, typically
/// the old `(integral lower upper body var)` order.
fn not_a_variable(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Ident(name) if CONSTANTS.contains(&name.as_str()) => Some(format!("`{name}` is a constant")),
        NodeKind::Ident(_) | NodeKind::Error => None,
        NodeKind::Number(_) => Some("this is a number".into()),
        NodeKind::Str(_) => Some("this is a string".into()),
        NodeKind::List(items) => {
            let Some((head, args)) = items.split_first() else {
                return Some("this is an empty list".into());
            };
            match (head.as_ident().map(Sym::as_str), args) {
                (Some("quote"), [x]) if x.as_ident().is_some() => None,
                (
                    Some("subscript" | "superscript" | "bar" | "hat" | "dot" | "ddot" | "arrow" | "bold" | "rm" | "prime"),
                    [base, ..],
                ) => not_a_variable(base),
                (Some("vec"), [_, ..]) => None,
                _ => Some("this is an expression".into()),
            }
        }
    }
}

/// `(ty-judge R (= a b) c)`: the premises are a list of premises, so a single
/// premise has to be wrapped. A list headed by a keyword is a premise itself.
fn ty_judge_premises(premises: &Node, diags: &mut Diags) {
    let Some(head) = premises.head() else { return };
    if registry().get(head).is_some() {
        diags.push(
            Diag::warning(
                DiagCode::Other,
                premises.span,
                format!("premises must be a parenthesised list of premises; wrap this single premise: `({premises})`"),
            )
            .with_help("(ty-judge name (premise ...) conclusion)"),
        );
    }
}

/// `(table (rwo a b))`: a bare list is a row, but a head one typo away from a
/// directive is almost certainly that directive misspelt.
fn table_children(children: &[Node], diags: &mut Diags) {
    const DIRECTIVES: [&str; 4] = ["header", "row", "align", "style"];
    for child in children {
        let Some(head) = child.head() else { continue };
        let name = head.as_str();
        if DIRECTIVES.contains(&name) {
            continue;
        }
        if let Some(directive) = DIRECTIVES.into_iter().find(|d| edit_distance(name, d) == 1) {
            diags.push(
                Diag::warning(
                    DiagCode::Other,
                    child.span,
                    format!("`{name}` is not a table directive, so this is a row whose first cell is `{name}`"),
                )
                .with_help(format!("did you mean `({directive} ...)`?")),
            );
        }
    }
}

#[cfg(test)]
#[path = "lint_tests.rs"]
mod tests;
