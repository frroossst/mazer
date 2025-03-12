use colored::Colorize;
use regex::Regex;
use std::fmt;

use crate::interpreter::Environment;

pub struct LispErr {
    message: String,
}

impl fmt::Debug for LispErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", "[ERROR]".red().bold(), self.message)
    }
}

impl fmt::Display for LispErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", "[ERRPR]".red().bold(), self.message)
    }
}

impl From<LispErr> for String {
    fn from(err: LispErr) -> Self {
        err.message
    }
}

impl LispErr {
    pub fn new(message: &str) -> Self {
        LispErr {
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LispExpr {
    Number(f64),
    String(String),
    Symbol(String),
    Boolean(bool),
    List(Vec<LispExpr>),
    Nil,
    Function(fn(Vec<LispExpr>, &mut Environment) -> Result<LispExpr, LispErr>),
}

impl fmt::Display for LispExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LispExpr::Number(n) => write!(f, "{}", n),
            LispExpr::String(s) => write!(f, "\"{}\"", s),
            LispExpr::Symbol(s) => write!(f, "{}", s),
            LispExpr::Boolean(b) => write!(f, "{}", b),
            LispExpr::Nil => write!(f, "nil"),
            LispExpr::List(list) => {
                write!(f, "(")?;
                for (i, expr) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
            LispExpr::Function(_) => write!(f, "<function>"),
        }
    }
}

#[macro_export]
macro_rules! wrap_mathml {
    ($content:expr) => {
        format!(
            "<math xmlns=\"http://www.w3.org/1998/Math/MathML\">{}</math>",
            $content
        )
    };
}

#[derive(Debug)]
pub struct MathML(String);

impl MathML {
    pub fn new(src: String) -> Self {
        MathML(src)
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for MathML {
    fn from(content: String) -> Self {
        MathML(wrap_mathml!(content))
    }
}

impl From<&LispExpr> for MathML {
    fn from(expr: &LispExpr) -> Self {
        let expr = expr.clone();
        match expr {
            LispExpr::Function(_) => MathML::new("<mrow>Error: function in expression</mrow>".to_string()),
            LispExpr::Number(n) => format!("<mn>{}</mn>", n).into(),
            LispExpr::Symbol(s) => format!("<mi>{}</mi>", s).into(),
            LispExpr::String(s) => format!("<mtext>{}</mtext>", s).into(),
            LispExpr::Boolean(b) => format!("<mn>{}</mn>", b).into(),
            LispExpr::Nil => "<mi>nil</mi>".to_string().into(),
            LispExpr::List(list) => {
                if list.is_empty() {
                    return MathML::new(String::new());
                }

                if let LispExpr::Symbol(operator) = &list[0] {
                    let args = &list[1..];
                    match operator.as_str() {
                        "+" => MathML::addition(args),
                        "-" => MathML::subtraction(args),
                        "*" => MathML::multiplication(args),
                        "/" => MathML::division(args),
                        "matrix"=> MathML::matrix(args),
                        _ => unimplemented!("From<&LispExpr> for MathML: operator `{}` not implemented", operator),
                    }
                } else {
                    return MathML::new("<mrow>Error: first element of a list must be a symbol</mrow>".to_string());
                }
            }
        }
    }
}


pub struct Parser {
    src: String,
    tokens: Vec<String>,
    ast: Vec<LispExpr>,
}

impl Parser {
    pub fn new(src: String) -> Self {
        let token = Parser::tokenize(&src);

        Parser {
            src: src,
            tokens: token,
            ast: Vec::new(),
        }
    }

    /// This is used when a lisp expression is within a fmt or eval
    /// call. We need to wrap it in parentheses to ensure it's
    /// treated as a single expression. Else will simply get back
    /// the first token or equivalent.
    /// Caller's responsibility to ensure the string is a valid
    /// Caller must call wrap_parens before the .parse() method.
    /// This also prevents imho the rather ugly redundant and repeated
    /// parens like so: fmt((expr)) when you can simply write fmt(expr)
    /// NOTE: does not check for balances parenthesis
    pub fn wrap_parens_safely(src: String) -> String {
        let src = src.trim();
        if src.starts_with('(') && src.ends_with(')') {
            src.to_string()
        } else {
            format!("({})", src)
        }
    }

    pub fn append_tokens(&mut self, src: String) {
        let token = Parser::tokenize(&src);
        self.tokens.extend(token);
    }

    /// This regular expression is used for tokenizing a Lisp-like language.
    /// 
    /// It matches and captures different types of tokens, including:
    /// 
    /// - **Whitespace and commas** (`[\s,]*`)  
    ///   - These are ignored as separators.
    /// 
    /// - **Special symbols** (`~@|[\[\]{}()'`~^@]`)  
    ///   - Matches Lisp syntax elements like `(`, `)`, `[`, `]`, `{`, `}`, `'`, `` ` ``, `~`, `@`, `^`, and `~@`.
    /// 
    /// - **String literals** (`"(?:\\.|[^\\"])*"?`)  
    ///   - Matches double-quoted strings, allowing for escaped characters (e.g., `"hello"`, `"escaped \" quote"`).
    ///   - The trailing `"?` allows capturing an incomplete string (e.g., `"unterminated`).
    /// 
    /// - **Comments** (`;.*`)  
    ///   - Matches Lisp-style comments starting with `;` and continuing to the end of the line.
    /// 
    /// - **Identifiers and other tokens** (`[^\s\[\]{}('"`,;)]*`)  
    ///   - Matches symbols, numbers, and variable names, ensuring they don't include special characters.
    pub fn tokenize(src: &str) -> Vec<String> {
        let regex =
            Regex::new(r#"[\s,]*(~@|[\[\]{}()'`~^@]|"(?:\\.|[^\\"])*"?|;.*|[^\s\[\]{}('"`,;)]*)"#)
                .expect("regex should always compile");
        let mut results = Vec::with_capacity(1024);

        for capture in regex.captures_iter(src) {
            let token = capture.get(1).unwrap().as_str();
            if token.is_empty() || token.starts_with(';') {
                continue; // skip empty tokens and comments
            }
            results.push(token.to_string());
        }

        results
    }

    pub fn parse(&mut self) -> LispExpr {
        let tokens = Parser::tokenize(&self.tokens.join(" "));
        let (expr, _) = Parser::parse_tokens(&tokens, 0);
        self.ast.push(expr.clone());
        expr
    }

    pub fn ast(&self) -> Vec<LispExpr> {
        self.ast.clone()
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    fn parse_tokens(tokens: &[String], start_index: usize) -> (LispExpr, usize) {
        if start_index >= tokens.len() {
            return (LispExpr::Nil, start_index);
        }

        let token = &tokens[start_index];

        if token == "(" {
            let mut list = Vec::new();
            let mut idx = start_index + 1;

            while idx < tokens.len() && tokens[idx] != ")" {
                let (expr, next_idx) = Parser::parse_tokens(tokens, idx);
                list.push(expr);
                idx = next_idx;
            }

            // Skip the closing parenthesis
            idx = if idx < tokens.len() { idx + 1 } else { idx };

            return (LispExpr::List(list), idx);
        } else {
            (Parser::parse_atom(token), start_index + 1)
        }
    }

    fn parse_atom(token: &str) -> LispExpr {
        // Handle strings
        if token.starts_with('"') {
            let content = if token.ends_with('"') && token.len() > 1 {
                &token[1..token.len() - 1]
            } else {
                &token[1..]
            };
            return LispExpr::String(content.to_string());
        }

        // Handle numbers
        if let Ok(num) = token.parse::<f64>() {
            return LispExpr::Number(num);
        }

        // Handle booleans and nil
        match token {
            "true" => return LispExpr::Boolean(true),
            "false" => return LispExpr::Boolean(false),
            "nil" => return LispExpr::Nil,
            _ => {}
        }

        // Otherwise it's a symbol
        LispExpr::Symbol(token.to_string())
    }
}

