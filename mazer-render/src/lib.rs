use mazer_atog::Atog;
use mazer_types::{LispAST, Environment};

pub mod docs;

pub trait ToMathML {
    fn to_mathml(&self) -> String;
}

impl ToMathML for LispAST {
    fn to_mathml(&self) -> String {
        format_mathml(self, None)
    }
}

/// Format LispAST to MathML with optional environment for user-defined function lookup
pub fn format_mathml_with_env(expr: &LispAST, env: Option<&Environment>) -> String {
    format_mathml(expr, env)
}

/// Main MathML formatting function
fn format_mathml(expr: &LispAST, env: Option<&Environment>) -> String {
    match expr {
        LispAST::Error(e) => format!("<merror><mtext>{}</mtext></merror>", escape_xml(e)),
        
        LispAST::Number(n) => format!("<mn>{}</mn>", n),
        
        LispAST::Bool(b) => format!("<mtext>{}</mtext>", b),
        
        LispAST::String(s) => format!("<mtext>{}</mtext>", escape_xml(s)),
        
        LispAST::Symbol(s) => {
            let t = format_symbol(s);
            dbg!(t.clone());
            t
        }
        
        LispAST::List(exprs) if exprs.is_empty() => "<mrow></mrow>".to_string(),
        
        LispAST::List(exprs) => format_list(exprs, env),
        
        LispAST::Application { name, args } => {
            let mut full_list = vec![LispAST::Symbol(name.clone())];
            full_list.extend(args.clone());
            format_list(&full_list, env)
        }
        
        LispAST::NativeFunc(_) | LispAST::UserFunc { .. } => {
            "<mtext>⟨function⟩</mtext>".to_string()
        }
    }
}

/// Format a list expression, handling special mathematical forms
fn format_list(exprs: &[LispAST], env: Option<&Environment>) -> String {
    if exprs.is_empty() {
        return "<mrow></mrow>".to_string();
    }
    
    // Check for special forms
    if let LispAST::Symbol(op) = &exprs[0] {
        let args = &exprs[1..];
        match op.as_str() {
            // Special forms
            "define" => return format_define(args, env),
            "defunc" => return format_defunc(args, env),
            "quote" => return format_quote(args, env),
            "string" => return format_string(args, env),
            
            // Arithmetic
            "+" | "add" => return format_infix_op(args, "+", env),
            "-" | "sub" => return format_subtraction(args, env),
            "*" | "mul" => return format_infix_op(args, "×", env),
            "/" | "div" => return format_fraction(args, env),
            
            "pow" | "^" | "expt" => return format_power(args, env),
            "frac" => return format_fraction(args, env),
            "sqrt" => return format_sqrt(args, env),
            "root" => return format_nthroot(args, env),
            
            // Comparisons
            "=" | "eq" => return format_infix_op(args, "=", env),
            "!=" | "neq" => return format_infix_op(args, "≠", env),
            "<" | "lt" => return format_infix_op(args, "<", env),
            ">" | "gt" => return format_infix_op(args, ">", env),
            "<=" | "le" | "leq" => return format_infix_op(args, "≤", env),
            ">=" | "ge" | "geq" => return format_infix_op(args, "≥", env),
            
            // Calculus
            "integral" | "int" => return format_integral(args, env),
            "sum" => return format_sum(args, env),
            "prod" | "product" => return format_product(args, env),
            "lim" | "limit" => return format_limit(args, env),
            "deriv" | "derivative" => return format_derivative(args, env),
            "partial" => return format_partial(args, env),
            
            // Trig functions
            "sin" | "cos" | "tan" | "cot" | "sec" | "csc" |
            "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh" => {
                return format_trig(op, args, env);
            }
            
            // Logarithms
            "ln" => return format_func("ln", args, env),
            "log" => return format_log(args, env),
            "exp" => return format_exp(args, env),
            
            // Other math functions
            "abs" => return format_abs(args, env),
            "floor" => return format_floor(args, env),
            "ceil" => return format_ceil(args, env),
            "fact" | "factorial" => return format_factorial(args, env),
            "binom" | "choose" => return format_binomial(args, env),
            
            // Matrices
            "matrix" => return format_matrix(args, env),
            "vec" | "vector" => return format_vector(args, env),
            "det" => return format_determinant(args, env),
            
            // Sets
            "set" => return format_set(args, env),
            "in" | "elem" => return format_infix_op(args, "∈", env),
            "notin" => return format_infix_op(args, "∉", env),
            "subset" => return format_infix_op(args, "⊆", env),
            "supset" => return format_infix_op(args, "⊇", env),
            "union" => return format_infix_op(args, "∪", env),
            "intersect" => return format_infix_op(args, "∩", env),
            
            // Logic
            "and" => return format_infix_op(args, "∧", env),
            "or" => return format_infix_op(args, "∨", env),
            "not" => return format_not(args, env),
            "implies" => return format_infix_op(args, "⟹", env),
            "iff" => return format_infix_op(args, "⟺", env),
            "forall" => return format_quantifier("∀", args, env),
            "exists" => return format_quantifier("∃", args, env),
            
            // Grouping
            "paren" => return format_parenthesized(args, env),
            "bracket" => return format_bracketed(args, env),
            "brace" => return format_braced(args, env),
            
            // Annotations
            "text" => return format_text(args, env),
            "subscript" => return format_subscript(args, env),
            "overline" | "bar" => return format_overline(args, env),
            "hat" => return format_hat(args, env),
            "dot" => return format_dot(args, env),
            "ddot" => return format_ddot(args, env),
            "vec-arrow" | "arrow" => return format_vec_arrow(args, env),
            
            // Generic function call - check if user-defined
            _ => {
                // check in mazer_atog environment first
                if let Some(e) = Atog::get(op) {
                    // we found the function to be in atog env
                    return format_symbol(e);
                }

                // Check if it's a user-defined function
                if let Some(e) = env {
                    if let Some(LispAST::UserFunc { .. }) = e.get(op) {
                        return format_func_application(op, args, env);
                    }
                }
                return format_func_application(op, args, env);
            }
        }
    }
    
    // Default: format as space-separated row
    let parts: Vec<_> = exprs.iter().map(|e| format_mathml(e, env)).collect();
    format!("<mrow>{}</mrow>", parts.join(""))
}

// ===== Special Forms =====

fn format_define(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>define requires 2 arguments</mtext></merror>".to_string();
    }
    let name = format_mathml(&args[0], env);
    let value = format_mathml(&args[1], env);
    format!("<mrow>{}<mo>≔</mo>{}</mrow>", name, value)
}

fn format_defunc(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() < 3 {
        return "<merror><mtext>defunc requires at least 3 arguments</mtext></merror>".to_string();
    }
    let name = format_mathml(&args[0], env);
    let params = format_mathml(&args[1], env);
    let body = format_mathml(&args[2], env);
    format!("<mrow>{}<mo>(</mo>{}<mo>)</mo><mo>=</mo>{}</mrow>", name, params, body)
}

fn format_quote(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow></mrow>".to_string();
    }

    // quoted expressions are rendered as-is
    match &args[0] {
        LispAST::String(s) => {
            format!("<mtext>{}</mtext>", escape_xml(s))
        }
        LispAST::Symbol(s) => {
            format_symbol(s)
        }
        _ => {
            format_mathml(&args[0], env)
        }
    }

}

fn format_string(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow></mrow>".to_string();
    }

    // String expressions are rendered by showing all their content
    // without the "string(...)" wrapper
    // Handle each argument, preserving parentheses for lists
    let parts: Vec<_> = args.iter().map(|e| {
        match e {
            LispAST::List(items) if !items.is_empty() => {
                // Render list contents with parentheses
                let inner = items.iter().map(|item| format_mathml(item, env)).collect::<Vec<_>>().join("");
                format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", inner)
            }
            _ => format_mathml(e, env)
        }
    }).collect();
    format!("<mrow>{}</mrow>", parts.join(""))
}

// ===== Arithmetic =====

fn format_infix_op(args: &[LispAST], op: &str, env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow></mrow>".to_string();
    }
    let parts: Vec<_> = args.iter().map(|e| format_mathml(e, env)).collect();
    let operator = format!("<mo>{}</mo>", op);
    format!("<mrow>{}</mrow>", parts.join(&operator))
}

fn format_subtraction(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow></mrow>".to_string();
    }
    if args.len() == 1 {
        let operand = format_mathml(&args[0], env);
        return format!("<mrow><mo>-</mo>{}</mrow>", operand);
    }
    format_infix_op(args, "−", env)
}

fn format_power(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>pow requires 2 arguments</mtext></merror>".to_string();
    }
    let base = format_mathml(&args[0], env);
    let exponent = format_mathml(&args[1], env);
    
    let base_wrapped = if needs_parens_for_power(&args[0]) {
        format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", base)
    } else {
        base
    };
    
    format!("<msup>{}{}</msup>", base_wrapped, exponent)
}

fn format_fraction(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>frac requires 2 arguments</mtext></merror>".to_string();
    }
    let numerator = format_mathml(&args[0], env);
    let denominator = format_mathml(&args[1], env);
    format!("<mfrac>{}{}</mfrac>", numerator, denominator)
}

fn format_sqrt(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>sqrt requires 1 argument</mtext></merror>".to_string();
    }
    let radicand = format_mathml(&args[0], env);
    format!("<msqrt>{}</msqrt>", radicand)
}

fn format_nthroot(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>root requires 2 arguments</mtext></merror>".to_string();
    }
    let index = format_mathml(&args[0], env);
    let radicand = format_mathml(&args[1], env);
    format!("<mroot>{}{}</mroot>", radicand, index)
}

// ===== Calculus =====

fn format_integral(args: &[LispAST], env: Option<&Environment>) -> String {
    match args.len() {
        1 => {
            // (integral expr) - unbounded integral without differential
            let integrand = format_mathml(&args[0], env);
            format!("<mrow><mo>∫</mo>{}</mrow>", integrand)
        }
        2 => {
            // (integral expr var) - indefinite integral with differential: ∫ expr dvar
            let integrand = format_mathml(&args[0], env);
            let var = format_mathml(&args[1], env);
            format!("<mrow><mo>∫</mo>{}<mo>d</mo>{}</mrow>", integrand, var)
        }
        3 => {
            // (integral lower upper expr) - definite integral without explicit differential
            let lower = format_mathml(&args[0], env);
            let upper = format_mathml(&args[1], env);
            let integrand = format_mathml(&args[2], env);
            format!("<mrow><msubsup><mo>∫</mo>{}{}</msubsup>{}</mrow>", lower, upper, integrand)
        }
        4 => {
            // (integral lower upper expr var) - definite integral with differential
            let lower = format_mathml(&args[0], env);
            let upper = format_mathml(&args[1], env);
            let integrand = format_mathml(&args[2], env);
            let var = format_mathml(&args[3], env);
            format!("<mrow><msubsup><mo>∫</mo>{}{}</msubsup>{}<mo>d</mo>{}</mrow>", lower, upper, integrand, var)
        }
        _ => "<merror><mtext>integral requires 1, 2, 3, or 4 arguments</mtext></merror>".to_string()
    }
}

fn format_sum(args: &[LispAST], env: Option<&Environment>) -> String {
    match args.len() {
        1 => {
            let summand = format_mathml(&args[0], env);
            format!("<mrow><mo>∑</mo>{}</mrow>", summand)
        }
        3 => {
            let lower = format_mathml(&args[0], env);
            let upper = format_mathml(&args[1], env);
            let summand = format_mathml(&args[2], env);
            format!("<mrow><munderover><mo>∑</mo>{}{}</munderover>{}</mrow>", lower, upper, summand)
        }
        _ => "<merror><mtext>sum requires 1 or 3 arguments</mtext></merror>".to_string()
    }
}

fn format_product(args: &[LispAST], env: Option<&Environment>) -> String {
    match args.len() {
        1 => {
            let factor = format_mathml(&args[0], env);
            format!("<mrow><mo>∏</mo>{}</mrow>", factor)
        }
        3 => {
            let lower = format_mathml(&args[0], env);
            let upper = format_mathml(&args[1], env);
            let factor = format_mathml(&args[2], env);
            format!("<mrow><munderover><mo>∏</mo>{}{}</munderover>{}</mrow>", lower, upper, factor)
        }
        _ => "<merror><mtext>product requires 1 or 3 arguments</mtext></merror>".to_string()
    }
}

fn format_limit(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() < 2 {
        return "<merror><mtext>limit requires at least 2 arguments</mtext></merror>".to_string();
    }
    let var = format_mathml(&args[0], env);
    let approach = format_mathml(&args[1], env);
    let limit_base = format!("<munder><mo>lim</mo><mrow>{}<mo>→</mo>{}</mrow></munder>", var, approach);
    
    if args.len() >= 3 {
        let expr = format_mathml(&args[2], env);
        format!("<mrow>{}{}</mrow>", limit_base, expr)
    } else {
        limit_base
    }
}

fn format_derivative(args: &[LispAST], env: Option<&Environment>) -> String {
    match args.len() {
        2 => {
            let expr = format_mathml(&args[0], env);
            let var = format_mathml(&args[1], env);
            format!("<mrow><mfrac><mi>d</mi><mrow><mi>d</mi>{}</mrow></mfrac>{}</mrow>", var, expr)
        }
        3 => {
            let expr = format_mathml(&args[0], env);
            let var = format_mathml(&args[1], env);
            let n = format_mathml(&args[2], env);
            format!("<mrow><mfrac><msup><mi>d</mi>{}</msup><mrow><mi>d</mi><msup>{}{}</msup></mrow></mfrac>{}</mrow>", n, var, n, expr)
        }
        _ => "<merror><mtext>derivative requires 2 or 3 arguments</mtext></merror>".to_string()
    }
}

fn format_partial(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>partial requires 2 arguments</mtext></merror>".to_string();
    }
    let expr = format_mathml(&args[0], env);
    let var = format_mathml(&args[1], env);
    format!("<mrow><mfrac><mo>∂</mo><mrow><mo>∂</mo>{}</mrow></mfrac>{}</mrow>", var, expr)
}

// ===== Functions =====

fn format_func(name: &str, args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return format!("<merror><mtext>{} requires 1 argument</mtext></merror>", name);
    }
    let arg = format_mathml(&args[0], env);
    format!("<mrow><mi>{}</mi><mo>(</mo>{}<mo>)</mo></mrow>", name, arg)
}

fn format_trig(name: &str, args: &[LispAST], env: Option<&Environment>) -> String {
    // renders just as functions regularly, but arctan and tanh and the likes 
    // are shown as tan^-1 
    if args.len() != 1 {
        return format!("<merror><mtext>{} requires 1 argument</mtext></merror>", name);
    }

    let arg = format_mathml(&args[0], env);
    let (func_name, is_inverse) = match name {
        "arcsin" => ("sin", true),
        "arccos" => ("cos", true),
        "arctan" => ("tan", true),
        _ => (name, false),
    };

    if is_inverse {
        format!("<mrow><msup><mi>{}</mi><mo>⁻¹</mo></msup><mo>(</mo>{}<mo>)</mo></mrow>", func_name, arg)
    } else {
        format!("<mrow><mi>{}</mi><mo>(</mo>{}<mo>)</mo></mrow>", func_name, arg)
    }

}

fn format_log(args: &[LispAST], env: Option<&Environment>) -> String {
    match args.len() {
        1 => {
            let arg = format_mathml(&args[0], env);
            format!("<mrow><mi>log</mi><mo>(</mo>{}<mo>)</mo></mrow>", arg)
        }
        2 => {
            let base = format_mathml(&args[0], env);
            let arg = format_mathml(&args[1], env);
            format!("<mrow><msub><mi>log</mi>{}</msub><mo>(</mo>{}<mo>)</mo></mrow>", base, arg)
        }
        _ => "<merror><mtext>log requires 1 or 2 arguments</mtext></merror>".to_string()
    }
}

fn format_exp(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>exp requires 1 argument</mtext></merror>".to_string();
    }
    let exponent = format_mathml(&args[0], env);
    format!("<msup><mi>e</mi>{}</msup>", exponent)
}

// ===== Other Math Functions =====

fn format_abs(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>abs requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>|</mo>{}<mo>|</mo></mrow>", inner)
}

fn format_floor(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>floor requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>⌊</mo>{}<mo>⌋</mo></mrow>", inner)
}

fn format_ceil(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>ceil requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>⌈</mo>{}<mo>⌉</mo></mrow>", inner)
}

fn format_factorial(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>factorial requires 1 argument</mtext></merror>".to_string();
    }
    let n = format_mathml(&args[0], env);
    let wrapped = if needs_parens_for_factorial(&args[0]) {
        format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", n)
    } else {
        n
    };
    format!("<mrow>{}<mo>!</mo></mrow>", wrapped)
}

fn format_binomial(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>binom requires 2 arguments</mtext></merror>".to_string();
    }
    let n = format_mathml(&args[0], env);
    let k = format_mathml(&args[1], env);
    format!("<mrow><mo>(</mo><mfrac linethickness=\"0\">{}{}</mfrac><mo>)</mo></mrow>", n, k)
}

// ===== Matrices =====

fn format_matrix(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mtable></mtable>".to_string();
    }
    
    let rows: Vec<_> = args.iter().map(|row_expr| {
        if let LispAST::List(row_items) = row_expr {
            let cells: Vec<_> = row_items.iter()
                .map(|item| format!("<mtd>{}</mtd>", format_mathml(item, env)))
                .collect();
            format!("<mtr>{}</mtr>", cells.join(""))
        } else {
            format!("<mtr><mtd>{}</mtd></mtr>", format_mathml(row_expr, env))
        }
    }).collect();
    
    format!("<mrow><mo>(</mo><mtable>{}</mtable><mo>)</mo></mrow>", rows.join(""))
}

fn format_vector(args: &[LispAST], env: Option<&Environment>) -> String {
    let rows: Vec<_> = args.iter()
        .map(|item| format!("<mtr><mtd>{}</mtd></mtr>", format_mathml(item, env)))
        .collect();
    format!("<mrow><mo>(</mo><mtable>{}</mtable><mo>)</mo></mrow>", rows.join(""))
}

fn format_determinant(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow><mo>|</mo><mtable></mtable><mo>|</mo></mrow>".to_string();
    }
    
    let rows: Vec<_> = args.iter().map(|row_expr| {
        if let LispAST::List(row_items) = row_expr {
            let cells: Vec<_> = row_items.iter()
                .map(|item| format!("<mtd>{}</mtd>", format_mathml(item, env)))
                .collect();
            format!("<mtr>{}</mtr>", cells.join(""))
        } else {
            format!("<mtr><mtd>{}</mtd></mtr>", format_mathml(row_expr, env))
        }
    }).collect();
    
    format!("<mrow><mo>|</mo><mtable>{}</mtable><mo>|</mo></mrow>", rows.join(""))
}

// ===== Sets =====

fn format_set(args: &[LispAST], env: Option<&Environment>) -> String {
    let elements: Vec<_> = args.iter().map(|e| format_mathml(e, env)).collect();
    format!("<mrow><mo>{{</mo>{}<mo>}}</mo></mrow>", elements.join("<mo>,</mo>"))
}

// ===== Logic =====

fn format_not(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>not requires 1 argument</mtext></merror>".to_string();
    }
    let operand = format_mathml(&args[0], env);
    format!("<mrow><mo>¬</mo>{}</mrow>", operand)
}

fn format_quantifier(symbol: &str, args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() < 2 {
        return "<merror><mtext>quantifier requires at least 2 arguments</mtext></merror>".to_string();
    }
    let var = format_mathml(&args[0], env);
    let expr = format_mathml(&args[1], env);
    format!("<mrow><mo>{}</mo>{}<mo>.</mo>{}</mrow>", symbol, var, expr)
}

// ===== Grouping =====

fn format_parenthesized(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow><mo>(</mo><mo>)</mo></mrow>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", inner)
}

fn format_bracketed(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow><mo>[</mo><mo>]</mo></mrow>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>[</mo>{}<mo>]</mo></mrow>", inner)
}

fn format_braced(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.is_empty() {
        return "<mrow><mo>{{</mo><mo>}}</mo></mrow>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mrow><mo>{{</mo>{}<mo>}}</mo></mrow>", inner)
}

// ===== Annotations =====

fn format_text(args: &[LispAST], _env: Option<&Environment>) -> String {
    let text_parts: Vec<_> = args.iter().map(|arg| {
        match arg {
            LispAST::String(s) => escape_xml(s),
            LispAST::Symbol(s) => s.clone(),
            _ => format!("{:?}", arg),
        }
    }).collect();
    format!("<mtext>{}</mtext>", text_parts.join(" "))
}

fn format_subscript(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 2 {
        return "<merror><mtext>subscript requires 2 arguments</mtext></merror>".to_string();
    }
    let base = format_mathml(&args[0], env);
    let sub = format_mathml(&args[1], env);
    format!("<msub>{}{}</msub>", base, sub)
}

fn format_overline(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>overline requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mover>{}<mo>¯</mo></mover>", inner)
}

fn format_hat(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>hat requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mover>{}<mo>^</mo></mover>", inner)
}

fn format_dot(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>dot requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mover>{}<mo>˙</mo></mover>", inner)
}

fn format_ddot(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>ddot requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mover>{}<mo>¨</mo></mover>", inner)
}

fn format_vec_arrow(args: &[LispAST], env: Option<&Environment>) -> String {
    if args.len() != 1 {
        return "<merror><mtext>vec-arrow requires 1 argument</mtext></merror>".to_string();
    }
    let inner = format_mathml(&args[0], env);
    format!("<mover>{}<mo>→</mo></mover>", inner)
}

// ===== Generic Function Application =====

fn format_func_application(name: &str, args: &[LispAST], env: Option<&Environment>) -> String {
    let formatted_args: Vec<_> = args.iter().map(|e| format_mathml(e, env)).collect();
    let func_name = format_symbol(name);
    
    if formatted_args.is_empty() {
        format!("<mrow>{}<mo>(</mo><mo>)</mo></mrow>", func_name)
    } else {
        format!("<mrow>{}<mo>(</mo>{}<mo>)</mo></mrow>", func_name, formatted_args.join("<mo>,</mo>"))
    }
}

// ===== Helper Functions =====

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

fn format_symbol(s: &str) -> String {
    if let Some(entity) = Atog::get(s) {
        format!("<mi>{}</mi>", entity)
    } else {
        format!("<mi>{}</mi>", escape_xml(s))
    }
}

fn needs_parens_for_power(expr: &LispAST) -> bool {
    matches!(expr, 
        LispAST::List(exprs) if !exprs.is_empty() && matches!(&exprs[0], 
            LispAST::Symbol(s) if matches!(s.as_str(), 
                "+" | "-" | "*" | "/" | "add" | "sub" | "mul" | "div"
            )
        )
    )
}

fn needs_parens_for_factorial(expr: &LispAST) -> bool {
    matches!(expr, LispAST::List(_) | LispAST::Application { .. })
}
