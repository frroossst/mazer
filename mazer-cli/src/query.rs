//! `mazer query`: search keywords, display names and a note's macros.

use std::fmt::Write as _;

use mazer_doc::Macro;
use mazer_doc::SourceMap;
use mazer_doc::math::{DisplayName, Keyword, ParamKind, display_names, registry};

/// How well an entry's name matches the query. Lower sorts first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rank {
    Exact,
    Prefix,
    Other,
}

/// One search result: what to sort by and what to print.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Hit {
    pub rank: Rank,
    pub name: String,
    pub text: String,
}

/// Ranks a candidate. `names` are the spellings it answers to, `others` the
/// remaining searchable text (docs). `needle` is already lower case.
fn rank(needle: &str, names: &[&str], others: &[&str]) -> Option<Rank> {
    let lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    if lower.iter().any(|n| n == needle) {
        Some(Rank::Exact)
    } else if lower.iter().any(|n| n.starts_with(needle)) {
        Some(Rank::Prefix)
    } else if lower.iter().any(|n| n.contains(needle))
        || others.iter().any(|o| o.to_lowercase().contains(needle))
    {
        Some(Rank::Other)
    } else {
        None
    }
}

fn describe_keyword(k: &Keyword) -> String {
    let mut out = format!("{}    (keyword)\n", k.signature());
    if !k.aliases.is_empty() {
        let _ = writeln!(out, "  aliases: {}", k.aliases.join(", "));
    }
    let _ = writeln!(out, "  {}", k.doc);
    for p in &k.params {
        let mark = match p.kind {
            ParamKind::Required => "",
            ParamKind::Optional => " (optional)",
            ParamKind::Rest => " (rest)",
        };
        let _ = writeln!(out, "    {}: {}{mark}", p.name, p.doc);
    }
    if !k.examples.is_empty() {
        out.push_str("  examples:\n");
        for e in &k.examples {
            let _ = writeln!(out, "    {e}");
        }
    }
    out
}

fn describe_display(d: &DisplayName) -> String {
    format!("{} \u{2192} {}    (display name)\n  {}\n", d.name, d.unicode, d.doc)
}

fn macro_signature(m: &Macro) -> String {
    if m.is_alias() {
        return m.name.to_string();
    }
    let mut out = format!("({}", m.name);
    for p in &m.params {
        out.push(' ');
        out.push_str(p.as_str());
    }
    if let Some(rest) = m.rest {
        let _ = write!(out, " {rest}...");
    }
    out.push(')');
    out
}

fn describe_macro(m: &Macro, sources: &SourceMap) -> String {
    let kind = if m.is_alias() { "alias" } else { "macro" };
    let mut out = format!("{}    ({kind})\n", macro_signature(m));
    if let Some(doc) = &m.doc {
        let _ = writeln!(out, "  {doc}");
    }
    let place = sources.file(m.span.file).map_or_else(
        || String::from("?"),
        |f| {
            let (line, col) = sources.line_col(m.span.file, m.span.start);
            format!("{}:{}:{}", f.name, line.0 + 1, col.0 + 1)
        },
    );
    let _ = writeln!(out, "  defined at {place}");
    let _ = writeln!(out, "  = {}", m.body);
    out
}

/// All hits for `text`, best first: exact name matches, then prefix matches,
/// then the rest, each alphabetically. `macros` come from `--file`.
#[must_use]
pub fn search<'a>(
    text: &str,
    macros: impl IntoIterator<Item = &'a Macro>,
    sources: &SourceMap,
) -> Vec<Hit> {
    let needle = text.trim().to_lowercase();
    let mut hits = Vec::new();

    for k in registry() {
        let names: Vec<&str> = k.names().collect();
        if let Some(rank) = rank(&needle, &names, &[k.doc]) {
            hits.push(Hit {
                rank,
                name: k.name.to_owned(),
                text: describe_keyword(k),
            });
        }
    }
    for d in display_names() {
        if let Some(rank) = rank(&needle, &[d.name], &[d.doc, d.unicode]) {
            hits.push(Hit {
                rank,
                name: d.name.to_owned(),
                text: describe_display(d),
            });
        }
    }
    for m in macros {
        let name = m.name.as_str();
        let doc = m.doc.as_deref().unwrap_or("");
        if let Some(rank) = rank(&needle, &[name], &[doc]) {
            hits.push(Hit {
                rank,
                name: name.to_owned(),
                text: describe_macro(m, sources),
            });
        }
    }

    hits.sort_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.name.as_str()).collect()
    }

    #[test]
    fn finds_integral_first() {
        let hits = search("integral", [], &SourceMap::new());
        assert_eq!(hits[0].name, "integral");
        assert_eq!(hits[0].rank, Rank::Exact);
        assert!(hits[0].text.contains("(integral "), "{}", hits[0].text);
    }

    #[test]
    fn finds_display_name() {
        let hits = search("tau", [], &SourceMap::new());
        let tau = hits.iter().find(|h| h.name == "tau").expect("tau");
        assert_eq!(tau.rank, Rank::Exact);
        assert!(tau.text.contains("tau \u{2192} \u{3c4}"), "{}", tau.text);
    }

    #[test]
    fn case_insensitive() {
        let a = search("INTEGRAL", [], &SourceMap::new());
        let b = search("integral", [], &SourceMap::new());
        assert_eq!(names(&a), names(&b));
    }

    #[test]
    fn ranking_groups_are_ordered_and_sorted() {
        let hits = search("in", [], &SourceMap::new());
        assert!(hits.len() > 1);
        for pair in hits.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            assert!(a.rank <= b.rank, "{} before {}", a.name, b.name);
            if a.rank == b.rank {
                assert!(
                    a.name.to_lowercase() <= b.name.to_lowercase(),
                    "{} before {}",
                    a.name,
                    b.name
                );
            }
        }
        // An exact match (the `in` keyword) leads.
        assert_eq!(hits[0].rank, Rank::Exact);
        assert!(hits.iter().any(|h| h.rank == Rank::Prefix));
    }

    #[test]
    fn rank_function() {
        assert_eq!(rank("a", &["a"], &[]), Some(Rank::Exact));
        assert_eq!(rank("a", &["B", "Ab"], &[]), Some(Rank::Prefix));
        assert_eq!(rank("a", &["xa"], &[]), Some(Rank::Other));
        assert_eq!(rank("a", &["x"], &["has A inside"]), Some(Rank::Other));
        assert_eq!(rank("a", &["x"], &["nope"]), None);
    }

    #[test]
    fn no_match() {
        assert_eq!(search("zzzzqqqq", [], &SourceMap::new()).len(), 0);
    }

    #[test]
    fn note_macros_are_searched() {
        let analysis =
            mazer_doc::analyze("(macro (zorp a) \"Zorps a.\" (frac a 2))\n", "n.zr");
        let hits = search("zorp", analysis.macros.iter(), &analysis.sources);
        assert_eq!(hits[0].name, "zorp");
        assert!(hits[0].text.contains("(zorp a)"), "{}", hits[0].text);
        assert!(hits[0].text.contains("n.zr:1:"), "{}", hits[0].text);
        assert!(hits[0].text.contains("Zorps a."), "{}", hits[0].text);
    }
}
