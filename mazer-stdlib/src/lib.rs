use fastnum::D512;
use mazer_types::LispAST;

fn check_if_all_args_numbers(args: &[LispAST]) -> bool {
    args.iter().all(|a| matches!(a, LispAST::Number(_)))
}

pub struct Native;

impl Native {
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
