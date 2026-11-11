//! A keyword call with its arguments already matched to parameters.

use mazer_syntax::{Diags, Node, Span};

use crate::keyword::Keyword;

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
    let _ = (keyword, list, diags);
    todo!("step 1, math core agent")
}
