use std::{collections::VecDeque, str::FromStr};

use bigdecimal::BigDecimal;

use crate::tokenizer::Token;


#[derive(Debug, Clone)]
pub enum Operators {
    Add,
    Subtract,
    Multiply,
    Divide,
    Exponent,
    Modulus,
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
            Operators::Add | Operators::Subtract => 1,
            Operators::Multiply | Operators::Divide | Operators::Modulus => 2,
            Operators::Exponent => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MToken {
    Operator(Operators),
    InFixFn(String),
    Variable(String),
    Call(String, Vec<MToken>),
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


#[derive(Debug)]
pub struct Parser {
    src: Vec<Token>,
    shunted: Vec<MToken>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        dbg!(&tokens);
        Parser {
            src: tokens,
            shunted: Vec::new(),
        }
    }
}
