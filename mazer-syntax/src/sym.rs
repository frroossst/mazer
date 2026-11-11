//! Interned names. One global table (the approach rustc uses for its own
//! `Symbol`), so a [`Sym`] is a `Copy` integer that can be created and turned
//! back into text from anywhere, with no interner to pass around.

use std::collections::HashMap;
use std::fmt;
use std::sync::{LazyLock, Mutex, PoisonError};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

#[derive(Default)]
struct Interner {
    by_name: HashMap<&'static str, u32>,
    names: Vec<&'static str>,
}

static INTERNER: LazyLock<Mutex<Interner>> = LazyLock::new(|| Mutex::new(Interner::default()));

impl Sym {
    /// Interns `name`. The same text always yields the same `Sym`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        let mut table = INTERNER.lock().unwrap_or_else(PoisonError::into_inner);
        let id = table.by_name.get(name).copied().unwrap_or_else(|| {
            // Names live for the whole process; leaking is what lets `as_str`
            // return `&'static str` without holding the lock.
            let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
            let id = u32::try_from(table.names.len()).unwrap_or(u32::MAX);
            table.names.push(leaked);
            table.by_name.insert(leaked, id);
            id
        });
        drop(table);
        Self(id)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        let table = INTERNER.lock().unwrap_or_else(PoisonError::into_inner);
        let name = table.names.get(self.0 as usize).copied().unwrap_or("");
        drop(table);
        name
    }
}

impl fmt::Debug for Sym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Display for Sym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Sym {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl PartialEq<str> for Sym {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Sym {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_text_same_sym() {
        let a = Sym::new("integral");
        let b = Sym::new("integral");
        let c = Sym::new("tau");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.as_str(), "integral");
        assert_eq!(format!("{a:?}"), "integral");
        assert!(c == "tau");
    }
}
