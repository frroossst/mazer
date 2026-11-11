//! The `MathML` emitter. Writes straight into one `String`; there is no
//! intermediate element tree.
//!
//! Text is escaped in exactly four places: [`MathMl::mi`], [`MathMl::mn`],
//! [`MathMl::mo`] and [`MathMl::mtext`] (plus attribute values in
//! [`MathMl::tag_attrs`]). Emitters never build a tag by hand.
//!
//! Every call to [`MathMl::expr`] writes exactly one element, so the result can
//! always be used as one child of `<mfrac>`, `<msub>`, `<msubsup>` and the
//! other elements that count their children. Keyword emitters keep the same
//! promise: when they write more than one element they wrap them in
//! [`MathMl::row`].

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Span, Sym};

use crate::call::try_bind;
use crate::display::{Class, display_name};
use crate::registry::registry;

/// A thin space (3/18 em), e.g. between an integrand and its `dx`.
pub(crate) const THIN_SPACE: &str = "0.167em";

/// How tightly an expression holds together, loosest first. A parent uses it
/// to decide whether a child needs parentheses: see [`MathMl::operand`].
///
/// Keyword files say which of their keywords are not [`Prec::Atom`] in their
/// own `precedence` function (see `keywords/mod.rs`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Prec {
    /// `⟹`, `⟺`, `∧`, `∨`.
    #[allow(dead_code)] // first used by the step 2 logic keywords
    Logic,
    /// `=`, `≠`, `<`, `≤`, `≈`, `∈`, `⊆`, ...
    #[allow(dead_code)] // first used by the step 2 comparison keywords
    Relation,
    /// `+`, binary `−`, `∪`.
    Additive,
    /// `×`, `÷`, `∩`, juxtaposition.
    Multiplicative,
    /// Unary `−`, unary `+`, `¬`, and a negative number literal.
    Prefix,
    /// Never needs parentheses: names, numbers, strings, fractions, scripts,
    /// roots, fences, function applications, named functions.
    Atom,
}

impl Prec {
    /// The next tighter level. For the operands after the first of a
    /// non-associative operator: `a − (b − c)` puts its right operand at
    /// `Prec::Additive.tighter()` so that a difference there gets parentheses.
    #[must_use]
    #[allow(dead_code)] // first used by the step 2 `-` and `/` keywords
    pub(crate) const fn tighter(self) -> Self {
        match self {
            Self::Logic => Self::Relation,
            Self::Relation => Self::Additive,
            Self::Additive => Self::Multiplicative,
            Self::Multiplicative => Self::Prefix,
            Self::Prefix | Self::Atom => Self::Atom,
        }
    }
}

/// How tightly `node` holds together once rendered.
pub(crate) fn precedence(node: &Node) -> Prec {
    match &node.kind {
        NodeKind::Number(text) if text.starts_with('-') => Prec::Prefix,
        NodeKind::List(items) => items
            .first()
            .and_then(Node::as_ident)
            .and_then(|head| registry().get(head))
            .map_or(Prec::Atom, |keyword| crate::keywords::precedence(keyword.name, items.len() - 1)),
        _ => Prec::Atom,
    }
}

pub struct MathMl<'a> {
    pub(crate) out: String,
    pub(crate) diags: &'a mut Diags,
}

impl<'a> MathMl<'a> {
    #[must_use]
    pub const fn new(diags: &'a mut Diags) -> Self {
        Self {
            out: String::new(),
            diags,
        }
    }

    /// The `MathML` written so far (no surrounding `<math>` element).
    #[must_use]
    pub fn finish(self) -> String {
        self.out
    }

    /// Renders any node. This is the recursive dispatcher:
    /// - `Number` -> `<mn>`
    /// - `Ident` -> a bare keyword's sign, else the display table, else the name itself
    /// - `Str` -> `<mtext>`
    /// - a list headed by a keyword -> bind the arguments, then that keyword's emitter
    /// - a list headed by anything else -> function application: `(f x y)` renders
    ///   as f(x, y); with no arguments, `(Gamma)` renders just the head. If the
    ///   head is one typo away from a keyword, also push an `UnknownForm` warning
    ///   with a did-you-mean.
    /// - `Error` -> `<merror>` (the reader already reported it)
    ///
    /// Details:
    /// - A negative number `-2` renders as a minus sign (U+2212) and `<mn>2</mn>`.
    /// - A name with no display form renders as `<mi>`, or as `<mo>` when it has
    ///   no letter or digit in it (`<>`, `|`, `::`).
    /// - A keyword call with the wrong number of arguments renders as an
    ///   `<merror>` holding the `Arity` message.
    /// - The empty list `()` renders as an empty `<mrow>`.
    pub fn expr(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Number(text) => self.number(text),
            NodeKind::Str(text) => self.mtext(text),
            NodeKind::Ident(sym) => self.ident(*sym, node.span),
            NodeKind::List(items) => self.list(node, items),
            NodeKind::Error => self.merror("error"),
        }
    }

    /// Writes `<name>`, runs `body`, writes `</name>`. The closure guarantees
    /// the closing tag.
    pub fn tag(&mut self, name: &'static str, body: impl FnOnce(&mut Self)) {
        self.tag_attrs(name, &[], body);
    }

    /// Like [`MathMl::tag`] with attributes. Attribute values are escaped.
    pub fn tag_attrs(&mut self, name: &'static str, attrs: &[(&str, &str)], body: impl FnOnce(&mut Self)) {
        self.open(name, attrs);
        self.out.push('>');
        body(self);
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
    }

    /// An identifier: `<mi>text</mi>`, escaped.
    pub fn mi(&mut self, text: &str) {
        self.tag("mi", |o| o.escape(text));
    }

    /// A number: `<mn>text</mn>`, escaped.
    pub fn mn(&mut self, text: &str) {
        self.tag("mn", |o| o.escape(text));
    }

    /// An operator: `<mo>text</mo>`, escaped.
    pub fn mo(&mut self, text: &str) {
        self.tag("mo", |o| o.escape(text));
    }

    /// Plain text: `<mtext>text</mtext>`, escaped.
    pub fn mtext(&mut self, text: &str) {
        self.tag("mtext", |o| o.escape(text));
    }

    /// `<mrow> ... </mrow>`
    pub fn row(&mut self, body: impl FnOnce(&mut Self)) {
        self.tag("mrow", body);
    }

    /// `<mfrac>num den</mfrac>`
    pub fn frac(&mut self, num: &Node, den: &Node) {
        self.tag("mfrac", |o| {
            o.expr(num);
            o.expr(den);
        });
    }

    /// `<msub>base script</msub>`. A base that is not an atom (`a + b`, `−x`)
    /// is put in parentheses first.
    pub fn sub(&mut self, base: &Node, script: &Node) {
        self.tag("msub", |o| {
            o.operand(base, Prec::Atom);
            o.expr(script);
        });
    }

    /// `<msup>base script</msup>`. A base that is not an atom (`a + b`, `−x`)
    /// is put in parentheses first.
    pub fn sup(&mut self, base: &Node, script: &Node) {
        self.tag("msup", |o| {
            o.operand(base, Prec::Atom);
            o.expr(script);
        });
    }

    /// A row wrapped in stretchy delimiters. Either delimiter may be `""`.
    ///
    /// Writes `<mrow><mo>open</mo> body <mo>close</mo></mrow>`. Brackets,
    /// braces and bars are stretchy by default in `MathML`'s operator
    /// dictionary, so no attribute is needed.
    pub fn fence(&mut self, open: &str, close: &str, body: impl FnOnce(&mut Self)) {
        self.row(|o| {
            if !open.is_empty() {
                o.mo(open);
            }
            body(o);
            if !close.is_empty() {
                o.mo(close);
            }
        });
    }

    /// Reports a problem: pushes the diagnostic and writes a visible `<merror>`
    /// in its place so the rest of the expression still renders.
    pub fn error(&mut self, diag: Diag) {
        self.merror(&diag.message);
        self.diags.push(diag);
    }
}

/// Helpers for keyword emitters. Crate-internal: they are how the keywords in
/// `keywords/` are written, not part of the contract with other crates.
impl MathMl<'_> {
    /// Pushes a diagnostic without writing anything, for problems that do not
    /// stop the expression from rendering normally (warnings, hints).
    pub(crate) fn report(&mut self, diag: Diag) {
        self.diags.push(diag);
    }

    /// Renders `node`, in parentheses if it holds together more loosely than
    /// `min` (see [`Prec`]). `(* (+ a b) c)` renders its first operand with
    /// `operand(.., Prec::Multiplicative)`, which gives `(a + b) × c`.
    pub(crate) fn operand(&mut self, node: &Node, min: Prec) {
        if precedence(node) < min {
            self.fence("(", ")", |o| o.expr(node));
        } else {
            self.expr(node);
        }
    }

    /// An infix chain `a op b op c` in one `<mrow>`, each operand rendered with
    /// [`MathMl::operand`] at `prec`. Treats the operator as associative: an
    /// operand at the same level is not parenthesised. With a single operand
    /// it writes just that operand (still one `<mrow>`).
    pub(crate) fn infix(&mut self, op: &str, prec: Prec, operands: &[Node]) {
        self.row(|o| {
            for (i, operand) in operands.iter().enumerate() {
                if i > 0 {
                    o.mo(op);
                }
                o.operand(operand, prec);
            }
        });
    }

    /// Function application: `head(a, b, c)`. `head` writes the function
    /// (exactly one element); the arguments are rendered in a parenthesised,
    /// comma-separated [`MathMl::fence`]. With no arguments only the head is
    /// written. Named functions such as `sin` use this too.
    pub(crate) fn apply(&mut self, head: impl FnOnce(&mut Self), args: &[Node]) {
        if args.is_empty() {
            head(self);
            return;
        }
        self.row(|o| {
            head(o);
            o.fence("(", ")", |o| o.separated(",", args));
        });
    }

    /// The items with `<mo>sep</mo>` between them, written flat into the
    /// current element (no `<mrow>` of its own: call it inside a row or fence).
    pub(crate) fn separated(&mut self, sep: &str, items: &[Node]) {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.mo(sep);
            }
            self.expr(item);
        }
    }

    /// A variable name: its display form if it has one (`tau` -> `<mi>τ</mi>`,
    /// `to` -> `<mo>→</mo>`), else the name itself. Never treats the name as a
    /// keyword; `quote` uses this to show a name as written.
    pub(crate) fn name(&mut self, name: Sym) {
        if let Some(display) = display_name(name) {
            match display.class {
                Class::Letter => self.mi(display.unicode),
                Class::Operator => self.mo(display.unicode),
            }
        } else {
            let text = name.as_str();
            if text.chars().any(char::is_alphanumeric) {
                self.mi(text);
            } else {
                self.mo(text);
            }
        }
    }

    /// Horizontal space: `<mspace width="width"/>`, e.g. [`THIN_SPACE`].
    pub(crate) fn space(&mut self, width: &str) {
        self.open("mspace", &[("width", width)]);
        self.out.push_str("/>");
    }
}

/// The dispatcher's pieces.
impl MathMl<'_> {
    fn number(&mut self, text: &str) {
        if let Some(magnitude) = text.strip_prefix('-') {
            self.row(|o| {
                o.mo("\u{2212}");
                o.mn(magnitude);
            });
        } else {
            self.mn(text);
        }
    }

    fn ident(&mut self, name: Sym, span: Span) {
        let Some(keyword) = registry().get(name) else {
            self.name(name);
            return;
        };
        if let Some(sign) = keyword.sign {
            self.mo(sign);
        } else {
            self.error(
                Diag::error(DiagCode::BareKeyword, span, format!("`{name}` needs arguments"))
                    .with_help(format!("write it as {}", keyword.signature())),
            );
        }
    }

    fn list(&mut self, node: &Node, items: &[Node]) {
        let Some((head, args)) = items.split_first() else {
            // `()`: an empty placeholder, e.g. an argument left blank on purpose.
            self.row(|_| {});
            return;
        };
        if let Some(name) = head.as_ident() {
            if let Some(keyword) = registry().get(name) {
                match try_bind(keyword, node) {
                    Ok(call) => (keyword.mathml)(&call, self),
                    Err(diag) => self.error(diag),
                }
                return;
            }
            if display_name(name).is_none()
                && let Some(suggestion) = registry().suggest(name.as_str())
            {
                self.report(
                    Diag::warning(
                        DiagCode::UnknownForm,
                        head.span,
                        format!("`{name}` is not a keyword, so this is shown as a function application"),
                    )
                    .with_help(format!("did you mean `{suggestion}`?")),
                );
            }
        }
        self.apply(|o| o.expr(head), args);
    }

    /// `<merror><mtext>text</mtext></merror>`, without a diagnostic.
    fn merror(&mut self, text: &str) {
        self.tag("merror", |o| o.mtext(text));
    }

    /// Writes `<name a="v" ...` (no closing `>`), escaping the values.
    fn open(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.out.push('<');
        self.out.push_str(name);
        for (key, value) in attrs {
            self.out.push(' ');
            self.out.push_str(key);
            self.out.push_str("=\"");
            self.escape(value);
            self.out.push('"');
        }
    }

    /// Writes `text` with `&`, `<`, `>`, `"` and `'` replaced by entities.
    /// Safe both in element content and in a double-quoted attribute value.
    fn escape(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '"' => self.out.push_str("&quot;"),
                '\'' => self.out.push_str("&#39;"),
                _ => self.out.push(ch),
            }
        }
    }
}

/// Renders one expression to `MathML` (no surrounding `<math>` element).
pub fn to_mathml(node: &Node, diags: &mut Diags) -> String {
    let mut out = MathMl::new(diags);
    out.expr(node);
    out.finish()
}

/// Checks one expression (keyword argument counts, bare keywords, typo'd
/// heads) and pushes diagnostics, without keeping any output. Used by
/// `mazer_doc::analyze`, which the LSP calls on every edit.
///
/// It runs the renderer and throws the output away, so it reports exactly
/// what rendering would report.
pub fn check(node: &Node, diags: &mut Diags) {
    let mut out = MathMl::new(diags);
    out.expr(node);
}

#[cfg(test)]
mod tests {
    use mazer_syntax::{Diag, DiagCode, Diags, Node, Severity, Span};

    use super::{MathMl, check};
    use crate::testing::{mathml, read, render, top_level_elements};

    fn with(f: impl FnOnce(&mut MathMl<'_>)) -> (String, Diags) {
        let mut diags = Diags::new();
        let mut out = MathMl::new(&mut diags);
        f(&mut out);
        let out = out.finish();
        (out, diags)
    }

    #[test]
    fn text_is_escaped() {
        let (out, _) = with(|o| {
            o.mi("a<b");
            o.mn("1&2");
            o.mo(">");
            o.mtext("say \"hi\" & 'bye'");
        });
        assert_eq!(
            out,
            "<mi>a&lt;b</mi><mn>1&amp;2</mn><mo>&gt;</mo><mtext>say &quot;hi&quot; &amp; &#39;bye&#39;</mtext>"
        );
        let (out, _) = with(|o| o.tag_attrs("mstyle", &[("mathcolor", "a\"<b>&")], |o| o.mi("x")));
        assert_eq!(out, "<mstyle mathcolor=\"a&quot;&lt;b&gt;&amp;\"><mi>x</mi></mstyle>");
        // Strings and names from the source are escaped too.
        assert_eq!(mathml("\"x < \\\"y\\\"\""), "<mtext>x &lt; &quot;y&quot;</mtext>");
        assert_eq!(mathml("(f List<T>)"), "<mrow><mi>f</mi><mrow><mo>(</mo><mi>List&lt;T&gt;</mi><mo>)</mo></mrow></mrow>");
    }

    #[test]
    fn structure_helpers() {
        let (out, _) = with(|o| o.row(|o| o.fence("", "|", |o| o.mi("x"))));
        assert_eq!(out, "<mrow><mrow><mi>x</mi><mo>|</mo></mrow></mrow>");
        let (out, _) = with(|o| o.tag("msqrt", |_| {}));
        assert_eq!(out, "<msqrt></msqrt>");
        let (out, _) = with(|o| o.space(super::THIN_SPACE));
        assert_eq!(out, "<mspace width=\"0.167em\"/>");
    }

    #[test]
    fn atoms() {
        assert_eq!(mathml("3.14"), "<mn>3.14</mn>");
        assert_eq!(mathml("1e-9"), "<mn>1e-9</mn>");
        assert_eq!(mathml("-2"), "<mrow><mo>\u{2212}</mo><mn>2</mn></mrow>");
        assert_eq!(mathml("x"), "<mi>x</mi>");
        assert_eq!(mathml("gen"), "<mi>gen</mi>");
        // A name made only of symbols is an operator.
        assert_eq!(mathml("<>"), "<mo>&lt;&gt;</mo>");
        assert_eq!(mathml("\"hello world\""), "<mtext>hello world</mtext>");
        // The empty placeholder still writes one (empty) element.
        assert_eq!(mathml("()"), "<mrow></mrow>");
    }

    #[test]
    fn reader_error_node() {
        let mut diags = Diags::new();
        let out = crate::to_mathml(&Node::error(Span::DUMMY), &mut diags);
        assert_eq!(out, "<merror><mtext>error</mtext></merror>");
        assert!(diags.is_empty(), "the reader already reported it");
    }

    #[test]
    fn function_application() {
        assert_eq!(
            mathml("(f x)"),
            "<mrow><mi>f</mi><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        // `(gen (G) t)`: a zero-argument application is just its head.
        assert_eq!(
            mathml("(gen (G) t)"),
            "<mrow><mi>gen</mi><mrow><mo>(</mo><mi>G</mi><mo>,</mo><mi>t</mi><mo>)</mo></mrow></mrow>"
        );
        assert_eq!(mathml("(G)"), "<mi>G</mi>");
        // The head may be any expression.
        assert_eq!(
            mathml("((subscript f 1) x)"),
            "<mrow><msub><mi>f</mi><mn>1</mn></msub><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        // `()` as an argument leaves it blank: f().
        assert_eq!(
            mathml("(f ())"),
            "<mrow><mi>f</mi><mrow><mo>(</mo><mrow></mrow><mo>)</mo></mrow></mrow>"
        );
        // Keyword calls inside arguments still dispatch.
        assert_eq!(
            mathml("(f (frac 1 2) y)"),
            "<mrow><mi>f</mi><mrow><mo>(</mo><mfrac><mn>1</mn><mn>2</mn></mfrac><mo>,</mo><mi>y</mi><mo>)</mo></mrow></mrow>"
        );
    }

    #[test]
    fn bare_keyword() {
        // With a sign: the sign, as an operator.
        assert_eq!(mathml("integral"), "<mo>∫</mo>");
        assert_eq!(mathml("add"), "<mo>+</mo>", "aliases work too");
        // Without one: a diagnostic and a visible error.
        let src = "(f frac)";
        let (out, diags) = render(src);
        assert_eq!(
            out,
            "<mrow><mi>f</mi><mrow><mo>(</mo><merror><mtext>`frac` needs arguments</mtext></merror><mo>)</mo></mrow></mrow>"
        );
        let [diag] = diags.as_slice() else { panic!("{diags:?}") };
        assert_eq!(diag.code, DiagCode::BareKeyword);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.span, read(src).args()[0].span);
        assert_eq!(diag.help.as_deref(), Some("write it as (frac num den)"));
    }

    #[test]
    fn did_you_mean() {
        let src = "(intgral f x)";
        let (out, diags) = render(src);
        assert_eq!(
            out,
            "<mrow><mi>intgral</mi><mrow><mo>(</mo><mi>f</mi><mo>,</mo><mi>x</mi><mo>)</mo></mrow></mrow>",
            "still rendered, as a function application"
        );
        let [diag] = diags.as_slice() else { panic!("{diags:?}") };
        assert_eq!(diag.code, DiagCode::UnknownForm);
        assert_eq!(diag.severity, Severity::Warning);
        assert_eq!(diag.span, read(src).as_list().unwrap()[0].span);
        assert_eq!(diag.help.as_deref(), Some("did you mean `integral`?"));

        let (_, diags) = render("(farc 1 2)");
        assert_eq!(diags.as_slice()[0].help.as_deref(), Some("did you mean `frac`?"));

        // Deliberate names are left alone.
        for src in ["(f x)", "(gen x)", "(min a b)", "(sup S)", "(sinh x)", "(Laplace f)"] {
            let (_, diags) = render(src);
            assert!(diags.is_empty(), "{src}: {diags:?}");
        }
    }

    #[test]
    fn errors_stay_local() {
        // A bad call becomes an <merror>; the rest of the expression renders.
        let (out, diags) = render("(+ (frac 1) x)");
        assert_eq!(
            out,
            "<mrow><merror><mtext>`frac` is missing its `den` argument</mtext></merror><mo>+</mo><mi>x</mi></mrow>"
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.as_slice()[0].code, DiagCode::Arity);

        let (out, diags) = with(|o| o.error(Diag::error(DiagCode::Other, Span::DUMMY, "a < b")));
        assert_eq!(out, "<merror><mtext>a &lt; b</mtext></merror>");
        assert_eq!(diags.as_slice()[0].message, "a < b");
    }

    #[test]
    fn check_reports_what_render_reports() {
        let src = "(+ (frac 1) (intgral f x) frac (sin 1 2))";
        let node = read(src);
        let mut checked = Diags::new();
        check(&node, &mut checked);
        let (out, rendered) = render(src);
        assert_eq!(checked, rendered);
        let codes: Vec<_> = checked.iter().map(|d| d.code).collect();
        assert_eq!(
            codes,
            [DiagCode::Arity, DiagCode::UnknownForm, DiagCode::BareKeyword, DiagCode::Arity]
        );
        assert_eq!(top_level_elements(&out), 1);
    }
}
