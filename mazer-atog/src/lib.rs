use std::collections::HashMap;
use std::sync::LazyLock;

/// A symbol entry with its unicode representation and documentation.
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub symbol: &'static str,
    pub doc: &'static str,
}

/// Declare all symbols with documentation. Every entry must have a non-empty doc string.
macro_rules! symbols {
    ( $( ($name:expr, $sym:expr, $doc:expr) ),* $(,)? ) => {{
        // Compile-time check: ensure no doc string is empty
        $( const _: () = assert!(!$doc.is_empty(), concat!("Symbol `", $name, "` is missing documentation")); )*

        HashMap::from([
            $( ($name, SymbolEntry { symbol: $sym, doc: $doc }) ),*
        ])
    }};
}

static SYMBOLS: LazyLock<HashMap<&'static str, SymbolEntry>> = LazyLock::new(|| {
    symbols![
        // Greek lowercase
        ("alpha",   "α", "Greek lowercase alpha"),
        ("beta",    "β", "Greek lowercase beta"),
        ("gamma",   "γ", "Greek lowercase gamma"),
        ("delta",   "δ", "Greek lowercase delta"),
        ("epsilon", "ε", "Greek lowercase epsilon"),
        ("zeta",    "ζ", "Greek lowercase zeta"),
        ("eta",     "η", "Greek lowercase eta"),
        ("theta",   "θ", "Greek lowercase theta"),
        ("iota",    "ι", "Greek lowercase iota"),
        ("kappa",   "κ", "Greek lowercase kappa"),
        ("lambda",  "λ", "Greek lowercase lambda"),
        ("mu",      "μ", "Greek lowercase mu"),
        ("nu",      "ν", "Greek lowercase nu"),
        ("xi",      "ξ", "Greek lowercase xi"),
        ("omicron", "ο", "Greek lowercase omicron"),
        ("pi",      "π", "Greek lowercase pi — ratio of circumference to diameter"),
        ("rho",     "ρ", "Greek lowercase rho"),
        ("sigma",   "σ", "Greek lowercase sigma"),
        ("tau",     "τ", "Greek lowercase tau"),
        ("upsilon", "υ", "Greek lowercase upsilon"),
        ("phi",     "φ", "Greek lowercase phi"),
        ("chi",     "χ", "Greek lowercase chi"),
        ("psi",     "ψ", "Greek lowercase psi"),
        ("omega",   "ω", "Greek lowercase omega"),

        // Greek uppercase
        ("Alpha",   "Α", "Greek uppercase Alpha"),
        ("Beta",    "Β", "Greek uppercase Beta"),
        ("Gamma",   "Γ", "Greek uppercase Gamma"),
        ("Delta",   "Δ", "Greek uppercase Delta — change/difference operator"),
        ("Epsilon", "Ε", "Greek uppercase Epsilon"),
        ("Zeta",    "Ζ", "Greek uppercase Zeta"),
        ("Eta",     "Η", "Greek uppercase Eta"),
        ("Theta",   "Θ", "Greek uppercase Theta"),
        ("Iota",    "Ι", "Greek uppercase Iota"),
        ("Kappa",   "Κ", "Greek uppercase Kappa"),
        ("Lambda",  "Λ", "Greek uppercase Lambda"),
        ("Mu",      "Μ", "Greek uppercase Mu"),
        ("Nu",      "Ν", "Greek uppercase Nu"),
        ("Xi",      "Ξ", "Greek uppercase Xi"),
        ("Omicron", "Ο", "Greek uppercase Omicron"),
        ("Pi",      "Π", "Greek uppercase Pi — product operator"),
        ("Rho",     "Ρ", "Greek uppercase Rho"),
        ("Sigma",   "Σ", "Greek uppercase Sigma — summation operator"),
        ("Tau",     "Τ", "Greek uppercase Tau"),
        ("Upsilon", "Υ", "Greek uppercase Upsilon"),
        ("Phi",     "Φ", "Greek uppercase Phi"),
        ("Chi",     "Χ", "Greek uppercase Chi"),
        ("Psi",     "Ψ", "Greek uppercase Psi"),
        ("Omega",   "Ω", "Greek uppercase Omega"),

        // Mathematical operators
        ("infinity",     "∞", "Infinity symbol"),
        ("partial",      "∂", "Partial derivative operator"),
        ("nabla",        "∇", "Nabla/del operator — gradient, divergence, curl"),
        ("therefore",    "∴", "Therefore (logical conclusion)"),
        ("angle",        "∠", "Angle symbol"),
        ("degree",       "°", "Degree symbol"),
        ("plusminus",    "±", "Plus-minus sign"),
        ("times",        "×", "Multiplication/cross product sign"),
        ("divide",       "÷", "Division sign"),

        // Comparison/relations
        ("approx",       "≈", "Approximately equal"),
        ("neq",          "≠", "Not equal"),
        ("leq",          "≤", "Less than or equal"),
        ("geq",          "≥", "Greater than or equal"),
        ("equiv",        "≡", "Identical/equivalent to"),

        // Set theory
        ("subset",       "⊂", "Proper subset"),
        ("supset",       "⊃", "Proper superset"),
        ("in",           "∈", "Element of a set"),
        ("notin",        "∉", "Not an element of a set"),
        ("union",        "∪", "Set union"),
        ("intersection", "∩", "Set intersection"),
        ("emptyset",     "∅", "Empty set"),

        // Logic
        ("forall",       "∀", "Universal quantifier — for all"),
        ("exists",       "∃", "Existential quantifier — there exists"),

        // Arrows
        ("arrowup",      "↑", "Upward arrow"),
        ("arrowdown",    "↓", "Downward arrow"),
        ("arrowleft",    "←", "Leftward arrow"),
        ("arrowright",   "→", "Rightward arrow"),
        ("arrow",        "→", "Right arrow (shorthand for arrowright)"),

        // Miscellaneous
        ("ellipsis",     "…", "Horizontal ellipsis — continuation dots"),

        // Special sets and transforms
        ("Laplace",      "𝓛", "Laplace transform operator"),
        ("Reals",        "ℝ", "Set of all real numbers"),
        ("reals",        "ℝ", "Set of all real numbers (alias for Reals)"),
        ("Naturals",     "ℕ", "Set of all natural numbers"),
        ("naturals",     "ℕ", "Set of all natural numbers (alias for Naturals)"),
        ("Integers",     "ℤ", "Set of all integers"),
        ("integers",     "ℤ", "Set of all integers (alias for Integers)"),
        ("Rationals",    "ℚ", "Set of all rational numbers"),
        ("rationals",    "ℚ", "Set of all rational numbers (alias for Rationals)"),
        ("Complex",      "ℂ", "Set of all complex numbers"),
        ("complex",      "ℂ", "Set of all complex numbers (alias for Complex)"),
    ]
});

pub struct Atog;

impl Atog {
    /// Look up a symbol name, returning its unicode character.
    pub fn get(s: &str) -> Option<&'static str> {
        SYMBOLS.get(s).map(|e| e.symbol)
    }

    /// Look up a symbol name, returning the full entry with documentation.
    pub fn get_entry(s: &str) -> Option<&'static SymbolEntry> {
        SYMBOLS.get(s)
    }

    /// Returns an iterator over all (name, entry) pairs.
    pub fn iter() -> impl Iterator<Item = (&'static &'static str, &'static SymbolEntry)> {
        SYMBOLS.iter()
    }
}
