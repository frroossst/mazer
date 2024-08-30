use std::collections::VecDeque;

use unicode_segmentation::UnicodeSegmentation;

use crate::pretty_err::DebugContext;

#[derive(Debug, Clone)]
pub enum Operators {
    Add,
    Subtract,
    Multiply,
    Divide,
    Exponent,
    Modulus,
    InFixFn(String),
    Eq,
}

impl PartialEq for Operators {
    fn eq(&self, other: &Self) -> bool {
        self.precedence() == other.precedence()
    }
}

impl PartialOrd for Operators {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.precedence().partial_cmp(&other.precedence())
    }

    fn ge(&self, other: &Self) -> bool {
        self.precedence() >= other.precedence()
    }

    fn gt(&self, other: &Self) -> bool {
        self.precedence() > other.precedence()
    }

    fn le(&self, other: &Self) -> bool {
        self.precedence() <= other.precedence()
    }

    fn lt(&self, other: &Self) -> bool {
        self.precedence() < other.precedence()
    }
}

impl Operators {
    pub fn precedence(&self) -> u8 {
        match self {
            Operators::Eq => 0,
            Operators::InFixFn(_) => 1,
            Operators::Add | Operators::Subtract => 2,
            Operators::Multiply | Operators::Divide | Operators::Modulus => 3,
            Operators::Exponent => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MToken {
    LetBind,
    Number(f64),
    Identifier(String),
    Operator(String),
    Literal(String),
    Exclamation,
    LeftParen,
    RightParen,
    Comma,
    Equals,
    Semicolon,
    LeftSquareBracket,
    RightSquareBracket,
}


#[derive(Debug, Clone)]
pub enum ASTNode {
    Number(f64),
    Variable(String),
    Literal(String),
    BinaryOp {
        op: String,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    UnaryOp {
        op: String,
        operand: Box<ASTNode>,
    },
    FunctionCall {
        name: String,
        args: Vec<ASTNode>,
    },
    Assignment {
        name: String,
        value: Box<ASTNode>,
    },
    Array(Vec<ASTNode>),
}

impl Into<String> for ASTNode {
    fn into(self) -> String {
        match self {
            ASTNode::Number(n) => n.to_string(),
            ASTNode::Variable(name) => name,
            ASTNode::Literal(lit) => lit,
            ASTNode::BinaryOp { op, left, right } => {
                let lhs: String = (*left).into();
                let rhs: String = (*right).into();
                format!("{} {} {}", lhs, op, rhs)
            },
            // recursively convert array of arrays to string
            ASTNode::Array(arr) => {
                let elements: Vec<String> = arr.into_iter().map(Into::into).collect();
                format!("[{}]", elements.join(", "))
            },
            _ => {
                unimplemented!("Into<String> for ASTNode not implemented for {:?}", self);
            }
        }
    }
}

// TODO: make this Into<Vec<ByteCode>> later
impl ASTNode {
    pub fn to_postfix(node: &Self) -> Vec<String> {
        match node {
            ASTNode::Number(n) => vec![n.to_string()],
            ASTNode::Variable(name) => vec![name.clone()],
            ASTNode::Literal(lit) => vec![lit.clone()],
            ASTNode::BinaryOp { op, left, right } => {
                let mut result = Self::to_postfix(left);
                result.extend(Self::to_postfix(right));
                result.push(op.clone());
                result
            }
            ASTNode::UnaryOp { op, operand } => {
                let mut result = Self::to_postfix(operand);
                result.push(op.clone());
                result
            }
            ASTNode::FunctionCall { name, args } => {
                let mut result = Vec::new();
                for arg in args {
                    result.extend(Self::to_postfix(arg));
                }
                result.push(format!("{}_{}", name, args.len()));
                result
            }
            ASTNode::Assignment { name, value } => {
                let mut result = Self::to_postfix(value);
                result.push(format!("STORE_{}", name));
                result
            },
            ASTNode::Array(elements) => {
                let mut result = Vec::new();
                for element in elements {
                    result.extend(Self::to_postfix(element));
                }
                result.push(format!("ARRAY_{}", elements.len()));
                result
            },
        }
    }
}

impl PartialEq for ASTNode {
    fn eq(&self, other: &Self) -> bool {
        let result = match (self, other) {
            (ASTNode::Number(a), ASTNode::Number(b)) => (a - b).abs() < f64::EPSILON,
            (ASTNode::Variable(a), ASTNode::Variable(b)) => a == b,
            (ASTNode::Literal(a), ASTNode::Literal(b)) => a == b,
            (ASTNode::BinaryOp { op: op1, left: left1, right: right1 },
             ASTNode::BinaryOp { op: op2, left: left2, right: right2 }) =>
                op1 == op2 && left1 == left2 && right1 == right2,
            (ASTNode::UnaryOp { op: op1, operand: operand1 },
             ASTNode::UnaryOp { op: op2, operand: operand2 }) =>
                op1 == op2 && operand1 == operand2,
            (ASTNode::FunctionCall { name: name1, args: args1 },
             ASTNode::FunctionCall { name: name2, args: args2 }) =>
                name1 == name2 && args1 == args2,
            (ASTNode::Assignment { name: name1, value: value1 },
             ASTNode::Assignment { name: name2, value: value2 }) =>
                name1 == name2 && value1 == value2,
            (ASTNode::Array(elements1), ASTNode::Array(elements2)) => elements1 == elements2,
            _ => false,
        };
        if !result {
            println!("Comparison failed: \nLeft:  {:#?}\nRight: {:#?}", self, other);
        }
        result
    }
}

pub enum ParserMode {
    // for fmt(expr)/ ${expr} and eval(expr) 
    Expression,
    // for let x = expr;
    Statement,
}

pub struct Parser {
    tokens: VecDeque<MToken>,
    current: Option<MToken>,
    mode: ParserMode,
}

impl Parser {
    pub fn new(stream: String) -> Self {
        let mut p = Parser {
            tokens: Parser::tokenize(stream),
            current: None,
            mode: ParserMode::Statement,
        };
        p.advance();

        p
    }

    fn tokenize(stream: String) -> VecDeque<MToken> {
        /*
        let stream: VecDeque<MToken> = stream.replace(";", " ; ")
            .replace("[", " [ ")
            .replace("]", " ] ")
            .replace("(", " ( ")
            .replace(")", " ) ")
            .replace(",", " , ")
            .replace("^", " ^ ")
            .replace("\"", " \" ")
            .split_whitespace()
            .map(|s| match s {
                "let" => MToken::Identifier("let".to_string()),
                "=" => MToken::Equals,
                ";" => MToken::Semicolon,
                "[" => MToken::LeftSquareBracket,
                "]" => MToken::RightSquareBracket,
                "(" => MToken::LeftParen,
                ")" => MToken::RightParen,
                "+" | "-" | "*" | "/" | "^" => MToken::Operator(s.to_string()),
                "," => MToken::Comma,
                s if s.parse::<f64>().is_ok() => MToken::Number(s.parse().unwrap()),
                _ => MToken::Identifier(s.to_string()),
            })
            .collect();
        */

        let mut tokens = VecDeque::new();
        let mut buffer = String::new();
        let mut in_quotes = false;

        for c in UnicodeSegmentation::graphemes(stream.as_str(), true) {
            if in_quotes && c != "\"" {
                buffer.push_str(c);
            } else if in_quotes && c == "\"" {
                in_quotes = false;
                let tok = MToken::Literal(buffer.clone());
                tokens.push_back(tok);
                buffer.clear();
            }

            match c {
                "=" => {
                    tokens.push_back(MToken::Equals);
                },
                ";" => {
                    tokens.push_back(MToken::Semicolon);
                },
                "[" => {
                    tokens.push_back(MToken::LeftSquareBracket);
                },
                "]" => {
                    tokens.push_back(MToken::RightSquareBracket);
                },
                "(" => {
                    tokens.push_back(MToken::LeftParen);
                },
                ")" => {
                    tokens.push_back(MToken::RightParen);
                },
                "+" | "-" | "*" | "/" | "^" => {
                    tokens.push_back(MToken::Operator(c.to_string()));
                },
                "," => {
                    tokens.push_back(MToken::Comma);
                },
                _ => { buffer.push_str(c); },
            }

            if c == " " {
                let b = buffer.trim();
                let t = match b.parse::<f64>() {
                    Ok(n) => MToken::Number(n),
                    Err(_) => {
                        if b == "let" {
                            MToken::LetBind
                        } else {
                            MToken::Identifier(b.to_string())
                        }
                    }
                };
                tokens.push_back(t);
                buffer.clear();
            }
        }

        dbg!(tokens.clone());

        tokens
    }

    #[inline]
    fn advance(&mut self) {
        self.current = self.tokens.pop_front();
    }

    fn expect(&mut self, expected: MToken) -> Result<(), DebugContext> {
        if self.current == Some(expected.clone()) {
            self.advance();
            Ok(())
        } else {
            panic!("Expected {:?}, found {:?}", expected, self.current);
        }
    }

    pub fn set_mode(&mut self, mode: ParserMode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn parse(&mut self) -> Result<Vec<ASTNode>, DebugContext> {
        let mut ast = Vec::new();
        while self.current.is_some() {
            match self.mode {
                ParserMode::Statement => ast.push(self.parse_statement()?),
                ParserMode::Expression => ast.push(self.parse_expression()?),
            }
        }
        assert_eq!(ast.len(), 1); // ! only for debugging 
        Ok(ast)
    }

}
