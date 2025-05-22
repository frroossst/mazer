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

        env.insert("dot".to_string(), LispExpr::Function(|args, _| {
            if args.len() != 2 {
                return Err(LispErr::new("dot requires exactly two arguments"));
            }
            
            let vec_a = if let LispExpr::List(list) = &args[0] {
                Matrix::list_to_vector(list)?
            } else {
                return Err(LispErr::new("dot requires vector arguments (lists)"));
            };
            
            let vec_b = if let LispExpr::List(list) = &args[1] {
                Matrix::list_to_vector(list)?
            } else {
                return Err(LispErr::new("dot requires vector arguments (lists)"));
            };
            
            if vec_a.len() != vec_b.len() {
                return Err(LispErr::new("Vectors must have same length for dot product"));
            }
            
            let result: f64 = vec_a.iter().zip(vec_b.iter()).map(|(a, b)| a * b).sum();
            Ok(LispExpr::Number(result))
        }));


        env.insert("integral".to_string(), LispExpr::Function(|args, _| {
            dbg!(&args);
            unimplemented!()
        }));

        env
    }

}

