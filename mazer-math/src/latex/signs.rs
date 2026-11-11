//! Spellings: the LaTeX for each keyword sign, for delimiters, and the
//! escaping rules for text and names.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::display::display_names;

/// The LaTeX spelling of a keyword sign (the `MathML` glyph, e.g. `∫`). Every
/// sign in the registry has an entry; a test checks it.
pub(super) fn sign(sign: &str) -> Option<&'static str> {
    Some(match sign {
        "+" => "+",
        "\u{2212}" => "-",
        "*" => "*",
        "\u{f7}" => "\\div",
        "\u{d7}" => "\\times",
        "=" => "=",
        "\u{2260}" => "\\neq",
        "<" => "<",
        ">" => ">",
        "\u{2264}" => "\\leq",
        "\u{2265}" => "\\geq",
        "\u{2248}" => "\\approx",
        "\u{2208}" => "\\in",
        "\u{2209}" => "\\notin",
        "\u{2282}" => "\\subset",
        "\u{2286}" => "\\subseteq",
        "\u{2287}" => "\\supseteq",
        "\u{222a}" => "\\cup",
        "\u{2229}" => "\\cap",
        "\u{2227}" => "\\land",
        "\u{2228}" => "\\lor",
        "\u{ac}" => "\\lnot",
        "\u{27f9}" => "\\implies",
        "\u{27fa}" => "\\iff",
        "\u{2200}" => "\\forall",
        "\u{2203}" => "\\exists",
        "\u{2192}" => "\\to",
        "\u{222b}" => "\\int",
        "\u{2211}" => "\\sum",
        "\u{220f}" => "\\prod",
        "\u{2202}" => "\\partial",
        "!" => "!",
        // Braced base: a bare prime after `\sum_{i}` would be a second superscript.
        "\u{2032}" => "{}'",
        "lim" => "\\lim",
        "sin" => "\\sin",
        "cos" => "\\cos",
        "tan" => "\\tan",
        "cot" => "\\cot",
        "sec" => "\\sec",
        "cosec" => "\\operatorname{cosec}",
        "arcsin" => "\\arcsin",
        "arccos" => "\\arccos",
        "arctan" => "\\arctan",
        "ln" => "\\ln",
        "log" => "\\log",
        "exp" => "\\exp",
        _ => return None,
    })
}

/// The LaTeX for a `\left` / `\right` delimiter. `""` is the invisible one.
pub(super) fn delimiter(delim: &str) -> String {
    match delim {
        "" | "." => ".".to_owned(),
        "{" => "\\{".to_owned(),
        "}" => "\\}".to_owned(),
        "\u{230a}" => "\\lfloor".to_owned(),
        "\u{230b}" => "\\rfloor".to_owned(),
        "\u{2308}" => "\\lceil".to_owned(),
        "\u{2309}" => "\\rceil".to_owned(),
        "\u{27e8}" => "\\langle".to_owned(),
        "\u{27e9}" => "\\rangle".to_owned(),
        other => other.to_owned(),
    }
}

/// A display name's LaTeX, safe to use as a script base: a spelling that
/// starts with a script (`^\circ`, the degree sign) gets an empty base.
pub(super) fn display_latex(latex: &str) -> String {
    if latex.starts_with(['^', '_']) {
        format!("{{{{}}{latex}}}")
    } else {
        latex.to_owned()
    }
}

/// Display names by what they show (`α`), for names typed as the character.
static BY_GLYPH: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for display in display_names() {
        if !display.latex.is_empty() {
            map.entry(display.unicode).or_insert(display.latex);
        }
    }
    map
});

/// A name as math: ASCII characters as they are (specials escaped), each
/// character that is a display-table glyph as its command (`α` -> `\alpha`,
/// `→` -> `\to`, `⊢` -> `\vdash`), and any other non-ASCII character wrapped
/// in `\text{...}`, so that the output is ASCII outside `\text`.
///
/// pdflatex itself cannot typeset such text without a Unicode-capable setup
/// (a font with the character, or `LuaLaTeX` / `XeLaTeX`); the wrapper only keeps
/// the rest of the document compiling.
pub(super) fn spell(text: &str) -> String {
    fn flush(out: &mut String, pending: &mut String) {
        if !pending.is_empty() {
            out.push_str("\\text{");
            out.push_str(pending);
            out.push('}');
            pending.clear();
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut pending = String::new();
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let glyph: &str = ch.encode_utf8(&mut buf);
        let piece = if ch.is_ascii() {
            escape_math(glyph)
        } else if let Some(latex) = BY_GLYPH.get(glyph) {
            display_latex(latex)
        } else {
            pending.push(ch);
            continue;
        };
        flush(&mut out, &mut pending);
        // A control word must not swallow a letter that follows it.
        if piece.starts_with(|c: char| c.is_ascii_alphabetic()) && super::ends_in_control_word(&out) {
            out.push(' ');
        }
        out.push_str(&piece);
    }
    flush(&mut out, &mut pending);
    out
}

/// Escapes `text` for inside `\text{...}`: `\ { } $ & # _ % ^ ~` and the
/// characters that print wrongly in text fonts (`< > |`).
pub(super) fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '$' => out.push_str("\\$"),
            '&' => out.push_str("\\&"),
            '#' => out.push_str("\\#"),
            '_' => out.push_str("\\_"),
            '%' => out.push_str("\\%"),
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '<' => out.push_str("\\textless{}"),
            '>' => out.push_str("\\textgreater{}"),
            '|' => out.push_str("\\textbar{}"),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Escapes a name for math mode (inside `\mathrm{...}` or on its own).
pub(super) fn escape_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\backslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '$' => out.push_str("\\$"),
            '&' => out.push_str("\\&"),
            '#' => out.push_str("\\#"),
            '_' => out.push_str("\\_"),
            '%' => out.push_str("\\%"),
            '^' => out.push_str("\\wedge{}"),
            '~' => out.push_str("\\sim{}"),
            c if c.is_control() || c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}
