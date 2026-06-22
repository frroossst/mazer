//! Structured errors for the Lisp pipeline (tokenize → parse → evaluate).
//!
//! These carry human-readable wording plus, where useful, a `help:` line that
//! the CLI renders via `miette`'s fancy reporter. The `Display` text is also
//! what the wasm/HTML path surfaces, so every message must read well on its own.

use miette::Diagnostic;
use thiserror::Error;

/// An error produced while tokenizing, parsing, or evaluating Lisp.
#[derive(Debug, Clone, Error, Diagnostic, PartialEq, Eq)]
pub enum LispError {
    /// A closing paren with no matching opener.
    #[error("unexpected ')'")]
    #[diagnostic(
        code(mazer::lisp::unexpected_close_paren),
        help("remove the extra ')' or add the opening '(' it was meant to close")
    )]
    UnexpectedCloseParen,

    /// Input ended while an expression was still open.
    #[error("unexpected end of input")]
    #[diagnostic(
        code(mazer::lisp::unexpected_eof),
        help("an expression is missing a closing ')'")
    )]
    UnexpectedEof,

    /// The block contained no expressions to evaluate.
    #[error("empty program: nothing to evaluate")]
    #[diagnostic(code(mazer::lisp::empty_program))]
    EmptyProgram,

    /// A numeric literal could not be parsed.
    #[error("'{text}' is not a valid number")]
    #[diagnostic(code(mazer::lisp::bad_number))]
    BadNumber {
        /// The offending text.
        text: String,
    },

    /// A form was called with the wrong number of arguments.
    #[error("{form}: expected {expected} argument(s), got {got}")]
    #[diagnostic(code(mazer::lisp::arity))]
    Arity {
        /// The form or function name, e.g. `define`.
        form: String,
        /// Human description of what was expected, e.g. `2` or `at least 1`.
        expected: String,
        /// How many arguments were actually supplied.
        got: usize,
    },

    /// A value of the wrong type was supplied to a form.
    #[error("{form}: expected {expected}, got {got}")]
    #[diagnostic(code(mazer::lisp::type_mismatch))]
    TypeMismatch {
        /// The form or function name, e.g. `add`.
        form: String,
        /// The expected type, e.g. `Number`.
        expected: String,
        /// The type that was actually supplied, e.g. `String`.
        got: String,
    },

    /// A symbol was referenced but never bound.
    #[error("unbound symbol '{name}'")]
    #[diagnostic(
        code(mazer::lisp::unbound_symbol),
        help("define it first, or check the spelling")
    )]
    UnboundSymbol {
        /// The symbol that could not be resolved.
        name: String,
    },

    /// An unbound symbol that closely matches a known name.
    #[error("unbound symbol '{name}' — did you mean '{suggestion}'?")]
    #[diagnostic(code(mazer::lisp::unbound_symbol))]
    UnboundSymbolDidYouMean {
        /// The symbol that could not be resolved.
        name: String,
        /// The closest known name.
        suggestion: String,
    },

    /// A non-callable value appeared in function position.
    #[error("not a function: a value of type {value_type} cannot be called")]
    #[diagnostic(code(mazer::lisp::not_a_function))]
    NotAFunction {
        /// The type of the value that was called.
        value_type: String,
    },

    /// Division by zero.
    #[error("division by zero")]
    #[diagnostic(code(mazer::lisp::division_by_zero))]
    DivisionByZero,

    /// A pre-rendered error message carried through the AST (e.g. a parse failure
    /// stored as a `LispAST::Error` node during document build).
    #[error("{0}")]
    #[diagnostic(code(mazer::lisp::message))]
    Message(String),
}
