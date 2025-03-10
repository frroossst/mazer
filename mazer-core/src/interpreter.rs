use std::{collections::HashMap, fmt::Debug};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::{parser::LispExpr, pretty_err::DebugContext};

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
            }
            Evaluation::Nil => "nil".to_string(),
            Evaluation::Error(e) => e.to_string(),
        }
    }
}

type Environment = HashMap<String, LispExpr>;

#[derive(Debug)]
pub struct Interpreter {
    env: Environment,
    chunks: HashMap<String, Vec<LispExpr>>,
    ctx: DebugContext,
}

impl Interpreter {
    pub fn new(ctx: DebugContext) -> Self {
        Interpreter {
            env: Interpreter::stdenv(),
            chunks: HashMap::new(),
            ctx,
        }
    }

    fn stdenv() -> Environment {
        let mut env = HashMap::new();
        
        env.insert("pi".to_string(), LispExpr::Number(std::f64::consts::PI));
        env.insert("e".to_string(), LispExpr::Number(std::f64::consts::E));
        
        env
    }
    
    pub fn get_temporary_variable(&self) -> String {
        let length = 8;
        let suffix: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect();

        format!("__temp__{}", suffix)
    }

    pub fn set_chunk(&mut self, symbol: String, definition: Vec<LispExpr>) {
        self.chunks.insert(symbol, definition);
    }

    pub fn get_chunk(&self, symbol: String) -> Option<Vec<LispExpr>> {
        self.chunks.get(&symbol).cloned()
    }

    pub fn eval(&self, symbol: String) -> Evaluation {
        Evaluation::Error("EVAL Not implemented".to_string())
    }

    pub fn fmt(&self, symbol: String) -> String {
        "FMT Not implemented".to_string()
    }
}
