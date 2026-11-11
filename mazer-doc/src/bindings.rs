//! Variables: `let` and `quote`.
//!
//! `let` is a real variable binding and is different from `alias`.
//!
//! ```lisp
//! (let x 2)
//! (show x)          ; 2
//! (show (+ x 1))    ; 2 + 1      show substitutes bound variables everywhere
//! (eval (+ x 1))    ; 3
//! (let y (+ x 1))   ; y holds (+ 2 1): captured at the let
//! (let x 3)         ; reassignable any number of times; document order
//! (show (= 'x x))   ; x = 3      quote shows the name itself
//! ```
//!
//! Rules:
//! - Values are stored unevaluated. Only `(eval ...)` computes.
//! - A `let` takes effect from where it is written; a later `let` for the same
//!   name replaces it for everything below.
//! - Capture at the `let`: when a `let` is processed, variables already bound
//!   are substituted into its value. So `(let x (+ x 1))` works and a later
//!   reassignment never changes an earlier definition.
//! - One pass over the note in document order, after macro expansion,
//!   substitutes variables into every `let`, `show` and `eval`. Afterwards the
//!   trees contain no bound variables.
//! - `(quote x)` / `'x` is never substituted.
//! - Substituted nodes keep the span of the place the variable was *used*.
//!
//! # Details
//!
//! - **Syntax.** `(let name expr)`: exactly one name and one expression,
//!   otherwise `BadDefinition`. A `let` may be written as a block or inside a
//!   sentence, list item or quote; either way it produces no output.
//! - **Names.** A `let` may not bind a keyword (or one of its second names), a
//!   reserved head or `tag` (`ShadowsBuiltin`). Binding a display name such as
//!   `tau` is allowed: it is an ordinary variable that happens to render as τ.
//!   Binding the name of an `alias` or `macro` is a `DuplicateDefinition`,
//!   since macros are expanded first and the variable could never be used.
//! - **Head position.** The head of a list is substituted like any other
//!   position, so after `(let f g)`, `(show (f x))` shows g(x). A bound name
//!   can never be a keyword, so a keyword head is never replaced.
//! - **Spans.** Every node of a substituted value takes the span of the use.
//! - **Quote.** The whole of a `(quote ...)` list, including its head, is left
//!   as written.
//! - **Size.** A substitution that would produce more than 100 000 nodes or
//!   nest more than 320 levels deep (`(let x (+ x x))` forty times over) is
//!   refused with a diagnostic: the `let` is not bound, or the shown expression
//!   becomes an error box.

use std::collections::HashMap;

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Span, Sym};

use crate::macros::Macros;
use crate::walk::{self, Measure};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Binding {
    pub name: Sym,
    /// The bound expression, unevaluated, with earlier variables already
    /// substituted in.
    pub value: Node,
    /// Where the `let` was written.
    pub span: Span,
}

/// The variables in scope at one point of the note.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Bindings {
    by_name: HashMap<Sym, Binding>,
}

impl Bindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: Sym) -> Option<&Binding> {
        self.by_name.get(&name)
    }

    /// Binds or rebinds a variable, returning the previous binding if any.
    pub fn insert(&mut self, binding: Binding) -> Option<Binding> {
        self.by_name.insert(binding.name, binding)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.by_name.values()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Replaces every bound variable in `node` with its value, leaving
/// `(quote ...)` contents untouched.
#[must_use]
pub fn substitute(node: &Node, env: &Bindings) -> Node {
    if env.is_empty() {
        return node.clone();
    }
    match &node.kind {
        NodeKind::Ident(sym) => env
            .get(*sym)
            .map_or_else(|| node.clone(), |binding| walk::respan(&binding.value, node.span)),
        NodeKind::List(_) if node.is_form("quote") => node.clone(),
        NodeKind::List(items) => Node::list(items.iter().map(|c| substitute(c, env)).collect(), node.span),
        NodeKind::Number(_) | NodeKind::Str(_) | NodeKind::Error => node.clone(),
    }
}

/// The `let` pass: the variables bound so far, in document order.
#[derive(Default)]
pub(crate) struct LetPass {
    env: Bindings,
    /// The size of each bound value, so a substitution can be measured before
    /// it is built.
    sizes: HashMap<Sym, Measure>,
    /// Every successful `let`, in document order.
    pub lets: Vec<Binding>,
}

impl LetPass {
    /// Substitutes the variables bound so far into `node`, unless the result
    /// would be too large; then reports it and returns an `Error` node.
    pub fn substitute(&self, node: &Node, diags: &mut Diags) -> Node {
        if self.measure(node).too_big() {
            diags.push(Diag::error(
                DiagCode::Other,
                node.span,
                format!(
                    "this expression is too large after substituting `let` variables (more than {} nodes or {} levels)",
                    walk::MAX_NODES,
                    walk::MAX_TREE_DEPTH
                ),
            ));
            return Node::error(node.span);
        }
        substitute(node, &self.env)
    }

    /// What [`substitute`] would produce, measured without building it.
    fn measure(&self, node: &Node) -> Measure {
        match &node.kind {
            NodeKind::Ident(sym) => self.sizes.get(sym).copied().unwrap_or(Measure::LEAF),
            NodeKind::List(_) if node.is_form("quote") => Measure::of(node),
            NodeKind::List(items) => Measure::list(items.iter().map(|c| self.measure(c))),
            NodeKind::Number(_) | NodeKind::Str(_) | NodeKind::Error => Measure::LEAF,
        }
    }

    /// Processes one `(let name expr)` whose value is already macro-expanded:
    /// checks it, substitutes the variables bound so far into the value and
    /// binds it. Returns the form with the substituted value.
    pub fn bind(&mut self, form: &Node, macros: &Macros, diags: &mut Diags) -> Node {
        let Some((name, value)) = let_parts(form, macros, diags) else {
            return form.clone();
        };
        let measure = self.measure(value);
        if measure.too_big() {
            diags.push(Diag::error(
                DiagCode::BadDefinition,
                value.span,
                format!(
                    "this value is too large after substituting `let` variables (more than {} nodes or {} levels)",
                    walk::MAX_NODES,
                    walk::MAX_TREE_DEPTH
                ),
            ));
            return form.clone();
        }
        let value = substitute(value, &self.env);
        let binding = Binding {
            name,
            value: value.clone(),
            span: form.span,
        };
        self.sizes.insert(name, measure);
        self.env.insert(binding.clone());
        self.lets.push(binding);
        let head = form.as_list().and_then(<[Node]>::first).cloned();
        let name_node = form.args().first().cloned();
        Node::list(head.into_iter().chain(name_node).chain([value]).collect(), form.span)
    }
}

/// The name and value of a well-formed `let`, reporting what is wrong otherwise.
fn let_parts<'a>(form: &'a Node, macros: &Macros, diags: &mut Diags) -> Option<(Sym, &'a Node)> {
    if walk::contains_error(form) {
        return None; // the reader already reported it
    }
    let [name, value] = form.args() else {
        diags.push(
            Diag::error(DiagCode::BadDefinition, form.span, "`let` needs a name and one expression")
                .with_help("write it as (let name expr)"),
        );
        return None;
    };
    let Some(sym) = name.as_ident() else {
        diags.push(
            Diag::error(DiagCode::BadDefinition, name.span, "expected the name of the variable")
                .with_help("write it as (let name expr)"),
        );
        return None;
    };
    if let Some(kind) = walk::builtin_kind(sym, false) {
        diags.push(
            Diag::error(DiagCode::ShadowsBuiltin, name.span, format!("`{sym}` is {kind}"))
                .with_help("choose another name for this variable"),
        );
        return None;
    }
    if let Some(def) = macros.get(sym) {
        diags.push(
            Diag::error(
                DiagCode::DuplicateDefinition,
                name.span,
                format!("`{sym}` is already defined as notation, so this variable could never be used"),
            )
            .with_label(def.span, "defined here"),
        );
        return None;
    }
    Some((sym, value))
}

#[cfg(test)]
mod tests;
