//! The built-in keywords, split into three files so they can be written in
//! parallel. Each file exposes `keywords() -> Vec<Keyword>`.
//!
//! The inventory (names, second names, argument order, what was removed and
//! what is new) is in `docs/ARCHITECTURE.md`, "Keyword inventory".

use crate::keyword::Keyword;

/// Arithmetic, comparison, sets, logic, grouping.
pub mod algebra;
/// Calculus and named functions (trig, logs, abs, floor, ...).
pub mod calculus;
/// Decoration, linear algebra, `table`, `ty-judge`, font variants, `cases`, `quote`.
pub mod notation;

/// Every built-in keyword.
#[must_use]
pub fn all() -> Vec<Keyword> {
    let mut all = algebra::keywords();
    all.extend(calculus::keywords());
    all.extend(notation::keywords());
    all
}
