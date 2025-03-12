use crate::parser::{LispExpr, MathML};

impl MathML {
    pub fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();
        
        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
    }

    pub fn subtraction(_args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = _args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>-</mo>")).into()
    }

    pub fn multiplication(_args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = _args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>*</mo>")).into()
    }

    pub fn division(_args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = _args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>/</mo>")).into()
    }

    pub fn matrix(args: &[LispExpr]) -> Self {
        let rows_mathml = args.iter().map(|row| {
            if let LispExpr::List(cells) = row {
                let cells_mathml = cells.iter()
                    .map(|cell| format!("<mtd>{}</mtd>", MathML::from(cell).to_string()))
                    .collect::<Vec<String>>()
                    .join("");
                format!("<mtr>{}</mtr>", cells_mathml)
            } else {
                "<mtr><mtd>Error: matrix row must be a list</mtd></mtr>".to_string()
            }
        }).collect::<Vec<String>>().join("");
        
        format!("<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>", rows_mathml).into()
    }

    pub fn fraction(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn subscript(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn superscript(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn vector(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn derivative(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn integral(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn limit(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn sum(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn abs(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn sqrt(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn nth_root(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

}
