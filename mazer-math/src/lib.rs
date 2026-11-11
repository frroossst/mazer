//! One code tree in, `MathML` (or `LaTeX`) out.
//!
//! This crate owns the keyword registry, the display-name table and the
//! emitters. It knows nothing about documents, macros, variables or tags;
//! those belong to `mazer-doc`, which hands this crate fully expanded trees.
//!
//! See `docs/ARCHITECTURE.md`, "Keywords and emitters".

pub mod call;
pub mod display;
pub mod keyword;
pub mod keywords;
mod lint;
pub mod mathml;
pub mod registry;

#[cfg(test)]
mod testing;

#[cfg(feature = "latex")]
pub mod latex;

pub use call::{Call, bind};
pub use display::{Class, DisplayName, display_name, display_names};
pub use keyword::{Keyword, MathMlFn, Param, ParamKind};
pub use mathml::{MathMl, check, to_mathml};
pub use registry::{Registry, registry};

#[cfg(feature = "latex")]
pub use latex::{Latex, LatexFn, to_latex};
