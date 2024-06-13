use std::{cmp::max, collections::HashMap};

use crate::solveable::MaybeSolveable;



#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Let,
    Equal,
    Symbol(String),
    Expression(String),
    Number(f64),
}

#[derive(Debug)]
pub struct Lexer {
    /// symbols are like variable store
    symbol_def: HashMap<String, MaybeSolveable>,
}

impl Lexer {

    pub fn new() -> Self {
        Lexer { 
            symbol_def: HashMap::new(),
        }
    }

    pub fn symbols(&self) -> HashMap<String, MaybeSolveable> {
        self.symbol_def.clone()
    }

    pub fn is_lexable(&self, content: &str) -> bool {
        content.starts_with("let")
    }

    pub fn is_var(text: &str) -> Option<&str> {
        // variables can contain letters, numbers, and underscores
        // but cannot start with a number or underscore, 
        // and cannot contain spaces
        let mut chars = text.chars();

        let first_char = chars.next().unwrap();

        if text.len() != 1 && (first_char.is_numeric() || first_char == '_') {
            return None;
        } else if text == "_" {
            return Some(text)
        } else {
            for c in chars {
                if !c.is_alphanumeric() && c != '_' {
                    return None;
                }
            }
            return Some(text)
        }

    }

    fn tokenize(content: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        for l in content.split(";") {
            for (x, t) in l.split_whitespace().enumerate() {
                if t.trim().starts_with("let") {
                    tokens.push(Token::Let);
                } else if t.trim() == "=" {
                    tokens.push(Token::Equal);
                    // what follows shold be expr
                    // get everything after the equal sign
                    // i.e. from current index to end of line
                    let rest_expr = l.split_whitespace().skip(x + 1).collect::<Vec<&str>>().join(" ");

                    if let Ok(num) = rest_expr.parse::<f64>() {
                        tokens.push(Token::Number(num));
                    } else {
                        tokens.push(Token::Expression(rest_expr.to_string()));
                    }

                    break;

                } else if let Some(var) = Lexer::is_var(t) {
                    tokens.push(Token::Symbol(var.to_string()));
                } 
            }
        }
        tokens
    }

    pub fn lex(&mut self, content: &str) {
        let tokens = Lexer::tokenize(content);
        
        // sanity checks
        // tokens should have one let
        // and one equal sign
        // and one expression or number
        assert_eq!(tokens.iter().filter(|t| matches!(t, Token::Let)).count(), 1);

        // check if tokens[2] is a variable
        let variable = match &tokens[1] {
            Token::Symbol(var) => var.to_string(),
            _ => panic!("Expected a variable name"),
        };

        assert_eq!(tokens.iter().filter(|t| matches!(t, Token::Equal)).count(), 1);
        assert_eq!(max(
            tokens.iter().filter(|t| matches!(t, Token::Expression(_))).count(),
            tokens.iter().filter(|t| matches!(t, Token::Number(_))).count()), 
        1);
        assert_eq!(tokens.len(), 4);

        // store the tokens in the symbols hashmap
        let solveable_expr: MaybeSolveable = tokens[3].clone().into();
        self.symbol_def.insert(variable, solveable_expr);
    }

}

impl From<Token> for MaybeSolveable {

    fn from(tokens: Token) -> Self {
        match tokens {
            Token::Expression(expr) => MaybeSolveable::Expression(expr),
            Token::Number(num) => MaybeSolveable::Number(num),
            _ => panic!("Expected an expression or number"),
        }
    }
}
