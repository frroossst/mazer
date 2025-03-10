use std::fmt;
use regex::Regex;

use crate::pretty_err::DebugContext;



#[derive(Debug, Clone)]
pub enum LispExpr {
    Number(f64),
    String(String),
    Symbol(String),
    Boolean(bool),
    List(Vec<LispExpr>),
    Nil,
}

impl fmt::Display for LispExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LispExpr::Number(n) => write!(f, "{}", n),
            LispExpr::String(s) => write!(f, "\"{}\"", s),
            LispExpr::Symbol(s) => write!(f, "{}", s),
            LispExpr::Boolean(b) => write!(f, "{}", b),
            LispExpr::Nil => write!(f, "nil"),
            LispExpr::List(list) => {
                write!(f, "(")?;
                for (i, expr) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
        }
    }
}


pub struct Parser {
    tokens: Vec<String>,
    ast: Vec<LispExpr>,
}

impl Parser {
    pub fn new(src: String) -> Self {
        let token = Parser::tokenize(&src);
        Parser {
            tokens: token,
            ast: Vec::new(),
        }
    }

    pub fn tokenize(src: &str) -> Vec<String> {
        let regex = Regex::new(r#"[\s,]*(~@|[\[\]{}()'`~^@]|"(?:\\.|[^\\"])*"?|;.*|[^\s\[\]{}('"`,;)]*)"#).expect("regex should always compile");
        let mut results = Vec::with_capacity(1024);

        for capture in regex.captures_iter(src) {
            let token = capture.get(1).unwrap().as_str();
            if token.is_empty() || token.starts_with(';') {
                continue; // skip empty tokens and comments
            }
            results.push(token.to_string());
        }
        
        results
    }

    pub fn parse(&mut self) -> Vec<LispExpr> {
        for token in &self.tokens {
            let token = Parser::tokenize(token);
            let (ast, _) = Parser::parse_tokens(&token, 0);
            self.ast.push(ast);
        }

        self.ast.clone()
    }

    fn parse_tokens(tokens: &[String], start_index: usize) -> (LispExpr, usize) {
        if start_index >= tokens.len() {
            return (LispExpr::Nil, start_index);
        }
        
        let token = &tokens[start_index];
        
        if token == "(" {
            let mut list = Vec::new();
            let mut idx = start_index + 1;
            
            while idx < tokens.len() && tokens[idx] != ")" {
                let (expr, next_idx) = Parser::parse_tokens(tokens, idx);
                list.push(expr);
                idx = next_idx;
            }
            
            // Skip the closing parenthesis
            idx = if idx < tokens.len() { idx + 1 } else { idx };
            
            return (LispExpr::List(list), idx);
        } else {
            (Parser::parse_atom(token), start_index + 1)
        }
    }

    fn parse_atom(token: &str) -> LispExpr {
        // Handle strings
        if token.starts_with('"') {
            let content = if token.ends_with('"') && token.len() > 1 {
                &token[1..token.len()-1]
            } else {
                &token[1..]
            };
            return LispExpr::String(content.to_string());
        }
        
        // Handle numbers
        if let Ok(num) = token.parse::<f64>() {
            return LispExpr::Number(num);
        }
        
        // Handle booleans and nil
        match token {
            "true" => return LispExpr::Boolean(true),
            "false" => return LispExpr::Boolean(false),
            "nil" => return LispExpr::Nil,
            _ => {}
        }
        
        // Otherwise it's a symbol
        LispExpr::Symbol(token.to_string())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        let src = "(+ 1 2)".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse().unwrap();

        dbg!(ast);

    }
}
