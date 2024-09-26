#[cfg(test)]
mod tests {

    use core::panic;

    use crate::tokenizer::{Emphasis, Token, Lexer};

    #[test]
    fn test_compact() {
        let tokens: Vec<Token> = vec![
            Token::Text(None, "a".to_string()),
            Token::Text(None, "b".to_string()),
            Token::Text(None, "c".to_string()),
            Token::Text(None, "d".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_ne!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 1);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "abcd".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };


        let tokens: Vec<Token> = vec![
            Token::Text(Some(Emphasis::Bold), "a".to_string()),
            Token::Text(Some(Emphasis::Bold), "b".to_string()),
            Token::Text(Some(Emphasis::Bold), "c".to_string()),
            Token::Text(Some(Emphasis::Bold), "d".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_ne!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 1);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "abcd".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };


        let tokens: Vec<Token> = vec![
            Token::Text(Some(Emphasis::Bold), "a".to_string()),
            Token::Text(Some(Emphasis::Bold), "b".to_string()),
            Token::Text(Some(Emphasis::Italic), "c".to_string()),
            Token::Text(Some(Emphasis::Italic), "d".to_string()),
            Token::Text(None, "e".to_string()),
            Token::Text(None, "f".to_string()),
            Token::Text(Some(Emphasis::Strikethrough),"g".to_string()),
            Token::Text(Some(Emphasis::Strikethrough),"h".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_ne!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 4);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "ab".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(1).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "cd".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(2).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "ef".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(3).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "gh".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };


        let tokens: Vec<Token> = vec![
            Token::Text(Some(Emphasis::Bold), "a".to_string()),
            Token::Text(Some(Emphasis::Italic), "b".to_string()),
            Token::Text(Some(Emphasis::Strikethrough),"c".to_string()),
            Token::Text(None, "d".to_string()),
            Token::Text(Some(Emphasis::Bold), "e".to_string()),
            Token::Text(Some(Emphasis::Italic), "f".to_string()),
            Token::Text(None, "g".to_string()),
            Token::Text(Some(Emphasis::Strikethrough),"h".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_eq!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 8);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "a".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(1).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "b".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(2).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "c".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(3).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "d".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(4).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "e".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(5).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "f".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(6).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "g".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(7).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "h".to_string());
            },
            _ => panic!("Token is not Text, expected Text"),
        };
    }

}
