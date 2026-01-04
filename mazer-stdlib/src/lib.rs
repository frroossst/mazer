use std::collections::HashMap;

use fastnum::D512;
use mazer_types::LispAST;

// prlude functions are functions that are valid lisp code that is parsed 
// and added to the environment at startup
pub struct Prelude;

impl Prelude {

    pub fn new() -> HashMap<String, &'static str> {
        let mut prelude = HashMap::new();
        
        prelude.insert("not".into(), Self::not());
        
        prelude
    }

    fn not() -> &'static str {
        r#"
            (defunc not (x)
                (if x
                    false
                    true)
                )
            )
        "#
    }
    
}


fn check_if_all_args_numbers(args: &[LispAST]) -> bool {
    args.iter().all(|a| matches!(a, LispAST::Number(_)))
}

pub struct Native;

impl Native {

    pub fn print(args: &[LispAST]) -> Result<LispAST, String> {
        for arg in args {
            match arg {
                LispAST::Number(n) => eprint!("{}", n),
                LispAST::Bool(b) => eprint!("{}", b),
                LispAST::Symbol(s) => eprint!("{}", s),
                LispAST::List(_) => eprint!("{:?}", arg),
                _ => eprint!("{:?}", arg),
            }
        }
        eprintln!();
        Ok(LispAST::Bool(true))
    }

    pub fn debug(args: &[LispAST]) -> Result<LispAST, String> {
        for arg in args {
            eprintln!("{:?}", arg);
        }
        Ok(LispAST::Bool(true))
    }

    pub fn add(args: &[LispAST]) -> Result<LispAST, String> {
        if !check_if_all_args_numbers(args) {
            return Err("All arguments to add must be numbers".to_string());
        }
        
        let sum = args.iter().fold(D512::from(0), |acc, x| {
            if let LispAST::Number(n) = x {
                acc + *n
            } else {
                acc
            }
        });
        
        Ok(LispAST::Number(sum))
    }
    
    pub fn sub(args: &[LispAST]) -> Result<LispAST, String> {
        if !check_if_all_args_numbers(args) {
            return Err("All arguments to sub must be numbers".to_string());
        }
        
        if args.is_empty() {
            return Err("- requires at least 1 argument".to_string());
        }
        
        let first = if let LispAST::Number(n) = &args[0] {
            *n
        } else {
            return Err("Unreachable".to_string());
        };
        
        if args.len() == 1 {
            return Ok(LispAST::Number(-first));
        }
        
        let result = args[1..].iter().fold(first, |acc, x| {
            if let LispAST::Number(n) = x {
                acc - *n
            } else {
                acc
            }
        });
        
        Ok(LispAST::Number(result))
    }
}
