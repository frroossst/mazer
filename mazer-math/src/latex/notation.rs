//! LaTeX emitters for the keywords of `keywords/notation.rs`.

use std::slice;

use mazer_syntax::{Diag, DiagCode, Node, NodeKind, Sym};

use super::{Latex, LatexFn, signs};
use crate::call::Call;
use crate::display::display_name;
use crate::mathml::Prec;

/// Most primes `(prime x n)` writes as marks; a larger count is written as `x^{(n)}`.
const MAX_PRIMES: usize = 8;

/// The emitters of this file, by the keyword's primary name.
pub(super) fn emitters() -> Vec<(&'static str, LatexFn)> {
    fn e(name: &'static str, f: LatexFn) -> (&'static str, LatexFn) {
        (name, f)
    }
    vec![
        e("subscript", |call, out| out.sub(call.req(0), call.req(1))),
        e("quote", |call, out| {
            let body = call.req(0);
            match body.as_ident() {
                Some(name) => out.name(name),
                None => out.expr(body),
            }
        }),
        e("text", text),
        e("superscript", |call, out| out.sup(call.req(0), call.req(1))),
        e("bar", |call, out| accent(call, out, "\\bar", "\\overline")),
        e("hat", |call, out| accent(call, out, "\\hat", "\\widehat")),
        e("dot", |call, out| accent(call, out, "\\dot", "\\dot")),
        e("ddot", |call, out| accent(call, out, "\\ddot", "\\ddot")),
        e("arrow", |call, out| accent(call, out, "\\vec", "\\overrightarrow")),
        e("box", |call, out| {
            let x = call.req(0);
            out.raw("\\boxed");
            out.group(|o| o.expr(x));
        }),
        e("prime", prime),
        e("funcapp", |call, out| out.application(call.req(0), call.rest())),
        e("mat", |call, out| grid(call, out, "pmatrix")),
        e("vec", |call, out| {
            out.environment("pmatrix", |o| {
                for (i, item) in call.rest().iter().enumerate() {
                    if i > 0 {
                        o.raw(" \\\\ ");
                    }
                    o.expr(item);
                }
            });
        }),
        e("det", |call, out| grid(call, out, "vmatrix")),
        e("table", table),
        e("ty-judge", ty_judge),
        e("bb", |call, out| font(call.req(0), out, "\\mathbb")),
        e("cal", |call, out| font(call.req(0), out, "\\mathcal")),
        e("frak", |call, out| font(call.req(0), out, "\\mathfrak")),
        e("bold", |call, out| font(call.req(0), out, "\\mathbf")),
        e("rm", |call, out| font(call.req(0), out, "\\mathrm")),
        e("cases", cases),
    ]
}

/// `(text w ...)`: the words of every argument, single-spaced, in one `\text{}`.
fn text(call: &Call<'_>, out: &mut Latex<'_>) {
    let mut words: Vec<String> = Vec::new();
    for arg in call.rest() {
        let piece = match &arg.kind {
            NodeKind::Ident(name) => name.as_str().to_owned(),
            NodeKind::Str(text) => text.to_string(),
            // A number as written, or a list as its source text.
            _ => arg.to_string(),
        };
        words.extend(piece.split_whitespace().map(str::to_owned));
    }
    out.text_run(&words.join(" "));
}

/// An accent over the argument. A name or number takes the narrow accent
/// (`\hat{x}`), anything else the wide one where LaTeX has it
/// (`\widehat{a + b}`).
fn accent(call: &Call<'_>, out: &mut Latex<'_>, narrow: &str, wide: &str) {
    let x = call.req(0);
    let single = match &x.kind {
        NodeKind::Ident(name) => name.as_str().chars().count() == 1 || display_name(*name).is_some(),
        NodeKind::Number(text) => text.chars().count() == 1,
        _ => false,
    };
    out.raw(if single { narrow } else { wide });
    out.group(|o| o.expr(x));
}

/// `(prime x [n])`: `x'`, `x''`, ... as a postfix, or `x^{(n)}` for a count that
/// is not a small plain number.
fn prime(call: &Call<'_>, out: &mut Latex<'_>) {
    let x = call.req(0);
    let Some(count) = call.opt(1) else {
        out.operand(x, Prec::Postfix);
        out.raw("'");
        return;
    };
    match plain_count(count) {
        // No primes at all: just the operand, exactly as if `prime` were not there.
        Some(0) => out.expr(x),
        Some(n) => {
            out.operand(x, Prec::Postfix);
            out.raw(&"'".repeat(n));
        }
        None => {
            out.operand(x, Prec::Postfix);
            out.raw("^");
            out.group(|o| {
                o.raw("(");
                o.expr(count);
                o.raw(")");
            });
        }
    }
}

/// The count of a `prime` as a small plain number, if it is one.
fn plain_count(count: &Node) -> Option<usize> {
    match &count.kind {
        NodeKind::Number(text) => text.parse::<usize>().ok().filter(|&n| n <= MAX_PRIMES),
        _ => None,
    }
}

/// The cells of a matrix or table row: the items of a list, or the node
/// itself as a single cell.
fn cells_of(node: &Node) -> &[Node] {
    node.as_list().unwrap_or_else(|| slice::from_ref(node))
}

/// The cells joined by ` & `.
fn cell_row(out: &mut Latex<'_>, cells: &[Node]) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.raw(" & ");
        }
        out.expr(cell);
    }
}

/// `mat` and `det`: an environment with one row per argument.
fn grid(call: &Call<'_>, out: &mut Latex<'_>, env: &str) {
    out.environment(env, |o| {
        for (i, row) in call.rest().iter().enumerate() {
            if i > 0 {
                o.raw(" \\\\ ");
            }
            cell_row(o, cells_of(row));
        }
    });
}

/// How a table looks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TableStyle {
    Grid,
    Clean,
    Plain,
}

/// `(table ...)`: see the keyword doc. Rows and directives are interpreted
/// here, never rendered as function applications. The output is an `array`
/// with `\hline` rules: every rule for `grid`, a frame and a rule under the
/// first row of a table with a header for `clean`, none for `plain`.
fn table(call: &Call<'_>, out: &mut Latex<'_>) {
    let mut style = TableStyle::Grid;
    let mut aligns: Vec<char> = Vec::new();
    let mut rows: Vec<(bool, &[Node])> = Vec::new();

    for arg in call.rest() {
        let Some(items) = arg.as_list().filter(|items| !items.is_empty()) else {
            // A non-list (or an empty list) is a single-cell row.
            rows.push((false, slice::from_ref(arg)));
            continue;
        };
        match items[0].as_ident().map(Sym::as_str) {
            Some("align") => {
                let given: Vec<char> = items[1..].iter().map(align_value).collect();
                if !given.is_empty() {
                    aligns = given;
                }
            }
            Some("style") => match items.get(1).and_then(Node::as_ident).map(Sym::as_str) {
                Some("grid") => style = TableStyle::Grid,
                Some("clean") => style = TableStyle::Clean,
                Some("plain") => style = TableStyle::Plain,
                _ => out.report(
                    Diag::warning(DiagCode::Other, arg.span, "`style` expects `grid`, `clean` or `plain`")
                        .with_help("(style grid|clean|plain)"),
                ),
            },
            Some("header") => rows.push((true, &items[1..])),
            Some("row") => rows.push((false, &items[1..])),
            // A bare list is a row of its own.
            _ => rows.push((false, items)),
        }
    }

    let columns = rows.iter().map(|&(_, cells)| cells.len()).max().unwrap_or(0).max(1);
    let has_header = rows.iter().any(|&(header, _)| header);
    let column = |i: usize| aligns.get(i).or_else(|| aligns.last()).copied().unwrap_or('l');
    let (vertical_all, frame) = match style {
        TableStyle::Grid => (true, true),
        TableStyle::Clean => (false, true),
        TableStyle::Plain => (false, false),
    };
    let mut spec = String::new();
    if frame {
        spec.push('|');
    }
    for i in 0..columns {
        spec.push(column(i));
        if vertical_all && i + 1 < columns {
            spec.push('|');
        }
    }
    if frame {
        spec.push('|');
    }

    // The rule below row `i`.
    let last = rows.len().saturating_sub(1);
    let rule_below = |i: usize| match style {
        TableStyle::Grid => true,
        TableStyle::Clean => (i == 0 && has_header) || i == last,
        TableStyle::Plain => false,
    };

    out.raw("\\begin{array}{");
    out.out.push_str(&spec);
    out.out.push('}');
    let mut parts: Vec<String> = Vec::new();
    if frame {
        parts.push("\\hline".to_owned());
    }
    for (i, &(header, cells)) in rows.iter().enumerate() {
        let mut row = out.capture(|o| {
            for (j, cell) in cells.iter().enumerate() {
                if j > 0 {
                    o.raw(" & ");
                }
                if header {
                    font(cell, o, "\\mathbf");
                } else {
                    o.expr(cell);
                }
            }
        });
        if i < last || rule_below(i) {
            row.push_str(" \\\\");
        }
        parts.push(row);
        if rule_below(i) {
            parts.push("\\hline".to_owned());
        }
    }
    if !parts.is_empty() {
        out.out.push(' ');
        out.out.push_str(&parts.join(" "));
        out.out.push(' ');
    }
    out.out.push_str("\\end{array}");
}

/// A column-alignment word as an `array` column letter; unknown words are left.
fn align_value(node: &Node) -> char {
    let word = match &node.kind {
        NodeKind::Ident(name) => name.as_str(),
        NodeKind::Str(text) => text,
        _ => "",
    };
    match word {
        "right" | "r" => 'r',
        "center" | "centre" | "c" | "middle" | "m" => 'c',
        _ => 'l',
    }
}

/// `(ty-judge name (premises...) conclusion)`: premises over a bar, the
/// conclusion under it and the rule name to the right.
fn ty_judge(call: &Call<'_>, out: &mut Latex<'_>) {
    let (name, premises, conclusion) = (call.req(0), call.req(1), call.req(2));
    // Braced, so a script after the rule attaches to all of it.
    out.group(|o| {
        o.raw("\\dfrac");
        o.group(|o| {
            for (i, premise) in cells_of(premises).iter().enumerate() {
                if i > 0 {
                    o.raw(" \\qquad ");
                }
                o.expr(premise);
            }
        });
        o.group(|o| o.expr(conclusion));
        o.raw(" \\; ");
        // A rule name such as T-VAR is upright text, not an italic variable.
        match &name.kind {
            NodeKind::Ident(word) => o.text_run(word.as_str()),
            NodeKind::Str(text) => o.text_run(text),
            _ => o.expr(name),
        }
    });
}

/// `(cases (value condition) ...)`: a left brace and a two-column array.
fn cases(call: &Call<'_>, out: &mut Latex<'_>) {
    out.environment("cases", |o| {
        for (i, clause) in call.rest().iter().enumerate() {
            let items = cells_of(clause);
            if let [_, _, extra, ..] = items {
                o.report(
                    Diag::warning(DiagCode::Other, extra.span, "a `cases` clause is a value and an optional condition")
                        .with_help("(cases (value condition) ...)"),
                );
            }
            if i > 0 {
                o.raw(" \\\\ ");
            }
            cell_row(o, &items[..items.len().min(2)]);
        }
    });
}

/// A font variant for `bb`, `cal`, `frak`, `bold` and `rm`: `command{...}`,
/// with a plain name or number written bare inside.
fn font(x: &Node, out: &mut Latex<'_>, command: &str) {
    out.raw(command);
    out.group(|o| match &x.kind {
        NodeKind::Ident(name) if display_name(*name).is_none() => {
            let text = name.as_str();
            if text.chars().any(char::is_alphanumeric) {
                let plain = if text.chars().count() == 1 {
                    signs::plain_name(text)
                } else {
                    signs::escape_math(text)
                };
                o.raw(&plain);
            } else {
                o.name(*name);
            }
        }
        NodeKind::Number(text) if !text.starts_with('-') => o.raw(text),
        _ => o.expr(x),
    });
}
