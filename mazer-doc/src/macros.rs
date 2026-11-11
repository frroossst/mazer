//! Notation: `alias` and `macro`.
//!
//! ```lisp
//! (alias R (bb R))                       ; an alias is a macro with no parameters
//! (macro (laplace f s)
//!   "Laplace transform of f, evaluated at s"
//!   (jux Laplace (brace f) (paren s)))
//! ```
//!
//! Expansion is pure, lazy tree substitution. Nothing is evaluated.
//!
//! Rules:
//! - `alias` and `macro` are block-level reserved heads. A malformed
//!   definition is a `BadDefinition` diagnostic.
//! - Scope is the whole note. Definitions from the embedded `stdlib.zr`, then
//!   from the note, are all collected before anything is expanded, so order
//!   does not matter. Defining a name twice is `DuplicateDefinition`.
//! - A macro or alias may not reuse the name of a built-in keyword or display
//!   name (`ShadowsBuiltin`).
//! - Each parameter name in the template is replaced by the argument exactly as
//!   written. Wrong argument count is `MacroArity`.
//! - The result is expanded again, so macros can use macros. A name is not
//!   re-expanded inside its own expansion (so `(alias R (bb R))` is fine).
//!   Nesting deeper than [`MAX_DEPTH`] is `MacroDepth`.
//! - Expanded nodes take the span of the place the macro was *used*; the
//!   definition span is attached to diagnostics as a secondary label.
//! - `(quote ...)` contents are still expanded; quoting only stops `let`
//!   substitution, not notation.

use std::collections::HashMap;

use mazer_syntax::{Diags, Document, Node, Span, Sym};

/// Maximum macro expansion depth before giving up with a diagnostic.
pub const MAX_DEPTH: usize = 64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Macro {
    pub name: Sym,
    /// Empty for an alias.
    pub params: Vec<Sym>,
    /// A final parameter that collects the remaining arguments, if any.
    pub rest: Option<Sym>,
    /// The optional string after the signature. Shown on LSP hover.
    pub doc: Option<Box<str>>,
    /// The template.
    pub body: Node,
    /// Where it was defined, for go-to-definition.
    pub span: Span,
}

impl Macro {
    /// Whether this is an alias (a bare name) rather than a macro that is
    /// called with arguments.
    #[must_use]
    pub const fn is_alias(&self) -> bool {
        self.params.is_empty() && self.rest.is_none()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Macros {
    by_name: HashMap<Sym, Macro>,
}

impl Macros {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: Sym) -> Option<&Macro> {
        self.by_name.get(&name)
    }

    /// Inserts a definition, returning the previous one of the same name.
    pub fn insert(&mut self, def: Macro) -> Option<Macro> {
        self.by_name.insert(def.name, def)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Macro> {
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

/// Collects every `alias` and `macro` definition in `doc` into `macros`,
/// reporting malformed, duplicate and shadowing definitions.
pub fn collect_macros(doc: &Document, macros: &mut Macros, diags: &mut Diags) {
    let _ = (doc, macros, diags);
    todo!("step 2, document agent")
}

/// Expands every macro and alias use in one code tree.
pub fn expand(node: &Node, macros: &Macros, diags: &mut Diags) -> Node {
    let _ = (node, macros, diags);
    todo!("step 2, document agent")
}
