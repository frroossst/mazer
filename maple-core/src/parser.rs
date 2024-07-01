use std::collections::VecDeque;

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
    Number(f64),
    Identifier(String),
    Operator(String),
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

impl PartialEq for ASTNode {
    fn eq(&self, other: &Self) -> bool {
        let result = match (self, other) {
            (ASTNode::Number(a), ASTNode::Number(b)) => (a - b).abs() < f64::EPSILON,
            (ASTNode::Variable(a), ASTNode::Variable(b)) => a == b,
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

pub struct Parser {
    tokens: VecDeque<MToken>,
    current: Option<MToken>,
}

impl Parser {
    pub fn new(stream: String) -> Self {
        let mut p = Parser {
            tokens: Parser::tokenize(stream),
            current: None,
        };
        p.advance();

        p
    }

    fn tokenize(stream: String) -> VecDeque<MToken> {
        let stream: VecDeque<MToken> = stream.replace(";", " ; ")
            .replace("[", " [ ")
            .replace("]", " ] ")
            .replace("(", " ( ")
            .replace(")", " ) ")
            .replace(",", " , ")
            .split_whitespace()
            .map(|s| match s {
                "let" => MToken::Identifier("let".to_string()),
                "=" => MToken::Equals,
                ";" => MToken::Semicolon,
                "[" => MToken::LeftSquareBracket,
                "]" => MToken::RightSquareBracket,
                "(" => MToken::LeftParen,
                ")" => MToken::RightParen,
                "+" | "-" | "*" | "/" | "**" => MToken::Operator(s.to_string()),
                "," => MToken::Comma,
                s if s.parse::<f64>().is_ok() => MToken::Number(s.parse().unwrap()),
                _ => MToken::Identifier(s.to_string()),
            })
            .collect();

        stream
    }

    fn advance(&mut self) {
        self.current = self.tokens.pop_front();
    }

    fn expect(&mut self, expected: MToken) {
        if self.current == Some(expected.clone()) {
            self.advance();
        } else {
            panic!("Expected {:?}, found {:?}", expected, self.current);
        }
    }

    pub fn parse(&mut self) -> Vec<ASTNode> {
        let mut ast = Vec::new();
        while self.current.is_some() {
            ast.push(self.parse_statement());
        }
        ast
    }

    fn parse_array_elements(&mut self) -> Vec<ASTNode> {
        let mut elements = Vec::new();
        if self.current != Some(MToken::RightSquareBracket) {
            loop {
                elements.push(self.parse_expression());
                if self.current != Some(MToken::Comma) {
                    break;
                }
                self.advance(); // Consume comma
            }
        }
        elements
    }

    fn parse_statement(&mut self) -> ASTNode {
        self.expect(MToken::Identifier("let".to_string()));
        if let Some(MToken::Identifier(name)) = self.current.clone() {
            self.advance();
            self.expect(MToken::Equals);
            let value = self.parse_expression();
            self.expect(MToken::Semicolon);
            ASTNode::Assignment {
                name,
                value: Box::new(value),
            }
        } else {
            panic!("Expected identifier after 'let'");
        }
    }

    fn parse_expression(&mut self) -> ASTNode {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, min_precedence: i32) -> ASTNode {
        let mut left = self.parse_primary();

        while let Some(MToken::Operator(op)) = &self.current {
            let precedence = self.get_precedence(op);
            if precedence < min_precedence {
                break;
            }

            let op = op.clone();
            self.advance(); // Consume operator

            let right = self.parse_binary_expression(precedence + 1);
            left = ASTNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        left
    }

    fn parse_primary(&mut self) -> ASTNode {
        let node = match self.current.clone() {
            Some(MToken::Number(n)) => {
                self.advance();
                ASTNode::Number(n)
            }
            Some(MToken::Identifier(name)) => {
                self.advance();
                if self.current == Some(MToken::LeftParen) {
                    self.advance(); // Consume left paren
                    let args = self.parse_function_arguments();
                    self.expect(MToken::RightParen);
                    ASTNode::FunctionCall { name, args }
                } else {
                    ASTNode::Variable(name)
                }
            }
            Some(MToken::LeftParen) => {
                self.advance();
                let expr = self.parse_expression();
                self.expect(MToken::RightParen);
                expr
            }
            Some(MToken::Operator(op)) if op == "-" => {
                self.advance();
                let operand = self.parse_primary();
                ASTNode::UnaryOp {
                    op,
                    operand: Box::new(operand),
                }
            },
            Some(MToken::LeftSquareBracket) => {
                self.advance(); // Consume '['
                let elements = self.parse_array_elements();
                self.expect(MToken::RightSquareBracket);
                ASTNode::Array(elements)
            },
            _ => panic!("Unexpected token: {:?}", self.current),
        };

        // Check for binary function syntax
        if let Some(MToken::Identifier(func_name)) = &self.current {
            if let ASTNode::FunctionCall { .. } = node {
                let func_name = func_name.clone();
                self.advance(); // Consume function name
                let right = self.parse_primary();
                return ASTNode::FunctionCall {
                    name: func_name,
                    args: vec![node, right],
                };
            }
        }

        node
    }

    fn parse_function_arguments(&mut self) -> Vec<ASTNode> {
        let mut args = Vec::new();
        if self.current != Some(MToken::RightParen) {
            loop {
                args.push(self.parse_expression());
                if self.current != Some(MToken::Comma) {
                    break;
                }
                self.advance(); // Consume comma
            }
        }
        args
    }

    fn get_precedence(&self, op: &str) -> i32 {
        match op {
            "+" | "-" => 1,
            "*" | "/" => 2,
            "**" => 3,
            _ => 0,
        }
    }

}
