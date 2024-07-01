use crate::parser::ASTNode;

pub struct Interpreter(ASTNode);

impl Interpreter {
    pub fn new(node: ASTNode) -> Interpreter {
        Interpreter(node)
    }

    pub fn fmt(&self) -> String {
        dbg!(&self.0);

        String::new()
    }
}

