use std::collections::HashMap;

use crate::ast::LispAST;


#[derive(Default)]
pub struct Environment {
    // variable bindings that are defined using `define`
    bindings: HashMap<String, Vec<LispAST>>,

    // user defined functions
    // name, (params, body)
    functions: HashMap<String, (Vec<String>, Vec<LispAST>)>,

    // native functions provided by the host environment
    native_functions: HashMap<String, fn(Vec<LispAST>) -> LispAST>,

    // optional parent environment for nested scopes (not currenlt supported)
    parent: Option<Box<Environment>>,
}


impl Environment {

    pub fn with_stdlib() -> Self {
        let mut env = Environment::default();







    }

}

