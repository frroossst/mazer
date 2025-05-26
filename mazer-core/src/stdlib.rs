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
            if args.len() != 2 {
                return Err(LispErr::new("dot requires exactly two arguments"));
            }
            // Flattens a matrix or vector to a Vec<f64>, skipping Symbol("matrix") and unwrapping single-list nesting
            fn flatten(expr: &LispExpr) -> Result<Vec<f64>, LispErr> {
                // Always skip a leading (matrix ...)
                let cur: &LispExpr = match expr {
                    LispExpr::List(list) if matches!(list.get(0), Some(LispExpr::Symbol(sym)) if sym == "matrix") => {
                        // Create a new owned list without the first element
                        let owned = LispExpr::List(list[1..].to_vec());
                        // Recurse on the owned value
                        return flatten(&owned);
                    }
                    _ => expr,
                };
                // Unwrap single nested list (for (matrix (1 2 3)))
                if let LispExpr::List(list) = cur {
                    if list.len() == 1 {
                        if let LispExpr::List(inner) = &list[0] {
                            return flatten(&LispExpr::List(inner.clone()));
                        }
                    }
                    // Row: (matrix 1 2 3)
                    if list.iter().all(|e| matches!(e, LispExpr::Number(_))) {
                        return Matrix::list_to_vector(list);
                    }
                    // Column: (matrix (1) (2) (3))
                    if list.iter().all(|e| matches!(e, LispExpr::List(_))) {
                        return list.iter().map(|e| {
                            if let LispExpr::List(inner) = e {
                                if inner.len() == 1 {
                                    if let LispExpr::Number(n) = inner[0] {
                                        Ok(n)
                                    } else {
                                        Err(LispErr::new("Column vector inner element is not a number"))
                                    }
                                } else {
                                    Err(LispErr::new("Column vector inner list must have exactly one element"))
                                }
                            } else {
                                Err(LispErr::new("Column vector element is not a list"))
                            }
                        }).collect();
                    }
                }
                Err(LispErr::new("Matrix/vector must be a flat list of numbers or a list of single-element lists"))
            }
            let v1 = flatten(&args[0])?;
            let v2 = flatten(&args[1])?;
            if v1.len() != v2.len() {
                return Err(LispErr::new("Vectors must be the same length for dot product"));
            }
            Ok(LispExpr::Number(v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum()))
        }));

        env.insert("integral".to_string(), LispExpr::Function(|args, _| {
            dbg!(&args);
            unimplemented!()
        }));

        env
    }

}

