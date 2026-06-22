use std::{
    collections::HashMap,
    io::{Write, stderr},
};

use fastnum::D512;
use mazer_types::{LispAST, LispError};

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

/// Ensure every argument is a `Number`, otherwise report the first offender's
/// type against `form`.
fn require_all_numbers(form: &str, args: &[LispAST]) -> Result<(), LispError> {
    for a in args {
        if !matches!(a, LispAST::Number(_)) {
            return Err(LispError::TypeMismatch {
                form: form.to_string(),
                expected: "Number".to_string(),
                got: a.type_name().to_string(),
            });
        }
    }
    Ok(())
}

pub struct Native;

impl Native {
    // type infer runtime
    pub fn reflect(args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.len() != 1 {
            return Err(LispError::Arity {
                form: "reflect".to_string(),
                expected: "1".to_string(),
                got: args.len(),
            });
        }

        Ok(LispAST::Symbol(args[0].type_name().to_string()))
    }

    pub fn print(args: &[LispAST]) -> Result<LispAST, LispError> {
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

    pub fn debug(args: &[LispAST]) -> Result<LispAST, LispError> {
        for arg in args {
            eprintln!("{:?}", arg);
        }
        stderr().flush().ok();

        Ok(LispAST::Bool(true))
    }

    pub fn add(args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.is_empty() {
            return Err(LispError::Arity {
                form: "add".to_string(),
                expected: "at least 1".to_string(),
                got: 0,
            });
        }
        require_all_numbers("add", args)?;

        let sum = args.iter().fold(D512::from(0), |acc, x| {
            if let LispAST::Number(n) = x {
                acc + *n
            } else {
                acc
            }
        });

        Ok(LispAST::Number(sum))
    }

    pub fn sub(args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.is_empty() {
            return Err(LispError::Arity {
                form: "sub".to_string(),
                expected: "at least 1".to_string(),
                got: 0,
            });
        }
        require_all_numbers("sub", args)?;

        // For single argument, return negation
        if args.len() == 1 {
            if let LispAST::Number(n) = &args[0] {
                return Ok(LispAST::Number(-*n));
            }
        }

        // For multiple arguments, subtract sequentially from first
        let first = if let LispAST::Number(n) = &args[0] {
            *n
        } else {
            return Err(LispError::TypeMismatch {
                form: "sub".to_string(),
                expected: "Number".to_string(),
                got: args[0].type_name().to_string(),
            });
        };

        let result = args[1..].iter().fold(first, |acc, x| {
            if let LispAST::Number(n) = x {
                acc - *n
            } else {
                acc
            }
        });

        Ok(LispAST::Number(result))
    }

    pub fn mul(args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.is_empty() {
            return Err(LispError::Arity {
                form: "mul".to_string(),
                expected: "at least 1".to_string(),
                got: 0,
            });
        }
        require_all_numbers("mul", args)?;

        let product = args.iter().fold(D512::from(1), |acc, x| {
            if let LispAST::Number(n) = x {
                acc * *n
            } else {
                acc
            }
        });

        Ok(LispAST::Number(product))
    }

    pub fn div(args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.is_empty() {
            return Err(LispError::Arity {
                form: "div".to_string(),
                expected: "at least 1".to_string(),
                got: 0,
            });
        }
        require_all_numbers("div", args)?;

        // For single argument, return reciprocal (1/x)
        if args.len() == 1 {
            if let LispAST::Number(n) = &args[0] {
                if *n == D512::from(0) {
                    return Err(LispError::DivisionByZero);
                }
                return Ok(LispAST::Number(D512::from(1) / *n));
            }
        }

        // For multiple arguments, divide sequentially from first
        let first = if let LispAST::Number(n) = &args[0] {
            *n
        } else {
            return Err(LispError::TypeMismatch {
                form: "div".to_string(),
                expected: "Number".to_string(),
                got: args[0].type_name().to_string(),
            });
        };

        let result = args[1..].iter().try_fold(first, |acc, x| {
            if let LispAST::Number(n) = x {
                if *n == D512::from(0) {
                    return Err(LispError::DivisionByZero);
                }
                Ok(acc / *n)
            } else {
                Ok(acc)
            }
        })?;

        Ok(LispAST::Number(result))
    }
}
