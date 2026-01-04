use std::collections::HashMap;

use mazer_types::LispAST;

use crate::environment::Environment;

pub struct Interpreter {
    fragments: HashMap<String, LispAST>,
    env: Environment,
}

impl Interpreter {
    pub fn new(fragments: HashMap<String, LispAST>, env: Environment) -> Self {
        Self { fragments, env }
    }

    pub fn results(&self) -> &HashMap<String, LispAST> {
        &self.fragments
    }
    
    pub fn run(&mut self) -> Result<LispAST, String> {
        let mut result = LispAST::Bool(false);
        
        for (_name, fragment) in self.fragments.clone() {
            result = self.eval(fragment)?;
            // dbg!(&result);
        }
        
        Ok(result)
    }
    
    pub fn eval(&mut self, expr: LispAST) -> Result<LispAST, String> {
        match expr {
            LispAST::Error(e) => Err(e),
            LispAST::Number(_) | LispAST::Bool(_) | LispAST::NativeFunc(_) | LispAST::UserFunc { .. } => Ok(expr),
            
            LispAST::Symbol(ref s) => {
                self.env.get(s)
                    .cloned()
                    .ok_or_else(|| format!("Unbound symbol: {}", s))
            }
            
            LispAST::List(ref exprs) if exprs.is_empty() => Ok(expr),
            
            LispAST::List(exprs) => {
                // Handle special forms
                if let LispAST::Symbol(ref s) = exprs[0] {
                    match s.as_str() {
                        "define" => return self.eval_define(&exprs[1..]),
                        "defunc" => return self.eval_defunc(&exprs[1..]),
                        "if" => return self.eval_if(&exprs[1..]),
                        "quote" => return exprs.get(1).cloned()
                            .ok_or_else(|| "quote requires 1 argument".to_string()),
                        _ => {}
                    }
                }
                
                // Function application - evaluate function and arguments
                let func = self.eval(exprs[0].clone())?;
                let args: Result<Vec<_>, _> = exprs[1..].iter()
                    .map(|e| self.eval(e.clone()))
                    .collect();
                let args = args?;
                
                self.apply(func, args)
            }
            
            // Application is lazy - args aren't evaluated yet
            LispAST::Application { name, args } => {
                let func = self.env.get(&name)
                    .cloned()
                    .ok_or_else(|| format!("Unbound function: {}", name))?;
                
                // Evaluate args before passing to function
                let evaled_args: Result<Vec<_>, _> = args.iter()
                    .map(|e| self.eval(e.clone()))
                    .collect();
                let evaled_args = evaled_args?;
                
                self.apply(func, evaled_args)
            }
        }
    }
    
    fn apply(&mut self, func: LispAST, args: Vec<LispAST>) -> Result<LispAST, String> {
        match func {
            LispAST::NativeFunc(f) => f(&args),
            LispAST::UserFunc { params, body } => {
                if params.len() != args.len() {
                    return Err(format!("Expected {} arguments, got {}", params.len(), args.len()));
                }
                
                // Create a new scope with parameters bound to arguments
                let mut saved_bindings = std::collections::HashMap::new();
                for (param, arg) in params.iter().zip(args.iter()) {
                    if let Some(existing) = self.env.get(param) {
                        saved_bindings.insert(param.clone(), existing.clone());
                    }
                    self.env.set(param.clone(), arg.clone());
                }
                
                // Evaluate the function body
                let result = self.eval((*body).clone());
                
                // Restore the original bindings
                for (param, original) in &saved_bindings {
                    self.env.set(param.clone(), original.clone());
                }
                
                result
            }
            _ => Err(format!("Not a function: {:?}", func)),
        }
    }
    
    fn eval_define(&mut self, args: &[LispAST]) -> Result<LispAST, String> {
        if args.len() != 2 {
            return Err("define requires 2 arguments".to_string());
        }
        
        let name = match &args[0] {
            LispAST::Symbol(s) => s.clone(),
            _ => return Err("define requires symbol as first argument".to_string()),
        };
        
        let value = self.eval(args[1].clone())?;
        self.env.set(name, value.clone());
        Ok(value)
    }
    
    fn eval_defunc(&mut self, args: &[LispAST]) -> Result<LispAST, String> {
        if args.len() != 3 {
            return Err("defunc requires 3 arguments: name, (params...), body".to_string());
        }
        
        let name = match &args[0] {
            LispAST::Symbol(s) => s.clone(),
            _ => return Err("defunc requires symbol as first argument".to_string()),
        };
        
        let params = match &args[1] {
            LispAST::List(param_list) => {
                param_list.iter().map(|p| {
                    match p {
                        LispAST::Symbol(s) => Ok(s.clone()),
                        _ => Err("Parameters must be symbols".to_string()),
                    }
                }).collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err("defunc requires parameter list as second argument".to_string()),
        };
        
        let body = args[2].clone();
        
        let user_func = LispAST::UserFunc {
            params,
            body: Box::new(body),
        };
        
        self.env.set(name, user_func.clone());
        Ok(user_func)
    }
    
    fn eval_if(&mut self, args: &[LispAST]) -> Result<LispAST, String> {
        if args.len() != 3 {
            return Err("if requires 3 arguments".to_string());
        }
        
        let cond = self.eval(args[0].clone())?;
        match cond {
            LispAST::Bool(true) => self.eval(args[1].clone()),
            LispAST::Bool(false) => self.eval(args[2].clone()),
            _ => Err("if condition must be boolean".to_string()),
        }
    }
}
