use std::collections::HashMap;

use mazer_macros::{fraction, exponent};

use crate::{parser::ASTNode, stdlib::StdLib};

pub struct Interpreter {
    stmts: HashMap<String, ASTNode>,
    stdlib: StdLib<'static>,
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
            ASTNode::Literal(lit) => {
                lit.trim_end_matches("\"").to_string()
            },
            ASTNode::Number(num) => {
                num.to_string()
            },
            ASTNode::FunctionCall { name, args } => {
                match self.stdlib.get_function(&name, args.clone()) {
                    Some(val) => val.fmt(),
                    None => {
                        format!("{}({})", name, args.iter().map(|x| self.fmt(x.clone())).collect::<Vec<String>>().join(", "))
                    }
                }
            },
            ASTNode::Variable(name) => {
                if self.is_variable(&name) {
                    self.fmt(self.get_stmt(&name).unwrap())
                } else if let Some(var) =  self.stdlib.get_variable(&name) {
                    var.fmt()
                } else {
                    name
                }
            },
            ASTNode::Array(arr) => {
                let mut arr_str: String = String::from("[");
               
                for elem in arr {
                    let elem_str: String = elem.into();
                    arr_str.push_str(&elem_str);
                    arr_str.push_str(", ");
                }
                arr_str.pop();
                arr_str.pop();
                arr_str.push(']');

                arr_str
            }
            ASTNode::Assignment { name: _, value } => {
                let val = *value;
                self.fmt(val)
            },
            ASTNode::BinaryOp { op, left, right } => {
                match op.as_str() {
                    "^" => {
                        let lhs = self.fmt(*left);
                        let rhs = self.fmt(*right);

                        exponent!(lhs, rhs)
                    },
                    "+" => {
                        let lhs  = self.fmt(*left);
                        let rhs = self.fmt(*right);

                        format!("{} + {}", lhs, rhs)
                    },
                    "-" => {
                        let lhs  = self.fmt(*left);
                        let rhs = self.fmt(*right);

                        format!("{} - {}", lhs, rhs)
                    },
                    "*" => {
                        let lhs  = self.fmt(*left);
                        let rhs = self.fmt(*right);

                        format!("{} * {}", lhs, rhs)
                    },
                    "/" => {
                        let lhs  = self.fmt(*left);
                        let rhs = self.fmt(*right);

                        fraction!(lhs, rhs)
                    }
                    _ => { 
                        unimplemented!("[ERROR] BinaryOp: {:?}", op);
                    },
                }
            },
            _ => {
                return format!("{:?}", expr);
            },
        }
    }
}
