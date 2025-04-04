#[cfg(test)]
mod markdown_tests {

    use core::panic;

    use crate::tokenizer::{Emphasis, Lexer, Token};

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
            }
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
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let tokens: Vec<Token> = vec![
            Token::Text(Some(Emphasis::Bold), "a".to_string()),
            Token::Text(Some(Emphasis::Bold), "b".to_string()),
            Token::Text(Some(Emphasis::Italic), "c".to_string()),
            Token::Text(Some(Emphasis::Italic), "d".to_string()),
            Token::Text(None, "e".to_string()),
            Token::Text(None, "f".to_string()),
            Token::Text(Some(Emphasis::Strikethrough), "g".to_string()),
            Token::Text(Some(Emphasis::Strikethrough), "h".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_ne!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 4);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "ab".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(1).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "cd".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(2).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "ef".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(3).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "gh".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let tokens: Vec<Token> = vec![
            Token::Text(Some(Emphasis::Bold), "a".to_string()),
            Token::Text(Some(Emphasis::Italic), "b".to_string()),
            Token::Text(Some(Emphasis::Strikethrough), "c".to_string()),
            Token::Text(None, "d".to_string()),
            Token::Text(Some(Emphasis::Bold), "e".to_string()),
            Token::Text(Some(Emphasis::Italic), "f".to_string()),
            Token::Text(None, "g".to_string()),
            Token::Text(Some(Emphasis::Strikethrough), "h".to_string()),
        ];

        let compacted = Lexer::compact(tokens.clone());

        assert_eq!(tokens.len(), compacted.len());
        assert_eq!(compacted.len(), 8);

        let c_tok = compacted.get(0).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "a".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(1).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "b".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(2).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "c".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(3).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "d".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(4).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Bold));
                assert_eq!(t, "e".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(5).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Italic));
                assert_eq!(t, "f".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(6).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, None);
                assert_eq!(t, "g".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };

        let c_tok = compacted.get(7).unwrap().clone();
        let _text = match c_tok {
            Token::Text(e, t) => {
                assert_eq!(e, Some(Emphasis::Strikethrough));
                assert_eq!(t, "h".to_string());
            }
            _ => panic!("Token is not Text, expected Text"),
        };
    }
}

#[cfg(test)]
mod parser_tests {
    use crate::{interpreter::{Environment, Interpreter}, parser::{LispExpr, MathML, Parser}, wrap_mathml};

    #[test]
    fn test_simple() {
        let src = "(+ 1 2)".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(list) = ast {
            list.len()
        } else {
            0
        };

        assert_eq!(list_len, 3);
    }

    #[test]
    fn test_nary() {
        let src = "(* 1 2 3 4 5)".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(list) = ast {
            list.len()
        } else {
            0
        };

        assert_eq!(list_len, 6);
    }

    #[test]
    fn test_nested() {
        let src = "(+ 1 (* 2 3))".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(ref list) = ast {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 3);

        // get the first memeber from within list
        let first = if let LispExpr::List(ref list) = ast {
            list[0].clone()
        } else {
            LispExpr::Nil
        };
        assert_eq!(first, LispExpr::Symbol("+".to_string()));

    }

    #[test]
    fn test_wrap_mathml() {
        let wrapped = wrap_mathml!("hello");
        assert_eq!(
            wrapped,
            "<math xmlns=\"http://www.w3.org/1998/Math/MathML\">hello</math>"
        );
    }

    #[test]
    fn test_mathml_repr() {
        let mut p = Parser::new("5".to_string());
        let expr = p.parse();
        let mathml = MathML::from(&expr);
        let repr = MathML::from(mathml.string());

        assert_eq!(wrap_mathml!(repr.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mn>5</mn></math>");

        let mut p = Parser::new("(+ 1 2)".to_string());
        let expr = p.parse();
        let mathml = MathML::from(&expr);
        let repr = MathML::from(mathml.string());

        assert_eq!(wrap_mathml!(repr.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><mn>1</mn><mo>+</mo><mn>2</mn></mrow></math>");
    }

    #[test]
    fn test_simple_tokenize() {

        let p = Parser::tokenize("(+ 1 2)");
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], "(");
        assert_eq!(p[1], "+");
        assert_eq!(p[2], "1");
        assert_eq!(p[3], "2");
        assert_eq!(p[4], ")");
    }

    #[test]
    fn test_nested_tokenize() {
        let p = Parser::tokenize("(+ 1 (sin (pow 2 3)))");
        assert_eq!(p.len(), 12);
        assert_eq!(p[0], "(");
        assert_eq!(p[1], "+");
        assert_eq!(p[2], "1");
        assert_eq!(p[3], "(");
        assert_eq!(p[4], "sin");
        assert_eq!(p[5], "(");
        assert_eq!(p[6], "pow");
        assert_eq!(p[7], "2");
        assert_eq!(p[8], "3");
        assert_eq!(p[9], ")");
        assert_eq!(p[10], ")");
        assert_eq!(p[11], ")");
    }

    #[test]
    fn test_addition_codegen() {
        let src = "(+ 1 2 3 4 5)";
        let mut p = Parser::new(src.into());
        let ast = p.parse();

        let list_len = if let LispExpr::List(list) = ast.clone() {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 6);

        let mathml: MathML = (&ast).into();

        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><mn>1</mn><mo>+</mo><mn>2</mn><mo>+</mo><mn>3</mn><mo>+</mo><mn>4</mn><mo>+</mo><mn>5</mn></mrow></math>");
    }

    #[test]
    fn test_matrix_codegen() {
        let mut p = Parser::new("(matrix (1 2 3) (4 5 6) (7 8 9))".into());
        let ast = p.parse();

        let list_len = if let LispExpr::List(list) = ast.clone() {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 4);

        let mathml: MathML = (&ast).into();

        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><mo>[</mo><mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd><mtd><mn>3</mn></mtd></mtr><mtr><mtd><mn>4</mn></mtd><mtd><mn>5</mn></mtd><mtd><mn>6</mn></mtd></mtr><mtr><mtd><mn>7</mn></mtd><mtd><mn>8</mn></mtd><mtd><mn>9</mn></mtd></mtr></mtable><mo>]</mo></mrow></math>");
    }

    #[test]
    fn test_nested_matrix_codegen() {
        let mut p = Parser::new(" (matrix ((matrix (0) (1))) (2) ((matrix (3 4))) (5) (6)) ".into());
        let ast = p.parse();

        let list_len = if let LispExpr::List(list) = ast.clone() {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 6);

        let mathml = MathML::from(&ast);

        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><mo>[</mo><mtable><mtr><mtd><mrow><mo>[</mo><mtable><mtr><mtd><mn>0</mn></mtd></mtr><mtr><mtd><mn>1</mn></mtd></mtr></mtable><mo>]</mo></mrow></mtd></mtr><mtr><mtd><mn>2</mn></mtd></mtr><mtr><mtd><mrow><mo>[</mo><mtable><mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable><mo>]</mo></mrow></mtd></mtr><mtr><mtd><mn>5</mn></mtd></mtr><mtr><mtd><mn>6</mn></mtd></mtr></mtable><mo>]</mo></mrow></math>");
    }


    #[test]
    fn test_integration() {
        let mut p = Parser::new("(integral x dx)".into());
        let ast = p.parse();

        let mathml = MathML::from(&ast);

        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow>
                <mo>∫</mo>
                <mrow><mi>x</mi></mrow>
                <mi>dx</mi>
            </mrow></math>");
    }

    #[test]
    fn test_chained_integration() {
        let src = "(integral (pow x 2) 0 1 dx)";
        let mut p = Parser::new(src.into());
        let ast = p.parse();

        let mathml = MathML::from(&ast);
        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow>
                    <msubsup>
                        <mo>∫</mo>
                        <mrow><mn>0</mn></mrow>
                        <mrow><mn>1</mn></mrow>
                    </msubsup>
                    <mrow><msup><mrow><mi>x</mi></mrow><mrow><mn>2</mn></mrow></msup></mrow>
                    <mi>dx</mi>
                </mrow></math>");

        let src = "(integral (integral (integral (pow x 2) 0 1 dx) 0 1 dy) 0 1 dz)";
        let mut p = Parser::new(src.into());
        let ast = p.parse();

        let mathml = MathML::from(&ast);
        assert_eq!(wrap_mathml!(mathml.string()), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow>
                    <msubsup>
                        <mo>∫</mo>
                        <mrow><mn>0</mn></mrow>
                        <mrow><mn>1</mn></mrow>
                    </msubsup>
                    <mrow><mrow>
                    <msubsup>
                        <mo>∫</mo>
                        <mrow><mn>0</mn></mrow>
                        <mrow><mn>1</mn></mrow>
                    </msubsup>
                    <mrow><mrow>
                    <msubsup>
                        <mo>∫</mo>
                        <mrow><mn>0</mn></mrow>
                        <mrow><mn>1</mn></mrow>
                    </msubsup>
                    <mrow><msup><mrow><mi>x</mi></mrow><mrow><mn>2</mn></mrow></msup></mrow>
                    <mi>dx</mi>
                </mrow></mrow>
                    <mi>dy</mi>
                </mrow></mrow>
                    <mi>dz</mi>
                </mrow></math>");
    }

    #[test]
    fn test_simple_env_sub() {
        let i = Interpreter::new();

        let mut env: Environment = i.environment();

        env.insert("x".to_string(), LispExpr::Number(1.0));

        let expression = Parser::new("x".to_string()).parse();
        let ans = Interpreter::eval_expr(&expression, &mut env);

        assert_eq!(ans.unwrap(), LispExpr::Number(1.0));
    }

    #[test]
    fn test_add_env_sub() {
        let i = Interpreter::new();

        let mut env: Environment = i.environment();

        env.insert("x".to_string(), LispExpr::Number(1.0));
        
        let expression = Parser::new("(+ x 2)".to_string()).parse();
        let ans = Interpreter::eval_expr(&expression, &mut env);

        assert_eq!(ans.unwrap(), LispExpr::Number(3.0));
    }

    #[test]
    fn test_env_substitution() {
        let i = Interpreter::new();

        let mut env: Environment = i.environment();

        let alpha = Parser::new("5".to_string()).parse();
        env.insert("alpha".to_string(), alpha.clone());

        let alpha_expr = Interpreter::eval_expr(&alpha, &mut env);
        assert_eq!(alpha_expr.is_ok(), true);
        assert_eq!(alpha_expr.unwrap(), LispExpr::Number(5.0));

        let beta = Parser::new("(* alpha 2)".to_string()).parse();
        env.insert("beta".to_string(), beta.clone());

        let beta_expr = Interpreter::eval_expr(&beta, &mut env);
        assert_eq!(beta_expr.is_ok(), true);
        assert_eq!(beta_expr.unwrap(), LispExpr::Number(10.0));

        // let gamma = Parser::new("(* beta 3)".to_string()).parse();
        // env.insert("gamma".to_string(), gamma.clone());

        // let gamma_expr = Interpreter::eval_expr(&gamma, &mut env);
        // assert_eq!(gamma_expr.is_ok(), true);
        // assert_eq!(gamma_expr.unwrap(), LispExpr::Number(30.0));

        env.insert("gamma".to_string(), LispExpr::Number(20.0));

        let expression = Parser::new("(+ alpha beta gamma)".to_string()).parse();

        let ans = Interpreter::eval_expr(&expression, &mut env);
        assert_eq!(ans.unwrap(), LispExpr::Number(35.0));

    }
}
