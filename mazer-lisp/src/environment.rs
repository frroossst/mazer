use std::collections::HashMap;

use crate::ast::LispAST;


pub struct Environment {
    bindings: HashMap<String, Vec<LispAST>>,
    parent: Option<Box<Environment>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }
}

impl Environment {

    pub fn with_stdlib() -> Self {
        let i = vec![


        ].into_iter();

        Self {
            bindings: i.collect(),
            parent: None,
        }

    }
}

