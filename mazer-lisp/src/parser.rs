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
        Tokenizer {
            src: src.to_string(),
        }
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
                // Handle numbers: must start with digit, or minus followed by digit
                c if c.is_numeric()
                    || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_numeric()) =>
                {
                    let start = i;
                    // Handle optional leading minus
                    if chars[i] == '-' {
                        i += 1;
                    }
                    // Parse digits before decimal point
                    while i < chars.len() && chars[i].is_numeric() {
                        i += 1;
                    }
                    // Handle decimal point and digits after
                    if i < chars.len()
                        && chars[i] == '.'
                        && i + 1 < chars.len()
                        && chars[i + 1].is_numeric()
                    {
                        i += 1; // consume '.'
                        while i < chars.len() && chars[i].is_numeric() {
                            i += 1;
                        }
                    }
                    // Handle scientific notation
                    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                        i += 1;
                        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                            i += 1;
                        }
                        while i < chars.len() && chars[i].is_numeric() {
                            i += 1;
                        }
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    let number = D512::from_str(&num_str, Context::default())
                        .map_err(|e| e.to_string())
                        .expect(&format!("Failed to parse number: '{}'", num_str));
                    tokens.push(LispToken::Number(number));
                }
                _ => {
                    let start = i;
                    while i < chars.len()
                        && !chars[i].is_whitespace()
                        && chars[i] != '('
                        && chars[i] != ')'
                    {
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
            pos: 0,
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
        let mut exprs = Vec::new();

        while self.pos < self.tokens.len() {
            exprs.push(self.parse_one()?);
        }

        if exprs.is_empty() {
            return Err("No expressions to parse".to_string());
        }

        if exprs.len() == 1 {
            Ok(exprs.into_iter().next().unwrap())
        } else {
            let mut begin_list = vec![LispAST::Symbol("begin".to_string())];
            begin_list.extend(exprs);
            Ok(LispAST::List(begin_list))
        }
    }

    fn parse_one(&mut self) -> Result<LispAST, String> {
        match self.advance() {
            Some(LispToken::Number(n)) => Ok(LispAST::Number(*n)),
            Some(LispToken::Symbol(s)) => match s.as_str() {
                "true" => Ok(LispAST::Bool(true)),
                "false" => Ok(LispAST::Bool(false)),
                _ => Ok(LispAST::Symbol(s.clone())),
            },
            Some(LispToken::OpenParen) => {
                let mut list = Vec::new();
                while !matches!(self.peek(), Some(LispToken::CloseParen) | None) {
                    list.push(self.parse_one()?);
                }
                self.advance(); // consume CloseParen
                Ok(LispAST::List(list))
            }
            Some(LispToken::CloseParen) => Err("Unexpected ')'".to_string()),
            None => Err("Unexpected EOF".to_string()),
        }
    }
}
