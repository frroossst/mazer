//! Diagnostics as plain data. No rendering lives here: the CLI renders with
//! `annotate-snippets`, the LSP converts to LSP diagnostics, HTML output shows
//! an inline error box.

use crate::span::Span;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// What kind of problem a [`Diag`] reports. Stages may add variants; nothing
/// may match on this exhaustively.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum DiagCode {
    // reader
    UnclosedParen,
    UnterminatedString,
    // keywords
    /// A list head one typo away from a keyword (warning with did-you-mean).
    UnknownForm,
    Arity,
    /// A keyword used as a bare word but it has no sign to render.
    BareKeyword,
    /// The requested output target is not implemented for this keyword.
    UnsupportedTarget,
    // alias / macro / let
    BadDefinition,
    DuplicateDefinition,
    ShadowsBuiltin,
    MacroArity,
    MacroDepth,
    // tags and references
    UnresolvedRef,
    DuplicateTag,
    MisplacedTag,
    // evaluation
    NestedEval,
    NoEvaluator,
    EvalFailed,
    // anything else
    Other,
}

impl DiagCode {
    /// A short stable name, shown as `error[name]` in the terminal.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UnclosedParen => "unclosed-paren",
            Self::UnterminatedString => "unterminated-string",
            Self::UnknownForm => "unknown-form",
            Self::Arity => "arity",
            Self::BareKeyword => "bare-keyword",
            Self::UnsupportedTarget => "unsupported-target",
            Self::BadDefinition => "bad-definition",
            Self::DuplicateDefinition => "duplicate-definition",
            Self::ShadowsBuiltin => "shadows-builtin",
            Self::MacroArity => "macro-arity",
            Self::MacroDepth => "macro-depth",
            Self::UnresolvedRef => "unresolved-ref",
            Self::DuplicateTag => "duplicate-tag",
            Self::MisplacedTag => "misplaced-tag",
            Self::NestedEval => "nested-eval",
            Self::NoEvaluator => "no-evaluator",
            Self::EvalFailed => "eval-failed",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diag {
    pub severity: Severity,
    pub code: DiagCode,
    pub message: String,
    pub span: Span,
    /// Secondary locations, e.g. "defined here".
    pub labels: Vec<(Span, String)>,
    /// A suggestion, e.g. "did you mean `integral`?".
    pub help: Option<String>,
}

impl Diag {
    #[must_use]
    pub fn new(severity: Severity, code: DiagCode, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            span,
            labels: Vec::new(),
            help: None,
        }
    }

    #[must_use]
    pub fn error(code: DiagCode, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, span, message)
    }

    #[must_use]
    pub fn warning(code: DiagCode, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, span, message)
    }

    #[must_use]
    pub fn hint(code: DiagCode, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Hint, code, span, message)
    }

    #[must_use]
    pub fn with_label(mut self, span: Span, text: impl Into<String>) -> Self {
        self.labels.push((span, text.into()));
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// The sink every stage pushes into. Stages never abort on a problem: they
/// push a [`Diag`], leave a placeholder, and carry on.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Diags(Vec<Diag>);

impl Diags {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, diag: Diag) {
        self.0.push(diag);
    }

    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Diag] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Diag> {
        self.0.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.0.iter().any(|d| d.severity == Severity::Error)
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Diag> {
        self.0
    }
}

impl<'a> IntoIterator for &'a Diags {
    type Item = &'a Diag;
    type IntoIter = std::slice::Iter<'a, Diag>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
