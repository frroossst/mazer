//! Notation: `alias` and `macro`.
//!
//! ```lisp
//! (alias R (bb R))                       ; an alias is a macro with no parameters
//! (macro (laplace f s)
//!   "Laplace transform of f, evaluated at s"
//!   (jux Laplace (brace f) (paren s)))
//! (macro (pair a rest...) (paren a rest))   ; `rest...` collects the remaining arguments
//! ```
//!
//! Expansion is pure, lazy tree substitution. Nothing is evaluated.
//!
//! Rules:
//! - `alias` and `macro` are block-level reserved heads. A malformed
//!   definition is a `BadDefinition` diagnostic.
//! - Scope is the whole note. Definitions from the embedded `stdlib.zr`, then
//!   from the note, are all collected before anything is expanded, so order
//!   does not matter. Defining a name twice is `DuplicateDefinition`.
//! - A macro or alias may not reuse the name of a built-in keyword or display
//!   name (`ShadowsBuiltin`).
//! - Each parameter name in the template is replaced by the argument exactly as
//!   written. Wrong argument count is `MacroArity`.
//! - The result is expanded again, so macros can use macros. A name is not
//!   re-expanded inside its own expansion (so `(alias R (bb R))` is fine).
//!   Nesting deeper than [`MAX_DEPTH`] is `MacroDepth`.
//! - Expanded nodes take the span of the place the macro was *used*; the
//!   definition span is attached to diagnostics as a secondary label.
//! - `(quote ...)` contents are still expanded; quoting only stops `let`
//!   substitution, not notation.
//! - A template may not contain a list headed by a reserved word (`show`,
//!   `eval`, `let`, `alias`, `macro`, `ref`) or by `tag`: such a definition
//!   is reported once, where it is written, and the name is not defined.
//!
//! # Details
//!
//! **Syntax.** `(alias name expansion)`: exactly a name and one expression.
//! `(macro (name params...) "doc" body)`: the signature is a list of names, the
//! doc string is optional (a `macro` with only a signature and a string has
//! that string as its body). `(macro (name) body)` has no parameters, so it is
//! the same as `(alias name body)`. Parameter names must be distinct.
//!
//! **Rest parameter.** A final parameter written with three trailing dots,
//! `rest...`, collects every remaining argument (possibly none). In the body it
//! is written without the dots, `rest`, and must stand as an element of a list:
//! the arguments are spliced in its place, so `(paren a rest)` with
//! `(pair 1 2 3)` becomes `(paren 1 2 3)`. A body that is just `rest` is a
//! `BadDefinition`. Only the last parameter may have the dots.
//!
//! **Where definitions are found.** In any code form of the document: a block
//! of its own (`BlockKind::Code`) or a form inside a sentence, list item,
//! block quote, heading or link (`InlineKind::Code`).
//!
//! **Reserved names.** A definition may not use the name of a keyword or one
//! of its second names (`add`), a display name (`tau`), a reserved head
//! (`show`, `eval`, `let`, `alias`, `macro`, `ref`) or `tag` (`ShadowsBuiltin`).
//!
//! **Uses.** An alias is used as a bare name, anywhere, including the head of a
//! list (`(R x)` becomes `((bb R) x)`). A macro with parameters is used as the
//! head of a list; written bare, it is left alone (it is just a variable).
//!
//! **Order of expansion.** The arguments of a call are expanded first, then
//! substituted into the template, and the rest of the template is expanded
//! again. Substituted arguments are not expanded a second time.
//!
//! **Self-reference.** While a template is being instantiated, uses of that
//! same name *in the template* are left alone (`(alias R (bb R))` gives
//! `(bb R)`). Only the innermost definition is blocked, so two definitions
//! that use each other, `(alias A (paren B))` and `(alias B (paren A))`, keep
//! going until [`MAX_DEPTH`] nested instantiations and then stop with one
//! `MacroDepth` diagnostic.
//!
//! **Reserved words in a template.** `(alias T (tag a))`, `(macro (f x) (eval
//! x))` and the like are rejected: `MisplacedTag` for a tag, `NestedEval` for
//! `eval`, `BadDefinition` for `let`/`alias`/`macro`, `Other` for
//! `show`/`ref`. A reserved word that is not the head of a list is just a
//! name and is fine.
//!
//! **Failure.** A use with the wrong number of arguments (`MacroArity`) or one
//! that nests too deeply (`MacroDepth`) is replaced by an `Error` node with the
//! use-site span, so the form renders as an error box. The same happens if one
//! expansion would produce more than 100 000 nodes or nest more than 320
//! levels deep (also `MacroDepth`). After the first `MacroDepth` in a tree,
//! the remaining uses in that tree are replaced silently, so one runaway
//! definition gives one diagnostic.
//!
//! **Document budget.** Every node a template or a copied argument creates is
//! charged to the document-wide budget described in the `walk` module
//! (250 000 nodes per `analyze`). [`expand`] on its own starts a fresh
//! budget; `analyze` shares one across the whole note and the `let` pass.

use std::collections::HashMap;

use mazer_syntax::{Diag, DiagCode, Diags, Document, Node, NodeKind, Span, Sym};

use crate::walk::{self, Budget, Measure};

/// Maximum macro expansion depth before giving up with a diagnostic.
pub const MAX_DEPTH: usize = 64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Macro {
    pub name: Sym,
    /// Empty for an alias.
    pub params: Vec<Sym>,
    /// A final parameter that collects the remaining arguments, if any.
    pub rest: Option<Sym>,
    /// The optional string after the signature. Shown on LSP hover.
    pub doc: Option<Box<str>>,
    /// The template.
    pub body: Node,
    /// Where it was defined, for go-to-definition.
    pub span: Span,
}

impl Macro {
    /// Whether this is an alias (a bare name) rather than a macro that is
    /// called with arguments.
    #[must_use]
    pub const fn is_alias(&self) -> bool {
        self.params.is_empty() && self.rest.is_none()
    }

    /// How the definition is called, e.g. `(laplace f s)` or `(pair a rest...)`;
    /// for an alias, just its name.
    fn signature(&self) -> String {
        if self.is_alias() {
            return self.name.to_string();
        }
        let mut out = format!("({}", self.name);
        for p in &self.params {
            out.push(' ');
            out.push_str(p.as_str());
        }
        if let Some(rest) = self.rest {
            out.push(' ');
            out.push_str(rest.as_str());
            out.push_str("...");
        }
        out.push(')');
        out
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Macros {
    by_name: HashMap<Sym, Macro>,
}

impl Macros {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: Sym) -> Option<&Macro> {
        self.by_name.get(&name)
    }

    /// Inserts a definition, returning the previous one of the same name.
    pub fn insert(&mut self, def: Macro) -> Option<Macro> {
        self.by_name.insert(def.name, def)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Macro> {
        self.by_name.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Collects every `alias` and `macro` definition in `doc` into `macros`,
/// reporting malformed, duplicate and shadowing definitions.
pub fn collect_macros(doc: &Document, macros: &mut Macros, diags: &mut Diags) {
    walk::for_each_code(doc, &mut |node, _| {
        let parsed = if node.is_form("alias") {
            parse_alias(node, diags)
        } else if node.is_form("macro") {
            parse_macro(node, diags)
        } else {
            return;
        };
        let Some((def, name_span)) = parsed else {
            return;
        };
        let mut body = def.body.clone();
        let outer = if node.is_form("alias") { "alias" } else { "macro" };
        if walk::strip_reserved(&mut body, outer, diags) {
            return;
        }
        if let Some(prev) = macros.get(def.name) {
            diags.push(
                Diag::error(
                    DiagCode::DuplicateDefinition,
                    name_span,
                    format!("`{}` is already defined", def.name),
                )
                .with_label(prev.span, "first defined here"),
            );
            return;
        }
        macros.insert(def);
    });
}

/// `(alias name expansion)`. Returns the definition and the span of its name.
fn parse_alias(node: &Node, diags: &mut Diags) -> Option<(Macro, Span)> {
    if walk::contains_error(node) {
        return None; // the reader already reported it
    }
    let [name, body] = node.args() else {
        diags.push(
            Diag::error(DiagCode::BadDefinition, node.span, "an alias needs a name and one expression")
                .with_help("write it as (alias name expansion)"),
        );
        return None;
    };
    let sym = definition_name(name, diags)?;
    Some((
        Macro {
            name: sym,
            params: Vec::new(),
            rest: None,
            doc: None,
            body: body.clone(),
            span: node.span,
        },
        name.span,
    ))
}

/// `(macro (name params...) "doc" body)`.
fn parse_macro(node: &Node, diags: &mut Diags) -> Option<(Macro, Span)> {
    if walk::contains_error(node) {
        return None;
    }
    let bad = |diags: &mut Diags, span: Span, message: &str| {
        diags.push(
            Diag::error(DiagCode::BadDefinition, span, message)
                .with_help("write it as (macro (name params...) \"optional doc\" body)"),
        );
    };
    let (sig, doc, body) = match node.args() {
        [sig, body] => (sig, None, body),
        [sig, doc, body] => {
            let NodeKind::Str(text) = &doc.kind else {
                bad(diags, doc.span, "expected a doc string here");
                return None;
            };
            (sig, Some(text.clone()), body)
        }
        _ => {
            bad(diags, node.span, "a macro needs a signature and a body");
            return None;
        }
    };
    let Some((first, params)) = sig.as_list().and_then(<[Node]>::split_first) else {
        bad(diags, sig.span, "expected a signature such as (name x y)");
        return None;
    };
    let name = definition_name(first, diags)?;
    let mut names: Vec<Sym> = Vec::with_capacity(params.len());
    let mut rest = None;
    for (i, param) in params.iter().enumerate() {
        let Some(sym) = param.as_ident() else {
            bad(diags, param.span, "a parameter must be a name");
            return None;
        };
        let text = sym.as_str();
        let (sym, is_rest) = match text.strip_suffix("...") {
            Some(base) if !base.is_empty() => (Sym::new(base), true),
            _ => (sym, false),
        };
        if is_rest && i + 1 != params.len() {
            bad(diags, param.span, "only the last parameter can collect the rest (`name...`)");
            return None;
        }
        if names.contains(&sym) || rest == Some(sym) {
            bad(diags, param.span, &format!("parameter `{sym}` appears twice"));
            return None;
        }
        if is_rest {
            rest = Some(sym);
        } else {
            names.push(sym);
        }
    }
    if let Some(rest) = rest
        && body.as_ident() == Some(rest)
    {
        bad(
            diags,
            body.span,
            &format!("the rest parameter `{rest}` can only be used inside a list, e.g. (paren {rest})"),
        );
        return None;
    }
    Some((
        Macro {
            name,
            params: names,
            rest,
            doc,
            body: body.clone(),
            span: node.span,
        },
        first.span,
    ))
}

/// The defined name: an identifier that is not built in.
fn definition_name(node: &Node, diags: &mut Diags) -> Option<Sym> {
    let Some(sym) = node.as_ident() else {
        diags.push(Diag::error(DiagCode::BadDefinition, node.span, "expected the name being defined"));
        return None;
    };
    if let Some(kind) = walk::builtin_kind(sym, true) {
        diags.push(
            Diag::error(DiagCode::ShadowsBuiltin, node.span, format!("`{sym}` is {kind}"))
                .with_help("choose another name for this definition"),
        );
        return None;
    }
    Some(sym)
}

/// Expands every macro and alias use in one code tree.
pub fn expand(node: &Node, macros: &Macros, diags: &mut Diags) -> Node {
    expand_with(node, macros, &mut Budget::new(), diags)
}

/// [`expand`], charging created nodes to a shared document budget.
pub(crate) fn expand_with(node: &Node, macros: &Macros, budget: &mut Budget, diags: &mut Diags) -> Node {
    if macros.is_empty() {
        return node.clone();
    }
    let mut expander = Expander {
        macros,
        diags,
        budget,
        depth: 0,
        produced: 0,
        stopped: false,
    };
    expander.node(node, None, 1)
}

/// One instantiation of a template: which definition, its (already expanded)
/// arguments, and the span every template node takes.
struct Inst<'a> {
    def: &'a Macro,
    args: &'a [Node],
    use_span: Span,
}

impl<'a> Inst<'a> {
    /// The argument a parameter name stands for.
    fn param(&self, name: Sym) -> Option<&'a Node> {
        let i = self.def.params.iter().position(|&p| p == name)?;
        self.args.get(i)
    }

    /// The arguments a rest parameter name stands for.
    fn rest(&self, name: Sym) -> Option<&'a [Node]> {
        (self.def.rest == Some(name)).then(|| self.args.get(self.def.params.len()..).unwrap_or(&[]))
    }

    fn blocks(&self, name: Sym) -> bool {
        self.def.name == name
    }
}

struct Expander<'m, 'd> {
    macros: &'m Macros,
    diags: &'d mut Diags,
    /// The document-wide budget for created nodes.
    budget: &'d mut Budget,
    /// Instantiations currently open.
    depth: usize,
    /// Nodes produced so far, against [`walk::MAX_NODES`].
    produced: usize,
    /// Set after the first `MacroDepth`: every later use becomes an `Error`.
    stopped: bool,
}

impl Expander<'_, '_> {
    /// Expands `node`. `inst` is set when `node` is part of a template being
    /// instantiated. `level` is the depth of the result in the output tree.
    fn node(&mut self, node: &Node, inst: Option<&Inst<'_>>, level: usize) -> Node {
        let span = inst.map_or(node.span, |i| i.use_span);
        match &node.kind {
            NodeKind::Ident(sym) => {
                if let Some(arg) = inst.and_then(|i| i.param(*sym)) {
                    return self.frozen(arg, span, level);
                }
                match self.macros.get(*sym) {
                    Some(def) if def.is_alias() && !inst.is_some_and(|i| i.blocks(*sym)) => {
                        self.call(def, &[], span, level)
                    }
                    _ => self.leaf(node, inst.is_some(), span),
                }
            }
            NodeKind::List(items) => {
                if level > walk::MAX_TREE_DEPTH {
                    return self.too_big(span, None);
                }
                self.produced += 1;
                if inst.is_some() {
                    self.budget.spend(1);
                }
                let mut out = Vec::with_capacity(items.len());
                for child in items {
                    if let Some(args) = child.as_ident().and_then(|s| inst.and_then(|i| i.rest(s))) {
                        for arg in args {
                            out.push(self.frozen(arg, span, level + 1));
                        }
                    } else {
                        out.push(self.node(child, inst, level + 1));
                    }
                }
                let call = out.first().and_then(Node::as_ident).and_then(|head| {
                    let def = self.macros.get(head)?;
                    (!def.is_alias() && !inst.is_some_and(|i| i.blocks(head))).then_some(def)
                });
                match call {
                    Some(def) => self.call(def, &out[1..], span, level),
                    None => Node::list(out, span),
                }
            }
            NodeKind::Number(_) | NodeKind::Str(_) | NodeKind::Error => self.leaf(node, inst.is_some(), span),
        }
    }

    /// Copies a leaf; `created` when it comes from a template.
    fn leaf(&mut self, node: &Node, created: bool, span: Span) -> Node {
        self.produced += 1;
        if created {
            self.budget.spend(1);
        }
        Node::new(node.kind.clone(), span)
    }

    /// A substituted argument: copied as it is (it was expanded already).
    fn frozen(&mut self, arg: &Node, span: Span, level: usize) -> Node {
        let m = Measure::of(arg);
        self.produced = self.produced.saturating_add(m.size);
        if self.produced > walk::MAX_NODES || level + m.depth > walk::MAX_TREE_DEPTH + 1 {
            return self.too_big(span, None);
        }
        if !self.budget.try_spend(m.size) {
            return self.over_budget(span, None);
        }
        arg.clone()
    }

    /// Instantiates `def` with already expanded `args` at the use `span`.
    fn call(&mut self, def: &Macro, args: &[Node], span: Span, level: usize) -> Node {
        if self.stopped {
            return Node::error(span);
        }
        if !self.budget.fits() {
            return self.over_budget(span, Some(def));
        }
        let fits = if def.rest.is_some() {
            args.len() >= def.params.len()
        } else {
            args.len() == def.params.len()
        };
        if !fits {
            let wanted = if def.rest.is_some() {
                format!("at least {}", def.params.len())
            } else {
                def.params.len().to_string()
            };
            self.diags.push(
                Diag::error(
                    DiagCode::MacroArity,
                    span,
                    format!("`{}` takes {wanted} argument{}, found {}", def.name, plural(&wanted), args.len()),
                )
                .with_label(def.span, "defined here")
                .with_help(format!("write it as {}", def.signature())),
            );
            return Node::error(span);
        }
        if self.depth >= MAX_DEPTH {
            self.stopped = true;
            self.diags.push(
                Diag::error(
                    DiagCode::MacroDepth,
                    span,
                    format!("expanding `{}` nests more than {MAX_DEPTH} definitions deep", def.name),
                )
                .with_label(def.span, "defined here")
                .with_help("definitions that use each other must stop somewhere"),
            );
            return Node::error(span);
        }
        if self.produced > walk::MAX_NODES {
            return self.too_big(span, Some(def));
        }
        self.depth += 1;
        let inst = Inst {
            def,
            args,
            use_span: span,
        };
        let out = self.node(&def.body, Some(&inst), level);
        self.depth -= 1;
        out
    }

    /// The document budget has run out: one diagnostic for the whole note,
    /// then every later use is refused silently.
    fn over_budget(&mut self, span: Span, def: Option<&Macro>) -> Node {
        self.stopped = true;
        self.budget.refuse(DiagCode::MacroDepth, span, def.map(|d| d.span), self.diags);
        Node::error(span)
    }

    fn too_big(&mut self, span: Span, def: Option<&Macro>) -> Node {
        if !self.stopped {
            self.stopped = true;
            let mut diag = Diag::error(
                DiagCode::MacroDepth,
                span,
                format!(
                    "this expansion is too large (more than {} nodes or {} levels)",
                    walk::MAX_NODES,
                    walk::MAX_TREE_DEPTH
                ),
            );
            if let Some(def) = def {
                diag = diag.with_label(def.span, "defined here");
            }
            self.diags.push(diag);
        }
        Node::error(span)
    }
}

fn plural(count: &str) -> &'static str {
    if count.ends_with(" 1") || count == "1" { "" } else { "s" }
}

#[cfg(test)]
mod tests;
