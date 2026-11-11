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
    #[must_use]
    pub fn suggest(&self, name: &str) -> Option<&'static str> {
        let _ = name;
        todo!("step 1, math core agent")
    }
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
