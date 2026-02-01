//! Function metadata and definitions for the Mazer language.
//!
//! This module defines [`ShowFunc`], an enum representing all built-in functions
//! in the Mazer language. Each variant is annotated with metadata including:
//!
//! - **Names**: The string identifiers used to invoke the function (e.g., `"sin"`, `"+"`)
//! - **Arity**: How many arguments the function accepts
//! - **Documentation**: A human-readable description
//! - **Symbol**: Optional MathML symbol override for rendering
//!
//! # Example
//!
//! ```ignore
//! use mazer_types::implfuncs::{ShowFunc, Arguments};
//!
//! let func = ShowFunc::from_name("arcsin");
//! assert_eq!(func.canonical_name(), "arcsin");
//! assert!(matches!(func.arity(), Arguments::Fixed(1)));
//! ```

use mazer_macros::FuncMeta;
use strum_macros::EnumIter;

/// Specifies how many arguments a function accepts.
///
/// This is used for validation and LSP completions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arguments {
    /// Exactly N arguments required.
    Fixed(usize),
    /// Between min and max arguments (inclusive).
    Range(usize, usize),
    /// At least N arguments required.
    Atleast(usize),
    /// Any number of arguments (0 or more).
    Variadic,
}

impl Arguments {
    /// Check if a given argument count is valid for this arity.
    pub fn is_valid(&self, count: usize) -> bool {
        match self {
            Arguments::Fixed(n) => count == *n,
            Arguments::Range(min, max) => count >= *min && count <= *max,
            Arguments::Atleast(min) => count >= *min,
            Arguments::Variadic => true,
        }
    }

    /// Returns the minimum number of arguments required.
    pub fn min(&self) -> usize {
        match self {
            Arguments::Fixed(n) => *n,
            Arguments::Range(min, _) => *min,
            Arguments::Atleast(min) => *min,
            Arguments::Variadic => 0,
        }
    }

    /// Returns the maximum number of arguments allowed, or None if unbounded.
    pub fn max(&self) -> Option<usize> {
        match self {
            Arguments::Fixed(n) => Some(*n),
            Arguments::Range(_, max) => Some(*max),
            Arguments::Atleast(_) => None,
            Arguments::Variadic => None,
        }
    }
}

/// Represents the kind of function (native or user-defined).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncKind {
    /// A built-in native function.
    Native,
    /// A user-defined function.
    UserDefined,
}

/// All built-in functions in the Mazer language.
///
/// Each variant represents a specific mathematical or structural operation.
/// Use [`ShowFunc::from_name`] to parse from a string, and the various
/// accessor methods to retrieve metadata.
///
/// # Categories
///
/// - **Core**: `define`, `defunc`, `quote`, `string`
/// - **Arithmetic**: `+`, `-`, `*`, `/`, `^`, `frac`, `sqrt`, `root`
/// - **Comparison**: `=`, `!=`, `<`, `>`, `<=`, `>=`, `approx`
/// - **Calculus**: `integral`, `sum`, `prod`, `limit`, `derivative`, `partial`
/// - **Trigonometry**: `sin`, `cos`, `tan`, `cot`, `sec`, `csc`, `arcsin`, `arccos`, `arctan`
/// - **Logarithms**: `ln`, `log`, `exp`
/// - **Other Math**: `abs`, `floor`, `ceil`, `factorial`, `binom`
/// - **Linear Algebra**: `matrix`, `vec`, `det`
/// - **Sets**: `set`, `in`, `notin`, `subset`, `superset`, `union`, `intersect`
/// - **Logic**: `and`, `or`, `not`, `implies`, `iff`, `forall`, `exists`
/// - **Grouping**: `paren`, `bracket`, `brace`
/// - **Annotations**: `text`, `subscript`, `superscript`, `overline`, `hat`, `dot`, `ddot`, `arrow`, `box`
#[derive(Debug, EnumIter, FuncMeta)]
pub enum ShowFunc {
    // =========================================================================
    // Core Language Constructs
    // =========================================================================

    /// Define a variable binding: `(define name value)`
    #[func(names = ["define"], arity = Fixed(2), doc = "Define a variable: (define name value)")]
    Define,

    /// Define a function: `(defunc name (params...) body)`
    #[func(names = ["defunc"], arity = AtLeast(3), doc = "Define a function: (defunc name (params) body)")]
    Defunc,

    /// Quote an expression to prevent evaluation: `(quote expr)` or `'expr`
    #[func(names = ["quote"], arity = Fixed(1), doc = "Quote an expression to prevent evaluation")]
    Quote,

    /// String literal or concatenation
    #[func(names = ["string"], arity = Variadic, doc = "String literal or concatenation")]
    String,

    // =========================================================================
    // Arithmetic Operations
    // =========================================================================

    /// Juxtaposition (implicit multiplication)
    #[func(names = ["jux", "juxtapose"], arity = AtLeast(2), doc = "Juxtaposition (implicit multiplication)")]
    Jux,

    /// Addition: `(+ a b ...)` or `(add a b ...)`
    #[func(names = ["+", "add"], arity = Variadic, doc = "Addition: (+ a b ...) or (add a b ...)", symbol = "+")]
    Add,

    /// Subtraction or negation: `(- a b)` or `(- a)`
    #[func(names = ["-", "sub"], arity = AtLeast(1), doc = "Subtraction: (- a b) or negation: (- a)", symbol = "-")]
    Sub,

    /// Multiplication: `(* a b ...)`
    #[func(names = ["*", "mul"], arity = Variadic, doc = "Multiplication: (* a b ...)", symbol = "×")]
    Mul,

    /// Division: `(/ a b)` renders as fraction
    #[func(names = ["/", "div"], arity = AtLeast(1), doc = "Division: (/ a b) renders as fraction", symbol = "÷")]
    Div,

    /// Exponentiation: `(^ base exp)` or `(pow base exp)`
    #[func(names = ["^", "pow"], arity = Fixed(2), doc = "Exponentiation: (^ base exp) or (pow base exp)")]
    Pow,

    /// Fraction: `(frac num denom)`
    #[func(names = ["frac"], arity = Fixed(2), doc = "Fraction: (frac num denom)")]
    Frac,

    /// Square root: `(sqrt x)`
    #[func(names = ["sqrt"], arity = Fixed(1), doc = "Square root: (sqrt x)")]
    Sqrt,

    /// Nth root: `(root n x)` for the nth root of x
    #[func(names = ["root"], arity = Fixed(2), doc = "Nth root: (root n x) for the nth root of x")]
    Root,

    // =========================================================================
    // Comparison Operations
    // =========================================================================

    /// Equality: `(= a b)` or `(eq a b)`
    #[func(names = ["=", "eq"], arity = AtLeast(2), doc = "Equality: (= a b) or (eq a b)", symbol = "=")]
    Eq,

    /// Not equal: `(!= a b)` or `(neq a b)`
    #[func(names = ["!=", "neq"], arity = Fixed(2), doc = "Not equal: (!= a b) or (neq a b)", symbol = "≠")]
    Neq,

    /// Greater than: `(> a b)`
    #[func(names = [">", "gt"], arity = AtLeast(2), doc = "Greater than: (> a b)", symbol = ">")]
    Gt,

    /// Less than: `(< a b)`
    #[func(names = ["<", "lt"], arity = AtLeast(2), doc = "Less than: (< a b)", symbol = "<")]
    Lt,

    /// Approximate equality: `(approx a b)`
    #[func(names = ["approx", "≈"], arity = Fixed(2), doc = "Approximate equality: (approx a b)", symbol = "≈")]
    Approx,

    /// Greater than or equal: `(>= a b)`
    #[func(names = [">=", "geq"], arity = AtLeast(2), doc = "Greater than or equal: (>= a b)", symbol = "≥")]
    Geq,

    /// Less than or equal: `(<= a b)`
    #[func(names = ["<=", "leq"], arity = AtLeast(2), doc = "Less than or equal: (<= a b)", symbol = "≤")]
    Leq,

    // =========================================================================
    // Calculus
    // =========================================================================

    /// Integration: `(integral expr)` or `(integral lower upper expr var)`
    #[func(names = ["integral"], arity = Range(1, 4), doc = "Integration: (integral expr) or (integral lower upper expr var)")]
    Integral,

    /// Summation: `(sum expr)` or `(sum lower upper expr)`
    #[func(names = ["sum"], arity = Range(1, 3), doc = "Summation: (sum expr) or (sum lower upper expr)")]
    Sum,

    /// Product notation: `(prod lower upper expr)`
    #[func(names = ["prod", "product"], arity = Range(1, 3), doc = "Product notation: (prod lower upper expr)")]
    Prod,

    /// Limit: `(limit var approach expr)`
    #[func(names = ["lim", "limit"], arity = Range(2, 3), doc = "Limit: (limit var approach expr)")]
    Limit,

    /// Derivative: `(derivative expr var)` or `(derivative expr var n)`
    #[func(names = ["derivative", "deriv"], arity = Range(2, 3), doc = "Derivative: (derivative expr var) or (derivative expr var n)")]
    Derivative,

    /// Partial derivative: `(partial expr var)`
    #[func(names = ["partial"], arity = Fixed(2), doc = "Partial derivative: (partial expr var)")]
    Partial,

    // =========================================================================
    // Trigonometric Functions
    // =========================================================================

    /// Sine: `(sin x)`
    #[func(names = ["sin"], arity = Fixed(1), doc = "Sine: (sin x)")]
    Sin,

    /// Cosine: `(cos x)`
    #[func(names = ["cos"], arity = Fixed(1), doc = "Cosine: (cos x)")]
    Cos,

    /// Tangent: `(tan x)`
    #[func(names = ["tan"], arity = Fixed(1), doc = "Tangent: (tan x)")]
    Tan,

    /// Cotangent: `(cot x)`
    #[func(names = ["cot"], arity = Fixed(1), doc = "Cotangent: (cot x)")]
    Cot,

    /// Secant: `(sec x)`
    #[func(names = ["sec"], arity = Fixed(1), doc = "Secant: (sec x)")]
    Sec,

    /// Cosecant: `(cosec x)` or `(csc x)`
    #[func(names = ["cosec", "csc"], arity = Fixed(1), doc = "Cosecant: (cosec x) or (csc x)")]
    Cosec,

    /// Inverse sine: `(arcsin x)`, rendered as sin⁻¹
    #[func(names = ["arcsin", "asin"], arity = Fixed(1), doc = "Inverse sine: (arcsin x), rendered as sin⁻¹")]
    Arcsin,

    /// Inverse cosine: `(arccos x)`, rendered as cos⁻¹
    #[func(names = ["arccos", "acos"], arity = Fixed(1), doc = "Inverse cosine: (arccos x), rendered as cos⁻¹")]
    Arccos,

    /// Inverse tangent: `(arctan x)`, rendered as tan⁻¹
    #[func(names = ["arctan", "atan"], arity = Fixed(1), doc = "Inverse tangent: (arctan x), rendered as tan⁻¹")]
    Arctan,

    // =========================================================================
    // Logarithms and Exponentials
    // =========================================================================

    /// Natural logarithm: `(ln x)`
    #[func(names = ["ln"], arity = Fixed(1), doc = "Natural logarithm: (ln x)")]
    Ln,

    /// Logarithm: `(log x)` or `(log base x)`
    #[func(names = ["log"], arity = Range(1, 2), doc = "Logarithm: (log x) or (log base x)")]
    Log,

    /// Exponential: `(exp x)` renders as e^x
    #[func(names = ["exp"], arity = Fixed(1), doc = "Exponential: (exp x) renders as e^x")]
    Exp,

    // =========================================================================
    // Other Mathematical Functions
    // =========================================================================

    /// Absolute value: `(abs x)` renders as |x|
    #[func(names = ["abs"], arity = Fixed(1), doc = "Absolute value: (abs x) renders as |x|")]
    Abs,

    /// Floor: `(floor x)` renders as ⌊x⌋
    #[func(names = ["floor"], arity = Fixed(1), doc = "Floor: (floor x) renders as ⌊x⌋")]
    Floor,

    /// Ceiling: `(ceil x)` renders as ⌈x⌉
    #[func(names = ["ceil"], arity = Fixed(1), doc = "Ceiling: (ceil x) renders as ⌈x⌉")]
    Ceil,

    /// Factorial: `(fact n)` renders as n!
    #[func(names = ["fact", "factorial"], arity = Fixed(1), doc = "Factorial: (fact n) renders as n!")]
    Fact,

    /// Binomial coefficient: `(binom n k)`
    #[func(names = ["binom", "nCr"], arity = Fixed(2), doc = "Binomial coefficient: (binom n k)")]
    Binom,

    // =========================================================================
    // Linear Algebra
    // =========================================================================

    /// Matrix: `(matrix (row1) (row2) ...)`
    #[func(names = ["mat", "matrix"], arity = Variadic, doc = "Matrix: (matrix (row1) (row2) ...)")]
    Matrix,

    /// Column vector: `(vec a b c ...)`
    #[func(names = ["vec", "vector"], arity = Variadic, doc = "Column vector: (vec a b c ...)")]
    Vec,

    /// Determinant: `(det (row1) (row2) ...)`
    #[func(names = ["det", "determinant"], arity = Variadic, doc = "Determinant: (det (row1) (row2) ...)")]
    Det,

    // =========================================================================
    // Set Operations
    // =========================================================================

    /// Set: `(set a b c ...)` renders as {a, b, c}
    #[func(names = ["set"], arity = Variadic, doc = "Set: (set a b c ...) renders as {a, b, c}")]
    Set,

    /// Element of: `(in x S)` renders as x ∈ S
    #[func(names = ["in"], arity = Fixed(2), doc = "Element of: (in x S) renders as x ∈ S", symbol = "∈")]
    In,

    /// Not element of: `(notin x S)` renders as x ∉ S
    #[func(names = ["notin"], arity = Fixed(2), doc = "Not element of: (notin x S) renders as x ∉ S", symbol = "∉")]
    NotIn,

    /// Subset: `(subset A B)` renders as A ⊆ B
    #[func(names = ["subset"], arity = Fixed(2), doc = "Subset: (subset A B) renders as A ⊆ B", symbol = "⊆")]
    Subset,

    /// Superset: `(superset A B)` renders as A ⊇ B
    #[func(names = ["superset"], arity = Fixed(2), doc = "Superset: (superset A B) renders as A ⊇ B", symbol = "⊇")]
    Superset,

    /// Union: `(union A B)` renders as A ∪ B
    #[func(names = ["union"], arity = AtLeast(2), doc = "Union: (union A B) renders as A ∪ B", symbol = "∪")]
    Union,

    /// Intersection: `(intersect A B)` renders as A ∩ B
    #[func(names = ["intersect"], arity = AtLeast(2), doc = "Intersection: (intersect A B) renders as A ∩ B", symbol = "∩")]
    Intersect,

    // =========================================================================
    // Logical Operations
    // =========================================================================

    /// Logical AND: `(and a b)`
    #[func(names = ["and"], arity = AtLeast(2), doc = "Logical AND: (and a b)", symbol = "∧")]
    And,

    /// Logical OR: `(or a b)`
    #[func(names = ["or"], arity = AtLeast(2), doc = "Logical OR: (or a b)", symbol = "∨")]
    Or,

    /// Logical NOT: `(not a)`
    #[func(names = ["not"], arity = Fixed(1), doc = "Logical NOT: (not a)", symbol = "¬")]
    Not,

    /// Implication: `(implies a b)` renders as a ⟹ b
    #[func(names = ["implies"], arity = Fixed(2), doc = "Implication: (implies a b) renders as a ⟹ b", symbol = "⟹")]
    Implies,

    /// Biconditional: `(iff a b)` renders as a ⟺ b
    #[func(names = ["iff"], arity = Fixed(2), doc = "Biconditional: (iff a b) renders as a ⟺ b", symbol = "⟺")]
    Iff,

    /// Universal quantifier: `(forall x P)`
    #[func(names = ["forall"], arity = Fixed(2), doc = "Universal quantifier: (forall x P)", symbol = "∀")]
    ForAll,

    /// Existential quantifier: `(exists x P)`
    #[func(names = ["exists"], arity = Fixed(2), doc = "Existential quantifier: (exists x P)", symbol = "∃")]
    Exists,

    // =========================================================================
    // Grouping / Delimiters
    // =========================================================================

    /// Parentheses: `(paren expr)` renders as (expr)
    #[func(names = ["paren"], arity = Fixed(1), doc = "Parentheses: (paren expr) renders as (expr)")]
    Paren,

    /// Brackets: `(bracket expr)` renders as [expr]
    #[func(names = ["bracket"], arity = Fixed(1), doc = "Brackets: (bracket expr) renders as [expr]")]
    Bracket,

    /// Braces: `(brace expr)` renders as {expr}
    #[func(names = ["brace"], arity = Fixed(1), doc = "Braces: (brace expr) renders as {expr}")]
    Brace,

    // =========================================================================
    // Annotations and Decorations
    // =========================================================================

    /// Text: `(text "string")` renders as plain text
    #[func(names = ["text"], arity = Variadic, doc = "Text: (text \"string\") renders as plain text")]
    Text,

    /// Subscript: `(subscript base sub)`
    #[func(names = ["subscript"], arity = Fixed(2), doc = "Subscript: (subscript base sub)")]
    Subscript,

    /// Superscript: `(superscript base sup)`
    #[func(names = ["superscript"], arity = Fixed(2), doc = "Superscript: (superscript base sup)")]
    Superscript,

    /// Overline: `(bar x)` renders as x̄
    #[func(names = ["bar", "overline"], arity = Fixed(1), doc = "Overline: (bar x) renders as x̄")]
    Overline,

    /// Hat: `(hat x)` renders as x̂
    #[func(names = ["hat"], arity = Fixed(1), doc = "Hat: (hat x) renders as x̂")]
    Hat,

    /// Dot: `(dot x)` renders as ẋ
    #[func(names = ["dot"], arity = Fixed(1), doc = "Dot: (dot x) renders as ẋ")]
    Dot,

    /// Double dot: `(ddot x)` renders as ẍ
    #[func(names = ["ddot"], arity = Fixed(1), doc = "Double dot: (ddot x) renders as ẍ")]
    Ddot,

    /// Arrow: `(arrow x)` renders as x⃗
    #[func(names = ["arrow"], arity = Fixed(1), doc = "Arrow: (arrow x) renders as x⃗")]
    Arrow,

    /// Box: `(box expr)` renders expr in a box
    #[func(names = ["box"], arity = Fixed(1), doc = "Box: (box expr) renders expr in a box")]
    Box,

    /// Fallback for user-defined or unknown functions.
    MaybeFunc(String),
}

impl From<String> for ShowFunc {
    fn from(s: String) -> Self {
        ShowFunc::from_name(&s)
    }
}

impl From<&str> for ShowFunc {
    fn from(s: &str) -> Self {
        ShowFunc::from_name(s)
    }
}
