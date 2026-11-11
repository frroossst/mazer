//! The socket for a future evaluation crate. Nothing here evaluates anything.
//!
//! Rules:
//! - `(show ...)` never evaluates. Only `(eval ...)` does.
//! - `(eval ...)` is standalone only: a block or an inline of its own. Nested
//!   inside `show` (or anything else) it is a `NestedEval` diagnostic.
//! - Evaluation runs during `render`, never during `analyze`, in document
//!   order. The evaluator keeps its own state across calls.
//! - The evaluator receives trees that are already macro-expanded and have
//!   `let` variables substituted, so it only sees keywords and free variables.
//! - With no evaluator configured, `(eval ...)` is a `NoEvaluator` diagnostic
//!   and an inline error box.

use mazer_syntax::{Diags, Node};

pub trait Evaluator {
    /// Evaluates one expression. The result is another [`Node`] (a number, a
    /// fraction, a whole symbolic expression ...), which is then rendered by
    /// the normal keyword emitters. On failure, push an `EvalFailed`
    /// diagnostic and return `None`.
    fn eval(&mut self, expr: &Node, diags: &mut Diags) -> Option<Node>;
}
