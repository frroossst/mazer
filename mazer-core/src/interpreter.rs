use std::{collections::HashMap, fmt::Debug};

use crate::parser::{LispErr, LispExpr};

pub type Environment = HashMap<String, LispExpr>;

#[derive(Debug)]
pub struct Interpreter {
    env: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Interpreter::stdlib(),
        }
    }

    /// Creates a standard environment with basic functions and constants.
    /// Only the most basic primitives and functions are included.
    pub fn stdenv() -> Environment {
        let mut env = HashMap::new();

        // constants
        env.insert("pi".to_string(), LispExpr::Number(std::f64::consts::PI));
        env.insert("e".to_string(), LispExpr::Number(std::f64::consts::E));

        // arithmetic functions
        env.insert("+".to_string(), LispExpr::Function(|args, _| {
            let sum: f64 = args.iter().map(|arg| {
                if let LispExpr::Number(n) = arg {
                    *n
                } else {
                    0.0
                }
            }).sum();
            Ok(LispExpr::Number(sum))
        }));

        env.insert("-".to_string(), LispExpr::Function(|args, _env| {
            if args.is_empty() {
                return Err(LispErr::new("Subtraction requires at least one argument"));
            }
            
            if let LispExpr::Number(first) = args[0] {
                if args.len() == 1 {
                    // Unary minus
                    return Ok(LispExpr::Number(-first));
                }
                
                let mut result = first;
                for arg in &args[1..] {
                    if let LispExpr::Number(n) = arg {
                        result -= n;
                    } else {
                        return Err(LispErr::new(&format!("Expected number, got: {}", arg)));
                    }
                }
                Ok(LispExpr::Number(result))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {}", args[0])))
            }
        }));
        
        env.insert("*".to_string(), LispExpr::Function(|args, _env| {
            let mut result = 1.0;
            for arg in args {
                if let LispExpr::Number(n) = arg {
                    result *= n;
                } else {
                    return Err(LispErr::new(&format!("Expected number, got: {}", arg)));
                }
            }
            Ok(LispExpr::Number(result))
        }));
        
        env.insert("/".to_string(), LispExpr::Function(|args, _env| {
            if args.is_empty() {
                return Err(LispErr::new("/ requires at least one argument"));
            }
            
            if let LispExpr::Number(first) = args[0] {
                if args.len() == 1 {
                    // Reciprocal
                    if first == 0.0 {
                        return Err(LispErr::new("Division by zero"));
                    }
                    return Ok(LispExpr::Number(1.0 / first));
                }
                
                let mut result = first;
                for arg in &args[1..] {
                    if let LispExpr::Number(n) = arg {
                        if *n == 0.0 {
                            return Err(LispErr::new("Division by zero"));
                        }
                        result /= n;
                    } else {
                        return Err(LispErr::new(&format!("Expected number, got: {}", arg)));
                    }
                }
                Ok(LispExpr::Number(result))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {}", args[0])))
            }
        }));
        
        // Comparison operations
        env.insert("=".to_string(), LispExpr::Function(|args, _env| {
            if args.len() < 2 {
                return Err(LispErr::new("= requires at least two arguments"));
            }
            
            if let LispExpr::Number(first) = args[0] {
                for arg in &args[1..] {
                    if let LispExpr::Number(n) = arg {
                        if first != *n {
                            return Ok(LispExpr::Boolean(false));
                        }
                    } else {
                        return Err(LispErr::new(&format!("Expected number, got: {}", arg)));
                    }
                }
                Ok(LispExpr::Boolean(true))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {}", args[0])))
            }
        }));
        
        env.insert(">".to_string(), LispExpr::Function(|args, _env| {
            if args.len() != 2 {
                return Err(LispErr::new("> requires exactly two arguments"));
            }
            
            if let (LispExpr::Number(a), LispExpr::Number(b)) = (&args[0], &args[1]) {
                Ok(LispExpr::Boolean(a > b))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {} and {}", args[0], args[1])))
            }
        }));
        
        env.insert("<".to_string(), LispExpr::Function(|args, _env| {
            if args.len() != 2 {
                return Err(LispErr::new("< requires exactly two arguments"));
            }
            
            if let (LispExpr::Number(a), LispExpr::Number(b)) = (&args[0], &args[1]) {
                Ok(LispExpr::Boolean(a < b))
            } else {
                Err(LispErr::new(&format!("Expected number, got: {} and {}", args[0], args[1])))
            }
        }));
        
        env
    }

    pub fn environment(&self) -> Environment {
        self.env.clone()
    }

    pub fn get_symbol(&self, symbol: String) -> Option<LispExpr> {
        self.env.get(&symbol).cloned()
    }

    pub fn set_symbol(&mut self, symbol: String, definition: LispExpr) {
        self.env.insert(symbol, definition);
    }

    pub fn eval(&self, symbol: String) -> Result<LispExpr, LispErr> {
        let _expr = self.get_symbol(symbol.clone()).ok_or(LispErr::new(&format!("Symbol {} not found", symbol)))?;
        unimplemented!("interpreter::eval")
    }
}
