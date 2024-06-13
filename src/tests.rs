#[cfg(test)]
mod tests {

    use crate::{parser::Parser, lexer::Lexer};

    #[test]
    fn test_parser() {
        let src = r#"
            icap()
            "#;

    }

    #[test]
    fn test_is_var() {
        assert_eq!(Lexer::is_var("a"), Some("a"));
        assert_eq!(Lexer::is_var("a1"), Some("a1"));
        assert_eq!(Lexer::is_var("a_"), Some("a_"));
        assert_eq!(Lexer::is_var("a_1"), Some("a_1"));
        assert_eq!(Lexer::is_var("1a"), None);
        assert_eq!(Lexer::is_var("_a"), None);
        assert_eq!(Lexer::is_var("_"), Some("_"));
        assert_eq!(Lexer::is_var("a b"), None);
        assert_eq!(Lexer::is_var("a{b"), None);
        assert_eq!(Lexer::is_var("a(b"), None);
    }

}
