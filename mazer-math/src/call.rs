//! A keyword call with its arguments already matched to parameters.

use mazer_syntax::{Diag, DiagCode, Diags, Node, Span};

use crate::keyword::{Keyword, ParamKind};

/// What an emitter receives. Arguments are matched to the keyword's declared
/// parameters before the emitter runs, so an emitter can rely on its required
/// arguments being present.
#[derive(Debug)]
pub struct Call<'a> {
    pub keyword: &'a Keyword,
    /// The whole `(keyword ...)` list.
    pub span: Span,
    /// One slot per declared `Required` / `Optional` parameter, in order.
    pub(crate) bound: Vec<Option<&'a Node>>,
    /// Arguments taken by a `Rest` parameter (empty if there is none).
    pub(crate) rest: &'a [Node],
}

impl<'a> Call<'a> {
    /// The argument for the parameter at position `index` of the keyword's
    /// parameter list, which must be a `Required` parameter.
    ///
    /// # Panics
    /// Panics if `index` is not a required parameter of this keyword. That is a
    /// bug in the keyword definition, caught by the registry test that renders
    /// every example.
    #[must_use]
    pub fn req(&self, index: usize) -> &'a Node {
        self.bound
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("`{}`: parameter {index} is not a bound required parameter", self.keyword.name))
    }

    /// The argument for the `Optional` parameter at position `index`, if given.
    #[must_use]
    pub fn opt(&self, index: usize) -> Option<&'a Node> {
        self.bound.get(index).copied().flatten()
    }

    /// The arguments collected by the `Rest` parameter.
    #[must_use]
    pub const fn rest(&self) -> &'a [Node] {
        self.rest
    }
}

/// Matches the arguments of `list` (a list headed by `keyword`) to the
/// keyword's parameters.
///
/// Required parameters are filled first, in order; extra arguments fill the
/// optional parameters left to right; anything beyond goes to the `Rest`
/// parameter. Too few or too many arguments pushes an `Arity` diagnostic
/// (with the keyword's signature as help) and returns `None`, in which case
/// the emitter must not be called.
pub fn bind<'a>(keyword: &'a Keyword, list: &'a Node, diags: &mut Diags) -> Option<Call<'a>> {
    match try_bind(keyword, list) {
        Ok(call) => Some(call),
        Err(diag) => {
            diags.push(diag);
            None
        }
    }
}

/// [`bind`] without the sink: the `Arity` diagnostic is returned instead of
/// pushed, so `MathMl::expr` can report it through `MathMl::error`.
pub(crate) fn try_bind<'a>(keyword: &'a Keyword, list: &'a Node) -> Result<Call<'a>, Diag> {
    let args = list.args();
    let min = keyword.min_args();
    let max = keyword.max_args();

    if args.len() < min {
        // Required parameters come first, so the first missing one is the
        // required parameter at position `args.len()`.
        let missing = keyword
            .params
            .iter()
            .filter(|p| p.kind == ParamKind::Required)
            .nth(args.len())
            .map_or("an", |p| p.name);
        let message = if min - args.len() == 1 {
            format!("`{}` is missing its `{missing}` argument", keyword.name)
        } else {
            format!(
                "`{}` is missing {} arguments, starting with `{missing}`",
                keyword.name,
                min - args.len()
            )
        };
        return Err(Diag::error(DiagCode::Arity, list.span, message).with_help(keyword.signature()));
    }

    if let Some(max) = max
        && args.len() > max
    {
        let message = format!(
            "`{}` takes {} but was given {}",
            keyword.name,
            plural_args(max, min),
            args.len()
        );
        let mut diag = Diag::error(DiagCode::Arity, list.span, message).with_help(keyword.signature());
        for extra in &args[max..] {
            diag = diag.with_label(extra.span, "unexpected argument");
        }
        return Err(diag);
    }

    // Required parameters take the first `min` arguments; optional ones take
    // what follows, left to right; the rest parameter takes the remainder.
    let mut spare = args.len() - min;
    let mut next = 0;
    let mut bound = Vec::with_capacity(keyword.params.len());
    for param in &keyword.params {
        let take = match param.kind {
            ParamKind::Required => true,
            ParamKind::Optional if spare > 0 => {
                spare -= 1;
                true
            }
            ParamKind::Optional => false,
            ParamKind::Rest => break,
        };
        if take {
            bound.push(Some(&args[next]));
            next += 1;
        } else {
            bound.push(None);
        }
    }

    Ok(Call {
        keyword,
        span: list.span,
        bound,
        rest: &args[next..],
    })
}

/// "2 arguments", "1 argument", "1 to 4 arguments", "at most 0 arguments".
fn plural_args(max: usize, min: usize) -> String {
    let noun = if max == 1 { "argument" } else { "arguments" };
    if min == max {
        format!("{max} {noun}")
    } else if min == 0 {
        format!("at most {max} {noun}")
    } else {
        format!("{min} to {max} {noun}")
    }
}

#[cfg(test)]
mod tests {
    use mazer_syntax::{DiagCode, Diags, Node, Severity};

    use super::bind;
    use crate::call::Call;
    use crate::keyword::Keyword;
    use crate::mathml::MathMl;
    use crate::testing::read;

    fn noop(_: &Call<'_>, _: &mut MathMl<'_>) {}

    /// `(k a b [c] [d] e...)`
    fn full() -> Keyword {
        Keyword::new("k", "Test keyword.", noop)
            .param("a", "First.")
            .param("b", "Second.")
            .optional("c", "Third.")
            .optional("d", "Fourth.")
            .rest("e", "The rest.")
    }

    /// `(frac num den)`
    fn two() -> Keyword {
        Keyword::new("frac", "Test keyword.", noop)
            .param("num", "Top.")
            .param("den", "Bottom.")
    }

    /// The bound slots and the rest arguments as source text.
    fn shape(call: &Call<'_>) -> (Vec<Option<String>>, Vec<String>) {
        (
            call.bound.iter().map(|n| n.map(Node::to_string)).collect(),
            call.rest().iter().map(Node::to_string).collect(),
        )
    }

    /// `""` stands for an empty slot.
    fn slots(names: &[&str]) -> Vec<Option<String>> {
        names
            .iter()
            .map(|n| (!n.is_empty()).then(|| (*n).to_owned()))
            .collect()
    }

    #[test]
    fn required_then_optional_then_rest() {
        let kw = full();
        let mut diags = Diags::new();

        let list = read("(k x y)");
        let call = bind(&kw, &list, &mut diags).unwrap();
        assert_eq!(shape(&call), (slots(&["x", "y", "", ""]), vec![]));
        assert_eq!(call.req(1).to_string(), "y");
        assert!(call.opt(2).is_none());
        assert_eq!(call.span, list.span);

        let list = read("(k x y z)");
        let call = bind(&kw, &list, &mut diags).unwrap();
        assert_eq!(shape(&call), (slots(&["x", "y", "z", ""]), vec![]));
        assert_eq!(call.opt(2).map(Node::to_string).as_deref(), Some("z"));

        let list = read("(k x y z w)");
        let call = bind(&kw, &list, &mut diags).unwrap();
        assert_eq!(shape(&call), (slots(&["x", "y", "z", "w"]), vec![]));

        let list = read("(k x y z w 1 2)");
        let call = bind(&kw, &list, &mut diags).unwrap();
        assert_eq!(
            shape(&call),
            (slots(&["x", "y", "z", "w"]), vec!["1".to_owned(), "2".to_owned()])
        );

        assert!(diags.is_empty());
    }

    #[test]
    fn rest_only() {
        let kw = Keyword::new("+", "Test keyword.", noop).rest("terms", "Terms.");
        let mut diags = Diags::new();
        for (src, n) in [("(+)", 0), ("(+ a)", 1), ("(+ a b c)", 3)] {
            let list = read(src);
            let call = bind(&kw, &list, &mut diags).unwrap();
            assert_eq!(call.bound, []);
            assert_eq!(call.rest().len(), n);
        }
        assert!(diags.is_empty());
    }

    #[test]
    fn too_few() {
        let kw = two();
        let mut diags = Diags::new();
        let list = read("(frac 1)");
        assert!(bind(&kw, &list, &mut diags).is_none());
        let diag = &diags.as_slice()[0];
        assert_eq!(diag.code, DiagCode::Arity);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.span, list.span);
        assert_eq!(diag.message, "`frac` is missing its `den` argument");
        assert_eq!(diag.help.as_deref(), Some("(frac num den)"));

        let mut diags = Diags::new();
        let list = read("(frac)");
        assert!(bind(&kw, &list, &mut diags).is_none());
        assert_eq!(diags.as_slice()[0].message, "`frac` is missing 2 arguments, starting with `num`");

        // Optional parameters never count as missing.
        let mut diags = Diags::new();
        let kw = full();
        let list = read("(k x)");
        assert!(bind(&kw, &list, &mut diags).is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.as_slice()[0].message, "`k` is missing its `b` argument");
        assert_eq!(diags.as_slice()[0].help.as_deref(), Some("(k a b [c] [d] e...)"));
    }

    #[test]
    fn too_many() {
        let mut diags = Diags::new();
        let kw = two();
        let list = read("(frac 1 2 3 4)");
        assert!(bind(&kw, &list, &mut diags).is_none());
        let diag = &diags.as_slice()[0];
        assert_eq!(diag.code, DiagCode::Arity);
        assert_eq!(diag.span, list.span);
        assert_eq!(diag.message, "`frac` takes 2 arguments but was given 4");
        assert_eq!(diag.help.as_deref(), Some("(frac num den)"));
        // Each extra argument is labelled.
        let extra: Vec<_> = diag.labels.iter().map(|(span, _)| *span).collect();
        assert_eq!(extra, [list.args()[2].span, list.args()[3].span]);

        let cases = [
            (Keyword::new("k", "T.", noop).param("a", "A.").optional("b", "B."), "(k 1 2 3)", "`k` takes 1 to 2 arguments but was given 3"),
            (Keyword::new("k", "T.", noop).optional("a", "A."), "(k 1 2)", "`k` takes at most 1 argument but was given 2"),
            (Keyword::new("k", "T.", noop).param("a", "A."), "(k 1 2)", "`k` takes 1 argument but was given 2"),
            (Keyword::new("k", "T.", noop), "(k 1)", "`k` takes 0 arguments but was given 1"),
        ];
        for (kw, src, message) in cases {
            let mut diags = Diags::new();
            let list = read(src);
            assert!(bind(&kw, &list, &mut diags).is_none());
            assert_eq!(diags.as_slice()[0].message, message);
        }
    }
}
