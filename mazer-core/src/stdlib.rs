use crate::{data_structures::Matrix, interpreter::{Environment, Interpreter}, parser::{LispErr, LispExpr}};


impl Interpreter {
    /// Creates a standard environment with common functions and constants
    /// This is the entire standard library of the interpreter!
    pub fn stdlib() -> Environment {
        let mut env = Interpreter::stdenv();
        // Math functions
        env.insert("sqrt".to_string(), LispExpr::Function(|args, _env| {
            if args.len() != 1 {
                return Err(LispErr::new("sqrt requires exactly one argument"));
            }
            
            if let LispExpr::Number(n) = args[0] {
                if n < 0.0 {
                    return Err(LispErr::new(&format!("Cannot take square root of negative number: {}", n)));
                }
                Ok(LispExpr::Number(n.sqrt()))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {}", args[0])))
            }
        }));
        
        env.insert("pow".to_string(), LispExpr::Function(|args, _env| {
            if args.len() != 2 {
                return Err(LispErr::new("pow requires exactly two arguments"));
            }
            
            if let (LispExpr::Number(base), LispExpr::Number(exp)) = (&args[0], &args[1]) {
                Ok(LispExpr::Number(base.powf(*exp)))
            } else {
                Err(LispErr::new(&format!("Expected numbers, got: {} and {}", args[0], args[1])))
            }
        }));

        // matrix constructor - just returns the list as-is 
        // a faer matrix is constructed on the fly as needed (lazily)
        env.insert("matrix".to_string(), LispExpr::Function(|args, _| {
            Ok(LispExpr::List(args.to_vec()))
        }));

        // Add the dot product function
        env.insert("dot".to_string(), LispExpr::Function(|args, env| {
            dbg!(&args);

            if args.len() != 2 {
                return Err(LispErr::new("dot requires exactly two arguments"));
            }

            // ensure both arguments are lists with first symbol being a matrix
            if !matches!(args[0], LispExpr::List(_)) || !matches!(args[1], LispExpr::List(_)) {
                return Err(LispErr::new("dot requires two lists as arguments"));
            }

            // figure out if matrix is [1, 2, 3] or
            // [ 1
            //   2
            //   3 ]

            let v1  = match &args[0] {
                LispExpr::List(list) => {
                    if list.len() == 1 && matches!(list[0], LispExpr::List(_)) {
                        // It's a column vector
                        list[0].clone()
                    } else {
                        // It's a row vector
                        LispExpr::List(list.clone())
                    }
                },
                _ => return Err(LispErr::new(&format!("Expected a list, got: {}", args[0]))),
            };

            let v2 = match &args[1] {
                LispExpr::List(list) => {
                    if list.len() == 1 && matches!(list[0], LispExpr::List(_)) {
                        // It's a column vector
                        list[0].clone()
                    } else {
                        // It's a row vector
                        LispExpr::List(list.clone())
                    }
                },
                _ => return Err(LispErr::new(&format!("Expected a list, got: {}", args[1]))),
            };

            unimplemented!("TODO")
        }));

        env.insert("integral".to_string(), LispExpr::Function(|args, _| {
            dbg!(&args);
            unimplemented!()
        }));

        env
    }

}

