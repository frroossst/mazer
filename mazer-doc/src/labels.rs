//! Equation tags and references.
//!
//! ```lisp
//! (show (tag laplace) (= (laplace f s) (exp (* (- a) s))))     ; numbered display equation
//! As shown in (ref laplace), ...                                 ; renders "(1)" linking to it
//! ```
//!
//! Rules:
//! - Only explicitly tagged equations are numbered.
//! - `(tag name)` is valid only as the first item of a *display* `(show ...)`
//!   (one standing alone as a block). On an inline show it is `MisplacedTag`.
//! - `(ref name)` is an inline reserved head.
//! - Labels are collected in a pass before rendering, so a reference may point
//!   forward. Numbers follow document order, starting at 1.
//! - Unknown reference: `UnresolvedRef`. Same tag twice: `DuplicateTag`.
//! - HTML: the equation gets `id="<anchor>"` and shows its number; a reference
//!   is a link to `#<anchor>` with the text `(n)`.
//! - LaTeX: `\label` / `\eqref`.

use std::collections::HashMap;

use mazer_syntax::{Diags, Document, Span, Sym};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Label {
    pub name: Sym,
    /// 1, 2, 3 ... in document order.
    pub number: u32,
    /// The HTML id references link to, e.g. `eq-laplace`.
    pub anchor: Box<str>,
    /// Where the tag was written, for go-to-definition.
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Labels {
    by_name: HashMap<Sym, Label>,
}

impl Labels {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: Sym) -> Option<&Label> {
        self.by_name.get(&name)
    }

    /// Inserts a label, returning the previous one of the same name.
    pub fn insert(&mut self, label: Label) -> Option<Label> {
        self.by_name.insert(label.name, label)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.by_name.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Finds every tagged display equation, numbers them in document order, and
/// reports duplicate tags, misplaced tags and unresolved references.
pub fn collect_labels(doc: &Document, diags: &mut Diags) -> Labels {
    let _ = (doc, diags);
    todo!("step 2, document agent")
}
