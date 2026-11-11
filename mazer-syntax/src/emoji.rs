//! The emoji shortcode table. Deliberately tiny: exactly the shortcodes the
//! author uses. Add one by adding a row.

/// `(shortcode name, character)`
pub const EMOJI: &[(&str, &str)] = &[
    ("sob", "😭"),
    ("maple", "🍁"),
    ("heart", "❤️"),
    ("eyes", "👀"),
    ("thinking", "🤔"),
    ("+1", "👍"),
    ("-1", "👎"),
    ("brain", "🧠"),
    ("fire", "🔥"),
    ("sparkles", "✨"),
    ("rocket", "🚀"),
    ("coffee", "☕"),
    ("sunglasses", "😎"),
];

/// The character for a shortcode name (without the colons), if it is known.
#[must_use]
pub fn lookup(name: &str) -> Option<&'static str> {
    EMOJI.iter().find(|(n, _)| *n == name).map(|(_, ch)| *ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_unknown() {
        assert_eq!(lookup("sob"), Some("😭"));
        assert_eq!(lookup("+1"), Some("👍"));
        assert_eq!(lookup("nope"), None);
        assert_eq!(EMOJI.len(), 13);
    }
}
