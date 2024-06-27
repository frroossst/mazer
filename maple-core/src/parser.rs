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
    Variable(String),
    // Call(String, Vec<MToken>),
    Call,
    Number(BigDecimal),
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

    pub fn tokenize(&mut self) -> Vec<MToken> {
        let mut tokens: Vec<MToken> = Vec::new();

        while self.pos < self.max {
            // implement shunting yard and add a CALL tag as the first argument to a function
        
        self.pos += 1;
        }
        tokens
    }


}

