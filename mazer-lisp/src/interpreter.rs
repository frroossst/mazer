use std::collections::BTreeMap;

use mazer_atog::Atog;
use mazer_types::{Environment, LispAST, LispError};
use strsim::levenshtein;
use unicode_segmentation::UnicodeSegmentation;

/// Maximum edit distance for a "did you mean" suggestion to be offered.
const SUGGESTION_THRESHOLD: usize = 2;

pub struct Interpreter {
    fragments: BTreeMap<String, LispAST>,
    env: Environment,
}

impl Interpreter {
    pub fn new(fragments: BTreeMap<String, LispAST>, env: Environment) -> Self {
        Self { fragments, env }
    }

    pub fn results(&self) -> &BTreeMap<String, LispAST> {
        &self.fragments
    }

    pub fn run(&mut self) -> Result<LispAST, LispError> {
        let mut result = LispAST::Bool(false);

        for (name, fragment) in self.fragments.clone() {
            result = self.eval(fragment)?;
            // Update the fragments map with the evaluated result
            self.fragments.insert(name, result.clone());
            // dbg!(&result);
        }

        Ok(result)
    }

    /// Build an unbound-symbol error, attaching a "did you mean" suggestion when a
    /// close match exists among the bindings or the symbol table.
    fn unbound(&self, name: &str) -> LispError {
        self.suggest(name).map_or_else(
            || LispError::UnboundSymbol { name: name.to_string() },
            |suggestion| LispError::UnboundSymbolDidYouMean {
                name: name.to_string(),
                suggestion,
            },
        )
    }

    /// Find the closest known name (an environment binding or a symbol-table
    /// entry) within [`SUGGESTION_THRESHOLD`] edits of `name`.
    fn suggest(&self, name: &str) -> Option<String> {
        let candidates = self
            .env
            .bindings
            .keys()
            .map(String::as_str)
            .chain(Atog::iter().map(|(k, _)| *k));

        let mut best: Option<(usize, String)> = None;
        for cand in candidates {
            let d = levenshtein(name, cand);
            if d <= SUGGESTION_THRESHOLD && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                best = Some((d, cand.to_string()));
            }
        }
        best.map(|(_, c)| c)
    }

    pub fn eval(&mut self, expr: LispAST) -> Result<LispAST, LispError> {
        match expr {
            LispAST::Error(e) => Err(LispError::Message(e)),
            LispAST::Number(_)
            | LispAST::Bool(_)
            | LispAST::String(_)
            | LispAST::NativeFunc(_)
            | LispAST::UserFunc { .. } => Ok(expr),

            LispAST::Symbol(ref s) => self.env.get(s).cloned().ok_or_else(|| self.unbound(s)),

            LispAST::List(ref exprs) if exprs.is_empty() => Ok(expr),

            LispAST::List(exprs) => {
                // Handle special forms
                if let LispAST::Symbol(ref s) = exprs[0] {
                    match s.as_str() {
                        "define" => return self.eval_define(&exprs[1..]),
                        "defunc" => return self.eval_defunc(&exprs[1..]),
                        "if" => return self.eval_if(&exprs[1..]),
                        "begin" => return self.eval_begin(&exprs[1..]),
                        "quote" => {
                            return exprs.get(1).cloned().ok_or_else(|| LispError::Arity {
                                form: "quote".to_string(),
                                expected: "1".to_string(),
                                got: exprs.len().saturating_sub(1),
                            });
                        }
                        "string" => return self.eval_string(&exprs[1..]),
                        _ => {}
                    }
                }

                // Function application - evaluate function and arguments
                let func = self.eval(exprs[0].clone())?;
                let args: Result<Vec<_>, _> =
                    exprs[1..].iter().map(|e| self.eval(e.clone())).collect();
                let args = args?;

                self.apply(func, args)
            }

            // Application is lazy - args aren't evaluated yet
            LispAST::Application { name, args } => {
                let func = self
                    .env
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| self.unbound(&name))?;

                // Evaluate args before passing to function
                let evaled_args: Result<Vec<_>, _> =
                    args.iter().map(|e| self.eval(e.clone())).collect();
                let evaled_args = evaled_args?;

                self.apply(func, evaled_args)
            }
        }
    }

    fn apply(&mut self, func: LispAST, args: Vec<LispAST>) -> Result<LispAST, LispError> {
        match func {
            LispAST::NativeFunc(f) => f(&args),
            LispAST::UserFunc { params, body } => {
                if params.len() != args.len() {
                    return Err(LispError::Arity {
                        form: "function".to_string(),
                        expected: params.len().to_string(),
                        got: args.len(),
                    });
                }

                // Create a new scope with parameters bound to arguments
                let mut saved_bindings = BTreeMap::new();
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
            other => Err(LispError::NotAFunction {
                value_type: other.type_name().to_string(),
            }),
        }
    }

    fn eval_define(&mut self, args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.len() != 2 {
            return Err(LispError::Arity {
                form: "define".to_string(),
                expected: "2".to_string(),
                got: args.len(),
            });
        }

        let name = match &args[0] {
            LispAST::Symbol(s) => s.clone(),
            other => {
                return Err(LispError::TypeMismatch {
                    form: "define".to_string(),
                    expected: "Symbol".to_string(),
                    got: other.type_name().to_string(),
                });
            }
        };

        let value = self.eval(args[1].clone())?;
        self.env.set(name, value.clone());
        Ok(value)
    }

    fn eval_defunc(&mut self, args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.len() != 3 {
            return Err(LispError::Arity {
                form: "defunc".to_string(),
                expected: "3 (name, (params...), body)".to_string(),
                got: args.len(),
            });
        }

        let name = match &args[0] {
            LispAST::Symbol(s) => s.clone(),
            other => {
                return Err(LispError::TypeMismatch {
                    form: "defunc".to_string(),
                    expected: "Symbol".to_string(),
                    got: other.type_name().to_string(),
                });
            }
        };

        let params = match &args[1] {
            LispAST::List(param_list) => param_list
                .iter()
                .map(|p| match p {
                    LispAST::Symbol(s) => Ok(s.clone()),
                    other => Err(LispError::TypeMismatch {
                        form: "defunc parameter".to_string(),
                        expected: "Symbol".to_string(),
                        got: other.type_name().to_string(),
                    }),
                })
                .collect::<Result<Vec<_>, _>>()?,
            other => {
                return Err(LispError::TypeMismatch {
                    form: "defunc".to_string(),
                    expected: "List (parameter list)".to_string(),
                    got: other.type_name().to_string(),
                });
            }
        };

        let body = args[2].clone();

        let user_func = LispAST::UserFunc {
            params,
            body: Box::new(body),
        };

        self.env.set(name, user_func.clone());
        Ok(user_func)
    }

    fn eval_if(&mut self, args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.len() != 3 {
            return Err(LispError::Arity {
                form: "if".to_string(),
                expected: "3".to_string(),
                got: args.len(),
            });
        }

        let cond = self.eval(args[0].clone())?;
        match cond {
            LispAST::Bool(true) => self.eval(args[1].clone()),
            LispAST::Bool(false) => self.eval(args[2].clone()),
            other => Err(LispError::TypeMismatch {
                form: "if condition".to_string(),
                expected: "Bool".to_string(),
                got: other.type_name().to_string(),
            }),
        }
    }

    fn eval_begin(&mut self, args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.is_empty() {
            return Err(LispError::Arity {
                form: "begin".to_string(),
                expected: "at least 1".to_string(),
                got: 0,
            });
        }
        let mut result = LispAST::Bool(false);
        for expr in args {
            result = self.eval(expr.clone())?;
        }
        Ok(result)
    }

    fn eval_string(&mut self, args: &[LispAST]) -> Result<LispAST, LispError> {
        if args.len() != 1 {
            return Err(LispError::Arity {
                form: "string".to_string(),
                expected: "1".to_string(),
                got: args.len(),
            });
        }

        // Evaluate the argument (e.g., to handle (quote ...) or other expressions)
        // but if it's a simple symbol, don't fail on unbound
        let value = match &args[0] {
            LispAST::Symbol(_) | LispAST::Number(_) | LispAST::Bool(_) | LispAST::String(_) => {
                // Simple literals - don't evaluate
                args[0].clone()
            }
            _ => {
                // Complex expressions like (quote ...) - evaluate them
                self.eval(args[0].clone())?
            }
        };

        let string_repr = match &value {
            LispAST::String(s) => s.clone(),
            LispAST::Symbol(s) => s.clone(),
            LispAST::Number(n) => n.to_string(),
            LispAST::Bool(b) => b.to_string(),
            LispAST::List(items) => {
                // Convert list to string representation using graphemes
                let parts: Vec<String> = items
                    .iter()
                    .map(|item| match item {
                        LispAST::String(s) => s.clone(),
                        LispAST::Symbol(s) => s.clone(),
                        LispAST::Number(n) => n.to_string(),
                        LispAST::Bool(b) => b.to_string(),
                        _ => format!("{:?}", item),
                    })
                    .collect();
                parts.join("")
            }
            _ => format!("{:?}", value),
        };

        // Validate UTF-8 by collecting graphemes
        let graphemes: Vec<&str> = string_repr.graphemes(true).collect();
        let validated = graphemes.join("");

        Ok(LispAST::String(validated))
    }

    pub fn env(&self) -> &Environment {
        &self.env
    }
}
