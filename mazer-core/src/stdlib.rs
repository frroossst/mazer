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
        env.insert("dot".to_string(), LispExpr::Function(|args, _env| {
            // Accepts two arguments, both should be lists (matrices/vectors)
            if args.len() != 2 {
                return Err(LispErr::new("dot requires exactly two arguments"));
            
            }

            dbg!(&args);

            // Helper to flatten a matrix to a vector of numbers
            fn flatten_matrix(expr: &LispExpr) -> Result<Vec<f64>, LispErr> {
                match expr {
                    LispExpr::List(list) => {
                        // If the first element is Symbol("matrix"), skip it
                        let slice = if let Some(LispExpr::Symbol(sym)) = list.get(0) {
                            if sym == "matrix" {
                                &list[1..]
                            } else {
                                &list[..]
                            }
                        } else {
                            &list[..]
                        };
                        // If it's a row vector: (matrix 1 2 3) => [1,2,3]
                        if slice.iter().all(|e| matches!(e, LispExpr::Number(_))) {
                            Matrix::list_to_vector(slice)
                        }
                        // If it's a column vector: (matrix (1) (2) (3)) => [1,2,3]
                        else if slice.iter().all(|e| matches!(e, LispExpr::List(_))) {
                            if slice.len() == 1 {
                                if let LispExpr::List(inner) = &slice[0] {
                                    flatten_matrix(&LispExpr::List(inner.clone()))
                                } else {
                                    Err(LispErr::new("Single element is not a list"))
                                }
                            } else {
                                let mut result = Vec::new();
                                for e in slice {
                                    if let LispExpr::List(inner) = e {
                                        if inner.len() == 1 {
                                            if let LispExpr::Number(n) = inner[0] {
                                                result.push(n);
                                            } else {
                                                return Err(LispErr::new("Column vector inner element is not a number"));
                                            }
                                        } else {
                                            return Err(LispErr::new("Column vector inner list must have exactly one element"));
                                        }
                                    } else {
                                        return Err(LispErr::new("Column vector element is not a list"));
                                    }
                                }
                                Ok(result)
                            }
                        } else {
                            Err(LispErr::new("Matrix/vector must be a flat list of numbers or a list of single-element lists"))
                        }
                    }
                    _ => Err(LispErr::new("dot expects list arguments")),
                }
            }
            let v1 = flatten_matrix(&args[0])?;
            let v2 = flatten_matrix(&args[1])?;
            if v1.len() != v2.len() {
                return Err(LispErr::new("Vectors must be the same length for dot product"));
            }
            let dot = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum::<f64>();
            Ok(LispExpr::Number(dot))
        }));

        env.insert("integral".to_string(), LispExpr::Function(|args, _| {
            dbg!(&args);
            unimplemented!()
        }));

        env
    }

}

