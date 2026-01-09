use std::{collections::HashMap, io::{Write, stderr}};

use fastnum::D512;
use mazer_types::LispAST;

// prlude functions are functions that are valid lisp code that is parsed 
// and added to the environment at startup
pub struct Prelude;

impl Prelude {

    pub fn new() -> HashMap<String, &'static str> {
        let mut prelude = HashMap::new();
        
        prelude.insert("not".into(), Self::not());
        prelude.insert("or".into(), Self::or());
        prelude.insert("and".into(), Self::and());
        prelude.insert("xor".into(), Self::xor());
        
        prelude
    }

    #[inline]
    fn not() -> &'static str {
        r#"(defunc not (x)
                (if x
                    false
                    true)
            )
        "#
    }

    #[inline]
    fn or() -> &'static str {
        r#"(defunc or (a b)
                (if a
                    true
                    b)
            )
        "#
    }

    #[inline]
    fn and() -> &'static str {
        r#"(defunc and (a b)
                (if a
                    b
                    false)
            )
        "#
    }

    #[inline]
    fn xor() -> &'static str {
        r#"(defunc xor (a b)
                (if a
                    (not b)
                    b)
            )
        "#
    }

}


#[inline]
fn check_if_all_args_numbers(args: &[LispAST]) -> bool {
    args.iter().all(|a| matches!(a, LispAST::Number(_)))
}

pub struct Native;

impl Native {

    // type infer runtime
    pub fn reflect(args: &[LispAST]) -> Result<LispAST, String> {
        if args.len() != 1 {
            return Err("reflect requires exactly 1 argument".to_string());
        }
        
        let type_str = match &args[0] {
            LispAST::Number(_) => "Number",
            LispAST::Bool(_) => "Bool",
            LispAST::String(_) => "String",
            LispAST::Symbol(_) => "Symbol",
            LispAST::List(_) => "List",
            LispAST::NativeFunc(_) => "NativeFunc",
            LispAST::UserFunc { .. } => "UserFunc",
            LispAST::Application { .. } => "Application",
            LispAST::Error(_) => "Error",
        };
        
        Ok(LispAST::Symbol(type_str.to_string()))
    }


    pub fn print(args: &[LispAST]) -> Result<LispAST, String> {
        for arg in args {
            match arg {
                LispAST::Number(n) => eprint!("{}", n),
                LispAST::Bool(b) => eprint!("{}", b),
                LispAST::String(s) => eprint!("{}", s),
                LispAST::Symbol(s) => eprint!("{}", s),
                LispAST::List(_) => eprint!("{:?}", arg),
                _ => eprint!("{:?}", arg),
            }
        }
        eprintln!();

        stderr().flush().ok();

        Ok(LispAST::Bool(true))
    }

    pub fn debug(args: &[LispAST]) -> Result<LispAST, String> {
        for arg in args {
            eprintln!("{:?}", arg);
        }
        stderr().flush().ok();

        Ok(LispAST::Bool(true))
    }

    pub fn add(args: &[LispAST]) -> Result<LispAST, String> {
        if !check_if_all_args_numbers(args) {
            return Err("All arguments to add must be numbers".to_string());
        }

        if args.is_empty() {
            return Err("+ requires at least 1 argument".to_string());
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

        let result = args.iter().fold(D512::from(0), |acc, x| {
            if let LispAST::Number(n) = x {
                if acc == D512::from(0) {
                    *n
                } else {
                    acc - *n
                }
            } else {
                acc
            }
        });
        
        Ok(LispAST::Number(result))
    }

    pub fn mul(args: &[LispAST]) -> Result<LispAST, String> {
        if !check_if_all_args_numbers(args) {
            return Err("All arguments to mul must be numbers".to_string());
        }
        
        if args.is_empty() {
            return Err("* requires at least 1 argument".to_string());
        }

        let product = args.iter().fold(D512::from(1), |acc, x| {
            if let LispAST::Number(n) = x {
                acc * *n
            } else {
                acc
            }
        });
        
        Ok(LispAST::Number(product))
    }

    pub fn div(args: &[LispAST]) -> Result<LispAST, String> {
        if !check_if_all_args_numbers(args) {
            return Err("All arguments to div must be numbers".to_string());
        }
        
        if args.is_empty() {
            return Err("/ requires at least 1 argument".to_string());
        }

        let result = args.iter().fold(None, |acc: Option<D512>, x| {
            if let LispAST::Number(n) = x {
                match acc {
                    None => Some(*n),
                    Some(a) => Some(a / *n),
                }
            } else {
                acc
            }
        });
        
        match result {
            Some(res) => Ok(LispAST::Number(res)),
            None => Err("Division failed".to_string()),
        }
    }
}
