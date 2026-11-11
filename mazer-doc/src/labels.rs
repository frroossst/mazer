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
//!
//! # Details
//!
//! - Tags and references are read from the tree as written: neither the tag
//!   name nor the reference name is ever macro-expanded or substituted.
//! - A `(tag ...)` anywhere other than the first item of a display `show`
//!   (inside an expression, in a `let`, an `eval`, a definition) is
//!   `MisplacedTag`; so is a tag on an inline `show`.
//! - The anchor is `eq-` followed by the name as written; it is escaped when
//!   written into HTML.
//! - A duplicate tag gets no number; the first one keeps it.

use std::collections::HashMap;

use mazer_syntax::{Diag, DiagCode, Diags, Document, Node, Span, Sym};

use crate::walk::{self, Place};

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
///
/// Also checks the shape of each `(tag name)` and `(ref name)`: both take
/// exactly one name (`BadDefinition` for a malformed tag, `Arity` /
/// `UnresolvedRef` for a malformed reference). Forms holding a reader error
/// are skipped; the reader already reported them.
pub fn collect_labels(doc: &Document, diags: &mut Diags) -> Labels {
    let mut labels = Labels::new();
    let mut refs: Vec<&Node> = Vec::new();
    walk::for_each_code(doc, &mut |form, place| {
        if form.is_form("ref") {
            refs.push(form);
            return;
        }
        let args = form.args();
        let tagged = form.is_form("show") && args.first().is_some_and(|a| a.is_form("tag"));
        if tagged {
            let tag = &args[0];
            if place == Place::Block {
                define(tag, &mut labels, diags);
            } else {
                diags.push(
                    Diag::error(DiagCode::MisplacedTag, tag.span, "only a display equation can be tagged")
                        .with_help("put the `(show (tag ...) ...)` in a paragraph of its own"),
                );
            }
            misplaced(tag.args(), diags);
            misplaced(&args[1..], diags);
        } else {
            misplaced(args, diags);
        }
    });
    for form in refs {
        resolve(form, &labels, diags);
    }
    labels
}

/// Registers one `(tag name)` of a display show.
fn define(tag: &Node, labels: &mut Labels, diags: &mut Diags) {
    if walk::contains_error(tag) {
        return;
    }
    let Some(name) = tag_name(tag) else {
        diags.push(
            Diag::error(DiagCode::BadDefinition, tag.span, "a tag needs exactly one name")
                .with_help("write it as (tag name)"),
        );
        return;
    };
    if let Some(first) = labels.get(name) {
        diags.push(
            Diag::error(DiagCode::DuplicateTag, tag.span, format!("the tag `{name}` is already used"))
                .with_label(first.span, "first used here"),
        );
        return;
    }
    let number = u32::try_from(labels.len() + 1).unwrap_or(u32::MAX);
    labels.insert(Label {
        name,
        number,
        anchor: anchor(name).into_boxed_str(),
        span: tag.span,
    });
}

/// The HTML id of the equation tagged `name`.
pub(crate) fn anchor(name: Sym) -> String {
    format!("eq-{name}")
}

/// The name in `(tag name)` / `(ref name)`, if the form is well shaped.
pub(crate) fn tag_name(form: &Node) -> Option<Sym> {
    match form.args() {
        [name] => name.as_ident(),
        _ => None,
    }
}

/// Reports every `(tag ...)` anywhere inside `nodes`.
fn misplaced(nodes: &[Node], diags: &mut Diags) {
    for node in nodes {
        if node.is_form("tag") {
            diags.push(walk::misplaced_tag(node.span));
        }
        if let Some(items) = node.as_list() {
            misplaced(items, diags);
        }
    }
}

/// Checks one `(ref name)`.
fn resolve(form: &Node, labels: &Labels, diags: &mut Diags) {
    if walk::contains_error(form) {
        return;
    }
    match form.args() {
        [name] => match name.as_ident() {
            Some(sym) if labels.get(sym).is_some() => {}
            Some(sym) => {
                let help = closest(sym, labels).map_or_else(
                    || format!("tag one with (show (tag {sym}) ...)"),
                    |near| format!("did you mean `{}`? (equation ({}))", near.name, near.number),
                );
                diags.push(
                    Diag::error(DiagCode::UnresolvedRef, name.span, format!("no equation is tagged `{sym}`"))
                        .with_help(help),
                );
            }
            None => diags.push(Diag::error(
                DiagCode::UnresolvedRef,
                name.span,
                "a reference needs the name of a tag",
            )),
        },
        _ => diags.push(
            Diag::error(DiagCode::Arity, form.span, "a reference takes exactly one tag name")
                .with_help("write it as (ref name)"),
        ),
    }
}

/// The label nearest to `name` by edit distance, if one is close enough to
/// be a typo: at most one edit for names of up to four characters, two for
/// longer ones. Ties go to the lower equation number.
fn closest(name: Sym, labels: &Labels) -> Option<&Label> {
    let name = name.as_str();
    let limit = if name.chars().count() <= 4 { 1 } else { 2 };
    labels
        .iter()
        .map(|label| (edit_distance(name, label.name.as_str()), label))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, label)| (*d, label.number))
        .map(|(_, label)| label)
}

/// Edit distance counting characters: insert, delete, substitute, and
/// swapping two neighbours, one edit each.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Three rows of the dynamic-programming table.
    let mut before: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut row: Vec<usize> = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        row[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (prev[j] + 1).min(row[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(before[j - 2] + 1);
            }
            row[j] = best;
        }
        std::mem::swap(&mut before, &mut prev);
        std::mem::swap(&mut prev, &mut row);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests;
