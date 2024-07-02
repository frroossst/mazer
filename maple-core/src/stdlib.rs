use maple_macros::{defintegral, integral};

use crate::parser::ASTNode;



pub struct StdLib;

impl StdLib {
    pub fn new() -> Self {
        StdLib
    }

    pub fn get_function(&self, name: &str, args: Vec<ASTNode>) -> Box<dyn Maple> {
        match name {
            "integral" => Box::new(Integral::new(args)),
            _ => panic!("Function not found in standard library")
        }
    }
}

/// stdlib traits
pub trait Maple {
    fn fmt(&self) -> String;
    fn eval(&self) -> f64;
}

/// Standard Library Structures

pub struct Integral {
    args: Vec<ASTNode>,
}

impl Integral {
    pub fn new(args: Vec<ASTNode>) -> Self {
        Integral { args }
    }
}

impl Maple for Integral {

    fn fmt(&self) -> String {
        if self.args.len() == 2 {
            let expr: String = self.args.get(0).unwrap().clone().into();
            let wrt: String = self.args.get(1).unwrap().clone().into();

            integral!(expr, wrt)
        } else if self.args.len() == 4 {
            let lower: String = self.args.get(0).unwrap().clone().into();
            let upper: String = self.args.get(1).unwrap().clone().into();
            let expr: String = self.args.get(2).unwrap().clone().into();
            let wrt: String = self.args.get(3).unwrap().clone().into();

            defintegral!(lower, upper, expr, wrt)
        } else {
            panic!("Invalid number of arguments for integral function")
        }
    }

    fn eval(&self) -> f64 {
        unimplemented!()
    }
}

pub struct Vector {
    args: Vec<ASTNode>,
}

impl Vector {
    pub fn new(args: Vec<ASTNode>) -> Self {
        Vector { args }
    }
}

impl Maple for Vector {
    fn fmt(&self) -> String {
        unimplemented!()
    }

    fn eval(&self) -> f64 {
        unimplemented!()
    }
}
