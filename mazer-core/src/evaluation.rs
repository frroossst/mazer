use crate::{interpreter::{Environment, Interpreter}, parser::{LispErr, LispExpr}};


impl Interpreter {
    pub fn eval_expr(expr: &LispExpr, env: &mut Environment) -> Result<LispExpr, LispErr> {
        match expr {
            // Self-evaluating expressions
            LispExpr::Number(_) | 
            LispExpr::String(_) | 
            LispExpr::Boolean(_) | 
            LispExpr::Nil => Ok(expr.clone()),

            LispExpr::Function(_) => {
                Ok(expr.clone())
            }
            
            // Symbol lookup
            LispExpr::Symbol(s) => {
                if let Some(value) = env.get(s) {
                    Ok(value.clone())
                } else {
                    Err(LispErr::new(&format!("Symbol {} not found", s)))
                }
            },
            
            // List evaluation (function call or special form)
            LispExpr::List(list) => {
                if list.is_empty() {
                    return Ok(LispExpr::Nil);
                }
                
                // Check for special forms first
                if let LispExpr::Symbol(op) = &list[0] {
                    match op.as_str() {
                        "quote" => {
                            if list.len() != 2 {
                                return Err(LispErr::new("quote requires exactly one argument"));
                            }
                            return Ok(list[1].clone());
                        },
                        
                        "if" => {
                            if list.len() < 3 || list.len() > 4 {
                                return Err(LispErr::new("if requires 2 or 3 arguments"));
                            }
                            
                            let condition = Interpreter::eval_expr(&list[1], env)?;
                            match condition {
                                LispExpr::Boolean(false) | LispExpr::Nil => {
                                    if list.len() == 4 {
                                        Interpreter::eval_expr(&list[3], env)
                                    } else {
                                        Ok(LispExpr::Nil)
                                    }
                                },
                                _ => Interpreter::eval_expr(&list[2], env),
                            }
                        },
                        
                        "define" => {
                            if list.len() != 3 {
                                return Err(LispErr::new("define requires exactly two arguments"));
                            }
                            
                            if let LispExpr::Symbol(name) = &list[1] {
                                let value = Interpreter::eval_expr(&list[2], env)?;
                                env.insert(name.clone(), value.clone());
                                Ok(value)
                            } else {
                                Err(LispErr::new("First argument to define must be a symbol"))
                            }
                        },
                        
                        "lambda" => {
                            if list.len() < 3 {
                                return Err(LispErr::new("lambda requires at least 2 arguments"));
                            }
                            
                            Err(LispErr::new("Lambda functions not implemented yet"))
                        },
                        
                        _ => {
                            // Regular function call
                            let evaluated_op = Interpreter::eval_expr(&list[0], env)?;
                            
                            match evaluated_op {
                                LispExpr::Function(func) => {
                                    let mut evaluated_args = Vec::new();
                                    for arg in &list[1..] {
                                        evaluated_args.push(Interpreter::eval_expr(arg, env)?);
                                    }
                                    func(evaluated_args, env)
                                },
                                _ => Err(LispErr::new(&format!("Expected function, got: {}", evaluated_op))),
                            }
                        }
                    }
                } else {
                    // First element is not a symbol
                    dbg!(&list[0]);
                    dbg!(&env);
                    let evaluated_op = Interpreter::eval_expr(&list[0], env)?;
                    match evaluated_op {
                        LispExpr::Function(func) => {
                            let mut evaluated_args = Vec::new();
                            for arg in &list[1..] {
                                evaluated_args.push(Interpreter::eval_expr(arg, env)?);
                            }
                            func(evaluated_args, env)
                        },
                        _ => Err(LispErr::new(&format!("Expected function, got: {}", evaluated_op))),
                    }
                }
            }
        }
        }
}
