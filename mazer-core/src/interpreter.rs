use std::{collections::HashMap, fmt::Debug};

use crate::{parser::{LispErr, LispExpr}, pretty_err::DebugContext};

pub type Environment = HashMap<String, LispExpr>;

#[derive(Debug)]
pub struct Interpreter {
    env: Environment,
    #[allow(dead_code)]
    ctx: DebugContext,
}

impl Interpreter {
    pub fn new(ctx: DebugContext) -> Self {
        Interpreter {
            env: Interpreter::stdenv(),
            ctx,
        }
    }

    fn stdenv() -> Environment {
        let mut env = HashMap::new();

        env.insert("pi".to_string(), LispExpr::Number(std::f64::consts::PI));
        env.insert("e".to_string(), LispExpr::Number(std::f64::consts::E));

        env
    }

    pub fn get_symbol(&self, symbol: String) -> Option<LispExpr> {
        self.env.get(&symbol).cloned()
    }

    pub fn set_symbol(&mut self, symbol: String, definition: LispExpr) {
        self.env.insert(symbol, definition);
    }

    pub fn eval(&self, symbol: String) -> Result<LispExpr, LispErr> {
        let expr = self.get_symbol(symbol.clone()).ok_or(LispErr::new(&format!("Symbol {} not found", symbol)))?;
        unimplemented!("interpreter::eval")
    }

}
