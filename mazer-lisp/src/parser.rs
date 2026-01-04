use fastnum::{D512, decimal::Context};
use mazer_types::LispAST;


pub enum LispToken {
    Symbol(String),
    Number(D512),
    OpenParen,
    CloseParen,
}

pub struct Tokenizer {
    src: String,
}

impl Tokenizer {

    pub fn new(src: &str) -> Self {
        Tokenizer { src: src.to_string() }
    }

    pub fn tokenize(&self) -> Vec<LispToken> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = self.src.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '(' => {
                    tokens.push(LispToken::OpenParen);
                    i += 1;
                }
                ')' => {
                    tokens.push(LispToken::CloseParen);
                    i += 1;
                }
                c if c.is_whitespace() => {
                    i += 1;
                }
                c if c.is_numeric() || c == '-' => {
                    let start = i;
                    while i < chars.len() && (chars[i].is_numeric() || chars[i] == '.' || chars[i] == '-' || chars[i] == 'e' || chars[i] == 'E') {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    let number = D512::from_str(&num_str, Context::default()).map_err(|e| e.to_string()).expect("Failed to parse number");
                    tokens.push(LispToken::Number(number));
                }
                _ => {
                    let start = i;
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ')' {
                        i += 1;
                    }
                    let sym_str: String = chars[start..i].iter().collect();
                    tokens.push(LispToken::Symbol(sym_str));
                }
            }
        }

        tokens
    }

}

pub struct Parser {
    tokens: Vec<LispToken>,
    pos: usize,
}

impl Parser {
    pub fn new(src: &str) -> Self {

        Parser { 
            tokens: Tokenizer::new(src).tokenize(), 
            pos: 0 
        }
    }
    
    fn peek(&self) -> Option<&LispToken> {
        self.tokens.get(self.pos)
    }
    
    fn advance(&mut self) -> Option<&LispToken> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }
    
    pub fn parse(&mut self) -> Result<LispAST, String> {
        match self.advance() {
            Some(LispToken::Number(n)) => Ok(LispAST::Number(*n)),
            Some(LispToken::Symbol(s)) => {
                match s.as_str() {
                    "true" => Ok(LispAST::Bool(true)),
                    "false" => Ok(LispAST::Bool(false)),
                    _ => Ok(LispAST::Symbol(s.clone())),
                }
            }
            Some(LispToken::OpenParen) => {
                let mut list = Vec::new();
                while !matches!(self.peek(), Some(LispToken::CloseParen) | None) {
                    list.push(self.parse()?);
                }
                self.advance(); // consume CloseParen
                Ok(LispAST::List(list))
            }
            Some(LispToken::CloseParen) => Err("Unexpected ')'".to_string()),
            None => Err("Unexpected EOF".to_string()),
        }
    }
}
