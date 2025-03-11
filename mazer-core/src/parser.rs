use regex::Regex;
use std::fmt;

use crate::interpreter::Environment;

#[derive(Debug, Clone, PartialEq)]
pub enum LispExpr {
    Number(f64),
    String(String),
    Symbol(String),
    Boolean(bool),
    List(Vec<LispExpr>),
    Nil,
    Function(fn(Vec<LispExpr>, &mut Environment) -> Result<LispExpr, String>),
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
            LispExpr::Number(n) => MathML::new(format!("<mn>{}</mn>", n)),
            LispExpr::Symbol(s) => MathML::new(format!("<mi>{}</mi>", s)),
            LispExpr::String(s) => MathML::new(format!("<mtext>{}</mtext>", s)),
            LispExpr::Boolean(b) => MathML::new(format!("<mn>{}</mn>", b)),
            LispExpr::Nil => MathML::new("<mi>nil</mi>".to_string()),
            LispExpr::List(list) => {
                if list.is_empty() {
                    return MathML::new(String::new());
                }

                if let LispExpr::Symbol(operator) = &list[0] {
                    let args = &list[1..];
                    match operator.as_str() {
                        "+" => MathML::addition(args),
                        "matrix"=> MathML::matrix(args),
                        _ => unimplemented!("From<&LispExpr> for MathML: operator `{}` not implemented", operator),
                    }
                } else {
                    return MathML::new("<mrow>Error: first element of a list must be a symbol</mrwo>".to_string());
                }

            }
        }

    }
}

impl MathML {
    fn addition(args: &[LispExpr]) -> Self {
        let args_mathml: Vec<String> = args.iter()
            .map(|arg| MathML::from(arg).to_string())
            .collect();
        
        format!("<mrow>{}</mrow>", args_mathml.join("<mo>+</mo>")).into()
    }

    fn matrix(args: &[LispExpr]) -> Self {
        let rows_mathml = args.iter().map(|row| {
            if let LispExpr::List(cells) = row {
                let cells_mathml = cells.iter()
                    .map(|cell| format!("<mtd>{}</mtd>", MathML::from(cell).to_string()))
                    .collect::<Vec<String>>()
                    .join("");
                format!("<mtr>{}</mtr>", cells_mathml)
            } else {
                "<mtr><mtd>Error: matrix row must be a list</mtd></mtr>".to_string()
            }
        }).collect::<Vec<String>>().join("");
        
        format!("<mrow><mo>[</mo><mtable>{}</mtable><mo>]</mo></mrow>", rows_mathml).into()
    }
}

pub struct Parser {
    tokens: Vec<String>,
    ast: Vec<LispExpr>,
}

impl Parser {
    pub fn new(src: String) -> Self {
        let token = Parser::tokenize(&src);
        Parser {
            tokens: token,
            ast: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let src = "(+ 1 2)".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(list) = ast {
            list.len()
        } else {
            0
        };

        assert_eq!(list_len, 3);
    }

    #[test]
    fn test_nary() {
        let src = "(* 1 2 3 4 5)".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(list) = ast {
            list.len()
        } else {
            0
        };

        assert_eq!(list_len, 6);
    }

    #[test]
    fn test_nested() {
        let src = "(+ 1 (* 2 3))".to_string();
        let mut parser = Parser::new(src);
        let ast = parser.parse();

        let list_len = if let LispExpr::List(ref list) = ast {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 3);

        // get the first memeber from within list
        let first = if let LispExpr::List(ref list) = ast {
            list[0].clone()
        } else {
            LispExpr::Nil
        };
        assert_eq!(first, LispExpr::Symbol("+".to_string()));

    }

    #[test]
    fn test_wrap_mathml() {
        let wrapped = wrap_mathml!("hello");
        assert_eq!(
            wrapped,
            "<math xmlns=\"http://www.w3.org/1998/Math/MathML\">hello</math>"
        );
    }

    #[test]
    fn test_simple_tokenize() {

        let p = Parser::tokenize("(+ 1 2)");
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], "(");
        assert_eq!(p[1], "+");
        assert_eq!(p[2], "1");
        assert_eq!(p[3], "2");
        assert_eq!(p[4], ")");
    }

    #[test]
    fn test_nested_tokenize() {
        let p = Parser::tokenize("(+ 1 (sin (pow 2 3)))");
        assert_eq!(p.len(), 12);
        assert_eq!(p[0], "(");
        assert_eq!(p[1], "+");
        assert_eq!(p[2], "1");
        assert_eq!(p[3], "(");
        assert_eq!(p[4], "sin");
        assert_eq!(p[5], "(");
        assert_eq!(p[6], "pow");
        assert_eq!(p[7], "2");
        assert_eq!(p[8], "3");
        assert_eq!(p[9], ")");
        assert_eq!(p[10], ")");
        assert_eq!(p[11], ")");
    }

    #[test]
    fn test_addition_codegen() {
        let mut p = Parser::new("(+ 1 2 3 4 5)".into());
        let ast = p.parse();

        let list_len = if let LispExpr::List(list) = ast.clone() {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 6);

        let mathml: MathML = (&ast).into();

        assert_eq!(mathml.to_string(), "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><mn>1</mn><mo>+</mo><mn>2</mn><mo>+</mo><mn>3</mn><mo>+</mo><mn>4</mn><mo>+</mo><mn>5</mn></mrow></math>");;
    }

    #[test]
    fn test_matrix_codegen() {
        let mut p = Parser::new("(matrix (1 2 3) (4 5 6) (7 8 9))".into());
        let ast = p.parse();

        let list_len = if let LispExpr::List(list) = ast.clone() {
            list.len()
        } else {
            0
        };
        assert_eq!(list_len, 4);

        let mathml: MathML = (&ast).into();

        dbg!(mathml.to_string());

    }
}
