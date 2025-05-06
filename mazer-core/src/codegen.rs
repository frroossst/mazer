use crate::parser::{LispExpr, MathML};

impl MathML {
    pub fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).string())
            .collect();
        
        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
    }

    pub fn subtraction(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>-</mo>")).into()
    }

    pub fn multiplication(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>*</mo>")).into()
    }

    pub fn division(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).string())
            .collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>/</mo>")).into()
    }

    pub fn power(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let exponent = MathML::from(&args[1]).string();

        format!("<msup><mrow>{}</mrow><mrow>{}</mrow></msup>", base, exponent).into()
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
        
        format!("<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>", rows_mathml).into()
    }

    pub fn fraction(args: &[LispExpr]) -> Self {
        let numerator = MathML::from(&args[0]).string();
        let denominator = MathML::from(&args[1]).string();

        format!("<mfrac><mrow>{}</mrow><mrow>{}</mrow></mfrac>", numerator, denominator).into()
    }

    pub fn subscript(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let subscript = MathML::from(&args[1]).string();

        format!("<msub><mrow>{}</mrow><mrow>{}</mrow></msub>", base, subscript).into()
    }

    pub fn superscript(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let superscript = MathML::from(&args[1]).string();

        format!("<msup><mrow>{}</mrow><mrow>{}</mrow></msup>", base, superscript).into()
    }

    pub fn vector(args: &[LispExpr]) -> Self {
        let components = args.iter()
            .map(|component| format!("<mtd>{}</mtd>", MathML::from(component).string()))
            .collect::<Vec<String>>()
            .join("");

        format!("<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>", components).into()
    }

    pub fn derivative(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn integral(args: &[LispExpr]) -> Self {
        dbg!(&args);
        if args.len() > 3 {
            if let LispExpr::Symbol(var) = &args[3] {
                // Definite integral
                format!("<mrow>
                    <msubsup>
                        <mo>∫</mo>
                        <mrow>{}</mrow>
                        <mrow>{}</mrow>
                    </msubsup>
                    <mrow>{}</mrow>
                    <mi>{}</mi>
                </mrow>",
                MathML::from(&args[1]).string(),
                MathML::from(&args[2]).string(),
                MathML::from(&args[0]).string(),
                var).into()
            } else {
                "<mrow>Error: integration variable must be a symbol</mrow>".to_string().into()
            }
        } else if let LispExpr::Symbol(var) = &args[1] {
            // Indefinite integral
            format!("<mrow>
                <mo>∫</mo>
                <mrow>{}</mrow>
                <mi>{}</mi>
            </mrow>",
            MathML::from(&args[0]).string(),
            var).into()
        } else {
            "<mrow>Error: integration variable must be a symbol</mrow>".to_string().into()
        }
    }

    pub fn sin(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!("<mrow><mi>sin</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>", arg).into()
    }

    pub fn cos(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!("<mrow><mi>cos</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>", arg).into()
    }

    pub fn tan(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!("<mrow><mi>tan</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>", arg).into()
    }

    pub fn limit(_args: &[LispExpr]) -> Self {
        unimplemented!()
    }

    pub fn sum(args: &[LispExpr]) -> Self {
        if args.len() >= 4 {
            if let LispExpr::Symbol(index) = &args[0] {
                let lower = MathML::from(&args[1]).string();
                let upper = MathML::from(&args[2]).string();
                let expr = MathML::from(&args[3]).string();

                format!(
                    "<mrow><munderover><mo>∑</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow>{}</munderover><mrow>{}</mrow></mrow>",
                    index, lower, upper, expr
                ).into()
            } else {
                "<mrow>Error: summation index must be a symbol</mrow>".to_string().into()
            }
        } else {
            "<mrow>Error: sum requires index, lower bound, upper bound, and expression</mrow>".to_string().into()
        }
    }

    pub fn abs(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!("<mrow><mo>|</mo>{}</mrow><mo>|</mo>", arg).into()
    }

    pub fn sqrt(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!("<msqrt>{}</msqrt>", arg).into()
    }

    pub fn nth_root(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let root = MathML::from(&args[1]).string();
        format!("<mroot><mrow>{}</mrow><mrow>{}</mrow></mroot>", base, root).into()
    }

}
