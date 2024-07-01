use crate::parser::ASTNode;

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
                    },
                    _ => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        }
        unimplemented!();

        String::new()
    }
}
