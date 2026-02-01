use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::sync::LazyLock;

static LETTERS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        // greek symbols
        ("alpha", "α"),
        ("Alpha", "Α"),
        ("beta", "β"),
        ("Beta", "Β"),
        ("gamma", "γ"),
        ("Gamma", "Γ"),
        ("delta", "δ"),
        ("Delta", "Δ"),
        ("epsilon", "ε"),
        ("Epsilon", "Ε"),
        ("zeta", "ζ"),
        ("Zeta", "Ζ"),
        ("eta", "η"),
        ("Eta", "Η"),
        ("theta", "θ"),
        ("Theta", "Θ"),
        ("iota", "ι"),
        ("Iota", "Ι"),
        ("kappa", "κ"),
        ("Kappa", "Κ"),
        ("lambda", "λ"),
        ("Lambda", "Λ"),
        ("mu", "μ"),
        ("Mu", "Μ"),
        ("nu", "ν"),
        ("Nu", "Ν"),
        ("xi", "ξ"),
        ("Xi", "Ξ"),
        ("omicron", "ο"),
        ("Omicron", "Ο"),
        ("pi", "π"),
        ("Pi", "Π"),
        ("rho", "ρ"),
        ("Rho", "Ρ"),
        ("sigma", "σ"),
        ("Sigma", "Σ"),
        ("tau", "τ"),
        ("Tau", "Τ"),
        ("upsilon", "υ"),
        ("Upsilon", "Υ"),
        ("phi", "φ"),
        ("Phi", "Φ"),
        ("chi", "χ"),
        ("Chi", "Χ"),
        ("psi", "ψ"),
        ("Psi", "Ψ"),
        ("omega", "ω"),
        ("Omega", "Ω"),
        // more math symbols
        ("infinity", "∞"),
        ("partial", "∂"),
        ("nabla", "∇"),
        ("therefore", "∴"),
        ("angle", "∠"),
        ("degree", "°"),
        ("plusminus", "±"),
        ("times", "×"),
        ("divide", "÷"),
        ("approx", "≈"),
        ("neq", "≠"),
        ("leq", "≤"),
        ("geq", "≥"),
        ("equiv", "≡"),
        ("subset", "⊂"),
        ("supset", "⊃"),
        ("in", "∈"),
        ("notin", "∉"),
        ("union", "∪"),
        ("intersection", "∩"),
        ("forall", "∀"),
        ("exists", "∃"),
        ("emptyset", "∅"),
        ("arrowup", "↑"),
        ("arrowdown", "↓"),
        ("arrowleft", "←"),
        ("arrowright", "→"),
        ("arrow", "→"),
        ("ellipsis", "…"),
        ("Laplace", "𝓛"),
        ("Reals", "ℝ"),
        ("reals", "ℝ"),
    ])
});

pub struct Atog;

impl Atog {
    pub fn get(s: &str) -> Option<&str> {
        LETTERS.get(s).copied()
    }
}

impl IntoIterator for Atog {
    type Item = (&'static str, &'static str);
    type IntoIter = IntoIter<&'static str, &'static str>;

    fn into_iter(self) -> Self::IntoIter {
        LETTERS.clone().into_iter()
    }
}
