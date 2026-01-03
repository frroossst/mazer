use std::{collections::HashMap, hash::Hash};

use crate::{ast::LispAST, environment::Environment};


// converts Eval blocks into Show(s)
pub struct Interpreter {
    fragments: HashMap<String, LispAST>,
    env: Environment,
}

impl Interpreter {
    pub fn new(fragments: HashMap<String, LispAST>, env: Environment) -> Self {
        Self { fragments, env }
    }

    pub fn results(&self) -> &HashMap<String, LispAST> {
        &self.fragments
    }

    pub fn run(&mut self) {
        for (k, v) in self.fragments.clone().iter() {
            // evaluate f in the environment
            let evaluated = self.eval(v);
            self.fragments.insert(k.clone(), evaluated);
        }
    }

    pub fn eval(&self, ast: &LispAST) -> LispAST {
        dbg!(ast);
        todo!()
    }
}
