use std::rc::Rc;

#[derive(Debug)]
pub enum Expression {
    Atom,
    Expression(Rc<Expression>),
}

#[derive(Debug, Clone)]
pub enum MaybeSolveable {
    Expression(String),
    Number(f64),
    // to be shown as is
    Literal(String),
}

pub trait Tapable: SolveableT + DebugT {}

// each data structure should be solveable
pub trait SolveableT {
    fn solve(&self) -> Result<f64, anyhow::Error>;
}

// each 
pub trait DebugT: std::fmt::Debug {}
