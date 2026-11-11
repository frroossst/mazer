//! Where expanded notation came from, so a diagnostic found in an expansion
//! can point at the template that caused it.
//!
//! # The mechanism
//!
//! Expanded nodes take the span of the place the alias or macro was *used*
//! (the agreed design), so on its own a diagnostic about an expanded node
//! cannot say which part of which template it is about. During `analyze`
//! every node instantiated from a template therefore first gets a
//! *provisional span*: an id in the reserved file [`PROVISIONAL`], with an
//! [`Origin`] in the [`Expansions`] side table recording the template node it
//! was copied from (its real span inside the definition), the definition, and
//! the span of the use it was instantiated for. That use span is itself
//! provisional when the use was written inside another template, so nested
//! expansions form a chain from the innermost template out to the place in
//! the note where the outermost notation was used.
//!
//! Checking runs on these trees. Then every diagnostic is translated
//! ([`Expansions::translate`]): its primary span becomes the real use site in
//! the note, a secondary label points at the template form inside the
//! definition ("in the expansion of `tuple`, defined here"), outer
//! definitions of a nested expansion get a label each (up to three), and the
//! help names the definition. Finally the trees are re-spanned
//! ([`Expansions::respan`]) so that [`crate::Analysis::resolved`] carries
//! only real use-site spans, as before.
//!
//! The public [`crate::expand`] does not use provisional spans.

use std::collections::{HashMap, HashSet};

use mazer_syntax::{ByteOffset, Diag, DiagCode, FileId, Node, NodeKind, Span, Sym};

/// The reserved file id of provisional spans. ([`Span::DUMMY`] uses
/// `u16::MAX`.)
pub const PROVISIONAL: FileId = FileId(u16::MAX - 1);

/// Labels for the definitions around the innermost one, at most.
const OUTER_LABELS: usize = 3;

/// Where one instantiated node came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Origin {
    /// The template node it was copied from, as written in the definition.
    /// `None` for the stand-in of a parameter while a definition is checked
    /// on its own.
    pub template: Option<Span>,
    /// The definition being instantiated.
    pub def: Sym,
    /// Where it was instantiated: the span of the use, provisional when the
    /// use itself came from a template.
    pub used_at: Span,
}

/// A closed problem: found by checking a definition on its own, keyed by
/// code, message and the template spans from the innermost template out to
/// the definition's own template form.
pub type ClosedKey = (DiagCode, String, Vec<Span>);

/// The side table of provisional spans.
#[derive(Debug, Default)]
pub struct Expansions {
    origins: Vec<Origin>,
}

/// Whether `span` is provisional.
pub fn is_provisional(span: Span) -> bool {
    span.file == PROVISIONAL
}

impl Expansions {
    /// A fresh provisional span for a node with this origin.
    pub fn fresh(&mut self, origin: Origin) -> Span {
        let id = ByteOffset::from_usize(self.origins.len());
        self.origins.push(origin);
        Span::new(PROVISIONAL, id, id)
    }

    pub const fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }

    pub fn origin(&self, span: Span) -> Option<&Origin> {
        if is_provisional(span) {
            self.origins.get(span.start.to_usize())
        } else {
            None
        }
    }

    /// The real span `span` stands for: itself when it is real, else the
    /// place in the note where the outermost notation was used. (A use span
    /// is always allocated before the nodes instantiated for it, so this
    /// ends.)
    pub fn real(&self, mut span: Span) -> Span {
        while let Some(origin) = self.origin(span) {
            span = origin.used_at;
        }
        span
    }

    /// The template origins of `span`, innermost first.
    pub fn chain(&self, mut span: Span) -> Vec<Origin> {
        let mut out = Vec::new();
        while let Some(origin) = self.origin(span) {
            if origin.template.is_some() {
                out.push(*origin);
            }
            span = origin.used_at;
        }
        out
    }

    /// The template spans of a chain, innermost first.
    pub fn key_spans(chain: &[Origin]) -> Vec<Span> {
        chain.iter().filter_map(|o| o.template).collect()
    }

    /// Whether `diag` (primary span provisional) repeats a closed problem of
    /// one of the definitions it was expanded through.
    pub fn is_closed(&self, diag: &Diag, closed: &HashSet<ClosedKey>) -> bool {
        if closed.is_empty() {
            return false;
        }
        let spans = Self::key_spans(&self.chain(diag.span));
        (1..=spans.len()).any(|n| closed.contains(&(diag.code, diag.message.clone(), spans[..n].to_vec())))
    }

    /// Replaces every provisional span in `node` by the real span it stands
    /// for.
    pub fn respan(&self, node: &mut Node) {
        if self.origins.is_empty() {
            return;
        }
        node.span = self.real(node.span);
        if let NodeKind::List(items) = &mut node.kind {
            for item in items {
                self.respan(item);
            }
        }
    }

    /// Translates a diagnostic found on provisional trees (see the module
    /// docs). `at_definition` is set when the diagnostic comes from checking
    /// a definition on its own: then the primary span is the template form
    /// inside that definition rather than a use. `calls` gives the head and
    /// argument count of an expanded list, for the help of an `Arity`
    /// problem.
    pub fn translate(&self, diag: &Diag, at_definition: bool, calls: &dyn Fn(Span) -> Option<(Sym, usize)>) -> Diag {
        let chain = self.chain(diag.span);
        let to_real = |span: Span| -> Span {
            if at_definition {
                self.chain(span).last().and_then(|o| o.template).unwrap_or_else(|| self.real(span))
            } else {
                self.real(span)
            }
        };
        let primary = to_real(diag.span);
        let mut labels: Vec<(Span, String)> = Vec::new();
        let mut add = |span: Span, text: String| {
            if span != primary && !labels.iter().any(|(s, t)| *s == span && *t == text) {
                labels.push((span, text));
            }
        };
        for (span, text) in &diag.labels {
            add(to_real(*span), text.clone());
        }
        // At a definition the outermost level is the definition itself,
        // where the primary span already is.
        let levels = if at_definition { chain.len().saturating_sub(1) } else { chain.len() };
        for (i, origin) in chain.iter().take(levels).enumerate() {
            let Some(template) = origin.template else {
                continue;
            };
            if i == 0 {
                add(template, format!("in the expansion of `{}`, defined here", origin.def));
            } else if i <= OUTER_LABELS {
                add(template, format!("`{}` is used in the expansion of `{}` here", chain[i - 1].def, origin.def));
            }
        }
        let mut out = Diag::new(diag.severity, diag.code, primary, diag.message.clone());
        out.labels = labels;
        out.help.clone_from(&diag.help);
        if let Some(inner) = chain.first().filter(|_| levels > 0) {
            let what = match (diag.code, calls(diag.span)) {
                (DiagCode::Arity, Some((head, count))) => format!(
                    "the template of `{}` calls `{head}` with {count} argument{}",
                    inner.def,
                    if count == 1 { "" } else { "s" }
                ),
                _ => format!("this comes from the template of `{}`", inner.def),
            };
            out.help = Some(match &diag.help {
                Some(help) => format!("{what}; {help}"),
                None => what,
            });
        }
        out
    }
}

/// For every provisional list node in `nodes`: its head (when a name) and
/// argument count, for the help of an `Arity` problem. Lists sharing a span
/// (a substituted `let` value) keep the outermost.
pub fn index_calls<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> HashMap<Span, (Sym, usize)> {
    fn walk(node: &Node, out: &mut HashMap<Span, (Sym, usize)>) {
        if let NodeKind::List(items) = &node.kind {
            if is_provisional(node.span)
                && let Some(head) = node.head()
            {
                out.entry(node.span).or_insert((head, items.len() - 1));
            }
            for item in items {
                walk(item, out);
            }
        }
    }
    let mut out = HashMap::new();
    for node in nodes {
        walk(node, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_resolve_to_the_outermost_use() {
        let mut exp = Expansions::default();
        let note = |a: u32, b: u32| Span::new(FileId(0), ByteOffset(a), ByteOffset(b));
        let outer_use = note(100, 110);
        let outer = exp.fresh(Origin {
            template: Some(note(10, 20)),
            def: Sym::new("outer"),
            used_at: outer_use,
        });
        let inner = exp.fresh(Origin {
            template: Some(note(30, 40)),
            def: Sym::new("inner"),
            used_at: outer,
        });
        assert_eq!(exp.real(inner), outer_use);
        let chain = exp.chain(inner);
        assert_eq!(Expansions::key_spans(&chain), [note(30, 40), note(10, 20)]);
        let diag = Diag::error(DiagCode::Other, inner, "m").with_help("h");
        let out = exp.translate(&diag, false, &|_| None);
        assert_eq!(out.span, outer_use);
        assert_eq!(out.labels[0], (note(30, 40), "in the expansion of `inner`, defined here".to_owned()));
        assert_eq!(out.labels[1], (note(10, 20), "`inner` is used in the expansion of `outer` here".to_owned()));
        assert_eq!(out.help.as_deref(), Some("this comes from the template of `inner`; h"));
        let mut tree = Node::list(vec![Node::ident("a", inner)], outer);
        exp.respan(&mut tree);
        assert_eq!(tree.span, outer_use);
        assert_eq!(tree.as_list().unwrap()[0].span, outer_use);
    }
}
