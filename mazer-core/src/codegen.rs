use crate::parser::{LispExpr, MathML};

impl MathML {
    pub fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).string())
            .collect();
        
        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
    }

    pub fn subtraction(args: &[LispExpr]) -> Self {
        if args.len() == 1 {
            format!("<mrow><mo>-</mo>{}</mrow>", MathML::from(&args[0]).string()).into()
        } else {
            let args_mathml = args.iter().map(|arg| MathML::from(arg).string()).collect::<Vec<_>>().join("<mo>-</mo>");
            format!("<mrow>{}</mrow>", args_mathml).into()
        }
    }

    pub fn multiplication(args: &[LispExpr]) -> Self {
        let args_mathml = args.iter().map(|arg| MathML::from(arg).string()).collect::<Vec<_>>().join("<mo>×</mo>");
        format!("<mrow>{}</mrow>", args_mathml).into()
    }

    pub fn division(args: &[LispExpr]) -> Self {
        format!("<mfrac>{}<mrow>{}</mrow></mfrac>", MathML::from(&args[0]).string(), MathML::from(&args[1]).string()).into()
    }

    pub fn square_root(args: &[LispExpr]) -> Self {
        format!("<msqrt>{}</msqrt>", MathML::from(&args[0]).string()).into()
    }

    pub fn power(args: &[LispExpr]) -> Self {
        format!("<msup><mrow>{}</mrow><mrow>{}</mrow></msup>", MathML::from(&args[0]).string(), MathML::from(&args[1]).string()).into()
    }

    pub fn fraction(args: &[LispExpr]) -> Self {
        format!("<mfrac><mrow>{}</mrow><mrow>{}</mrow></mfrac>", MathML::from(&args[0]).string(), MathML::from(&args[1]).string()).into()
    }

    pub fn matrix(args: &[LispExpr]) -> Self {
        let rows_mathml = args.iter().map(|row| {
            if let LispExpr::List(cells) = row {
                let cells_mathml = cells.iter()
                    .map(|cell| format!("<mtd>{}</mtd>", MathML::from(cell).string()))
                    .collect::<Vec<String>>()
                    .join("");
                format!("<mtr>{}</mtr>", cells_mathml)
            } else {
                "<mtr><mtd>Error: matrix row must be a list</mtd></mtr>".to_string()
            }
        }).collect::<Vec<String>>().join("");
        
        format!("<mrow>
            <mo>[</mo>
            <mtable>
                {}
            </mtable>
            <mo>]</mo>
        </mrow>", rows_mathml).into()
    }

    pub fn integral(args: &[LispExpr]) -> Self {
        let integrand = MathML::from(&args[0]).string();
        let limits = MathML::from(&args[1]).string();
        format!("<mrow><mo>∫</mo><mrow>{}</mrow><mrow>{}</mrow></mrow>", integrand, limits).into()
    }

}
