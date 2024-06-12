#[cfg(test)]
mod tests {

    use crate::{parser::Parser, lexer::Lexer};

    #[test]
    fn test_parser() {
        let src = r#"
            icap()
            "#;
        let mut p = Parser::new(src);

        let line = p.next();
        assert_eq!(line, Some("icap()".to_string()));

        let lxr = Lexer::new(line);
        lxr.process();

    }

}
