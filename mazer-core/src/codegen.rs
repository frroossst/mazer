use crate::parser::{LispExpr, MathML};

impl MathML {
    pub fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter().map(|arg| MathML::from(arg).string()).collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
    }

    pub fn subtraction(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter().map(|arg| MathML::from(arg).string()).collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>-</mo>")).into()
    }

    pub fn multiplication(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter().map(|arg| MathML::from(arg).string()).collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>*</mo>")).into()
    }

    pub fn division(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter().map(|arg| MathML::from(arg).string()).collect();

        format!("<mrow>{}</mrow>", args_mathml.join("<mo>/</mo>")).into()
    }

    pub fn power(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let exponent = MathML::from(&args[1]).string();

        format!(
            "<msup><mrow>{}</mrow><mrow>{}</mrow></msup>",
            base, exponent
        )
        .into()
    }

    pub fn matrix(args: &[LispExpr]) -> Self {
        let rows_mathml = args
            .iter()
            .map(|row| {
                if let LispExpr::List(cells) = row {
                    let cells_mathml = cells
                        .iter()
                        .map(|cell| format!("<mtd>{}</mtd>", MathML::from(cell).string()))
                        .collect::<Vec<String>>()
                        .join("");
                    format!("<mtr>{}</mtr>", cells_mathml)
                } else {
                    "<mtr><mtd>Error: matrix row must be a list</mtd></mtr>".to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("");

        format!(
            "<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>",
            rows_mathml
        )
        .into()
    }

    pub fn fraction(args: &[LispExpr]) -> Self {
        let numerator = MathML::from(&args[0]).string();
        let denominator = MathML::from(&args[1]).string();

        format!(
            "<mfrac><mrow>{}</mrow><mrow>{}</mrow></mfrac>",
            numerator, denominator
        )
        .into()
    }

    pub fn subscript(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let subscript = MathML::from(&args[1]).string();

        format!(
            "<msub><mrow>{}</mrow><mrow>{}</mrow></msub>",
            base, subscript
        )
        .into()
    }

    pub fn superscript(args: &[LispExpr]) -> Self {
        let base = MathML::from(&args[0]).string();
        let superscript = MathML::from(&args[1]).string();

        format!(
            "<msup><mrow>{}</mrow><mrow>{}</mrow></msup>",
            base, superscript
        )
        .into()
    }

    pub fn vector(args: &[LispExpr]) -> Self {
        let components = args
            .iter()
            .map(|component| format!("<mtd>{}</mtd>", MathML::from(component).string()))
            .collect::<Vec<String>>()
            .join("");

        format!(
            "<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>",
            components
        )
        .into()
    }

    pub fn derivative(args: &[LispExpr]) -> Self {
        if args.len() >= 2 {
            if let LispExpr::Symbol(var) = &args[1] {
                let expr = MathML::from(&args[0]).string();

                // Check if order is specified
                if args.len() >= 3 {
                    if let LispExpr::Number(n) = args[2] {
                        if n as u32 > 1 {
                            return format!("<mrow><mfrac><msup><mi>d</mi><mn>{}</mn></msup><mrow><mi>d</mi><msup><mi>{}</mi><mn>{}</mn></msup></mrow></mfrac><mrow>{}</mrow></mrow>", 
                                n as u32, var, n as u32, expr).into();
                        }
                    }
                }

                // First order derivative
                format!("<mrow><mfrac><mi>d</mi><mrow><mi>d</mi><mi>{}</mi></mrow></mfrac><mrow>{}</mrow></mrow>", 
                    var, expr).into()
            } else {
                "<mrow>Error: differentiation variable must be a symbol</mrow>"
                    .to_string()
                    .into()
            }
        } else {
            "<mrow>Error: derivative requires at least an expression and variable</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn determinant(args: &[LispExpr]) -> Self {
        if args.len() == 1 {
            if let LispExpr::List(rows) = &args[0] {
                let rows_mathml = rows
                    .iter()
                    .map(|row| {
                        if let LispExpr::List(cells) = row {
                            let cells_mathml = cells
                                .iter()
                                .map(|cell| format!("<mtd>{}</mtd>", MathML::from(cell).string()))
                                .collect::<Vec<String>>()
                                .join("");
                            format!("<mtr>{}</mtr>", cells_mathml)
                        } else {
                            "<mtr><mtd>Error: matrix row must be a list</mtd></mtr>".to_string()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("");

                format!(
                    "<mrow><mo>|</mo><mtable>{}</mtable><mo>|</mo></mrow>",
                    rows_mathml
                )
                .into()
            } else {
                "<mrow>Error: determinant argument must be a matrix</mrow>"
                    .to_string()
                    .into()
            }
        } else {
            "<mrow>Error: determinant requires exactly one matrix argument</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn product(args: &[LispExpr]) -> Self {
        if args.len() >= 4 {
            if let LispExpr::Symbol(index) = &args[1] {
                let expr = MathML::from(&args[0]).string();
                let lower = MathML::from(&args[2]).string();
                let upper = MathML::from(&args[3]).string();

                format!("<mrow><munderover><mo>∏</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow>{}</munderover><mrow>{}</mrow></mrow>",
                    index, lower, upper, expr).into()
            } else {
                "<mrow>Error: product index must be a symbol</mrow>"
                    .to_string()
                    .into()
            }
        } else {
            "<mrow>Error: product requires expression, index, lower bound and upper bound</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn log(args: &[LispExpr]) -> Self {
        if args.len() == 1 {
            let arg = MathML::from(&args[0]).string();
            format!(
                "<mrow><mi>log</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
                arg
            )
            .into()
        } else if args.len() >= 2 {
            // Logarithm with base
            let base = args[0].to_string();
            if base == "e" {
                // pass args[0] onwars
                return Self::ln(args[1..].as_ref());
            }

            let base = MathML::from(&args[0]).string();
            let arg = MathML::from(&args[1]).string();
            format!("<mrow><msub><mi>log</mi><mrow>{}</mrow></msub><mo>&#x2061;</mo><mrow>{}</mrow></mrow>", 
                base, arg).into()
        } else {
            "<mrow>Error: log requires at least one argument</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn ln(args: &[LispExpr]) -> Self {
        if args.len() == 1 {
            let arg = MathML::from(&args[0]).string();
            format!(
                "<mrow><mi>ln</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
                arg
            )
            .into()
        } else {
            "<mrow>Error: ln requires exactly one argument</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn binomial(args: &[LispExpr]) -> Self {
        if args.len() >= 2 {
            let n = MathML::from(&args[0]).string();
            let k = MathML::from(&args[1]).string();

            format!("<mrow><mo>(</mo><mfrac linethickness=\"0\"><mrow>{}</mrow><mrow>{}</mrow></mfrac><mo>)</mo></mrow>", 
                n, k).into()
        } else {
            "<mrow>Error: binomial requires n and k</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn integral(args: &[LispExpr]) -> Self {
        if args.len() > 3 {
            if let LispExpr::Symbol(var) = &args[3] {
                // Definite integral
                format!(
                    "<mrow>
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
                    var
                )
                .into()
            } else {
                "<mrow>Error: integration variable must be a symbol</mrow>"
                    .to_string()
                    .into()
            }
        } else if let LispExpr::Symbol(var) = &args[1] {
            // Indefinite integral
            format!(
                "<mrow>
                <mo>∫</mo>
                <mrow>{}</mrow>
                <mi>{}</mi>
            </mrow>",
                MathML::from(&args[0]).string(),
                var
            )
            .into()
        } else {
            "<mrow>Error: integration variable must be a symbol</mrow>"
                .to_string()
                .into()
        }
    }

    pub fn sin(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>sin</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn cos(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>cos</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn tan(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>tan</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn sec(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>sec</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn csc(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>csc</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn cot(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>cot</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn arcsin(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>arcsin</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn arccos(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>arccos</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn arctan(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>arctan</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn sinh(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>sinh</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn cosh(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>cosh</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
    }

    pub fn tanh(args: &[LispExpr]) -> Self {
        let arg = MathML::from(&args[0]).string();
        format!(
            "<mrow><mi>tanh</mi><mo>&#x2061;</mo><mrow>{}</mrow></mrow>",
            arg
        )
        .into()
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
                "<mrow>Error: summation index must be a symbol</mrow>"
                    .to_string()
                    .into()
            }
        } else {
            "<mrow>Error: sum requires index, lower bound, upper bound, and expression</mrow>"
                .to_string()
                .into()
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
