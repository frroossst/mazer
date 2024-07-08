use maple_macros::*;

use crate::{interpreter::Interpreter, parser::{ASTNode, Parser, ParserMode}};



pub struct StdLib;

impl StdLib {
    pub fn new() -> Self {
        StdLib
    }

    pub fn get_function(&self, name: &str, args: Vec<ASTNode>) -> Option<Box<dyn Maple>> {
        match name {
            "integral" => Some(Box::new(Integral::new(args))),
            _ => None,
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<Box<dyn Maple>> {
        let stdlib_vars = std::vec![ // to avoid ambiguity with maple macros
            "realNum",
            "thereExists",
            "forAll",
            "pi",
            "theta",
            "phi",
            "lambda",
            "alpha",
        ];

        if stdlib_vars.contains(&name) {
            Some(Box::new(VarConst::new(name)))
        } else {
            None
        }
    }

}

/// stdlib traits
pub trait Maple {
    fn fmt(&self) -> String;
    fn eval(&self) -> f64;
}

/// Standard Library Variable/Constants
pub struct VarConst {
    var: String,
}

impl VarConst {
    pub fn new(var: &str) -> Self {
        VarConst {
            var: var.to_string(),
        }
    }
}

impl Maple for VarConst {
    fn fmt(&self) -> String {
        match self.var.as_str() {
            "realNum" => realNum!().to_string(),
            "thereExists" => thereExists!().to_string(),
            "forAll" => forAll!().to_string(),
            "pi" => pi!().to_string(),
            "theta" => theta!().to_string(),
            "phi" => phi!().to_string(),
            "lambda" => lambda!().to_string(),
            "alpha" => alpha!().to_string(),
            _ => self.var.clone(),
        }
    }

    fn eval(&self) -> f64 {
        unimplemented!()
    }
}

/// Standard Library Structures
#[derive(Debug)]
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
        let i = Interpreter::new();
        if self.args.len() == 2 {
            let expr: String = i.fmt(self.args.get(0).unwrap().clone());
            let wrt: String = self.args.get(1).unwrap().clone().into();

            integral!(expr, wrt)
        } else if self.args.len() == 4 {
            let lower: String = self.args.get(0).unwrap().clone().into();
            let upper: String = self.args.get(1).unwrap().clone().into();
            let expr: String = i.fmt(self.args.get(2).unwrap().clone());
            let wrt: String = self.args.get(3).unwrap().clone().into();

            dbg!(&expr);

            defintegral!(lower, upper, expr, wrt)
        } else {
            panic!("Invalid number of arguments for integral function")
        }
    }

    fn eval(&self) -> f64 {
        unimplemented!()
    }
}
