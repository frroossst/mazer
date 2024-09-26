use std::{collections::HashMap, fmt::Debug};

use crate::{parser::ASTNode, pretty_err::DebugContext};



pub enum Evaluation {
    Number(f64),
    Symbol(String),
    List(Vec<Evaluation>),
    Nil,
    Error(String),
}

impl ToString for Evaluation {
    fn to_string(&self) -> String {
        match self {
            Evaluation::Number(n) => n.to_string(),
            Evaluation::Symbol(s) => s.to_string(),
            Evaluation::List(l) => {
                let mut res = String::new();
                res.push_str("(");
                for (i, e) in l.iter().enumerate() {
                    res.push_str(&e.to_string());
                    if i < l.len() - 1 {
                        res.push_str(" ");
                    }
                }
                res.push_str(")");
                res
            },
            Evaluation::Nil => "nil".to_string(),
            Evaluation::Error(e) => e.to_string(),
        }
    }
}

pub struct Interpreter {
    chunks: HashMap<String, Vec<ASTNode>>,
    ctx: DebugContext,
}

impl Interpreter {
    pub fn new(ctx: DebugContext) -> Self {
        Interpreter {
            chunks: HashMap::new(),
            ctx,
        }
    }

    pub fn add_chunk(&mut self, symbol: String, definition: Vec<ASTNode>) {

    }

    pub fn eval(&self, symbol: String) -> Evaluation {
        Evaluation::Error("Not implemented".to_string())
    }

    pub fn fmt(&self, symbol: String) -> String {
        "Not implemented".to_string()
    }

}
