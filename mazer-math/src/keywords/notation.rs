//! Decoration, linear algebra, `table`, `ty-judge`, font variants, `cases`, `quote`.

use std::slice;

use mazer_syntax::{Diag, DiagCode, Node, NodeKind, Sym};

use crate::call::Call;
use crate::display::display_name;
use crate::keyword::Keyword;
use crate::mathml::{MathMl, Prec};

/// Most primes `(prime x n)` writes as marks; a larger count is written as `x^(n)`.
const MAX_PRIMES: usize = 8;

#[must_use]
#[allow(clippy::too_many_lines)] // one builder chain per keyword, as in the other keyword files
pub fn keywords() -> Vec<Keyword> {
    vec![
        Keyword::new("subscript", "Attaches a subscript to an expression: `(subscript x i)` shows xᵢ.", |call, out| {
            out.sub(call.req(0), call.req(1));
        })
        .param("base", "The expression the subscript is attached to.")
        .param("script", "The subscript, written smaller and lower.")
        .example("(subscript x i)")
        .example("(subscript a (+ n 1))"),
        Keyword::new(
            "quote",
            "Shows its argument as written, without substituting `let` variables: after \
             `(let x 2)`, `'x` still shows the name x. A quoted name is never read as a \
             keyword. `'x` is short for `(quote x)`.",
            |call, out| {
                let body = call.req(0);
                match body.as_ident() {
                    Some(name) => out.name(name),
                    None => out.expr(body),
                }
            },
        )
        .param("expr", "The name or expression to show as written.")
        .example("(quote x)")
        .example("'integral"),
        // --- Decoration ---------------------------------------------------
        Keyword::new(
            "text",
            "Plain upright text inside a formula: `(text area under the curve)` or \
             `(text \"area under the curve\")`. The words, bare or quoted, are joined with \
             single spaces.",
            text,
        )
        .rest("words", "The words to show: bare names, numbers or quoted strings.")
        .example("(text area under the curve)")
        .example("(text \"area under the curve\")")
        .example("(text for n 2)"),
        Keyword::new(
            "superscript",
            "Attaches a superscript to an expression: `(superscript x 2)` shows x².",
            |call, out| out.sup(call.req(0), call.req(1)),
        )
        .param("base", "The expression the superscript is attached to.")
        .param("script", "The superscript, written smaller and higher.")
        .example("(superscript x 2)")
        .example("(superscript (subscript a n) (+ n 1))"),
        Keyword::new("bar", "Draws a bar over an expression: `(bar x)` shows x̄.", |call, out| {
            accent(call, out, "¯");
        })
        .alias("overline")
        .param("x", "The expression to put the bar over.")
        .example("(bar x)")
        .example("(overline (+ a b))"),
        Keyword::new("hat", "Puts a hat over an expression: `(hat x)` shows x̂.", |call, out| {
            accent(call, out, "^");
        })
        .param("x", "The expression to put the hat over.")
        .example("(hat x)")
        .example("(hat (subscript v i))"),
        Keyword::new("dot", "Puts a dot over an expression, for a time derivative: `(dot x)` shows ẋ.", |call, out| {
            accent(call, out, "˙");
        })
        .param("x", "The expression to put the dot over.")
        .example("(dot x)")
        .example("(dot (subscript q i))"),
        Keyword::new(
            "ddot",
            "Puts two dots over an expression, for a second time derivative: `(ddot x)` shows ẍ.",
            |call, out| accent(call, out, "¨"),
        )
        .param("x", "The expression to put the two dots over.")
        .example("(ddot x)")
        .example("(ddot (subscript q i))"),
        Keyword::new(
            "arrow",
            "Puts a vector arrow over an expression: `(arrow v)` shows v with → above it. \
             Written on its own, `arrow` is the glyph →.",
            |call, out| accent(call, out, "→"),
        )
        .param("x", "The expression to put the arrow over.")
        .sign("→")
        .example("(arrow v)")
        .example("(arrow (+ a b))"),
        Keyword::new("box", "Draws a box around an expression, to highlight it.", |call, out| {
            out.tag_attrs("mrow", &[("style", "border: 1px solid black; padding: 0.2em;")], |o| {
                o.expr(call.req(0));
            });
        })
        .param("x", "The expression to box.")
        .example("(box (+ a b))")
        .example("(box (frac 1 2))"),
        Keyword::new(
            "prime",
            "Primes an expression: `(prime f)` shows f′ and `(prime f 2)` shows f″. A count \
             above 8, or one that is not a plain number, is written as f^(n).",
            prime,
        )
        .param("x", "The expression to prime.")
        .optional("count", "How many primes; defaults to 1.")
        .example("(prime f)")
        .example("(prime f 2)")
        .example("(prime f n)"),
        Keyword::new(
            "funcapp",
            "Applies any expression as a function: `(funcapp f x y)` shows f(x, y). With only \
             the head, `(funcapp f)` shows just f.",
            |call, out| out.application(call.req(0), call.rest()),
        )
        .alias("fapp")
        .param("head", "The function: a name or any expression.")
        .rest("args", "The arguments, shown in parentheses separated by commas.")
        .example("(funcapp f x)")
        .example("(funcapp f x y)")
        .example("(fapp (+ f g) x)"),
        // --- Linear algebra -----------------------------------------------
        Keyword::new(
            "mat",
            "A matrix in parentheses; each argument is one row, written as a list of cells: \
             `(mat (a b) (c d))`. A row that is not a list is a one-cell row.",
            |call, out| grid(call, out, "(", ")"),
        )
        .alias("matrix")
        .rest("rows", "The rows, each a list of cells.")
        .example("(mat (a b) (c d))")
        .example("(matrix (1 0) (0 1))"),
        Keyword::new(
            "vec",
            "A column vector in parentheses: `(vec a b c)` stacks a, b and c.",
            |call, out| {
                out.fence("(", ")", |o| {
                    o.tag("mtable", |o| {
                        for item in call.rest() {
                            cell_row(o, slice::from_ref(item));
                        }
                    });
                });
            },
        )
        .alias("vector")
        .rest("items", "The entries, from top to bottom.")
        .example("(vec x y z)")
        .example("(vector 1 2)"),
        Keyword::new(
            "det",
            "A determinant: a matrix between vertical bars. Each argument is one row, written \
             as a list of cells: `(det (a b) (c d))`.",
            |call, out| grid(call, out, "|", "|"),
        )
        .alias("determinant")
        .rest("rows", "The rows, each a list of cells.")
        .example("(det (a b) (c d))")
        .example("(determinant (1 2) (3 4))"),
        Keyword::new(
            "table",
            "A table. Each `(row a b ...)` is a data row and `(header a b ...)` a bold header \
             row; a bare list is also a row. `(align left right center)` sets the column \
             alignment (the last value repeats) and `(style grid)`, `(style clean)` or \
             `(style plain)` the look; grid is the default.",
            table,
        )
        .rest("rows", "Rows `(row ...)`, `(header ...)`, and the directives `(align ...)` and `(style ...)`.")
        .example("(table (header Name Age) (row Alice 30) (row Bob 25))")
        .example("(table (style clean) (align left right) (header n f) (row 1 (frac 1 2)))")
        .example("(table (style plain) (a b) (c d))"),
        Keyword::new(
            "ty-judge",
            "An inference rule: the premises above a bar, the conclusion below, and the rule \
             name to the right: `(ty-judge T-Var (premise ...) conclusion)`. The premises are a \
             list, or a single expression.",
            ty_judge,
        )
        .alias("ty-judgement")
        .param("name", "The rule name, shown upright to the right of the bar.")
        .param("premises", "A list of the premises, shown side by side above the bar.")
        .param("conclusion", "The conclusion, below the bar.")
        .example("(ty-judge T-Var ((subscript x i)) (+ x y))")
        .example("(ty-judge \"T-App\" ((+ a b) (+ c d)) (frac a b))"),
        // --- Font variants --------------------------------------------------
        Keyword::new("bb", "Blackboard bold (double-struck) letters: `(bb R)` shows ℝ.", |call, out| {
            font(call, out, Font::DoubleStruck);
        })
        .param("x", "The name, number or expression to restyle.")
        .example("(bb R)")
        .example("(bb (+ a b))"),
        Keyword::new("cal", "Calligraphic (script) letters: `(cal L)` shows ℒ.", |call, out| {
            font(call, out, Font::Script);
        })
        .param("x", "The name, number or expression to restyle.")
        .example("(cal L)")
        .example("(cal (subscript F n))"),
        Keyword::new("frak", "Fraktur letters: `(frak g)` shows 𝔤.", |call, out| {
            font(call, out, Font::Fraktur);
        })
        .param("x", "The name, number or expression to restyle.")
        .example("(frak g)")
        .example("(frak (+ a b))"),
        Keyword::new("bold", "Bold letters and digits: `(bold v)` shows a bold v.", |call, out| {
            font(call, out, Font::Bold);
        })
        .param("x", "The name, number or expression to restyle.")
        .example("(bold v)")
        .example("(bold (+ a b))"),
        Keyword::new("rm", "Upright roman letters instead of italic: `(rm d)` shows an upright d.", |call, out| {
            font(call, out, Font::Roman);
        })
        .param("x", "The name, number or expression to restyle.")
        .example("(rm d)")
        .example("(rm (subscript T max))"),
        Keyword::new(
            "cases",
            "A piecewise definition with a left brace. Each clause is a list `(value condition)`; \
             a clause with only a value has no condition. Write a condition as an expression, or \
             as `(text otherwise)`.",
            cases,
        )
        .rest("clauses", "The clauses, each a list of a value and an optional condition.")
        .example("(cases (x (text if x is positive)) (0 (text otherwise)))")
        .example("(cases (a b) (c))"),
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`. The
/// registry resolves second names to the primary name before asking, but
/// listing both is harmless.
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    let _ = args;
    match name {
        "superscript" => Some(Prec::Power),
        "prime" => Some(Prec::Postfix),
        _ => None,
    }
}

/// `(text w ...)`: the words of every argument, single-spaced, in one `<mtext>`.
fn text(call: &Call<'_>, out: &mut MathMl<'_>) {
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
    out.mtext(&words.join(" "));
}

/// An accent over the argument: `<mover>x <mo>accent</mo></mover>`.
fn accent(call: &Call<'_>, out: &mut MathMl<'_>, glyph: &str) {
    out.tag("mover", |o| {
        o.expr(call.req(0));
        o.mo(glyph);
    });
}

/// `(prime x [n])`: `x′`, `x″`, ... as a postfix, or `x^(n)` for a count that
/// is not a small plain number.
fn prime(call: &Call<'_>, out: &mut MathMl<'_>) {
    let x = call.req(0);
    let Some(count) = call.opt(1) else {
        out.tag("msup", |o| {
            o.operand(x, Prec::Postfix);
            o.mo("′");
        });
        return;
    };
    let plain = match &count.kind {
        NodeKind::Number(text) => text.parse::<usize>().ok().filter(|&n| n <= MAX_PRIMES),
        _ => None,
    };
    match plain {
        // No primes at all: just the operand.
        Some(0) => out.operand(x, Prec::Postfix),
        Some(n) => out.tag("msup", |o| {
            o.operand(x, Prec::Postfix);
            o.mo(&"′".repeat(n));
        }),
        None => out.tag("msup", |o| {
            o.operand(x, Prec::Postfix);
            o.fence("(", ")", |o| o.expr(count));
        }),
    }
}

/// The cells of a matrix or table row: the items of a list, or the node
/// itself as a single cell.
fn cells_of(node: &Node) -> &[Node] {
    node.as_list().unwrap_or_else(|| slice::from_ref(node))
}

/// `<mtr><mtd>cell</mtd> ...</mtr>`
fn cell_row(out: &mut MathMl<'_>, cells: &[Node]) {
    out.tag("mtr", |o| {
        for cell in cells {
            o.tag("mtd", |o| o.expr(cell));
        }
    });
}

/// `mat` and `det`: a fenced `<mtable>` with one row per argument.
fn grid(call: &Call<'_>, out: &mut MathMl<'_>, open: &str, close: &str) {
    out.fence(open, close, |o| {
        o.tag("mtable", |o| {
            for row in call.rest() {
                cell_row(o, cells_of(row));
            }
        });
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
/// here, never rendered as function applications.
fn table(call: &Call<'_>, out: &mut MathMl<'_>) {
    let mut style = TableStyle::Grid;
    let mut aligns: Vec<&'static str> = Vec::new();
    let mut rows: Vec<(bool, &[Node])> = Vec::new();

    for arg in call.rest() {
        let Some(items) = arg.as_list().filter(|items| !items.is_empty()) else {
            // A non-list (or an empty list) is a single-cell row.
            rows.push((false, slice::from_ref(arg)));
            continue;
        };
        match items[0].as_ident().map(Sym::as_str) {
            Some("align") => {
                let given: Vec<&'static str> = items[1..].iter().map(align_value).collect();
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

    let has_header = rows.iter().any(|&(header, _)| header);
    let column_align = if aligns.is_empty() { "left".to_owned() } else { aligns.join(" ") };
    let mut attrs: Vec<(&str, &str)> = vec![
        ("columnspacing", "1em"),
        ("rowspacing", "0.5em"),
        ("columnalign", &column_align),
    ];
    match style {
        // Lines between every row and column, plus an outer frame.
        TableStyle::Grid => attrs.extend([("frame", "solid"), ("rowlines", "solid"), ("columnlines", "solid")]),
        // An outer frame and one rule under the header, if there is one.
        TableStyle::Clean => {
            attrs.extend([("frame", "solid"), ("columnlines", "none")]);
            if has_header {
                attrs.push(("rowlines", "solid none"));
            }
        }
        TableStyle::Plain => {}
    }

    out.tag_attrs("mtable", &attrs, |o| {
        for (header, cells) in rows {
            o.tag("mtr", |o| {
                for cell in cells {
                    o.tag("mtd", |o| {
                        if header {
                            o.tag_attrs("mstyle", &[("mathvariant", "bold")], |o| o.expr(cell));
                        } else {
                            o.expr(cell);
                        }
                    });
                }
            });
        }
    });
}

/// A column-alignment word as a `columnalign` value; unknown words are left.
fn align_value(node: &Node) -> &'static str {
    let word = match &node.kind {
        NodeKind::Ident(name) => name.as_str(),
        NodeKind::Str(text) => text,
        _ => "",
    };
    match word {
        "right" | "r" => "right",
        "center" | "centre" | "c" | "middle" | "m" => "center",
        _ => "left",
    }
}

/// `(ty-judge name (premises...) conclusion)`: premises over a bar, the
/// conclusion under it and the rule name to the right.
fn ty_judge(call: &Call<'_>, out: &mut MathMl<'_>) {
    let (name, premises, conclusion) = (call.req(0), call.req(1), call.req(2));
    out.row(|o| {
        o.tag("mfrac", |o| {
            o.row(|o| {
                for (i, premise) in cells_of(premises).iter().enumerate() {
                    if i > 0 {
                        o.space("2em");
                    }
                    o.expr(premise);
                }
            });
            o.row(|o| o.expr(conclusion));
        });
        o.space("0.5em");
        // A rule name such as T-VAR is upright text, not an italic variable.
        match &name.kind {
            NodeKind::Ident(word) => o.mtext(word.as_str()),
            NodeKind::Str(text) => o.mtext(text),
            _ => o.expr(name),
        }
    });
}

/// `(cases (value condition) ...)`: a left brace and a two-column table.
fn cases(call: &Call<'_>, out: &mut MathMl<'_>) {
    out.fence("{", "", |o| {
        let attrs = [("columnalign", "left left"), ("columnspacing", "1em"), ("rowspacing", "0.4em")];
        o.tag_attrs("mtable", &attrs, |o| {
            for clause in call.rest() {
                let items = cells_of(clause);
                if let [_, _, extra, ..] = items {
                    o.report(
                        Diag::warning(DiagCode::Other, extra.span, "a `cases` clause is a value and an optional condition")
                            .with_help("(cases (value condition) ...)"),
                    );
                }
                cell_row(o, &items[..items.len().min(2)]);
            }
        });
    });
}

/// A font variant for `bb`, `cal`, `frak`, `bold` and `rm`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Font {
    Bold,
    DoubleStruck,
    Script,
    Fraktur,
    Roman,
}

impl Font {
    /// The `mathvariant` value.
    const fn variant(self) -> &'static str {
        match self {
            Self::Bold => "bold",
            Self::DoubleStruck => "double-struck",
            Self::Script => "script",
            Self::Fraktur => "fraktur",
            Self::Roman => "normal",
        }
    }

    /// The Unicode mathematical alphanumeric for `ch`, so the variant also shows
    /// in browsers that only honour `mathvariant="normal"`. Other characters
    /// are left as they are.
    fn style(self, ch: char) -> char {
        let (upper, lower, digit, holes): (u32, u32, Option<u32>, &[(char, char)]) = match self {
            Self::Bold => (0x1D400, 0x1D41A, Some(0x1D7CE), &[]),
            Self::DoubleStruck => (
                0x1D538,
                0x1D552,
                Some(0x1D7D8),
                &[('C', 'ℂ'), ('H', 'ℍ'), ('N', 'ℕ'), ('P', 'ℙ'), ('Q', 'ℚ'), ('R', 'ℝ'), ('Z', 'ℤ')],
            ),
            Self::Script => (
                0x1D49C,
                0x1D4B6,
                None,
                &[
                    ('B', 'ℬ'),
                    ('E', 'ℰ'),
                    ('F', 'ℱ'),
                    ('H', 'ℋ'),
                    ('I', 'ℐ'),
                    ('L', 'ℒ'),
                    ('M', 'ℳ'),
                    ('R', 'ℛ'),
                    ('e', 'ℯ'),
                    ('g', 'ℊ'),
                    ('o', 'ℴ'),
                ],
            ),
            Self::Fraktur => (
                0x1D504,
                0x1D51E,
                None,
                &[('C', 'ℭ'), ('H', 'ℌ'), ('I', 'ℑ'), ('R', 'ℜ'), ('Z', 'ℨ')],
            ),
            Self::Roman => return ch,
        };
        if let Some(&(_, mapped)) = holes.iter().find(|&&(from, _)| from == ch) {
            return mapped;
        }
        let code = match ch {
            'A'..='Z' => upper + (u32::from(ch) - u32::from('A')),
            'a'..='z' => lower + (u32::from(ch) - u32::from('a')),
            '0'..='9' => match digit {
                Some(start) => start + (u32::from(ch) - u32::from('0')),
                None => return ch,
            },
            _ => return ch,
        };
        char::from_u32(code).unwrap_or(ch)
    }

    fn apply(self, text: &str) -> String {
        text.chars().map(|ch| self.style(ch)).collect()
    }
}

/// A font variant: one styled token for a name or number, otherwise the
/// normally rendered argument, all inside `<mstyle mathvariant=...>`.
fn font(call: &Call<'_>, out: &mut MathMl<'_>, font: Font) {
    let x = call.req(0);
    out.tag_attrs("mstyle", &[("mathvariant", font.variant())], |o| match &x.kind {
        NodeKind::Ident(name) if display_name(*name).is_none() => {
            let text = name.as_str();
            if !text.chars().any(char::is_alphanumeric) {
                o.name(*name);
            } else if font == Font::Roman {
                // `<mi>` would be italicised again if it is a single letter.
                o.mtext(text);
            } else {
                o.mi(&font.apply(text));
            }
        }
        NodeKind::Number(text) if !text.starts_with('-') => o.mn(&font.apply(text)),
        _ => o.expr(x),
    });
}

#[cfg(test)]
mod tests {
    use crate::testing::{mathml, render, visible, with_standins};

    #[test]
    fn subscript() {
        assert_eq!(mathml("(subscript x i)"), "<msub><mi>x</mi><mi>i</mi></msub>");
        assert_eq!(
            mathml("(subscript a (+ n 1))"),
            "<msub><mi>a</mi><mrow><mi>n</mi><mo>+</mo><mn>1</mn></mrow></msub>"
        );
        // A base that is not an atom gets parentheses: (a + b)ᵢ.
        assert_eq!(
            mathml("(subscript (+ a b) i)"),
            "<msub><mrow><mo>(</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>)</mo></mrow><mi>i</mi></msub>"
        );
    }

    #[test]
    fn quote() {
        assert_eq!(mathml("(quote x)"), "<mi>x</mi>");
        assert_eq!(mathml("'x"), "<mi>x</mi>");
        // A quoted keyword name is just a name: no sign, no diagnostic.
        assert_eq!(mathml("'integral"), "<mi>integral</mi>");
        assert_eq!(mathml("'frac"), "<mi>frac</mi>");
        // A quoted expression renders normally.
        assert_eq!(mathml("'(+ a b)"), "<mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow>");
    }

    #[test]
    fn text_words_string_and_mixed() {
        assert_eq!(mathml("(text area under the curve)"), "<mtext>area under the curve</mtext>");
        assert_eq!(mathml("(text \"area under the curve\")"), "<mtext>area under the curve</mtext>");
        assert_eq!(mathml("(text area \"under  the\" curve 2)"), "<mtext>area under the curve 2</mtext>");
        assert_eq!(mathml("(text)"), "<mtext></mtext>");
        // A nested list is its source text.
        assert_eq!(mathml("(text see (a b))"), "<mtext>see (a b)</mtext>");
        // Words are never read as keywords.
        assert_eq!(mathml("(text sum of frac)"), "<mtext>sum of frac</mtext>");
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(mathml("(text \"a<b & \\\"q\\\"\")"), "<mtext>a&lt;b &amp; &quot;q&quot;</mtext>");
        assert_eq!(visible("(text \"a<b & 'c'\")"), "a<b & 'c'");
    }

    #[test]
    fn superscript() {
        assert_eq!(mathml("(superscript x 2)"), "<msup><mi>x</mi><mn>2</mn></msup>");
        assert_eq!(
            mathml("(superscript (+ a b) 2)"),
            "<msup><mrow><mo>(</mo><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>)</mo></mrow><mn>2</mn></msup>"
        );
        // Power level: a power as a base is parenthesised, as a script base is.
        assert_eq!(visible("(superscript (superscript x 2) 3)"), "(x2)3");
    }

    #[test]
    fn accents() {
        assert_eq!(mathml("(bar x)"), "<mover><mi>x</mi><mo>¯</mo></mover>");
        assert_eq!(mathml("(overline x)"), "<mover><mi>x</mi><mo>¯</mo></mover>");
        assert_eq!(mathml("(hat x)"), "<mover><mi>x</mi><mo>^</mo></mover>");
        assert_eq!(mathml("(dot x)"), "<mover><mi>x</mi><mo>˙</mo></mover>");
        assert_eq!(mathml("(ddot x)"), "<mover><mi>x</mi><mo>¨</mo></mover>");
        assert_eq!(mathml("(arrow v)"), "<mover><mi>v</mi><mo>→</mo></mover>");
    }

    #[test]
    fn accents_over_a_compound_argument() {
        // The base of an accent is delimited by the accent: no parentheses.
        assert_eq!(
            mathml("(bar (+ a b))"),
            "<mover><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>¯</mo></mover>"
        );
        assert_eq!(
            mathml("(hat (subscript v i))"),
            "<mover><msub><mi>v</mi><mi>i</mi></msub><mo>^</mo></mover>"
        );
        assert_eq!(visible("(arrow (frac a b))"), "ab→");
        assert_eq!(visible("(dot (ddot x))"), "x¨˙");
    }

    #[test]
    fn bare_arrow_is_a_glyph() {
        assert_eq!(mathml("arrow"), "<mo>→</mo>");
        with_standins(|| {
            assert_eq!(
                mathml("(jux alpha arrow alpha)"),
                "<mrow><mi>α</mi><mspace width=\"0.167em\"/><mo>→</mo><mspace width=\"0.167em\"/><mi>α</mi></mrow>"
            );
        });
    }

    #[test]
    fn boxed() {
        assert_eq!(
            mathml("(box x)"),
            "<mrow style=\"border: 1px solid black; padding: 0.2em;\"><mi>x</mi></mrow>"
        );
        assert_eq!(
            mathml("(box (frac 1 2))"),
            "<mrow style=\"border: 1px solid black; padding: 0.2em;\"><mfrac><mn>1</mn><mn>2</mn></mfrac></mrow>"
        );
    }

    #[test]
    fn prime_counts() {
        assert_eq!(mathml("(prime f)"), "<msup><mi>f</mi><mo>′</mo></msup>");
        assert_eq!(mathml("(prime f 1)"), "<msup><mi>f</mi><mo>′</mo></msup>");
        assert_eq!(mathml("(prime f 2)"), "<msup><mi>f</mi><mo>′′</mo></msup>");
        assert_eq!(mathml("(prime f 3)"), "<msup><mi>f</mi><mo>′′′</mo></msup>");
        assert_eq!(mathml("(prime f 0)"), "<mi>f</mi>");
        // A symbolic or large count is a parenthesised superscript.
        assert_eq!(
            mathml("(prime f n)"),
            "<msup><mi>f</mi><mrow><mo>(</mo><mi>n</mi><mo>)</mo></mrow></msup>"
        );
        assert_eq!(visible("(prime f 100)"), "f(100)");
        // Postfix: the operand is parenthesised unless atomic.
        assert_eq!(visible("(prime (+ a b))"), "(a+b)′");
        assert_eq!(visible("(prime (prime f))"), "f′′");
        assert_eq!(visible("(superscript (prime f) 2)"), "(f′)2");
    }

    #[test]
    fn funcapp() {
        assert_eq!(mathml("(funcapp f)"), "<mi>f</mi>");
        assert_eq!(
            mathml("(funcapp f x)"),
            "<mrow><mi>f</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(
            mathml("(funcapp f x y)"),
            "<mrow><mi>f</mi><mrow><mo>(</mo><mi>x</mi><mo>,</mo><mi>y</mi><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(visible("(fapp f x y z)"), "f(x,y,z)");
        // The head may be any expression; a loose one is parenthesised.
        assert_eq!(visible("(funcapp (+ f g) x)"), "(f+g)(x)");
        assert_eq!(visible("(funcapp (prime f) x)"), "f′(x)");
        assert_eq!(visible("(funcapp delta (+ t a))"), "δ(t+a)");
    }

    #[test]
    fn matrices() {
        assert_eq!(
            mathml("(mat (a b) (c d))"),
            "<mrow><mo>(</mo><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr>\
             <mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable><mo>)</mo></mrow>"
        );
        assert_eq!(mathml("(matrix (1 0) (0 1))"), mathml("(mat (1 0) (0 1))"));
        // Cells are expressions, and a row that is not a list is one cell.
        assert_eq!(visible("(mat ((frac a b) 0) 1)"), "(ab01)");
        assert_eq!(
            mathml("(mat x)"),
            "<mrow><mo>(</mo><mtable><mtr><mtd><mi>x</mi></mtd></mtr></mtable><mo>)</mo></mrow>"
        );
        assert_eq!(mathml("(mat)"), "<mrow><mo>(</mo><mtable></mtable><mo>)</mo></mrow>");
    }

    #[test]
    fn vectors() {
        assert_eq!(
            mathml("(vec x y z)"),
            "<mrow><mo>(</mo><mtable><mtr><mtd><mi>x</mi></mtd></mtr><mtr><mtd><mi>y</mi></mtd></mtr>\
             <mtr><mtd><mi>z</mi></mtd></mtr></mtable><mo>)</mo></mrow>"
        );
        assert_eq!(mathml("(vector 1 2)"), mathml("(vec 1 2)"));
        // An entry that is a list is one expression, not a row.
        assert_eq!(visible("(vec (+ a b) c)"), "(a+bc)");
    }

    #[test]
    fn determinants() {
        assert_eq!(
            mathml("(det (a b) (c d))"),
            "<mrow><mo>|</mo><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr>\
             <mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable><mo>|</mo></mrow>"
        );
        assert_eq!(mathml("(determinant (1 2) (3 4))"), mathml("(det (1 2) (3 4))"));
    }

    // --- table: ported from the old renderer's `table_tests` ---

    const SAMPLE: &str = "(header Name Age City) (row Alice 30 NYC) (row Bob 25 LA)";

    #[test]
    fn grid_is_the_default_style() {
        let out = mathml(&format!("(table {SAMPLE})"));
        assert!(out.starts_with("<mtable"));
        assert!(out.ends_with("</mtable>"));
        assert!(out.contains("frame=\"solid\""));
        assert!(out.contains("rowlines=\"solid\""));
        assert!(out.contains("columnlines=\"solid\""));
        assert_eq!(mathml(&format!("(table (style grid) {SAMPLE})")), out);
    }

    #[test]
    fn header_cells_are_bold_and_rows_are_emitted() {
        let out = mathml(&format!("(table {SAMPLE})"));
        assert!(out.contains("<mstyle mathvariant=\"bold\"><mi>Name</mi></mstyle>"));
        // Three rows total (1 header + 2 data).
        assert_eq!(out.matches("<mtr>").count(), 3);
        // Header has one bold cell per column; data rows have none.
        assert_eq!(out.matches("mathvariant=\"bold\"").count(), 3);
        // Data cell renders without bold wrapper.
        assert!(out.contains("<mtd><mi>Alice</mi></mtd>"));
        assert!(out.contains("<mtd><mn>30</mn></mtd>"));
    }

    #[test]
    fn table_exact_output() {
        assert_eq!(
            mathml("(table (header A) (row x))"),
            "<mtable columnspacing=\"1em\" rowspacing=\"0.5em\" columnalign=\"left\" frame=\"solid\" \
             rowlines=\"solid\" columnlines=\"solid\"><mtr><mtd><mstyle mathvariant=\"bold\"><mi>A</mi></mstyle></mtd></mtr>\
             <mtr><mtd><mi>x</mi></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn dispatches_through_expr() {
        let out = mathml("(table (header A) (row x))");
        assert!(out.contains("<mtable"));
        assert!(out.contains("mathvariant=\"bold\""));
    }

    #[test]
    fn align_directive_sets_columnalign() {
        let out = mathml("(table (align left right c) (row a b c))");
        assert!(out.contains("columnalign=\"left right center\""));
        // The align directive itself is not rendered as a row.
        assert_eq!(out.matches("<mtr>").count(), 1);
        assert!(!out.contains("align</mi>"));
        // Short forms and strings.
        let out = mathml("(table (align r \"centre\" middle bogus) (row a b c d))");
        assert!(out.contains("columnalign=\"right center center left\""));
    }

    #[test]
    fn defaults_to_left_alignment() {
        assert!(mathml(&format!("(table {SAMPLE})")).contains("columnalign=\"left\""));
    }

    #[test]
    fn clean_style_rules_under_header_only() {
        let out = mathml(&format!("(table (style clean) {SAMPLE})"));
        assert!(out.contains("frame=\"solid\""));
        assert!(out.contains("columnlines=\"none\""));
        assert!(out.contains("rowlines=\"solid none\""));
        assert!(!out.contains("columnlines=\"solid\""));
        // Without a header there is no rule at all.
        let out = mathml("(table (style clean) (row a b))");
        assert!(out.contains("frame=\"solid\""));
        assert!(!out.contains("rowlines"));
    }

    #[test]
    fn plain_style_has_no_frame_or_lines() {
        let out = mathml("(table (style plain) (row a b))");
        assert!(!out.contains("frame"));
        assert!(!out.contains("rowlines"));
        assert!(!out.contains("columnlines"));
    }

    #[test]
    fn bare_lists_are_treated_as_rows() {
        let out = mathml("(table (a b) (c d))");
        assert_eq!(out.matches("<mtr>").count(), 2);
        assert!(!out.contains("mathvariant=\"bold\""));
        assert!(out.contains("<mtd><mi>a</mi></mtd>"));
        // A non-list is a one-cell row.
        assert_eq!(mathml("(table x)").matches("<mtr><mtd><mi>x</mi></mtd></mtr>").count(), 1);
    }

    #[test]
    fn cells_may_hold_math_expressions() {
        let src = "(table (header x (funcapp f x)) (row 1 (frac 1 2)))";
        let out = mathml(src);
        assert!(out.contains("<mfrac>"));
        assert!(out.contains("<mstyle mathvariant=\"bold\"><mrow><mi>f</mi>"));
        assert!(visible(src).contains("f(x)"));
    }

    #[test]
    fn directives_and_rows_raise_no_hints() {
        // `header`, `row`, `align`, `style` are close to keywords; none may be
        // reported as a typo'd function application.
        let (_, diags) = render("(table (style clean) (align left) (header a) (row b))");
        assert!(diags.is_empty(), "{diags:?}");
        let (_, diags) = render("(mat (row a) (rows b))");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn table_with_the_notes_header() {
        with_standins(|| {
            let src = "(table (header Logic (jux Type Theory) (jux Category Theory)) (row Proposition Type Object))";
            assert_eq!(visible(src), "LogicType TheoryCategory TheoryPropositionTypeObject");
            assert_eq!(mathml(src).matches("<mtr>").count(), 2);
        });
    }

    #[test]
    fn unknown_style_is_a_warning() {
        let (out, diags) = render("(table (style fancy) (row a))");
        assert_eq!(diags.len(), 1);
        assert!(out.contains("frame=\"solid\""));
    }

    #[test]
    fn table_cells_are_escaped() {
        let out = mathml("(table (header \"a<b\") (row \"x & \\\"y\\\"\"))");
        assert!(out.contains("<mtext>a&lt;b</mtext>"));
        assert!(out.contains("<mtext>x &amp; &quot;y&quot;</mtext>"));
        assert_eq!(visible("(table (row \"1 < 2\"))"), "1 < 2");
    }

    #[test]
    fn ty_judge() {
        assert_eq!(
            mathml("(ty-judge T-Var ((subscript x i) y) (+ x y))"),
            "<mrow><mfrac><mrow><msub><mi>x</mi><mi>i</mi></msub><mspace width=\"2em\"/><mi>y</mi></mrow>\
             <mrow><mrow><mi>x</mi><mo>+</mo><mi>y</mi></mrow></mrow></mfrac><mspace width=\"0.5em\"/>\
             <mtext>T-Var</mtext></mrow>"
        );
        assert_eq!(mathml("(ty-judgement \"T-App\" () x)"), mathml("(ty-judge T-App () x)"));
        // No premises: an empty row above the bar. A string name is escaped.
        assert!(mathml("(ty-judge \"a<b\" () x)").contains("<mtext>a&lt;b</mtext>"));
        // A name that is not a plain word renders as math.
        assert!(mathml("(ty-judge (subscript T 1) () x)").ends_with("<msub><mi>T</mi><mn>1</mn></msub></mrow>"));
        // Premises that are not a list are the single premise.
        assert_eq!(visible("(ty-judge R x y)"), "xy R");
    }

    #[test]
    fn font_variants_on_an_identifier() {
        assert_eq!(mathml("(bb R)"), "<mstyle mathvariant=\"double-struck\"><mi>ℝ</mi></mstyle>");
        assert_eq!(visible("(bb N)"), "ℕ");
        assert_eq!(visible("(bb A)"), "𝔸");
        assert_eq!(visible("(bb x)"), "𝕩");
        assert_eq!(mathml("(cal L)"), "<mstyle mathvariant=\"script\"><mi>ℒ</mi></mstyle>");
        assert_eq!(visible("(cal A)"), "𝒜");
        assert_eq!(mathml("(frak g)"), "<mstyle mathvariant=\"fraktur\"><mi>𝔤</mi></mstyle>");
        assert_eq!(visible("(frak R)"), "ℜ");
        assert_eq!(mathml("(bold v)"), "<mstyle mathvariant=\"bold\"><mi>𝐯</mi></mstyle>");
        assert_eq!(mathml("(rm d)"), "<mstyle mathvariant=\"normal\"><mtext>d</mtext></mstyle>");
        // A number: bold digits, others unchanged.
        assert_eq!(mathml("(bold 1)"), "<mstyle mathvariant=\"bold\"><mn>𝟏</mn></mstyle>");
        assert_eq!(mathml("(bb 1)"), "<mstyle mathvariant=\"double-struck\"><mn>𝟙</mn></mstyle>");
        assert_eq!(mathml("(cal 1)"), "<mstyle mathvariant=\"script\"><mn>1</mn></mstyle>");
        // A name with a display form keeps it.
        assert_eq!(mathml("(bold alpha)"), "<mstyle mathvariant=\"bold\"><mi>α</mi></mstyle>");
    }

    #[test]
    fn font_variants_on_a_compound() {
        assert_eq!(
            mathml("(bb (+ a b))"),
            "<mstyle mathvariant=\"double-struck\"><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow></mstyle>"
        );
        assert_eq!(
            mathml("(cal (subscript F n))"),
            "<mstyle mathvariant=\"script\"><msub><mi>F</mi><mi>n</mi></msub></mstyle>"
        );
        assert_eq!(
            mathml("(rm (frac a b))"),
            "<mstyle mathvariant=\"normal\"><mfrac><mi>a</mi><mi>b</mi></mfrac></mstyle>"
        );
        assert_eq!(
            mathml("(bold (frak x))"),
            "<mstyle mathvariant=\"bold\"><mstyle mathvariant=\"fraktur\"><mi>𝔵</mi></mstyle></mstyle>"
        );
        // A keyword name is just a name here.
        assert_eq!(visible("(bold sum)"), "𝐬𝐮𝐦");
    }

    #[test]
    fn cases_with_and_without_conditions() {
        assert_eq!(
            mathml("(cases (x (text if positive)) (0 (text otherwise)))"),
            "<mrow><mo>{</mo><mtable columnalign=\"left left\" columnspacing=\"1em\" rowspacing=\"0.4em\">\
             <mtr><mtd><mi>x</mi></mtd><mtd><mtext>if positive</mtext></mtd></mtr>\
             <mtr><mtd><mn>0</mn></mtd><mtd><mtext>otherwise</mtext></mtd></mtr></mtable></mrow>"
        );
        // A clause with one item has no condition; a non-list is a value.
        assert_eq!(
            mathml("(cases (a) b)"),
            "<mrow><mo>{</mo><mtable columnalign=\"left left\" columnspacing=\"1em\" rowspacing=\"0.4em\">\
             <mtr><mtd><mi>a</mi></mtd></mtr><mtr><mtd><mi>b</mi></mtd></mtr></mtable></mrow>"
        );
        assert_eq!(visible("(cases ((frac a b) (+ x 1)))"), "{abx+1");
    }

    #[test]
    fn cases_extra_items_are_a_warning() {
        let (out, diags) = render("(cases (a b c))");
        assert_eq!(diags.len(), 1);
        assert!(!out.contains("<mi>c</mi>"));
        // No hints for the clause heads.
        let (_, diags) = render("(cases (row a) (style b))");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn precedence_levels() {
        assert_eq!(visible("(subscript (prime f) 1)"), "(f′)1");
        assert_eq!(visible("(subscript (superscript x 2) i)"), "(x2)i");
        // Atoms need no parentheses as a base.
        assert_eq!(visible("(superscript (bar x) 2)"), "x¯2");
        assert_eq!(visible("(superscript (funcapp f x) 2)"), "f(x)2");
        assert_eq!(visible("(superscript (mat (a b)) 2)"), "(ab)2");
    }
}
