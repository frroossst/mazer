use crate::parser::{LispExpr, MathML};

impl MathML {
    pub fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();
        
        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
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
}
