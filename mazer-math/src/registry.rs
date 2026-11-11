//! The table of all keywords, built once at startup.

use std::collections::HashMap;
use std::sync::LazyLock;

use mazer_syntax::Sym;

use crate::keyword::Keyword;

#[derive(Debug, Default)]
pub struct Registry {
    keywords: Vec<Keyword>,
    /// Primary names and aliases, both pointing at an index in `keywords`.
    by_name: HashMap<Sym, usize>,
}

impl Registry {
    /// Builds the lookup table. If two keywords claim the same name the first
    /// one wins here; the registry test reports the clash.
    #[must_use]
    pub fn from_keywords(keywords: Vec<Keyword>) -> Self {
        let mut by_name = HashMap::new();
        for (index, keyword) in keywords.iter().enumerate() {
            for name in keyword.names() {
                by_name.entry(Sym::new(name)).or_insert(index);
            }
        }
        Self { keywords, by_name }
    }

    /// Looks a keyword up by its name or one of its aliases.
    #[must_use]
    pub fn get(&self, name: Sym) -> Option<&Keyword> {
        self.by_name.get(&name).map(|&i| &self.keywords[i])
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Keyword> {
        self.keywords.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.keywords.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }

    /// The keyword name closest to `name`, if one is within a small edit
    /// distance. Used for did-you-mean on a list head that is not a keyword.
    ///
    /// The distance is counted in characters, and swapping two neighbouring
    /// characters counts as one edit. How many edits are allowed grows with
    /// the length of `name` (see [`max_distance`]): short names such as `f`,
    /// `min` or `sup` are almost always deliberate variables and would
    /// otherwise be "one typo" from `+`, `sin` or `sum`. A name that merely
    /// extends a keyword (`sinh` from `sin`) is also taken to be deliberate.
    /// A name that is itself a keyword gets no suggestion. Ties go to the
    /// keyword registered first.
    #[must_use]
    pub fn suggest(&self, name: &str) -> Option<&'static str> {
        let limit = max_distance(name.chars().count());
        if limit == 0 || self.get(Sym::new(name)).is_some() {
            return None;
        }
        let mut best: Option<(usize, &'static str)> = None;
        for candidate in self.keywords.iter().flat_map(Keyword::names) {
            if name.starts_with(candidate) {
                continue;
            }
            let d = edit_distance(name, candidate);
            if d <= limit && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, candidate));
            }
        }
        best.map(|(_, candidate)| candidate)
    }
}

/// How many edits [`Registry::suggest`] allows for a name of `len` characters.
///
/// The old interpreter allowed 2 edits for every name. That is fine for long
/// names but makes every one- to three-letter variable a "typo" of some
/// keyword, so short names get no suggestion at all.
const fn max_distance(len: usize) -> usize {
    match len {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// Optimal-string-alignment distance between `a` and `b`, in characters:
/// Levenshtein (insert, delete, substitute) plus swapping two neighbouring
/// characters as a single edit, so `fracc` and `farc` are both one typo away
/// from `frac`.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Rolling rows: two rows back (needed for swaps), the previous row, the current row.
    let mut before = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(before[j - 2] + 1);
            }
            cur[j] = d;
        }
        std::mem::swap(&mut before, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

impl<'a> IntoIterator for &'a Registry {
    type Item = &'a Keyword;
    type IntoIter = std::slice::Iter<'a, Keyword>;

    fn into_iter(self) -> Self::IntoIter {
        self.keywords.iter()
    }
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    #[allow(unused_mut)]
    let mut keywords = crate::keywords::all();
    #[cfg(feature = "latex")]
    crate::latex::attach(&mut keywords);
    Registry::from_keywords(keywords)
});

/// The global keyword registry.
#[must_use]
pub fn registry() -> &'static Registry {
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::{Registry, edit_distance, registry};
    use crate::call::Call;
    use crate::keyword::Keyword;
    use crate::mathml::MathMl;

    fn noop(_: &Call<'_>, _: &mut MathMl<'_>) {}

    #[test]
    fn distance() {
        assert_eq!(edit_distance("frac", "frac"), 0);
        assert_eq!(edit_distance("frc", "frac"), 1);
        assert_eq!(edit_distance("fracc", "frac"), 1);
        assert_eq!(edit_distance("frak", "frac"), 1);
        assert_eq!(edit_distance("farc", "frac"), 1, "a swap is one edit");
        assert_eq!(edit_distance("intergal", "integral"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("τx", "τy"), 1, "counted in characters, not bytes");
        assert_eq!(edit_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn suggest_from_the_builtin_keywords() {
        let r = registry();
        assert_eq!(r.suggest("intgral"), Some("integral"));
        assert_eq!(r.suggest("intergal"), Some("integral"));
        assert_eq!(r.suggest("farc"), Some("frac"));
        assert_eq!(r.suggest("Frac"), Some("frac"));
        assert_eq!(r.suggest("subscrpt"), Some("subscript"));
        assert_eq!(r.suggest("subscirpt"), Some("subscript"));
        // A keyword itself, and names that are clearly not typos.
        assert_eq!(r.suggest("frac"), None);
        assert_eq!(r.suggest("xyzzy"), None);
        // Short names never get a suggestion.
        for name in ["f", "x", "+x", "sni", "min"] {
            assert_eq!(r.suggest(name), None, "{name}");
        }
        // A name extending a keyword is deliberate.
        assert_eq!(r.suggest("sinh"), None);
        assert_eq!(r.suggest("fracs"), None);
    }

    #[test]
    fn suggest_limits_and_ties() {
        let r = Registry::from_keywords(vec![
            Keyword::new("abcd", "T.", noop),
            Keyword::new("abce", "T.", noop).alias("longername"),
        ]);
        // One edit from both: the first registered wins.
        assert_eq!(r.suggest("abcf"), Some("abcd"));
        // Closer beats earlier.
        assert_eq!(r.suggest("abce"), None, "a keyword itself");
        assert_eq!(r.suggest("xbce"), Some("abce"));
        // Aliases are candidates; long names allow two edits, short ones one.
        assert_eq!(r.suggest("lnogernmae"), Some("longername"));
        assert_eq!(r.suggest("abxy"), None);
    }
}
