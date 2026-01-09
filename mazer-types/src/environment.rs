use std::collections::BTreeMap;
use crate::LispAST;

pub type EnvMap = BTreeMap<String, LispAST>;

#[derive(Clone)]
pub struct Environment {
    pub bindings: EnvMap,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
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
