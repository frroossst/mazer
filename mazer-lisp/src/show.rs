use mazer_types::LispAST;

use crate::environment::Environment;

/// The Show interpreter formats expressions symbolically as MathML
/// without computing their values.
pub struct Show {
    env: Environment,
}

impl Show {
    pub fn new(env: Environment) -> Self {
        Self { env }
    }

    /// Format an expression as MathML, keeping it symbolic
    pub fn format(&mut self, expr: LispAST) -> Result<String, String> {
        match expr {
            LispAST::Error(e) => Ok(format!("<merror><mtext>{}</mtext></merror>", escape_xml(&e))),
            
            LispAST::Number(n) => Ok(format!("<mn>{}</mn>", n)),
            
            LispAST::Bool(b) => Ok(format!("<mtext>{}</mtext>", b)),
            
            LispAST::String(s) => Ok(format!("<mtext>{}</mtext>", escape_xml(&s))),
            
            LispAST::Symbol(ref s) => {
                // Check if symbol is bound to a value - if so, still show symbolically
                // This keeps "a" as "a" even if (define a 5) was called
                Ok(format_symbol(s))
            }
            
            LispAST::List(ref exprs) if exprs.is_empty() => {
                Ok("<mrow></mrow>".to_string())
            }
            
            LispAST::List(exprs) => {
                self.format_list(&exprs)
            }
            
            LispAST::Application { name, args } => {
                // Format as function application symbolically
                let mut full_list = vec![LispAST::Symbol(name)];
                full_list.extend(args);
                self.format_list(&full_list)
            }
            
            LispAST::NativeFunc(_) | LispAST::UserFunc { .. } => {
                Ok("<mtext>⟨function⟩</mtext>".to_string())
            }
        }
    }
    
    /// Format a list expression, handling special mathematical forms
    fn format_list(&mut self, exprs: &[LispAST]) -> Result<String, String> {
        if exprs.is_empty() {
            return Ok("<mrow></mrow>".to_string());
        }
        
        // Check for special forms
        if let LispAST::Symbol(ref op) = exprs[0] {
            match op.as_str() {
                // Special forms that don't format as math
                "define" => return self.format_define(&exprs[1..]),
                "defunc" => return self.format_defunc(&exprs[1..]),
                "quote" => return self.format_quote(&exprs[1..]),
                
                // Mathematical operations
                "+" | "add" => return self.format_infix_op(&exprs[1..], "+"),
                "-" | "sub" => return self.format_subtraction(&exprs[1..]),
                "*" | "mul" => return self.format_infix_op(&exprs[1..], "×"),
                "/" | "div" => return self.format_fraction(&exprs[1..]),
                
                "pow" | "^" | "expt" => return self.format_power(&exprs[1..]),
                "frac" => return self.format_fraction(&exprs[1..]),
                "sqrt" => return self.format_sqrt(&exprs[1..]),
                "root" => return self.format_nthroot(&exprs[1..]),
                
                // Comparison operators
                "=" | "eq" => return self.format_infix_op(&exprs[1..], "="),
                "!=" | "neq" => return self.format_infix_op(&exprs[1..], "≠"),
                "<" | "lt" => return self.format_infix_op(&exprs[1..], "<"),
                ">" | "gt" => return self.format_infix_op(&exprs[1..], ">"),
                "<=" | "le" | "leq" => return self.format_infix_op(&exprs[1..], "≤"),
                ">=" | "ge" | "geq" => return self.format_infix_op(&exprs[1..], "≥"),
                
                // Calculus
                "integral" | "int" => return self.format_integral(&exprs[1..]),
                "sum" => return self.format_sum(&exprs[1..]),
                "prod" | "product" => return self.format_product(&exprs[1..]),
                "lim" | "limit" => return self.format_limit(&exprs[1..]),
                "deriv" | "derivative" => return self.format_derivative(&exprs[1..]),
                "partial" => return self.format_partial(&exprs[1..]),
                
                // Trigonometric functions
                "sin" | "cos" | "tan" | "cot" | "sec" | "csc" |
                "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh" => {
                    return self.format_trig_func(op, &exprs[1..]);
                }
                
                // Logarithms and exponentials
                "ln" => return self.format_func("ln", &exprs[1..]),
                "log" => return self.format_log(&exprs[1..]),
                "exp" => return self.format_exp(&exprs[1..]),
                
                // Other mathematical functions
                "abs" => return self.format_abs(&exprs[1..]),
                "floor" => return self.format_floor(&exprs[1..]),
                "ceil" => return self.format_ceil(&exprs[1..]),
                "fact" | "factorial" => return self.format_factorial(&exprs[1..]),
                "binom" | "choose" => return self.format_binomial(&exprs[1..]),
                
                // Matrices and vectors
                "matrix" => return self.format_matrix(&exprs[1..]),
                "vec" | "vector" => return self.format_vector(&exprs[1..]),
                "det" => return self.format_determinant(&exprs[1..]),
                
                // Set notation
                "set" => return self.format_set(&exprs[1..]),
                "in" | "elem" => return self.format_infix_op(&exprs[1..], "∈"),
                "notin" => return self.format_infix_op(&exprs[1..], "∉"),
                "subset" => return self.format_infix_op(&exprs[1..], "⊆"),
                "supset" => return self.format_infix_op(&exprs[1..], "⊇"),
                "union" => return self.format_infix_op(&exprs[1..], "∪"),
                "intersect" => return self.format_infix_op(&exprs[1..], "∩"),
                
                // Logic
                "and" => return self.format_infix_op(&exprs[1..], "∧"),
                "or" => return self.format_infix_op(&exprs[1..], "∨"),
                "not" => return self.format_not(&exprs[1..]),
                "implies" => return self.format_infix_op(&exprs[1..], "⟹"),
                "iff" => return self.format_infix_op(&exprs[1..], "⟺"),
                "forall" => return self.format_quantifier("∀", &exprs[1..]),
                "exists" => return self.format_quantifier("∃", &exprs[1..]),
                
                // Grouping/display
                "paren" => return self.format_parenthesized(&exprs[1..]),
                "bracket" => return self.format_bracketed(&exprs[1..]),
                "brace" => return self.format_braced(&exprs[1..]),
                
                // Text annotation
                "text" => return self.format_text(&exprs[1..]),
                "subscript" => return self.format_subscript(&exprs[1..]),
                "overline" | "bar" => return self.format_overline(&exprs[1..]),
                "hat" => return self.format_hat(&exprs[1..]),
                "dot" => return self.format_dot(&exprs[1..]),
                "ddot" => return self.format_ddot(&exprs[1..]),
                "vec-arrow" | "arrow" => return self.format_vec_arrow(&exprs[1..]),
                
                // Generic function application
                _ => {
                    // Check if it's a user-defined function - show as function application
                    if let Some(LispAST::UserFunc { params: _, body: _ }) = self.env.get(op).cloned() {
                        return self.format_func_application(op, &exprs[1..]);
                    }
                    // Otherwise format as generic function call
                    return self.format_func_application(op, &exprs[1..]);
                }
            }
        }
        
        // Default: format as space-separated row
        let parts: Result<Vec<_>, _> = exprs.iter()
            .map(|e| self.format(e.clone()))
            .collect();
        let parts = parts?;
        Ok(format!("<mrow>{}</mrow>", parts.join("")))
    }
    
    // ===== Special Forms =====
    
    fn format_define(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("define requires 2 arguments".to_string());
        }
        let name = self.format(args[0].clone())?;
        let value = self.format(args[1].clone())?;
        Ok(format!("<mrow>{}<mo>≔</mo>{}</mrow>", name, value))
    }
    
    fn format_defunc(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() < 3 {
            return Err("defunc requires at least 3 arguments".to_string());
        }
        let name = self.format(args[0].clone())?;
        let params = self.format(args[1].clone())?;
        let body = self.format(args[2].clone())?;
        Ok(format!("<mrow>{}<mo>(</mo>{}<mo>)</mo><mo>=</mo>{}</mrow>", name, params, body))
    }
    
    fn format_quote(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mrow></mrow>".to_string());
        }
        self.format(args[0].clone())
    }
    
    // ===== Arithmetic Operations =====
    
    fn format_infix_op(&mut self, args: &[LispAST], op: &str) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mrow></mrow>".to_string());
        }
        
        let parts: Result<Vec<_>, _> = args.iter()
            .map(|e| self.format(e.clone()))
            .collect();
        let parts = parts?;
        
        let operator = format!("<mo>{}</mo>", op);
        Ok(format!("<mrow>{}</mrow>", parts.join(&operator)))
    }
    
    fn format_subtraction(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mrow></mrow>".to_string());
        }
        
        if args.len() == 1 {
            // Unary minus
            let operand = self.format(args[0].clone())?;
            return Ok(format!("<mrow><mo>-</mo>{}</mrow>", operand));
        }
        
        self.format_infix_op(args, "−")
    }
    
    fn format_power(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("pow requires 2 arguments".to_string());
        }
        
        let base = self.format(args[0].clone())?;
        let exponent = self.format(args[1].clone())?;
        
        // Wrap complex bases in parentheses
        let base_wrapped = if needs_parens_for_power(&args[0]) {
            format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", base)
        } else {
            base
        };
        
        Ok(format!("<msup>{}{}</msup>", base_wrapped, exponent))
    }
    
    fn format_fraction(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("frac requires 2 arguments".to_string());
        }
        
        let numerator = self.format(args[0].clone())?;
        let denominator = self.format(args[1].clone())?;
        
        Ok(format!("<mfrac>{}{}</mfrac>", numerator, denominator))
    }
    
    fn format_sqrt(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("sqrt requires 1 argument".to_string());
        }
        
        let radicand = self.format(args[0].clone())?;
        Ok(format!("<msqrt>{}</msqrt>", radicand))
    }
    
    fn format_nthroot(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("root requires 2 arguments (index, radicand)".to_string());
        }
        
        let index = self.format(args[0].clone())?;
        let radicand = self.format(args[1].clone())?;
        
        Ok(format!("<mroot>{}{}</mroot>", radicand, index))
    }
    
    // ===== Calculus =====
    
    fn format_integral(&mut self, args: &[LispAST]) -> Result<String, String> {
        match args.len() {
            // Indefinite integral: (integral expr)
            1 => {
                let integrand = self.format(args[0].clone())?;
                Ok(format!("<mrow><mo>∫</mo>{}</mrow>", integrand))
            }
            // Definite integral: (integral lower upper expr)
            3 => {
                let lower = self.format(args[0].clone())?;
                let upper = self.format(args[1].clone())?;
                let integrand = self.format(args[2].clone())?;
                Ok(format!(
                    "<mrow><msubsup><mo>∫</mo>{}{}</msubsup>{}</mrow>",
                    lower, upper, integrand
                ))
            }
            // Definite integral with dx: (integral lower upper expr var)
            4 => {
                let lower = self.format(args[0].clone())?;
                let upper = self.format(args[1].clone())?;
                let integrand = self.format(args[2].clone())?;
                let var = self.format(args[3].clone())?;
                Ok(format!(
                    "<mrow><msubsup><mo>∫</mo>{}{}</msubsup>{}<mo>d</mo>{}</mrow>",
                    lower, upper, integrand, var
                ))
            }
            _ => Err("integral requires 1, 3, or 4 arguments".to_string())
        }
    }
    
    fn format_sum(&mut self, args: &[LispAST]) -> Result<String, String> {
        match args.len() {
            // Simple sum: (sum expr)
            1 => {
                let summand = self.format(args[0].clone())?;
                Ok(format!("<mrow><mo>∑</mo>{}</mrow>", summand))
            }
            // Bounded sum: (sum lower upper expr)
            3 => {
                let lower = self.format(args[0].clone())?;
                let upper = self.format(args[1].clone())?;
                let summand = self.format(args[2].clone())?;
                Ok(format!(
                    "<mrow><munderover><mo>∑</mo>{}{}</munderover>{}</mrow>",
                    lower, upper, summand
                ))
            }
            _ => Err("sum requires 1 or 3 arguments".to_string())
        }
    }
    
    fn format_product(&mut self, args: &[LispAST]) -> Result<String, String> {
        match args.len() {
            1 => {
                let factor = self.format(args[0].clone())?;
                Ok(format!("<mrow><mo>∏</mo>{}</mrow>", factor))
            }
            3 => {
                let lower = self.format(args[0].clone())?;
                let upper = self.format(args[1].clone())?;
                let factor = self.format(args[2].clone())?;
                Ok(format!(
                    "<mrow><munderover><mo>∏</mo>{}{}</munderover>{}</mrow>",
                    lower, upper, factor
                ))
            }
            _ => Err("product requires 1 or 3 arguments".to_string())
        }
    }
    
    fn format_limit(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() < 2 {
            return Err("limit requires at least 2 arguments (var, approach, [expr])".to_string());
        }
        
        let var = self.format(args[0].clone())?;
        let approach = self.format(args[1].clone())?;
        
        let limit_base = format!(
            "<munder><mo>lim</mo><mrow>{}<mo>→</mo>{}</mrow></munder>",
            var, approach
        );
        
        if args.len() >= 3 {
            let expr = self.format(args[2].clone())?;
            Ok(format!("<mrow>{}{}</mrow>", limit_base, expr))
        } else {
            Ok(limit_base)
        }
    }
    
    fn format_derivative(&mut self, args: &[LispAST]) -> Result<String, String> {
        match args.len() {
            // (deriv expr var) - derivative of expr with respect to var
            2 => {
                let expr = self.format(args[0].clone())?;
                let var = self.format(args[1].clone())?;
                Ok(format!(
                    "<mrow><mfrac><mi>d</mi><mrow><mi>d</mi>{}</mrow></mfrac>{}</mrow>",
                    var, expr
                ))
            }
            // (deriv expr var n) - nth derivative
            3 => {
                let expr = self.format(args[0].clone())?;
                let var = self.format(args[1].clone())?;
                let n = self.format(args[2].clone())?;
                Ok(format!(
                    "<mrow><mfrac><msup><mi>d</mi>{}</msup><mrow><mi>d</mi><msup>{}{}</msup></mrow></mfrac>{}</mrow>",
                    n, var, n, expr
                ))
            }
            _ => Err("derivative requires 2 or 3 arguments".to_string())
        }
    }
    
    fn format_partial(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("partial requires 2 arguments (expr, var)".to_string());
        }
        
        let expr = self.format(args[0].clone())?;
        let var = self.format(args[1].clone())?;
        
        Ok(format!(
            "<mrow><mfrac><mo>∂</mo><mrow><mo>∂</mo>{}</mrow></mfrac>{}</mrow>",
            var, expr
        ))
    }
    
    // ===== Trigonometric Functions =====
    
    fn format_trig_func(&mut self, name: &str, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err(format!("{} requires 1 argument", name));
        }
        
        let arg = self.format(args[0].clone())?;
        
        // Add parentheses for clarity with trig functions
        Ok(format!("<mrow><mi>{}</mi><mo>(</mo>{}<mo>)</mo></mrow>", name, arg))
    }
    
    fn format_func(&mut self, name: &str, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err(format!("{} requires 1 argument", name));
        }
        
        let arg = self.format(args[0].clone())?;
        Ok(format!("<mrow><mi>{}</mi><mo>(</mo>{}<mo>)</mo></mrow>", name, arg))
    }
    
    fn format_log(&mut self, args: &[LispAST]) -> Result<String, String> {
        match args.len() {
            1 => {
                // log base 10
                let arg = self.format(args[0].clone())?;
                Ok(format!("<mrow><mi>log</mi><mo>(</mo>{}<mo>)</mo></mrow>", arg))
            }
            2 => {
                // log with custom base
                let base = self.format(args[0].clone())?;
                let arg = self.format(args[1].clone())?;
                Ok(format!(
                    "<mrow><msub><mi>log</mi>{}</msub><mo>(</mo>{}<mo>)</mo></mrow>",
                    base, arg
                ))
            }
            _ => Err("log requires 1 or 2 arguments".to_string())
        }
    }
    
    fn format_exp(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("exp requires 1 argument".to_string());
        }
        
        let exponent = self.format(args[0].clone())?;
        Ok(format!("<msup><mi>e</mi>{}</msup>", exponent))
    }
    
    // ===== Other Mathematical Functions =====
    
    fn format_abs(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("abs requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>|</mo>{}<mo>|</mo></mrow>", inner))
    }
    
    fn format_floor(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("floor requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>⌊</mo>{}<mo>⌋</mo></mrow>", inner))
    }
    
    fn format_ceil(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("ceil requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>⌈</mo>{}<mo>⌉</mo></mrow>", inner))
    }
    
    fn format_factorial(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("factorial requires 1 argument".to_string());
        }
        
        let n = self.format(args[0].clone())?;
        
        // Wrap in parens if complex
        let wrapped = if needs_parens_for_factorial(&args[0]) {
            format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", n)
        } else {
            n
        };
        
        Ok(format!("<mrow>{}<mo>!</mo></mrow>", wrapped))
    }
    
    fn format_binomial(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("binom requires 2 arguments (n, k)".to_string());
        }
        
        let n = self.format(args[0].clone())?;
        let k = self.format(args[1].clone())?;
        
        Ok(format!(
            "<mrow><mo>(</mo><mfrac linethickness=\"0\">{}{}</mfrac><mo>)</mo></mrow>",
            n, k
        ))
    }
    
    // ===== Matrices and Vectors =====
    
    fn format_matrix(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mtable></mtable>".to_string());
        }
        
        let mut rows = Vec::new();
        for row_expr in args {
            if let LispAST::List(row_items) = row_expr {
                let mut cells = Vec::new();
                for item in row_items {
                    let formatted = self.format(item.clone())?;
                    cells.push(format!("<mtd>{}</mtd>", formatted));
                }
                rows.push(format!("<mtr>{}</mtr>", cells.join("")));
            } else {
                let formatted = self.format(row_expr.clone())?;
                rows.push(format!("<mtr><mtd>{}</mtd></mtr>", formatted));
            }
        }
        
        Ok(format!(
            "<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>",
            rows.join("")
        ))
    }
    
    fn format_vector(&mut self, args: &[LispAST]) -> Result<String, String> {
        let mut cells = Vec::new();
        for item in args {
            let formatted = self.format(item.clone())?;
            cells.push(format!("<mtr><mtd>{}</mtd></mtr>", formatted));
        }
        
        Ok(format!(
            "<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>",
            cells.join("")
        ))
    }
    
    fn format_determinant(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mrow><mo>|</mo><mo>|</mo></mrow>".to_string());
        }
        
        let mut rows = Vec::new();
        for row_expr in args {
            if let LispAST::List(row_items) = row_expr {
                let mut cells = Vec::new();
                for item in row_items {
                    let formatted = self.format(item.clone())?;
                    cells.push(format!("<mtd>{}</mtd>", formatted));
                }
                rows.push(format!("<mtr>{}</mtr>", cells.join("")));
            } else {
                let formatted = self.format(row_expr.clone())?;
                rows.push(format!("<mtr><mtd>{}</mtd></mtr>", formatted));
            }
        }
        
        Ok(format!(
            "<mrow><mo>|</mo><mtable>{}</mtable><mo>|</mo></mrow>",
            rows.join("")
        ))
    }
    
    // ===== Set Notation =====
    
    fn format_set(&mut self, args: &[LispAST]) -> Result<String, String> {
        let mut elements = Vec::new();
        for e in args {
            elements.push(self.format(e.clone())?);
        }
        
        Ok(format!(
            "<mrow><mo>{{</mo>{}<mo>}}</mo></mrow>",
            elements.join("<mo>,</mo>")
        ))
    }
    
    // ===== Logic =====
    
    fn format_not(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("not requires 1 argument".to_string());
        }
        
        let operand = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>¬</mo>{}</mrow>", operand))
    }
    
    fn format_quantifier(&mut self, symbol: &str, args: &[LispAST]) -> Result<String, String> {
        if args.len() < 2 {
            return Err(format!("{} requires at least 2 arguments (var, expr)", symbol));
        }
        
        let var = self.format(args[0].clone())?;
        let expr = self.format(args[1].clone())?;
        
        Ok(format!("<mrow><mo>{}</mo>{}<mo>:</mo>{}</mrow>", symbol, var, expr))
    }
    
    // ===== Grouping =====
    
    fn format_parenthesized(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("paren requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", inner))
    }
    
    fn format_bracketed(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("bracket requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>[</mo>{}<mo>]</mo></mrow>", inner))
    }
    
    fn format_braced(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("brace requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mrow><mo>{{</mo>{}<mo>}}</mo></mrow>", inner))
    }
    
    // ===== Text Annotations =====
    
    fn format_text(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.is_empty() {
            return Ok("<mtext></mtext>".to_string());
        }
        
        let text: Vec<String> = args.iter()
            .map(|arg| match arg {
                LispAST::String(s) => escape_xml(s),
                LispAST::Symbol(s) => escape_xml(s),
                _ => format!("{:?}", arg),
            })
            .collect();
        
        Ok(format!("<mtext>{}</mtext>", text.join(" ")))
    }
    
    fn format_subscript(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("subscript requires 2 arguments (base, subscript)".to_string());
        }
        
        let base = self.format(args[0].clone())?;
        let sub = self.format(args[1].clone())?;
        
        Ok(format!("<msub>{}{}</msub>", base, sub))
    }
    
    fn format_overline(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("overline requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mover>{}<mo>¯</mo></mover>", inner))
    }
    
    fn format_hat(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("hat requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mover>{}<mo>^</mo></mover>", inner))
    }
    
    fn format_dot(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("dot requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mover>{}<mo>˙</mo></mover>", inner))
    }
    
    fn format_ddot(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("ddot requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mover>{}<mo>¨</mo></mover>", inner))
    }
    
    fn format_vec_arrow(&mut self, args: &[LispAST]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("vec-arrow requires 1 argument".to_string());
        }
        
        let inner = self.format(args[0].clone())?;
        Ok(format!("<mover>{}<mo>→</mo></mover>", inner))
    }
    
    // ===== Generic Function Application =====
    
    fn format_func_application(&mut self, name: &str, args: &[LispAST]) -> Result<String, String> {
        let formatted_args: Result<Vec<_>, _> = args.iter()
            .map(|e| self.format(e.clone()))
            .collect();
        let formatted_args = formatted_args?;
        
        let func_name = format_symbol(name);
        
        if formatted_args.is_empty() {
            Ok(format!("<mrow>{}<mo>(</mo><mo>)</mo></mrow>", func_name))
        } else {
            Ok(format!(
                "<mrow>{}<mo>(</mo>{}<mo>)</mo></mrow>",
                func_name,
                formatted_args.join("<mo>,</mo>")
            ))
        }
    }
    
    pub fn env(&self) -> &Environment {
        &self.env
    }
}

// ===== Helper Functions =====

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

/// Format a symbol, handling Greek letters and special symbols
fn format_symbol(s: &str) -> String {
    use mazer_atog::Atog;
    
    if let Some(entity) = Atog::get(s) {
        format!("<mi>{}</mi>", entity)
    } else {
        format!("<mi>{}</mi>", escape_xml(s))
    }
}

/// Check if an expression needs parentheses when used as a base in exponentiation
fn needs_parens_for_power(expr: &LispAST) -> bool {
    matches!(expr, 
        LispAST::List(exprs) if !exprs.is_empty() && matches!(&exprs[0], 
            LispAST::Symbol(s) if matches!(s.as_str(), 
                "+" | "-" | "*" | "/" | "add" | "sub" | "mul" | "div"
            )
        )
    )
}

/// Check if an expression needs parentheses when followed by factorial
fn needs_parens_for_factorial(expr: &LispAST) -> bool {
    matches!(expr, 
        LispAST::List(_) | LispAST::Application { .. }
    )
}
