//! Display names: variables that are shown as a special character.
//!
//! `tau` is an ordinary variable. This table only says that when it is
//! *displayed* it is drawn as `τ` (or `\tau` in LaTeX). It is renderer-only
//! data; an evaluator never reads it.

use std::collections::HashMap;
use std::sync::LazyLock;

use mazer_syntax::Sym;

/// Which `MathML` element wraps the character, which decides its spacing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// `<mi>`: sits tight like a variable (`τ`, `Γ`, `∞`, `∅`).
    Letter,
    /// `<mo>`: spaced like an operator (`→`, `⊢`, `∘`, `↦`).
    Operator,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DisplayName {
    /// What is typed, e.g. `tau`.
    pub name: &'static str,
    /// What `MathML` shows, e.g. `τ`.
    pub unicode: &'static str,
    /// Mandatory. Shown in LSP completion and hover.
    pub doc: &'static str,
    pub class: Class,
    /// The LaTeX spelling, e.g. `\tau`. Empty if none has been given yet.
    #[cfg(feature = "latex")]
    pub latex: &'static str,
}

impl DisplayName {
    #[must_use]
    pub const fn letter(name: &'static str, unicode: &'static str, doc: &'static str) -> Self {
        Self::new(name, unicode, doc, Class::Letter)
    }

    #[must_use]
    pub const fn operator(name: &'static str, unicode: &'static str, doc: &'static str) -> Self {
        Self::new(name, unicode, doc, Class::Operator)
    }

    const fn new(name: &'static str, unicode: &'static str, doc: &'static str, class: Class) -> Self {
        Self {
            name,
            unicode,
            doc,
            class,
            #[cfg(feature = "latex")]
            latex: "",
        }
    }

    /// Sets the LaTeX spelling. Always callable so table rows need no `#[cfg]`;
    /// without the `latex` feature the value is simply dropped.
    #[must_use]
    #[cfg_attr(not(feature = "latex"), allow(unused_mut, unused_variables, clippy::missing_const_for_fn))]
    pub const fn latex(mut self, latex: &'static str) -> Self {
        #[cfg(feature = "latex")]
        {
            self.latex = latex;
        }
        self
    }
}

/// The rows of the table. The built-in set must be comprehensive: Greek
/// (both cases), common constants (`infinity`, `hbar`, `emptyset`, ...),
/// arrows, set and logic symbols, and the type-theory symbols the notes use
/// (`turnstile`, `mapsto`, `circ`, ...).
#[allow(clippy::missing_const_for_fn)] // skeleton placeholder: becomes non-const when the table is filled in
fn table() -> Vec<DisplayName> {
    // step 1, display names agent
    Vec::new()
}

static TABLE: LazyLock<Vec<DisplayName>> = LazyLock::new(table);

static BY_NAME: LazyLock<HashMap<Sym, usize>> = LazyLock::new(|| {
    TABLE
        .iter()
        .enumerate()
        .map(|(i, d)| (Sym::new(d.name), i))
        .collect()
});

/// Every display name, in table order.
#[must_use]
pub fn display_names() -> &'static [DisplayName] {
    &TABLE
}

/// The display form of a variable name, if it has one.
#[must_use]
pub fn display_name(name: Sym) -> Option<&'static DisplayName> {
    BY_NAME.get(&name).map(|&i| &TABLE[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_class() {
        let tau = DisplayName::letter("tau", "τ", "Greek lowercase tau").latex("\\tau");
        assert_eq!(tau.class, Class::Letter);
        let arrow = DisplayName::operator("to", "→", "Right arrow").latex("\\to");
        assert_eq!(arrow.class, Class::Operator);
        #[cfg(feature = "latex")]
        assert_eq!(tau.latex, "\\tau");
    }
}
