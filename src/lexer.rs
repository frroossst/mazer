use std::{collections::HashMap, rc::Rc};

#[derive(Debug)]
pub enum Expression {
    Expression(Rc<Expression>),
    Atom(AtomicData),
}

#[derive(Debug)]
pub enum AtomicData {
    Variable(String),
    Number(f64),
    InBuilt(InBuiltFn)
    // TODO: add more types
}

#[derive(Debug)]
pub enum InBuiltFn {
    icap,

}


#[derive(Debug)]
pub struct Lexer {
    /// symbols are like variables
    symbols: HashMap<String, Expression>,
}

impl Lexer {

    pub fn new() -> Self {
        Lexer { 
            symbols: HashMap::new(),
        }
    }

    pub fn is_lexable(&self, content: &str) -> bool {
        let inbuilt: Vec<&str> = vec!["fmt", "eval", "let"];

        for func in inbuilt {
            if content.contains(func) {
                return true;
            }
        }
        false
    }

    pub fn lex(&self, content: &str) {
        // if let case
        if content.contains("let") {
            // split on whitespace
            let tokens = content.split(" ");
        }

    }
}
