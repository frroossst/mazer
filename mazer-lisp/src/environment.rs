use std::collections::BTreeMap;
use mazer_stdlib::{Native, Prelude};
use mazer_types::LispAST;

use crate::{interpreter::Interpreter, parser::Parser};

type EnvMap = BTreeMap<String, LispAST>;

#[derive(Clone)]
pub struct Environment {
    bindings: EnvMap,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
    }

    pub fn with_prelude(&mut self) -> Self {
        let prelude = Prelude::new();

        for (_k, v) in prelude {
            let mut parser = Parser::new(&v);
            let ast = parser.parse().expect("Failed to parse prelude function");
            let mut interp = Interpreter::new(BTreeMap::new(), Self { bindings: self.bindings.clone() });
            interp.eval(ast).expect("Failed to evaluate prelude function");
            self.bindings = interp.env().bindings.clone();
        }

        Self { bindings: self.bindings.clone() }
    }
    
    pub fn with_native(&mut self) -> Self {
        let mut env = EnvMap::new();
        
        // TODO: add more stdlib functions here
        env.insert("+".into(), mazer_types::LispAST::NativeFunc(Native::add));
        env.insert("-".into(), mazer_types::LispAST::NativeFunc(Native::sub));

        env.insert("reflect".into(), mazer_types::LispAST::NativeFunc(Native::reflect));
        env.insert("print".into(), mazer_types::LispAST::NativeFunc(Native::print));
        env.insert("debug".into(), mazer_types::LispAST::NativeFunc(Native::debug));

        self.extend(&env);

        Self { bindings: env }
        
    }

    pub fn extend(&mut self, other: &EnvMap) {
        for (k, v) in other {
            self.bindings.insert(k.clone(), v.clone());
        }
    }
    
    pub fn insert(&mut self, name: &str, value: LispAST) {
        self.bindings.insert(name.to_string(), value);
    }
    
    pub fn get(&self, name: &str) -> Option<&LispAST> {
        self.bindings.get(name)
    }
    
    pub fn set(&mut self, name: String, value: LispAST) {
        self.bindings.insert(name, value);
    }
}
