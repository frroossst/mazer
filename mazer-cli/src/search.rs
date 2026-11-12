//! The equation search box (`assets/mazer-search.js`, shared with the
//! language server's preview and the website loader), put into full pages.
//!
//! It lives here and not in `mazer-doc` so the script is never compiled into
//! the wasm module.

/// The script, as it is in the repository.
pub const SCRIPT: &str = include_str!("../../assets/mazer-search.js");

/// `page` with the search script inserted just before its last `</body>`
/// (appended when there is none).
pub fn with_search(page: &str) -> String {
    let script = format!("<script>{SCRIPT}</script>");
    page.rfind("</body>").map_or_else(
        || format!("{page}{script}"),
        |at| format!("{}{script}{}", &page[..at], &page[at..]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_can_be_inlined() {
        // Inside an inline <script>, `</script` would end the element early
        // and `<!--` changes how the parser looks for the end.
        let lower = SCRIPT.to_ascii_lowercase();
        assert!(!lower.contains("</script"));
        assert!(!SCRIPT.contains("<!--"));
        assert!(std::str::from_utf8(include_bytes!("../../assets/mazer-search.js")).is_ok());
        assert!(SCRIPT.contains("__mazerSearch"));
    }

    #[test]
    fn inserted_before_the_last_body_end() {
        let page = with_search("<html><body><p>x</p></body></html>");
        assert!(page.starts_with("<html><body><p>x</p><script>"), "{page}");
        assert!(page.ends_with("</script></body></html>"), "{page}");
        assert_eq!(page.matches("<script>").count(), 1);
        let no_body = with_search("<p>x</p>");
        assert!(no_body.starts_with("<p>x</p><script>") && no_body.ends_with("</script>"));
    }
}
