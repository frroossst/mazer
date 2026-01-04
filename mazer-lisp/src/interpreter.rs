use std::collections::HashMap;

use mazer_types::LispAST;

use crate::environment::Environment;



// converts Eval blocks into Show(s)
pub struct Interpreter {
    fragments: HashMap<String, LispAST>,
    env: Environment,
}

impl Interpreter {
    pub fn new(fragments: HashMap<String, LispAST>, env: Environment) -> Self {
        Self { fragments, env }
    }

    pub fn results(&self) -> HashMap<String, LispAST> {
        self.fragments.clone()
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
        LispAST::Error("todo!".to_string())
    }
}
