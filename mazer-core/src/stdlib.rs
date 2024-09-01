use mazer_macros::*;

use crate::{interpreter::Interpreter, parser::ASTNode};



pub struct StdLib<'a> {
    funcs: Vec<&'a str>,
    vars: Vec<&'a str>,
}

impl<'a> StdLib<'a> {
    pub fn new() -> Self {
        StdLib {
            funcs: std::vec![
                "integral",
            ],
            vars: std::vec![
                "realNum",
                "thereExists",
                "forAll",
                "infinity",
                "angle",
                "degrees",
                "Alpha",
                "alpha",
                "Beta",
                "beta",
                "Gamma",
                "gamma",
                "Delta",
                "delta",
                "Epsilon",
                "epsilon",
                "Zeta",
                "zeta",
                "Eta",
                "eta",
                "Theta",
                "theta",
                "Iota",
                "iota",
                "Kappa",
                "kappa",
                "Lambda",
                "lambda",
                "Mu",
                "mu",
                "Nu",
                "nu",
                "Xi",
                "xi",
                "Omicron",
                "omicron",
                "Pi",
                "pi",
                "Rho",
                "rho",
                "Sigma",
                "sigma",
                "Tau",
                "tau",
                "Upsilon",
                "upsilon",
                "Phi",
                "phi",
                "Chi",
                "chi",
                "Psi",
                "psi",
                "Omega",
                "omega",
            ],
        }
    }

    pub fn stdlib(&self) -> (Vec<&'a str>, Vec<&'a str>) {
        (self.funcs.clone(), self.vars.clone())
    }

    pub fn get_function(&self, name: &str, args: Vec<ASTNode>) -> Option<Box<dyn Maple>> {
        if !self.funcs.contains(&name) {
            return None;
        }
        match name {
            "integral" => Some(Box::new(Integral::new(args))),
            _ => None,
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<Box<dyn Maple>> {
        if self.vars.contains(&name) {
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
            "infinity" => infinity!().to_string(),
            "angle" => angle!().to_string(),
            "degrees" => degrees!().to_string(),
            "Alpha" => Alpha!().to_string(),
            "alpha" => alpha!().to_string(),
            "Beta" => Beta!().to_string(),
            "beta" => beta!().to_string(),
            "Gamma" => Gamma!().to_string(),
            "gamma" => gamma!().to_string(),
            "Delta" => Delta!().to_string(),
            "delta" => delta!().to_string(),
            "Epsilon" => Epsilon!().to_string(),
            "epsilon" => epsilon!().to_string(),
            "Zeta" => Zeta!().to_string(),
            "zeta" => zeta!().to_string(),
            "Eta" => Eta!().to_string(),
            "eta" => eta!().to_string(),
            "Theta" => Theta!().to_string(),
            "theta" => theta!().to_string(),
            "Iota" => Iota!().to_string(),
            "iota" => iota!().to_string(),
            "Kappa" => Kappa!().to_string(),
            "kappa" => kappa!().to_string(),
            "Lambda" => Lambda!().to_string(),
            "lambda" => lambda!().to_string(),
            "Mu" => Mu!().to_string(),
            "mu" => mu!().to_string(),
            "Nu" => Nu!().to_string(),
            "nu" => nu!().to_string(),
            "Xi" => Xi!().to_string(),
            "xi" => xi!().to_string(),
            "Omicron" => Omicron!().to_string(),
            "omicron" => omicron!().to_string(),
            "Pi" => Pi!().to_string(),
            "pi" => pi!().to_string(),
            "Rho" => Rho!().to_string(),
            "rho" => rho!().to_string(),
            "Sigma" => Sigma!().to_string(),
            "sigma" => sigma!().to_string(),
            "Tau" => Tau!().to_string(),
            "tau" => tau!().to_string(),
            "Upsilon" => Upsilon!().to_string(),
            "upsilon" => upsilon!().to_string(),
            "Phi" => Phi!().to_string(),
            "phi" => phi!().to_string(),
            "Chi" => Chi!().to_string(),
            "chi" => chi!().to_string(),
            "Psi" => Psi!().to_string(),
            "psi" => psi!().to_string(),
            "Omega" => Omega!().to_string(),
            "omega" => omega!().to_string(),
            _ => self.var.clone(),
        }
    }

    fn eval(&self) -> f64 {
        unimplemented!("eval is not implemented")
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

            defintegral!(lower, upper, expr, wrt)
        } else {
            panic!("Invalid number of arguments for integral function")
        }
    }

    fn eval(&self) -> f64 {
        unimplemented!("eval is not implemented")
    }
}
