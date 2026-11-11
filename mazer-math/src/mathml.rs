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
//!
//! # Parentheses
//!
//! Parentheses are never written by looking at a child's head. Every keyword
//! call has a `Prec` level (declared in the `precedence` function of its
//! keyword file); a parent says how tightly an argument must hold together in
//! each position, and `MathMl::operand`, `MathMl::infix`,
//! `MathMl::prefix` and `MathMl::postfix` add parentheses when it does not.
//! On top of the levels there are two rules about the edges of an expression:
//!
//! - **Signs.** An expression that starts with a sign (a negative number,
//!   unary `−` or `+`, or an arithmetic chain whose first operand does) is
//!   parenthesised after another arithmetic operator, so two signs never touch:
//!   `a + (−2)`, `−(−x)`, `a (−b)`. It needs none at the very start of an
//!   arithmetic row (`−2 + a`, `−s × t`) or in a relation or logic row
//!   (`x = −2`).
//! - **Big operators.** `∑ a` takes everything after it up to the next `+`,
//!   so a big operator, or a product or signed expression ending in one, is
//!   parenthesised anywhere something tighter than a sum follows it:
//!   `(∑ a) × b`, `(−∑ a) × b`, but `b × ∑ a` and `∑ a + b`.

use mazer_syntax::{Diag, DiagCode, Diags, Node, NodeKind, Span, Sym};

use crate::call::try_bind;
use crate::display::{Class, display_name};
use crate::keyword::Keyword;
use crate::registry::registry;

/// A thin space (3/18 em), e.g. between an integrand and its `dx`.
pub(crate) const THIN_SPACE: &str = "0.167em";

/// How tightly an expression holds together once rendered, loosest first.
/// A parent uses it to decide whether a child needs parentheses: see
/// [`MathMl::operand`]. The table in `keywords/mod.rs` says which keyword
/// belongs to which level and how to write each kind of keyword.
///
/// Keyword files say which of their keywords are not [`Prec::Atom`] in their
/// own `precedence` function.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[allow(dead_code)] // most levels are first used by the step 2 keywords
pub(crate) enum Prec {
    /// `∀x. P`, `∃x. P`. The body runs as far right as it can, so a
    /// quantifier is parenthesised in every operand position: `(∀x. P) ∧ Q`.
    /// Its own body never is.
    Binder,
    /// `⟹`, `⟺`. Not associative: a nested implication on either side is
    /// parenthesised, `(a ⟹ b) ⟹ c`.
    Implication,
    /// `∨`.
    Or,
    /// `∧`. Tighter than `∨`: `(a ∨ b) ∧ c`, but `a ∧ b ∨ c`.
    And,
    /// `¬`. Looser than the relations, as in logic: `¬x = y` is `¬(x = y)`,
    /// while `(¬a) = b` needs its parentheses.
    Not,
    /// `=`, `≠`, `<`, `≤`, `≈`, `∈`, `⊆`, ...: `a < b < c` is a chain, and a
    /// relation inside a relation is parenthesised.
    Relation,
    /// `+`, binary `−`, `∪`.
    Additive,
    /// Unary `−`, unary `+`, a negative number. Nothing else: this level means
    /// "starts with a sign".
    Prefix,
    /// `∫`, `∑`, `∏`, `lim`, `d/dx`: a big operator followed by its body.
    BigOp,
    /// `×`, `÷`, `∩`, juxtaposition.
    Multiplicative,
    /// A stacked fraction (`frac`, two-argument `/`).
    Fraction,
    /// A superscript: `x²`, `eˣ`.
    Power,
    /// `n!`, `f′`.
    Postfix,
    /// Never needs parentheses: names, numbers, strings, subscripts, roots,
    /// fences, function applications, named functions, accents, matrices.
    Atom,
}

impl Prec {
    /// The next tighter level. [`Assoc::Left`] and [`Assoc::Neither`] use it
    /// for the operands that must not be at the row's own level.
    #[must_use]
    pub(crate) const fn tighter(self) -> Self {
        match self {
            Self::Binder => Self::Implication,
            Self::Implication => Self::Or,
            Self::Or => Self::And,
            Self::And => Self::Not,
            Self::Not => Self::Relation,
            Self::Relation => Self::Additive,
            Self::Additive => Self::Prefix,
            Self::Prefix => Self::BigOp,
            Self::BigOp => Self::Multiplicative,
            Self::Multiplicative => Self::Fraction,
            Self::Fraction => Self::Power,
            Self::Power => Self::Postfix,
            Self::Postfix | Self::Atom => Self::Atom,
        }
    }
}

/// How the operands of an infix row may group, for [`MathMl::infix`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // `Left` and `Neither` are first used by the step 2 keywords
pub(crate) enum Assoc {
    /// Associative: an operand at the row's own level needs no parentheses.
    /// `+`, `×`, `∪`, `∩`, `∧`, `∨`, juxtaposition.
    Both,
    /// Groups to the left: the first operand may be at the row's level, the
    /// others must be tighter. Binary `−` and `÷`: `a − b − c`, `a − (b − c)`.
    Left,
    /// Every operand must be tighter than the row. Relation chains
    /// (`a < b < c`, but `(a < b) = c`) and `⟹`, `⟺`.
    Neither,
}

/// How tightly `node` holds together once rendered.
pub(crate) fn precedence(node: &Node) -> Prec {
    let node = see_through(node);
    match &node.kind {
        NodeKind::Number(text) if text.starts_with('-') => Prec::Prefix,
        NodeKind::List(_) => match declared(node) {
            // A product that ends in a big operator is open to the right like
            // the big operator itself: `a ∑ b` followed by `× c` would read as
            // `a ∑ (b × c)`. (A `÷` row parenthesises its last operand anyway;
            // for it this only adds harmless parentheses.)
            Some((Prec::Multiplicative, args @ [.., last]))
                if precedence(last) == Prec::BigOp && !(args.len() > 1 && leads_with_sign(last)) =>
            {
                Prec::BigOp
            }
            Some((level, _)) => level,
            None => Prec::Atom,
        },
        _ => Prec::Atom,
    }
}

/// The level a keyword file declares for the call `node`, and its arguments.
/// `None` if `node` is not a keyword call (then it is an atom).
fn declared(node: &Node) -> Option<(Prec, &[Node])> {
    let (head, args) = node.as_list()?.split_first()?;
    let keyword = registry().get(head.as_ident()?)?;
    if args.is_empty() && keyword.sign.is_some() {
        // `(neq)` renders the sign itself, whatever the keyword's levels say.
        return None;
    }
    Some((crate::keywords::precedence(keyword.name, args.len()), args))
}

/// Strips the wrappers that render exactly as what they wrap, so the node
/// holds together exactly as the wrapped one does: `(quote x)`, a font variant
/// (`(bb (+ a b))` is a sum, so its parent parenthesises it like one), a
/// funcapp or prime with nothing to add, and a call with a non-keyword head
/// and no arguments (`((+ b c))` renders as `b + c`). See
/// `keywords::see_through` for the keyword cases.
fn see_through(node: &Node) -> &Node {
    let Some((head, args)) = node.as_list().and_then(<[Node]>::split_first) else {
        return node;
    };
    let inner = head.as_ident().and_then(|name| registry().get(name)).map_or_else(
        || args.is_empty().then_some(head),
        |keyword| crate::keywords::see_through(keyword.name, args),
    );
    inner.map_or(node, see_through)
}

/// Whether `node` renders starting with a `−` or `+` sign: a negative number,
/// unary `−`/`+` (the [`Prec::Prefix`] level), or an additive or
/// multiplicative row whose first operand starts with a sign and is not
/// parenthesised (`−2 + a`, `−s × t`).
///
/// Relies on the rule that keywords at those two levels render as an
/// [`MathMl::infix`] row (or [`MathMl::juxtapose`]) starting with their first
/// argument.
fn leads_with_sign(node: &Node) -> bool {
    let node = see_through(node);
    match &node.kind {
        NodeKind::Number(text) => text.starts_with('-'),
        NodeKind::List(_) => match declared(node) {
            Some((Prec::Prefix, _)) => true,
            Some((level @ (Prec::Additive | Prec::Multiplicative), args @ [first, ..])) => {
                leads_with_sign(first) && !chain_needs_parens(first, 0, args.len(), level, Assoc::Both)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Whether a signed expression ends in a big operator whose body would run
/// on into what follows: `−∑ a`. Such an expression cannot lead a product
/// unparenthesised.
fn opens_right(node: &Node) -> bool {
    match declared(see_through(node)) {
        Some((Prec::Prefix, [.., last])) => precedence(last) == Prec::BigOp && !leads_with_sign(last),
        _ => false,
    }
}

/// Whether operand `index` of `count` in an infix row at level `prec` needs
/// parentheses. See [`MathMl::infix`].
fn chain_needs_parens(node: &Node, index: usize, count: usize, prec: Prec, assoc: Assoc) -> bool {
    let min = match assoc {
        // A big operator may end a product: `b × ∑ a`.
        Assoc::Both if prec == Prec::Multiplicative && index + 1 == count => Prec::BigOp,
        Assoc::Both => prec,
        Assoc::Left if index == 0 => prec,
        Assoc::Left | Assoc::Neither => prec.tighter(),
    };
    let level = precedence(node);
    if prec < Prec::Additive {
        // Relation and logic rows: a sign after `=` or `∧` is fine (`x = −2`).
        return level < min;
    }
    if index == 0 {
        // A sign may start an arithmetic row: `−2 + a`, `−s × t`, but not
        // `−∑ a × b`.
        return level < min && (level != Prec::Prefix || opens_right(node));
    }
    // Two signs never touch: `a + (−2)`, `a × (−b)`.
    level < min || leads_with_sign(node)
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
    ///   as f(x, y); with no arguments, `(Gamma)` renders just the head. A head
    ///   that is not an atom is parenthesised: `((+ f g) x)` is (f + g)(x). If
    ///   the head is one typo away from a keyword, also push an `UnknownForm`
    ///   hint with a did-you-mean.
    /// - `Error` -> `<merror>` (the reader already reported it)
    ///
    /// Details:
    /// - A negative number `-2` renders as a minus sign (U+2212) and `<mn>2</mn>`.
    /// - A name with no display form renders as `<mi>`, or as `<mo>` when it has
    ///   no letter or digit in it (`<>`, `|`, `::`).
    /// - A keyword call with the wrong number of arguments renders as an
    ///   `<merror>` holding the `Arity` message.
    /// - The empty list `()` renders as an empty `<mrow>`.
    /// - A keyword head with no arguments renders exactly like the bare
    ///   keyword: `(neq)` is its sign `≠`, and `(frac)` reports `BareKeyword`.
    ///   Only a keyword with no sign that accepts zero arguments (`(set)`,
    ///   `(mat)`) runs its emitter.
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

    /// Like [`MathMl::tag`] with attributes. Attribute values are escaped;
    /// attribute names must be ASCII letters, digits and `-` (checked in debug
    /// builds).
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

    /// `<msub>base script</msub>`. A base that is not an atom (`a + b`, `−x`,
    /// `x²`, `a/b`, `n!`, `∫ f dx`) is put in parentheses first.
    pub fn sub(&mut self, base: &Node, script: &Node) {
        self.tag("msub", |o| {
            o.operand(base, Prec::Atom);
            o.expr(script);
        });
    }

    /// `<msup>base script</msup>`. A base that is not an atom (`a + b`, `−x`,
    /// `x²`, `a/b`, `n!`, `∫ f dx`) is put in parentheses first: `(n!)²`.
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
/// `keywords/` are written, not part of the contract with other crates. The
/// table in `keywords/mod.rs` says which one each kind of keyword uses.
impl MathMl<'_> {
    /// Pushes a diagnostic without writing anything, for problems that do not
    /// stop the expression from rendering normally (warnings, hints).
    pub(crate) fn report(&mut self, diag: Diag) {
        self.diags.push(diag);
    }

    /// Renders `node`, in parentheses if it holds together more loosely than
    /// `min` (see [`Prec`]), or if `min` is [`Prec::Prefix`] or tighter and
    /// `node` starts with a sign (`−(−x)`, `∑ (−a)`).
    ///
    /// For one argument in a position of its own: the operand of a prefix or
    /// postfix operator, the body of a big operator, a script base. Rows of
    /// operands use [`MathMl::infix`] instead.
    pub(crate) fn operand(&mut self, node: &Node, min: Prec) {
        let parens = precedence(node) < min || (min >= Prec::Prefix && leads_with_sign(node));
        self.parenthesised_if(parens, node);
    }

    /// An infix row `a op b op c` in one `<mrow>`, at level `prec`, e.g.
    /// `infix("+", Prec::Additive, Assoc::Both, terms)`.
    ///
    /// An operand is parenthesised when it is looser than the row allows
    /// (`assoc` decides whether an operand at the row's own level is allowed,
    /// see [`Assoc`]), and in an arithmetic row (`prec` at least
    /// [`Prec::Additive`]) when it starts with a sign and is not the first
    /// operand: `a + (−2)`, but `−2 + a` and `x = −2`. The last operand of an
    /// associative [`Prec::Multiplicative`] row may be a big operator:
    /// `b × ∑ a`.
    ///
    /// With a single operand it writes just that operand (inside the
    /// `<mrow>`, parenthesised by the same rules); with none, an empty
    /// `<mrow>`.
    pub(crate) fn infix(&mut self, op: &str, prec: Prec, assoc: Assoc, operands: &[Node]) {
        let operands: Vec<&Node> = operands.iter().collect();
        self.chain(prec, assoc, &operands, |o| o.mo(op));
    }

    /// [`MathMl::infix`] with exactly two operands, for keywords with two
    /// required parameters: `binary("⟹", Prec::Implication, Assoc::Neither,
    /// call.req(0), call.req(1))`.
    #[allow(dead_code)] // first used by the step 2 keywords
    pub(crate) fn binary(&mut self, op: &str, prec: Prec, assoc: Assoc, left: &Node, right: &Node) {
        self.chain(prec, assoc, &[left, right], |o| o.mo(op));
    }

    /// Juxtaposition: the operands side by side with a thin space between
    /// them, as an associative [`Prec::Multiplicative`] row (same rules as
    /// [`MathMl::infix`]): `a (−b)`, `(a + b) c`, `2 ∑ a`.
    #[allow(dead_code)] // first used by the step 2 keywords
    pub(crate) fn juxtapose(&mut self, operands: &[Node]) {
        let operands: Vec<&Node> = operands.iter().collect();
        self.chain(Prec::Multiplicative, Assoc::Both, &operands, |o| o.space(THIN_SPACE));
    }

    /// A prefix operator: `<mrow><mo>op</mo> operand</mrow>`, the operand
    /// rendered with [`MathMl::operand`] at `min`. Unary `−`/`+` pass
    /// [`Prec::BigOp`] (`−(a + b)`, `−(−x)`, `−a b`, `−∑ a`); `¬` passes
    /// [`Prec::Not`] (`¬¬p`, `¬(p ∧ q)`).
    pub(crate) fn prefix(&mut self, op: &str, operand: &Node, min: Prec) {
        self.row(|o| {
            o.mo(op);
            o.operand(operand, min);
        });
    }

    /// A postfix operator: `<mrow>operand <mo>op</mo></mrow>`. The operand is
    /// parenthesised unless it is an atom: `n!`, `f(x)!`, `(n + 1)!`, `(x²)!`,
    /// `(a/b)!`, and `(n!)!` (never `n!!`, which reads as a double factorial).
    #[allow(dead_code)] // first used by the step 2 keywords
    pub(crate) fn postfix(&mut self, op: &str, operand: &Node) {
        self.row(|o| {
            o.operand(operand, Prec::Atom);
            o.mo(op);
        });
    }

    /// A quantifier: `<mrow><mo>sign</mo> var <mo>.</mo> body</mrow>`, i.e.
    /// `∀x. P`. The body is never parenthesised; the whole quantifier is a
    /// [`Prec::Binder`] and so is parenthesised wherever it is an operand.
    #[allow(dead_code)] // first used by the step 2 keywords
    pub(crate) fn binder(&mut self, sign: &str, var: &Node, body: &Node) {
        self.row(|o| {
            o.mo(sign);
            o.expr(var);
            o.mo(".");
            o.expr(body);
        });
    }

    /// `<munder>base under</munder>`: something written underneath, e.g. the
    /// limits of `∑` or `lim`. Each closure writes exactly one element.
    pub(crate) fn under(&mut self, base: impl FnOnce(&mut Self), under: impl FnOnce(&mut Self)) {
        self.tag("munder", |o| {
            base(o);
            under(o);
        });
    }

    /// `<munderover>base under over</munderover>`: limits below and above, as
    /// on `∑` and `∏`. Each closure writes exactly one element.
    pub(crate) fn under_over(
        &mut self,
        base: impl FnOnce(&mut Self),
        under: impl FnOnce(&mut Self),
        over: impl FnOnce(&mut Self),
    ) {
        self.tag("munderover", |o| {
            base(o);
            under(o);
            over(o);
        });
    }

    /// Function application: `head(a, b, c)`. `head` writes the function
    /// (exactly one element); the arguments are rendered in a parenthesised,
    /// comma-separated [`MathMl::fence`]. With no arguments only the head is
    /// written. Named functions such as `sin` use this with a fixed head.
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

    /// Function application with an expression as the function: what
    /// `(f x)` renders as. The head is parenthesised if it is looser than a
    /// power, so `(f + g)(x)` and `(a/b)(x)`, but `f′(x)` and `f⁻¹(x)`. With no
    /// arguments it writes just the head, unparenthesised.
    pub(crate) fn application(&mut self, head: &Node, args: &[Node]) {
        if args.is_empty() {
            self.expr(head);
        } else {
            self.apply(|o| o.operand(head, Prec::Power), args);
        }
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
        match registry().get(name) {
            Some(keyword) => self.bare(keyword, name, span),
            None => self.name(name),
        }
    }

    /// A keyword with no arguments: its sign if it has one, else the
    /// `BareKeyword` diagnostic.
    fn bare(&mut self, keyword: &Keyword, name: Sym, span: Span) {
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
                // `(neq)` is `neq`; `(set)` and `(mat)` run their emitters.
                if args.is_empty() && (keyword.sign.is_some() || keyword.min_args() > 0) {
                    self.bare(keyword, name, node.span);
                    return;
                }
                match try_bind(keyword, node) {
                    Ok(call) => (keyword.mathml)(&call, self),
                    Err(diag) => self.error(diag),
                }
                return;
            }
            if display_name(name).is_none()
                && let Some(suggestion) = registry().suggest(name.as_str())
            {
                // A hint, not a warning: the name may well be deliberate.
                self.report(
                    Diag::hint(
                        DiagCode::UnknownForm,
                        head.span,
                        format!("`{name}` is not a keyword, so this is shown as a function application"),
                    )
                    .with_help(format!("did you mean `{suggestion}`?")),
                );
            }
        }
        self.application(head, args);
    }

    /// The operands of [`MathMl::infix`] / [`MathMl::juxtapose`], with `sep`
    /// written between them.
    fn chain(&mut self, prec: Prec, assoc: Assoc, operands: &[&Node], sep: impl Fn(&mut Self)) {
        self.row(|o| {
            for (i, operand) in operands.iter().enumerate() {
                if i > 0 {
                    sep(o);
                }
                let parens = chain_needs_parens(operand, i, operands.len(), prec, assoc);
                o.parenthesised_if(parens, operand);
            }
        });
    }

    fn parenthesised_if(&mut self, parens: bool, node: &Node) {
        if parens {
            self.fence("(", ")", |o| o.expr(node));
        } else {
            self.expr(node);
        }
    }

    /// `<merror><mtext>text</mtext></merror>`, without a diagnostic.
    fn merror(&mut self, text: &str) {
        self.tag("merror", |o| o.mtext(text));
    }

    /// Writes `<name a="v" ...` (no closing `>`), escaping the values.
    fn open(&mut self, name: &str, attrs: &[(&str, &str)]) {
        debug_assert!(is_xml_name(name), "invalid element name {name:?}");
        self.out.push('<');
        self.out.push_str(name);
        for (key, value) in attrs {
            debug_assert!(is_xml_name(key), "invalid attribute name {key:?}");
            self.out.push(' ');
            self.out.push_str(key);
            self.out.push_str("=\"");
            self.escape(value);
            self.out.push('"');
        }
    }

    /// Writes `text` with `&`, `<`, `>`, `"` and `'` replaced by entities.
    /// Safe both in element content and in a double-quoted attribute value.
    ///
    /// Characters XML does not allow at all are dropped: the C0 control
    /// characters other than tab and newline (a note may contain them by
    /// accident), and U+FFFE / U+FFFF.
    fn escape(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '"' => self.out.push_str("&quot;"),
                '\'' => self.out.push_str("&#39;"),
                '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}' => {}
                _ => self.out.push(ch),
            }
        }
    }
}

/// Element and attribute names written by this crate: non-empty ASCII
/// letters, digits and `-`. Nothing else needs escaping in a name.
fn is_xml_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
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
    fn characters_xml_forbids_are_dropped() {
        // C0 controls other than tab and newline are not allowed in XML 1.0.
        let (out, _) = with(|o| {
            o.mi("a\u{0}b\u{7}c\u{1b}d\re");
            o.mtext("tab\there\nnewline");
            o.mo("\u{ffff}+");
            o.mn("1\u{8}2");
        });
        assert_eq!(out, "<mi>abcde</mi><mtext>tab\there\nnewline</mtext><mo>+</mo><mn>12</mn>");
        let (out, _) = with(|o| o.tag_attrs("mstyle", &[("mathcolor", "r\u{1}ed")], |o| o.mi("x")));
        assert_eq!(out, "<mstyle mathcolor=\"red\"><mi>x</mi></mstyle>");
        // From the source too.
        assert_eq!(mathml("\"bell\u{7}\""), "<mtext>bell</mtext>");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "invalid attribute name")]
    fn attribute_names_are_checked() {
        let _ = with(|o| o.tag_attrs("mstyle", &[("onload=\"x", "y")], |_| {}));
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
        // A head that is not an atom is parenthesised: (f + g)(x).
        assert_eq!(
            mathml("((+ f g) x)"),
            "<mrow><mrow><mo>(</mo><mrow><mi>f</mi><mo>+</mo><mi>g</mi></mrow><mo>)</mo></mrow>\
             <mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow></mrow>"
        );
        // ... but not with no arguments, where only the head is written.
        assert_eq!(mathml("((+ f g))"), "<mrow><mi>f</mi><mo>+</mo><mi>g</mi></mrow>");
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
        assert_eq!(diag.severity, Severity::Hint, "the name may be deliberate");
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

/// The precedence rules, checked on what a reader sees (tags stripped; a
/// thin space shows as a space). Keywords that do not exist yet (`-`, `*`,
/// `and`, `fact`, ...) are stand-ins from `testing.rs` written by the recipes
/// in `keywords/mod.rs`; once the real keyword exists these tests run against
/// it instead, so they also check the step 2 keywords.
#[cfg(test)]
mod parentheses {
    use mazer_syntax::{DiagCode, Diags};

    use super::check;
    use crate::testing::{mathml, read, render, visible, with_standins};

    #[track_caller]
    fn shows(src: &str, expected: &str) {
        assert_eq!(with_standins(|| visible(src)), expected, "{src}");
    }


    #[test]
    fn font_variants_hold_together_like_their_argument() {
        // A styled compound is parenthesised by its parent exactly when the
        // unstyled compound would be.
        shows("(- (rm (+ a b)))", "\u{2212}(a+b)");
        shows("(^ (bb (+ a b)) 2)", "(a+b)2");
        shows("(* c (bold (+ a b)))", "c\u{d7}(a+b)");
        shows("(fact (cal (* a b)))", "(a\u{d7}b)!");
        shows("(sum (bb (+ a b)) n)", "\u{2211}n(a+b)");
        // A styled atom is still an atom, and a styled sign still starts with one.
        shows("(- (bb x))", "\u{2212}\u{1d569}");
        shows("(^ (bold x) 2)", "\u{1d431}2");
        shows("(+ a (bb -2))", "a+(\u{2212}2)");
        shows("(* (sum a n) (rm b))", "(\u{2211}na)\u{d7}b");
    }

    #[test]
    fn zero_argument_application_takes_its_heads_level() {
        shows("(* a ((+ b c)))", "a\u{d7}(b+c)");
        shows("(^ (funcapp (+ f g)) 2)", "(f+g)2");
        shows("(+ a ((* b c)))", "a+b\u{d7}c");
        shows("(^ ((Gamma)) 2)", "\u{393}2");
        // With arguments the head is parenthesised by `application` itself.
        shows("(^ ((+ f g) x) 2)", "(f+g)(x)2");
    }

    #[test]
    fn prime_with_count_zero_is_not_there() {
        shows("(^ (prime (+ a b) 0) 2)", "(a+b)2");
        shows("(- (prime (+ a b) 0))", "\u{2212}(a+b)");
        shows("(prime (prime f 0))", "f\u{2032}");
    }

    #[test]
    fn postfix_operands() {
        shows("(fact (fact n))", "(n!)!");
        shows("(prime (fact n))", "n!\u{2032}");
        shows("(prime (prime f))", "f\u{2032}\u{2032}");
        shows("(fact (* a b))", "(a\u{d7}b)!");
    }

    #[test]
    fn big_operators_and_products() {
        shows("(* (sum a n) b)", "(\u{2211}na)\u{d7}b");
        shows("(* b (sum a n))", "b\u{d7}\u{2211}na");
        shows("(* 2 (sum a n) b)", "2\u{d7}(\u{2211}na)\u{d7}b");
        shows("(^ (derivative f x) 2)", "(ddxf)2");
        shows("(- (frac a b))", "\u{2212}ab");
    }

    #[test]
    fn quantifier_inside_and() {
        shows("(and (forall x (= x y)) p)", "(\u{2200}x.x=y)\u{2227}p");
        shows("(and p (forall x (= x y)))", "p\u{2227}(\u{2200}x.x=y)");
        shows(
            "(and (exists x (< x y)) (forall z (in z S)))",
            "(\u{2203}x.x<y)\u{2227}(\u{2200}z.z\u{2208}S)",
        );
    }

    #[test]
    fn accents_and_primes_as_script_bases() {
        let x = |accent: &str| format!("<mover><mi>x</mi><mo>{accent}</mo></mover>");
        // An accent is an atom: no parentheses.
        assert_eq!(
            mathml("(^ (hat x) 2)"),
            format!("<msup>{}<mn>2</mn></msup>", x("^"))
        );
        assert_eq!(
            mathml("(^ (bar x) 2)"),
            format!("<msup>{}<mn>2</mn></msup>", x("\u{af}"))
        );
        assert_eq!(
            mathml("(subscript (hat x) i)"),
            format!("<msub>{}<mi>i</mi></msub>", x("^"))
        );
        assert_eq!(
            mathml("(subscript (bar x) i)"),
            format!("<msub>{}<mi>i</mi></msub>", x("\u{af}"))
        );
        // A prime is postfix, so as a base it is parenthesised like `(n!)²`.
        shows("(^ (prime f) 2)", "(f\u{2032})2");
        shows("(subscript (prime f) i)", "(f\u{2032})i");
    }

    #[test]
    fn text_and_fonts_as_operands() {
        shows("(+ (text hi) a)", "hi+a");
        shows("(* (text hi) a)", "hi\u{d7}a");
        shows("(^ (text hi) 2)", "hi2");
        shows("(+ (bb R) a)", "\u{211d}+a");
        shows("(* (bb R) a)", "\u{211d}\u{d7}a");
        shows("(^ (bb R) 2)", "\u{211d}2");
    }

    #[test]
    fn grids_hold_any_expression_in_a_cell() {
        shows(
            "(cases ((+ a b) (text if p)) ((frac a b) q) ((- x) r))",
            "{a+bif pabq\u{2212}xr",
        );
        shows("(mat ((+ a b) (frac a b)) ((- x) c))", "(a+bab\u{2212}xc)");
        shows("(table (row (+ a b) (frac a b) (- x)))", "a+bab\u{2212}x");
        shows("(table (header (+ a b) (- x)) (row (frac a b) c))", "a+b\u{2212}xabc");
    }

    #[test]
    fn zero_argument_calls_are_the_bare_keyword() {
        for (src, sign) in [
            ("(in)", "\u{2208}"),
            ("(arrow)", "\u{2192}"),
            ("(forall)", "\u{2200}"),
            ("(partial)", "\u{2202}"),
            ("(neq)", "\u{2260}"),
            ("(+)", "+"),
            ("(prime)", "\u{2032}"),
        ] {
            assert_eq!(mathml(src), format!("<mo>{sign}</mo>"), "{src}");
        }
        // It is an atom, and `check` agrees with rendering.
        shows("(^ (neq) 2)", "\u{2260}2");
        shows("(* a (+))", "a\u{d7}+");
        // No sign, but zero arguments is a valid call: the emitter runs.
        assert_eq!(mathml("(set)"), "<mrow><mo>{</mo><mo>}</mo></mrow>");
        // No sign and arguments required: the `BareKeyword` diagnostic.
        for src in ["(frac)", "frac"] {
            let (out, diags) = render(src);
            assert!(out.starts_with("<merror>"), "{src}: {out}");
            assert_eq!(diags.len(), 1, "{src}");
            assert_eq!(diags.as_slice()[0].code, DiagCode::BareKeyword, "{src}");
            let mut checked = Diags::new();
            check(&read(src), &mut checked);
            assert_eq!(checked, diags, "{src}");
        }
    }

    #[test]
    fn logic_levels() {
        shows("(and (or a b) c)", "(a∨b)∧c");
        shows("(and c (or a b))", "c∧(a∨b)");
        shows("(or (and a b) c)", "a∧b∨c");
        shows("(and (and a b) c)", "a∧b∧c");
        // Implication is not associative, on either side.
        shows("(implies (implies a b) c)", "(a⟹b)⟹c");
        shows("(implies a (implies b c))", "a⟹(b⟹c)");
        shows("(iff (implies a b) c)", "(a⟹b)⟺c");
        shows("(implies (or a b) (and c d))", "a∨b⟹c∧d");
        // Relations are tighter than the connectives, `¬` sits between.
        shows("(and (= a b) c)", "a=b∧c");
        shows("(and (not a) b)", "¬a∧b");
        shows("(not (and a b))", "¬(a∧b)");
        shows("(not (not p))", "¬¬p");
        shows("(not (= x y))", "¬x=y");
        shows("(= (not a) b)", "(¬a)=b");
        // A relation inside a relation is parenthesised; a chain is not.
        shows("(= a b c)", "a=b=c");
        shows("(= (= a b) c)", "(a=b)=c");
    }

    #[test]
    fn quantifiers() {
        // The body runs to the right and is never parenthesised ...
        shows("(forall x (implies P Q))", "∀x.P⟹Q");
        shows("(forall x (exists y (and P Q)))", "∀x.∃y.P∧Q");
        // ... and the whole quantifier is parenthesised as an operand.
        shows("(and (forall x P) Q)", "(∀x.P)∧Q");
        shows("(and Q (exists x P))", "Q∧(∃x.P)");
        shows("(implies (exists x P) Q)", "(∃x.P)⟹Q");
        shows("(not (forall x P))", "¬(∀x.P)");
    }

    #[test]
    fn signs() {
        shows("(+ a -2)", "a+(−2)");
        shows("(+ -2)", "+(−2)");
        shows("(+ (+ a))", "+(+a)");
        shows("(- a -2)", "a−(−2)");
        shows("(- (- x))", "−(−x)");
        shows("(jux a (- b))", "a (−b)");
        shows("(* a -2)", "a×(−2)");
        // A sign may lead a row ...
        shows("(+ -2 a)", "−2+a");
        shows("(- (- a) b)", "−a−b");
        shows("(* -s t)", "−s×t");
        shows("(jux -2 x)", "−2 x");
        // ... and follow a relation.
        shows("(= x -2)", "x=−2");
        shows("(= y (- x))", "y=−x");
        // A row that starts with a sign counts as signed.
        shows("(+ b (+ -2 a))", "b+(−2+a)");
        shows("(+ b (* -2 a))", "b+(−2×a)");
        shows("(- (* -2 a))", "−(−2×a)");
        // Unary minus binds looser than a product, tighter than a sum.
        shows("(- (* a b))", "−a×b");
        shows("(- (+ a b))", "−(a+b)");
        shows("(* (- a) b)", "−a×b");
        shows("(^ (- a) 2)", "(−a)2");
        shows("(^ -2 2)", "(−2)2");
        // Grouping on the right of a binary minus.
        shows("(- a (- b c))", "a−(b−c)");
        shows("(- (- a b) c)", "a−b−c");
        shows("(- a (+ b c))", "a−(b+c)");
    }

    #[test]
    fn big_operators() {
        // As a script base.
        shows("(subscript (integral f x) 2)", "(∫f dx)2");
        // In a product: parenthesised unless last.
        shows("(* (sum a n) b)", "(∑na)×b");
        shows("(* b (sum a n))", "b×∑na");
        shows("(jux (sum a n) b)", "(∑na) b");
        shows("(jux b (sum a n))", "b ∑na");
        // A product ending in one is open to the right too.
        shows("(* (* b (sum a n)) c)", "(b×∑na)×c");
        shows("(* (- (sum a n)) b)", "(−∑na)×b");
        // In a sum it needs none.
        shows("(+ (sum a n) b)", "∑na+b");
        shows("(+ b (sum a n))", "b+∑na");
        shows("(- (sum a n))", "−∑na");
        // The body: parenthesised when additive, signed or looser.
        shows("(sum (+ a b) n)", "∑n(a+b)");
        shows("(sum (- a) n)", "∑n(−a)");
        shows("(sum (* a b) n)", "∑na×b");
        shows("(sum (sum a m) n)", "∑n∑ma");
        shows("(integral (sum a n) x)", "∫∑na dx");
        shows("(integral (- f) x)", "∫(−f) dx");
        // Anything tighter than a product parenthesises it.
        shows("(^ (sum a n) 2)", "(∑na)2");
        shows("(fact (sum a n))", "(∑na)!");
    }

    #[test]
    fn postfix_and_scripts() {
        shows("(^ (fact n) 2)", "(n!)2");
        shows("(fact (^ x 2))", "(x2)!");
        shows("(fact (+ n 1))", "(n+1)!");
        shows("(fact (- n))", "(−n)!");
        shows("(fact (subscript x i))", "xi!");
        shows("(prime (fact n))", "n!′");
        shows("(^ (prime f) 2)", "(f′)2");
        shows("(prime (^ x 2))", "(x2)′");
        // Powers and fractions as bases.
        shows("(^ (^ x 2) 3)", "(x2)3");
        shows("(^ (subscript x i) 2)", "xi2");
        shows("(subscript (^ x 2) i)", "(x2)i");
        shows("(^ (frac a b) 2)", "(ab)2");
        shows("(subscript (frac a b) i)", "(ab)i");
        shows("(fact (frac a b))", "(ab)!");
        shows("(* (frac a b) c)", "ab×c");
        shows("(- (frac a b))", "−ab");
    }

    #[test]
    fn application_heads() {
        shows("((+ f g) x)", "(f+g)(x)");
        shows("((prime f) x)", "f′(x)");
        shows("((^ f -1) x)", "f−1(x)");
        shows("((frac a b) x)", "(ab)(x)");
        shows("((subscript f 1) x)", "f1(x)");
    }

    #[test]
    fn quote_is_transparent() {
        shows("(* (quote (+ a b)) c)", "(a+b)×c");
        shows("(+ a '-2)", "a+(−2)");
    }
}
