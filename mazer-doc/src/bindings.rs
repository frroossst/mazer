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

use std::collections::HashMap;

use mazer_syntax::{Node, Span, Sym};

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
    let _ = (node, env);
    todo!("step 2, document agent")
}
