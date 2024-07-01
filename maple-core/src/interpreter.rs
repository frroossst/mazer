use crate::{parser::ASTNode, stdlib::{Integral, Maple}};

pub struct Interpreter(ASTNode);

impl Interpreter {
    pub fn new(node: ASTNode) -> Interpreter {
        Interpreter(node)
    }

    pub fn fmt(&self) -> String {
        match &self.0 {
            ASTNode::FunctionCall { name, args } => {
                match name.as_str() {
                    "integral" => {
                        return Integral::new(args.clone()).fmt();
                    },
                    _ => unimplemented!(),
                }
            }
            _ => {
                unimplemented!();
            }
        }
    }
}
