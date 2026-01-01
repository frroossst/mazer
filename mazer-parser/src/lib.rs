use rayon::prelude::*;
use std::fmt;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEndOfInput { expected: String, position: usize },
    InvalidSyntax { message: String, position: usize },
    EmptyInput,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEndOfInput { expected, position } => {
                write!(
                    f,
                    "Unexpected end of input at position {}: expected {}",
                    position, expected
                )
            }
            ParseError::InvalidSyntax { message, position } => {
                write!(f, "Invalid syntax at position {}: {}", position, message)
            }
            ParseError::EmptyInput => write!(f, "Cannot parse empty input"),
        }
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum AST {
    Header { level: u8, text: String },
    BulletPoint { text: String },
    CheckboxUnchecked { text: String },
    CheckboxChecked { text: String },
    BlockQuote { content: String },
    Spoiler { content: String },
    Link { text: String, url: String },
    CodeBlock { language: Option<String>, code: String },
    InlineCode { code: String },
    Bold { text: String },
    Italic { text: String },
    Underline { text: String },
    Strikethrough { text: String },
    PageSeparator,
    EvalScheme { code: String },
    ShowScheme { code: String },
    Text { content: String },
    Paragraph { children: Vec<AST> },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Header(u8),
    BulletPoint,
    CheckboxUnchecked,
    CheckboxChecked,
    BlockQuote,
    TripleBacktick,
    SingleBacktick,
    DoublePipe,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    TripleDash,
    DoubleStar,
    SingleStar,
    Underscore,
    Tilde,
    Newline,
    Text(String),
    Whitespace(String),
}

struct Tokenizer<'a> {
    _input: &'a str,
    pos: usize,
    graphemes: Vec<&'a str>,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            _input: input,
            pos: 0,
            graphemes: input.graphemes(true).collect(),
        }
    }

    fn peek(&self, offset: usize) -> Option<&str> {
        self.graphemes.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<&str> {
        if self.pos < self.graphemes.len() {
            let g = self.graphemes[self.pos];
            self.pos += 1;
            Some(g)
        } else {
            None
        }
    }

    fn advance_by(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.graphemes.len());
    }

    fn skip_whitespace_inline(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(g) = self.peek(0) {
            if g == " " || g == "\t" {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos > start {
            Some(self.graphemes[start..self.pos].concat())
        } else {
            None
        }
    }

    fn consume_until(&mut self, predicate: impl Fn(&str) -> bool) -> String {
        let start = self.pos;
        while let Some(g) = self.peek(0) {
            if predicate(g) {
                break;
            }
            self.advance();
        }
        self.graphemes[start..self.pos].concat()
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::with_capacity(self.graphemes.len() / 10);

        while self.pos < self.graphemes.len() {
            let g = match self.peek(0) {
                Some(s) => s,
                None => break,
            };

            match g {
                "\n" => {
                    self.advance();
                    tokens.push(Token::Newline);
                }
                "\r" => {
                    self.advance();
                    if self.peek(0) == Some("\n") {
                        self.advance();
                    }
                    tokens.push(Token::Newline);
                }
                "#" if self.pos == 0 || matches!(tokens.last(), Some(Token::Newline)) => {
                    let mut level = 0;
                    while self.peek(0) == Some("#") && level < 6 {
                        self.advance();
                        level += 1;
                    }
                    if self.peek(0) == Some(" ") {
                        self.advance();
                        tokens.push(Token::Header(level));
                    } else {
                        tokens.push(Token::Text("#".repeat(level as usize)));
                    }
                }
                "-" => {
                    // Only check for special markdown syntax at start of line
                    if self.pos == 0 || matches!(tokens.last(), Some(Token::Newline)) {
                        if self.peek(1) == Some("-") && self.peek(2) == Some("-") {
                            let peek3 = self.peek(3);
                            if peek3.is_none() || peek3 == Some("\n") || peek3 == Some(" ") {
                                self.advance_by(3);
                                tokens.push(Token::TripleDash);
                                continue;
                            }
                        }

                        if self.peek(1) == Some("[") {
                            self.advance_by(2);
                            match self.peek(0) {
                                Some(" ") if self.peek(1) == Some("]") => {
                                    self.advance_by(2);
                                    if self.peek(0) == Some(" ") {
                                        self.advance();
                                    }
                                    tokens.push(Token::CheckboxUnchecked);
                                    continue;
                                }
                                Some("x") | Some("X") if self.peek(1) == Some("]") => {
                                    self.advance_by(2);
                                    if self.peek(0) == Some(" ") {
                                        self.advance();
                                    }
                                    tokens.push(Token::CheckboxChecked);
                                    continue;
                                }
                                _ => {
                                    tokens.push(Token::Text("-[".to_string()));
                                    continue;
                                }
                            }
                        }

                        if self.peek(1) == Some(" ") {
                            self.advance_by(2);
                            tokens.push(Token::BulletPoint);
                            continue;
                        }
                    }
                    
                    // For hyphens not at start of line or not followed by space, treat as text
                    self.advance();
                    tokens.push(Token::Text("-".to_string()));
                }
                "`" => {
                    if self.peek(1) == Some("`") && self.peek(2) == Some("`") {
                        self.advance_by(3);
                        tokens.push(Token::TripleBacktick);
                    } else {
                        self.advance();
                        tokens.push(Token::SingleBacktick);
                    }
                }
                "|" => {
                    if self.peek(1) == Some("|") {
                        self.advance_by(2);
                        tokens.push(Token::DoublePipe);
                    } else {
                        self.advance();
                        tokens.push(Token::Text("|".to_string()));
                    }
                }
                "[" => {
                    self.advance();
                    tokens.push(Token::LeftBracket);
                }
                "]" => {
                    self.advance();
                    tokens.push(Token::RightBracket);
                }
                "(" => {
                    self.advance();
                    tokens.push(Token::LeftParen);
                }
                ")" => {
                    self.advance();
                    tokens.push(Token::RightParen);
                }
                ">" if self.pos == 0 || matches!(tokens.last(), Some(Token::Newline)) => {
                    self.advance();
                    if self.peek(0) == Some(" ") {
                        self.advance();
                    }
                    tokens.push(Token::BlockQuote);
                }
                "*" => {
                    if self.peek(1) == Some("*") {
                        self.advance_by(2);
                        tokens.push(Token::DoubleStar);
                    } else {
                        self.advance();
                        tokens.push(Token::SingleStar);
                    }
                }
                "_" => {
                    self.advance();
                    tokens.push(Token::Underscore);
                }
                "~" => {
                    self.advance();
                    tokens.push(Token::Tilde);
                }
                " " | "\t" => {
                    if let Some(ws) = self.skip_whitespace_inline() {
                        tokens.push(Token::Whitespace(ws));
                    }
                }
                _ => {
                    let text = self.consume_until(|g| {
                        matches!(
                            g,
                            "\n" | "\r"
                                | "`"
                                | "|"
                                | "["
                                | "]"
                                | "("
                                | ")"
                                | "#"
                                | "-"
                                | ">"
                                | "*"
                                | "_"
                                | "~"
                        )
                    });
                    if !text.is_empty() {
                        tokens.push(Token::Text(text));
                    }
                }
            }
        }

        tokens
    }
}

struct TokenParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(0), Some(Token::Newline)) {
            self.advance();
        }
    }

    fn collect_line_text(&mut self) -> String {
        let mut text = String::new();
        while let Some(token) = self.peek(0) {
            match token {
                Token::Newline => break,
                Token::Text(t) => {
                    text.push_str(t);
                    self.advance();
                }
                Token::Whitespace(ws) => {
                    text.push_str(ws);
                    self.advance();
                }
                _ => break,
            }
        }
        text
    }

    fn parse_inline_elements(&mut self, until_newline: bool) -> Vec<AST> {
        let mut elements = Vec::new();
        let mut text_buffer = String::new();

        let flush_text = |text_buffer: &mut String, elements: &mut Vec<AST>| {
            if !text_buffer.is_empty() {
                elements.push(AST::Text {
                    content: std::mem::take(text_buffer),
                });
            }
        };

        while let Some(token) = self.peek(0) {
            match token {
                Token::Newline if until_newline => break,
                Token::Newline => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    text_buffer.push('\n');
                }
                Token::SingleBacktick => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut code = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::SingleBacktick => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                code.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                code.push_str(ws);
                                self.advance();
                            }
                            Token::Newline => {
                                code.push('\n');
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::InlineCode { code });
                }
                Token::DoublePipe => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut content = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::DoublePipe => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                content.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                content.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::Spoiler { content });
                }
                Token::DoubleStar => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut text = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::DoubleStar => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                text.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                text.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::Bold { text });
                }
                Token::SingleStar => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut text = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::SingleStar => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                text.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                text.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::Italic { text });
                }
                Token::Underscore => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut text = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::Underscore => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                text.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                text.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::Underline { text });
                }
                Token::Tilde => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut text = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::Tilde => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                text.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                text.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    elements.push(AST::Strikethrough { text });
                }
                Token::LeftBracket => {
                    flush_text(&mut text_buffer, &mut elements);
                    self.advance();
                    let mut link_text = String::new();
                    while let Some(token) = self.peek(0) {
                        match token {
                            Token::RightBracket => {
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                link_text.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                link_text.push_str(ws);
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }

                    if matches!(self.peek(0), Some(Token::LeftParen)) {
                        self.advance();
                        let mut url = String::new();
                        while let Some(token) = self.peek(0) {
                            match token {
                                Token::RightParen => {
                                    self.advance();
                                    break;
                                }
                                Token::Text(t) => {
                                    url.push_str(t);
                                    self.advance();
                                }
                                Token::Whitespace(ws) => {
                                    url.push_str(ws);
                                    self.advance();
                                }
                                _ => {
                                    self.advance();
                                }
                            }
                        }
                        elements.push(AST::Link {
                            text: link_text,
                            url,
                        });
                    } else {
                        text_buffer.push('[');
                        text_buffer.push_str(&link_text);
                        text_buffer.push(']');
                    }
                }
                Token::LeftParen => {
                    let saved_pos = self.pos;
                    self.advance();

                    if let Some(Token::Text(first_word)) = self.peek(0) {
                        let word_trimmed = first_word.trim();
                        if word_trimmed == "eval" || word_trimmed == "show" {
                            let is_eval = word_trimmed == "eval";
                            self.advance();

                            if matches!(self.peek(0), Some(Token::Whitespace(_))) {
                                self.advance();
                            }

                            let mut scheme_code = String::new();
                            let mut paren_depth = 1;

                            while let Some(token) = self.peek(0) {
                                match token {
                                    Token::LeftParen => {
                                        scheme_code.push('(');
                                        paren_depth += 1;
                                        self.advance();
                                    }
                                    Token::RightParen => {
                                        paren_depth -= 1;
                                        if paren_depth == 0 {
                                            self.advance();
                                            break;
                                        }
                                        scheme_code.push(')');
                                        self.advance();
                                    }
                                    Token::Text(t) => {
                                        scheme_code.push_str(t);
                                        self.advance();
                                    }
                                    Token::Whitespace(ws) => {
                                        scheme_code.push_str(ws);
                                        self.advance();
                                    }
                                    Token::Newline => {
                                        scheme_code.push('\n');
                                        self.advance();
                                    }
                                    _ => {
                                        self.advance();
                                    }
                                }
                            }

                            flush_text(&mut text_buffer, &mut elements);
                            if is_eval {
                                elements.push(AST::EvalScheme { code: scheme_code });
                            } else {
                                elements.push(AST::ShowScheme { code: scheme_code });
                            }
                            continue;
                        }
                    }

                    self.pos = saved_pos;
                    self.advance();
                    text_buffer.push('(');
                }
                Token::RightParen => {
                    text_buffer.push(')');
                    self.advance();
                }
                Token::Text(t) => {
                    text_buffer.push_str(t);
                    self.advance();
                }
                Token::Whitespace(ws) => {
                    text_buffer.push_str(ws);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        flush_text(&mut text_buffer, &mut elements);
        elements
    }

    fn parse(&mut self) -> Vec<AST> {
        let mut ast_nodes = Vec::new();

        while self.pos < self.tokens.len() {
            self.skip_newlines();

            if self.pos >= self.tokens.len() {
                break;
            }

            match self.peek(0) {
                Some(Token::Header(level)) => {
                    let level = *level;
                    self.advance();
                    let text = self.collect_line_text();

                    ast_nodes.push(AST::Header { level: level.clamp(1, 6) , text });
                }
                Some(Token::BulletPoint) => {
                    self.advance();
                    let inline = self.parse_inline_elements(true);
                    let text = inline
                        .into_iter()
                        .map(|node| match node {
                            AST::Text { content } => content,
                            AST::InlineCode { code } => format!("`{}`", code),
                            _ => String::new(),
                        })
                        .collect::<String>();
                    ast_nodes.push(AST::BulletPoint { text });
                }
                Some(Token::CheckboxUnchecked) => {
                    self.advance();
                    let text = self.collect_line_text();
                    ast_nodes.push(AST::CheckboxUnchecked { text });
                }
                Some(Token::CheckboxChecked) => {
                    self.advance();
                    let text = self.collect_line_text();
                    ast_nodes.push(AST::CheckboxChecked { text });
                }
                Some(Token::BlockQuote) => {
                    self.advance();
                    let content = self.collect_line_text();
                    ast_nodes.push(AST::BlockQuote { content });
                }
                Some(Token::TripleDash) => {
                    self.advance();
                    ast_nodes.push(AST::PageSeparator);
                }
                Some(Token::TripleBacktick) => {
                    self.advance();
                    let mut language = None;
                    if let Some(Token::Text(lang)) = self.peek(0) {
                        language = Some(lang.clone());
                        self.advance();
                    }

                    while matches!(self.peek(0), Some(Token::Whitespace(_))) {
                        self.advance();
                    }
                    if matches!(self.peek(0), Some(Token::Newline)) {
                        self.advance();
                    }

                    eprintln!("DEBUG: Starting code block parsing at pos {}", self.pos);
                    let mut code = String::new();
                    let mut iterations = 0;
                    while let Some(token) = self.peek(0) {
                        iterations += 1;
                        if iterations % 100 == 0 {
                            eprintln!("DEBUG: Iteration {}, pos {}, token: {:?}", iterations, self.pos, token);
                        }
                        match token {
                            Token::TripleBacktick => {
                                eprintln!("DEBUG: Found closing backticks at pos {}", self.pos);
                                self.advance();
                                break;
                            }
                            Token::Text(t) => {
                                code.push_str(t);
                                self.advance();
                            }
                            Token::Whitespace(ws) => {
                                code.push_str(ws);
                                self.advance();
                            }
                            Token::Newline => {
                                code.push('\n');
                                self.advance();
                            }
                            Token::LeftParen => {
                                code.push('(');
                                self.advance();
                            }
                            Token::RightParen => {
                                code.push(')');
                                self.advance();
                            }
                            Token::LeftBracket => {
                                code.push('[');
                                self.advance();
                            }
                            Token::RightBracket => {
                                code.push(']');
                                self.advance();
                            }
                            Token::SingleBacktick => {
                                code.push('`');
                                self.advance();
                            }
                            Token::DoublePipe => {
                                code.push_str("||");
                                self.advance();
                            }
                            Token::DoubleStar => {
                                code.push_str("**");
                                self.advance();
                            }
                            Token::SingleStar => {
                                code.push('*');
                                self.advance();
                            }
                            Token::Underscore => {
                                code.push('_');
                                self.advance();
                            }
                            Token::Tilde => {
                                code.push('~');
                                self.advance();
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }

                    ast_nodes.push(AST::CodeBlock { language, code });
                }
                _ => {
                    let start_pos = self.pos;
                    let inline = self.parse_inline_elements(false);
                    if !inline.is_empty() {
                        if inline.len() == 1 {
                            ast_nodes.push(inline.into_iter().next().unwrap());
                        } else {
                            ast_nodes.push(AST::Paragraph { children: inline });
                        }
                    }
                    if self.pos == start_pos {
                        self.advance();
                    }
                }
            }
        }

        ast_nodes
    }
}

pub struct Parser<'a> {
    input: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub fn parse(&self) -> Result<Vec<AST>> {
        if self.input.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let lines: Vec<&str> = self.input.lines().collect();

        // Decide whether to use parallel parsing based on input size
        if lines.len() < 100 {
            self.parse_sequential()
        } else {
            self.parse_parallel()
        }
    }

    fn parse_sequential(&self) -> Result<Vec<AST>> {
        let mut tokenizer = Tokenizer::new(self.input);
        let tokens = tokenizer.tokenize();
        Ok(Parser::parse_tokens(tokens))
    }

    fn parse_parallel(&self) -> Result<Vec<AST>> {
        let lines: Vec<&str> = self.input.lines().collect();
        let chunk_size = (lines.len() / rayon::current_num_threads()).max(50);
        let chunks: Vec<_> = lines
            .chunks(chunk_size)
            .map(|chunk| chunk.join("\n"))
            .collect();

        Ok(chunks
            .par_iter()
            .filter_map(|chunk| {
                let mut tokenizer = Tokenizer::new(chunk);
                let tokens = tokenizer.tokenize();
                Some(Parser::parse_tokens(tokens))
            })
            .flatten()
            .collect())
    }

    fn parse_tokens(tokens: Vec<Token>) -> Vec<AST> {
        let mut parser = TokenParser::new(tokens);
        parser.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers() {
        let input = "# Header 1\n## Header 2\n### Header 3";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 3);
        assert!(matches!(ast[0], AST::Header { level: 1, .. }));
        assert!(matches!(ast[1], AST::Header { level: 2, .. }));
        assert!(matches!(ast[2], AST::Header { level: 3, .. }));
    }

    #[test]
    fn test_bullet_points() {
        let input = "- Item 1\n- Item 2";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 2);
        assert!(matches!(ast[0], AST::BulletPoint { .. }));
    }

    #[test]
    fn test_checkboxes() {
        let input = "-[ ] Unchecked\n-[x] Checked";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 2);
        assert!(matches!(ast[0], AST::CheckboxUnchecked { .. }));
        assert!(matches!(ast[1], AST::CheckboxChecked { .. }));
    }

    #[test]
    fn test_spoiler() {
        let input = "This is ||spoiled content|| here";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| match node {
            AST::Paragraph { children } =>
                children.iter().any(|c| matches!(c, AST::Spoiler { .. })),
            AST::Spoiler { .. } => true,
            _ => false,
        }));
    }

    #[test]
    fn test_link() {
        let input = "[Click here](https://example.com)";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| matches!(node, AST::Link { .. })));
    }

    #[test]
    fn test_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let ast = Parser::new(input).parse().unwrap();
        assert!(matches!(ast[0], AST::CodeBlock { .. }));
        if let AST::CodeBlock { language, .. } = &ast[0] {
            assert_eq!(language, &Some("rust".to_string()));
        }
    }

    #[test]
    fn test_code_block_no_language() {
        let input = "```\nsome code\n```";
        let ast = Parser::new(input).parse().unwrap();
        assert!(matches!(ast[0], AST::CodeBlock { .. }));
        if let AST::CodeBlock { language, code } = &ast[0] {
            assert_eq!(language, &None);
            assert_eq!(code, "some code\n");
        }
    }

    #[test]
    fn test_inline_code() {
        let input = "This is `inline code` here";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| match node {
            AST::Paragraph { children } =>
                children.iter().any(|c| matches!(c, AST::InlineCode { .. })),
            _ => false,
        }));
    }

    #[test]
    fn test_page_separator() {
        let input = "---";
        let ast = Parser::new(input).parse().unwrap();
        assert!(matches!(ast[0], AST::PageSeparator));
    }

    #[test]
    fn test_eval_scheme() {
        let input = "Result: (eval (+ 1 1))";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| match node {
            AST::Paragraph { children } =>
                children.iter().any(|c| matches!(c, AST::EvalScheme { .. })),
            AST::EvalScheme { .. } => true,
            _ => false,
        }));
    }

    #[test]
    fn test_show_scheme() {
        let input = "(show (matrix (1 1 1)))";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| match node {
            AST::Paragraph { children } =>
                children.iter().any(|c| matches!(c, AST::ShowScheme { .. })),
            AST::ShowScheme { .. } => true,
            _ => false,
        }));
    }

    #[test]
    fn test_normal_parentheses() {
        let input = "This is a (Sentence).";
        let ast = Parser::new(input).parse().unwrap();
        if let AST::Text { content } = &ast[0] {
            assert!(content.contains("(Sentence)"));
        }
    }

    #[test]
    fn test_nested_scheme() {
        let input = "(eval (+ 1 1))";
        let ast = Parser::new(input).parse().unwrap();
        assert!(
            ast.iter()
                .any(|node| matches!(node, AST::EvalScheme { .. }))
        );
        if let AST::EvalScheme { code } = &ast[0] {
            assert!(code.contains("(+ 1 1)"));
        }
    }

    #[test]
    fn test_emoji_in_text() {
        let input = "Hello 👋 World 🌍";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 1);
        if let AST::Text { content } = &ast[0] {
            assert!(content.contains("👋"));
            assert!(content.contains("🌍"));
            assert_eq!(content, "Hello 👋 World 🌍");
        } else {
            panic!("Expected AST::Text, got {:?}", ast[0]);
        }
    }

    #[test]
    fn test_emoji_in_header() {
        let input = "# Header with emoji 🚀";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 1);
        if let AST::Header { level, text } = &ast[0] {
            assert_eq!(*level, 1);
            assert!(text.contains("🚀"));
            assert_eq!(text, "Header with emoji 🚀");
        } else {
            panic!("Expected AST::Header");
        }
    }

    #[test]
    fn test_emoji_in_bullet_point() {
        let input = "- Item with emoji 📝";
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 1);
        if let AST::BulletPoint { text } = &ast[0] {
            assert!(text.contains("📝"));
        } else {
            panic!("Expected AST::BulletPoint");
        }
    }

    #[test]
    fn test_emoji_in_bold() {
        let input = "**Bold text 💪**";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.iter().any(|node| match node {
            AST::Bold { text } => text.contains("💪"),
            AST::Paragraph { children } => children.iter().any(|c| match c {
                AST::Bold { text } => text.contains("💪"),
                _ => false,
            }),
            _ => false,
        }));
    }

    #[test]
    fn test_multibyte_utf8_characters() {
        let input = "こんにちは世界 مرحبا بالعالم Привет мир";
        let ast = Parser::new(input).parse().unwrap();
        if let AST::Text { content } = &ast[0] {
            assert!(content.contains("こんにちは世界"));
            assert!(content.contains("مرحبا بالعالم"));
            assert!(content.contains("Привет мир"));
        } else {
            panic!("Expected AST::Text");
        }
    }

    #[test]
    fn test_emoji_with_skin_tone_modifier() {
        // Skin tone modifiers are combining characters that should be treated as single graphemes
        let input = "👍🏽 thumbs up";
        let ast = Parser::new(input).parse().unwrap();
        if let AST::Text { content } = &ast[0] {
            assert!(content.contains("👍🏽"));
            assert_eq!(content, "👍🏽 thumbs up");
        } else {
            panic!("Expected AST::Text");
        }
    }

    #[test]
    fn test_complex_emoji_sequences() {
        // Family emoji is a complex sequence: 👨‍👩‍👧‍👦
        let input = "Family: 👨‍👩‍👧‍👦";
        let ast = Parser::new(input).parse().unwrap();
        if let AST::Text { content } = &ast[0] {
            assert!(content.contains("👨‍👩‍👧‍👦"));
        } else {
            panic!("Expected AST::Text");
        }
    }

    #[test]
    fn test_mixed_emoji_and_markdown() {
        let input = "# 🎉 Celebration 🎊\n- First item 🥇\n- Second item 🥈";
        let ast = Parser::new(input).parse().unwrap();
        assert!(ast.len() >= 2);

        // Check header
        if let AST::Header { text, .. } = &ast[0] {
            assert!(text.contains("🎉"));
            assert!(text.contains("🎊"));
            assert!(text.contains("Celebration"));
        }

        // Check bullet points
        let bullet_count = ast.iter().filter(|node| matches!(node, AST::BulletPoint { .. })).count();
        assert_eq!(bullet_count, 2);
    }

    #[test]
    fn test_scheme_in_codeblock() {
        let input = r#"```scheme
(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))
```
"#;
        let ast = Parser::new(input).parse().unwrap();
        assert_eq!(ast.len(), 1);
        if let AST::CodeBlock { language, code } = &ast[0] {
            assert_eq!(language, &Some("scheme".to_string()));
            assert!(code.contains("factorial"));
        } else {
            panic!("Expected CodeBlock, got {:?}", ast[0]);
        }
    }
}
