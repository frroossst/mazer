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
    Cap(Rc<Expression>),
    Vec(Rc<Expression>),
}

#[derive(Debug)]
pub enum LexableTag {
    Let,
    Fmt,
    Eval,
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

    pub fn is_lexable(&self, content: &str) -> (bool, Option<LexableTag>) {
        let inbuilt: Vec<&str> = vec!["fmt", "eval", "let"];

        if content.contains(inbuilt[0]) {
            return (true, Some(LexableTag::Fmt));
        } else if content.contains(inbuilt[1]) {
            return (true, Some(LexableTag::Eval));
        } else if content.contains(inbuilt[2]) {
            return (true, Some(LexableTag::Let));
        } else {
            return (false, None)
        }

    }

    pub fn lex(&self, content: &str, lexable_tag: LexableTag) {
        match lexable_tag {
            LexableTag::Let => {
            },
            LexableTag::Fmt => {
            },
            LexableTag::Eval => {
            },
        }
    }
}
