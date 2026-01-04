use std::collections::HashMap;
use mazer_types::LispAST;

type EnvMap = HashMap<String, LispAST>;

pub struct Environment {
    bindings: EnvMap,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
    
    pub fn with_stdlib() -> Self {
        let mut env = Self::new();
        
        env.insert("+", LispAST::NativeFunc(mazer_stdlib::Native::add));
        env.insert("-", LispAST::NativeFunc(mazer_stdlib::Native::sub));
        // TODO: add more stdlib functions here
        
        env
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
