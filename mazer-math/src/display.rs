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
    /// The LaTeX spelling, e.g. `\tau`. Every built-in row has one; empty only for a row that omits `.latex(...)`.
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
#[allow(clippy::too_many_lines)] // one declarative row per name; splitting would only hide the table
fn table() -> Vec<DisplayName> {
    vec![
        // Greek lowercase
        DisplayName::letter("alpha",    "α", "Greek lowercase alpha").latex("\\alpha"),
        DisplayName::letter("beta",     "β", "Greek lowercase beta").latex("\\beta"),
        DisplayName::letter("gamma",    "γ", "Greek lowercase gamma").latex("\\gamma"),
        DisplayName::letter("delta",    "δ", "Greek lowercase delta").latex("\\delta"),
        DisplayName::letter("epsilon",  "ε", "Greek lowercase epsilon").latex("\\epsilon"),
        DisplayName::letter("zeta",     "ζ", "Greek lowercase zeta").latex("\\zeta"),
        DisplayName::letter("eta",      "η", "Greek lowercase eta").latex("\\eta"),
        DisplayName::letter("theta",    "θ", "Greek lowercase theta").latex("\\theta"),
        DisplayName::letter("iota",     "ι", "Greek lowercase iota").latex("\\iota"),
        DisplayName::letter("kappa",    "κ", "Greek lowercase kappa").latex("\\kappa"),
        DisplayName::letter("lambda",   "λ", "Greek lowercase lambda").latex("\\lambda"),
        DisplayName::letter("mu",       "μ", "Greek lowercase mu").latex("\\mu"),
        DisplayName::letter("nu",       "ν", "Greek lowercase nu").latex("\\nu"),
        DisplayName::letter("xi",       "ξ", "Greek lowercase xi").latex("\\xi"),
        DisplayName::letter("omicron",  "ο", "Greek lowercase omicron").latex("o"),
        DisplayName::letter("pi",       "π", "Greek lowercase pi, ratio of circumference to diameter").latex("\\pi"),
        DisplayName::letter("rho",      "ρ", "Greek lowercase rho").latex("\\rho"),
        DisplayName::letter("sigma",    "σ", "Greek lowercase sigma").latex("\\sigma"),
        DisplayName::letter("tau",      "τ", "Greek lowercase tau").latex("\\tau"),
        DisplayName::letter("upsilon",  "υ", "Greek lowercase upsilon").latex("\\upsilon"),
        DisplayName::letter("phi",      "φ", "Greek lowercase phi").latex("\\phi"),
        DisplayName::letter("chi",      "χ", "Greek lowercase chi").latex("\\chi"),
        DisplayName::letter("psi",      "ψ", "Greek lowercase psi").latex("\\psi"),
        DisplayName::letter("omega",    "ω", "Greek lowercase omega").latex("\\omega"),

        // Greek lowercase variants
        DisplayName::letter("varepsilon",  "ϵ", "Greek variant form: varepsilon").latex("\\varepsilon"),
        DisplayName::letter("vartheta",    "ϑ", "Greek variant form: vartheta").latex("\\vartheta"),
        DisplayName::letter("varkappa",    "ϰ", "Greek variant form: varkappa").latex("\\varkappa"),
        DisplayName::letter("varpi",       "ϖ", "Greek variant form: varpi").latex("\\varpi"),
        DisplayName::letter("varrho",      "ϱ", "Greek variant form: varrho").latex("\\varrho"),
        DisplayName::letter("varsigma",    "ς", "Greek variant form: varsigma").latex("\\varsigma"),
        DisplayName::letter("varphi",      "ϕ", "Greek variant form: varphi").latex("\\varphi"),
        DisplayName::letter("digamma",     "ϝ", "Greek variant form: digamma").latex("\\digamma"),

        // Greek uppercase
        DisplayName::letter("Alpha",    "Α", "Greek uppercase Alpha").latex("A"),
        DisplayName::letter("Beta",     "Β", "Greek uppercase Beta").latex("B"),
        DisplayName::letter("Gamma",    "Γ", "Greek uppercase Gamma").latex("\\Gamma"),
        DisplayName::letter("Delta",    "Δ", "Greek uppercase Delta, change/difference operator").latex("\\Delta"),
        DisplayName::letter("Epsilon",  "Ε", "Greek uppercase Epsilon").latex("E"),
        DisplayName::letter("Zeta",     "Ζ", "Greek uppercase Zeta").latex("Z"),
        DisplayName::letter("Eta",      "Η", "Greek uppercase Eta").latex("H"),
        DisplayName::letter("Theta",    "Θ", "Greek uppercase Theta").latex("\\Theta"),
        DisplayName::letter("Iota",     "Ι", "Greek uppercase Iota").latex("I"),
        DisplayName::letter("Kappa",    "Κ", "Greek uppercase Kappa").latex("K"),
        DisplayName::letter("Lambda",   "Λ", "Greek uppercase Lambda").latex("\\Lambda"),
        DisplayName::letter("Mu",       "Μ", "Greek uppercase Mu").latex("M"),
        DisplayName::letter("Nu",       "Ν", "Greek uppercase Nu").latex("N"),
        DisplayName::letter("Xi",       "Ξ", "Greek uppercase Xi").latex("\\Xi"),
        DisplayName::letter("Omicron",  "Ο", "Greek uppercase Omicron").latex("O"),
        DisplayName::letter("Pi",       "Π", "Greek uppercase Pi, product operator").latex("\\Pi"),
        DisplayName::letter("Rho",      "Ρ", "Greek uppercase Rho").latex("P"),
        DisplayName::letter("Sigma",    "Σ", "Greek uppercase Sigma, summation operator").latex("\\Sigma"),
        DisplayName::letter("Tau",      "Τ", "Greek uppercase Tau").latex("T"),
        DisplayName::letter("Upsilon",  "Υ", "Greek uppercase Upsilon").latex("\\Upsilon"),
        DisplayName::letter("Phi",      "Φ", "Greek uppercase Phi").latex("\\Phi"),
        DisplayName::letter("Chi",      "Χ", "Greek uppercase Chi").latex("X"),
        DisplayName::letter("Psi",      "Ψ", "Greek uppercase Psi").latex("\\Psi"),
        DisplayName::letter("Omega",    "Ω", "Greek uppercase Omega").latex("\\Omega"),

        // Constants and letters
        DisplayName::letter("infinity",     "∞", "Infinity symbol").latex("\\infty"),
        DisplayName::letter("nabla",        "∇", "Nabla/del operator: gradient, divergence, curl").latex("\\nabla"),
        DisplayName::letter("emptyset",     "∅", "Empty set").latex("\\emptyset"),
        DisplayName::letter("angle",        "∠", "Angle symbol").latex("\\angle"),
        DisplayName::letter("degree",       "°", "Degree symbol").latex("^\\circ"),
        DisplayName::letter("hbar",         "ℏ", "Reduced Planck constant").latex("\\hbar"),
        DisplayName::letter("ell",          "ℓ", "Script small l (length, angular momentum quantum number)").latex("\\ell"),
        DisplayName::letter("aleph",        "ℵ", "Aleph (cardinality)").latex("\\aleph"),
        DisplayName::letter("beth",         "ℶ", "Beth (cardinality)").latex("\\beth"),
        DisplayName::letter("angstrom",     "Å", "Angstrom, unit of length").latex("\\text{\\AA}"),
        DisplayName::letter("imath",        "ı", "Dotless i").latex("\\imath"),
        DisplayName::letter("jmath",        "ȷ", "Dotless j").latex("\\jmath"),
        DisplayName::letter("wp",           "℘", "Weierstrass elliptic function").latex("\\wp"),
        DisplayName::letter("Laplace",      "𝓛", "Laplace transform operator").latex("\\mathcal{L}"),
        DisplayName::letter("Fourier",      "ℱ", "Fourier transform operator").latex("\\mathcal{F}"),
        DisplayName::letter("Hamiltonian",  "ℋ", "Hamiltonian operator").latex("\\mathcal{H}"),
        DisplayName::letter("Re",           "ℜ", "Real part").latex("\\Re"),
        DisplayName::letter("Im",           "ℑ", "Imaginary part").latex("\\Im"),

        // Number sets
        DisplayName::letter("Reals",        "ℝ", "Set of all real numbers").latex("\\mathbb{R}"),
        DisplayName::letter("reals",        "ℝ", "Set of all real numbers (alias for Reals)").latex("\\mathbb{R}"),
        DisplayName::letter("Naturals",     "ℕ", "Set of all natural numbers").latex("\\mathbb{N}"),
        DisplayName::letter("naturals",     "ℕ", "Set of all natural numbers (alias for Naturals)").latex("\\mathbb{N}"),
        DisplayName::letter("Integers",     "ℤ", "Set of all integer numbers").latex("\\mathbb{Z}"),
        DisplayName::letter("integers",     "ℤ", "Set of all integer numbers (alias for Integers)").latex("\\mathbb{Z}"),
        DisplayName::letter("Rationals",    "ℚ", "Set of all rational numbers").latex("\\mathbb{Q}"),
        DisplayName::letter("rationals",    "ℚ", "Set of all rational numbers (alias for Rationals)").latex("\\mathbb{Q}"),
        DisplayName::letter("Complex",      "ℂ", "Set of all complex numbers").latex("\\mathbb{C}"),
        DisplayName::letter("complex",      "ℂ", "Set of all complex numbers (alias for Complex)").latex("\\mathbb{C}"),
        DisplayName::letter("Complexes",    "ℂ", "Set of all complex numbers (alias for Complex)").latex("\\mathbb{C}"),
        DisplayName::letter("complexes",    "ℂ", "Set of all complex numbers (alias for Complex)").latex("\\mathbb{C}"),
        DisplayName::letter("Primes",       "ℙ", "Set of all prime numbers").latex("\\mathbb{P}"),
        DisplayName::letter("Quaternions",  "ℍ", "Set of all quaternions").latex("\\mathbb{H}"),

        // Arrows
        DisplayName::operator("to",                  "→", "Right arrow, function type or limit").latex("\\to"),
        DisplayName::operator("arrowright",          "→", "Rightward arrow").latex("\\rightarrow"),
        DisplayName::operator("arrowleft",           "←", "Leftward arrow").latex("\\leftarrow"),
        DisplayName::operator("arrowup",             "↑", "Upward arrow").latex("\\uparrow"),
        DisplayName::operator("arrowdown",           "↓", "Downward arrow").latex("\\downarrow"),
        DisplayName::operator("mapsto",              "↦", "Maps-to arrow, as in substitutions [α ↦ τ]").latex("\\mapsto"),
        DisplayName::operator("rightarrow",          "→", "Right arrow").latex("\\rightarrow"),
        DisplayName::operator("leftarrow",           "←", "Left arrow").latex("\\leftarrow"),
        DisplayName::operator("leftrightarrow",      "↔", "Left-right arrow").latex("\\leftrightarrow"),
        DisplayName::operator("uparrow",             "↑", "Up arrow").latex("\\uparrow"),
        DisplayName::operator("downarrow",           "↓", "Down arrow").latex("\\downarrow"),
        DisplayName::operator("updownarrow",         "↕", "Up-down arrow").latex("\\updownarrow"),
        DisplayName::operator("Rightarrow",          "⇒", "Double right arrow").latex("\\Rightarrow"),
        DisplayName::operator("Leftarrow",           "⇐", "Double left arrow").latex("\\Leftarrow"),
        DisplayName::operator("Leftrightarrow",      "⇔", "Double left-right arrow").latex("\\Leftrightarrow"),
        DisplayName::operator("Uparrow",             "⇑", "Double up arrow").latex("\\Uparrow"),
        DisplayName::operator("Downarrow",           "⇓", "Double down arrow").latex("\\Downarrow"),
        DisplayName::operator("longrightarrow",      "⟶", "Long right arrow").latex("\\longrightarrow"),
        DisplayName::operator("longleftarrow",       "⟵", "Long left arrow").latex("\\longleftarrow"),
        DisplayName::operator("longleftrightarrow",  "⟷", "Long left-right arrow").latex("\\longleftrightarrow"),
        DisplayName::operator("Longrightarrow",      "⟹", "Long double right arrow").latex("\\Longrightarrow"),
        DisplayName::operator("Longleftarrow",       "⟸", "Long double left arrow").latex("\\Longleftarrow"),
        DisplayName::operator("Longleftrightarrow",  "⟺", "Long double left-right arrow").latex("\\Longleftrightarrow"),
        DisplayName::operator("longmapsto",          "⟼", "Long maps-to arrow").latex("\\longmapsto"),
        DisplayName::operator("hookrightarrow",      "↪", "Hooked right arrow (inclusion)").latex("\\hookrightarrow"),
        DisplayName::operator("hookleftarrow",       "↩", "Hooked left arrow").latex("\\hookleftarrow"),
        DisplayName::operator("twoheadrightarrow",   "↠", "Two-headed right arrow (surjection)").latex("\\twoheadrightarrow"),
        DisplayName::operator("rightsquigarrow",     "⇝", "Squiggly right arrow").latex("\\rightsquigarrow"),
        DisplayName::operator("nearrow",             "↗", "North-east arrow").latex("\\nearrow"),
        DisplayName::operator("searrow",             "↘", "South-east arrow").latex("\\searrow"),
        DisplayName::operator("nwarrow",             "↖", "North-west arrow").latex("\\nwarrow"),
        DisplayName::operator("swarrow",             "↙", "South-west arrow").latex("\\swarrow"),
        DisplayName::operator("rightharpoonup",      "⇀", "Right harpoon with barb up").latex("\\rightharpoonup"),
        DisplayName::operator("leftharpoonup",       "↼", "Left harpoon with barb up").latex("\\leftharpoonup"),
        DisplayName::operator("rightleftharpoons",   "⇌", "Right harpoon over left harpoon (equilibrium)").latex("\\rightleftharpoons"),

        // Relations
        DisplayName::operator("equiv",      "≡", "Identical/equivalent to").latex("\\equiv"),
        DisplayName::operator("nequiv",     "≢", "Not identical").latex("\\not\\equiv"),
        DisplayName::operator("sim",        "∼", "Similar to, distributed as").latex("\\sim"),
        DisplayName::operator("simeq",      "≃", "Asymptotically equal").latex("\\simeq"),
        DisplayName::operator("cong",       "≅", "Congruent / isomorphic").latex("\\cong"),
        DisplayName::operator("propto",     "∝", "Proportional to").latex("\\propto"),
        DisplayName::operator("ll",         "≪", "Much less than").latex("\\ll"),
        DisplayName::operator("gg",         "≫", "Much greater than").latex("\\gg"),
        DisplayName::operator("le",         "≤", "Less than or equal (LaTeX name)").latex("\\le"),
        DisplayName::operator("ge",         "≥", "Greater than or equal (LaTeX name)").latex("\\ge"),
        DisplayName::operator("ne",         "≠", "Not equal (LaTeX name)").latex("\\ne"),
        DisplayName::operator("prec",       "≺", "Precedes").latex("\\prec"),
        DisplayName::operator("succ",       "≻", "Succeeds").latex("\\succ"),
        DisplayName::operator("preceq",     "⪯", "Precedes or equals").latex("\\preceq"),
        DisplayName::operator("succeq",     "⪰", "Succeeds or equals").latex("\\succeq"),
        DisplayName::operator("perp",       "⊥", "Perpendicular / orthogonal").latex("\\perp"),
        DisplayName::operator("parallel",   "∥", "Parallel").latex("\\parallel"),
        DisplayName::operator("mid",        "∣", "Divides").latex("\\mid"),
        DisplayName::operator("nmid",       "∤", "Does not divide").latex("\\nmid"),
        DisplayName::operator("triangleq",  "≜", "Equal by definition").latex("\\triangleq"),
        DisplayName::operator("doteq",      "≐", "Dot equals").latex("\\doteq"),
        DisplayName::operator("asymp",      "≍", "Asymptotically equivalent").latex("\\asymp"),

        // Sets
        DisplayName::operator("subseteq",      "⊆", "Subset or equal").latex("\\subseteq"),
        DisplayName::operator("supseteq",      "⊇", "Superset or equal").latex("\\supseteq"),
        DisplayName::operator("supset",        "⊃", "Proper superset").latex("\\supset"),
        DisplayName::operator("subsetneq",     "⊊", "Proper subset, strictly").latex("\\subsetneq"),
        DisplayName::operator("supsetneq",     "⊋", "Proper superset, strictly").latex("\\supsetneq"),
        DisplayName::operator("ni",            "∋", "Contains as element").latex("\\ni"),
        DisplayName::operator("cup",           "∪", "Set union").latex("\\cup"),
        DisplayName::operator("cap",           "∩", "Set intersection").latex("\\cap"),
        DisplayName::operator("intersection",  "∩", "Set intersection").latex("\\cap"),
        DisplayName::operator("setminus",      "∖", "Set difference").latex("\\setminus"),
        DisplayName::operator("sqcup",         "⊔", "Square union (disjoint union, join)").latex("\\sqcup"),
        DisplayName::operator("sqcap",         "⊓", "Square intersection (meet)").latex("\\sqcap"),
        DisplayName::operator("bigcup",        "⋃", "Large union").latex("\\bigcup"),
        DisplayName::operator("bigcap",        "⋂", "Large intersection").latex("\\bigcap"),
        DisplayName::operator("sqsubseteq",    "⊑", "Square subset or equal (subtyping, information order)").latex("\\sqsubseteq"),
        DisplayName::operator("sqsupseteq",    "⊒", "Square superset or equal").latex("\\sqsupseteq"),

        // Logic and type theory
        DisplayName::operator("turnstile",  "⊢", "Turnstile: proves/yields (syntactic entailment)").latex("\\vdash"),
        DisplayName::operator("vdash",      "⊢", "Turnstile: proves/yields (alias for turnstile)").latex("\\vdash"),
        DisplayName::operator("nvdash",     "⊬", "Does not prove").latex("\\nvdash"),
        DisplayName::operator("dashv",      "⊣", "Reverse turnstile").latex("\\dashv"),
        DisplayName::operator("models",     "⊨", "Models / semantic entailment").latex("\\models"),
        DisplayName::operator("vDash",      "⊨", "Models / semantic entailment (LaTeX name)").latex("\\vDash"),
        DisplayName::operator("Vdash",      "⊩", "Forces").latex("\\Vdash"),
        DisplayName::operator("top",        "⊤", "Top / true").latex("\\top"),
        DisplayName::operator("bot",        "⊥", "Bottom / false / contradiction").latex("\\bot"),
        DisplayName::operator("land",       "∧", "Logical and").latex("\\land"),
        DisplayName::operator("lor",        "∨", "Logical or").latex("\\lor"),
        DisplayName::operator("lnot",       "¬", "Logical not").latex("\\lnot"),
        DisplayName::operator("neg",        "¬", "Logical negation").latex("\\neg"),
        DisplayName::operator("wedge",      "∧", "Wedge (and, meet, exterior product)").latex("\\wedge"),
        DisplayName::operator("vee",        "∨", "Vee (or, join)").latex("\\vee"),
        DisplayName::operator("therefore",  "∴", "Therefore (logical conclusion)").latex("\\therefore"),
        DisplayName::operator("because",    "∵", "Because").latex("\\because"),
        DisplayName::operator("qed",        "∎", "End of proof").latex("\\blacksquare"),

        // Binary operators
        DisplayName::operator("times",          "×", "Multiplication/cross product sign").latex("\\times"),
        DisplayName::operator("divide",         "÷", "Division sign").latex("\\div"),
        DisplayName::operator("plusminus",      "±", "Plus-minus sign").latex("\\pm"),
        DisplayName::operator("pm",             "±", "Plus-minus sign (LaTeX name)").latex("\\pm"),
        DisplayName::operator("mp",             "∓", "Minus-plus sign").latex("\\mp"),
        DisplayName::operator("circ",           "∘", "Ring operator: function/substitution composition").latex("\\circ"),
        DisplayName::operator("cdot",           "⋅", "Centered dot (dot product, multiplication)").latex("\\cdot"),
        DisplayName::operator("bullet",         "∙", "Bullet operator").latex("\\bullet"),
        DisplayName::operator("star",           "⋆", "Star operator (convolution, Hodge star)").latex("\\star"),
        DisplayName::operator("ast",            "∗", "Asterisk operator").latex("\\ast"),
        DisplayName::operator("oplus",          "⊕", "Circled plus (direct sum, xor)").latex("\\oplus"),
        DisplayName::operator("ominus",         "⊖", "Circled minus").latex("\\ominus"),
        DisplayName::operator("otimes",         "⊗", "Circled times (tensor product)").latex("\\otimes"),
        DisplayName::operator("oslash",         "⊘", "Circled slash").latex("\\oslash"),
        DisplayName::operator("odot",           "⊙", "Circled dot").latex("\\odot"),
        DisplayName::operator("dagger",         "†", "Dagger (Hermitian adjoint)").latex("\\dagger"),
        DisplayName::operator("ddagger",        "‡", "Double dagger").latex("\\ddagger"),
        DisplayName::operator("wr",             "≀", "Wreath product").latex("\\wr"),
        DisplayName::operator("amalg",          "⨿", "Coproduct / amalgamation").latex("\\amalg"),
        DisplayName::operator("triangleleft",   "◁", "Left triangle (normal subgroup)").latex("\\triangleleft"),
        DisplayName::operator("triangleright",  "▷", "Right triangle").latex("\\triangleright"),
        DisplayName::operator("ltimes",         "⋉", "Left semidirect product").latex("\\ltimes"),
        DisplayName::operator("rtimes",         "⋊", "Right semidirect product").latex("\\rtimes"),
        DisplayName::operator("langle",         "⟨", "Left angle bracket (bra, inner product)").latex("\\langle"),
        DisplayName::operator("rangle",         "⟩", "Right angle bracket (ket, inner product)").latex("\\rangle"),

        // Dots
        DisplayName::operator("ellipsis",  "…", "Horizontal ellipsis: continuation dots").latex("\\ldots"),
        DisplayName::operator("ldots",     "…", "Low horizontal dots").latex("\\ldots"),
        DisplayName::operator("cdots",     "⋯", "Centered horizontal dots").latex("\\cdots"),
        DisplayName::operator("vdots",     "⋮", "Vertical dots").latex("\\vdots"),
        DisplayName::operator("ddots",     "⋱", "Diagonal dots").latex("\\ddots"),

        // Calculus and misc
        DisplayName::operator("oint",      "∮", "Contour integral sign").latex("\\oint"),
        DisplayName::operator("iint",      "∬", "Double integral sign").latex("\\iint"),
        DisplayName::operator("iiint",     "∭", "Triple integral sign").latex("\\iiint"),
        DisplayName::operator("triangle",  "△", "Triangle (Laplacian, small change)").latex("\\triangle"),
        DisplayName::operator("square",    "□", "Square (d'Alembertian)").latex("\\square"),
        DisplayName::operator("diamond",   "⋄", "Diamond (modal possibility)").latex("\\diamond"),
    ]
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

    const KEYWORDS: &[&str] = &[
        "+", "add", "-", "sub", "*", "mul", "/", "div", "^", "pow", "frac", "sqrt", "root", "jux",
        "juxtapose", "=", "eq", "!=", "neq", ">", "gt", "<", "lt", "approx", "≈", ">=", "geq", "<=",
        "leq", "integral", "sum", "prod", "product", "lim", "limit", "derivative", "deriv",
        "partial", "dd", "sin", "cos", "tan", "cot", "sec", "cosec", "csc", "arcsin", "asin",
        "arccos", "acos", "arctan", "atan", "ln", "log", "exp", "abs", "floor", "ceil", "fact",
        "factorial", "binom", "nCr", "mat", "matrix", "vec", "vector", "det", "determinant",
        "table", "ty-judge", "ty-judgement", "set", "in", "notin", "subset", "superset", "union",
        "intersect", "and", "or", "not", "implies", "iff", "forall", "exists", "paren", "bracket",
        "brace", "text", "subscript", "superscript", "bar", "overline", "hat", "dot", "ddot",
        "arrow", "box", "prime", "funcapp", "fapp", "evalat", "eval-at", "quote", "bb", "cal",
        "frak", "bold", "rm", "cases", "show", "eval", "let", "alias", "macro", "ref", "tag",
    ];

    #[test]
    fn no_duplicate_names() {
        let mut seen = std::collections::HashSet::new();
        for d in display_names() {
            assert!(seen.insert(d.name), "duplicate display name `{}`", d.name);
        }
    }

    #[test]
    fn rows_are_complete() {
        for d in display_names() {
            assert!(!d.name.is_empty(), "empty name");
            assert!(!d.unicode.is_empty(), "`{}` has no unicode", d.name);
            assert!(!d.doc.is_empty(), "`{}` has no doc", d.name);
        }
    }

    #[test]
    fn no_keyword_clash() {
        for d in display_names() {
            assert!(!KEYWORDS.contains(&d.name), "`{}` is a keyword name", d.name);
        }
    }

    #[cfg(feature = "latex")]
    #[test]
    fn every_row_has_latex() {
        for d in display_names() {
            assert!(!d.latex.is_empty(), "`{}` has no latex", d.name);
            assert!(
                d.latex.starts_with('\\') || d.latex.starts_with('^') || d.latex.is_ascii(),
                "`{}` has odd latex `{}`",
                d.name,
                d.latex
            );
        }
    }

    #[test]
    fn lookup() {
        let tau = display_name(Sym::new("tau")).expect("tau is in the table");
        assert_eq!(tau.unicode, "τ");
        assert!(display_name(Sym::new("no-such-display-name")).is_none());
        let n = display_names().len();
        assert_eq!(n, TABLE.len());
        assert!(n > 100);
    }

    #[test]
    fn classes() {
        let class = |n: &str| display_name(Sym::new(n)).expect(n).class;
        assert_eq!(class("tau"), Class::Letter);
        assert_eq!(class("hbar"), Class::Letter);
        assert_eq!(class("turnstile"), Class::Operator);
        assert_eq!(class("mapsto"), Class::Operator);
    }
}
