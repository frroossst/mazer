//! Decoration, linear algebra, `table`, `ty-judge`, font variants, `cases`, `quote`.

use crate::keyword::Keyword;
use crate::mathml::Prec;

#[must_use]
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
    ]
}

/// The keywords of this file that are not atoms; see `keywords/mod.rs`.
#[allow(clippy::missing_const_for_fn, clippy::unnecessary_wraps)] // step 2 adds non-const string matches here
pub(crate) fn precedence(name: &str, args: usize) -> Option<Prec> {
    let _ = (name, args);
    None
}

#[cfg(test)]
mod tests {
    use crate::testing::mathml;

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
}
