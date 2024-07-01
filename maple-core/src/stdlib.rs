use maple_macros::{defintegral, integral};

use crate::parser::ASTNode;

pub struct InBuiltFunctionRegistry {
    registry: Vec<String>,
    infix: Vec<String>,
}

impl InBuiltFunctionRegistry {

    pub fn new() -> Self {
        InBuiltFunctionRegistry {
            registry: vec![
                String::from("integral"),
                String::from("dot"),
                String::from("vec"),
                String::from("matrix"),
                "foo".to_string(),
                "bar".to_string(),
                "qux".to_string(),
            ],
            infix: vec![
                String::from("dot"),
            ]
        }
    }

    pub fn is_infix_fn(&self, func: &str) -> bool {
        self.infix.contains(&func.to_string())
    }

    pub fn is_function(&self, func: &str) -> bool {
        self.registry.contains(&func.to_string())
    }

}

/// stdlib trai
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
