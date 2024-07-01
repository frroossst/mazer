#[cfg(test)]
mod tests {

    use core::panic;

    use crate::{parser::{ASTNode, Parser}, tokenizer::{Emphasis, Token, Tokenizer}};

    #[test]
    fn test_compact() {
        let tokens: Vec<Token> = vec![
            Token::Text(None, "a".to_string()),
            Token::Text(None, "b".to_string()),
            Token::Text(None, "c".to_string()),
            Token::Text(None, "d".to_string()),
        ];

        let compacted = Tokenizer::compact(tokens.clone());

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

        let compacted = Tokenizer::compact(tokens.clone());

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

        let compacted = Tokenizer::compact(tokens.clone());

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

        let compacted = Tokenizer::compact(tokens.clone());

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

    #[test]
    fn test_simple_assignment() {
        let mut parser = Parser::new("let x = 123 ;".to_string());
        let ast = parser.parse();

        assert_eq!(ast, vec![
            ASTNode::Assignment {
                name: "x".to_string(),
                value: Box::new(ASTNode::Number(123.0)),
            }
        ]);
    }

    #[test]
    fn test_binary_operation() {
        let mut parser = Parser::new("let a = 1 + 2 * 3 ;".to_string());
        let ast = parser.parse();

        assert_eq!(ast, vec![
            ASTNode::Assignment {
                name: "a".to_string(),
                value: Box::new(ASTNode::BinaryOp {
                    op: "+".to_string(),
                    left: Box::new(ASTNode::Number(1.0)),
                    right: Box::new(ASTNode::BinaryOp {
                        op: "*".to_string(),
                        left: Box::new(ASTNode::Number(2.0)),
                        right: Box::new(ASTNode::Number(3.0)),
                    }),
                }),
            }
        ]);
    }

    #[test]
    fn test_nested_function_calls() {
        let mut parser = Parser::new("let nest = foo ( bar ( qux ( 0 ) , 1 ) , 2 ) ;".to_string());
        let ast = parser.parse();

        assert_eq!(ast, vec![
            ASTNode::Assignment {
                name: "nest".to_string(),
                value: Box::new(ASTNode::FunctionCall {
                    name: "foo".to_string(),
                    args: vec![
                        ASTNode::FunctionCall {
                            name: "bar".to_string(),
                            args: vec![
                                ASTNode::FunctionCall {
                                    name: "qux".to_string(),
                                    args: vec![ASTNode::Number(0.0)]
                                },
                                ASTNode::Number(1.0)
                            ]
                        },
                        ASTNode::Number(2.0)
                    ]
                }),
            }
        ]);
    }

    #[test]
    fn test_new_binary_function_syntax() {
        let mut parser = Parser::new("let dvec = vec ( 1 , 2 , 3 ) dot vec ( 4 , 5 , 6 ) ;".to_string());
        let ast = parser.parse();

        assert_eq!(ast, vec![
            ASTNode::Assignment {
                name: "dvec".to_string(),
                value: Box::new(ASTNode::FunctionCall {
                    name: "dot".to_string(),
                    args: vec![
                        ASTNode::FunctionCall {
                            name: "vec".to_string(),
                            args: vec![
                                ASTNode::Number(1.0),
                                ASTNode::Number(2.0),
                                ASTNode::Number(3.0),
                            ]
                        },
                        ASTNode::FunctionCall {
                            name: "vec".to_string(),
                            args: vec![
                                ASTNode::Number(4.0),
                                ASTNode::Number(5.0),
                                ASTNode::Number(6.0),
                            ]
                        },
                    ]
                }),
            }
        ]);
    }

    #[test]
    fn test_complex_expression() {
        let mut parser = Parser::new("let z = dot ( vec ( 1 , 2 , 3 ) , vec ( 4 , 5 , 6 ) ) * 10 ;".to_string());
        let ast = parser.parse();

        assert_eq!(ast, vec![
            ASTNode::Assignment {
                name: "z".to_string(),
                value: Box::new(ASTNode::BinaryOp {
                    op: "*".to_string(),
                    left: Box::new(ASTNode::FunctionCall {
                        name: "dot".to_string(),
                        args: vec![
                            ASTNode::FunctionCall {
                                name: "vec".to_string(),
                                args: vec![
                                    ASTNode::Number(1.0),
                                    ASTNode::Number(2.0),
                                    ASTNode::Number(3.0),
                                ]
                            },
                            ASTNode::FunctionCall {
                                name: "vec".to_string(),
                                args: vec![
                                    ASTNode::Number(4.0),
                                    ASTNode::Number(5.0),
                                    ASTNode::Number(6.0),
                                ]
                            },
                        ]
                    }),
                    right: Box::new(ASTNode::Number(10.0)),
                }),
            }
        ]);
    }

    #[test]
    fn test_ast_to_postfix() {
        let ast = ASTNode::Assignment {
            name: "z".to_string(),
            value: Box::new(ASTNode::BinaryOp {
                op: "*".to_string(),
                left: Box::new(ASTNode::FunctionCall {
                    name: "dot".to_string(),
                    args: vec![
                        ASTNode::FunctionCall {
                            name: "vec".to_string(),
                            args: vec![
                                ASTNode::Number(1.0),
                                ASTNode::Number(2.0),
                                ASTNode::Number(3.0),
                            ]
                        },
                        ASTNode::FunctionCall {
                            name: "vec".to_string(),
                            args: vec![
                                ASTNode::Number(4.0),
                                ASTNode::Number(5.0),
                                ASTNode::Number(6.0),
                            ]
                        },
                    ]
                }),
                right: Box::new(ASTNode::Number(10.0)),
            }),
        };

        let postfix = ASTNode::to_postfix(&ast);
        assert_eq!(postfix, vec![
            "1", "2", "3", "vec_3", 
            "4", "5", "6", "vec_3", 
            "dot_2", "10", "*", 
            "STORE_z"
        ]);
    }

    #[test]
    fn test_nested_arrays() {
        let mut parser = Parser::new("let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];".to_string());
        let ast = parser.parse();

        let expected = vec![
            ASTNode::Assignment {
                name: "matrix".to_string(),
                value: Box::new(ASTNode::Array(vec![
                    ASTNode::Array(vec![ASTNode::Number(1.0), ASTNode::Number(2.0), ASTNode::Number(3.0)]),
                    ASTNode::Array(vec![ASTNode::Number(4.0), ASTNode::Number(5.0), ASTNode::Number(6.0)]),
                    ASTNode::Array(vec![ASTNode::Number(7.0), ASTNode::Number(8.0), ASTNode::Number(9.0)]),
                ])),
            }
        ];

        assert_eq!(ast, expected);
    }
}
