use std::fmt::Debug;

use crate::pretty_err::{DebugContext, PrettyErr};

#[derive(Debug)]
pub enum MarkdownTag {
    Header(HeaderKind, String),
    LineSeparator,
    Checkbox(bool, String),
    BulletPoint(String),
    Blockquote(String),
    Link(String),
}

#[derive(Debug)]
pub enum HeaderKind {
    H1,
    H2,
    H3
}

impl Into<usize> for HeaderKind {
    fn into(self) -> usize {
        match self {
            HeaderKind::H1 => { 1 },
            HeaderKind::H2 => { 2 },
            HeaderKind::H3 => { 3 },
        }
    }
}

impl From<usize> for HeaderKind {
    fn from(val: usize) -> Self {
        match val {
            1 => { HeaderKind::H1 },
            2 => { HeaderKind::H2 },
            3 => { HeaderKind::H3 },
            _ => { HeaderKind::H1 },
        }
    }
}

#[derive(Debug)]
pub enum Token {
    LetExpr(String),
    Literal(String),
    Text(String),
    Comment(String),
    Markdown(MarkdownTag),
}

#[derive(Debug)]
pub struct Tokenizer {
    src: String,
    pos: usize,
    max: usize,
    debug_ctx: DebugContext,
}

impl Tokenizer {

    pub fn new(src: String, ctx: DebugContext) -> Self {
        let max = src.chars().count();
        Tokenizer {
            src,
            pos: 0,
            max,
            debug_ctx: ctx,
        }
    }
    
    fn char(&mut self) -> char {
        self.src.chars().nth(self.pos).expect("No more characters")
    }

    fn advance_char(&mut self) {
        if self.pos >= self.max {
            return;
        }
        self.pos += 1;
    }

    fn must_consume(&mut self, c: char) -> Result<(), anyhow::Error> {
        let curr = self.char();
        if curr != c {
            // ! how to handle errors
        }
        self.advance_char();
        Ok(())
    }

    fn consume_whitespace(&mut self) {
        while self.char().is_whitespace() {
            self.pos += 1;
        }
    }

    fn consume_until_not(&mut self, c: char) -> &str {
        let start= self.pos;
        while self.char() == c {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    fn consume_till(&mut self, c: char) -> &str {
        let start = self.pos;
        while self.char() != c {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    fn consume_line(&mut self) -> &str {
        self.consume_till('\n')
    }

    pub fn next_line(&mut self) -> Option<Vec<Token>> {
        if self.pos >= self.max {
            return None;
        }

        let mut tokens: Vec<Token> = Vec::new();
        while let Some(tok) = self.next_token() {
            tokens.push(tok)
        }

        self.advance_char();

        Some(tokens)
    }

    fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.max || self.char() == '\n' {
            return None;
        }

        let curr_token = self.char();

        // headers
        if self.char() == '#' {
            let hash_count = self.consume_until_not('#').len();

            let heading = self.consume_line().trim();
            let header_kind: HeaderKind = hash_count.into();

            return Some(
                Token::Markdown(
                    MarkdownTag::Header(header_kind, heading.to_string())
                )
            );
        // bullets or checkboxes 
        } else if curr_token == '-' {
            self.advance_char();
            self.consume_whitespace();

            let is_bullet = self.char() != '[';
            if is_bullet {
                let bullet = self.consume_line().trim();
                return Some(
                    Token::Markdown(
                        MarkdownTag::BulletPoint(bullet.to_string())
                    )
                );
            }

            self.advance_char();
            let is_checked = self.char() == 'x';

            self.advance_char();
            self.must_consume(']');
            self.consume_whitespace();

            let checkbox = self.consume_line().trim();
            return Some(
                Token::Markdown(
                    MarkdownTag::Checkbox(is_checked, checkbox.to_string())
                )
            );
        }

        None
    }

} 
