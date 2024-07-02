use core::panic;
use std::collections::HashMap;

use crate::{parser::ASTNode, stdlib::StdLib};

pub struct Interpreter {
    stmts: HashMap<String, ASTNode>,
    stdlib: StdLib,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            stmts: HashMap::new(),
            stdlib: StdLib::new(),
        }
    }

    pub fn is_variable(&self, name: &str) -> bool {
        self.stmts.contains_key(name)
    }

    pub fn add_stmt(&mut self, name: String, stmt: ASTNode) {
        self.stmts.insert(name, stmt);
    }

    pub fn get_stmt(&self, name: &str) -> Option<ASTNode> {
        self.stmts.get(name).cloned()
    }

    pub fn fmt(&self, expr: ASTNode) -> String {
        match expr {
            ASTNode::FunctionCall { name, args } => {
                self.stdlib.get_function(&name, args).fmt()
            },
            ASTNode::Variable(name) => {
                println!("Variable: {}", name);
                if self.is_variable(&name) {
                    self.fmt(self.get_stmt(&name).unwrap())
                } else {
                    name
                }
            },
            ASTNode::Assignment { name: _, value } => {
                let val = *value;
                self.fmt(val)
            },
            _ => {
                dbg!(expr.clone());
                panic!("Unknown expression type")
            },
        }
    }

}
