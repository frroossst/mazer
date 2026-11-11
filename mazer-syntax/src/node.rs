//! The S-expression tree the reader produces for code such as `(show ...)`.
//!
//! It is syntax only. There are no runtime values here: the tree stays lazy and
//! symbolic from the reader through to the emitters. Whether a name is a
//! keyword or a variable is decided later, by `mazer-math`'s registry.

use std::fmt;

use crate::span::Span;
use crate::sym::Sym;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NodeKind {
    /// Any name: `x`, `tau`, `Laplace`, `integral`, `+`.
    Ident(Sym),
    /// A number exactly as written (`3.14`, `1e-9`). Never parsed in the core.
    Number(Box<str>),
    /// The contents of a `"..."` literal, with escapes already resolved.
    Str(Box<str>),
    /// `(head arg arg ...)`. The head is simply the first child.
    List(Vec<Node>),
    /// Placeholder left where the reader recovered from a mistake. Always
    /// accompanied by a diagnostic.
    Error,
}

impl Node {
    #[must_use]
    pub const fn new(kind: NodeKind, span: Span) -> Self {
        Self { kind, span }
    }

    #[must_use]
    pub fn ident(name: &str, span: Span) -> Self {
        Self::new(NodeKind::Ident(Sym::new(name)), span)
    }

    #[must_use]
    pub fn number(text: &str, span: Span) -> Self {
        Self::new(NodeKind::Number(text.into()), span)
    }

    #[must_use]
    pub fn string(text: &str, span: Span) -> Self {
        Self::new(NodeKind::Str(text.into()), span)
    }

    #[must_use]
    pub const fn list(items: Vec<Self>, span: Span) -> Self {
        Self::new(NodeKind::List(items), span)
    }

    #[must_use]
    pub const fn error(span: Span) -> Self {
        Self::new(NodeKind::Error, span)
    }

    #[must_use]
    pub const fn as_ident(&self) -> Option<Sym> {
        match self.kind {
            NodeKind::Ident(sym) => Some(sym),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match &self.kind {
            NodeKind::List(items) => Some(items),
            _ => None,
        }
    }

    /// For a list whose first child is a name, that name.
    #[must_use]
    pub fn head(&self) -> Option<Sym> {
        self.as_list()?.first()?.as_ident()
    }

    /// For a list, everything after the first child.
    #[must_use]
    pub fn args(&self) -> &[Self] {
        match self.as_list() {
            Some([_, rest @ ..]) => rest,
            _ => &[],
        }
    }

    /// Whether this is a list headed by the name `head`.
    #[must_use]
    pub fn is_form(&self, head: &str) -> bool {
        self.head().is_some_and(|h| h == head)
    }

    /// Structural equality that ignores spans. Meant for tests and for
    /// comparing a tree before and after a pass.
    #[must_use]
    pub fn same_shape(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (NodeKind::List(a), NodeKind::List(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_shape(y))
            }
            (a, b) => a == b,
        }
    }
}

/// Prints the tree back as S-expression text, e.g. `(frac 1 (+ a b))`.
/// `Error` nodes print as `<error>`.
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            NodeKind::Ident(sym) => f.write_str(sym.as_str()),
            NodeKind::Number(text) => f.write_str(text),
            NodeKind::Str(text) => {
                f.write_str("\"")?;
                for ch in text.chars() {
                    match ch {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        _ => write!(f, "{ch}")?,
                    }
                }
                f.write_str("\"")
            }
            NodeKind::List(items) => {
                f.write_str("(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str(")")
            }
            NodeKind::Error => f.write_str("<error>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_accessors() {
        let s = Span::DUMMY;
        let node = Node::list(
            vec![
                Node::ident("frac", s),
                Node::number("1", s),
                Node::list(vec![Node::ident("+", s), Node::ident("a", s), Node::string("b\"c", s)], s),
            ],
            s,
        );
        assert_eq!(node.to_string(), "(frac 1 (+ a \"b\\\"c\"))");
        assert!(node.is_form("frac"));
        assert_eq!(node.args().len(), 2);
        assert!(node.same_shape(&node.clone()));
    }
}
