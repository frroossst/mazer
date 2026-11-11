//! Signatures and markdown docs for keywords, display names, macros and
//! reserved words.
//!
//! Shared by completion, hover and signature help so they always agree.

use std::fmt::Write as _;

use mazer_doc::Macro;
use mazer_doc::math::{Class, DisplayName, Keyword, ParamKind};

/// The fence language for code in markdown. Scheme is the closest common
/// highlighter to mazer's Lisp.
pub const FENCE: &str = "scheme";

/// One parameter of a [`Signature`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigParam {
    /// How it is written in the signature: `body`, `[lower]`, `terms...`.
    pub text: String,
    pub doc: String,
    pub kind: ParamKind,
}

/// A callable form: a keyword, a macro or a reserved word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    pub params: Vec<SigParam>,
    pub doc: String,
}

impl Signature {
    /// The signature as written, e.g. `(integral body var [lower] [upper])`,
    /// and the UTF-16 offsets of each parameter in it.
    #[must_use]
    pub fn label(&self) -> (String, Vec<[u32; 2]>) {
        let mut out = format!("({}", self.name);
        let mut offsets = Vec::with_capacity(self.params.len());
        for p in &self.params {
            out.push(' ');
            let start = crate::convert::utf16_len(&out);
            out.push_str(&p.text);
            offsets.push([start, crate::convert::utf16_len(&out)]);
        }
        out.push(')');
        (out, offsets)
    }

    /// Which parameter the argument at `index` binds to when the call has
    /// `count` arguments, by the rule of `mazer_math::bind`: required
    /// parameters take arguments first, spare arguments fill optional ones
    /// left to right, the rest parameter takes whatever is left. `None` when
    /// the argument is one too many.
    #[must_use]
    pub fn active(&self, count: usize, index: usize) -> Option<usize> {
        let count = count.max(index + 1);
        let required = self.params.iter().filter(|p| p.kind == ParamKind::Required).count();
        let mut spare = count.saturating_sub(required);
        let mut slot = 0;
        for (i, param) in self.params.iter().enumerate() {
            match param.kind {
                ParamKind::Required => {}
                ParamKind::Optional if spare > 0 => spare -= 1,
                ParamKind::Optional => continue,
                ParamKind::Rest => return Some(i),
            }
            if slot == index {
                return Some(i);
            }
            slot += 1;
        }
        None
    }

    /// A snippet that writes a call's head and its required parameters as
    /// tab stops, e.g. `frac ${1:num} ${2:den}`.
    #[must_use]
    pub fn snippet(&self, name: &str) -> String {
        let mut out = escape_snippet(name);
        let required = self.params.iter().filter(|p| p.kind == ParamKind::Required);
        for (stop, p) in (1..).zip(required) {
            let _ = write!(out, " ${{{stop}:{}}}", escape_snippet(&p.text));
        }
        out
    }

    /// Whether the call takes any required argument.
    #[must_use]
    pub fn has_required(&self) -> bool {
        self.params.iter().any(|p| p.kind == ParamKind::Required)
    }
}

/// Escapes the characters that are special in an LSP snippet.
#[must_use]
pub fn escape_snippet(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '$' | '}' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// A keyword's signature.
#[must_use]
pub fn keyword_signature(keyword: &Keyword, called_as: &str) -> Signature {
    Signature {
        name: called_as.to_owned(),
        params: keyword
            .params
            .iter()
            .map(|p| SigParam {
                text: match p.kind {
                    ParamKind::Required => p.name.to_owned(),
                    ParamKind::Optional => format!("[{}]", p.name),
                    ParamKind::Rest => format!("{}...", p.name),
                },
                doc: p.doc.to_owned(),
                kind: p.kind,
            })
            .collect(),
        doc: keyword.doc.to_owned(),
    }
}

/// A macro's signature (an alias has no parameters).
#[must_use]
pub fn macro_signature(def: &Macro) -> Signature {
    let mut params: Vec<SigParam> = def
        .params
        .iter()
        .map(|p| SigParam {
            text: p.as_str().to_owned(),
            doc: String::new(),
            kind: ParamKind::Required,
        })
        .collect();
    if let Some(rest) = def.rest {
        params.push(SigParam {
            text: format!("{rest}..."),
            doc: "Any number of arguments, spliced into the template.".to_owned(),
            kind: ParamKind::Rest,
        });
    }
    Signature {
        name: def.name.as_str().to_owned(),
        params,
        doc: def.doc.as_deref().unwrap_or("").to_owned(),
    }
}

/// The reserved words, with a one-line description each, in the order they
/// are offered.
pub const RESERVED: &[(&str, &str)] = &[
    ("show", "Show math: a display equation alone in its paragraph, inline math in a sentence."),
    ("let", "Bind a variable; later `show`, `eval` and `let` use its value."),
    ("alias", "Define notation with no parameters: every use of the name is replaced."),
    ("macro", "Define notation with parameters: `(name args...)` is replaced by the template."),
    ("ref", "A link to a tagged equation, shown as its number."),
    ("eval", "Evaluate an expression and show the result (needs an evaluator)."),
];

/// The hand-written signature of a reserved word (or `tag`).
#[must_use]
pub fn reserved_signature(name: &str) -> Option<Signature> {
    let p = |text: &str, doc: &str, kind| SigParam {
        text: text.to_owned(),
        doc: doc.to_owned(),
        kind,
    };
    let (params, doc) = match name {
        "show" => (
            vec![
                p(
                    "[(tag name)]",
                    "Numbers a display equation so `(ref name)` can link to it. Only as the first item of a show alone in its paragraph.",
                    ParamKind::Optional,
                ),
                p("expr...", "What to show. Several expressions are written side by side: `(show a b)` is `(show (jux a b))`.", ParamKind::Rest),
            ],
            "Shows math. Alone in its paragraph it is a display equation (which a `(tag name)` can number); inside a sentence it is inline math. `let` variables are substituted, notation is expanded; nothing is evaluated.",
        ),
        "eval" => (
            vec![p("expr", "The expression to evaluate.", ParamKind::Required)],
            "Evaluates `expr` with the configured evaluator and shows the result like a `show`. Must stand on its own, not inside another form. There is no evaluator in the editor or the website build, so it is not evaluated here.",
        ),
        "let" => (
            vec![
                p("name", "The variable. May be rebound later; a display name such as `tau` is allowed.", ParamKind::Required),
                p("value", "Its value, kept unevaluated. Earlier variables in it are substituted now.", ParamKind::Required),
            ],
            "Binds a variable from here on: every later `show`, `eval` and `let` substitutes its value. `'name` shows the name itself. Produces no output.",
        ),
        "alias" => (
            vec![
                p("name", "The new name. Not a keyword, display name or reserved word.", ParamKind::Required),
                p("expansion", "What every use of the name is replaced by, as written.", ParamKind::Required),
            ],
            "Notation with no parameters: every use of `name`, anywhere in the note, is replaced by `expansion`. Produces no output.",
        ),
        "macro" => (
            vec![
                p("(name params...)", "The name and its parameters; a last `rest...` collects the remaining arguments.", ParamKind::Required),
                p("[\"doc\"]", "An optional doc string, shown on hover.", ParamKind::Optional),
                p("body", "The template. Each parameter is replaced by its argument as written.", ParamKind::Required),
            ],
            "Notation with parameters: `(name a b)` is replaced by `body` with the parameters substituted. Applies to the whole note. Produces no output.",
        ),
        "ref" => (
            vec![p("name", "The name given in `(tag name)`.", ParamKind::Required)],
            "A link to the equation tagged `name`, shown as its number, e.g. (2). Prose only.",
        ),
        "tag" => (
            vec![p("name", "The label; `(ref name)` links here.", ParamKind::Required)],
            "Numbers a display equation. Only as the first item of a `show` that stands alone in its paragraph.",
        ),
        _ => return None,
    };
    Some(Signature {
        name: name.to_owned(),
        params,
        doc: doc.to_owned(),
    })
}

/// A snippet for a reserved word typed after `(`.
#[must_use]
#[allow(clippy::literal_string_with_formatting_args)] // `${1:name}` is snippet syntax, not a format string
pub fn reserved_snippet(name: &str) -> String {
    match name {
        "show" => "show ${1:expr}",
        "eval" => "eval ${1:expr}",
        "let" => "let ${1:name} ${2:value}",
        "alias" => "alias ${1:name} ${2:expansion}",
        "macro" => "macro (${1:name} ${2:params}) ${3:body}",
        "ref" => "ref ${1:name}",
        "tag" => "tag ${1:name}",
        _ => name,
    }
    .to_owned()
}

/// Markdown for a signature: the call in a code block, then the doc, then
/// the parameters.
fn signature_markdown(sig: &Signature, out: &mut String) {
    let _ = writeln!(out, "```{FENCE}\n{}\n```", sig.label().0);
    if !sig.doc.is_empty() {
        let _ = writeln!(out, "\n{}", sig.doc);
    }
    if !sig.params.is_empty() {
        out.push_str("\n**Parameters**\n");
        for p in &sig.params {
            let note = match p.kind {
                ParamKind::Required => "",
                ParamKind::Optional => " (optional)",
                ParamKind::Rest => " (any number)",
            };
            if p.doc.is_empty() {
                let _ = writeln!(out, "- `{}`{note}", p.text);
            } else {
                let _ = writeln!(out, "- `{}`{note}: {}", p.text, p.doc);
            }
        }
    }
}

/// Full markdown documentation of a keyword: signature, doc, sign,
/// parameters, other names and examples. `called_as` is the name the user
/// wrote (a second name is mentioned as such).
#[must_use]
pub fn keyword_markdown(keyword: &Keyword, called_as: &str) -> String {
    let mut out = String::new();
    signature_markdown(&keyword_signature(keyword, keyword.name), &mut out);
    if called_as != keyword.name {
        let _ = writeln!(out, "\n`{called_as}` is another name for `{}`.", keyword.name);
    }
    if let Some(sign) = keyword.sign {
        let _ = writeln!(out, "\nWritten bare, `{called_as}` shows **{sign}**.");
    }
    let others: Vec<String> = keyword.names().filter(|n| *n != called_as).map(|n| format!("`{n}`")).collect();
    if !others.is_empty() && called_as == keyword.name {
        let _ = writeln!(out, "\nAlso called {}.", others.join(", "));
    }
    if !keyword.examples.is_empty() {
        let _ = writeln!(out, "\n**Examples**\n```{FENCE}");
        for example in &keyword.examples {
            let _ = writeln!(out, "{example}");
        }
        out.push_str("```\n");
    }
    out
}

/// Markdown for a display name: the character big, then its name and doc.
#[must_use]
pub fn display_markdown(display: &DisplayName) -> String {
    let class = match display.class {
        Class::Letter => "a letter",
        Class::Operator => "an operator (spaced like one)",
    };
    format!(
        "# {}\n\n`{}` shows as **{}**: {}.\n\nRendered as {class}. It is an ordinary variable; only its display is special.",
        display.unicode,
        display.name,
        display.unicode,
        display.doc.trim_end_matches('.'),
    )
}

/// Markdown for a macro or alias, with `place` saying where it is defined.
#[must_use]
pub fn macro_markdown(def: &Macro, place: &str) -> String {
    let mut out = String::new();
    let kind = if def.is_alias() { "alias" } else { "macro" };
    let _ = writeln!(out, "```{FENCE}\n{}\n```", def.signature());
    if let Some(doc) = def.doc.as_deref() {
        let _ = writeln!(out, "\n{doc}");
    }
    let _ = writeln!(out, "\nNotation ({kind}), expands to:\n```{FENCE}\n{}\n```", def.body);
    let _ = writeln!(out, "\nDefined {place}.");
    out
}

/// Markdown for a reserved word.
#[must_use]
pub fn reserved_markdown(name: &str) -> String {
    let mut out = String::new();
    if let Some(sig) = reserved_signature(name) {
        signature_markdown(&sig, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use mazer_doc::Sym;
    use mazer_doc::math::registry;

    use super::*;

    fn kw(name: &str) -> &'static Keyword {
        registry().get(Sym::new(name)).unwrap()
    }

    #[test]
    fn integral_binding() {
        let sig = keyword_signature(kw("integral"), "integral");
        // body var [lower] [upper]
        assert_eq!(sig.active(0, 0), Some(0));
        assert_eq!(sig.active(1, 1), Some(1));
        assert_eq!(sig.active(2, 2), Some(2), "a third argument is the lower bound");
        assert_eq!(sig.active(3, 2), Some(2));
        assert_eq!(sig.active(4, 3), Some(3));
        assert_eq!(sig.active(5, 4), None, "a fifth argument is one too many");
        let (label, offsets) = sig.label();
        assert_eq!(label, "(integral body var [lower] [upper])");
        let [a, b] = offsets[2];
        assert_eq!(&label[a as usize..b as usize], "[lower]");
    }

    #[test]
    fn rest_and_optional_in_the_middle() {
        let sig = reserved_signature("macro").unwrap();
        // (name params...) ["doc"] body: with two arguments the second is the body.
        assert_eq!(sig.active(2, 1), Some(2));
        assert_eq!(sig.active(3, 1), Some(1));
        assert_eq!(sig.active(3, 2), Some(2));
        let show = reserved_signature("show").unwrap();
        // A rest parameter stays active.
        assert_eq!(show.active(1, 5), Some(1));
        let plus = keyword_signature(kw("+"), "+");
        assert!(plus.params.iter().any(|p| p.kind == ParamKind::Rest));
        assert_eq!(plus.active(7, 6), Some(plus.params.len() - 1));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)] // snippet syntax
    fn snippets() {
        let sig = keyword_signature(kw("frac"), "frac");
        assert_eq!(sig.snippet("frac"), "frac ${1:num} ${2:den}");
        assert_eq!(escape_snippet("a$b}c\\"), "a\\$b\\}c\\\\");
    }

    #[test]
    fn keyword_docs_mention_everything() {
        let md = keyword_markdown(kw("integral"), "integral");
        assert!(md.contains("(integral body var [lower] [upper])"), "{md}");
        assert!(md.contains("∫"));
        assert!(md.contains("**Parameters**"));
        assert!(md.contains("**Examples**"));
    }
}
