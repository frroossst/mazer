use crate::{
    interpreter::{Environment, Interpreter},
    parser::{LispErr, LispExpr},
};

impl Interpreter {
    /// Creates a standard environment with common functions and constants
    /// This is the entire standard library of the interpreter!
    pub fn stdlib() -> Environment {
        let mut env = Interpreter::stdenv();
        // Math functions
        env.insert(
            "sqrt".to_string(),
            LispExpr::Function(|args, _env| {
                if args.len() != 1 {
                    return Err(LispErr::new("sqrt requires exactly one argument"));
                }

                if let LispExpr::Number(n) = args[0] {
                    if n < 0.0 {
                        return Err(LispErr::new(&format!(
                            "Cannot take square root of negative number: {}",
                            n
                        )));
                    }
                    Ok(LispExpr::Number(n.sqrt()))
                } else {
                    Err(LispErr::new(&format!("Expected number, got: {}", args[0])))
                }
            }),
        );

        env.insert(
            "pow".to_string(),
            LispExpr::Function(|args, _env| {
                if args.len() != 2 {
                    return Err(LispErr::new("pow requires exactly two arguments"));
                }

                if let (LispExpr::Number(base), LispExpr::Number(exp)) = (&args[0], &args[1]) {
                    Ok(LispExpr::Number(base.powf(*exp)))
                } else {
                    Err(LispErr::new(&format!(
                        "Expected numbers, got: {} and {}",
                        args[0], args[1]
                    )))
                }
            }),
        );

        // matrix constructor - just returns the list as-is
        // a faer matrix is constructed on the fly as needed (lazily)
        env.insert(
            "matrix".to_string(),
            LispExpr::Function(|args, _| Ok(LispExpr::List(args.to_vec()))),
        );

        // Add the dot product function
        env.insert(
            "dot".to_string(),
            LispExpr::Function(|_args, _env| unimplemented!()),
        );

        env.insert(
            "integral".to_string(),
            LispExpr::Function(|_args, _| unimplemented!()),
        );

        env
    }
}
