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
//! (show (= 'x x))   ; x = 2      quote shows the name itself
//! (let x 3)         ; error: a variable is defined once; x stays 2
//! ```
//!
//! Rules:
//! - Values are stored unevaluated. Only `(eval ...)` computes.
//! - A `let` takes effect from where it is written, for the rest of the note.
//! - A variable is defined once per note. A second `let` of the same name is
//!   a `DuplicateDefinition` and binds nothing, so the first value stays in
//!   effect. A `let` that was rejected (malformed, a reserved name, a value
//!   too large) defines nothing, so a later `let` of that name is fine; one
//!   refused by the node budget does define its variable, as an error.
//! - Capture at the `let`: when a `let` is processed, variables already bound
//!   are substituted into its value. So `(let y (+ x 1))` holds the value `x`
//!   had there, and in `(let x (+ x 1))`, as the first definition of `x`, the
//!   inner `x` is a free name.
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
//!   becomes an error box. Every substituted value is also charged to the
//!   document-wide node budget (see the `walk` module): storing a value is
//!   free, each use costs the value's size. When the budget runs out, the
//!   expression becomes an error box, one diagnostic (`Other`) is reported
//!   for the whole note, and a `let` refused this way makes its variable an
//!   error from then on.
//! - **Problems in a value.** The value of a `let` is checked once, where it
//!   is bound (after substituting earlier variables), and its keyword
//!   problems (`(let y (frac 1))`) are reported there. Each use of the
//!   variable still renders the `<merror>`, but does not report the same
//!   problem again. The `let` itself shows an error box, since it renders
//!   nothing else. A value that is a single name is not checked at the
//!   `let`: `(let f frac)` then `(show (f a b))` is fine.

use std::collections::HashMap;
use std::rc::Rc;

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Span, Sym};

use crate::macros::Macros;
use crate::walk::{self, Budget, Measure};

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

    /// Binds a variable, returning the previous binding of that name if any.
    ///
    /// This is the plain table operation and replaces an existing binding.
    /// In a note a name is bound at most once: the `let` pass rejects a
    /// second definition before it gets here.
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

/// A keyword problem, without its place: what `mazer_math::check` reports
/// for a value, so the same problem can be recognised at each use.
type Problem = (DiagCode, String);

/// The `let` pass: the variables bound so far, in document order.
#[derive(Default)]
pub(crate) struct LetPass {
    env: Bindings,
    /// The size of each bound value, so a substitution can be measured before
    /// it is built.
    sizes: HashMap<Sym, Measure>,
    /// The keyword problems of each bound value, found (and reported) once,
    /// where it was bound.
    problems: HashMap<Sym, Rc<[Problem]>>,
    /// For each place a variable was substituted, the problems its value
    /// carries, so a check of the surrounding expression does not report
    /// them a second time.
    known: HashMap<Span, Vec<Rc<[Problem]>>>,
    /// The span of the name in the `let` that defined each variable, for the
    /// diagnostic on a second definition.
    defined: HashMap<Sym, Span>,
    /// Every successful `let`, in document order: at most one per name.
    pub lets: Vec<Binding>,
}

impl LetPass {
    /// Substitutes the variables bound so far into `node`, unless the result
    /// would be too large or the document budget has run out; then reports
    /// it (the budget only once per document) and returns an `Error` node.
    pub fn substitute(&mut self, node: &Node, budget: &mut Budget, diags: &mut Diags) -> Node {
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
        let cost = self.cost(node);
        if cost > 0 && !budget.try_spend(cost) {
            budget.refuse(DiagCode::Other, node.span, None, diags);
            return Node::error(node.span);
        }
        self.subst(node)
    }

    /// Whether `diag`, found by checking an expression, is a problem of a
    /// substituted value that was already reported where it was bound.
    pub fn already_reported(&self, diag: &Diag) -> bool {
        self.known.get(&diag.span).is_some_and(|lists| {
            lists
                .iter()
                .any(|list| list.iter().any(|(code, message)| *code == diag.code && *message == diag.message))
        })
    }

    /// [`substitute`], recording the problems of every substituted value.
    fn subst(&mut self, node: &Node) -> Node {
        match &node.kind {
            NodeKind::Ident(sym) => {
                let Some(binding) = self.env.get(*sym) else {
                    return node.clone();
                };
                let value = walk::respan(&binding.value, node.span);
                if let Some(problems) = self.problems.get(sym).filter(|p| !p.is_empty()) {
                    self.known.entry(node.span).or_default().push(Rc::clone(problems));
                }
                value
            }
            NodeKind::List(_) if node.is_form("quote") => node.clone(),
            NodeKind::List(items) => Node::list(items.iter().map(|c| self.subst(c)).collect(), node.span),
            NodeKind::Number(_) | NodeKind::Str(_) | NodeKind::Error => node.clone(),
        }
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

    /// How many nodes [`substitute`] would create: the size of every value
    /// it copies in.
    fn cost(&self, node: &Node) -> usize {
        match &node.kind {
            NodeKind::Ident(sym) => self.sizes.get(sym).map_or(0, |m| m.size),
            NodeKind::List(_) if node.is_form("quote") => 0,
            NodeKind::List(items) => items.iter().fold(0, |acc, c| acc.saturating_add(self.cost(c))),
            NodeKind::Number(_) | NodeKind::Str(_) | NodeKind::Error => 0,
        }
    }

    /// Processes one `(let name expr)` whose value is already macro-expanded:
    /// checks it, substitutes the variables bound so far into the value,
    /// reports the value's keyword problems (once, here) and binds it.
    /// Returns the form with the substituted value. A second definition of a
    /// name is reported and binds nothing.
    pub fn bind(&mut self, form: &Node, macros: &Macros, budget: &mut Budget, diags: &mut Diags) -> Node {
        let Some((name, name_span, value)) = let_parts(form, macros, diags) else {
            return form.clone();
        };
        if let Some(&first) = self.defined.get(&name) {
            diags.push(
                Diag::error(DiagCode::DuplicateDefinition, name_span, format!("`{name}` is already defined"))
                    .with_label(first, "first defined here")
                    .with_help("a variable can be defined only once; choose another name"),
            );
            return form.clone();
        }
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
        let cost = self.cost(value);
        let (value, measure) = if cost > 0 && !budget.try_spend(cost) {
            // Out of budget: the variable stands for an error from here on,
            // so its uses are error boxes rather than the bare name.
            budget.refuse(DiagCode::Other, value.span, None, diags);
            (Node::error(value.span), Measure::LEAF)
        } else {
            (self.subst(value), measure)
        };
        let mut found = Diags::new();
        // A bare name is not checked here: `(let f frac)` is fine when `f`
        // is used as a head, and a bare use is reported where it is shown.
        if value.as_ident().is_none() {
            mazer_math::check(&value, &mut found);
        }
        let mut problems = Vec::new();
        for diag in found.into_vec() {
            problems.push((diag.code, diag.message.clone()));
            if !self.already_reported(&diag) {
                diags.push(diag);
            }
        }
        self.problems.insert(name, problems.into());
        let binding = Binding {
            name,
            value: value.clone(),
            span: form.span,
        };
        self.sizes.insert(name, measure);
        // A `let` refused by the node budget is recorded too, with its
        // `Error` value: its variable is an error from here on, and editors
        // asking which binding is in scope must see that.
        self.env.insert(binding.clone());
        self.defined.insert(name, name_span);
        self.lets.push(binding);
        let head = form.as_list().and_then(<[Node]>::first).cloned();
        let name_node = form.args().first().cloned();
        Node::list(head.into_iter().chain(name_node).chain([value]).collect(), form.span)
    }
}

/// The name, the span of the name and the value of a well-formed `let`,
/// reporting what is wrong otherwise.
fn let_parts<'a>(form: &'a Node, macros: &Macros, diags: &mut Diags) -> Option<(Sym, Span, &'a Node)> {
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
    Some((sym, name.span, value))
}

#[cfg(test)]
mod tests;
