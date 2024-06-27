use std::{collections::VecDeque, fmt::Debug, str::FromStr};

use bigdecimal::BigDecimal;

use crate::stdlib::InBuiltFunctionRegistry;


#[derive(Debug, Clone)]
pub enum Operators {
    Add,
    Subtract,
    Multiply,
    Divide,
    Exponent,
    Modulus,
    InFixFn(String),
    Eq,
}

impl PartialEq for Operators {
    fn eq(&self, other: &Self) -> bool {
        self.precedence() == other.precedence()
    }
}

impl PartialOrd for Operators {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.precedence().partial_cmp(&other.precedence())
    }

    fn ge(&self, other: &Self) -> bool {
        self.precedence() >= other.precedence()
    }

    fn gt(&self, other: &Self) -> bool {
        self.precedence() > other.precedence()
    }

    fn le(&self, other: &Self) -> bool {
        self.precedence() <= other.precedence()
    }

    fn lt(&self, other: &Self) -> bool {
        self.precedence() < other.precedence()
    }
}

impl Operators {
    pub fn precedence(&self) -> u8 {
        match self {
            Operators::Eq => 0,
            Operators::InFixFn(_) => 1,
            Operators::Add | Operators::Subtract => 2,
            Operators::Multiply | Operators::Divide | Operators::Modulus => 3,
            Operators::Exponent => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MToken {
    Operator(Operators),
    InFixFn(String),
    Identifier(String),
    Literal(String),
    Number(BigDecimal),
    Comma,
    OpenParen,
    CloseParen,
    EoF,
}

pub struct Stack(Vec<MToken>);

impl Stack {
    pub fn new() -> Self {
        Stack(Vec::new())
    }

    pub fn push(&mut self, token: MToken) {
        self.0.push(token);
    }

    pub fn pop(&mut self) -> Option<MToken> {
        self.0.pop()
    }

    pub fn peek(&self) -> Option<MToken> {
        self.0.last().cloned()
    }
}

pub struct Queue(VecDeque<MToken>);

impl Queue {
    pub fn new() -> Self {
        Queue(VecDeque::new())
    }

    pub fn push(&mut self, token: MToken) {
        self.0.push_back(token);
    }

    pub fn pop(&mut self) -> Option<MToken> {
        self.0.pop_front()
    }

    pub fn peek(&self) -> Option<MToken> {
        self.0.front().cloned()
    }
}

pub struct ShuntingYard {
    stack: Stack,
    queue: Queue,
    expr:Vec<MToken>,
}

impl ShuntingYard {
    pub fn new(expr: Vec<MToken>) -> Self {
        ShuntingYard {
            stack: Stack::new(),
            queue: Queue::new(),
            expr,
        }
    }

    pub fn shunt(&mut self) -> Stack {
        unimplemented!()
    }
}


pub struct Parser {
    src: Vec::<char>,
    pos: usize,
    max: usize,
    reg: InBuiltFunctionRegistry,
}

impl Parser {
    pub fn new(stream: String) -> Self {

        // check if the stream is empty and if the stream
        // is made up of only ascii characters
        if stream.is_empty() {
            // [ERROR]
            panic!("Empty stream");
        } 
        
        if !stream.is_ascii() {
            // [ERROR]
            panic!("Stream contains non-ascii characters");
        }

        let src = stream.chars().collect::<Vec<char>>();

        let max = src.len();
        Parser {
            src,
            pos: 0,
            max,
            reg: InBuiltFunctionRegistry::new(),
        }
    }

    fn peek(&self) -> char {
        self.src.get(self.pos + 1).unwrap_or(&' ').clone()
    }

    fn consume_whitespace(&mut self) {
        while self.pos < self.max {
            let curr = self.src[self.pos];
            if curr.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn consume_number(&mut self) -> BigDecimal {
        let mut num = String::new();
        let mut dot = false;
        let mut neg = false;

        while self.pos < self.max {
            let curr = self.src[self.pos];

            if curr.is_ascii_digit() {
                num.push(curr);
            } else if curr == '.' && !dot {
                num.push(curr);
                dot = true;
            } else if curr == '-' && !neg {
                num.push('-');
                neg = true
            } else {
                break;
            }

            self.pos += 1;
        }
        BigDecimal::from_str(&num).unwrap()
    }

    fn consume_identifier(&mut self) -> String {
        let mut ident = String::new();

        while self.pos < self.max {
            let curr = self.src[self.pos];

            if ident.is_empty() && curr.is_ascii_digit() {
                panic!("Invalid identifier");
            }

            if curr.is_ascii_alphabetic() {
                ident.push(curr);
            } else {
                break;
            }

            self.pos += 1;
        }
        ident
    }

    fn is_operator(&mut self, c: char) -> bool {
        match c {
            '+' | '-' | '*' | '/' | '^' | '%' => true,
            _ => false,
        }
    }

    fn consume_operator(&mut self) -> Operators {
        let curr = self.src[self.pos];
        let op = match curr {
            '+' => Operators::Add,
            '-' => Operators::Subtract,
            '*' => Operators::Multiply,
            '/' => Operators::Divide,
            '^' => Operators::Exponent,
            '%' => Operators::Modulus,
            _ => { panic!("should not parse infix yet!") }
        };
        self.pos += 1;
        op
    }

    fn consume_till(&mut self, c: char) -> String {
        let mut ident = String::new();

        while self.pos < self.max {
            let curr = self.src[self.pos];

            if curr == c {
                break;
            }

            ident.push(curr);
            self.pos += 1;
        }
        ident
    }

    pub fn tokenize(&mut self) -> Vec<MToken> {
        let mut tokens: Vec<MToken> = Vec::new();

        while let Some(tok) = self.next_token() {
            tokens.push(tok);
        }

        dbg!(&tokens);
        tokens
    }

    fn next_token(&mut self) -> Option<MToken> {
        if self.pos >= self.max {
            return None;
        }

        let curr = self.src[self.pos];

        if curr.is_ascii_whitespace() {
            self.consume_whitespace();
            return self.next_token();
        } else if curr == '"' {
            self.pos += 1;
            let literal = self.consume_till('\"');
            self.pos += 1;
            return Some(MToken::Literal(literal));
        } else if (curr == '-' && self.peek().is_ascii_digit()) || curr.is_ascii_digit() {
            return Some(MToken::Number(self.consume_number()));
        } else if curr.is_ascii_alphabetic() {
            let ident = self.consume_identifier();
            if self.reg.is_function(&ident) {
                return Some(MToken::InFixFn(ident));
            }
            return Some(MToken::Identifier(ident));
        } else if self.is_operator(curr) {
            return Some(MToken::Operator(self.consume_operator()));
        } else {
            match curr {
                '(' => {
                    self.pos += 1;
                    return Some(MToken::OpenParen);
                }
                ')' => {
                    self.pos += 1;
                    return Some(MToken::CloseParen);
                }
                ',' => {
                    self.pos += 1;
                    return Some(MToken::Comma);
                }
                _ => {
                    self.pos += 1;
                    return None;
                }
            }
        }
    }

}

